mod models;
mod scanner;
use models::*;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use sysinfo::Disks;
use tauri::State;
#[derive(Default)]
struct AppState {
    scanned: Mutex<HashMap<String, CleanupItem>>,
    cancelled: Mutex<HashSet<String>>,
}
#[tauri::command]
fn list_disks() -> Vec<DiskInfo> {
    Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|d| DiskInfo {
            id: d.mount_point().display().to_string(),
            name: d.name().to_string_lossy().into(),
            mount: d.mount_point().display().to_string(),
            total_bytes: d.total_space(),
            free_bytes: d.available_space(),
        })
        .collect()
}
#[tauri::command]
fn scan_cleanup(state: State<AppState>) -> Vec<CleanupItem> {
    let items = scanner::scan();
    let mut known = state.scanned.lock();
    known.clear();
    for item in &items {
        known.insert(item.id.clone(), item.clone());
    }
    items
}
#[tauri::command]
fn cancel_task(task_id: String, state: State<AppState>) {
    state.cancelled.lock().insert(task_id);
}
#[tauri::command]
fn execute_cleanup(
    request: ExecuteRequest,
    state: State<AppState>,
) -> Result<ExecuteResult, String> {
    if !request.confirmed {
        return Err("必须确认清理操作".into());
    }
    let known = state.scanned.lock();
    let mut result = ExecuteResult {
        reclaimed_bytes: 0,
        succeeded: 0,
        failed: vec![],
    };
    for id in request.item_ids {
        if !known.contains_key(&id) {
            result.failed.push(ItemFailure {
                id,
                error: "条目不属于最近一次扫描".into(),
            });
            continue;
        }
        match scanner::validated_path(&id).and_then(|p| scanner::clear_contents(&p)) {
            Ok(bytes) => {
                result.reclaimed_bytes += bytes;
                result.succeeded += 1
            }
            Err(error) => result.failed.push(ItemFailure { id, error }),
        }
    }
    Ok(result)
}
#[tauri::command]
fn list_apps() -> Vec<AppEntry> {
    platform::apps()
}
#[tauri::command]
fn list_startup_entries() -> Vec<StartupEntry> {
    platform::startups()
}
#[tauri::command]
fn uninstall_app(_id: String, _confirmed: bool) -> Result<(), String> {
    Err("卸载功能仅在 Windows 构建中通过受控注册表标识执行".into())
}
#[tauri::command]
fn set_startup_enabled(_id: String, _enabled: bool, _confirmed: bool) -> Result<(), String> {
    Err("启动项修改仅在 Windows 构建中可用".into())
}
#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn apps() -> Vec<AppEntry> {
        vec![]
    }
    pub fn startups() -> Vec<StartupEntry> {
        vec![]
    }
}
#[cfg(windows)]
mod platform {
    use super::*;
    use winreg::{enums::*, RegKey};
    pub fn apps() -> Vec<AppEntry> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let paths = [
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ];
        let mut out = vec![];
        for path in paths {
            if let Ok(root) = hklm.open_subkey(path) {
                for key in root.enum_keys().filter_map(Result::ok) {
                    if let Ok(k) = root.open_subkey(&key) {
                        if let Ok(name) = k.get_value::<String, _>("DisplayName") {
                            let uninstallable = k.get_value::<String, _>("UninstallString").is_ok();
                            out.push(AppEntry {
                                id: key,
                                name,
                                publisher: k.get_value("Publisher").unwrap_or_default(),
                                version: k.get_value("DisplayVersion").unwrap_or_default(),
                                size_bytes: k.get_value::<u32, _>("EstimatedSize").unwrap_or(0)
                                    as u64
                                    * 1024,
                                installed_at: k.get_value("InstallDate").unwrap_or_default(),
                                uninstallable,
                            })
                        }
                    }
                }
            }
        }
        out
    }
    pub fn startups() -> Vec<StartupEntry> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(root) = hkcu.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run") else {
            return vec![];
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
            cancel_task,
            execute_cleanup,
            list_apps,
            list_startup_entries,
            uninstall_app,
            set_startup_enabled
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Qingpan")
}
