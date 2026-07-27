use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub(crate) const QUARANTINE_SCHEMA_VERSION: u32 = 1;
pub(crate) const QUARANTINE_PROTOCOL: &str = "quarantine-preview-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum JournalEvent {
    Prepared,
    Copying,
    ObjectVerified,
    ObjectCommitted,
    SourceDeletePrepared,
    Committed,
    SourceRetained,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalEntry {
    pub schema_version: u32,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_entry_sha256: Option<String>,
    pub occurred_at_ms: u64,
    pub event: JournalEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuarantineManifest {
    pub schema_version: u32,
    pub protocol: String,
    pub record_id: Uuid,
    pub file_name: String,
    pub rule_id: String,
    pub plan_id: String,
    pub created_at_ms: u64,
    pub size_bytes: u64,
    pub sha256: String,
    pub object_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum QuarantineRecordState {
    Committed,
    SourceRetained,
    RecoveryRequired,
    Damaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuarantineRecord {
    pub record_id: String,
    pub file_name: String,
    pub rule_id: String,
    pub plan_id: String,
    pub created_at_ms: u64,
    pub size_bytes: u64,
    pub state: QuarantineRecordState,
    pub exportable: bool,
    pub source_retained: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuarantineListResponse {
    pub records: Vec<QuarantineRecord>,
    pub corrupt_records: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportQuarantineCopyResult {
    pub operation_id: String,
    pub record_id: String,
    pub exported_directory: String,
    pub exported_file_name: String,
    pub bytes: u64,
    pub quarantine_source_retained: bool,
    pub audit_persisted: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct QuarantineCandidate {
    pub source_path: PathBuf,
    pub file_name: String,
    pub rule_id: String,
    pub plan_id: String,
    pub expected_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageResult {
    pub record_id: String,
    pub size_bytes: u64,
    pub source_retained: bool,
    pub recovery_required: bool,
    pub detail: Option<String>,
}
