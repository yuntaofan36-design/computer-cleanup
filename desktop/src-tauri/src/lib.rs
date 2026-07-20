mod apps;
mod audit;
mod models;
mod scanner;
mod storage;

use models::*;
use parking_lot::Mutex;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sysinfo::Disks;
use tauri::State;
use uuid::Uuid;

const CLEANUP_RULE_VERSION: &str = "cleanup-rules-v2";
const MAX_CONCURRENT_READ_TASKS: usize = 3;

#[derive(Default)]
struct AppState {
    scanned: Mutex<HashMap<String, scanner::CleanupSnapshot>>,
    tasks: Mutex<HashMap<String, Arc<AtomicBool>>>,
    installed_apps: Mutex<apps::InstalledAppSnapshot>,
}

impl AppState {
    fn begin_task(&self, task_id: &str) -> Result<Arc<AtomicBool>, String> {
        Uuid::parse_str(task_id).map_err(|_| "任务 ID 格式无效".to_string())?;
        let mut tasks = self.tasks.lock();
        if tasks.len() >= MAX_CONCURRENT_READ_TASKS {
            return Err("已有过多只读扫描任务，请等待或取消后重试".into());
        }
        if tasks.contains_key(task_id) {
            return Err("任务 ID 已在使用".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        tasks.insert(task_id.to_string(), Arc::clone(&cancel));
        Ok(cancel)
    }

    fn finish_task(&self, task_id: &str) {
        self.tasks.lock().remove(task_id);
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeStorageRequest {
    task_id: String,
    root: String,
    #[serde(default)]
    options: storage::DirectoryScanOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LargeFileScanRequest {
    task_id: String,
    root: String,
    #[serde(default)]
    options: storage::LargeFileScanOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateFileScanRequest {
    task_id: String,
    root: String,
    #[serde(default)]
    options: storage::DuplicateScanOptions,
}

#[tauri::command]
fn list_disks() -> Vec<DiskInfo> {
    Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|disk| DiskInfo {
            id: disk.mount_point().display().to_string(),
            name: disk.name().to_string_lossy().into(),
            mount: disk.mount_point().display().to_string(),
            total_bytes: disk.total_space(),
            free_bytes: disk.available_space(),
        })
        .collect()
}

#[tauri::command]
fn scan_cleanup(state: State<AppState>) -> Vec<CleanupItem> {
    let snapshots = scanner::scan();
    let items = snapshots
        .iter()
        .map(|snapshot| snapshot.item().clone())
        .collect::<Vec<_>>();
    let mut known = state.scanned.lock();
    known.clear();
    for snapshot in snapshots {
        known.insert(snapshot.item().id.clone(), snapshot);
    }
    items
}

#[tauri::command]
fn execute_cleanup(
    request: ExecuteRequest,
    state: State<AppState>,
) -> Result<ExecuteResult, String> {
    if !request.confirmed {
        return Err("必须确认清理操作".into());
    }
    if request.item_ids.is_empty() || request.item_ids.len() > 100 {
        return Err("清理计划必须包含 1 到 100 个规则条目".into());
    }

    let mut record =
        audit::OperationRecord::new(audit::OperationKind::Cleanup, CLEANUP_RULE_VERSION);
    let mut result = ExecuteResult {
        reclaimed_bytes: 0,
        succeeded: 0,
        failed: Vec::new(),
    };
    let mut seen = HashSet::new();
    let mut queued = Vec::new();

    {
        let mut known = state.scanned.lock();
        for id in request.item_ids {
            if !seen.insert(id.clone()) {
                let error = "清理计划包含重复条目".to_string();
                result.failed.push(ItemFailure {
                    id: id.clone(),
                    error: error.clone(),
                    path: None,
                });
                record.failed.push(audit::OperationDetail {
                    item_id: id,
                    path: None,
                    bytes: 0,
                    detail: error,
                });
                continue;
            }
            match known.remove(&id) {
                Some(snapshot) => queued.push((id, snapshot)),
                None => {
                    let error = "条目不属于最近一次扫描或计划已执行".to_string();
                    result.failed.push(ItemFailure {
                        id: id.clone(),
                        error: error.clone(),
                        path: None,
                    });
                    record.failed.push(audit::OperationDetail {
                        item_id: id,
                        path: None,
                        bytes: 0,
                        detail: error,
                    });
                }
            }
        }
    }

    for (id, snapshot) in queued {
        let outcome = scanner::execute(&snapshot);
        result.reclaimed_bytes = result
            .reclaimed_bytes
            .saturating_add(outcome.reclaimed_bytes);
        if outcome.reclaimed_bytes > 0 || outcome.failures.is_empty() {
            record.succeeded.push(audit::OperationDetail {
                item_id: id.clone(),
                path: None,
                bytes: outcome.reclaimed_bytes,
                detail: "已按扫描快照逐文件复检并处理".into(),
            });
        }
        if outcome.failures.is_empty() {
            result.succeeded += 1;
        } else {
            for failure in outcome.failures {
                let error = failure.error;
                result.failed.push(ItemFailure {
                    id: id.clone(),
                    error: error.clone(),
                    path: Some(failure.path.display().to_string()),
                });
                record.skipped.push(audit::OperationDetail {
                    item_id: id.clone(),
                    path: None,
                    bytes: 0,
                    detail: error,
                });
            }
        }
    }

    record.reclaimed_bytes = result.reclaimed_bytes;
    record.completed_at_ms = audit::unix_time_ms();
    record.status = audit_status(&record);
    persist_audit(&record);
    Ok(result)
}

fn audit_status(record: &audit::OperationRecord) -> audit::OperationStatus {
    if !record.failed.is_empty() {
        if record.succeeded.is_empty() && record.skipped.is_empty() {
            audit::OperationStatus::Failed
        } else {
            audit::OperationStatus::PartiallySucceeded
        }
    } else if !record.skipped.is_empty() {
        if record.succeeded.is_empty() {
            audit::OperationStatus::Skipped
        } else {
            audit::OperationStatus::PartiallySucceeded
        }
    } else {
        audit::OperationStatus::Succeeded
    }
}

fn persist_audit(record: &audit::OperationRecord) {
    let outcome = audit::default_path().and_then(|path| audit::append_record(path, record));
    if let Err(error) = outcome {
        eprintln!("failed to persist local audit record: {error}");
    }
}

#[tauri::command]
fn list_operation_records(limit: usize) -> Result<audit::RecentRecords, String> {
    let path = audit::default_path().map_err(|error| error.to_string())?;
    audit::read_recent(path, limit.clamp(1, 500)).map_err(|error| error.to_string())
}

#[tauri::command]
async fn analyze_storage(
    request: AnalyzeStorageRequest,
    state: State<'_, AppState>,
) -> Result<storage::StorageAnalysisResult, String> {
    let cancel = state.begin_task(&request.task_id)?;
    let task_id = request.task_id;
    let root = PathBuf::from(request.root);
    let options = request.options;
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        storage::scan_directory_usage(&root, options, &cancel)
    })
    .await
    .map_err(|error| format!("目录分析任务异常结束: {error}"));
    state.finish_task(&task_id);
    outcome?
}

#[tauri::command]
async fn scan_large_files(
    request: LargeFileScanRequest,
    state: State<'_, AppState>,
) -> Result<storage::LargeFileScanResult, String> {
    let cancel = state.begin_task(&request.task_id)?;
    let task_id = request.task_id;
    let root = PathBuf::from(request.root);
    let options = request.options;
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        storage::scan_large_files(&root, options, &cancel)
    })
    .await
    .map_err(|error| format!("大文件扫描任务异常结束: {error}"));
    state.finish_task(&task_id);
    outcome?
}

#[tauri::command]
async fn scan_duplicate_files(
    request: DuplicateFileScanRequest,
    state: State<'_, AppState>,
) -> Result<storage::DuplicateScanResult, String> {
    let cancel = state.begin_task(&request.task_id)?;
    let task_id = request.task_id;
    let root = PathBuf::from(request.root);
    let options = request.options;
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        storage::scan_duplicates(&root, options, &cancel)
    })
    .await
    .map_err(|error| format!("重复文件扫描任务异常结束: {error}"));
    state.finish_task(&task_id);
    outcome?
}

#[tauri::command]
fn cancel_task(task_id: String, state: State<AppState>) -> bool {
    let Some(cancel) = state.tasks.lock().get(&task_id).cloned() else {
        return false;
    };
    cancel.store(true, Ordering::Relaxed);
    true
}

#[tauri::command]
fn list_apps(state: State<AppState>) -> Vec<AppEntry> {
    let snapshot = apps::enumerate();
    let entries = snapshot.entries();
    *state.installed_apps.lock() = snapshot;
    entries
}

#[tauri::command]
async fn get_app_icon(id: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let request = {
        let snapshot = state.installed_apps.lock();
        snapshot.icon_request(&id)?
    };

    tauri::async_runtime::spawn_blocking(move || apps::load_app_icon(request))
        .await
        .map_err(|error| format!("应用图标任务异常结束: {error}"))
}

#[tauri::command]
fn uninstall_app(
    id: String,
    confirmed: bool,
    state: State<AppState>,
) -> Result<apps::LaunchResult, String> {
    if !confirmed {
        return Err("必须确认后才能启动官方卸载器".into());
    }

    let started_at = audit::unix_time_ms();
    let request = {
        let snapshot = state.installed_apps.lock();
        snapshot.uninstall_request(&id)
    };
    let outcome = request.and_then(apps::launch_prepared_uninstaller);
    let mut record = audit::OperationRecord::new(audit::OperationKind::Uninstall, "uninstall-v1");
    record.started_at_ms = started_at;
    record.completed_at_ms = audit::unix_time_ms();
    match &outcome {
        Ok(result) => {
            record.status = audit::OperationStatus::Succeeded;
            record.succeeded.push(audit::OperationDetail {
                item_id: id,
                path: None,
                bytes: 0,
                detail: format!("已启动注册的官方卸载器，进程 ID {}", result.pid),
            });
        }
        Err(error) => {
            record.status = audit::OperationStatus::Failed;
            record.failed.push(audit::OperationDetail {
                item_id: id,
                path: None,
                bytes: 0,
                detail: error.clone(),
            });
        }
    }
    persist_audit(&record);
    outcome
}

#[tauri::command]
fn list_startup_entries() -> Vec<StartupEntry> {
    startup_platform::startups()
}

#[tauri::command]
fn set_startup_enabled(_id: String, _enabled: bool, _confirmed: bool) -> Result<(), String> {
    Err("启动项修改仅在 Windows 构建中可用".into())
}

#[cfg(windows)]
#[tauri::command]
fn reveal_in_explorer(path: String) -> Result<(), String> {
    use std::fs;
    use std::process::Command;

    if path.is_empty() || path.len() > 32_767 {
        return Err("文件路径无效".into());
    }
    let target = PathBuf::from(path);
    let metadata =
        fs::symlink_metadata(&target).map_err(|error| format!("路径不可访问: {error}"))?;
    let mut command = Command::new("explorer.exe");
    if metadata.is_dir() {
        command.arg(&target);
    } else {
        command.arg("/select,").arg(&target);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开文件资源管理器: {error}"))
}

#[cfg(not(windows))]
#[tauri::command]
fn reveal_in_explorer(_path: String) -> Result<(), String> {
    Err("文件资源管理器定位仅支持 Windows".into())
}

#[cfg(not(windows))]
mod startup_platform {
    use super::*;
    pub fn startups() -> Vec<StartupEntry> {
        Vec::new()
    }
}

#[cfg(windows)]
mod startup_platform {
    use super::*;
    use winreg::{enums::*, RegKey};

    pub fn startups() -> Vec<StartupEntry> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(root) = hkcu.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run") else {
            return Vec::new();
        };
        root.enum_values()
            .filter_map(Result::ok)
            .map(|(name, value)| StartupEntry {
                id: format!("hkcu:{name}"),
                name,
                publisher: String::new(),
                command: String::from_utf8_lossy(&value.bytes)
                    .trim_matches(char::from(0))
                    .into(),
                enabled: true,
                impact: "未知".into(),
                scope: "当前用户".into(),
            })
            .collect()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_disks,
            scan_cleanup,
            execute_cleanup,
            list_operation_records,
            analyze_storage,
            scan_large_files,
            scan_duplicate_files,
            cancel_task,
            list_apps,
            get_app_icon,
            uninstall_app,
            list_startup_entries,
            set_startup_enabled,
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Qingpan")
}
