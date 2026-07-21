use crate::{
    browsers::{self, BrowserDataRoot, BrowserProcess},
    models::{CleanupItem, DeleteMode, RiskLevel},
};
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
const WECHAT_INSTALLATIONS: [(&str, &str, &str); 3] = [
    ("WeChat", "wechat", "微信"),
    ("Weixin", "weixin", "微信 4.x"),
    ("xwechat", "xwechat", "微信 4.x"),
];
const WECHAT_REGENERABLE_DIRECTORIES: [(&str, &str, &str, &str); 6] = [
    ("Cache", "cache", "微信运行缓存", "网络缓存"),
    ("Code Cache", "code-cache", "微信运行缓存", "代码缓存"),
    ("GPUCache", "gpu-cache", "微信运行缓存", "图形缓存"),
    ("Log", "log", "微信诊断数据", "运行日志"),
    ("Logs", "logs", "微信诊断数据", "运行日志"),
    (
        "Crashpad/reports",
        "crash-reports",
        "微信诊断数据",
        "崩溃报告",
    ),
];
const WECHAT_ATTACHMENT_IMAGE_DIRECTORIES: &[&str] = &["Image", "Thumb"];
const WECHAT_ATTACHMENT_VIDEO_DIRECTORIES: &[&str] = &["Video"];
const WECHAT_ATTACHMENT_FILE_DIRECTORIES: &[&str] = &["File"];
const WECHAT_ATTACHMENT_VOICE_DIRECTORIES: &[&str] = &["Audio", "Voice", "Voice2"];
const XWECHAT_ATTACHMENT_IMAGE_DIRECTORIES: &[&str] = &["Img"];
const XWECHAT_ATTACHMENT_VIDEO_DIRECTORIES: &[&str] = &["V"];
const XWECHAT_ATTACHMENT_VOICE_DIRECTORIES: &[&str] = &["Audio", "Voice", "Voice2"];

#[derive(Clone, Copy)]
pub enum FileMatcher {
    AllFilesRecursive,
    RootFileName {
        prefix: &'static str,
        suffix: &'static str,
    },
    DescendantDirectoryName {
        names: &'static [&'static str],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootBase {
    Local,
    Roaming,
    WeChatDocuments,
    XWeChatData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessGuard {
    Browser(BrowserProcess),
    VsCode,
    Discord,
    Figma,
    WeChat,
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

fn wechat_rule(
    id: impl Into<String>,
    category: &'static str,
    name: impl Into<String>,
    base: RootBase,
    relative: impl Into<PathBuf>,
) -> Rule {
    Rule {
        id: id.into(),
        category,
        name: name.into(),
        base,
        relative: relative.into(),
        risk: RiskLevel::Low,
        matcher: FileMatcher::AllFilesRecursive,
        minimum_age: None,
        process_guard: Some(ProcessGuard::WeChat),
    }
}

fn wechat_user_rule(
    id: impl Into<String>,
    category: &'static str,
    name: impl Into<String>,
    base: RootBase,
    relative: impl Into<PathBuf>,
    matcher: FileMatcher,
) -> Rule {
    Rule {
        id: id.into(),
        category,
        name: name.into(),
        base,
        relative: relative.into(),
        risk: RiskLevel::High,
        matcher,
        minimum_age: None,
        process_guard: Some(ProcessGuard::WeChat),
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

fn add_wechat_rules(rules: &mut Vec<Rule>, base: RootBase, base_id: &'static str) {
    for (installation, installation_id, product_name) in WECHAT_INSTALLATIONS {
        let installation_relative = Path::new("Tencent").join(installation);
        for (directory, directory_id, category, display_name) in WECHAT_REGENERABLE_DIRECTORIES {
            rules.push(wechat_rule(
                format!("wechat-{base_id}-{installation_id}-{directory_id}"),
                category,
                format!("{product_name} · {display_name}"),
                base,
                installation_relative.join(directory),
            ));
        }
    }
}

fn add_wechat_user_directory_rule(
    rules: &mut Vec<Rule>,
    documents_root: &Path,
    account: &str,
    account_id: &str,
    suffix: &'static str,
    category: &'static str,
    display_name: &'static str,
    relative: PathBuf,
) {
    if !trusted_directory(&documents_root.join(&relative)) {
        return;
    }
    rules.push(wechat_user_rule(
        format!("wechat-user-{account_id}-{suffix}"),
        category,
        format!("微信 · {account} · {display_name}"),
        RootBase::WeChatDocuments,
        relative,
        FileMatcher::AllFilesRecursive,
    ));
}

fn msg_attach_has_directory(root: &Path, names: &[&str]) -> bool {
    const MAX_DISCOVERY_DEPTH: usize = 6;
    const MAX_DISCOVERED_DIRECTORIES: usize = 50_000;

    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut discovered = 0usize;
    while let Some((parent, depth)) = pending.pop() {
        if depth >= MAX_DISCOVERY_DEPTH {
            continue;
        }
        for child in child_directory_names(&parent) {
            discovered += 1;
            if discovered > MAX_DISCOVERED_DIRECTORIES {
                return false;
            }
            if names.iter().any(|name| child.eq_ignore_ascii_case(name)) {
                return true;
            }
            pending.push((parent.join(child), depth + 1));
        }
    }
    false
}

fn add_wechat_attachment_rule(
    rules: &mut Vec<Rule>,
    documents_root: &Path,
    account: &str,
    account_id: &str,
    suffix: &'static str,
    category: &'static str,
    display_name: &'static str,
    relative: &Path,
    directory_names: &'static [&'static str],
) {
    let root = documents_root.join(relative);
    if !trusted_directory(&root) || !msg_attach_has_directory(&root, directory_names) {
        return;
    }
    rules.push(wechat_user_rule(
        format!("wechat-user-{account_id}-{suffix}"),
        category,
        format!("微信 · {account} · {display_name}"),
        RootBase::WeChatDocuments,
        relative,
        FileMatcher::DescendantDirectoryName {
            names: directory_names,
        },
    ));
}

fn discover_wechat_user_data_rules(rules: &mut Vec<Rule>, documents_root: &Path) {
    let accounts_relative = Path::new("WeChat Files");
    let accounts_root = documents_root.join(accounts_relative);
    for account in child_directory_names(&accounts_root) {
        if ["All Users", "Applet", "WMPF"]
            .iter()
            .any(|ignored| account.eq_ignore_ascii_case(ignored))
        {
            continue;
        }

        let account_relative = accounts_relative.join(&account);
        let account_root = documents_root.join(&account_relative);
        if !trusted_directory(&account_root.join("Msg"))
            && !trusted_directory(&account_root.join("FileStorage"))
        {
            continue;
        }
        let account_id = stable_id_component(&account);

        for (relative, suffix, category, display_name) in [
            ("Msg", "chat-records", "微信聊天记录", "聊天记录"),
            ("FileStorage/Image", "images", "微信图片", "聊天图片"),
            ("FileStorage/Video", "videos", "微信视频", "聊天视频"),
            ("FileStorage/File", "files", "微信文件", "聊天文件"),
            ("FileStorage/Fav", "favorites", "微信收藏", "收藏内容"),
            (
                "FileStorage/CustomEmotion",
                "emotions",
                "微信表情",
                "自定义表情",
            ),
            ("FileStorage/Audio", "audio", "微信语音", "语音消息"),
            ("FileStorage/Voice", "voice", "微信语音", "语音消息"),
            ("FileStorage/Voice2", "voice2", "微信语音", "语音消息"),
        ] {
            add_wechat_user_directory_rule(
                rules,
                documents_root,
                &account,
                &account_id,
                suffix,
                category,
                display_name,
                account_relative.join(relative),
            );
        }

        let msg_attach_relative = account_relative.join("FileStorage/MsgAttach");
        for (suffix, category, display_name, directory_names) in [
            (
                "attachment-images",
                "微信图片",
                "消息附件图片",
                WECHAT_ATTACHMENT_IMAGE_DIRECTORIES,
            ),
            (
                "attachment-videos",
                "微信视频",
                "消息附件视频",
                WECHAT_ATTACHMENT_VIDEO_DIRECTORIES,
            ),
            (
                "attachment-files",
                "微信文件",
                "消息附件文件",
                WECHAT_ATTACHMENT_FILE_DIRECTORIES,
            ),
            (
                "attachment-voices",
                "微信语音",
                "语音消息",
                WECHAT_ATTACHMENT_VOICE_DIRECTORIES,
            ),
        ] {
            add_wechat_attachment_rule(
                rules,
                documents_root,
                &account,
                &account_id,
                suffix,
                category,
                display_name,
                &msg_attach_relative,
                directory_names,
            );
        }
    }
}

fn add_xwechat_user_directory_rule(
    rules: &mut Vec<Rule>,
    data_root: &Path,
    account: &str,
    account_id: &str,
    suffix: &'static str,
    category: &'static str,
    display_name: &'static str,
    relative: PathBuf,
) {
    if !trusted_directory(&data_root.join(&relative)) {
        return;
    }
    rules.push(wechat_user_rule(
        format!("wechat-user-x-{account_id}-{suffix}"),
        category,
        format!("微信 4.x · {account} · {display_name}"),
        RootBase::XWeChatData,
        relative,
        FileMatcher::AllFilesRecursive,
    ));
}

fn add_xwechat_attachment_rule(
    rules: &mut Vec<Rule>,
    data_root: &Path,
    account: &str,
    account_id: &str,
    suffix: &'static str,
    category: &'static str,
    display_name: &'static str,
    relative: &Path,
    directory_names: &'static [&'static str],
) {
    let root = data_root.join(relative);
    if !trusted_directory(&root) || !msg_attach_has_directory(&root, directory_names) {
        return;
    }
    rules.push(wechat_user_rule(
        format!("wechat-user-x-{account_id}-{suffix}"),
        category,
        format!("微信 4.x · {account} · {display_name}"),
        RootBase::XWeChatData,
        relative,
        FileMatcher::DescendantDirectoryName {
            names: directory_names,
        },
    ));
}

fn discover_xwechat_user_data_rules(rules: &mut Vec<Rule>, data_root: &Path) {
    let accounts_relative = Path::new("xwechat_files");
    let accounts_root = data_root.join(accounts_relative);
    for account in child_directory_names(&accounts_root) {
        if account.eq_ignore_ascii_case("All Users") {
            continue;
        }

        let account_relative = accounts_relative.join(&account);
        let account_root = data_root.join(&account_relative);
        if !trusted_directory(&account_root.join("db_storage"))
            && !trusted_directory(&account_root.join("msg"))
        {
            continue;
        }
        let account_id = stable_id_component(&account);

        for (relative, suffix, category, display_name) in [
            (
                "db_storage/message",
                "chat-messages",
                "微信聊天记录",
                "聊天消息数据库",
            ),
            (
                "db_storage/session",
                "chat-sessions",
                "微信聊天记录",
                "会话索引",
            ),
            ("msg/file", "files", "微信文件", "聊天文件"),
            ("msg/video", "videos", "微信视频", "聊天视频"),
            ("db_storage/favorite", "favorites", "微信收藏", "收藏数据"),
            ("db_storage/emoticon", "emotions", "微信表情", "表情数据"),
            ("msg/audio", "audio", "微信语音", "语音消息"),
            ("msg/voice", "voice", "微信语音", "语音消息"),
            ("msg/voice2", "voice2", "微信语音", "语音消息"),
        ] {
            add_xwechat_user_directory_rule(
                rules,
                data_root,
                &account,
                &account_id,
                suffix,
                category,
                display_name,
                account_relative.join(relative),
            );
        }

        let attach_relative = account_relative.join("msg/attach");
        for (suffix, category, display_name, directory_names) in [
            (
                "attachment-images",
                "微信图片",
                "消息附件图片",
                XWECHAT_ATTACHMENT_IMAGE_DIRECTORIES,
            ),
            (
                "attachment-videos",
                "微信视频",
                "消息附件视频",
                XWECHAT_ATTACHMENT_VIDEO_DIRECTORIES,
            ),
            (
                "attachment-voices",
                "微信语音",
                "语音消息",
                XWECHAT_ATTACHMENT_VOICE_DIRECTORIES,
            ),
        ] {
            add_xwechat_attachment_rule(
                rules,
                data_root,
                &account,
                &account_id,
                suffix,
                category,
                display_name,
                &attach_relative,
                directory_names,
            );
        }
    }
}

fn rules_for_roots(
    local_root: Option<&Path>,
    roaming_root: Option<&Path>,
    wechat_documents_root: Option<&Path>,
    xwechat_data_root: Option<&Path>,
) -> Vec<Rule> {
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
    add_wechat_rules(&mut rules, RootBase::Local, "local");
    add_wechat_rules(&mut rules, RootBase::Roaming, "roaming");
    if let Some(documents_root) = wechat_documents_root {
        discover_wechat_user_data_rules(&mut rules, documents_root);
    }
    if let Some(data_root) = xwechat_data_root {
        discover_xwechat_user_data_rules(&mut rules, data_root);
    }

    for browser_rule in browsers::discover_cache_rules(local_root, roaming_root) {
        rules.push(Rule {
            id: browser_rule.id,
            category: "浏览器缓存",
            name: browser_rule.name,
            base: match browser_rule.base {
                BrowserDataRoot::Local => RootBase::Local,
                BrowserDataRoot::Roaming => RootBase::Roaming,
            },
            relative: browser_rule.relative,
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: None,
            process_guard: Some(ProcessGuard::Browser(browser_rule.process)),
        });
    }

    rules
}

pub fn rules() -> Vec<Rule> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    rules_for_roots(
        local_root.as_deref(),
        roaming_root.as_deref(),
        wechat_documents_root.as_deref(),
        xwechat_data_root.as_deref(),
    )
}

pub fn local_root() -> Option<PathBuf> {
    dirs::data_local_dir()
}

pub fn roaming_root() -> Option<PathBuf> {
    dirs::data_dir()
}

fn configured_wechat_documents_root(content: &str, default_root: &Path) -> PathBuf {
    let configured = content.lines().find_map(|line| {
        line.trim_start_matches('\u{feff}')
            .strip_prefix("MyDocument:")
            .map(str::trim)
    });
    let Some(configured) = configured.filter(|value| !value.is_empty()) else {
        return default_root.to_path_buf();
    };
    if configured.len() > 32_767 {
        return default_root.to_path_buf();
    }

    let configured = PathBuf::from(configured);
    if !configured.is_absolute() {
        return default_root.to_path_buf();
    }
    if configured
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("WeChat Files"))
    {
        return configured
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| default_root.to_path_buf());
    }
    configured
}

fn wechat_documents_root_for(
    roaming_root: Option<&Path>,
    default_documents_root: Option<&Path>,
) -> Option<PathBuf> {
    let default_root = default_documents_root?;
    let Some(roaming_root) = roaming_root else {
        return Some(default_root.to_path_buf());
    };
    let config_path = roaming_root.join("Tencent/WeChat/All Users/config/3ebffe94.ini");
    let Ok(bytes) = fs::read(config_path) else {
        return Some(default_root.to_path_buf());
    };
    if bytes.len() > 64 * 1024 {
        return Some(default_root.to_path_buf());
    }
    let content = String::from_utf8_lossy(&bytes);
    Some(configured_wechat_documents_root(&content, default_root))
}

pub fn wechat_documents_root() -> Option<PathBuf> {
    let roaming_root = roaming_root();
    let default_documents_root = dirs::document_dir();
    wechat_documents_root_for(roaming_root.as_deref(), default_documents_root.as_deref())
}

fn xwechat_data_root_for(
    roaming_root: Option<&Path>,
    default_home_root: Option<&Path>,
) -> Option<PathBuf> {
    let default_root = default_home_root?;
    let Some(roaming_root) = roaming_root else {
        return Some(default_root.to_path_buf());
    };
    let config_root = roaming_root.join("Tencent/xwechat/config");
    if !trusted_directory(&config_root) {
        return Some(default_root.to_path_buf());
    }
    let mut candidates = fs::read_dir(config_root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).ok()?;
            if is_link_or_reparse(&metadata) || !metadata.is_file() || metadata.len() > 4096 {
                return None;
            }
            let stem = path.file_stem()?.to_str()?;
            if path.extension()?.to_str()?.eq_ignore_ascii_case("ini")
                && stem.len() == 32
                && stem.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    candidates.sort();
    for path in candidates {
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let value = String::from_utf8_lossy(&bytes);
        let value = value.trim().trim_start_matches('\u{feff}');
        if value.is_empty() || value.len() > 32_767 || value.contains('\0') {
            continue;
        }
        let configured = PathBuf::from(value);
        if !configured.is_absolute() {
            continue;
        }
        let data_root = if configured
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("xwechat_files"))
        {
            let Some(parent) = configured.parent() else {
                continue;
            };
            parent.to_path_buf()
        } else {
            configured
        };
        if trusted_directory(&data_root.join("xwechat_files")) {
            return Some(data_root);
        }
    }
    Some(default_root.to_path_buf())
}

pub fn xwechat_data_root() -> Option<PathBuf> {
    let roaming_root = roaming_root();
    let home_root = dirs::home_dir();
    xwechat_data_root_for(roaming_root.as_deref(), home_root.as_deref())
}

fn base_root_for<'a>(
    rule: &Rule,
    local_root: Option<&'a Path>,
    roaming_root: Option<&'a Path>,
    wechat_documents_root: Option<&'a Path>,
    xwechat_data_root: Option<&'a Path>,
) -> Option<&'a Path> {
    match rule.base {
        RootBase::Local => local_root,
        RootBase::Roaming => roaming_root,
        RootBase::WeChatDocuments => wechat_documents_root,
        RootBase::XWeChatData => xwechat_data_root,
    }
}

pub fn path_for(rule: &Rule) -> Option<PathBuf> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let root = base_root_for(
        rule,
        local_root.as_deref(),
        roaming_root.as_deref(),
        wechat_documents_root.as_deref(),
        xwechat_data_root.as_deref(),
    )?;
    Some(root.join(&rule.relative))
}

fn guarded_process_names(guard: ProcessGuard) -> &'static [&'static str] {
    match guard {
        ProcessGuard::Browser(browser) => browser.names(),
        ProcessGuard::VsCode => &["code.exe", "code", "code-insiders.exe", "code-insiders"],
        ProcessGuard::Discord => &["discord.exe", "discord"],
        ProcessGuard::Figma => &["figma.exe", "figma"],
        ProcessGuard::WeChat => &[
            "wechat.exe",
            "wechat",
            "wechatappex.exe",
            "wechatappex",
            "weixin.exe",
            "weixin",
            "weixinappex.exe",
            "weixinappex",
        ],
    }
}

fn process_guard_name(guard: ProcessGuard) -> &'static str {
    match guard {
        ProcessGuard::Browser(browser) => browser.display_name(),
        ProcessGuard::VsCode => "Visual Studio Code",
        ProcessGuard::Discord => "Discord",
        ProcessGuard::Figma => "Figma",
        ProcessGuard::WeChat => "微信",
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

fn process_guard_error(rule: &Rule, process_names: Option<&[String]>) -> Option<String> {
    let guard = rule.process_guard?;
    let Some(process_names) = process_names else {
        return Some(format!(
            "无法确认 {} 的运行状态；为避免误清理正在使用的数据，本次已安全跳过",
            process_guard_name(guard)
        ));
    };
    process_guard_blocks(guard, process_names).then(|| {
        format!(
            "检测到 {} 正在运行；为避免误清理正在使用的数据，本次已安全跳过",
            process_guard_name(guard)
        )
    })
}

fn rule_is_executable(rule: &Rule, process_names: Option<&[String]>) -> bool {
    match (rule.process_guard, process_names) {
        (None, _) => true,
        (Some(guard), Some(process_names)) => !process_guard_blocks(guard, process_names),
        (Some(_), None) => false,
    }
}

fn rule_can_be_scanned(rule: &Rule, process_names: Option<&[String]>) -> bool {
    matches!(rule.process_guard, Some(ProcessGuard::Browser(_)))
        || rule_is_executable(rule, process_names)
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
        FileMatcher::DescendantDirectoryName { names } => path
            .components()
            .rev()
            .skip(1)
            .take(depth.saturating_sub(1))
            .any(|component| {
                let component = component.as_os_str().to_string_lossy();
                names
                    .iter()
                    .any(|name| component.eq_ignore_ascii_case(name))
            }),
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
    wechat_documents_root: Option<&Path>,
    xwechat_data_root: Option<&Path>,
    process_names: Option<&[String]>,
) -> Vec<CleanupSnapshot> {
    let scan_started_at = SystemTime::now();

    rules_for_roots(
        local_root,
        roaming_root,
        wechat_documents_root,
        xwechat_data_root,
    )
    .into_iter()
    .filter(|rule| rule_can_be_scanned(rule, process_names))
    .filter_map(|rule| {
        let base_root = base_root_for(
            &rule,
            local_root,
            roaming_root,
            wechat_documents_root,
            xwechat_data_root,
        )?;
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
        let is_user_data = matches!(&rule.risk, RiskLevel::High);
        if is_user_data && directory.files.is_empty() {
            return None;
        }
        let blocked_reason = matches!(rule.process_guard, Some(ProcessGuard::Browser(_)))
            .then(|| process_guard_error(&rule, process_names))
            .flatten();
        let item = CleanupItem {
            id: rule.id,
            category: rule.category.into(),
            name: rule.name,
            path: directory.root.display().to_string(),
            description: if is_user_data {
                "微信用户数据；只有主动选择并确认后才会永久删除".into()
            } else {
                "可由应用或 Windows 自动重新生成".into()
            },
            blocked_reason,
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
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let process_names = running_process_names();
    scan_with_environment(
        local_root.as_deref(),
        roaming_root.as_deref(),
        wechat_documents_root.as_deref(),
        xwechat_data_root.as_deref(),
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
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let base_root = base_root_for(
        rule,
        local_root.as_deref(),
        roaming_root.as_deref(),
        wechat_documents_root.as_deref(),
        xwechat_data_root.as_deref(),
    )
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

    if rule.process_guard.is_some() {
        let process_names = running_process_names();
        if let Some(error) = process_guard_error(&rule, process_names.as_deref()) {
            return DeleteOutcome {
                reclaimed_bytes: 0,
                failures: vec![DeleteFailure {
                    path: snapshot.root.clone(),
                    error,
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
                blocked_reason: None,
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
    fn only_explicit_wechat_user_data_rules_are_high_risk() {
        assert!(rules().iter().all(|rule| {
            !matches!(&rule.risk, RiskLevel::High)
                || (rule.id.starts_with("wechat-user-")
                    && matches!(rule.base, RootBase::WeChatDocuments | RootBase::XWeChatData)
                    && rule.process_guard == Some(ProcessGuard::WeChat))
        }))
    }

    #[test]
    fn wechat_documents_config_supports_default_and_absolute_custom_roots() {
        let directory = TestDirectory::new();
        let default_root = directory.path().join("Documents");
        let custom_root = directory.path().join("Custom WeChat Data");
        let custom_with_leaf = custom_root.join("WeChat Files");

        assert_eq!(
            configured_wechat_documents_root("MyDocument:", &default_root),
            default_root
        );
        assert_eq!(
            configured_wechat_documents_root(
                &format!("MyDocument:{}", custom_root.display()),
                &default_root,
            ),
            custom_root
        );
        assert_eq!(
            configured_wechat_documents_root(
                &format!("MyDocument:{}", custom_with_leaf.display()),
                &default_root,
            ),
            custom_with_leaf
                .parent()
                .expect("custom root should have a parent")
        );
        assert_eq!(
            configured_wechat_documents_root("MyDocument:relative/path", &default_root),
            default_root
        );
    }

    #[test]
    fn xwechat_config_supports_an_absolute_data_root() {
        let directory = TestDirectory::new();
        let roaming = directory.path().join("Roaming");
        let default_home = directory.path().join("Home");
        let configured = directory.path().join("Configured Home");
        let config_root = roaming.join("Tencent/xwechat/config");
        create_directory(&config_root, "");
        create_directory(&configured, "xwechat_files");
        fs::write(
            config_root.join("51a1fffea11325a1e4104c6b3de47af7.ini"),
            configured.display().to_string(),
        )
        .expect("xwechat path config should be written");
        fs::write(config_root.join("untrusted.ini"), b"C:\\wrong")
            .expect("untrusted config fixture should be written");

        assert_eq!(
            xwechat_data_root_for(Some(&roaming), Some(&default_home)),
            Some(configured)
        );
        assert_eq!(
            xwechat_data_root_for(None, Some(&default_home)),
            Some(default_home)
        );
    }

    #[test]
    fn wechat_user_data_scan_classifies_explicit_directories_and_skips_unknown_data() {
        use std::collections::BTreeSet;

        let directory = TestDirectory::new();
        let documents = directory.path().join("Documents");
        let account = documents.join("WeChat Files/account-test");
        for (relative, content) in [
            ("Msg/message.db", b"message".as_slice()),
            ("FileStorage/Image/2026-07/image.dat", b"image".as_slice()),
            ("FileStorage/Video/2026-07/video.mp4", b"video".as_slice()),
            ("FileStorage/File/2026-07/file.zip", b"file".as_slice()),
            ("FileStorage/Fav/favorite.dat", b"favorite".as_slice()),
            (
                "FileStorage/CustomEmotion/emotion.dat",
                b"emotion".as_slice(),
            ),
            (
                "FileStorage/MsgAttach/contact/Image/2026-07/attachment.dat",
                b"attachment-image".as_slice(),
            ),
            (
                "FileStorage/MsgAttach/contact/Audio/2026-07/voice.dat",
                b"voice".as_slice(),
            ),
            (
                "FileStorage/MsgAttach/contact/Unknown/keep.dat",
                b"must-keep".as_slice(),
            ),
            ("FileStorage/Temp/keep.tmp", b"must-keep".as_slice()),
            ("FileStorage/General/keep.dat", b"must-keep".as_slice()),
        ] {
            let path = account.join(relative);
            create_directory(
                path.parent().expect("fixture file should have a parent"),
                "",
            );
            fs::write(path, content).expect("fixture file should be written");
        }

        let rules = rules_for_roots(None, None, Some(&documents), None);
        let user_rules = rules
            .iter()
            .filter(|rule| rule.id.starts_with("wechat-user-"))
            .collect::<Vec<_>>();
        assert_eq!(user_rules.len(), 8);
        assert!(user_rules.iter().all(|rule| {
            matches!(&rule.risk, RiskLevel::High)
                && rule.base == RootBase::WeChatDocuments
                && rule.relative.starts_with("WeChat Files/account-test")
        }));
        assert!(user_rules.iter().all(|rule| {
            !rule.relative.components().any(|component| {
                ["Temp", "General"]
                    .iter()
                    .any(|name| component.as_os_str().eq_ignore_ascii_case(name))
            })
        }));

        let closed = Vec::<String>::new();
        let snapshots = scan_with_environment(None, None, Some(&documents), None, Some(&closed));
        let categories = snapshots
            .iter()
            .map(|snapshot| snapshot.item.category.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            categories,
            BTreeSet::from([
                "微信聊天记录",
                "微信图片",
                "微信视频",
                "微信文件",
                "微信语音",
                "微信收藏",
                "微信表情",
            ])
        );
        assert!(snapshots.iter().all(|snapshot| {
            matches!(&snapshot.item.risk, RiskLevel::High) && !snapshot.files.is_empty()
        }));
        assert!(snapshots
            .iter()
            .flat_map(|snapshot| &snapshot.files)
            .all(|file| {
                file.path
                    .strip_prefix(&account)
                    .expect("snapshot should stay under the account root")
                    .components()
                    .all(|component| {
                        !["Unknown", "Temp", "General"].iter().any(|name| {
                            component
                                .as_os_str()
                                .to_string_lossy()
                                .eq_ignore_ascii_case(name)
                        })
                    })
            }));

        let running = vec!["WeChat.exe".to_string()];
        assert!(
            scan_with_environment(None, None, Some(&documents), None, Some(&running)).is_empty()
        );
    }

    #[test]
    fn xwechat_user_data_scan_classifies_only_known_new_client_directories() {
        use std::collections::BTreeSet;

        let directory = TestDirectory::new();
        let data_root = directory.path().join("Home");
        let account = data_root.join("xwechat_files/account-new");
        for (relative, content) in [
            ("db_storage/message/message.db", b"message".as_slice()),
            ("db_storage/session/session.db", b"session".as_slice()),
            ("db_storage/favorite/favorite.db", b"favorite".as_slice()),
            ("db_storage/emoticon/emoticon.db", b"emoticon".as_slice()),
            ("msg/file/document.pdf", b"file".as_slice()),
            ("msg/video/video.mp4", b"video".as_slice()),
            (
                "msg/attach/contact/2026-07/Img/image.dat",
                b"image".as_slice(),
            ),
            (
                "msg/attach/contact/2026-07/Rec/item/V/video.mp4",
                b"attachment-video".as_slice(),
            ),
            (
                "msg/attach/contact/2026-07/Audio/voice.dat",
                b"voice".as_slice(),
            ),
            (
                "msg/attach/contact/2026-07/Unknown/keep.dat",
                b"must-keep".as_slice(),
            ),
            ("cache/keep.dat", b"must-keep".as_slice()),
            ("temp/keep.dat", b"must-keep".as_slice()),
        ] {
            let path = account.join(relative);
            create_directory(
                path.parent().expect("fixture file should have a parent"),
                "",
            );
            fs::write(path, content).expect("fixture file should be written");
        }

        let rules = rules_for_roots(None, None, None, Some(&data_root));
        let user_rules = rules
            .iter()
            .filter(|rule| rule.id.starts_with("wechat-user-x-"))
            .collect::<Vec<_>>();
        assert_eq!(user_rules.len(), 9);
        assert!(user_rules.iter().all(|rule| {
            matches!(&rule.risk, RiskLevel::High)
                && rule.base == RootBase::XWeChatData
                && rule.relative.starts_with("xwechat_files/account-new")
        }));

        let closed = Vec::<String>::new();
        let snapshots = scan_with_environment(None, None, None, Some(&data_root), Some(&closed));
        let categories = snapshots
            .iter()
            .map(|snapshot| snapshot.item.category.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            categories,
            BTreeSet::from([
                "微信聊天记录",
                "微信图片",
                "微信视频",
                "微信文件",
                "微信语音",
                "微信收藏",
                "微信表情",
            ])
        );
        assert!(snapshots
            .iter()
            .flat_map(|snapshot| &snapshot.files)
            .all(|file| {
                file.path
                    .strip_prefix(&account)
                    .expect("snapshot should stay under the xwechat account root")
                    .components()
                    .all(|component| {
                        !["Unknown", "cache", "temp"].iter().any(|name| {
                            component
                                .as_os_str()
                                .to_string_lossy()
                                .eq_ignore_ascii_case(name)
                        })
                    })
            }));

        let running = vec!["Weixin.exe".to_string()];
        assert!(
            scan_with_environment(None, None, None, Some(&data_root), Some(&running)).is_empty()
        );
    }

    #[test]
    fn browser_profile_discovery_only_creates_strict_cache_leaf_rules() {
        use std::collections::BTreeSet;

        let directory = TestDirectory::new();
        let local = directory.path().join("Local");
        let roaming = directory.path().join("Roaming");
        create_directory(&local, Path::new("Microsoft/Edge/User Data"));
        create_directory(&local, Path::new("Google/Chrome/User Data"));
        create_directory(&local, Path::new("Vivaldi/User Data"));
        create_directory(&local, Path::new("Opera Software/Opera GX Stable"));
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

        create_profile_data(&local.join("Vivaldi/User Data"), "Default");
        for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
            let opera_root = local.join("Opera Software/Opera GX Stable");
            create_directory(&opera_root, cache_directory);
            fs::write(
                opera_root.join(cache_directory).join("cache-entry"),
                b"regenerable",
            )
            .expect("Opera GX cache entry should be written");
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

        let discovered = rules_for_roots(Some(&local), Some(&roaming), None, None);
        let browser_rules = discovered
            .iter()
            .filter(|rule| matches!(rule.process_guard, Some(ProcessGuard::Browser(_))))
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
        for (cache_directory, _) in REGENERABLE_CACHE_DIRECTORIES {
            expected.insert(
                Path::new("Vivaldi/User Data")
                    .join("Default")
                    .join(cache_directory),
            );
            expected.insert(Path::new("Opera Software/Opera GX Stable").join(cache_directory));
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
        let repeated_ids = rules_for_roots(Some(&local), Some(&roaming), None, None)
            .into_iter()
            .filter(|rule| matches!(rule.process_guard, Some(ProcessGuard::Browser(_))))
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
    fn running_browser_is_scanned_read_only_and_marked_unavailable() {
        let directory = TestDirectory::new();
        let local = directory.path().join("Local");
        let roaming = directory.path().join("Roaming");
        create_directory(&local, Path::new("Google/Chrome/User Data"));
        create_directory(&roaming, Path::new("placeholder"));
        create_profile_data(&local.join("Google/Chrome/User Data"), "Default");

        let running = vec!["chrome.exe".to_string()];
        let snapshots =
            scan_with_environment(Some(&local), Some(&roaming), None, None, Some(&running));
        let chrome = snapshots
            .iter()
            .filter(|snapshot| snapshot.item.category == "浏览器缓存")
            .collect::<Vec<_>>();

        assert_eq!(chrome.len(), REGENERABLE_CACHE_DIRECTORIES.len());
        assert!(chrome.iter().all(|snapshot| {
            snapshot
                .item
                .blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("Google Chrome 正在运行"))
        }));
    }

    #[test]
    fn application_rules_use_roaming_and_only_explicit_cache_directories() {
        use std::collections::BTreeSet;

        let rules = rules_for_roots(None, Some(Path::new("Roaming")), None, None);
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
    fn wechat_rules_only_target_explicit_regenerable_leaf_directories() {
        let discovered = rules_for_roots(None, None, None, None);
        let wechat_rules = discovered
            .iter()
            .filter(|rule| rule.process_guard == Some(ProcessGuard::WeChat))
            .collect::<Vec<_>>();

        assert_eq!(wechat_rules.len(), 36);
        for rule in &wechat_rules {
            assert!(matches!(rule.base, RootBase::Local | RootBase::Roaming));
            assert!(rule.category.starts_with("微信"));
            assert!(matches!(rule.risk, RiskLevel::Low));
            assert!(matches!(rule.matcher, FileMatcher::AllFilesRecursive));

            let relative = rule.relative.to_string_lossy().replace('\\', "/");
            assert!(WECHAT_INSTALLATIONS.iter().any(|(installation, _, _)| {
                WECHAT_REGENERABLE_DIRECTORIES
                    .iter()
                    .any(|(leaf, _, _, _)| {
                        relative.eq_ignore_ascii_case(&format!("Tencent/{installation}/{leaf}"))
                    })
            }));
            assert!(![
                "WeChat Files",
                "xwechat_files",
                "FileStorage",
                "MsgAttach",
                "Image",
                "Video",
                "File",
                "Voice2",
                "Emotion",
                "Temp",
            ]
            .iter()
            .any(|protected| {
                rule.relative.components().any(|component| {
                    component
                        .as_os_str()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(protected)
                })
            }));
        }
    }

    #[test]
    fn wechat_running_during_scan_skips_all_wechat_rules() {
        let directory = TestDirectory::new();
        let local = directory.path().join("Local");
        let roaming = directory.path().join("Roaming");
        create_directory(&local, "Tencent/WeChat/Cache");
        create_directory(&roaming, "placeholder");
        fs::write(
            local.join("Tencent/WeChat/Cache/cache-entry"),
            b"regenerable",
        )
        .expect("WeChat cache entry should be written");

        let closed = Vec::<String>::new();
        let closed_scan =
            scan_with_environment(Some(&local), Some(&roaming), None, None, Some(&closed));
        assert_eq!(
            closed_scan
                .iter()
                .filter(|snapshot| snapshot.item.category.starts_with("微信"))
                .count(),
            1
        );

        let running = vec!["WeChat.exe".to_string()];
        let running_scan =
            scan_with_environment(Some(&local), Some(&roaming), None, None, Some(&running));
        assert!(running_scan
            .iter()
            .all(|snapshot| !snapshot.item.category.starts_with("微信")));
    }

    #[test]
    fn execution_guard_rejects_wechat_restarted_after_scan() {
        let rule = rules_for_roots(None, None, None, None)
            .into_iter()
            .find(|rule| rule.id == "wechat-local-wechat-cache")
            .expect("WeChat cache rule should exist");
        let closed = Vec::<String>::new();
        assert!(process_guard_error(&rule, Some(&closed)).is_none());

        let restarted = vec!["Weixin.exe".to_string()];
        let error = process_guard_error(&rule, Some(&restarted))
            .expect("running WeChat should block execution recheck");
        assert!(error.contains("微信"));
        assert!(error.contains("正在运行"));
        let unknown_state =
            process_guard_error(&rule, None).expect("unknown process state should fail closed");
        assert!(unknown_state.contains("无法确认"));
        assert!(unknown_state.contains("微信"));
    }

    #[test]
    fn process_guard_matching_is_exact_case_insensitive_and_pure() {
        for (guard, process_name) in [
            (ProcessGuard::Browser(BrowserProcess::Edge), "MSEDGE.EXE"),
            (ProcessGuard::Browser(BrowserProcess::Chrome), "chrome.exe"),
            (
                ProcessGuard::Browser(BrowserProcess::Firefox),
                "firefox.exe",
            ),
            (ProcessGuard::VsCode, "Code.exe"),
            (ProcessGuard::Discord, "Discord.exe"),
            (ProcessGuard::Figma, "Figma.exe"),
            (ProcessGuard::WeChat, "WECHAT.EXE"),
            (ProcessGuard::WeChat, "Weixin.exe"),
            (ProcessGuard::WeChat, "WeChatAppEx.exe"),
            (ProcessGuard::WeChat, "WeixinAppEx.exe"),
        ] {
            assert!(process_guard_blocks(guard, &[process_name.to_string()]));
        }

        let chrome_running = vec![
            "explorer.exe".to_string(),
            r"C:\Program Files\Google\Chrome\CHROME.EXE".to_string(),
        ];
        assert!(process_guard_blocks(
            ProcessGuard::Browser(BrowserProcess::Chrome),
            &chrome_running
        ));
        assert!(!process_guard_blocks(
            ProcessGuard::Browser(BrowserProcess::Edge),
            &chrome_running
        ));

        let lookalikes = vec![
            "chrome_proxy.exe".to_string(),
            "msedgewebview2.exe".to_string(),
            "discord-helper.exe".to_string(),
            "wechat-helper.exe".to_string(),
            "weixin-updater.exe".to_string(),
        ];
        assert!(!process_guard_blocks(
            ProcessGuard::Browser(BrowserProcess::Chrome),
            &lookalikes
        ));
        assert!(!process_guard_blocks(
            ProcessGuard::Browser(BrowserProcess::Edge),
            &lookalikes
        ));
        assert!(!process_guard_blocks(ProcessGuard::Discord, &lookalikes));
        assert!(!process_guard_blocks(ProcessGuard::WeChat, &lookalikes));

        let chrome_rule = cache_rule(
            "test-chrome-cache",
            "test",
            RootBase::Local,
            "cache",
            Some(ProcessGuard::Browser(BrowserProcess::Chrome)),
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
