mod journal;
mod reconcile;
mod repository;
mod restore;
mod staging;
mod types;

pub(crate) use types::{
    ExportQuarantineCopyResult, QuarantineCandidate, QuarantineListResponse, StageResult,
};

use parking_lot::Mutex;
use repository::Repository;
use std::env;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) const MAX_QUARANTINE_PLAN_FILES: usize = 100;
pub(crate) const MAX_QUARANTINE_PLAN_BYTES: u64 = 1024 * 1024 * 1024;

pub(crate) struct QuarantineService {
    repository: Option<Repository>,
    operation_lock: Mutex<()>,
}

impl Default for QuarantineService {
    fn default() -> Self {
        Self {
            repository: default_repository_root().map(Repository::new),
            operation_lock: Mutex::new(()),
        }
    }
}

impl QuarantineService {
    fn repository(&self) -> Result<&Repository, String> {
        self.repository
            .as_ref()
            .ok_or_else(|| "LOCALAPPDATA 不可用，实验性隔离功能已安全关闭".to_string())
    }

    pub(crate) fn stage_file<F>(
        &self,
        candidate: QuarantineCandidate,
        validate_source: F,
    ) -> Result<StageResult, String>
    where
        F: FnMut(&Path) -> Result<(), String>,
    {
        let _guard = self.operation_lock.lock();
        staging::stage_file(self.repository()?, candidate, validate_source)
    }

    pub(crate) fn list(&self, limit: usize) -> Result<QuarantineListResponse, String> {
        let _guard = self.operation_lock.lock();
        self.repository()?
            .list(limit.clamp(1, 500))
            .map_err(|error| format!("无法读取隔离仓库: {error}"))
    }

    pub(crate) fn export_copy(
        &self,
        record_id: &str,
    ) -> Result<ExportQuarantineCopyResult, String> {
        let record_id =
            Uuid::parse_str(record_id).map_err(|_| "隔离记录 ID 格式无效".to_string())?;
        let _guard = self.operation_lock.lock();
        restore::export_copy(self.repository()?, record_id)
    }
}

fn default_repository_root() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs::data_local_dir)
        .map(|root| root.join("Qingpan").join("quarantine-preview-v1"))
}
