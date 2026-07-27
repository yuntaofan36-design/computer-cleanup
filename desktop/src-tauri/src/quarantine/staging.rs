use super::journal;
use super::repository::Repository;
use super::types::{
    JournalEvent, QuarantineCandidate, QuarantineManifest, StageResult, QUARANTINE_PROTOCOL,
    QUARANTINE_SCHEMA_VERSION,
};
use crate::audit;
use crate::fs_safety::file_identity_from_file;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use uuid::Uuid;

const COPY_BUFFER_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_QUARANTINE_FILE_BYTES: u64 = 1024 * 1024 * 1024;

fn open_source(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x0000_0001;
        return OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(path);
    }
    #[cfg(not(windows))]
    File::open(path)
}

fn hash_file(path: &Path, expected_size: u64) -> Result<String, String> {
    let file = open_source(path).map_err(|error| format!("无法读取待校验文件: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("无法读取待校验文件元数据: {error}"))?;
    if !metadata.is_file() || metadata.len() != expected_size {
        return Err("待校验文件大小或类型已变化".into());
    }
    let mut reader = BufReader::with_capacity(COPY_BUFFER_BYTES, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut bytes = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("读取待校验文件失败: {error}"))?;
        if read == 0 {
            break;
        }
        bytes = bytes.saturating_add(read as u64);
        if bytes > expected_size {
            return Err("待校验文件在读取期间增长".into());
        }
        hasher.update(&buffer[..read]);
    }
    if bytes != expected_size {
        return Err("待校验文件在读取期间缩短".into());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn copy_source_to_object(
    source_path: &Path,
    object: File,
    expected_size: u64,
) -> Result<String, String> {
    let source = open_source(source_path).map_err(|error| format!("无法打开源文件: {error}"))?;
    let metadata = source
        .metadata()
        .map_err(|error| format!("无法读取源文件元数据: {error}"))?;
    if !metadata.is_file() || metadata.len() != expected_size {
        return Err("源文件大小或类型已变化".into());
    }
    let source_identity = file_identity_from_file(&source, &metadata)
        .map_err(|_| "无法确认源文件卷身份，已安全保留".to_string())?;
    let object_metadata = object
        .metadata()
        .map_err(|error| format!("无法读取隔离对象元数据: {error}"))?;
    let object_identity = file_identity_from_file(&object, &object_metadata)
        .map_err(|_| "无法确认隔离仓库卷身份，已安全保留".to_string())?;
    if !source_identity.same_volume(object_identity) {
        return Err("源文件与实验性隔离仓库不在同一卷，已安全保留".into());
    }

    let mut reader = BufReader::with_capacity(COPY_BUFFER_BYTES, source);
    let mut writer = BufWriter::with_capacity(COPY_BUFFER_BYTES, object);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    let mut bytes = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("读取源文件失败: {error}"))?;
        if read == 0 {
            break;
        }
        bytes = bytes.saturating_add(read as u64);
        if bytes > expected_size {
            return Err("源文件在复制期间增长".into());
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| format!("写入隔离对象失败: {error}"))?;
        hasher.update(&buffer[..read]);
    }
    if bytes != expected_size {
        return Err("源文件在复制期间缩短".into());
    }
    writer
        .flush()
        .map_err(|error| format!("刷新隔离对象失败: {error}"))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| format!("持久化隔离对象失败: {error}"))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_file_name(file_name: &str) -> Result<(), String> {
    let path = Path::new(file_name);
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || path.is_absolute()
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name)
    {
        return Err("隔离文件名无效".into());
    }
    Ok(())
}

fn mark_source_retained(
    repository: &Repository,
    record_id: Uuid,
    size_bytes: u64,
    detail: String,
) -> Result<StageResult, String> {
    journal::append(
        &repository.journal_path(record_id),
        JournalEvent::SourceRetained,
    )
    .map_err(|error| format!("源文件已保留，但无法持久化隔离终态: {error}"))?;
    Ok(StageResult {
        record_id: record_id.to_string(),
        size_bytes,
        source_retained: true,
        recovery_required: false,
        detail: Some(detail),
    })
}

fn verify_retained_source<F>(
    source_path: &Path,
    expected_size: u64,
    expected_hash: &str,
    validate_source: &mut F,
) -> Result<(), String>
where
    F: FnMut(&Path) -> Result<(), String>,
{
    validate_source(source_path)
        .map_err(|error| format!("删除失败后无法确认源文件身份: {error}"))?;
    let current_hash = hash_file(source_path, expected_size)
        .map_err(|error| format!("删除失败后无法确认源文件内容: {error}"))?;
    if current_hash != expected_hash {
        return Err("删除失败后源文件内容与已提交隔离对象不一致".into());
    }
    validate_source(source_path)
        .map_err(|error| format!("删除失败后最终源文件身份复检失败: {error}"))
}

fn stage_file_with_remover<F, R>(
    repository: &Repository,
    candidate: QuarantineCandidate,
    mut validate_source: F,
    mut remove_source: R,
) -> Result<StageResult, String>
where
    F: FnMut(&Path) -> Result<(), String>,
    R: FnMut(&Path) -> io::Result<()>,
{
    validate_file_name(&candidate.file_name)?;
    if candidate.expected_size > MAX_QUARANTINE_FILE_BYTES {
        return Err("单个文件超过实验性隔离上限，已保留源文件".into());
    }
    validate_source(&candidate.source_path)?;
    repository
        .ensure_layout()
        .map_err(|error| format!("无法初始化隔离仓库: {error}"))?;

    let record_id = Uuid::new_v4();
    let journal_path = repository.journal_path(record_id);
    journal::append(&journal_path, JournalEvent::Prepared)
        .map_err(|error| format!("无法准备隔离日志: {error}"))?;
    journal::append(&journal_path, JournalEvent::Copying)
        .map_err(|error| format!("无法持久化隔离复制状态: {error}"))?;

    let object = repository
        .create_object(record_id)
        .map_err(|error| format!("无法创建隔离对象: {error}"))?;
    let source_hash =
        match copy_source_to_object(&candidate.source_path, object, candidate.expected_size) {
            Ok(hash) => hash,
            Err(error) => {
                let _ = repository.remove_object(record_id);
                return Err(error);
            }
        };
    if let Err(error) = journal::append(&journal_path, JournalEvent::ObjectVerified) {
        let _ = repository.remove_object(record_id);
        return Err(format!("无法持久化对象校验状态，源文件已保留: {error}"));
    }

    let object_hash = match hash_file(&repository.object_path(record_id), candidate.expected_size) {
        Ok(hash) => hash,
        Err(error) => {
            let _ = repository.remove_object(record_id);
            return Err(error);
        }
    };
    if object_hash != source_hash {
        let _ = repository.remove_object(record_id);
        return Err("隔离对象复读哈希不一致，源文件已保留".into());
    }

    let manifest = QuarantineManifest {
        schema_version: QUARANTINE_SCHEMA_VERSION,
        protocol: QUARANTINE_PROTOCOL.into(),
        record_id,
        file_name: candidate.file_name,
        rule_id: candidate.rule_id,
        plan_id: candidate.plan_id,
        created_at_ms: audit::unix_time_ms(),
        size_bytes: candidate.expected_size,
        sha256: source_hash.clone(),
        object_name: format!("{record_id}.blob"),
    };
    if let Err(error) = repository.write_manifest(&manifest) {
        let _ = repository.remove_object(record_id);
        return Err(format!("无法提交隔离清单，源文件已保留: {error}"));
    }
    journal::append(&journal_path, JournalEvent::ObjectCommitted)
        .map_err(|error| format!("无法提交隔离日志，源文件已保留: {error}"))?;

    if let Err(error) = validate_source(&candidate.source_path) {
        return mark_source_retained(
            repository,
            record_id,
            candidate.expected_size,
            format!("删除前身份复检失败，源文件已保留: {error}"),
        );
    }
    match hash_file(&candidate.source_path, candidate.expected_size) {
        Ok(current_hash) if current_hash == source_hash => {}
        Ok(_) => {
            return mark_source_retained(
                repository,
                record_id,
                candidate.expected_size,
                "删除前内容哈希已变化，源文件已保留".into(),
            );
        }
        Err(error) => {
            return mark_source_retained(
                repository,
                record_id,
                candidate.expected_size,
                format!("删除前内容复检失败，源文件已保留: {error}"),
            );
        }
    }
    if let Err(error) = validate_source(&candidate.source_path) {
        return mark_source_retained(
            repository,
            record_id,
            candidate.expected_size,
            format!("删除前最终身份复检失败，源文件已保留: {error}"),
        );
    }

    journal::append(&journal_path, JournalEvent::SourceDeletePrepared)
        .map_err(|error| format!("无法持久化删除准备状态，源文件已保留: {error}"))?;
    if let Err(delete_error) = remove_source(&candidate.source_path) {
        return match verify_retained_source(
            &candidate.source_path,
            candidate.expected_size,
            &source_hash,
            &mut validate_source,
        ) {
            Ok(()) => mark_source_retained(
                repository,
                record_id,
                candidate.expected_size,
                format!(
                    "隔离对象已提交；源文件删除失败，经身份与内容复检确认原文件仍被保留: {delete_error}"
                ),
            ),
            Err(validation_error) => Ok(StageResult {
                record_id: record_id.to_string(),
                size_bytes: candidate.expected_size,
                source_retained: false,
                recovery_required: true,
                detail: Some(format!(
                    "源文件删除结果无法确认，记录已转入恢复必需状态；未追加 SourceRetained: 删除错误: {delete_error}; 后置复检: {validation_error}"
                )),
            }),
        };
    }
    journal::append(&journal_path, JournalEvent::Committed)
        .map_err(|error| format!("源文件已移除，但隔离终态持久化失败: {error}"))?;

    Ok(StageResult {
        record_id: record_id.to_string(),
        size_bytes: candidate.expected_size,
        source_retained: false,
        recovery_required: false,
        detail: None,
    })
}

pub(crate) fn stage_file<F>(
    repository: &Repository,
    candidate: QuarantineCandidate,
    validate_source: F,
) -> Result<StageResult, String>
where
    F: FnMut(&Path) -> Result<(), String>,
{
    stage_file_with_remover(repository, candidate, validate_source, |path| {
        fs::remove_file(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::{reconcile, types::QuarantineRecordState};
    use std::path::PathBuf;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("qingpan-stage-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn successful_stage_commits_object_before_removing_source() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source.tmp");
        fs::write(&source, b"recoverable").unwrap();
        let repository = Repository::new(directory.0.join("repository"));
        let candidate = QuarantineCandidate {
            source_path: source.clone(),
            file_name: "source.tmp".into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            expected_size: 11,
        };
        let result = stage_file(&repository, candidate, |path| {
            path.is_file()
                .then_some(())
                .ok_or_else(|| "missing".to_string())
        })
        .expect("stage should succeed");
        assert!(!source.exists());
        assert!(!result.source_retained);
        assert!(!result.recovery_required);
        assert!(repository
            .object_path(Uuid::parse_str(&result.record_id).unwrap())
            .is_file());
    }

    #[test]
    fn failed_final_validation_retains_source_and_committed_object() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source.tmp");
        fs::write(&source, b"recoverable").unwrap();
        let repository = Repository::new(directory.0.join("repository"));
        let candidate = QuarantineCandidate {
            source_path: source.clone(),
            file_name: "source.tmp".into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            expected_size: 11,
        };
        let mut validations = 0usize;
        let result = stage_file(&repository, candidate, |_| {
            validations += 1;
            if validations > 1 {
                Err("changed".into())
            } else {
                Ok(())
            }
        })
        .expect("retained source is a recorded outcome");
        assert!(source.is_file());
        assert!(result.source_retained);
        assert!(!result.recovery_required);
        assert!(repository
            .object_path(Uuid::parse_str(&result.record_id).unwrap())
            .is_file());
    }

    #[test]
    fn delete_error_marks_source_retained_only_after_post_delete_proof() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source.tmp");
        fs::write(&source, b"recoverable").unwrap();
        let repository = Repository::new(directory.0.join("repository"));
        let candidate = QuarantineCandidate {
            source_path: source.clone(),
            file_name: "source.tmp".into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            expected_size: 11,
        };
        let mut validations = 0usize;

        let result = stage_file_with_remover(
            &repository,
            candidate,
            |path| {
                validations += 1;
                path.is_file()
                    .then_some(())
                    .ok_or_else(|| "missing".to_string())
            },
            |_| Err(io::Error::new(io::ErrorKind::PermissionDenied, "simulated")),
        )
        .expect("a proven retained source is a recorded outcome");

        assert_eq!(validations, 5);
        assert!(source.is_file());
        assert!(result.source_retained);
        assert!(!result.recovery_required);
        let record_id = Uuid::parse_str(&result.record_id).unwrap();
        let entries = journal::read(&repository.journal_path(record_id)).unwrap();
        assert_eq!(entries.last().unwrap().event, JournalEvent::SourceRetained);
        assert_eq!(
            reconcile::derive_state(&entries).unwrap(),
            QuarantineRecordState::SourceRetained
        );
    }

    #[test]
    fn delete_error_without_post_delete_proof_requires_recovery() {
        let directory = TestDirectory::new();
        let source = directory.0.join("source.tmp");
        fs::write(&source, b"recoverable").unwrap();
        let repository = Repository::new(directory.0.join("repository"));
        let candidate = QuarantineCandidate {
            source_path: source.clone(),
            file_name: "source.tmp".into(),
            rule_id: "temp".into(),
            plan_id: Uuid::new_v4().to_string(),
            expected_size: 11,
        };
        let mut validations = 0usize;

        let result = stage_file_with_remover(
            &repository,
            candidate,
            |_| {
                validations += 1;
                if validations >= 4 {
                    Err("post-delete identity unavailable".into())
                } else {
                    Ok(())
                }
            },
            |_| Err(io::Error::other("simulated ambiguous deletion result")),
        )
        .expect("an uncertain deletion is persisted as recovery required");

        assert_eq!(validations, 4);
        assert!(source.is_file());
        assert!(!result.source_retained);
        assert!(result.recovery_required);
        assert!(repository
            .object_path(Uuid::parse_str(&result.record_id).unwrap())
            .is_file());
        let record_id = Uuid::parse_str(&result.record_id).unwrap();
        let entries = journal::read(&repository.journal_path(record_id)).unwrap();
        assert_eq!(
            entries.last().unwrap().event,
            JournalEvent::SourceDeletePrepared
        );
        assert_eq!(
            reconcile::derive_state(&entries).unwrap(),
            QuarantineRecordState::RecoveryRequired
        );
    }
}
