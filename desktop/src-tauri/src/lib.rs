mod apps;
mod audit;
mod browsers;
mod capability_policy;
mod cleanup_plan;
mod commands;
mod fs_safety;
mod models;
mod partitions;
mod quarantine;
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
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

const LARGE_FILE_RULE_VERSION: &str = "large-file-snapshot-v1";
const MAX_CONCURRENT_READ_TASKS: usize = 3;

#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) cleanup_plans: cleanup_plan::CleanupPlanStore,
    pub(crate) quarantine: Arc<quarantine::QuarantineService>,
    large_files: Mutex<HashMap<String, storage::LargeFileSnapshot>>,
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

fn emit_large_file_delete_progress(app: &AppHandle, progress: LargeFileDeleteProgress) {
    let _ = app.emit("large-file-delete-progress", progress);
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
        storage::scan_large_files_with_snapshots(&root, options, &cancel)
    })
    .await
    .map_err(|error| format!("大文件扫描任务异常结束: {error}"));
    state.finish_task(&task_id);
    let (result, snapshots) = outcome??;
    if !result.stats.cancelled {
        *state.large_files.lock() = snapshots;
    }
    Ok(result)
}

#[tauri::command]
fn delete_large_files(
    request: LargeFileDeleteRequest,
    state: State<AppState>,
    app: AppHandle,
) -> Result<LargeFileDeleteResult, String> {
    capability_policy::require(
        capability_policy::DangerousWriteCapability::PermanentOriginalFileDelete,
    )?;
    if !request.confirmed_permanent {
        return Err("必须明确确认永久删除大文件".into());
    }
    if request.item_ids.is_empty() || request.item_ids.len() > 2_000 {
        return Err("大文件清理计划必须包含 1 到 2000 个条目".into());
    }

    let total = request.item_ids.len();
    let mut result = LargeFileDeleteResult {
        deleted_bytes: 0,
        succeeded_ids: Vec::new(),
        failed: Vec::new(),
    };
    let mut record =
        audit::OperationRecord::new(audit::OperationKind::Cleanup, LARGE_FILE_RULE_VERSION);
    let mut seen = HashSet::new();
    let mut queued = Vec::new();

    {
        let mut known = state.large_files.lock();
        for id in request.item_ids {
            if !seen.insert(id.clone()) {
                let error = "大文件清理计划包含重复条目".to_string();
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
                    let error = "条目不属于最近一次大文件扫描、受保护或计划已执行".to_string();
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

    let mut completed = total.saturating_sub(queued.len());
    emit_large_file_delete_progress(
        &app,
        LargeFileDeleteProgress {
            phase: "starting".into(),
            completed,
            total,
            current_item_id: String::new(),
            current_name: String::new(),
            current_path: String::new(),
            deleted_bytes: 0,
            failed: result.failed.len(),
        },
    );

    for (id, snapshot) in queued {
        let name = snapshot.entry().name.clone();
        let path = snapshot.entry().path.clone();
        emit_large_file_delete_progress(
            &app,
            LargeFileDeleteProgress {
                phase: "running".into(),
                completed,
                total,
                current_item_id: id.clone(),
                current_name: name.clone(),
                current_path: path.clone(),
                deleted_bytes: result.deleted_bytes,
                failed: result.failed.len(),
            },
        );

        match storage::delete_large_file(&snapshot) {
            Ok(bytes) => {
                result.deleted_bytes = result.deleted_bytes.saturating_add(bytes);
                result.succeeded_ids.push(id.clone());
                record.succeeded.push(audit::OperationDetail {
                    item_id: id.clone(),
                    path: Some(path.clone()),
                    bytes,
                    detail: "已按最近一次大文件扫描快照复检并永久删除".into(),
                });
            }
            Err(error) => {
                result.failed.push(ItemFailure {
                    id: id.clone(),
                    error: error.clone(),
                    path: Some(path.clone()),
                });
                record.skipped.push(audit::OperationDetail {
                    item_id: id.clone(),
                    path: Some(path.clone()),
                    bytes: 0,
                    detail: error,
                });
            }
        }
        completed = completed.saturating_add(1);
        emit_large_file_delete_progress(
            &app,
            LargeFileDeleteProgress {
                phase: "item_complete".into(),
                completed,
                total,
                current_item_id: id,
                current_name: name,
                current_path: path,
                deleted_bytes: result.deleted_bytes,
                failed: result.failed.len(),
            },
        );
    }

    emit_large_file_delete_progress(
        &app,
        LargeFileDeleteProgress {
            phase: "complete".into(),
            completed: total,
            total,
            current_item_id: String::new(),
            current_name: String::new(),
            current_path: String::new(),
            deleted_bytes: result.deleted_bytes,
            failed: result.failed.len(),
        },
    );

    record.reclaimed_bytes = result.deleted_bytes;
    record.completed_at_ms = audit::unix_time_ms();
    record.status = audit_status(&record);
    persist_audit(&record);
    Ok(result)
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
    capability_policy::require(
        capability_policy::DangerousWriteCapability::LegacyWin32UninstallLaunch,
    )?;
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

#[cfg(windows)]
#[tauri::command]
fn set_startup_enabled(id: String, enabled: bool, confirmed: bool) -> Result<(), String> {
    startup_platform::set_enabled(&id, enabled, confirmed)
}

#[cfg(not(windows))]
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
    use std::time::{SystemTime, UNIX_EPOCH};
    use winreg::{enums::*, types::FromRegValue, RegKey, RegValue};

    const RUN_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const STARTUP_APPROVED_RUN_KEY: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const STARTUP_ID_PREFIX: &str = "hkcu:";
    const MAX_REGISTRY_VALUE_NAME_UTF16_LEN: usize = 16_383;
    const STARTUP_APPROVED_ENABLED: u32 = 2;
    const STARTUP_APPROVED_DISABLED: u32 = 3;
    const WINDOWS_EPOCH_OFFSET_SECONDS: u64 = 11_644_473_600;

    pub fn startups() -> Vec<StartupEntry> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(root) = hkcu.open_subkey(RUN_KEY) else {
            return Vec::new();
        };
        let approved = hkcu.open_subkey(STARTUP_APPROVED_RUN_KEY).ok();

        root.enum_values()
            .filter_map(Result::ok)
            .map(|(name, value)| {
                let enabled = approved
                    .as_ref()
                    .and_then(|key| key.get_raw_value(&name).ok())
                    .map(|value| startup_approved_enabled(&value))
                    .unwrap_or(true);

                StartupEntry {
                    id: format!("{STARTUP_ID_PREFIX}{name}"),
                    name,
                    publisher: String::new(),
                    command: String::from_reg_value(&value).unwrap_or_default(),
                    enabled,
                    impact: "未知".into(),
                    scope: "当前用户".into(),
                }
            })
            .collect()
    }

    pub fn set_enabled(id: &str, enabled: bool, confirmed: bool) -> Result<(), String> {
        if !confirmed {
            return Err("必须确认后才能修改启动项".into());
        }

        let name = startup_value_name(id)?;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_QUERY_VALUE)
            .map_err(|error| format!("无法读取当前用户启动项: {error}"))?;
        let command = run
            .get_raw_value(name)
            .map_err(|_| "启动项不存在或已更改".to_string())?;
        if !matches!(command.vtype, REG_SZ | REG_EXPAND_SZ) {
            return Err("启动项注册表值类型不受支持".into());
        }

        let (approved, _) = hkcu
            .create_subkey_with_flags(STARTUP_APPROVED_RUN_KEY, KEY_QUERY_VALUE | KEY_SET_VALUE)
            .map_err(|error| format!("无法打开启动项状态注册表: {error}"))?;
        let previous = match approved.get_raw_value(name) {
            Ok(value) if value.vtype == REG_BINARY => Some(value),
            Ok(_) => return Err("启动项状态注册表值类型不受支持".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("无法读取启动项状态: {error}")),
        };
        approved
            .set_raw_value(name, &startup_approved_value(previous, enabled))
            .map_err(|error| format!("无法更新启动项状态: {error}"))
    }

    fn startup_value_name(id: &str) -> Result<&str, String> {
        let Some(name) = id.strip_prefix(STARTUP_ID_PREFIX) else {
            return Err("启动项 ID 格式无效".into());
        };
        if name.is_empty()
            || name.contains('\0')
            || name.encode_utf16().count() > MAX_REGISTRY_VALUE_NAME_UTF16_LEN
        {
            return Err("启动项 ID 格式无效".into());
        }
        Ok(name)
    }

    fn startup_approved_enabled(value: &RegValue) -> bool {
        if value.vtype != REG_BINARY || value.bytes.len() < std::mem::size_of::<u32>() {
            return true;
        }
        let state = u32::from_le_bytes(value.bytes[..4].try_into().expect("checked length"));
        state != STARTUP_APPROVED_DISABLED
    }

    fn startup_approved_value(previous: Option<RegValue>, enabled: bool) -> RegValue {
        let mut bytes = previous.map(|value| value.bytes).unwrap_or_default();
        if bytes.len() < 12 {
            bytes.resize(12, 0);
        }

        let state = if enabled {
            STARTUP_APPROVED_ENABLED
        } else {
            STARTUP_APPROVED_DISABLED
        };
        bytes[..4].copy_from_slice(&state.to_le_bytes());
        let changed_at = if enabled { 0 } else { windows_filetime_now() };
        bytes[4..12].copy_from_slice(&changed_at.to_le_bytes());

        RegValue {
            bytes,
            vtype: REG_BINARY,
        }
    }

    fn windows_filetime_now() -> u64 {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        elapsed
            .as_secs()
            .saturating_add(WINDOWS_EPOCH_OFFSET_SECONDS)
            .saturating_mul(10_000_000)
            .saturating_add(u64::from(elapsed.subsec_nanos()) / 100)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn startup_ids_are_scoped_and_reject_empty_or_nul_names() {
            assert_eq!(startup_value_name("hkcu:OneDrive"), Ok("OneDrive"));
            assert!(startup_value_name("OneDrive").is_err());
            assert!(startup_value_name("hkcu:").is_err());
            assert!(startup_value_name("hkcu:bad\0name").is_err());
        }

        #[test]
        fn startup_approved_values_round_trip_without_touching_the_run_command() {
            let disabled = startup_approved_value(None, false);
            assert_eq!(disabled.vtype, REG_BINARY);
            assert_eq!(disabled.bytes.len(), 12);
            assert!(!startup_approved_enabled(&disabled));
            assert_ne!(&disabled.bytes[4..12], &[0; 8]);

            let enabled = startup_approved_value(Some(disabled), true);
            assert!(startup_approved_enabled(&enabled));
            assert_eq!(&enabled.bytes[4..12], &[0; 8]);
        }

        #[test]
        fn malformed_or_unknown_approved_state_fails_open_as_enabled() {
            let malformed = RegValue {
                bytes: vec![STARTUP_APPROVED_DISABLED as u8],
                vtype: REG_BINARY,
            };
            let unknown = RegValue {
                bytes: 0_u32.to_le_bytes().to_vec(),
                vtype: REG_BINARY,
            };
            assert!(startup_approved_enabled(&malformed));
            assert!(startup_approved_enabled(&unknown));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_disks,
            partitions::list_partition_disks,
            partitions::open_windows_disk_management,
            commands::cleanup::handlers::scan_cleanup_v2,
            commands::cleanup::handlers::create_cleanup_plan,
            commands::cleanup::handlers::execute_cleanup_plan,
            commands::quarantine::list_quarantine_preview,
            commands::quarantine::export_quarantine_copy_preview,
            list_operation_records,
            analyze_storage,
            scan_large_files,
            delete_large_files,
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
        .expect("failed to run Lumina Clean")
}
