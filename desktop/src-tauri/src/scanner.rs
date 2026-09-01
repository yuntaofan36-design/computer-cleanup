use crate::{
    browsers::{self, BrowserDataRoot, BrowserProcess},
    fs_safety::{
        file_identity_from_file, hard_link_count_from_file, has_only_default_data_stream,
        is_link_or_reparse, is_offline_or_recall, FileIdentity,
    },
    models::{CleanupItem, DeleteMode, RiskLevel},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::{self, File, Metadata},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, SystemTime},
};
use sysinfo::{ProcessesToUpdate, System};
use walkdir::WalkDir;

const TEMP_MINIMUM_AGE: Duration = Duration::from_secs(72 * 60 * 60);

/// Upper bound on threads used to verify candidate files.
///
/// Verification is dominated by waiting, not computing: each file needs a handle
/// open, and on-access antivirus inspection can turn that into tens of
/// milliseconds. Measured on a real Maven repository with Defender real-time
/// protection enabled, over disjoint slices of 558 `.jar` files so no arm
/// benefited from another's cached verdict:
///
/// | threads | ms/file |
/// |---------|---------|
/// | 1       | 77.52   |
/// | 8       | 14.51   |
/// | 16      | 7.20    |
/// | 32      | 6.28    |
///
/// Sixteen roughly doubles eight's throughput because the extra threads overlap
/// scan latency rather than compete for cores. Thirty-two adds only 15% more, so
/// the cap stays at sixteen to leave the machine responsive while a scan runs.
/// The pool is also capped by the actual candidate count, so small rules never
/// spawn idle threads.
const MAX_SNAPSHOT_VERIFY_THREADS: usize = 16;

/// Candidate count below which verification stays on the calling thread.
///
/// Spawning a pool costs more than it saves for a handful of files, and most
/// discovered rules are small.
const MIN_CANDIDATES_FOR_PARALLEL_VERIFY: usize = 64;
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

/// Regenerable Windows diagnostic and cache locations, as
/// `(id, display name, base, relative)`.
///
/// Every entry is user-writable and refilled by Windows or an application on
/// demand, so it needs no elevation and no privileged interface.
///
/// Deliberately excluded, because they are not user-owned files that this
/// scanner may delete directly:
/// - `C:\Windows\SoftwareDistribution\Download` and `Windows\Logs` require
///   elevation and, for Update state, stopping services first. Per ADR-005 that
///   belongs to an on-demand elevated executor, not this pass.
/// - `C:\Windows\Installer` holds MSI/MSP caches that programs need in order to
///   repair, patch and uninstall; removing them breaks those operations.
/// - The registry is out of scope entirely: roadmap section 15.3 keeps registry
///   work as a non-committed candidate needing its own threat model.
const SYSTEM_JUNK_RULES: [(&str, &str, RootBase, &str); 5] = [
    (
        "system-crash-dumps",
        "Windows · 应用崩溃转储",
        RootBase::Local,
        "CrashDumps",
    ),
    (
        "system-wer-reports",
        "Windows · 错误报告存档",
        RootBase::Local,
        "Microsoft/Windows/WER",
    ),
    (
        "system-inet-cache",
        "Windows · 联网组件缓存",
        RootBase::Local,
        "Microsoft/Windows/INetCache",
    ),
    (
        "system-shader-cache",
        "Windows · D3D 着色器缓存",
        RootBase::Local,
        "D3DSCache",
    ),
    (
        "system-component-cache",
        "Windows · 组件图标缓存",
        RootBase::Local,
        "Microsoft/Windows/Caches",
    ),
];

/// Tencent-family regenerable caches and logs, as `(id, display name, relative)`.
///
/// Every path here was confirmed to exist on a real installation rather than
/// inferred from the usual Electron layout: QQNT keeps no `Cache`/`GPUCache`
/// directories under its application data root, so rules named after those would
/// never match. Chat data is deliberately absent — QQ history lives under
/// `Documents\Tencent Files\nt_qq\global\nt_db`, and opening a delete path to it
/// requires the production-grade quarantine that is still gated.
const TENCENT_CACHE_RULES: [(&str, &str, &str); 4] = [
    ("qq-shared-logs", "腾讯 · 共享运行日志", "Tencent/Logs"),
    ("qq-temp", "QQ · 发送暂存目录", "Tencent/QQ/STemp"),
    (
        "qqlive-cache",
        "腾讯视频 · 网页内核缓存",
        "Tencent/QQLive/Webkit3/Cache",
    ),
    (
        "wemeet-logs",
        "腾讯会议 · 运行日志",
        "Tencent/WeMeet/Global/Logs",
    ),
];

/// Tencent products that version their cache directories, as
/// `(id, display name, relative root)`.
///
/// WeGame stores caches under version-stamped parents such as `qbcore109\cache`,
/// and TenioDL under numeric ids such as `1601\cache`. Pointing a rule at the
/// parent would sweep in configuration and installed payloads, so these match
/// only files that sit beneath a directory literally named `cache`.
const TENCENT_VERSIONED_CACHE_RULES: [(&str, &str, &str); 2] = [
    ("wegame-core-cache", "WeGame · 核心缓存", "Tencent/WeGame"),
    (
        "tenio-download-cache",
        "腾讯下载组件 · 缓存",
        "Tencent/TenioDL",
    ),
];

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
    Home,
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
    Qq,
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
    identity: FileIdentity,
}

#[derive(Clone, Debug)]
pub struct CleanupSnapshot {
    item: CleanupItem,
    root: PathBuf,
    canonical_root: PathBuf,
    files: Vec<FileSnapshot>,
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutionFile {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
}

impl CleanupSnapshot {
    pub fn item(&self) -> &CleanupItem {
        &self.item
    }

    #[cfg(test)]
    pub(crate) fn test_snapshot(id: &str, marker: &str, risk: RiskLevel) -> Self {
        Self::test_snapshot_with_mode(id, marker, risk, DeleteMode::Permanent, 1, 1)
    }

    #[cfg(test)]
    pub(crate) fn test_snapshot_with_mode(
        id: &str,
        marker: &str,
        risk: RiskLevel,
        delete_mode: DeleteMode,
        file_count: usize,
        size_bytes: u64,
    ) -> Self {
        let root = PathBuf::from(marker);
        Self {
            item: CleanupItem {
                id: id.to_string(),
                category: "test".into(),
                name: id.to_string(),
                path: marker.to_string(),
                description: String::new(),
                blocked_reason: None,
                size_bytes,
                file_count,
                risk,
                delete_mode,
            },
            canonical_root: root.clone(),
            root,
            files: Vec::new(),
        }
    }
}

pub(crate) fn execution_files(snapshot: &CleanupSnapshot) -> Vec<ExecutionFile> {
    snapshot
        .files
        .iter()
        .map(|file| ExecutionFile {
            path: file.path.clone(),
            file_name: file
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size: file.size,
        })
        .collect()
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

/// Regenerable package-manager caches, as `(id, display name, base, relative)`.
///
/// Every entry points at a directory the tool refills on demand from a remote
/// registry, so removing it costs download time and nothing else.
///
/// Each path is deliberately a specific cache subdirectory rather than the tool's
/// root, because those roots also hold state that must survive:
/// - `~/.cargo/bin` contains installed executables.
/// - `~/.cargo/registry/index` is refetched but drives offline resolution.
/// - `~/.gradle/wrapper` holds downloaded Gradle distributions.
/// - `npm-cache/_npx` and `_logs` are not content-addressable package data.
/// - `npm-cache/_cacache/tmp` is npm's staging area for in-flight writes, so an
///   `npm install` running during cleanup would have its scratch files removed.
///
/// pnpm's store is intentionally absent. It is content-addressable and hard-links
/// its files into every project's `node_modules`, so deleting files underneath it
/// corrupts live projects instead of freeing space. The existing single-hard-link
/// guard already refuses those files; `pnpm store prune` is the supported route
/// and needs a command-orchestration action this scanner does not provide.
const DEVELOPER_CACHE_RULES: [(&str, &str, RootBase, &str); 8] = [
    (
        "npm-cache-content",
        "npm · 包内容缓存",
        RootBase::Local,
        "npm-cache/_cacache/content-v2",
    ),
    (
        "npm-cache-index",
        "npm · 包索引缓存",
        RootBase::Local,
        "npm-cache/_cacache/index-v5",
    ),
    (
        "pip-http-cache",
        "pip · 下载缓存",
        RootBase::Local,
        "pip/cache/http-v2",
    ),
    (
        "pip-wheel-cache",
        "pip · 构建 wheel 缓存",
        RootBase::Local,
        "pip/cache/wheels",
    ),
    (
        "cargo-registry-cache",
        "Cargo · crate 归档缓存",
        RootBase::Home,
        ".cargo/registry/cache",
    ),
    (
        "cargo-registry-src",
        "Cargo · crate 解压源码",
        RootBase::Home,
        ".cargo/registry/src",
    ),
    (
        "gradle-cache",
        "Gradle · 构建缓存",
        RootBase::Home,
        ".gradle/caches",
    ),
    (
        "maven-repository",
        "Maven · 本地仓库",
        RootBase::Home,
        ".m2/repository",
    ),
];

fn add_developer_cache_rules(rules: &mut Vec<Rule>) {
    for (id, name, base, relative) in DEVELOPER_CACHE_RULES {
        rules.push(Rule {
            id: id.into(),
            category: "开发者缓存",
            name: name.into(),
            base,
            relative: PathBuf::from(relative),
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: None,
            // These caches are safe to remove while a build runs: the tools treat a
            // missing entry as a miss and refetch. Guarding on every possible build
            // process would be unreliable, and the per-file identity recheck already
            // refuses anything that changes mid-cleanup.
            process_guard: None,
        });
    }
}

fn add_system_junk_rules(rules: &mut Vec<Rule>) {
    for (id, name, base, relative) in SYSTEM_JUNK_RULES {
        rules.push(Rule {
            id: id.into(),
            category: "系统垃圾",
            name: name.into(),
            base,
            relative: PathBuf::from(relative),
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: None,
            process_guard: None,
        });
    }
}

fn add_qq_rules(rules: &mut Vec<Rule>) {
    for (id, name, relative) in TENCENT_CACHE_RULES {
        rules.push(Rule {
            id: id.into(),
            category: "QQ 缓存",
            name: name.into(),
            base: RootBase::Roaming,
            relative: PathBuf::from(relative),
            risk: RiskLevel::Low,
            matcher: FileMatcher::AllFilesRecursive,
            minimum_age: None,
            // A running client can hold these open and rewrite them mid-scan, so the
            // same guard used for WeChat applies.
            process_guard: Some(ProcessGuard::Qq),
        });
    }
    for (id, name, relative) in TENCENT_VERSIONED_CACHE_RULES {
        rules.push(Rule {
            id: id.into(),
            category: "QQ 缓存",
            name: name.into(),
            base: RootBase::Roaming,
            relative: PathBuf::from(relative),
            risk: RiskLevel::Low,
            matcher: FileMatcher::DescendantDirectoryName { names: &["cache"] },
            minimum_age: None,
            process_guard: Some(ProcessGuard::Qq),
        });
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
    add_developer_cache_rules(&mut rules);
    add_system_junk_rules(&mut rules);
    add_qq_rules(&mut rules);
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

/// Base directory for tool caches that live directly under the user profile.
///
/// Cargo, Gradle and Maven place their caches in `~/.cargo`, `~/.gradle` and
/// `~/.m2` rather than under `AppData`, so they need this base.
pub fn home_root() -> Option<PathBuf> {
    dirs::home_dir()
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
    home_root: Option<&'a Path>,
    wechat_documents_root: Option<&'a Path>,
    xwechat_data_root: Option<&'a Path>,
) -> Option<&'a Path> {
    match rule.base {
        RootBase::Local => local_root,
        RootBase::Roaming => roaming_root,
        RootBase::Home => home_root,
        RootBase::WeChatDocuments => wechat_documents_root,
        RootBase::XWeChatData => xwechat_data_root,
    }
}

pub fn path_for(rule: &Rule) -> Option<PathBuf> {
    let local_root = local_root();
    let roaming_root = roaming_root();
    let home_root = home_root();
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let root = base_root_for(
        rule,
        local_root.as_deref(),
        roaming_root.as_deref(),
        home_root.as_deref(),
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
        ProcessGuard::Qq => &["qq.exe", "qq", "qqnt.exe", "qqnt", "qqprotect.exe"],
    }
}

fn process_guard_name(guard: ProcessGuard) -> &'static str {
    match guard {
        ProcessGuard::Browser(browser) => browser.display_name(),
        ProcessGuard::VsCode => "Visual Studio Code",
        ProcessGuard::Discord => "Discord",
        ProcessGuard::Figma => "Figma",
        ProcessGuard::WeChat => "微信",
        ProcessGuard::Qq => "QQ",
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
        if is_link_or_reparse(&metadata) || is_offline_or_recall(&metadata) || !metadata.is_dir() {
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

/// Captures a stable identity for `path`, or `None` when the file must not be
/// touched.
///
/// This opens a handle, which dominates scan cost: roughly 26s per 30k files on
/// measured hardware, versus 2s for a metadata-only pass. The expense is
/// deliberate and cannot be deferred to deletion time, because
/// `validate_snapshot_file` proves a file is unchanged by comparing the current
/// FileId against the one recorded here. Without a scan-time baseline there is
/// nothing to compare against and the TOCTOU guarantee is lost, so a faster scan
/// would be paid for with a weaker safety guarantee.
fn stable_file_identity(
    path: &Path,
    metadata: &Metadata,
    modified: SystemTime,
) -> Option<FileIdentity> {
    if !metadata.is_file() || is_link_or_reparse(metadata) || is_offline_or_recall(metadata) {
        return None;
    }
    let file = File::open(path).ok()?;
    let handle_metadata = file.metadata().ok()?;
    // The alternate-data-stream check is deliberately performed only here, after the
    // handle exists. Enumerating streams costs a syscall per file (2.7s per 30k
    // files measured), and doing it again before the open added no guarantee: both
    // calls resolve the same path, and this one is the stronger of the two because
    // the handle pins the file while identity, size and mtime are re-verified.
    if !handle_metadata.is_file()
        || is_link_or_reparse(&handle_metadata)
        || is_offline_or_recall(&handle_metadata)
        || handle_metadata.len() != metadata.len()
        || handle_metadata.modified().ok()? != modified
        || hard_link_count_from_file(&file, &handle_metadata).ok()? != 1
        || !has_only_default_data_stream(path).ok()?
    {
        return None;
    }
    file_identity_from_file(&file, &handle_metadata).ok()
}

fn snapshot_directory(
    root: &Path,
    rule: &Rule,
    scan_started_at: SystemTime,
) -> Result<DirectorySnapshot, String> {
    let root_metadata =
        fs::symlink_metadata(root).map_err(|error| format!("无法读取清理目录: {error}"))?;
    if is_link_or_reparse(&root_metadata)
        || is_offline_or_recall(&root_metadata)
        || !root_metadata.is_dir()
    {
        return Err("清理目录不是可信的普通目录".into());
    }

    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("无法验证清理目录: {error}"))?;

    // Phase 1, sequential: walk the tree and keep the entries that pass the cheap,
    // metadata-only checks. Traversal stays single-threaded because WalkDir's
    // filter_entry chain is what prevents descending into links, reparse points and
    // cloud placeholders; parallelising the walk itself would not preserve that.
    let mut candidates = Vec::new();
    let entries = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || fs::symlink_metadata(entry.path())
                    .map(|metadata| {
                        !is_link_or_reparse(&metadata) && !is_offline_or_recall(&metadata)
                    })
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
        // WalkDir already performed a symlink-aware stat to classify this entry, so
        // its cached metadata is reused instead of issuing a second identical
        // syscall per file. filter_entry above has already rejected links, reparse
        // points and offline placeholders along the path.
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let path = entry.into_path();
        if is_link_or_reparse(&metadata) || is_offline_or_recall(&metadata) || !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if !matches_rule(rule, &path, depth, modified, scan_started_at) {
            continue;
        }
        candidates.push((path, metadata, modified));
    }

    // Phase 2: resolve each candidate's real path and pin its identity. These are
    // the expensive per-file steps (a handle open plus stream enumeration dominates
    // scan cost at roughly 39s per 30k files single-threaded, versus 5.6s across
    // eight threads), and each candidate is independent: the work only reads the
    // filesystem and borrows `canonical_root` immutably, so no ordering or shared
    // state is involved.
    //
    // Both branches call the same verifier, so a file rejected serially is rejected
    // identically in parallel; only the thread it runs on differs.
    let mut files: Vec<FileSnapshot> =
        if candidates.len() < MIN_CANDIDATES_FOR_PARALLEL_VERIFY {
            candidates
                .into_iter()
                .filter_map(|candidate| verify_candidate(candidate, &canonical_root))
                .collect()
        } else {
            match verification_pool() {
                Some(pool) => pool.install(|| {
                    candidates
                        .into_par_iter()
                        .filter_map(|candidate| verify_candidate(candidate, &canonical_root))
                        .collect()
                }),
                // A pool is an optimisation, never a correctness requirement: if the
                // OS refuses the threads, verify on this thread instead of failing
                // the scan.
                None => candidates
                    .into_iter()
                    .filter_map(|candidate| verify_candidate(candidate, &canonical_root))
                    .collect(),
            }
        };

    // Sorting after collection restores a deterministic order that does not depend
    // on how work was distributed across threads.
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(DirectorySnapshot {
        root: root.to_path_buf(),
        canonical_root,
        files,
    })
}

/// Returns the process-wide verification pool, building it at most once.
///
/// A scan evaluates dozens of rules, and each one previously constructed and tore
/// down its own pool. Spawning sixteen OS threads per rule is pure overhead when
/// every rule needs the same pool, so the threads are created once and reused.
/// `None` means the OS refused the threads and callers must verify serially.
fn verification_pool() -> Option<&'static rayon::ThreadPool> {
    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(MAX_SNAPSHOT_VERIFY_THREADS)
            .thread_name(|index| format!("qingpan-verify-{index}"))
            .build()
            .ok()
    })
    .as_ref()
}

/// Confirms a candidate still resolves inside `canonical_root` and pins its
/// identity, or rejects it.
///
/// This is the single definition of "a file may be snapshotted", shared by the
/// serial and parallel verification paths so the two cannot drift apart.
fn verify_candidate(
    candidate: (PathBuf, Metadata, SystemTime),
    canonical_root: &Path,
) -> Option<FileSnapshot> {
    let (path, metadata, modified) = candidate;
    let canonical_path = fs::canonicalize(&path).ok()?;
    if canonical_path == *canonical_root || !canonical_path.starts_with(canonical_root) {
        return None;
    }
    let identity = stable_file_identity(&path, &metadata, modified)?;
    Some(FileSnapshot {
        path,
        canonical_path,
        size: metadata.len(),
        modified,
        identity,
    })
}

fn scan_with_environment(
    local_root: Option<&Path>,
    roaming_root: Option<&Path>,
    home_root: Option<&Path>,
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
            home_root,
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

        if directory.files.is_empty() {
            return None;
        }
        let size_bytes = directory.files.iter().map(|file| file.size).sum();
        let file_count = directory.files.len();
        let is_user_data = matches!(&rule.risk, RiskLevel::High);
        let use_preview_quarantine = rule.id == "temp";
        let blocked_reason = if is_user_data {
            Some("用户数据隔离尚未达到生产发布门禁；当前版本仅只读展示并安全保留".into())
        } else {
            matches!(rule.process_guard, Some(ProcessGuard::Browser(_)))
                .then(|| process_guard_error(&rule, process_names))
                .flatten()
        };
        let item = CleanupItem {
            id: rule.id,
            category: rule.category.into(),
            name: rule.name,
            path: directory.root.display().to_string(),
            description: if is_user_data {
                "微信用户数据；当前版本仅分析，不执行删除或隔离".into()
            } else if use_preview_quarantine {
                "实验性隔离：验证副本后移除源文件，可导出副本但不释放净空间".into()
            } else {
                "可由应用或 Windows 自动重新生成".into()
            },
            blocked_reason,
            size_bytes,
            file_count,
            risk: rule.risk,
            delete_mode: if use_preview_quarantine || is_user_data {
                DeleteMode::Quarantine
            } else {
                DeleteMode::Permanent
            },
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
    let home_root = home_root();
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let process_names = running_process_names();
    scan_with_environment(
        local_root.as_deref(),
        roaming_root.as_deref(),
        home_root.as_deref(),
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
    if is_link_or_reparse(&root_metadata)
        || is_offline_or_recall(&root_metadata)
        || !root_metadata.is_dir()
    {
        return Err("清理目录已变为链接或非目录对象".into());
    }

    let local_root = local_root();
    let roaming_root = roaming_root();
    let home_root = home_root();
    let wechat_documents_root = wechat_documents_root();
    let xwechat_data_root = xwechat_data_root();
    let base_root = base_root_for(
        rule,
        local_root.as_deref(),
        roaming_root.as_deref(),
        home_root.as_deref(),
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
    if is_link_or_reparse(metadata) || is_offline_or_recall(metadata) || !metadata.is_file() {
        return Err("文件已变为链接、云占位或非普通文件".into());
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
        if is_link_or_reparse(&metadata) || is_offline_or_recall(&metadata) || !metadata.is_dir() {
            return Err("文件父目录已变为链接或非目录对象".into());
        }
    }
    Ok(())
}

fn validate_snapshot_file(
    root: &Path,
    canonical_root: &Path,
    snapshot: &FileSnapshot,
) -> Result<(), String> {
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
    match has_only_default_data_stream(&snapshot.path) {
        Ok(true) => {}
        Ok(false) => return Err("文件包含额外数据流，已安全保留".into()),
        Err(_) => return Err("无法复检文件数据流，已安全保留".into()),
    }

    let file =
        File::open(&snapshot.path).map_err(|error| format!("无法打开文件进行身份复检: {error}"))?;
    let handle_metadata = file
        .metadata()
        .map_err(|error| format!("无法读取打开文件的身份信息: {error}"))?;
    validate_file_metadata(&handle_metadata, snapshot)?;
    let current_identity = file_identity_from_file(&file, &handle_metadata)
        .map_err(|_| "无法确认文件身份，已安全保留".to_string())?;
    if current_identity != snapshot.identity {
        return Err("文件身份与扫描快照不一致，已安全保留".into());
    }
    let link_count = hard_link_count_from_file(&file, &handle_metadata)
        .map_err(|_| "无法确认文件硬链接状态，已安全保留".to_string())?;
    if link_count != 1 {
        return Err("文件硬链接状态不允许安全删除，已安全保留".into());
    }
    match has_only_default_data_stream(&snapshot.path) {
        Ok(true) => {}
        Ok(false) => return Err("文件新增了额外数据流，已安全保留".into()),
        Err(_) => return Err("无法完成文件数据流复检，已安全保留".into()),
    }
    drop(file);

    let metadata = fs::symlink_metadata(&snapshot.path)
        .map_err(|error| format!("文件在验证期间失效: {error}"))?;
    validate_parent_chain(root, &snapshot.path)?;
    validate_file_metadata(&metadata, snapshot)?;
    let canonical_after = fs::canonicalize(&snapshot.path)
        .map_err(|error| format!("无法完成文件路径复检: {error}"))?;
    if canonical_after != canonical_path {
        return Err("文件路径在身份复检期间发生变化，已安全保留".into());
    }
    match has_only_default_data_stream(&snapshot.path) {
        Ok(true) => Ok(()),
        Ok(false) => Err("文件在复检期间新增了额外数据流，已安全保留".into()),
        Err(_) => Err("无法完成删除前数据流复检，已安全保留".into()),
    }
}

pub(crate) fn revalidate_execution_file(
    snapshot: &CleanupSnapshot,
    path: &Path,
) -> Result<(), String> {
    let file = snapshot
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| "文件不属于清理计划快照".to_string())?;
    let rule = rule_for_snapshot(snapshot)?;
    let canonical_root = validated_snapshot_root(snapshot, &rule)?;
    if rule.process_guard.is_some() {
        let process_names = running_process_names();
        if let Some(error) = process_guard_error(&rule, process_names.as_deref()) {
            return Err(error);
        }
    }
    validate_snapshot_file(&snapshot.root, &canonical_root, file)
}

fn delete_snapshot_file(
    root: &Path,
    canonical_root: &Path,
    snapshot: &FileSnapshot,
) -> Result<u64, String> {
    validate_snapshot_file(root, canonical_root, snapshot)?;
    fs::remove_file(&snapshot.path).map_err(|error| format!("删除失败: {error}"))?;
    Ok(snapshot.size)
}

fn delete_snapshot_files_with_progress<F>(
    snapshot: &CleanupSnapshot,
    canonical_root: &Path,
    on_progress: &mut F,
) -> DeleteOutcome
where
    F: FnMut(usize, usize, &Path, u64, usize),
{
    let mut outcome = DeleteOutcome::default();
    let total_files = snapshot.files.len();
    let report_every = total_files
        .saturating_add(99)
        .checked_div(100)
        .unwrap_or(1)
        .max(1);
    for (index, file) in snapshot.files.iter().enumerate() {
        match delete_snapshot_file(&snapshot.root, canonical_root, file) {
            Ok(bytes) => outcome.reclaimed_bytes = outcome.reclaimed_bytes.saturating_add(bytes),
            Err(error) => outcome.failures.push(DeleteFailure {
                path: file.path.clone(),
                error,
            }),
        }
        let completed_files = index + 1;
        if completed_files == total_files || completed_files % report_every == 0 {
            on_progress(
                completed_files,
                total_files,
                &file.path,
                outcome.reclaimed_bytes,
                outcome.failures.len(),
            );
        }
    }
    outcome
}

#[cfg(test)]
fn delete_snapshot_files(snapshot: &CleanupSnapshot, canonical_root: &Path) -> DeleteOutcome {
    delete_snapshot_files_with_progress(snapshot, canonical_root, &mut |_, _, _, _, _| {})
}

#[cfg(test)]
pub fn execute(snapshot: &CleanupSnapshot) -> DeleteOutcome {
    execute_with_progress(snapshot, |_, _, _, _, _| {})
}

pub fn execute_with_progress<F>(snapshot: &CleanupSnapshot, mut on_progress: F) -> DeleteOutcome
where
    F: FnMut(usize, usize, &Path, u64, usize),
{
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

    delete_snapshot_files_with_progress(snapshot, &canonical_root, &mut on_progress)
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
                file_count: directory.files.len(),
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
        let snapshots = scan_with_environment(None, None, None, Some(&documents), None, Some(&closed));
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
        assert!(snapshots.iter().all(|snapshot| {
            matches!(&snapshot.item.delete_mode, DeleteMode::Quarantine)
                && snapshot.item.blocked_reason.is_some()
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
            scan_with_environment(None, None, None, Some(&documents), None, Some(&running)).is_empty()
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
        let snapshots = scan_with_environment(None, None, None, None, Some(&data_root), Some(&closed));
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
        assert!(snapshots.iter().all(|snapshot| {
            matches!(&snapshot.item.delete_mode, DeleteMode::Quarantine)
                && snapshot.item.blocked_reason.is_some()
        }));

        let running = vec!["Weixin.exe".to_string()];
        assert!(
            scan_with_environment(None, None, None, None, Some(&data_root), Some(&running)).is_empty()
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
            scan_with_environment(Some(&local), Some(&roaming), None, None, None, Some(&running));
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
            scan_with_environment(Some(&local), Some(&roaming), None, None, None, Some(&closed));
        assert_eq!(
            closed_scan
                .iter()
                .filter(|snapshot| snapshot.item.category.starts_with("微信"))
                .count(),
            1
        );

        let running = vec!["WeChat.exe".to_string()];
        let running_scan =
            scan_with_environment(Some(&local), Some(&roaming), None, None, None, Some(&running));
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
    fn scan_omits_empty_rules_and_reports_the_snapshot_file_count() {
        let roaming = TestDirectory::new();
        create_directory(roaming.path(), "Code/Cache");
        let process_names: Vec<String> = Vec::new();

        let empty =
            scan_with_environment(None, Some(roaming.path()), None, None, None, Some(&process_names));
        assert!(!empty
            .iter()
            .any(|snapshot| snapshot.item().id == "vscode-cache"));

        fs::write(roaming.path().join("Code/Cache/cache-entry"), b"cache")
            .expect("cache entry should be written");
        let populated =
            scan_with_environment(None, Some(roaming.path()), None, None, None, Some(&process_names));
        let snapshot = populated
            .iter()
            .find(|snapshot| snapshot.item().id == "vscode-cache")
            .expect("populated cache should be returned");

        assert_eq!(snapshot.item().file_count, 1);
        assert_eq!(snapshot.item().size_bytes, 5);
    }

    /// Developer caches are refilled from a registry, so a rule may only ever
    /// point at a cache subdirectory. Targeting a tool root would take
    /// `~/.cargo/bin` (installed executables), `~/.gradle/wrapper` (downloaded
    /// distributions) or npm's `_npx`/`_logs` with it, none of which are package
    /// cache data.
    #[test]
    fn developer_cache_rules_never_target_a_tool_root() {
        let mut rules = Vec::new();
        add_developer_cache_rules(&mut rules);
        assert!(!rules.is_empty());

        for rule in &rules {
            assert_eq!(rule.category, "开发者缓存");
            assert!(matches!(rule.risk, RiskLevel::Low));
            // Every target must be nested, never a bare tool root.
            let depth = rule.relative.components().count();
            assert!(
                depth >= 2,
                "{} targets a tool root: {}",
                rule.id,
                rule.relative.display()
            );
            for forbidden in [
                ".cargo/bin",
                ".cargo/registry/index",
                ".gradle/wrapper",
                ".m2/wrapper",
                "npm-cache/_npx",
                "npm-cache/_logs",
                "npm-cache/_cacache/tmp",
            ] {
                assert!(
                    !rule.relative.starts_with(forbidden),
                    "{} must not target {forbidden}",
                    rule.id
                );
            }
        }
    }

    /// pnpm's store hard-links its content into every project's `node_modules`.
    /// Deleting files underneath it damages live projects rather than freeing
    /// space, so it must be reached through `pnpm store prune` instead. This pins
    /// the exclusion so a future rule cannot reintroduce it by accident.
    #[test]
    fn developer_cache_rules_exclude_the_pnpm_store() {
        let mut rules = Vec::new();
        add_developer_cache_rules(&mut rules);

        assert!(rules.iter().all(|rule| {
            let relative = rule.relative.to_string_lossy().to_ascii_lowercase();
            !relative.contains("pnpm")
        }));
    }

    /// WeGame and TenioDL version their cache parents, so the rule targets the
    /// product root and relies on the matcher to pick only `cache` descendants.
    /// This proves configuration and installed payloads sitting next to the cache
    /// are never collected.
    #[test]
    fn versioned_tencent_cache_rules_collect_only_cache_descendants() {
        let roaming = TestDirectory::new();
        let process_names: Vec<String> = Vec::new();
        create_directory(roaming.path(), "Tencent/WeGame/qbcore109/cache");
        fs::write(
            roaming
                .path()
                .join("Tencent/WeGame/qbcore109/cache/page.dat"),
            b"cache",
        )
        .expect("cache file should be written");
        // Configuration and an installed payload next to the cache must be kept.
        create_directory(roaming.path(), "Tencent/WeGame/qbcore109/config");
        let config = roaming
            .path()
            .join("Tencent/WeGame/qbcore109/config/settings.ini");
        fs::write(&config, b"keep me").expect("config should be written");
        let payload = roaming.path().join("Tencent/WeGame/launcher.exe");
        fs::write(&payload, b"binary").expect("payload should be written");

        let snapshots = scan_with_environment(
            None,
            Some(roaming.path()),
            None,
            None,
            None,
            Some(&process_names),
        );
        let snapshot = snapshots
            .iter()
            .find(|snapshot| snapshot.item().id == "wegame-core-cache")
            .expect("wegame cache rule should be discovered");

        assert_eq!(snapshot.item().file_count, 1);
        assert!(snapshot
            .files
            .iter()
            .all(|file| file.path.ends_with("page.dat")));
        assert!(config.exists());
        assert!(payload.exists());
    }

    /// QQ chat history and received files live under `Documents\Tencent Files`.
    /// No Tencent rule may reach into that tree, because a delete path to user
    /// chat data depends on the production-grade quarantine that is still gated.
    #[test]
    fn tencent_rules_never_target_chat_data() {
        let mut rules = Vec::new();
        add_qq_rules(&mut rules);

        assert!(!rules.is_empty());
        for rule in &rules {
            assert!(
                matches!(rule.base, RootBase::Roaming),
                "{} must stay under application data",
                rule.id
            );
            let relative = rule.relative.to_string_lossy().to_ascii_lowercase();
            for forbidden in ["tencent files", "nt_qq", "nt_db", "nt_data"] {
                assert!(
                    !relative.contains(forbidden),
                    "{} must not target {forbidden}",
                    rule.id
                );
            }
        }
    }

    /// Windows diagnostic rules must stay inside user-writable locations. Paths
    /// that need elevation or a privileged interface belong to the on-demand
    /// elevated executor under ADR-005, not to this pass.
    #[test]
    fn system_junk_rules_stay_within_user_writable_locations() {
        let mut rules = Vec::new();
        add_system_junk_rules(&mut rules);

        assert!(!rules.is_empty());
        for rule in &rules {
            assert!(
                matches!(rule.base, RootBase::Local),
                "{} must resolve under LocalAppData",
                rule.id
            );
            assert!(matches!(rule.risk, RiskLevel::Low));
            let relative = rule.relative.to_string_lossy().to_ascii_lowercase();
            // Elevation-bound and repair-critical Windows locations are excluded.
            for forbidden in [
                "softwaredistribution",
                "installer",
                "winsxs",
                "system32",
                "servicing",
            ] {
                assert!(
                    !relative.contains(forbidden),
                    "{} must not target {forbidden}",
                    rule.id
                );
            }
        }
    }

    /// A cache rule must only collect files from inside its own directory, and the
    /// `Home` base has to resolve for tools that live under the user profile
    /// rather than `AppData`.
    #[test]
    fn home_based_developer_cache_scan_collects_only_its_own_files() {
        let home = TestDirectory::new();
        let process_names: Vec<String> = Vec::new();
        // Regenerable cache content that the rule should collect.
        create_directory(home.path(), ".cargo/registry/cache");
        fs::write(
            home.path().join(".cargo/registry/cache/crate-archive.crate"),
            b"archive",
        )
        .expect("cache file should be written");
        // Installed executables and the index must be left untouched.
        create_directory(home.path(), ".cargo/bin");
        let executable = home.path().join(".cargo/bin/cargo.exe");
        fs::write(&executable, b"binary").expect("bin file should be written");
        create_directory(home.path(), ".cargo/registry/index");
        let index = home.path().join(".cargo/registry/index/config.json");
        fs::write(&index, b"index").expect("index file should be written");

        let snapshots = scan_with_environment(
            None,
            None,
            Some(home.path()),
            None,
            None,
            Some(&process_names),
        );
        let snapshot = snapshots
            .iter()
            .find(|snapshot| snapshot.item().id == "cargo-registry-cache")
            .expect("cargo cache rule should be discovered");

        assert_eq!(snapshot.item().file_count, 1);
        assert_eq!(snapshot.item().size_bytes, 7);
        assert!(snapshot
            .files
            .iter()
            .all(|file| file.path.starts_with(home.path().join(".cargo/registry/cache"))));
        // Nothing outside the cache directory may be scheduled for removal.
        assert!(snapshots.iter().flat_map(|item| &item.files).all(|file| {
            file.path != executable && file.path != index
        }));
    }

    /// An alternate data stream carries content that the logical size never
    /// reports, so such a file must never be snapshotted or deleted. The check runs
    /// once, after the handle is opened; this pins that it still rejects ADS files
    /// so the deduplicated syscall cannot silently weaken the guard.
    #[cfg(windows)]
    #[test]
    fn files_with_alternate_data_streams_are_never_snapshotted() {
        let directory = TestDirectory::new();
        let plain = directory.path().join("plain.tmp");
        let tagged = directory.path().join("tagged.tmp");
        fs::write(&plain, b"plain").expect("plain file should be written");
        fs::write(&tagged, b"tagged").expect("tagged file should be written");
        // Attach a second data stream; if the volume is not NTFS this cannot be
        // exercised and the test would be meaningless.
        if fs::write(
            directory.path().join("tagged.tmp:extra"),
            b"hidden-payload",
        )
        .is_err()
        {
            return;
        }
        assert!(
            !has_only_default_data_stream(&tagged).expect("stream state should be readable"),
            "test setup failed to attach an alternate data stream"
        );

        let rule = rule_by_id("temp");
        let snapshot = snapshot_directory(directory.path(), &rule, SystemTime::now() + TEMP_MINIMUM_AGE)
            .expect("directory should be snapshotted");

        let paths: Vec<_> = snapshot.files.iter().map(|file| file.path.clone()).collect();
        assert!(paths.contains(&plain), "ordinary file should be collected");
        assert!(
            !paths.contains(&tagged),
            "file with an alternate data stream must be excluded"
        );
    }

    /// The parallel verification path must be indistinguishable from the serial
    /// one. This crosses `MIN_CANDIDATES_FOR_PARALLEL_VERIFY` so the pool is
    /// actually used, then checks the full snapshot rather than just its length:
    /// order must be deterministic and every recorded field must match a
    /// independently computed expectation.
    #[test]
    fn parallel_verification_matches_serial_results_above_the_threshold() {
        let directory = TestDirectory::new();
        let count = MIN_CANDIDATES_FOR_PARALLEL_VERIFY * 3;
        for index in 0..count {
            // Distinct sizes make a mixed-up pairing of path and size detectable.
            let payload = vec![b'x'; index + 1];
            fs::write(directory.path().join(format!("file-{index:04}.tmp")), &payload)
                .expect("candidate should be written");
        }
        let rule = test_rule();

        let snapshot = snapshot_directory(directory.path(), &rule, SystemTime::now())
            .expect("directory should be snapshotted");

        assert!(
            snapshot.files.len() >= MIN_CANDIDATES_FOR_PARALLEL_VERIFY,
            "fixture must cross the parallel threshold to exercise the pool"
        );
        assert_eq!(snapshot.files.len(), count);
        // Deterministic ordering regardless of how work was distributed.
        let mut sorted = snapshot.files.clone();
        sorted.sort_by(|left, right| left.path.cmp(&right.path));
        assert_eq!(
            snapshot
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect::<Vec<_>>(),
            sorted.iter().map(|file| file.path.clone()).collect::<Vec<_>>()
        );
        // Each entry must still carry the size belonging to its own path.
        for file in &snapshot.files {
            let expected = fs::symlink_metadata(&file.path).expect("candidate metadata");
            assert_eq!(file.size, expected.len(), "size mismatch for {:?}", file.path);
            assert!(file.canonical_path.starts_with(&snapshot.canonical_root));
        }
        let unique: std::collections::BTreeSet<_> =
            snapshot.files.iter().map(|file| file.path.clone()).collect();
        assert_eq!(unique.len(), count, "no candidate may be duplicated or dropped");
    }

    /// Safety rejections must survive parallelisation. Above the threshold, a file
    /// carrying an alternate data stream still has to be excluded while its
    /// ordinary neighbours are kept.
    #[cfg(windows)]
    #[test]
    fn parallel_verification_still_rejects_alternate_data_streams() {
        let directory = TestDirectory::new();
        let count = MIN_CANDIDATES_FOR_PARALLEL_VERIFY * 2;
        for index in 0..count {
            fs::write(
                directory.path().join(format!("plain-{index:04}.tmp")),
                b"plain",
            )
            .expect("plain file should be written");
        }
        let tagged = directory.path().join("tagged.tmp");
        fs::write(&tagged, b"tagged").expect("tagged file should be written");
        if fs::write(directory.path().join("tagged.tmp:extra"), b"hidden").is_err() {
            return;
        }
        assert!(!has_only_default_data_stream(&tagged).expect("stream state"));

        let snapshot = snapshot_directory(directory.path(), &test_rule(), SystemTime::now())
            .expect("directory should be snapshotted");

        assert!(snapshot.files.len() >= MIN_CANDIDATES_FOR_PARALLEL_VERIFY);
        assert_eq!(snapshot.files.len(), count);
        assert!(
            !snapshot.files.iter().any(|file| file.path == tagged),
            "ADS file must stay excluded on the parallel path"
        );
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

    #[test]
    fn execution_revalidation_rejects_a_path_outside_the_snapshot() {
        let directory = TestDirectory::new();
        let scanned_file = directory.path().join("scanned.tmp");
        fs::write(&scanned_file, b"snapshot").expect("scanned file should be written");
        let snapshot = cleanup_snapshot(directory.path());
        let outside = directory.path().join("not-in-plan.tmp");
        fs::write(&outside, b"keep").expect("outside file should be written");

        let error = revalidate_execution_file(&snapshot, &outside)
            .expect_err("a path outside the immutable snapshot must fail");

        assert!(error.contains("不属于清理计划快照"));
        assert!(outside.exists());
    }

    #[test]
    fn same_size_replacement_is_rejected_by_file_identity() {
        let directory = TestDirectory::new();
        let file = directory.path().join("identity.tmp");
        fs::write(&file, b"before").expect("initial file should be written");
        let mut snapshot = cleanup_snapshot(directory.path());

        fs::remove_file(&file).expect("original file should be removed for replacement fixture");
        fs::write(&file, b"after!").expect("same-size replacement should be written");
        snapshot.files[0].modified = fs::symlink_metadata(&file)
            .and_then(|metadata| metadata.modified())
            .expect("replacement modified time should be readable");

        let outcome = delete_snapshot_files(&snapshot, &snapshot.canonical_root);

        assert_eq!(outcome.reclaimed_bytes, 0);
        assert_eq!(outcome.failures.len(), 1);
        assert!(outcome.failures[0].error.contains("身份"));
        assert!(file.exists());
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn files_with_multiple_hard_links_are_not_snapshotted() {
        let directory = TestDirectory::new();
        let first = directory.path().join("first.tmp");
        let second = directory.path().join("second.tmp");
        fs::write(&first, b"linked content").expect("source file should be written");
        fs::hard_link(&first, &second).expect("hard-link fixture should be created");

        let snapshot = snapshot_directory(directory.path(), &test_rule(), SystemTime::now())
            .expect("directory scan should complete");

        assert!(snapshot.files.is_empty());
        assert!(first.exists());
        assert!(second.exists());
    }

    #[test]
    fn cleanup_progress_reaches_total_with_monotonic_reclaimed_bytes() {
        let directory = TestDirectory::new();
        fs::write(directory.path().join("first.tmp"), b"one")
            .expect("first test file should be written");
        fs::write(directory.path().join("second.tmp"), b"second")
            .expect("second test file should be written");
        fs::write(directory.path().join("third.tmp"), b"third file")
            .expect("third test file should be written");
        let snapshot = cleanup_snapshot(directory.path());
        let total_files = snapshot.files.len();
        let mut reports = Vec::new();

        let outcome = delete_snapshot_files_with_progress(
            &snapshot,
            &snapshot.canonical_root,
            &mut |completed, total, path, reclaimed, failed| {
                reports.push((completed, total, path.to_path_buf(), reclaimed, failed));
            },
        );

        assert!(outcome.failures.is_empty());
        assert_eq!(outcome.reclaimed_bytes, 19);
        assert_eq!(reports.last().map(|report| report.0), Some(total_files));
        assert!(reports.iter().all(|report| report.1 == total_files));
        assert!(reports.windows(2).all(|pair| pair[0].3 <= pair[1].3));
        assert!(reports.iter().all(|report| report.4 == 0));
    }
}
