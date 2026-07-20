use crate::models::{CleanupItem, DeleteMode, RiskLevel};
use std::{
    fs::{self, Metadata},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use sysinfo::{ProcessesToUpdate, System};
use walkdir::WalkDir;

const TEMP_MINIMUM_AGE: Duration = Duration::from_secs(72 * 60 * 60);
const REGENERABLE_CACHE_DIRECTORIES: [(&str, &str); 3] = [
    ("Cache", "cache"),
    ("Code Cache", "code-cache"),
    ("GPUCache", "gpu-cache"),
];

#[derive(Clone, Copy)]
pub enum FileMatcher {
    AllFilesRecursive,
    RootFileName {
        prefix: &'static str,
        suffix: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootBase {
    Local,
    Roaming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessGuard {
    Edge,
    Chrome,
    Firefox,
    VsCode,
    Discord,
    Figma,
}

#[derive(Clone)]
pub struct Rule {
    pub id: String,
    pub category: &'static str,
    pub name: String,
    pub base: RootBase,
    pub relative: PathBuf,
    pub risk: RiskLevel,
    pub matcher: FileMatcher,
    pub minimum_age: Option<Duration>,
    process_guard: Option<ProcessGuard>,
}

#[derive(Clone, Debug)]
struct FileSnapshot {
    path: PathBuf,
    canonical_path: PathBuf,
    size: u64,
    modified: SystemTime,
}

#[derive(Clone, Debug)]
pub struct CleanupSnapshot {
    item: CleanupItem,
    root: PathBuf,
    canonical_root: PathBuf,
    files: Vec<FileSnapshot>,
}

impl CleanupSnapshot {
    pub fn item(&self) -> &CleanupItem {
        &self.item
    }
}

#[derive(Debug)]
pub struct DeleteFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Default)]
pub struct DeleteOutcome {
    pub reclaimed_bytes: u64,
    pub failures: Vec<DeleteFailure>,
}

struct DirectorySnapshot {
    root: PathBuf,
    canonical_root: PathBuf,
    files: Vec<FileSnapshot>,
}

fn cache_rule(
    id: impl Into<String>,
    name: impl Into<String>,
    base: RootBase,
    relative: impl Into<PathBuf>,
    process_guard: Option<ProcessGuard>,
) -> Rule {
    Rule {
        id: id.into(),
        category: "应用缓存",
        name: name.into(),
        base,
        relative: relative.into(),
        risk: RiskLevel::Low,
        matcher: FileMatcher::AllFilesRecursive,
        minimum_age: None,
        process_guard,
    }
}

fn stable_id_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn trusted_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_link_or_reparse(&metadata))
        .unwrap_or(false)
}

fn child_directory_names(parent: &Path) -> Vec<String> {
    if !trusted_directory(parent) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut names = entries
        .flatten()
        .filter(|entry| trusted_directory(&entry.path()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    names.sort_by_cached_key(|name| name.to_ascii_lowercase());
    names
}

fn discover_chromium_rules(
    rules: &mut Vec<Rule>,
    local_root: &Path,
    product_id: &'static str,
    product_name: &'static str,
    user_data_relative: &Path,
    process_guard: ProcessGuard,
) {
    let user_data = local_root.join(user_data_relative);
    for profile in child_directory_names(&user_data) {
        if profile != "Default"
            && !profile
                .strip_prefix("Profile ")
                .is_some_and(|suffix| !suffix.is_empty())
        {
            continue;
        }

        let profile_relative = user_data_relative.join(&profile);
        let profile_id = stable_id_component(&profile);
        for (cache_directory, cache_id) in REGENERABLE_CACHE_DIRECTORIES {
            let relative = profile_relative.join(cache_directory);
            if !trusted_directory(&local_root.join(&relative)) {
                continue;
            }
            rules.push(cache_rule(
                format!("{product_id}-profile-{profile_id}-{cache_id}"),
                format!("{product_name} · {profile} · {cache_directory}"),
                RootBase::Local,
                relative,
                Some(process_guard),
            ));
        }
    }
}

fn discover_firefox_rules(rules: &mut Vec<Rule>, local_root: &Path) {
    let profiles_relative = Path::new("Mozilla/Firefox/Profiles");
    let profiles_root = local_root.join(profiles_relative);
    for profile in child_directory_names(&profiles_root) {
        let relative = profiles_relative.join(&profile).join("cache2");
        if !trusted_directory(&local_root.join(&relative)) {
            continue;
        }
        rules.push(cache_rule(
            format!("firefox-profile-{}-cache2", stable_id_component(&profile)),
            format!("Firefox · {profile} · cache2"),
            RootBase::Local,
            relative,
            Some(ProcessGuard::Firefox),
        ));
    }
}

fn add_application_cache_rules(
    rules: &mut Vec<Rule>,
    product_id: &'static str,
    product_name: &'static str,
    application_relative: &Path,
    process_guard: ProcessGuard,
) {
    for (cache_directory, cache_id) in REGENERABLE_CACHE_DIRECTORIES {
        rules.push(cache_rule(
            format!("{product_id}-{cache_id}"),
            format!("{product_name} · {cache_directory}"),
            RootBase::Roaming,
            application_relative.join(cache_directory),
            Some(process_guard),
        ));
    }
}

fn rules_for_roots(local_root: Option<&Path>, _roaming_root: Option<&Path>) -> Vec<Rule> {
    let mut rules = vec![
        Rule {
            id: "temp".into(),
            category: "系统缓存",
            name: "临时文件".into(),
            base: RootBase::Local,
            relative: "Temp".into(),
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: Some(TEMP_MINIMUM_AGE),
            process_guard: None,
        },
        Rule {
            id: "thumbs".into(),
            category: "系统缓存",
            name: "缩略图缓存".into(),
            base: RootBase::Local,
            relative: "Microsoft/Windows/Explorer".into(),
            risk: RiskLevel::Low,
            matcher: FileMatcher::RootFileName {
                prefix: "thumbcache_",
                suffix: ".db",
            },
            minimum_age: None,
            process_guard: None,
        },
    ];

    add_application_cache_rules(
        &mut rules,
        "vscode",
        "Visual Studio Code",
        Path::new("Code"),
        ProcessGuard::VsCode,
    );
    add_application_cache_rules(
        &mut rules,
        "discord",
        "Discord",
        Path::new("discord"),
        ProcessGuard::Discord,
    );
    add_application_cache_rules(
        &mut rules,
        "figma",
        "Figma",
        Path::new("Figma"),
        ProcessGuard::Figma,
    );

    if let Some(local_root) = local_root {
        discover_chromium_rules(
            &mut rules,
            local_root,
            "edge",
            "Microsoft Edge",
            Path::new("Microsoft/Edge/User Data"),
            ProcessGuard::Edge,
        );
        discover_chromium_rules(
            &mut rules,
            local_root,
            "chrome",
            "Google Chrome",
            Path::new("Google/Chrome/User Data"),
            ProcessGuard::Chrome,
        );
        discover_firefox_rules(&mut rules, local_root);
    }

    rules
}

pub fn rules() -> Vec<Rule> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    rules_for_roots(local_root.as_deref(), roaming_root.as_deref())
}

pub fn local_root() -> Option<PathBuf> {
    dirs::data_local_dir()
}

pub fn roaming_root() -> Option<PathBuf> {
    dirs::data_dir()
}

fn base_root_for<'a>(
    rule: &Rule,
    local_root: Option<&'a Path>,
    roaming_root: Option<&'a Path>,
) -> Option<&'a Path> {
    match rule.base {
        RootBase::Local => local_root,
        RootBase::Roaming => roaming_root,
    }
}

pub fn path_for(rule: &Rule) -> Option<PathBuf> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    let root = base_root_for(rule, local_root.as_deref(), roaming_root.as_deref())?;
    Some(root.join(&rule.relative))
}

fn guarded_process_names(guard: ProcessGuard) -> &'static [&'static str] {
    match guard {
        ProcessGuard::Edge => &["msedge.exe", "msedge"],
        ProcessGuard::Chrome => &["chrome.exe", "chrome"],
        ProcessGuard::Firefox => &["firefox.exe", "firefox"],
        ProcessGuard::VsCode => &["code.exe", "code", "code-insiders.exe", "code-insiders"],
        ProcessGuard::Discord => &["discord.exe", "discord"],
        ProcessGuard::Figma => &["figma.exe", "figma"],
    }
}

fn process_guard_name(guard: ProcessGuard) -> &'static str {
    match guard {
        ProcessGuard::Edge => "Microsoft Edge",
        ProcessGuard::Chrome => "Google Chrome",
        ProcessGuard::Firefox => "Firefox",
        ProcessGuard::VsCode => "Visual Studio Code",
        ProcessGuard::Discord => "Discord",
        ProcessGuard::Figma => "Figma",
    }
}

fn process_basename(process_name: &str) -> &str {
    process_name
        .trim()
        .rsplit(|character| character == '/' || character == '\\')
        .next()
        .unwrap_or_default()
}

fn process_guard_blocks(guard: ProcessGuard, process_names: &[String]) -> bool {
    process_names.iter().any(|process_name| {
        let basename = process_basename(process_name);
        guarded_process_names(guard)
            .iter()
            .any(|guarded| basename.eq_ignore_ascii_case(guarded))
    })
}

fn rule_is_executable(rule: &Rule, process_names: Option<&[String]>) -> bool {
    match (rule.process_guard, process_names) {
        (None, _) => true,
        (Some(guard), Some(process_names)) => !process_guard_blocks(guard, process_names),
        (Some(_), None) => false,
    }
}

fn running_process_names() -> Option<Vec<String>> {
    let mut system = System::new();
    if system.refresh_processes(ProcessesToUpdate::All, true) == 0 {
        return None;
    }
    Some(
        system
            .processes()
            .values()
            .map(|process| process.name().to_string_lossy().into_owned())
            .collect(),
    )
}

fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    #[cfg(not(windows))]
    false
}

fn validate_rule_directory_chain(base_root: &Path, rule_root: &Path) -> Result<(), String> {
    let relative = rule_root
        .strip_prefix(base_root)
        .map_err(|_| "清理目录不在规则基准目录内".to_string())?;
    if relative.as_os_str().is_empty() {
        return Err("清理规则不得指向整个基准目录".into());
    }

    let mut current = base_root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("清理目录链已失效或不可访问: {error}"))?;
        if is_link_or_reparse(&metadata) || !metadata.is_dir() {
            return Err("清理目录链包含链接、重解析点或非目录对象".into());
        }
    }
    Ok(())
}

fn matches_rule(
    rule: &Rule,
    path: &Path,
    depth: usize,
    modified: SystemTime,
    scan_started_at: SystemTime,
) -> bool {
    if let Some(minimum_age) = rule.minimum_age {
        let Ok(age) = scan_started_at.duration_since(modified) else {
            return false;
        };
        if age < minimum_age {
            return false;
        }
    }

    match rule.matcher {
        FileMatcher::AllFilesRecursive => true,
        FileMatcher::RootFileName { prefix, suffix } => {
            if depth != 1 {
                return false;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            let file_name = file_name.to_ascii_lowercase();
            file_name.len() >= prefix.len() + suffix.len()
                && file_name.starts_with(prefix)
                && file_name.ends_with(suffix)
        }
    }
}

fn snapshot_directory(
    root: &Path,
    rule: &Rule,
    scan_started_at: SystemTime,
) -> Result<DirectorySnapshot, String> {
    let root_metadata =
        fs::symlink_metadata(root).map_err(|error| format!("无法读取清理目录: {error}"))?;
    if is_link_or_reparse(&root_metadata) || !root_metadata.is_dir() {
        return Err("清理目录不是可信的普通目录".into());
    }

    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("无法验证清理目录: {error}"))?;
    let mut files = Vec::new();

    let entries = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || fs::symlink_metadata(entry.path())
                    .map(|metadata| !is_link_or_reparse(&metadata))
                    .unwrap_or(false)
        });
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.depth() == 0 || !entry.file_type().is_file() {
            continue;
        }

        let depth = entry.depth();
        let path = entry.into_path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if is_link_or_reparse(&metadata) || !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if !matches_rule(rule, &path, depth, modified, scan_started_at) {
            continue;
        }
        let Ok(canonical_path) = fs::canonicalize(&path) else {
            continue;
        };
        if canonical_path == canonical_root || !canonical_path.starts_with(&canonical_root) {
            continue;
        }

        files.push(FileSnapshot {
            path,
            canonical_path,
            size: metadata.len(),
            modified,
        });
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(DirectorySnapshot {
        root: root.to_path_buf(),
        canonical_root,
        files,
    })
}

fn scan_with_environment(
    local_root: Option<&Path>,
    roaming_root: Option<&Path>,
    process_names: Option<&[String]>,
) -> Vec<CleanupSnapshot> {
    let scan_started_at = SystemTime::now();

    rules_for_roots(local_root, roaming_root)
        .into_iter()
        .filter(|rule| rule_is_executable(rule, process_names))
        .filter_map(|rule| {
            let base_root = base_root_for(&rule, local_root, roaming_root)?;
            let canonical_base_root = fs::canonicalize(base_root).ok()?;
            let root = base_root.join(&rule.relative);
            validate_rule_directory_chain(base_root, &root).ok()?;
            let directory = snapshot_directory(&root, &rule, scan_started_at).ok()?;
            if directory.canonical_root == canonical_base_root
                || !directory.canonical_root.starts_with(&canonical_base_root)
            {
                return None;
            }

            let size_bytes = directory.files.iter().map(|file| file.size).sum();
            let item = CleanupItem {
                id: rule.id,
                category: rule.category.into(),
                name: rule.name,
                path: directory.root.display().to_string(),
                description: "可由应用或 Windows 自动重新生成".into(),
                size_bytes,
                risk: rule.risk,
                delete_mode: DeleteMode::Permanent,
            };

            Some(CleanupSnapshot {
                item,
                root: directory.root,
                canonical_root: directory.canonical_root,
                files: directory.files,
            })
        })
        .collect()
}

pub fn scan() -> Vec<CleanupSnapshot> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    let process_names = running_process_names();
    scan_with_environment(
        local_root.as_deref(),
        roaming_root.as_deref(),
        process_names.as_deref(),
    )
}

fn rule_for_snapshot(snapshot: &CleanupSnapshot) -> Result<Rule, String> {
    rules()
        .into_iter()
        .find(|rule| rule.id == snapshot.item.id.as_str())
        .ok_or_else(|| "未知清理条目".to_string())
}

fn validated_snapshot_root(snapshot: &CleanupSnapshot, rule: &Rule) -> Result<PathBuf, String> {
    let expected_root = path_for(&rule).ok_or_else(|| "无法定位用户缓存目录".to_string())?;
    if expected_root != snapshot.root {
        return Err("清理目录与扫描快照不一致".into());
    }

    let root_metadata =
        fs::symlink_metadata(&snapshot.root).map_err(|error| format!("清理目录已失效: {error}"))?;
    if is_link_or_reparse(&root_metadata) || !root_metadata.is_dir() {
        return Err("清理目录已变为链接或非目录对象".into());
    }

    let local_root = local_root();
    let roaming_root = roaming_root();
    let base_root = base_root_for(rule, local_root.as_deref(), roaming_root.as_deref())
        .ok_or_else(|| "无法定位用户缓存目录".to_string())?;
    validate_rule_directory_chain(base_root, &snapshot.root)?;
    let canonical_base_root =
        fs::canonicalize(base_root).map_err(|error| format!("无法验证用户缓存目录: {error}"))?;
    let canonical_root =
        fs::canonicalize(&snapshot.root).map_err(|error| format!("无法验证清理目录: {error}"))?;
    if canonical_root == canonical_base_root || !canonical_root.starts_with(&canonical_base_root) {
        return Err("清理目录不在允许范围内".into());
    }
    if canonical_root != snapshot.canonical_root {
        return Err("清理目录在扫描后发生变化".into());
    }

    Ok(canonical_root)
}

fn validate_file_metadata(metadata: &Metadata, snapshot: &FileSnapshot) -> Result<(), String> {
    if is_link_or_reparse(metadata) || !metadata.is_file() {
        return Err("文件已变为链接或非普通文件".into());
    }
    if metadata.len() != snapshot.size {
        return Err(format!(
            "文件大小已变化（扫描时 {} 字节，当前 {} 字节）",
            snapshot.size,
            metadata.len()
        ));
    }
    let modified = metadata
        .modified()
        .map_err(|error| format!("无法读取文件修改时间: {error}"))?;
    if modified != snapshot.modified {
        return Err("文件修改时间已变化".into());
    }
    Ok(())
}

fn validate_parent_chain(root: &Path, file: &Path) -> Result<(), String> {
    let parent = file
        .parent()
        .ok_or_else(|| "快照文件没有有效父目录".to_string())?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| "快照文件父目录不在规则目录内".to_string())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("文件父目录已失效或不可访问: {error}"))?;
        if is_link_or_reparse(&metadata) || !metadata.is_dir() {
            return Err("文件父目录已变为链接或非目录对象".into());
        }
    }
    Ok(())
}

fn delete_snapshot_file(
    root: &Path,
    canonical_root: &Path,
    snapshot: &FileSnapshot,
) -> Result<u64, String> {
    if snapshot.path == root || !snapshot.path.starts_with(root) {
        return Err("快照文件不在规则目录内".into());
    }
    validate_parent_chain(root, &snapshot.path)?;

    let metadata = fs::symlink_metadata(&snapshot.path)
        .map_err(|error| format!("文件已失效或不可访问: {error}"))?;
    validate_file_metadata(&metadata, snapshot)?;

    let canonical_path =
        fs::canonicalize(&snapshot.path).map_err(|error| format!("无法验证文件路径: {error}"))?;
    if canonical_path == canonical_root || !canonical_path.starts_with(canonical_root) {
        return Err("文件当前不在规则目录内".into());
    }
    if canonical_path != snapshot.canonical_path {
        return Err("文件解析路径在扫描后发生变化".into());
    }

    // Re-read link-aware metadata after canonicalization so a path change during
    // validation is detected before the path is handed to the delete operation.
    let metadata = fs::symlink_metadata(&snapshot.path)
        .map_err(|error| format!("文件在验证期间失效: {error}"))?;
    validate_parent_chain(root, &snapshot.path)?;
    validate_file_metadata(&metadata, snapshot)?;

    fs::remove_file(&snapshot.path).map_err(|error| format!("删除失败: {error}"))?;
    Ok(snapshot.size)
}

fn delete_snapshot_files(snapshot: &CleanupSnapshot, canonical_root: &Path) -> DeleteOutcome {
    let mut outcome = DeleteOutcome::default();
    for file in &snapshot.files {
        match delete_snapshot_file(&snapshot.root, canonical_root, file) {
            Ok(bytes) => outcome.reclaimed_bytes = outcome.reclaimed_bytes.saturating_add(bytes),
            Err(error) => outcome.failures.push(DeleteFailure {
                path: file.path.clone(),
                error,
            }),
        }
    }
    outcome
}

pub fn execute(snapshot: &CleanupSnapshot) -> DeleteOutcome {
    let rule = match rule_for_snapshot(snapshot) {
        Ok(rule) => rule,
        Err(error) => {
            return DeleteOutcome {
                reclaimed_bytes: 0,
                failures: vec![DeleteFailure {
                    path: snapshot.root.clone(),
                    error,
                }],
            };
        }
    };

    let canonical_root = match validated_snapshot_root(snapshot, &rule) {
        Ok(canonical_root) => canonical_root,
        Err(error) => {
            return DeleteOutcome {
                reclaimed_bytes: 0,
                failures: vec![DeleteFailure {
                    path: snapshot.root.clone(),
                    error,
                }],
            };
        }
    };

    if let Some(guard) = rule.process_guard {
        let Some(process_names) = running_process_names() else {
            return DeleteOutcome {
                reclaimed_bytes: 0,
                failures: vec![DeleteFailure {
                    path: snapshot.root.clone(),
                    error: format!(
                        "无法确认 {} 的运行状态；为避免误清理活跃缓存，本次已安全跳过",
                        process_guard_name(guard)
                    ),
                }],
            };
        };
        if process_guard_blocks(guard, &process_names) {
            return DeleteOutcome {
                reclaimed_bytes: 0,
                failures: vec![DeleteFailure {
                    path: snapshot.root.clone(),
                    error: format!(
                        "检测到 {} 正在运行；为避免清理活跃缓存，本次已安全跳过",
                        process_guard_name(guard)
                    ),
                }],
            };
        }
    }

    delete_snapshot_files(snapshot, &canonical_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after the Unix epoch")
                .as_nanos();
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "qingpan-scanner-test-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            for entry in WalkDir::new(&self.0)
                .follow_links(false)
                .contents_first(true)
                .into_iter()
                .flatten()
            {
                if entry.path() == self.0 {
                    continue;
                }
                let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
                    continue;
                };
                if metadata.is_dir() && !is_link_or_reparse(&metadata) {
                    let _ = fs::remove_dir(entry.path());
                } else {
                    let _ = fs::remove_file(entry.path());
                }
            }
            let _ = fs::remove_dir(&self.0);
        }
    }

    fn test_rule() -> Rule {
        Rule {
            id: "test".into(),
            category: "test",
            name: "test".into(),
            base: RootBase::Local,
            relative: PathBuf::new(),
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: None,
            process_guard: None,
        }
    }

    fn rule_by_id(id: &str) -> Rule {
        rules()
            .into_iter()
            .find(|rule| rule.id == id)
            .expect("test rule should exist")
    }

    fn cleanup_snapshot(root: &Path) -> CleanupSnapshot {
        let directory = snapshot_directory(root, &test_rule(), SystemTime::now())
            .expect("directory should be snapshotted");
        CleanupSnapshot {
            item: CleanupItem {
                id: "test".into(),
                category: "test".into(),
                name: "test".into(),
                path: root.display().to_string(),
                description: String::new(),
                size_bytes: directory.files.iter().map(|file| file.size).sum(),
                risk: RiskLevel::Low,
                delete_mode: DeleteMode::Permanent,
            },
            root: directory.root,
            canonical_root: directory.canonical_root,
            files: directory.files,
        }
    }

    fn create_directory(root: &Path, relative: impl AsRef<Path>) {
        fs::create_dir_all(root.join(relative)).expect("test directory tree should be created");
    }

    fn create_profile_data(user_data: &Path, profile: &str) {
        let profile_root = user_data.join(profile);
        for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
            create_directory(&profile_root, cache_directory);
            fs::write(
                profile_root.join(cache_directory).join("cache-entry"),
                b"regenerable",
            )
            .expect("cache entry should be written");
        }
        for protected_directory in ["Local Storage", "IndexedDB", "Projects", "CacheStorage"] {
            create_directory(&profile_root, protected_directory);
        }
        for protected_file in ["Cookies", "History", "Login Data", "Bookmarks"] {
            fs::write(profile_root.join(protected_file), b"must keep")
                .expect("protected profile data should be written");
        }
    }

    #[test]
    fn ids_are_unique() {
        let rules = rules();
        let mut ids = rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), rules.len())
    }

    #[test]
    fn rules_are_not_high_risk() {
        assert!(rules()
            .iter()
            .all(|rule| !matches!(rule.risk, RiskLevel::High)))
    }

    #[test]
    fn browser_profile_discovery_only_creates_strict_cache_leaf_rules() {
        use std::collections::BTreeSet;

        let directory = TestDirectory::new();
        let local = directory.path().join("Local");
        let roaming = directory.path().join("Roaming");
        create_directory(&local, Path::new("Microsoft/Edge/User Data"));
        create_directory(&local, Path::new("Google/Chrome/User Data"));
        create_directory(&local, Path::new("Mozilla/Firefox/Profiles"));
        create_directory(&roaming, Path::new("placeholder"));

        let edge_user_data = local.join("Microsoft/Edge/User Data");
        for profile in ["Default", "Profile 1", "Profile Work"] {
            create_profile_data(&edge_user_data, profile);
        }
        create_profile_data(&edge_user_data, "Guest Profile");
        create_profile_data(&edge_user_data, "System Profile");

        let chrome_user_data = local.join("Google/Chrome/User Data");
        for profile in ["Default", "Profile 2"] {
            create_profile_data(&chrome_user_data, profile);
        }

        for profile in ["alpha.default-release", "beta.work"] {
            let profile_root = local.join("Mozilla/Firefox/Profiles").join(profile);
            create_directory(&profile_root, "cache2");
            fs::write(profile_root.join("cache2/cache-entry"), b"regenerable")
                .expect("Firefox cache entry should be written");
            create_directory(&profile_root, "storage");
            fs::write(profile_root.join("cookies.sqlite"), b"must keep")
                .expect("Firefox protected data should be written");
        }

        let discovered = rules_for_roots(Some(&local), Some(&roaming));
        let browser_rules = discovered
            .iter()
            .filter(|rule| {
                matches!(
                    rule.process_guard,
                    Some(ProcessGuard::Edge)
                        | Some(ProcessGuard::Chrome)
                        | Some(ProcessGuard::Firefox)
                )
            })
            .collect::<Vec<_>>();
        let actual = browser_rules
            .iter()
            .map(|rule| rule.relative.clone())
            .collect::<BTreeSet<_>>();

        let mut expected = BTreeSet::new();
        for profile in ["Default", "Profile 1", "Profile Work"] {
            for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
                expected.insert(
                    Path::new("Microsoft/Edge/User Data")
                        .join(profile)
                        .join(cache_directory),
                );
            }
        }
        for profile in ["Default", "Profile 2"] {
            for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
                expected.insert(
                    Path::new("Google/Chrome/User Data")
                        .join(profile)
                        .join(cache_directory),
                );
            }
        }
        for profile in ["alpha.default-release", "beta.work"] {
            expected.insert(
                Path::new("Mozilla/Firefox/Profiles")
                    .join(profile)
                    .join("cache2"),
            );
        }

        assert_eq!(actual, expected);
        let ids = browser_rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), browser_rules.len());
        let repeated_ids = rules_for_roots(Some(&local), Some(&roaming))
            .into_iter()
            .filter(|rule| {
                matches!(
                    rule.process_guard,
                    Some(ProcessGuard::Edge)
                        | Some(ProcessGuard::Chrome)
                        | Some(ProcessGuard::Firefox)
                )
            })
            .map(|rule| rule.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ids.into_iter().map(str::to_owned).collect::<BTreeSet<_>>(),
            repeated_ids
        );
        assert!(discovered.iter().all(|rule| {
            !rule.relative.components().any(|component| {
                let component = component.as_os_str().to_string_lossy();
                [
                    "Cookies",
                    "History",
                    "Login Data",
                    "Bookmarks",
                    "Local Storage",
                    "IndexedDB",
                    "Projects",
                    "CacheStorage",
                ]
                .iter()
                .any(|protected| component.eq_ignore_ascii_case(protected))
            })
        }));
    }

    #[test]
    fn application_rules_use_roaming_and_only_explicit_cache_directories() {
        use std::collections::BTreeSet;

        let rules = rules_for_roots(None, Some(Path::new("Roaming")));
        let application_rules = rules
            .iter()
            .filter(|rule| {
                matches!(
                    rule.process_guard,
                    Some(ProcessGuard::VsCode)
                        | Some(ProcessGuard::Discord)
                        | Some(ProcessGuard::Figma)
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(application_rules.len(), 9);
        assert!(application_rules
            .iter()
            .all(|rule| rule.base == RootBase::Roaming));
        let actual = application_rules
            .iter()
            .map(|rule| rule.relative.clone())
            .collect::<BTreeSet<_>>();
        let mut expected = BTreeSet::new();
        for application in ["Code", "discord", "Figma"] {
            for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
                expected.insert(Path::new(application).join(cache_directory));
            }
        }
        assert_eq!(actual, expected);
    }

    #[test]
    fn process_guard_matching_is_exact_case_insensitive_and_pure() {
        for (guard, process_name) in [
            (ProcessGuard::Edge, "MSEDGE.EXE"),
            (ProcessGuard::Chrome, "chrome.exe"),
            (ProcessGuard::Firefox, "firefox.exe"),
            (ProcessGuard::VsCode, "Code.exe"),
            (ProcessGuard::Discord, "Discord.exe"),
            (ProcessGuard::Figma, "Figma.exe"),
        ] {
            assert!(process_guard_blocks(guard, &[process_name.to_string()]));
        }

        let chrome_running = vec![
            "explorer.exe".to_string(),
            r"C:\Program Files\Google\Chrome\CHROME.EXE".to_string(),
        ];
        assert!(process_guard_blocks(ProcessGuard::Chrome, &chrome_running));
        assert!(!process_guard_blocks(ProcessGuard::Edge, &chrome_running));

        let lookalikes = vec![
            "chrome_proxy.exe".to_string(),
            "msedgewebview2.exe".to_string(),
            "discord-helper.exe".to_string(),
        ];
        assert!(!process_guard_blocks(ProcessGuard::Chrome, &lookalikes));
        assert!(!process_guard_blocks(ProcessGuard::Edge, &lookalikes));
        assert!(!process_guard_blocks(ProcessGuard::Discord, &lookalikes));

        let chrome_rule = cache_rule(
            "test-chrome-cache",
            "test",
            RootBase::Local,
            "cache",
            Some(ProcessGuard::Chrome),
        );
        assert!(!rule_is_executable(&chrome_rule, Some(&chrome_running)));
        assert!(rule_is_executable(&chrome_rule, Some(&lookalikes)));
        assert!(!rule_is_executable(&chrome_rule, None));
        assert!(rule_is_executable(&test_rule(), Some(&chrome_running)));
    }

    #[test]
    fn thumbs_only_snapshots_matching_root_files() {
        let directory = TestDirectory::new();
        let thumbcache = directory.path().join("thumbcache_256.db");
        let unrelated = directory.path().join("iconcache_256.db");
        let misleading = directory.path().join("thumbcache_256.db.backup");
        fs::write(&thumbcache, b"thumbnail cache").expect("thumbcache should be written");
        fs::write(&unrelated, b"unrelated explorer data")
            .expect("unrelated file should be written");
        fs::write(&misleading, b"not a cache database").expect("misleading file should be written");

        let snapshot =
            snapshot_directory(directory.path(), &rule_by_id("thumbs"), SystemTime::now())
                .expect("Explorer directory should be snapshotted");

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.files[0].path, thumbcache);
        assert!(unrelated.exists());
        assert!(misleading.exists());
    }

    #[test]
    fn temp_files_younger_than_72_hours_are_not_snapshotted() {
        let directory = TestDirectory::new();
        let recent_file = directory.path().join("recent.tmp");
        fs::write(&recent_file, b"recent temporary data").expect("temp file should be written");
        let modified = fs::symlink_metadata(&recent_file)
            .and_then(|metadata| metadata.modified())
            .expect("temp modified time should be readable");
        let rule = rule_by_id("temp");

        let recent_snapshot = snapshot_directory(directory.path(), &rule, modified)
            .expect("temp directory should be snapshotted");
        let eligible_snapshot =
            snapshot_directory(directory.path(), &rule, modified + TEMP_MINIMUM_AGE)
                .expect("temp directory should be snapshotted at cutoff");

        assert!(recent_snapshot.files.is_empty());
        assert_eq!(eligible_snapshot.files.len(), 1);
        assert_eq!(eligible_snapshot.files[0].path, recent_file);
    }

    #[test]
    fn unknown_rule_id_is_rejected_without_deleting() {
        let directory = TestDirectory::new();
        let file = directory.path().join("keep.tmp");
        fs::write(&file, b"keep").expect("test file should be written");
        let snapshot = cleanup_snapshot(directory.path());

        let outcome = execute(&snapshot);

        assert_eq!(outcome.reclaimed_bytes, 0);
        assert_eq!(outcome.failures.len(), 1);
        assert!(outcome.failures[0].error.contains("未知清理条目"));
        assert!(file.exists());
    }

    #[test]
    fn files_created_after_scan_are_not_deleted() {
        let directory = TestDirectory::new();
        let scanned_file = directory.path().join("scanned.tmp");
        let new_file = directory.path().join("created-after-scan.tmp");
        fs::write(&scanned_file, b"snapshot").expect("scanned file should be written");
        let snapshot = cleanup_snapshot(directory.path());

        fs::write(&new_file, b"new user data").expect("new file should be written");
        let outcome = delete_snapshot_files(&snapshot, &snapshot.canonical_root);

        assert_eq!(outcome.reclaimed_bytes, 8);
        assert!(outcome.failures.is_empty());
        assert!(!scanned_file.exists());
        assert!(new_file.exists());
    }

    #[test]
    fn files_modified_after_scan_are_skipped() {
        let directory = TestDirectory::new();
        let changed_file = directory.path().join("changed.tmp");
        fs::write(&changed_file, b"before").expect("initial file should be written");
        let snapshot = cleanup_snapshot(directory.path());

        fs::write(&changed_file, b"after with a different size")
            .expect("changed file should be written");
        let outcome = delete_snapshot_files(&snapshot, &snapshot.canonical_root);

        assert_eq!(outcome.reclaimed_bytes, 0);
        assert_eq!(outcome.failures.len(), 1);
        assert!(outcome.failures[0].error.contains("已变化"));
        assert!(changed_file.exists());
    }
}
