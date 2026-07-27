use crate::{models::CleanupItem, scanner::CleanupSnapshot};
use serde::{Deserialize, Serialize};

pub(crate) const CLEANUP_RULE_VERSION: &str = "cleanup-rules-v4";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupScanResponse {
    pub scan_id: String,
    pub rule_version: String,
    pub expires_at_ms: u64,
    pub items: Vec<CleanupItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateCleanupPlanRequest {
    pub scan_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupPlanPreview {
    pub plan_id: String,
    pub scan_id: String,
    pub rule_version: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub items: Vec<CleanupItem>,
    pub total_items: usize,
    pub total_files: usize,
    pub total_bytes: u64,
    pub irreversible_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteCleanupPlanRequest {
    pub plan_id: String,
    pub confirmed: bool,
    #[serde(default)]
    pub confirmed_irreversible_item_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CleanupPlan {
    pub preview: CleanupPlanPreview,
    pub snapshots: Vec<CleanupSnapshot>,
}
