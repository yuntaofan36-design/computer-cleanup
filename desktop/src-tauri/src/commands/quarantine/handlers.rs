use crate::quarantine::{ExportQuarantineCopyResult, QuarantineListResponse};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub(crate) fn list_quarantine_preview(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<QuarantineListResponse, String> {
    state.quarantine.list(limit)
}

#[tauri::command]
pub(crate) fn export_quarantine_copy_preview(
    record_id: String,
    state: State<'_, AppState>,
) -> Result<ExportQuarantineCopyResult, String> {
    state.quarantine.export_copy(&record_id)
}
