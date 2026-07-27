use super::{
    journal, reconcile,
    repository::Repository,
    types::{
        ExportQuarantineCopyResult, QuarantineManifest, QuarantineRecordState, QUARANTINE_PROTOCOL,
    },
};
use crate::audit;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const COPY_BUFFER_BYTES: usize = 1024 * 1024;
const EXPORT_ROOT_NAME: &str = "restore-exports";
const EXPORT_DIRECTORY_PREFIX: &str = "Qingpan-Export-";

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message.into())
}

#[cfg(windows)]
fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn ensure_export_root(repository: &Repository) -> io::Result<PathBuf> {
    let parent = repository
        .root()
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| invalid_input("quarantine repository has no export parent"))?;
    let export_root = parent.join(EXPORT_ROOT_NAME);
    fs::create_dir_all(&export_root)?;
    let metadata = fs::symlink_metadata(&export_root)?;
    if !metadata.is_dir() || is_reparse_or_symlink(&metadata) {
        return Err(invalid_input(
            "quarantine export root is not a trusted local directory",
        ));
    }
    Ok(export_root)
}

fn sha256_file(path: &Path) -> io::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut bytes = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        bytes = bytes
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("quarantine export byte count overflow"))?;
    }
    Ok((bytes, format!("{:x}", hasher.finalize())))
}

fn copy_and_verify(
    repository: &Repository,
    manifest: &QuarantineManifest,
    destination: &Path,
) -> io::Result<u64> {
    let object_path = repository.object_path(manifest.record_id);
    let path_metadata = fs::symlink_metadata(&object_path)?;
    if !path_metadata.is_file() || is_reparse_or_symlink(&path_metadata) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "quarantine object is not a trusted regular file",
        ));
    }
    let mut source = File::open(&object_path)?;
    let source_metadata = source.metadata()?;
    if !source_metadata.is_file() || source_metadata.len() != manifest.size_bytes {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "quarantine object size does not match its manifest",
        ));
    }

    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut copied = 0u64;
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        target.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        copied = copied
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("quarantine export byte count overflow"))?;
    }
    target.flush()?;
    target.sync_all()?;
    drop(target);

    let source_sha256 = format!("{:x}", hasher.finalize());
    if copied != manifest.size_bytes || source_sha256 != manifest.sha256 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "quarantine object failed manifest verification",
        ));
    }

    let (exported_bytes, exported_sha256) = sha256_file(destination)?;
    if exported_bytes != manifest.size_bytes || exported_sha256 != manifest.sha256 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "exported quarantine copy failed verification",
        ));
    }
    Ok(exported_bytes)
}

fn cleanup_created_export_directory(
    export_root: &Path,
    export_directory: &Path,
    operation_id: Uuid,
) {
    let expected_name = format!("{EXPORT_DIRECTORY_PREFIX}{operation_id}");
    if export_directory.parent() == Some(export_root)
        && export_directory.file_name().and_then(|name| name.to_str()) == Some(&expected_name)
    {
        let _ = fs::remove_dir_all(export_directory);
    }
}

fn append_success_audit(
    audit_path: Option<&Path>,
    operation_id: Uuid,
    manifest: &QuarantineManifest,
) -> bool {
    let Some(audit_path) = audit_path else {
        return false;
    };
    let mut record =
        audit::OperationRecord::new(audit::OperationKind::RestoreExport, QUARANTINE_PROTOCOL);
    record.operation_id = operation_id;
    record.status = audit::OperationStatus::Succeeded;
    record.completed_at_ms = audit::unix_time_ms();
    record.reclaimed_bytes = 0;
    record.staged_bytes = manifest.size_bytes;
    record.succeeded.push(audit::OperationDetail {
        item_id: manifest.record_id.to_string(),
        path: None,
        bytes: manifest.size_bytes,
        detail: "exported verified quarantine copy; quarantine object retained".into(),
    });
    audit::append_record(audit_path, &record).is_ok()
}

fn export_copy_with_audit_path(
    repository: &Repository,
    record_id: Uuid,
    audit_path: Option<&Path>,
) -> Result<ExportQuarantineCopyResult, String> {
    let manifest = repository
        .read_manifest(record_id)
        .map_err(|error| format!("无法读取隔离记录: {error}"))?;
    let entries = journal::read(&repository.journal_path(record_id))
        .map_err(|error| format!("隔离记录日志无效: {error}"))?;
    let state =
        reconcile::derive_state(&entries).map_err(|error| format!("隔离记录状态无效: {error}"))?;
    if matches!(state, QuarantineRecordState::RecoveryRequired) {
        return Err(
            "隔离记录处于恢复必需状态，普通导出已拒绝；仅可由未来的救援/取证流程处理".into(),
        );
    }
    if !matches!(
        state,
        QuarantineRecordState::Committed | QuarantineRecordState::SourceRetained
    ) {
        return Err("隔离记录当前不可导出".into());
    }

    let operation_id = Uuid::new_v4();
    let export_root =
        ensure_export_root(repository).map_err(|error| format!("无法准备固定导出目录: {error}"))?;
    let export_directory = export_root.join(format!("{EXPORT_DIRECTORY_PREFIX}{operation_id}"));
    fs::create_dir(&export_directory).map_err(|error| format!("无法创建唯一导出目录: {error}"))?;

    let destination = export_directory.join(&manifest.file_name);
    let exported_bytes = match copy_and_verify(repository, &manifest, &destination) {
        Ok(bytes) => bytes,
        Err(error) => {
            cleanup_created_export_directory(&export_root, &export_directory, operation_id);
            return Err(format!("导出隔离副本失败: {error}"));
        }
    };

    let audit_persisted = append_success_audit(audit_path, operation_id, &manifest);
    Ok(ExportQuarantineCopyResult {
        operation_id: operation_id.to_string(),
        record_id: record_id.to_string(),
        exported_directory: export_directory.to_string_lossy().into_owned(),
        exported_file_name: manifest.file_name,
        bytes: exported_bytes,
        quarantine_source_retained: true,
        audit_persisted,
    })
}

pub(crate) fn export_copy(
    repository: &Repository,
    record_id: Uuid,
) -> Result<ExportQuarantineCopyResult, String> {
    let audit_path = audit::default_path().ok();
    export_copy_with_audit_path(repository, record_id, audit_path.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::types::{JournalEvent, QuarantineManifest, QUARANTINE_SCHEMA_VERSION};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("qingpan-restore-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if self
                .0
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("qingpan-restore-"))
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    fn record(
        repository: &Repository,
        bytes: &[u8],
        sha256: Option<String>,
        terminal: JournalEvent,
    ) -> Uuid {
        let record_id = Uuid::new_v4();
        let mut object = repository.create_object(record_id).unwrap();
        object.write_all(bytes).unwrap();
        object.flush().unwrap();
        object.sync_all().unwrap();
        let manifest = QuarantineManifest {
            schema_version: QUARANTINE_SCHEMA_VERSION,
            protocol: QUARANTINE_PROTOCOL.into(),
            record_id,
            file_name: "cache.tmp".into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            created_at_ms: 1,
            size_bytes: bytes.len() as u64,
            sha256: sha256.unwrap_or_else(|| format!("{:x}", Sha256::digest(bytes))),
            object_name: format!("{record_id}.blob"),
        };
        repository.write_manifest(&manifest).unwrap();
        for event in [
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
        ] {
            journal::append(&repository.journal_path(record_id), event).unwrap();
        }
        if terminal == JournalEvent::Committed {
            journal::append(
                &repository.journal_path(record_id),
                JournalEvent::SourceDeletePrepared,
            )
            .unwrap();
        }
        if terminal != JournalEvent::ObjectCommitted {
            journal::append(&repository.journal_path(record_id), terminal).unwrap();
        }
        record_id
    }

    #[test]
    fn exports_to_fixed_unique_directory_without_removing_object() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("quarantine-preview-v1"));
        let record_id = record(
            &repository,
            b"verified quarantine content",
            None,
            JournalEvent::Committed,
        );
        let audit_path = directory.0.join("audit.jsonl");

        let result =
            export_copy_with_audit_path(&repository, record_id, Some(&audit_path)).unwrap();
        let export_directory = PathBuf::from(&result.exported_directory);
        assert_eq!(
            export_directory.parent(),
            Some(directory.0.join(EXPORT_ROOT_NAME).as_path())
        );
        assert_eq!(
            fs::read(export_directory.join("cache.tmp")).unwrap(),
            b"verified quarantine content"
        );
        assert!(repository.object_path(record_id).exists());
        assert!(result.quarantine_source_retained);
        assert!(result.audit_persisted);

        let audit = audit::read_recent(&audit_path, 10).unwrap();
        assert_eq!(audit.records.len(), 1);
        assert_eq!(audit.records[0].kind, audit::OperationKind::RestoreExport);
        assert_eq!(
            audit.records[0].operation_id.to_string(),
            result.operation_id
        );
        assert!(audit.records[0]
            .succeeded
            .iter()
            .all(|detail| detail.path.is_none()));
    }

    #[test]
    fn hash_failure_removes_only_the_new_export_directory() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("quarantine-preview-v1"));
        let record_id = record(
            &repository,
            b"object bytes",
            Some("0".repeat(64)),
            JournalEvent::SourceRetained,
        );

        let error = export_copy_with_audit_path(&repository, record_id, None).unwrap_err();
        assert!(error.contains("导出隔离副本失败"));
        assert!(repository.object_path(record_id).exists());
        let export_root = directory.0.join(EXPORT_ROOT_NAME);
        assert_eq!(fs::read_dir(export_root).unwrap().count(), 0);
    }

    #[test]
    fn uncertain_delete_state_rejects_ordinary_export() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("quarantine-preview-v1"));
        let record_id = record(
            &repository,
            b"uncertain",
            None,
            JournalEvent::SourceDeletePrepared,
        );

        let error = export_copy_with_audit_path(&repository, record_id, None).unwrap_err();
        assert!(error.contains("恢复必需状态"));
        assert!(error.contains("救援/取证流程"));
        assert!(repository.object_path(record_id).exists());
        assert!(!directory.0.join(EXPORT_ROOT_NAME).exists());
    }

    #[test]
    fn audit_failure_does_not_roll_back_a_verified_export() {
        let directory = TestDirectory::new();
        let repository = Repository::new(directory.0.join("quarantine-preview-v1"));
        let record_id = record(
            &repository,
            b"copy survives audit failure",
            None,
            JournalEvent::Committed,
        );

        let result = export_copy_with_audit_path(&repository, record_id, None).unwrap();
        assert!(!result.audit_persisted);
        assert!(Path::new(&result.exported_directory)
            .join(&result.exported_file_name)
            .exists());
        assert!(repository.object_path(record_id).exists());
    }
}
