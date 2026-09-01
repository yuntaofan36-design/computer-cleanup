use crate::{
    capability_policy::{CapabilityPolicy, DangerousWriteCapability},
    models::AppEntry,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;

const ID_DOMAIN: &[u8] = b"qingpan-installed-app-v1\0";
const APP_ICON_SIZE: u32 = 48;
const MAX_ICON_DATA_URL_BYTES: usize = 256 * 1024;
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[derive(Clone, Debug, PartialEq, Eq)]
struct IconSource {
    path: String,
    index: i32,
}

#[derive(Clone)]
pub(crate) struct AppIconRequest {
    display_icon: Option<IconSource>,
    uninstall_command: Option<String>,
}

pub(crate) struct UninstallRequest {
    app_id: String,
    command: String,
}

/// A point-in-time view of installed applications.
///
/// The registry identity and uninstall command deliberately remain private. The
/// UI only receives the opaque IDs and display data returned by [`entries`].
/// Replacing this snapshot after every enumeration makes an old UI selection
/// unusable without ever accepting a command line from the UI.
#[derive(Default)]
pub struct InstalledAppSnapshot {
    records: HashMap<String, InstalledAppRecord>,
}

impl fmt::Debug for InstalledAppSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InstalledAppSnapshot")
            .field("record_count", &self.records.len())
            .finish_non_exhaustive()
    }
}

impl InstalledAppSnapshot {
    pub fn entries(&self) -> Vec<AppEntry> {
        let mut entries = self
            .records
            .values()
            .map(InstalledAppRecord::to_entry)
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        entries
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(crate) fn icon_request(&self, id: &str) -> Result<AppIconRequest, String> {
        let record = self
            .records
            .get(id)
            .ok_or_else(|| "应用不属于最近一次枚举快照".to_string())?;
        Ok(AppIconRequest {
            display_icon: record.icon_source.clone(),
            uninstall_command: record.uninstall_command.clone(),
        })
    }

    pub(crate) fn uninstall_request(&self, id: &str) -> Result<UninstallRequest, String> {
        let record = self
            .records
            .get(id)
            .ok_or_else(|| "应用不属于最近一次枚举快照".to_string())?;
        let command = record
            .uninstall_command
            .clone()
            .ok_or_else(|| "此应用未注册可用的官方卸载器".to_string())?;
        Ok(UninstallRequest {
            app_id: id.to_string(),
            command,
        })
    }
}

struct InstalledAppRecord {
    id: String,
    name: String,
    publisher: String,
    version: String,
    size_bytes: u64,
    installed_at: String,
    uninstall_command: Option<String>,
    icon_source: Option<IconSource>,
}

impl InstalledAppRecord {
    fn to_entry(&self) -> AppEntry {
        AppEntry {
            id: self.id.clone(),
            name: self.name.clone(),
            publisher: self.publisher.clone(),
            version: self.version.clone(),
            size_bytes: self.size_bytes,
            installed_at: self.installed_at.clone(),
            uninstallable: self
                .uninstall_command
                .as_deref()
                .is_some_and(command_is_launchable),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub app_id: String,
    pub pid: u32,
    pub status: String,
}

/// Enumerate the per-user and machine-wide 32-bit and 64-bit uninstall views.
///
/// Individual inaccessible or malformed keys are ignored. This is important on
/// managed Windows machines where seeing one protected entry must not hide all
/// other installed applications.
pub fn enumerate() -> InstalledAppSnapshot {
    platform::enumerate()
}

pub(crate) fn load_app_icon(request: AppIconRequest) -> Option<String> {
    platform::load_icon_png(&request).and_then(|png| png_data_url(&png))
}

pub(crate) fn load_startup_icon(command: &str) -> Option<String> {
    let source = command_icon_source(command)?;
    load_app_icon(AppIconRequest {
        display_icon: Some(source),
        uninstall_command: None,
    })
}

pub(crate) fn launch_prepared_uninstaller(
    request: UninstallRequest,
) -> Result<LaunchResult, String> {
    launch_prepared_uninstaller_with_policy(request, CapabilityPolicy::compiled())
}

fn launch_prepared_uninstaller_with_policy(
    request: UninstallRequest,
    policy: CapabilityPolicy,
) -> Result<LaunchResult, String> {
    policy.require(DangerousWriteCapability::LegacyWin32UninstallLaunch)?;
    platform::launch(&request.app_id, &request.command)
}

fn stable_app_id(hive: &str, view: &str, registry_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ID_DOMAIN);
    hasher.update(hive.as_bytes());
    hasher.update([0]);
    hasher.update(view.as_bytes());
    hasher.update([0]);
    hasher.update(registry_key.to_lowercase().as_bytes());
    let digest = hasher.finalize();

    let mut id = String::with_capacity(4 + digest.len() * 2);
    id.push_str("app_");
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(id, "{byte:02x}");
    }
    id
}

fn select_visible_uninstaller(
    uninstall_string: Option<String>,
    removal_blocked: bool,
) -> Option<String> {
    if removal_blocked {
        None
    } else {
        uninstall_string
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedCommand {
    executable: String,
    arguments: Vec<String>,
}

fn parse_uninstall_command(command: &str) -> Result<ParsedCommand, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("卸载命令为空".into());
    }
    if command.contains('\0') {
        return Err("卸载命令包含 NUL 字符".into());
    }

    let mut parts = parse_windows_arguments(command)?;
    if parts.is_empty() {
        return Err("卸载命令没有可执行文件".into());
    }

    // A few installers publish an unquoted absolute path containing spaces.
    // Windows' ambiguous executable search is unsafe, so recover only through
    // the first explicit `.exe` boundary and later require that exact file.
    if !command.starts_with('"') && parts.len() > 1 {
        if let Some(boundary) = unquoted_executable_boundary(command) {
            let candidate = command[..boundary].trim();
            if looks_like_windows_absolute_path(candidate) {
                let arguments = parse_windows_arguments(command[boundary..].trim_start())?;
                parts.clear();
                parts.push(candidate.to_string());
                parts.extend(arguments);
            }
        }
    }

    let executable = expand_environment_variables(&parts[0]);
    let arguments = parts[1..]
        .iter()
        .map(|argument| expand_environment_variables(argument))
        .collect();
    Ok(ParsedCommand {
        executable,
        arguments,
    })
}

/// Parse the Windows CRT command-line quoting rules without invoking a shell.
fn parse_windows_arguments(command: &str) -> Result<Vec<String>, String> {
    let chars = command.chars().collect::<Vec<_>>();
    let mut arguments = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        while index < chars.len() && is_command_whitespace(chars[index]) {
            index += 1;
        }
        if index == chars.len() {
            break;
        }

        let mut argument = String::new();
        let mut in_quotes = false;
        while index < chars.len() {
            if !in_quotes && is_command_whitespace(chars[index]) {
                break;
            }

            let mut backslashes = 0;
            while index < chars.len() && chars[index] == '\\' {
                backslashes += 1;
                index += 1;
            }

            if index < chars.len() && chars[index] == '"' {
                argument.extend(std::iter::repeat_n('\\', backslashes / 2));
                if backslashes % 2 == 1 {
                    argument.push('"');
                    index += 1;
                } else if in_quotes && index + 1 < chars.len() && chars[index + 1] == '"' {
                    argument.push('"');
                    index += 2;
                } else {
                    in_quotes = !in_quotes;
                    index += 1;
                }
                continue;
            }

            argument.extend(std::iter::repeat_n('\\', backslashes));
            if index == chars.len() || (!in_quotes && is_command_whitespace(chars[index])) {
                break;
            }
            argument.push(chars[index]);
            index += 1;
        }

        if in_quotes {
            return Err("卸载命令包含未闭合的引号".into());
        }
        arguments.push(argument);
    }

    Ok(arguments)
}

fn is_command_whitespace(character: char) -> bool {
    character == ' ' || character == '\t'
}

fn unquoted_executable_boundary(command: &str) -> Option<usize> {
    let lower = command.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut offset = 0;
    while let Some(relative) = lower[offset..].find(".exe") {
        let end = offset + relative + 4;
        if end == bytes.len() || bytes.get(end).is_some_and(u8::is_ascii_whitespace) {
            return Some(end);
        }
        offset = end;
    }
    None
}

fn looks_like_windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || path.starts_with(r"\\")
}

fn expand_environment_variables(value: &str) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(start) = remaining.find('%') {
        expanded.push_str(&remaining[..start]);
        let after_start = &remaining[start + 1..];
        let Some(relative_end) = after_start.find('%') else {
            expanded.push_str(&remaining[start..]);
            return expanded;
        };
        let name = &after_start[..relative_end];
        if name.is_empty() {
            expanded.push_str("%%");
        } else if let Some(replacement) = std::env::var_os(name) {
            expanded.push_str(&replacement.to_string_lossy());
        } else {
            expanded.push('%');
            expanded.push_str(name);
            expanded.push('%');
        }
        remaining = &after_start[relative_end + 1..];
    }
    expanded.push_str(remaining);
    expanded
}

fn parse_display_icon(value: &str) -> Option<IconSource> {
    let original = value.trim();
    if original.is_empty()
        || original.len() > 32_767
        || original.contains('\0')
        || original.starts_with("@{")
    {
        return None;
    }

    let location = original.strip_prefix('@').unwrap_or(original).trim_start();
    let (path, index) = if let Some(quoted) = location.strip_prefix('"') {
        let closing_quote = quoted.find('"')?;
        let path = &quoted[..closing_quote];
        let suffix = quoted[closing_quote + 1..].trim();
        let index = if suffix.is_empty() {
            0
        } else {
            suffix.strip_prefix(',')?.trim().parse::<i32>().ok()?
        };
        (path, index)
    } else if let Some((path, suffix)) = location.rsplit_once(',') {
        let suffix = suffix.trim();
        if suffix.is_empty() {
            return None;
        }
        match suffix.parse::<i32>() {
            Ok(index) => (path, index),
            Err(_) => (location, 0),
        }
    } else {
        (location, 0)
    };

    let path = expand_environment_variables(path.trim());
    let lower = path.to_ascii_lowercase();
    if path.is_empty()
        || path.len() > 32_767
        || path.contains('\0')
        || lower.starts_with("ms-resource:")
    {
        return None;
    }
    Some(IconSource { path, index })
}

fn command_icon_source(command: &str) -> Option<IconSource> {
    let parsed = parse_uninstall_command(command).ok()?;
    let path = parsed.executable.trim();
    if !looks_like_local_windows_absolute_path(path) {
        return None;
    }
    let file_name = path
        .rsplit(|character| matches!(character, '\\' | '/'))
        .next()
        .unwrap_or_default();
    if is_blocked_icon_fallback(file_name)
        || !file_name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    Some(IconSource {
        path: path.to_string(),
        index: 0,
    })
}

fn looks_like_local_windows_absolute_path(path: &str) -> bool {
    fn is_drive_absolute(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/')
    }

    if is_drive_absolute(path) {
        return true;
    }
    let bytes = path.as_bytes();
    bytes.len() > 4 && bytes[..4] == [b'\\', b'\\', b'?', b'\\'] && is_drive_absolute(&path[4..])
}

fn is_blocked_icon_fallback(file_name: &str) -> bool {
    matches!(
        file_name.to_ascii_lowercase().as_str(),
        "msiexec.exe"
            | "rundll32.exe"
            | "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
            | "mshta.exe"
            | "regsvr32.exe"
            | "bash.exe"
            | "sh.exe"
            | "wsl.exe"
    )
}

fn encode_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().saturating_add(2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
        output.push(ALPHABET[((value >> 6) & 0x3f) as usize] as char);
        output.push(ALPHABET[(value & 0x3f) as usize] as char);
    }

    match chunks.remainder() {
        [first] => {
            let value = u32::from(*first) << 16;
            output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
            output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
            output.push('=');
            output.push('=');
        }
        [first, second] => {
            let value = (u32::from(*first) << 16) | (u32::from(*second) << 8);
            output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
            output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
            output.push(ALPHABET[((value >> 6) & 0x3f) as usize] as char);
            output.push('=');
        }
        [] => {}
        _ => unreachable!("chunks_exact remainder is shorter than three bytes"),
    }
    output
}

fn png_data_url(png: &[u8]) -> Option<String> {
    if !png.starts_with(PNG_SIGNATURE) {
        return None;
    }
    let encoded_len = png.len().checked_add(2)?.checked_div(3)?.checked_mul(4)?;
    let serialized_len = PNG_DATA_URL_PREFIX.len().checked_add(encoded_len)?;
    if serialized_len > MAX_ICON_DATA_URL_BYTES {
        return None;
    }

    let encoded = encode_base64(png);
    let mut data_url = String::with_capacity(serialized_len);
    data_url.push_str(PNG_DATA_URL_PREFIX);
    data_url.push_str(&encoded);
    Some(data_url)
}

#[cfg(windows)]
fn command_is_launchable(command: &str) -> bool {
    platform::command_is_launchable(command)
}

#[cfg(not(windows))]
fn command_is_launchable(_command: &str) -> bool {
    false
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::fs;
    use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{HGLOBAL, RPC_E_CHANGED_MODE},
            Graphics::Imaging::{
                CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
                IWICBitmapFrameEncode, IWICImagingFactory, WICBitmapEncoderNoCache,
            },
            Storage::FileSystem::GetDriveTypeW,
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize,
                StructuredStorage::{CreateStreamOnHGlobal, IPropertyBag2},
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, STATFLAG_NONAME, STATSTG,
                STREAM_SEEK_SET,
            },
            UI::{
                Shell::SHDefExtractIconW,
                WindowsAndMessaging::{DestroyIcon, HICON},
            },
        },
    };
    use winreg::{
        enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        },
        RegKey, HKEY,
    };

    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const DRIVE_REMOTE_TYPE: u32 = 4;

    struct RegistrySource {
        hive_name: &'static str,
        hive: HKEY,
        view_name: &'static str,
        view_flag: u32,
    }

    struct PreparedCommand {
        executable: PathBuf,
        arguments: Vec<String>,
    }

    struct OwnedIcon(HICON);

    impl Drop for OwnedIcon {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe {
                    let _ = DestroyIcon(self.0);
                }
            }
        }
    }

    struct ComApartment {
        uninitialize: bool,
    }

    impl ComApartment {
        fn initialize() -> Result<Self, String> {
            let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if result.is_ok() {
                Ok(Self { uninitialize: true })
            } else if result == RPC_E_CHANGED_MODE {
                // The worker already belongs to another COM apartment. WIC is
                // still usable there, but this call must not be balanced.
                Ok(Self {
                    uninitialize: false,
                })
            } else {
                Err(format!("无法初始化图标编码组件: {result:?}"))
            }
        }
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            if self.uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    pub(super) fn enumerate() -> InstalledAppSnapshot {
        let sources = [
            RegistrySource {
                hive_name: "HKCU",
                hive: HKEY_CURRENT_USER,
                view_name: "64",
                view_flag: KEY_WOW64_64KEY,
            },
            RegistrySource {
                hive_name: "HKCU",
                hive: HKEY_CURRENT_USER,
                view_name: "32",
                view_flag: KEY_WOW64_32KEY,
            },
            RegistrySource {
                hive_name: "HKLM",
                hive: HKEY_LOCAL_MACHINE,
                view_name: "64",
                view_flag: KEY_WOW64_64KEY,
            },
            RegistrySource {
                hive_name: "HKLM",
                hive: HKEY_LOCAL_MACHINE,
                view_name: "32",
                view_flag: KEY_WOW64_32KEY,
            },
        ];

        let mut snapshot = InstalledAppSnapshot::default();
        for source in sources {
            enumerate_source(&source, &mut snapshot);
        }
        snapshot
    }

    pub(super) fn load_icon_png(request: &AppIconRequest) -> Option<Vec<u8>> {
        let mut sources = Vec::with_capacity(2);
        if let Some(source) = request.display_icon.clone() {
            sources.push(source);
        }
        if let Some(source) = request
            .uninstall_command
            .as_deref()
            .and_then(command_icon_source)
        {
            let duplicate = sources.iter().any(|existing| {
                existing.index == source.index && existing.path.eq_ignore_ascii_case(&source.path)
            });
            if !duplicate {
                sources.push(source);
            }
        }

        sources
            .into_iter()
            .find_map(|source| extract_icon_png(&source).ok())
    }

    fn extract_icon_png(source: &IconSource) -> Result<Vec<u8>, String> {
        let path = validated_icon_path(source)?;
        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        if wide_path.len() > 32_768 {
            return Err("图标文件路径过长".into());
        }

        let mut raw_icon = HICON::default();
        let result = unsafe {
            SHDefExtractIconW(
                PCWSTR(wide_path.as_ptr()),
                source.index,
                0,
                Some(&mut raw_icon),
                None,
                APP_ICON_SIZE,
            )
        };
        let icon = OwnedIcon(raw_icon);
        result
            .ok()
            .map_err(|error| format!("Windows Shell 无法提取应用图标: {error}"))?;
        if icon.0 .0.is_null() {
            return Err("Windows Shell 没有返回应用图标".into());
        }

        encode_icon_as_png(icon.0)
    }

    fn validated_icon_path(source: &IconSource) -> Result<PathBuf, String> {
        let candidate = PathBuf::from(source.path.trim());
        if !candidate.is_absolute() || is_network_path(&candidate) || is_remote_drive(&candidate) {
            return Err("图标来源必须是本地绝对路径".into());
        }
        let extension = candidate
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "exe" | "dll" | "ico" | "cpl") {
            return Err("图标来源不是受支持的本地资源文件".into());
        }

        let metadata = fs::symlink_metadata(&candidate)
            .map_err(|error| format!("图标文件不可访问: {error}"))?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err("图标来源不能是目录、符号链接或重解析点".into());
        }
        let canonical =
            fs::canonicalize(&candidate).map_err(|error| format!("无法验证图标路径: {error}"))?;
        if is_network_path(&canonical) || is_remote_drive(&canonical) {
            return Err("拒绝从网络路径读取应用图标".into());
        }
        Ok(canonical)
    }

    fn is_remote_drive(path: &Path) -> bool {
        use std::path::{Component, Prefix};

        let Some(Component::Prefix(prefix)) = path.components().next() else {
            return false;
        };
        let letter = match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
            _ => return false,
        };
        let root = [u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0];
        unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE_TYPE }
    }

    fn encode_icon_as_png(icon: HICON) -> Result<Vec<u8>, String> {
        let _apartment = ComApartment::initialize()?;
        unsafe {
            let factory: IWICImagingFactory = CoCreateInstance(
                &CLSID_WICImagingFactory,
                None::<&windows::core::IUnknown>,
                CLSCTX_INPROC_SERVER,
            )
            .map_err(|error| format!("无法创建图标编码器: {error}"))?;
            let bitmap = factory
                .CreateBitmapFromHICON(icon)
                .map_err(|error| format!("无法读取 HICON 像素: {error}"))?;
            let mut width = 0;
            let mut height = 0;
            bitmap
                .GetSize(&mut width, &mut height)
                .map_err(|error| format!("无法读取图标尺寸: {error}"))?;
            if width == 0 || height == 0 || width > 256 || height > 256 {
                return Err("Windows Shell 返回了异常图标尺寸".into());
            }

            // The stream owns its anonymous HGLOBAL and releases it when the
            // final COM reference is dropped.
            let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)
                .map_err(|error| format!("无法创建图标内存流: {error}"))?;
            let encoder = factory
                .CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())
                .map_err(|error| format!("无法创建 PNG 编码器: {error}"))?;
            encoder
                .Initialize(&stream, WICBitmapEncoderNoCache)
                .map_err(|error| format!("无法初始化 PNG 编码器: {error}"))?;

            let mut frame: Option<IWICBitmapFrameEncode> = None;
            let mut options: Option<IPropertyBag2> = None;
            encoder
                .CreateNewFrame(&mut frame, &mut options)
                .map_err(|error| format!("无法创建 PNG 图像帧: {error}"))?;
            let frame = frame.ok_or_else(|| "PNG 编码器没有返回图像帧".to_string())?;
            frame
                .Initialize(options.as_ref())
                .map_err(|error| format!("无法初始化 PNG 图像帧: {error}"))?;
            frame
                .SetSize(width, height)
                .map_err(|error| format!("无法设置 PNG 图标尺寸: {error}"))?;
            let mut pixel_format = GUID_WICPixelFormat32bppBGRA;
            frame
                .SetPixelFormat(&mut pixel_format)
                .map_err(|error| format!("无法设置 PNG 像素格式: {error}"))?;
            frame
                .WriteSource(&bitmap, std::ptr::null())
                .map_err(|error| format!("无法编码 PNG 图标像素: {error}"))?;
            frame
                .Commit()
                .map_err(|error| format!("无法提交 PNG 图像帧: {error}"))?;
            encoder
                .Commit()
                .map_err(|error| format!("无法提交 PNG 图标: {error}"))?;

            let mut stat: STATSTG = std::mem::zeroed();
            stream
                .Stat(&mut stat, STATFLAG_NONAME)
                .map_err(|error| format!("无法读取 PNG 数据长度: {error}"))?;
            let length = usize::try_from(stat.cbSize)
                .map_err(|_| "PNG 图标数据长度超出平台限制".to_string())?;
            if length == 0 || length > MAX_ICON_DATA_URL_BYTES {
                return Err("PNG 图标数据为空或超过 256 KiB".into());
            }

            stream
                .Seek(0, STREAM_SEEK_SET, None)
                .map_err(|error| format!("无法重置 PNG 内存流: {error}"))?;
            let mut png = vec![0_u8; length];
            let mut bytes_read = 0_u32;
            stream
                .Read(
                    png.as_mut_ptr().cast(),
                    u32::try_from(length).map_err(|_| "PNG 图标数据过大".to_string())?,
                    Some(&mut bytes_read),
                )
                .ok()
                .map_err(|error| format!("无法读取 PNG 内存流: {error}"))?;
            if bytes_read as usize != length {
                return Err("PNG 内存流读取不完整".into());
            }
            Ok(png)
        }
    }

    fn enumerate_source(source: &RegistrySource, snapshot: &mut InstalledAppSnapshot) {
        let hive = RegKey::predef(source.hive);
        let Ok(root) = hive.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | source.view_flag)
        else {
            return;
        };

        for subkey_name in root.enum_keys().filter_map(Result::ok) {
            let Ok(key) = root.open_subkey_with_flags(&subkey_name, KEY_READ | source.view_flag)
            else {
                continue;
            };
            if read_dword(&key, "SystemComponent") == Some(1)
                || read_non_empty(&key, "ParentKeyName").is_some()
                || is_windows_update(&key)
            {
                continue;
            }

            let Some(name) = read_non_empty(&key, "DisplayName") else {
                continue;
            };
            let id = stable_app_id(source.hive_name, source.view_name, &subkey_name);
            // Never fall back to QuietUninstallString: after an explicit user
            // confirmation, the vendor's normal visible uninstaller must be
            // shown. Quiet-only entries remain listed but are not launchable.
            let uninstall_command = select_visible_uninstaller(
                read_non_empty(&key, "UninstallString"),
                read_dword(&key, "NoRemove") == Some(1),
            );
            let icon_source =
                read_non_empty(&key, "DisplayIcon").and_then(|value| parse_display_icon(&value));
            let record = InstalledAppRecord {
                id: id.clone(),
                name,
                publisher: read_non_empty(&key, "Publisher").unwrap_or_default(),
                version: read_non_empty(&key, "DisplayVersion").unwrap_or_default(),
                size_bytes: estimated_size_bytes(&key),
                installed_at: read_non_empty(&key, "InstallDate").unwrap_or_default(),
                uninstall_command,
                icon_source,
            };
            snapshot.records.insert(id, record);
        }
    }

    fn read_non_empty(key: &RegKey, value_name: &str) -> Option<String> {
        key.get_value::<String, _>(value_name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn read_dword(key: &RegKey, value_name: &str) -> Option<u32> {
        key.get_value::<u32, _>(value_name).ok()
    }

    fn estimated_size_bytes(key: &RegKey) -> u64 {
        let kibibytes = key
            .get_value::<u32, _>("EstimatedSize")
            .map(u64::from)
            .or_else(|_| key.get_value::<u64, _>("EstimatedSize"))
            .or_else(|_| {
                key.get_value::<String, _>("EstimatedSize")
                    .map(|value| value.trim().parse::<u64>().unwrap_or(0))
            })
            .unwrap_or(0);
        kibibytes.saturating_mul(1024)
    }

    fn is_windows_update(key: &RegKey) -> bool {
        read_non_empty(key, "ReleaseType").is_some_and(|release_type| {
            matches!(
                release_type.to_ascii_lowercase().as_str(),
                "hotfix" | "security update" | "update rollup"
            )
        })
    }

    pub(super) fn launch(id: &str, command: &str) -> Result<LaunchResult, String> {
        let parsed = parse_uninstall_command(command)?;
        let prepared = validate_command(parsed)?;
        let mut process = Command::new(&prepared.executable);
        process.args(&prepared.arguments);
        if let Some(parent) = prepared.executable.parent() {
            process.current_dir(parent);
        }

        let child = process
            .spawn()
            .map_err(|error| format!("无法启动注册的卸载器: {error}"))?;
        let pid = child.id();
        drop(child);
        Ok(LaunchResult {
            app_id: id.to_string(),
            pid,
            status: "started".into(),
        })
    }

    pub(super) fn command_is_launchable(command: &str) -> bool {
        parse_uninstall_command(command)
            .and_then(validate_command)
            .is_ok()
    }

    fn validate_command(parsed: ParsedCommand) -> Result<PreparedCommand, String> {
        let executable = resolve_executable(&parsed.executable)?;
        Ok(PreparedCommand {
            executable,
            arguments: parsed.arguments,
        })
    }

    fn resolve_executable(raw_executable: &str) -> Result<PathBuf, String> {
        let raw_executable = raw_executable.trim();
        if raw_executable.is_empty() {
            return Err("卸载命令没有可执行文件".into());
        }

        let lower_name = raw_executable.to_ascii_lowercase();
        let bare_name = lower_name.strip_suffix(".exe").unwrap_or(&lower_name);
        let has_path_separator = raw_executable
            .chars()
            .any(|character| matches!(character, '\\' | '/'));
        let candidate = if !has_path_separator && matches!(bare_name, "msiexec" | "rundll32") {
            let system_root = std::env::var_os("SystemRoot")
                .ok_or_else(|| "无法定位 Windows 系统目录".to_string())?;
            PathBuf::from(system_root)
                .join("System32")
                .join(format!("{bare_name}.exe"))
        } else {
            PathBuf::from(raw_executable)
        };

        if !candidate.is_absolute() {
            return Err("卸载器必须使用绝对路径或受信任的 Windows 系统程序".into());
        }
        if is_network_path(&candidate) {
            return Err("拒绝从网络路径启动卸载器".into());
        }
        if !candidate
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            return Err("卸载器必须是可执行文件，不能是脚本或 shell 命令".into());
        }

        let file_name = candidate
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if is_blocked_interpreter(file_name) {
            return Err("拒绝通过命令解释器启动卸载操作".into());
        }

        let link_metadata = fs::symlink_metadata(&candidate)
            .map_err(|error| format!("卸载器文件不可访问: {error}"))?;
        if link_metadata.file_type().is_symlink()
            || link_metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err("卸载器文件不能是符号链接或重解析点".into());
        }
        if !link_metadata.is_file() {
            return Err("卸载器路径不是普通文件".into());
        }

        let canonical =
            fs::canonicalize(&candidate).map_err(|error| format!("无法验证卸载器路径: {error}"))?;
        if is_network_path(&canonical) {
            return Err("拒绝从网络路径启动卸载器".into());
        }
        Ok(canonical)
    }

    fn is_network_path(path: &Path) -> bool {
        let value = path.as_os_str().to_string_lossy();
        let lower = value.to_ascii_lowercase();
        lower.starts_with(r"\\?\unc\") || (lower.starts_with(r"\\") && !lower.starts_with(r"\\?\"))
    }

    fn is_blocked_interpreter(file_name: &str) -> bool {
        matches!(
            file_name.to_ascii_lowercase().as_str(),
            "cmd.exe"
                | "powershell.exe"
                | "pwsh.exe"
                | "wscript.exe"
                | "cscript.exe"
                | "mshta.exe"
                | "bash.exe"
                | "sh.exe"
                | "wsl.exe"
        )
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub(super) fn enumerate() -> InstalledAppSnapshot {
        InstalledAppSnapshot::default()
    }

    pub(super) fn load_icon_png(_request: &AppIconRequest) -> Option<Vec<u8>> {
        None
    }

    pub(super) fn launch(_id: &str, _command: &str) -> Result<LaunchResult, String> {
        Err("应用卸载器仅支持 Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_base64_for_test(value: &str) -> Option<Vec<u8>> {
        fn sextet(value: u8) -> Option<u8> {
            match value {
                b'A'..=b'Z' => Some(value - b'A'),
                b'a'..=b'z' => Some(value - b'a' + 26),
                b'0'..=b'9' => Some(value - b'0' + 52),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        }

        let bytes = value.as_bytes();
        if bytes.len() % 4 != 0 {
            return None;
        }
        let mut decoded = Vec::with_capacity(bytes.len() / 4 * 3);
        let chunk_count = bytes.len() / 4;
        for (index, chunk) in bytes.chunks_exact(4).enumerate() {
            let first = sextet(chunk[0])?;
            let second = sextet(chunk[1])?;
            let final_chunk = index + 1 == chunk_count;
            decoded.push((first << 2) | (second >> 4));

            if chunk[2] == b'=' {
                if !final_chunk || chunk[3] != b'=' {
                    return None;
                }
                continue;
            }
            let third = sextet(chunk[2])?;
            decoded.push((second << 4) | (third >> 2));

            if chunk[3] == b'=' {
                if !final_chunk {
                    return None;
                }
                continue;
            }
            let fourth = sextet(chunk[3])?;
            decoded.push((third << 6) | fourth);
        }
        Some(decoded)
    }

    #[test]
    fn parses_quoted_executable_and_arguments() {
        let parsed = parse_uninstall_command(
            r#""C:\Program Files\Example App\uninstall.exe" /remove "all users""#,
        )
        .expect("quoted command should parse");

        assert_eq!(
            parsed.executable,
            r"C:\Program Files\Example App\uninstall.exe"
        );
        assert_eq!(parsed.arguments, ["/remove", "all users"]);
    }

    #[test]
    fn parses_msiexec_product_command() {
        let parsed =
            parse_uninstall_command(r"MsiExec.exe /I{12345678-1234-1234-1234-1234567890AB}")
                .expect("MSI command should parse");

        assert_eq!(parsed.executable, "MsiExec.exe");
        assert_eq!(
            parsed.arguments,
            ["/I{12345678-1234-1234-1234-1234567890AB}"]
        );
    }

    #[test]
    fn recovers_unquoted_executable_path_with_spaces() {
        let parsed = parse_uninstall_command(
            r"C:\Program Files\Example App\uninstall.exe --uninstall --scope user",
        )
        .expect("unquoted executable should be recovered without shell search");

        assert_eq!(
            parsed.executable,
            r"C:\Program Files\Example App\uninstall.exe"
        );
        assert_eq!(parsed.arguments, ["--uninstall", "--scope", "user"]);
    }

    #[test]
    fn handles_windows_backslashes_before_quote() {
        let parsed = parse_windows_arguments(r#"tool.exe "a\\\"b" "c d""#)
            .expect("escaped quote should parse");

        assert_eq!(parsed, ["tool.exe", "a\\\"b", "c d"]);
    }

    #[test]
    fn rejects_unclosed_quotes() {
        let error = parse_uninstall_command(r#""C:\Program Files\bad.exe /remove"#)
            .expect_err("malformed command should be rejected");
        assert!(error.contains("未闭合"));
    }

    #[test]
    fn rejects_unknown_snapshot_id_without_starting_any_process() {
        let snapshot = InstalledAppSnapshot::default();
        let error = snapshot
            .uninstall_request("app_forged")
            .err()
            .expect("unknown IDs must be rejected before platform launch");

        assert_eq!(error, "应用不属于最近一次枚举快照");
    }

    #[test]
    fn production_release_policy_rejects_before_starting_uninstaller() {
        let error = launch_prepared_uninstaller_with_policy(
            UninstallRequest {
                app_id: "app_test".into(),
                command: "must-not-start.exe /uninstall".into(),
            },
            CapabilityPolicy::production_release_for_test(),
        )
        .err()
        .expect("production release must reject legacy Win32 uninstall launch");

        assert!(error.contains("发布策略未启用"));
        assert!(error.contains("未启动外部进程"));
    }

    #[test]
    fn app_id_is_stable_and_separates_registry_views() {
        let first = stable_app_id("HKLM", "64", "{PRODUCT-GUID}");
        let same = stable_app_id("HKLM", "64", "{product-guid}");
        let other_view = stable_app_id("HKLM", "32", "{PRODUCT-GUID}");

        assert_eq!(first, same);
        assert_ne!(first, other_view);
        assert!(first.starts_with("app_"));
        assert_eq!(first.len(), 68);
    }

    #[test]
    fn requires_a_normal_visible_uninstall_string() {
        // A quiet-only registry entry reaches this policy as `None`; there is
        // deliberately no QuietUninstallString parameter or fallback.
        assert_eq!(select_visible_uninstaller(None, false), None);
        assert_eq!(
            select_visible_uninstaller(Some("visible.exe".into()), true),
            None
        );
        assert_eq!(
            select_visible_uninstaller(Some("visible.exe".into()), false),
            Some("visible.exe".into())
        );
    }

    #[test]
    fn parses_display_icon_quotes_commas_and_resource_ids() {
        let quoted = parse_display_icon(r#" @"C:\Program Files\Acme, Inc\resources.dll", -42 "#)
            .expect("quoted DisplayIcon should parse");
        assert_eq!(
            quoted,
            IconSource {
                path: r"C:\Program Files\Acme, Inc\resources.dll".into(),
                index: -42,
            }
        );

        let unquoted = parse_display_icon(r"C:\Program Files\Acme, Inc\acme.exe,7")
            .expect("the last numeric comma suffix is the resource index");
        assert_eq!(
            unquoted,
            IconSource {
                path: r"C:\Program Files\Acme, Inc\acme.exe".into(),
                index: 7,
            }
        );
    }

    #[test]
    fn rejects_malformed_or_indirect_non_file_display_icons() {
        assert!(parse_display_icon(r#""C:\app.exe",not-an-index"#).is_none());
        assert!(parse_display_icon("ms-resource:AppIcon").is_none());
        assert!(parse_display_icon("@{Package?ms-resource://icon}").is_none());
        assert!(parse_display_icon("C:\\app.exe\0,0").is_none());
    }

    #[test]
    fn uninstall_icon_fallback_accepts_only_local_non_interpreter_exe() {
        let source = command_icon_source(r#""C:\Program Files\Example\uninstall.exe" /remove"#)
            .expect("local vendor executable should be usable as a fallback");
        assert_eq!(source.path, r"C:\Program Files\Example\uninstall.exe");
        assert_eq!(source.index, 0);

        assert!(command_icon_source(r"MsiExec.exe /I{PRODUCT}").is_none());
        assert!(
            command_icon_source(r"C:\Windows\System32\rundll32.exe setup.dll,Remove").is_none()
        );
        assert!(command_icon_source(r"C:\Windows\System32\cmd.exe /c remove").is_none());
        assert!(command_icon_source(r"\\server\share\uninstall.exe /remove").is_none());
    }

    #[test]
    fn png_data_url_is_base64_round_trippable_and_bounded() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");

        let mut png = PNG_SIGNATURE.to_vec();
        png.extend_from_slice(&[0, 1, 2, 3, 4]);
        let data_url = png_data_url(&png).expect("small PNG should serialize");
        let payload = data_url
            .strip_prefix(PNG_DATA_URL_PREFIX)
            .expect("data URL should use the PNG prefix");
        assert_eq!(payload, "iVBORw0KGgoAAQIDBA==");

        let mut oversized = vec![0_u8; MAX_ICON_DATA_URL_BYTES];
        oversized[..PNG_SIGNATURE.len()].copy_from_slice(PNG_SIGNATURE);
        assert!(png_data_url(&oversized).is_none());
        assert!(png_data_url(b"not a png").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn extracts_real_system_icon_through_shell_wic_and_data_url() {
        let system_root =
            std::env::var_os("SystemRoot").expect("Windows must expose the SystemRoot directory");
        let system32 = std::path::PathBuf::from(system_root).join("System32");
        let data_url = ["shell32.dll", "imageres.dll"]
            .into_iter()
            .filter_map(|file_name| {
                let path = system32.join(file_name);
                path.is_file().then_some(path)
            })
            .find_map(|path| {
                load_app_icon(AppIconRequest {
                    display_icon: Some(IconSource {
                        path: path.display().to_string(),
                        index: 0,
                    }),
                    uninstall_command: None,
                })
            })
            .expect("Shell and WIC should extract at least one Windows system icon");

        assert!(data_url.starts_with(PNG_DATA_URL_PREFIX));
        assert!(data_url.len() > PNG_DATA_URL_PREFIX.len() + 16);
        assert!(data_url.len() <= MAX_ICON_DATA_URL_BYTES);
        let png = decode_base64_for_test(
            data_url
                .strip_prefix(PNG_DATA_URL_PREFIX)
                .expect("system icon should be a PNG data URL"),
        )
        .expect("system icon payload should be valid base64");
        assert!(png.starts_with(PNG_SIGNATURE));
        assert!(png.len() > PNG_SIGNATURE.len() + 32);
        assert!(png.len() < MAX_ICON_DATA_URL_BYTES);
    }

    #[cfg(windows)]
    #[test]
    fn extracts_real_executable_icon_from_a_startup_command() {
        let system_root =
            std::env::var_os("SystemRoot").expect("Windows must expose the SystemRoot directory");
        let executable = std::path::PathBuf::from(system_root)
            .join("System32")
            .join("notepad.exe");
        assert!(
            executable.is_file(),
            "Windows Notepad executable should exist"
        );

        let command = format!(r#""{}" /background"#, executable.display());
        let data_url = load_startup_icon(&command)
            .expect("a startup command should resolve to its executable's real icon");

        assert!(data_url.starts_with(PNG_DATA_URL_PREFIX));
        let png = decode_base64_for_test(
            data_url
                .strip_prefix(PNG_DATA_URL_PREFIX)
                .expect("startup icon should be a PNG data URL"),
        )
        .expect("startup icon payload should be valid base64");
        assert!(png.starts_with(PNG_SIGNATURE));
    }
}
