use super::types::{JournalEntry, JournalEvent, QUARANTINE_SCHEMA_VERSION};
use crate::audit;
use crate::fs_safety::is_link_or_reparse;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::Path;

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message.into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_existing_journal(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !is_link_or_reparse(&metadata) => Ok(()),
        Ok(_) => Err(invalid_data(
            "quarantine journal is not a trusted regular file",
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(crate) fn read(path: &Path) -> io::Result<Vec<JournalEntry>> {
    validate_existing_journal(path)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut reader = BufReader::new(file);
    let mut raw = Vec::new();
    let mut entries = Vec::new();
    let mut previous_line_hash: Option<String> = None;

    loop {
        raw.clear();
        if reader.read_until(b'\n', &mut raw)? == 0 {
            break;
        }
        if raw.last() != Some(&b'\n') {
            return Err(invalid_data("quarantine journal has a truncated tail"));
        }
        raw.pop();
        if raw.last() == Some(&b'\r') {
            raw.pop();
        }
        if raw.is_empty() {
            return Err(invalid_data("quarantine journal contains an empty entry"));
        }

        let entry: JournalEntry =
            serde_json::from_slice(&raw).map_err(|error| invalid_data(error.to_string()))?;
        let expected_sequence = entries.len() as u64 + 1;
        if entry.schema_version != QUARANTINE_SCHEMA_VERSION {
            return Err(invalid_data("unsupported quarantine journal schema"));
        }
        if entry.sequence != expected_sequence {
            return Err(invalid_data(
                "quarantine journal sequence is not contiguous",
            ));
        }
        if entry.previous_entry_sha256 != previous_line_hash {
            return Err(invalid_data("quarantine journal hash chain is invalid"));
        }
        previous_line_hash = Some(sha256_hex(&raw));
        entries.push(entry);
    }
    Ok(entries)
}

pub(crate) fn append(path: &Path, event: JournalEvent) -> io::Result<JournalEntry> {
    validate_existing_journal(path)?;
    let entries = read(path)?;
    let previous_entry_sha256 = if entries.is_empty() {
        None
    } else {
        let raw = std::fs::read(path)?;
        let line = raw
            .strip_suffix(b"\n")
            .ok_or_else(|| invalid_data("quarantine journal has a truncated tail"))?
            .rsplit(|byte| *byte == b'\n')
            .next()
            .ok_or_else(|| invalid_data("quarantine journal is empty"))?;
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        Some(sha256_hex(line))
    };
    let entry = JournalEntry {
        schema_version: QUARANTINE_SCHEMA_VERSION,
        sequence: entries.len() as u64 + 1,
        previous_entry_sha256,
        occurred_at_ms: audit::unix_time_ms(),
        event,
    };
    let mut encoded =
        serde_json::to_vec(&entry).map_err(|error| invalid_data(error.to_string()))?;
    encoded.push(b'\n');
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&encoded)?;
    file.flush()?;
    file.sync_all()?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("qingpan-journal-{}.jsonl", Uuid::new_v4()))
    }

    #[test]
    fn append_builds_a_contiguous_hash_chain() {
        let path = test_path();
        append(&path, JournalEvent::Prepared).expect("first append should work");
        append(&path, JournalEvent::Copying).expect("second append should work");
        let entries = read(&path).expect("journal should verify");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].sequence, 2);
        assert!(entries[1].previous_entry_sha256.is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn truncated_tail_fails_closed() {
        let path = test_path();
        append(&path, JournalEvent::Prepared).expect("append should work");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"schemaVersion\":1").unwrap();
        file.flush().unwrap();
        assert_eq!(read(&path).unwrap_err().kind(), ErrorKind::InvalidData);
        let _ = fs::remove_file(path);
    }
}
