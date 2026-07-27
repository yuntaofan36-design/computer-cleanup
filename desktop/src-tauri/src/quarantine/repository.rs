use super::{
    journal, reconcile,
    types::{
        QuarantineListResponse, QuarantineManifest, QuarantineRecord, QuarantineRecordState,
        QUARANTINE_PROTOCOL, QUARANTINE_SCHEMA_VERSION,
    },
};
use crate::fs_safety::is_link_or_reparse;
use serde_json::Value;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const JOURNAL_DIRECTORY: &str = "journal";
const OBJECT_DIRECTORY: &str = "objects";
const MANIFEST_DIRECTORY: &str = "manifests";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MANIFEST_FIELDS: [&str; 10] = [
    "schemaVersion",
    "protocol",
    "recordId",
    "fileName",
    "ruleId",
    "planId",
    "createdAtMs",
    "sizeBytes",
    "sha256",
    "objectName",
];

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message.into())
}

fn validate_repository_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err(invalid_data(
            "quarantine repository path is not a trusted regular directory",
        ));
    }
    Ok(())
}

fn object_name(record_id: Uuid) -> String {
    format!("{record_id}.blob")
}

fn manifest_name(record_id: Uuid) -> String {
    format!("{record_id}.json")
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_safe_export_file_name(value: &str) -> bool {
    if value.is_empty()
        || value.encode_utf16().count() > 255
        || value == "."
        || value == ".."
        || value.ends_with(' ')
        || value.ends_with('.')
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
    {
        return false;
    }

    let path = Path::new(value);
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return false;
    }
    if path.file_name() != Some(OsStr::new(value)) {
        return false;
    }

    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

fn validate_manifest(manifest: &QuarantineManifest, expected_id: Uuid) -> io::Result<()> {
    if manifest.schema_version != QUARANTINE_SCHEMA_VERSION {
        return Err(invalid_data("unsupported quarantine manifest schema"));
    }
    if manifest.protocol != QUARANTINE_PROTOCOL {
        return Err(invalid_data("unsupported quarantine manifest protocol"));
    }
    if manifest.record_id != expected_id {
        return Err(invalid_data("quarantine manifest record ID mismatch"));
    }
    if manifest.object_name != object_name(expected_id) {
        return Err(invalid_data(
            "quarantine manifest object name is not canonical",
        ));
    }
    if !is_safe_export_file_name(&manifest.file_name) {
        return Err(invalid_data("quarantine manifest file name is unsafe"));
    }
    if manifest.rule_id.is_empty() || manifest.plan_id.is_empty() {
        return Err(invalid_data("quarantine manifest origin is incomplete"));
    }
    if !is_lower_sha256(&manifest.sha256) {
        return Err(invalid_data("quarantine manifest SHA-256 is not canonical"));
    }
    Ok(())
}

pub(crate) struct Repository {
    root: PathBuf,
}

impl Repository {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn ensure_layout(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        for path in self.root.ancestors().take(3) {
            validate_repository_directory(path)?;
        }
        for name in [JOURNAL_DIRECTORY, OBJECT_DIRECTORY, MANIFEST_DIRECTORY] {
            let path = self.root.join(name);
            fs::create_dir(&path).or_else(|error| {
                if error.kind() == ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(error)
                }
            })?;
            validate_repository_directory(&path)?;
        }
        Ok(())
    }

    pub(crate) fn journal_path(&self, record_id: Uuid) -> PathBuf {
        self.root
            .join(JOURNAL_DIRECTORY)
            .join(format!("{record_id}.jsonl"))
    }

    pub(crate) fn object_path(&self, record_id: Uuid) -> PathBuf {
        self.root
            .join(OBJECT_DIRECTORY)
            .join(object_name(record_id))
    }

    fn manifest_path(&self, record_id: Uuid) -> PathBuf {
        self.root
            .join(MANIFEST_DIRECTORY)
            .join(manifest_name(record_id))
    }

    pub(crate) fn create_object(&self, record_id: Uuid) -> io::Result<File> {
        self.ensure_layout()?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.object_path(record_id))
    }

    pub(crate) fn remove_object(&self, record_id: Uuid) -> io::Result<()> {
        fs::remove_file(self.object_path(record_id))
    }

    pub(crate) fn write_manifest(&self, manifest: &QuarantineManifest) -> io::Result<()> {
        validate_manifest(manifest, manifest.record_id)?;
        self.ensure_layout()?;
        let mut encoded =
            serde_json::to_vec(manifest).map_err(|error| invalid_data(error.to_string()))?;
        encoded.push(b'\n');
        if encoded.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(invalid_data("quarantine manifest exceeds the size limit"));
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.manifest_path(manifest.record_id))?;
        file.write_all(&encoded)?;
        file.flush()?;
        file.sync_all()
    }

    pub(crate) fn read_manifest(&self, record_id: Uuid) -> io::Result<QuarantineManifest> {
        let path = self.manifest_path(record_id);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
            return Err(invalid_data(
                "quarantine manifest is not a bounded regular file",
            ));
        }
        let raw = fs::read(path)?;
        let value: Value =
            serde_json::from_slice(&raw).map_err(|error| invalid_data(error.to_string()))?;
        let fields = value
            .as_object()
            .ok_or_else(|| invalid_data("quarantine manifest root is not an object"))?;
        let expected = MANIFEST_FIELDS.into_iter().collect::<HashSet<_>>();
        let actual = fields.keys().map(String::as_str).collect::<HashSet<_>>();
        if actual != expected {
            return Err(invalid_data("quarantine manifest fields are not closed"));
        }
        let manifest: QuarantineManifest =
            serde_json::from_value(value).map_err(|error| invalid_data(error.to_string()))?;
        validate_manifest(&manifest, record_id)?;
        Ok(manifest)
    }

    pub(crate) fn list(&self, limit: usize) -> io::Result<QuarantineListResponse> {
        let manifest_root = self.root.join(MANIFEST_DIRECTORY);
        let entries = match fs::read_dir(&manifest_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(QuarantineListResponse {
                    records: Vec::new(),
                    corrupt_records: 0,
                });
            }
            Err(error) => return Err(error),
        };

        let mut records = Vec::new();
        let mut corrupt_records = 0usize;
        for entry in entries {
            let entry = entry?;
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => {
                    corrupt_records = corrupt_records.saturating_add(1);
                    continue;
                }
            };
            if !file_type.is_file() {
                corrupt_records = corrupt_records.saturating_add(1);
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                corrupt_records = corrupt_records.saturating_add(1);
                continue;
            };
            let Some(stem) = name.strip_suffix(".json") else {
                corrupt_records = corrupt_records.saturating_add(1);
                continue;
            };
            let Ok(record_id) = Uuid::parse_str(stem) else {
                corrupt_records = corrupt_records.saturating_add(1);
                continue;
            };
            if name != manifest_name(record_id) {
                corrupt_records = corrupt_records.saturating_add(1);
                continue;
            }

            let manifest = match self.read_manifest(record_id) {
                Ok(manifest) => manifest,
                Err(_) => {
                    corrupt_records = corrupt_records.saturating_add(1);
                    continue;
                }
            };
            let journal_entries = match journal::read(&self.journal_path(record_id)) {
                Ok(entries) if !entries.is_empty() => entries,
                Ok(_) | Err(_) => {
                    corrupt_records = corrupt_records.saturating_add(1);
                    continue;
                }
            };
            let reconciled_state = match reconcile::derive_state(&journal_entries) {
                Ok(state) => state,
                Err(_) => {
                    corrupt_records = corrupt_records.saturating_add(1);
                    continue;
                }
            };

            let object_is_valid = fs::symlink_metadata(self.object_path(record_id))
                .map(|metadata| metadata.is_file() && metadata.len() == manifest.size_bytes)
                .unwrap_or(false);
            let state = if object_is_valid {
                reconciled_state
            } else {
                QuarantineRecordState::Damaged
            };
            let source_retained = matches!(reconciled_state, QuarantineRecordState::SourceRetained);
            let exportable = object_is_valid
                && matches!(
                    state,
                    QuarantineRecordState::Committed | QuarantineRecordState::SourceRetained
                );
            records.push(QuarantineRecord {
                record_id: manifest.record_id.to_string(),
                file_name: manifest.file_name,
                rule_id: manifest.rule_id,
                plan_id: manifest.plan_id,
                created_at_ms: manifest.created_at_ms,
                size_bytes: manifest.size_bytes,
                state,
                exportable,
                source_retained,
            });
        }

        records.sort_by(|left, right| {
            right
                .created_at_ms
                .cmp(&left.created_at_ms)
                .then_with(|| left.record_id.cmp(&right.record_id))
        });
        records.truncate(limit);
        Ok(QuarantineListResponse {
            records,
            corrupt_records,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::types::{JournalEvent, QUARANTINE_PROTOCOL};
    use sha2::{Digest, Sha256};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("qingpan-repository-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if self
                .0
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with("qingpan-repository-"))
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    fn manifest(
        record_id: Uuid,
        name: &str,
        created_at_ms: u64,
        bytes: &[u8],
    ) -> QuarantineManifest {
        QuarantineManifest {
            schema_version: QUARANTINE_SCHEMA_VERSION,
            protocol: QUARANTINE_PROTOCOL.into(),
            record_id,
            file_name: name.into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            created_at_ms,
            size_bytes: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            object_name: object_name(record_id),
        }
    }

    fn add_record(
        repository: &Repository,
        created_at_ms: u64,
        bytes: &[u8],
        events: &[JournalEvent],
    ) -> Uuid {
        let record_id = Uuid::new_v4();
        let mut object = repository.create_object(record_id).unwrap();
        object.write_all(bytes).unwrap();
        object.flush().unwrap();
        object.sync_all().unwrap();
        repository
            .write_manifest(&manifest(record_id, "cache.tmp", created_at_ms, bytes))
            .unwrap();
        for event in events {
            journal::append(&repository.journal_path(record_id), *event).unwrap();
        }
        record_id
    }

    #[test]
    fn layout_is_lazy_and_object_and_manifest_are_create_new() {
        let directory = TestDirectory::new();
        let root = directory.0.join("store");
        let repository = Repository::new(root.clone());
        assert!(!root.exists());

        let record_id = Uuid::new_v4();
        let object = repository.create_object(record_id).unwrap();
        drop(object);
        assert!(repository.create_object(record_id).is_err());

        let manifest = manifest(record_id, "cache.tmp", 1, b"");
        repository.write_manifest(&manifest).unwrap();
        assert_eq!(repository.read_manifest(record_id).unwrap(), manifest);
        assert!(repository.write_manifest(&manifest).is_err());
    }

    #[test]
    fn manifest_rejects_noncanonical_paths_and_object_names() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("store"));
        let record_id = Uuid::new_v4();
        let mut invalid = manifest(record_id, "..\\escape.txt", 1, b"data");
        assert_eq!(
            repository.write_manifest(&invalid).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        invalid.file_name = "safe.txt".into();
        invalid.object_name = "other.blob".into();
        assert_eq!(
            repository.write_manifest(&invalid).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn list_maps_states_marks_damaged_and_sorts_before_limiting() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("store"));
        let prefix = [
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
        ];
        let retained = add_record(&repository, 20, b"retained", &prefix);
        let committed = add_record(
            &repository,
            30,
            b"committed",
            &[
                JournalEvent::Prepared,
                JournalEvent::Copying,
                JournalEvent::ObjectVerified,
                JournalEvent::ObjectCommitted,
                JournalEvent::SourceDeletePrepared,
                JournalEvent::Committed,
            ],
        );
        let uncertain = add_record(
            &repository,
            10,
            b"uncertain",
            &[
                JournalEvent::Prepared,
                JournalEvent::Copying,
                JournalEvent::ObjectVerified,
                JournalEvent::ObjectCommitted,
                JournalEvent::SourceDeletePrepared,
            ],
        );
        repository.remove_object(retained).unwrap();
        fs::write(
            repository.root.join(MANIFEST_DIRECTORY).join("bad.json"),
            b"{}",
        )
        .unwrap();

        let listed = repository.list(10).unwrap();
        assert_eq!(listed.corrupt_records, 1);
        assert_eq!(listed.records.len(), 3);
        assert_eq!(listed.records[0].record_id, committed.to_string());
        assert_eq!(listed.records[0].state, QuarantineRecordState::Committed);
        assert!(listed.records[0].exportable);
        assert_eq!(listed.records[1].record_id, retained.to_string());
        assert_eq!(listed.records[1].state, QuarantineRecordState::Damaged);
        assert!(listed.records[1].source_retained);
        assert!(!listed.records[1].exportable);
        assert_eq!(listed.records[2].record_id, uncertain.to_string());
        assert_eq!(
            listed.records[2].state,
            QuarantineRecordState::RecoveryRequired
        );
        assert!(!listed.records[2].exportable);

        let limited = repository.list(1).unwrap();
        assert_eq!(limited.records.len(), 1);
        assert_eq!(limited.records[0].record_id, committed.to_string());
        assert_eq!(limited.corrupt_records, 1);
    }
}
