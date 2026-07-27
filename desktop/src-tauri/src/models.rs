use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub id: String,
    pub name: String,
    pub mount: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    pub category: String,
    pub name: String,
    pub path: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    pub size_bytes: u64,
    pub file_count: usize,
    pub risk: RiskLevel,
    pub delete_mode: DeleteMode,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteMode {
    Permanent,
    RecycleBin,
    Quarantine,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteResult {
    pub reclaimed_bytes: u64,
    pub staged_bytes: u64,
    pub succeeded: usize,
    pub failed: Vec<ItemFailure>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProgress {
    pub phase: String,
    pub completed_items: usize,
    pub total_items: usize,
    pub completed_files: usize,
    pub total_files: usize,
    pub current_item_id: String,
    pub current_item_name: String,
    pub current_path: String,
    pub reclaimed_bytes: u64,
    pub failed_files: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileDeleteRequest {
    pub item_ids: Vec<String>,
    pub confirmed_permanent: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileDeleteResult {
    pub deleted_bytes: u64,
    pub succeeded_ids: Vec<String>,
    pub failed: Vec<ItemFailure>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileDeleteProgress {
    pub phase: String,
    pub completed: usize,
    pub total: usize,
    pub current_item_id: String,
    pub current_name: String,
    pub current_path: String,
    pub deleted_bytes: u64,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFailure {
    pub id: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub size_bytes: u64,
    pub installed_at: String,
    pub uninstallable: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub command: String,
    pub enabled: bool,
    pub impact: String,
    pub scope: String,
}
