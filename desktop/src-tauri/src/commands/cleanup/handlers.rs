use crate::{
    cleanup_plan::{
        self, CleanupPlanPreview, CleanupScanRequest, CleanupScanResponse,
        CreateCleanupPlanRequest, ExecuteCleanupPlanRequest, CLEANUP_RULE_VERSION,
    },
    models::ExecuteResult,
    scanner, AppState,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) async fn scan_cleanup_v2(
    request: CleanupScanRequest,
    state: State<'_, AppState>,
) -> Result<CleanupScanResponse, String> {
    let cancel = state.begin_task(&request.task_id)?;
    let task_id = request.task_id;
    let outcome = tauri::async_runtime::spawn_blocking(move || scanner::scan(&cancel)).await;
    state.finish_task(&task_id);

    let snapshots = outcome
        .map_err(|error| format!("清理扫描任务异常结束: {error}"))?
        .ok_or_else(|| "清理扫描已取消".to_string())?;

    state
        .cleanup_plans
        .record_scan(CLEANUP_RULE_VERSION, snapshots)
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
