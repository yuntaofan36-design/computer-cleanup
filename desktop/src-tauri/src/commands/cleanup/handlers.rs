use crate::{
    cleanup_plan::{
        self, CleanupPlanPreview, CleanupScanResponse, CreateCleanupPlanRequest,
        ExecuteCleanupPlanRequest, CLEANUP_RULE_VERSION,
    },
    models::ExecuteResult,
    scanner, AppState,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn scan_cleanup_v2(state: State<'_, AppState>) -> Result<CleanupScanResponse, String> {
    state
        .cleanup_plans
        .record_scan(CLEANUP_RULE_VERSION, scanner::scan())
}

#[tauri::command]
pub(crate) fn create_cleanup_plan(
    request: CreateCleanupPlanRequest,
    state: State<'_, AppState>,
) -> Result<CleanupPlanPreview, String> {
    state.cleanup_plans.create_plan(request)
}

#[tauri::command]
pub(crate) fn execute_cleanup_plan(
    request: ExecuteCleanupPlanRequest,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ExecuteResult, String> {
    let plan = state.cleanup_plans.take_confirmed_plan(request)?;
    Ok(cleanup_plan::execute(plan, &state.quarantine, &app))
}
