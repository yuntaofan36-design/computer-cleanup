use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const AUDIT_SCHEMA_VERSION: u32 = 1;

static APPEND_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Succeeded,
    PartiallySucceeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Cleanup,
    Quarantine,
    Uninstall,
    RestoreExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDetail {
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub bytes: u64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub rule_version: String,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    /// Bytes no longer occupying their original volume after the operation.
    pub reclaimed_bytes: u64,
    /// Bytes still occupying disk space in a quarantine/staging directory.
    pub staged_bytes: u64,
    pub succeeded: Vec<OperationDetail>,
    pub failed: Vec<OperationDetail>,
    pub skipped: Vec<OperationDetail>,
}

impl OperationRecord {
    pub fn new(kind: OperationKind, rule_version: impl Into<String>) -> Self {
        let now = unix_time_ms();
        Self {
            schema_version: AUDIT_SCHEMA_VERSION,
            operation_id: Uuid::new_v4(),
            kind,
            status: OperationStatus::Skipped,
            rule_version: rule_version.into(),
            started_at_ms: now,
            completed_at_ms: now,
            reclaimed_bytes: 0,
            staged_bytes: 0,
            succeeded: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentRecords {
    /// Newest appended record first. Timestamps are not trusted for ordering.
    pub records: Vec<OperationRecord>,
    pub skipped_corrupt_lines: usize,
}

/// Protocol for a future non-destructive restore operation.
///
/// An implementation must export into a dedicated, newly created directory.
/// It must reject an existing destination and must never overwrite original
/// paths in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreExportRequest {
    pub source_operation_id: Uuid,
    pub destination_directory: PathBuf,
}

/// Returns `%LOCALAPPDATA%/Qingpan/audit.jsonl` on Windows.
pub fn default_path() -> io::Result<PathBuf> {
    let local_app_data = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs::data_local_dir)
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "LOCALAPPDATA is unavailable"))?;
    Ok(local_app_data.join("Qingpan").join("audit.jsonl"))
}

pub fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

/// Appends exactly one JSON record and durably synchronizes it to disk.
pub fn append_record(path: impl AsRef<Path>, record: &OperationRecord) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let mut encoded = serde_json::to_vec(record)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    encoded.push(b'\n');

    let _guard = APPEND_LOCK
        .lock()
        .map_err(|_| io::Error::other("audit append lock is poisoned"))?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&encoded)?;
    file.flush()?;
    file.sync_all()
}

/// Reads valid records in reverse append order.
///
/// Malformed JSON, invalid UTF-8, and truncated lines are counted and skipped.
/// A missing history file is treated as an empty history. The scan remains
/// bounded to `limit` retained records, while still counting every bad line.
pub fn read_recent(path: impl AsRef<Path>, limit: usize) -> io::Result<RecentRecords> {
    let file = match File::open(path.as_ref()) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(RecentRecords {
                records: Vec::new(),
                skipped_corrupt_lines: 0,
            });
        }
        Err(error) => return Err(error),
    };

    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut recent = VecDeque::with_capacity(limit);
    let mut skipped_corrupt_lines = 0;

    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }

        match serde_json::from_slice::<OperationRecord>(&line) {
            Ok(record) if limit > 0 => {
                if recent.len() == limit {
                    recent.pop_front();
                }
                recent.push_back(record);
            }
            Ok(_) => {}
            Err(_) => skipped_corrupt_lines += 1,
        }
    }

    Ok(RecentRecords {
        records: recent.into_iter().rev().collect(),
        skipped_corrupt_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = env::temp_dir().join(format!("qingpan-audit-test-{}", Uuid::new_v4()));
            fs::create_dir(&path).expect("temporary audit directory should be created");
            Self(path)
        }

        fn audit_path(&self) -> PathBuf {
            self.0.join("audit.jsonl")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if self
                .0
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("qingpan-audit-test-"))
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    fn record(index: u64) -> OperationRecord {
        OperationRecord {
            schema_version: AUDIT_SCHEMA_VERSION,
            operation_id: Uuid::new_v4(),
            kind: OperationKind::Cleanup,
            status: OperationStatus::Succeeded,
            rule_version: "cleanup-rules-v1".into(),
            started_at_ms: index * 10,
            completed_at_ms: index * 10 + 1,
            reclaimed_bytes: index * 100,
            staged_bytes: index * 25,
            succeeded: vec![OperationDetail {
                item_id: format!("item-{index}"),
                path: Some(format!(r"C:\Temp\item-{index}.tmp")),
                bytes: index * 100,
                detail: "deleted after snapshot validation".into(),
            }],
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    #[test]
    fn appends_and_reads_newest_first() {
        let directory = TestDirectory::new();
        let path = directory.audit_path();
        let first = record(1);
        let second = record(2);

        append_record(&path, &first).expect("first record should append");
        append_record(&path, &second).expect("second record should append");

        let result = read_recent(&path, 10).expect("history should be readable");
        assert_eq!(result.skipped_corrupt_lines, 0);
        assert_eq!(result.records, vec![second, first]);
    }

    #[test]
    fn corrupt_lines_are_counted_and_skipped() {
        let directory = TestDirectory::new();
        let path = directory.audit_path();
        let first = record(1);
        let second = record(2);
        append_record(&path, &first).expect("first record should append");

        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("history should open for corruption fixture");
        file.write_all(b"{truncated-json\n\xff\xfe\n")
            .expect("corruption fixture should append");
        file.flush().expect("corruption fixture should flush");

        append_record(&path, &second).expect("valid append should survive corrupt history");
        let result = read_recent(&path, 10).expect("corrupt history should remain readable");

        assert_eq!(result.skipped_corrupt_lines, 2);
        assert_eq!(result.records, vec![second, first]);
    }

    #[test]
    fn limit_keeps_only_most_recent_records() {
        let directory = TestDirectory::new();
        let path = directory.audit_path();
        let records = [record(1), record(2), record(3)];
        for record in &records {
            append_record(&path, record).expect("record should append");
        }

        let result = read_recent(&path, 2).expect("limited history should be readable");
        assert_eq!(result.records, vec![records[2].clone(), records[1].clone()]);

        let empty = read_recent(&path, 0).expect("zero limit should be supported");
        assert!(empty.records.is_empty());
        assert_eq!(empty.skipped_corrupt_lines, 0);
    }
}
