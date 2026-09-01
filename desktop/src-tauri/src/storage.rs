use crate::fs_safety::{
    allocated_size, file_identity_from_file, hard_link_count_from_file,
    has_only_default_data_stream, is_link_or_reparse, is_offline_or_recall,
    may_differ_from_logical_size, FileIdentity,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs::{self, File, Metadata},
    io::{self, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    time::{SystemTime, UNIX_EPOCH},
};
use walkdir::WalkDir;

const HARD_MAX_FILES: usize = 2_000_000;
const HARD_MAX_RESULTS: usize = 50_000;
const HARD_MAX_ENTRIES: usize = 8_000_000;
const HARD_MAX_SAMPLE_BYTES: usize = 1024 * 1024;
const HARD_MAX_EXCLUDED_PATHS: usize = 64;
const HARD_MAX_EXCLUDED_PATH_LENGTH: usize = 32_767;
const HASH_BUFFER_BYTES: usize = 128 * 1024;

const DEFAULT_DIRECTORY_MAX_FILES: usize = 500_000;
const DEFAULT_DIRECTORY_MAX_RESULTS: usize = 10_000;
const DEFAULT_LARGE_FILE_MINIMUM_BYTES: u64 = 100 * 1024 * 1024;
const DEFAULT_LARGE_FILE_MAX_FILES: usize = 500_000;
const DEFAULT_LARGE_FILE_MAX_RESULTS: usize = 2_000;
const DEFAULT_DUPLICATE_MINIMUM_BYTES: u64 = 1024 * 1024;
const DEFAULT_DUPLICATE_MAX_FILES: usize = 250_000;
const DEFAULT_DUPLICATE_MAX_GROUPS: usize = 2_000;
const DEFAULT_DUPLICATE_MAX_MEMBERS: usize = 10_000;
const DEFAULT_DUPLICATE_SAMPLE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub visited_entries: u64,
    pub scanned_files: u64,
    pub skipped: u64,
    /// Additional hard links to already-counted content, excluded from totals so
    /// shared clusters are charged exactly once.
    pub deduplicated_hard_links: u64,
    pub cancelled: bool,
    pub limit_reached: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DirectoryScanOptions {
    pub max_files: usize,
    pub max_results: usize,
    pub excluded_paths: Vec<String>,
}

impl Default for DirectoryScanOptions {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_DIRECTORY_MAX_FILES,
            max_results: DEFAULT_DIRECTORY_MAX_RESULTS,
            excluded_paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LargeFileScanOptions {
    pub min_size_bytes: u64,
    pub max_files: usize,
    pub max_results: usize,
    pub excluded_paths: Vec<String>,
}

impl Default for LargeFileScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: DEFAULT_LARGE_FILE_MINIMUM_BYTES,
            max_files: DEFAULT_LARGE_FILE_MAX_FILES,
            max_results: DEFAULT_LARGE_FILE_MAX_RESULTS,
            excluded_paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DuplicateScanOptions {
    pub min_size_bytes: u64,
    pub max_files: usize,
    pub max_groups: usize,
    pub max_members: usize,
    pub sample_bytes: usize,
    pub excluded_paths: Vec<String>,
}

impl Default for DuplicateScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: DEFAULT_DUPLICATE_MINIMUM_BYTES,
            max_files: DEFAULT_DUPLICATE_MAX_FILES,
            max_groups: DEFAULT_DUPLICATE_MAX_GROUPS,
            max_members: DEFAULT_DUPLICATE_MAX_MEMBERS,
            sample_bytes: DEFAULT_DUPLICATE_SAMPLE_BYTES,
            excluded_paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryUsage {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub allocated_bytes: u64,
    pub percent: f64,
    pub color: String,
    pub kind: String,
    pub file_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategory {
    pub id: String,
    pub label: String,
    pub size_bytes: u64,
    pub allocated_bytes: u64,
    pub color: String,
    pub description: String,
    pub file_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageAnalysisResult {
    pub root: String,
    pub total_size_bytes: u64,
    pub total_allocated_bytes: u64,
    pub total_file_count: u64,
    pub directories: Vec<DirectoryUsage>,
    pub categories: Vec<StorageCategory>,
    #[serde(flatten)]
    pub stats: ScanStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Normal,
    Attention,
    Protected,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub allocated_bytes: u64,
    pub modified_at: String,
    #[serde(rename = "type")]
    pub file_type: String,
    pub sensitivity: Sensitivity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileScanResult {
    pub root: String,
    pub files: Vec<LargeFileEntry>,
    pub total_matched_bytes: u64,
    #[serde(flatten)]
    pub stats: ScanStats,
}

#[derive(Clone, Debug)]
pub struct LargeFileSnapshot {
    entry: LargeFileEntry,
    canonical_root: PathBuf,
    candidate: FileCandidate,
}

impl LargeFileSnapshot {
    pub fn entry(&self) -> &LargeFileEntry {
        &self.entry
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateMember {
    pub id: String,
    pub name: String,
    pub path: String,
    pub modified_at: String,
    pub suggested_keep: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub protected: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum DuplicateMatch {
    #[serde(rename = "full_hash")]
    FullHash,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    pub hash: String,
    pub size_bytes: u64,
    pub reclaimable_bytes: u64,
    #[serde(rename = "match")]
    pub match_kind: DuplicateMatch,
    pub total_members: usize,
    pub members: Vec<DuplicateMember>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanResult {
    pub root: String,
    pub groups: Vec<DuplicateGroup>,
    pub candidate_files: u64,
    pub sampled_files: u64,
    pub hashed_files: u64,
    pub reclaimable_bytes: u64,
    #[serde(flatten)]
    pub stats: ScanStats,
}

#[derive(Default)]
struct UsageAccumulator {
    size_bytes: u64,
    allocated_bytes: u64,
    file_count: u64,
}

#[derive(Clone, Debug)]
struct FileCandidate {
    path: PathBuf,
    size_bytes: u64,
    modified: Option<SystemTime>,
    identity: Option<FileIdentity>,
}

#[derive(Clone, Copy)]
struct CategoryDefinition {
    id: &'static str,
    label: &'static str,
    color: &'static str,
    description: &'static str,
    rank: u8,
}

const CATEGORY_APPS: CategoryDefinition = CategoryDefinition {
    id: "apps",
    label: "应用与游戏",
    color: "#265dff",
    description: "已安装应用、游戏和应用数据",
    rank: 0,
};
const CATEGORY_MEDIA: CategoryDefinition = CategoryDefinition {
    id: "media",
    label: "图片与视频",
    color: "#10a37f",
    description: "图片、视频和音频等用户媒体文件",
    rank: 1,
};
const CATEGORY_SYSTEM: CategoryDefinition = CategoryDefinition {
    id: "system",
    label: "系统与保留",
    color: "#f0a02f",
    description: "Windows、恢复环境及系统保留内容",
    rank: 2,
};
const CATEGORY_DOCUMENTS: CategoryDefinition = CategoryDefinition {
    id: "docs",
    label: "文档与项目",
    color: "#e05d6f",
    description: "文档、源代码、设计文件和项目归档",
    rank: 3,
};
const CATEGORY_OTHER: CategoryDefinition = CategoryDefinition {
    id: "other",
    label: "其他",
    color: "#7b67d8",
    description: "尚未归入以上类别的内容",
    rank: 4,
};

/// Resolves the identity of a file that has more than one hard link.
///
/// Opening a handle is expensive: measured against a 60k-file pnpm store it cost
/// roughly 12x a metadata-only pass (58s vs 4.7s), because the link count is only
/// readable from an open handle. Callers must therefore gate this behind a cheap
/// filter and never call it for every file in a tree.
///
/// Multi-linked files matter because the same clusters are reachable through
/// several paths. Counting each path would inflate totals: `WinSxS` and pnpm's
/// content-addressable store are built largely from hard links, and naive
/// accumulation reports more than the real occupancy.
fn multi_link_identity(path: &Path) -> io::Result<Option<FileIdentity>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if hard_link_count_from_file(&file, &metadata)? <= 1 {
        return Ok(None);
    }
    file_identity_from_file(&file, &metadata).map(Some)
}

/// Minimum logical size for a file to be checked for hard links.
///
/// Hard-link detection needs an open handle, which is far more expensive than
/// reading metadata. Grouping by size alone does not bound that cost: in a real
/// 413k-file pnpm store 97.4% of files shared a size with some other file, so
/// almost every file would be opened.
///
/// A size floor bounds the work by the only thing dedup can affect, which is
/// bytes. In that same store, files at or above this threshold were 2.06% of all
/// files yet accounted for 87.2% of all bytes. Smaller duplicates are left
/// counted more than once, which slightly overstates usage rather than promising
/// space that deleting cannot deliver.
const HARD_LINK_CHECK_MINIMUM_BYTES: u64 = 64 * 1024;

/// A file that might be one of several hard links to the same content.
///
/// Everything needed to reverse its contribution is captured during the cheap
/// metadata pass, so the correction phase never has to walk the tree again.
struct LinkCandidate {
    path: PathBuf,
    logical_bytes: u64,
    allocated_bytes: u64,
    category_id: &'static str,
    owning_child: Option<PathBuf>,
}

/// Returns the root's immediate child directory that owns `path`, if any.
///
/// Direct files of the root (`depth < 2`) belong to no child directory and are
/// reported only in the totals and category rollups.
fn owning_direct_child(canonical_root: &Path, path: &Path, depth: usize) -> Option<PathBuf> {
    if depth < 2 {
        return None;
    }
    let relative = path.strip_prefix(canonical_root).ok()?;
    let Component::Normal(first) = relative.components().next()? else {
        return None;
    };
    Some(canonical_root.join(first))
}

/// Recursively analyzes `root` without following links or reparse points.
///
/// `directories` contains only the root's immediate child directories. Direct
/// files still contribute to totals and categories. The function never writes
/// to the scanned tree.
///
/// Sizes are reported twice: `size_bytes` is the logical length users recognize,
/// while `allocated_bytes` is the physical occupancy that a deletion would
/// actually release. Hard-linked content is counted once so shared stores are
/// not multiplied.
pub fn scan_directory_usage(
    root: &Path,
    options: DirectoryScanOptions,
    cancel: &AtomicBool,
) -> Result<StorageAnalysisResult, String> {
    validate_limits(options.max_files, options.max_results)?;
    let canonical_root = validate_root(root)?;
    let excluded_paths = normalize_excluded_paths(&canonical_root, &options.excluded_paths)?;
    let visible_root = visible_root(&canonical_root);
    let display_root = visible_root.display().to_string();
    let mut stats = ScanStats::default();
    if is_cancelled(cancel, &mut stats) {
        return Ok(StorageAnalysisResult {
            root: display_root,
            total_size_bytes: 0,
            total_allocated_bytes: 0,
            total_file_count: 0,
            directories: Vec::new(),
            categories: Vec::new(),
            stats,
        });
    }

    let directories: RefCell<HashMap<PathBuf, UsageAccumulator>> = RefCell::new(HashMap::new());
    let mut categories: HashMap<&'static str, (CategoryDefinition, UsageAccumulator)> =
        HashMap::new();
    let mut totals = UsageAccumulator::default();
    let directory_limit_hit = Cell::new(false);
    // Paths grouped by logical size. Hard links to the same content always share a
    // size, so a size seen only once cannot be a duplicate and never needs a handle.
    let link_candidates: RefCell<HashMap<u64, Vec<LinkCandidate>>> = RefCell::new(HashMap::new());

    walk_regular_files(
        &canonical_root,
        options.max_files,
        options.max_results,
        &excluded_paths,
        cancel,
        &mut stats,
        |path, depth| {
            if depth != 1 {
                return;
            }
            let mut directories = directories.borrow_mut();
            if directories.contains_key(path) {
                return;
            }
            if directories.len() >= options.max_results {
                directory_limit_hit.set(true);
                return;
            }
            directories.insert(path.to_path_buf(), UsageAccumulator::default());
        },
        |path, metadata, depth| {
            let logical_bytes = metadata.len();
            // Physical allocation is what actually frees up on disk: NTFS-compressed
            // and sparse files occupy less than their logical length. Only those files
            // are queried, because for ordinary files the logical length already is
            // the allocation and querying all of them cost 32.5s versus 3ms on a
            // 413k-file store for identical totals. The query is path-level and never
            // opens the file, so it cannot hydrate cloud placeholders. If the volume
            // cannot answer, fall back to logical size so a failure understates
            // savings rather than dropping the file.
            let allocated_bytes = if may_differ_from_logical_size(metadata) {
                allocated_size(path, metadata).unwrap_or(logical_bytes)
            } else {
                logical_bytes
            };
            // Hard-link detection needs an open handle, so it is limited to files
            // large enough to matter. See HARD_LINK_CHECK_MINIMUM_BYTES: size grouping
            // alone does not bound this, because nearly all small files share a size.
            if logical_bytes >= HARD_LINK_CHECK_MINIMUM_BYTES {
                link_candidates
                    .borrow_mut()
                    .entry(logical_bytes)
                    .or_default()
                    .push(LinkCandidate {
                        path: path.to_path_buf(),
                        logical_bytes,
                        allocated_bytes,
                        category_id: classify_storage_category(path).id,
                        owning_child: owning_direct_child(&canonical_root, path, depth),
                    });
            }
            totals.size_bytes = totals.size_bytes.saturating_add(logical_bytes);
            totals.allocated_bytes = totals.allocated_bytes.saturating_add(allocated_bytes);
            totals.file_count = totals.file_count.saturating_add(1);

            let definition = classify_storage_category(path);
            let category = categories
                .entry(definition.id)
                .or_insert_with(|| (definition, UsageAccumulator::default()));
            category.1.size_bytes = category.1.size_bytes.saturating_add(logical_bytes);
            category.1.allocated_bytes = category.1.allocated_bytes.saturating_add(allocated_bytes);
            category.1.file_count = category.1.file_count.saturating_add(1);

            let Some(direct_child) = owning_direct_child(&canonical_root, path, depth) else {
                return;
            };
            if let Some(usage) = directories.borrow_mut().get_mut(&direct_child) {
                usage.size_bytes = usage.size_bytes.saturating_add(logical_bytes);
                usage.allocated_bytes = usage.allocated_bytes.saturating_add(allocated_bytes);
                usage.file_count = usage.file_count.saturating_add(1);
            }
        },
    );

    if directory_limit_hit.get() {
        stats.limit_reached = true;
    }

    // Correction phase: charge hard-linked content once. Only sizes seen more than
    // once can hide a hard link, so the expensive handle-based identity check runs
    // on a small fraction of the tree instead of every file.
    let mut counted_identities: HashSet<FileIdentity> = HashSet::new();
    for (_, group) in link_candidates.into_inner() {
        if group.len() < 2 {
            continue;
        }
        for candidate in group {
            let Ok(Some(identity)) = multi_link_identity(&candidate.path) else {
                continue;
            };
            if counted_identities.insert(identity) {
                // First path to this content keeps the bytes.
                continue;
            }
            // A repeat link frees nothing when deleted, so its contribution is
            // removed from every rollup it was added to.
            stats.deduplicated_hard_links = stats.deduplicated_hard_links.saturating_add(1);
            totals.size_bytes = totals.size_bytes.saturating_sub(candidate.logical_bytes);
            totals.allocated_bytes = totals
                .allocated_bytes
                .saturating_sub(candidate.allocated_bytes);
            totals.file_count = totals.file_count.saturating_sub(1);
            if let Some((_, usage)) = categories.get_mut(candidate.category_id) {
                usage.size_bytes = usage.size_bytes.saturating_sub(candidate.logical_bytes);
                usage.allocated_bytes = usage
                    .allocated_bytes
                    .saturating_sub(candidate.allocated_bytes);
                usage.file_count = usage.file_count.saturating_sub(1);
            }
            if let Some(child) = candidate.owning_child {
                if let Some(usage) = directories.borrow_mut().get_mut(&child) {
                    usage.size_bytes = usage.size_bytes.saturating_sub(candidate.logical_bytes);
                    usage.allocated_bytes = usage
                        .allocated_bytes
                        .saturating_sub(candidate.allocated_bytes);
                    usage.file_count = usage.file_count.saturating_sub(1);
                }
            }
        }
    }

    let mut directory_results = directories
        .into_inner()
        .into_iter()
        .map(|(path, usage)| {
            let (kind, color) = classify_directory(&path);
            DirectoryUsage {
                id: stable_id("dir", &path),
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string()),
                path: visible_path(&canonical_root, &visible_root, &path)
                    .display()
                    .to_string(),
                size_bytes: usage.size_bytes,
                allocated_bytes: usage.allocated_bytes,
                percent: if totals.size_bytes == 0 {
                    0.0
                } else {
                    usage.size_bytes as f64 / totals.size_bytes as f64 * 100.0
                },
                color: color.to_string(),
                kind: kind.to_string(),
                file_count: usage.file_count,
            }
        })
        .collect::<Vec<_>>();
    directory_results.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| path_text_cmp(&left.path, &right.path))
    });

    let mut category_results = categories
        .into_values()
        .map(|(definition, usage)| {
            (
                definition.rank,
                StorageCategory {
                    id: definition.id.to_string(),
                    label: definition.label.to_string(),
                    size_bytes: usage.size_bytes,
                    allocated_bytes: usage.allocated_bytes,
                    color: definition.color.to_string(),
                    description: definition.description.to_string(),
                    file_count: usage.file_count,
                },
            )
        })
        .collect::<Vec<_>>();
    category_results.sort_by_key(|(rank, _)| *rank);

    Ok(StorageAnalysisResult {
        root: display_root,
        total_size_bytes: totals.size_bytes,
        total_allocated_bytes: totals.allocated_bytes,
        total_file_count: totals.file_count,
        directories: directory_results,
        categories: category_results
            .into_iter()
            .map(|(_, category)| category)
            .collect(),
        stats,
    })
}

/// Finds the largest regular, locally available files below `root`.
///
/// The result is sorted by descending logical size and is bounded by
/// `max_results`. No file is opened for content reads.
pub fn scan_large_files_with_snapshots(
    root: &Path,
    options: LargeFileScanOptions,
    cancel: &AtomicBool,
) -> Result<(LargeFileScanResult, HashMap<String, LargeFileSnapshot>), String> {
    validate_limits(options.max_files, options.max_results)?;
    let canonical_root = validate_root(root)?;
    let excluded_paths = normalize_excluded_paths(&canonical_root, &options.excluded_paths)?;
    let visible_root = visible_root(&canonical_root);
    let display_root = visible_root.display().to_string();
    let mut stats = ScanStats::default();
    if is_cancelled(cancel, &mut stats) {
        return Ok((
            LargeFileScanResult {
                root: display_root,
                files: Vec::new(),
                total_matched_bytes: 0,
                stats,
            },
            HashMap::new(),
        ));
    }

    let mut files = Vec::with_capacity(options.max_results.min(4096));
    let snapshots = RefCell::new(HashMap::new());
    let matched_count = Cell::new(0usize);
    let matched_bytes = Cell::new(0u64);
    let prune_at = options
        .max_results
        .saturating_mul(2)
        .max(options.max_results.saturating_add(1));

    walk_regular_files(
        &canonical_root,
        options.max_files,
        options.max_results,
        &excluded_paths,
        cancel,
        &mut stats,
        |_, _| {},
        |path, metadata, _| {
            if metadata.len() < options.min_size_bytes {
                return;
            }
            matched_count.set(matched_count.get().saturating_add(1));
            matched_bytes.set(matched_bytes.get().saturating_add(metadata.len()));
            let mut candidate = FileCandidate {
                path: path.to_path_buf(),
                size_bytes: metadata.len(),
                modified: metadata.modified().ok(),
                identity: None,
            };
            let (file_type, mut sensitivity, mut note) = classify_large_file(path);
            if sensitivity != Sensitivity::Protected {
                match (
                    file_identity_and_link_count(&candidate),
                    has_only_default_data_stream(path),
                ) {
                    (Ok((identity, 1)), Ok(true)) => candidate.identity = Some(identity),
                    (Ok((_identity, links)), Ok(true)) if links > 1 => {
                        sensitivity = Sensitivity::Protected;
                        note = Some("文件存在多个硬链接，无法准确承诺释放空间".into());
                    }
                    (Ok(_), Ok(false)) => {
                        sensitivity = Sensitivity::Protected;
                        note = Some("文件包含额外数据流，无法作为普通大文件安全删除".into());
                    }
                    _ => {
                        sensitivity = Sensitivity::Protected;
                        note = Some("无法建立稳定文件身份，当前结果仅供查看".into());
                    }
                }
            }
            let entry = LargeFileEntry {
                id: stable_id("large", path),
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string()),
                path: visible_path(&canonical_root, &visible_root, path)
                    .display()
                    .to_string(),
                size_bytes: metadata.len(),
                allocated_bytes: metadata.len(),
                modified_at: format_system_time(metadata.modified().ok()),
                file_type,
                sensitivity,
                note,
            };
            if candidate.identity.is_some() {
                snapshots.borrow_mut().insert(
                    entry.id.clone(),
                    LargeFileSnapshot {
                        entry: entry.clone(),
                        canonical_root: canonical_root.clone(),
                        candidate,
                    },
                );
            }
            files.push(entry);
            if files.len() >= prune_at {
                sort_large_files(&mut files);
                files.truncate(options.max_results);
                let retained = files
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect::<HashSet<_>>();
                snapshots
                    .borrow_mut()
                    .retain(|id, _| retained.contains(id.as_str()));
            }
        },
    );

    sort_large_files(&mut files);
    files.truncate(options.max_results);
    let retained = files
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<HashSet<_>>();
    snapshots
        .borrow_mut()
        .retain(|id, _| retained.contains(id.as_str()));
    if matched_count.get() > options.max_results {
        stats.limit_reached = true;
    }

    Ok((
        LargeFileScanResult {
            root: display_root,
            files,
            total_matched_bytes: matched_bytes.get(),
            stats,
        },
        snapshots.into_inner(),
    ))
}

#[cfg(test)]
pub fn scan_large_files(
    root: &Path,
    options: LargeFileScanOptions,
    cancel: &AtomicBool,
) -> Result<LargeFileScanResult, String> {
    scan_large_files_with_snapshots(root, options, cancel).map(|(result, _)| result)
}

pub fn delete_large_file(snapshot: &LargeFileSnapshot) -> Result<u64, String> {
    if snapshot.entry.sensitivity == Sensitivity::Protected {
        return Err("受保护文件不允许通过大文件清理删除".into());
    }
    let current_root = validate_root(&snapshot.canonical_root)?;
    if current_root != snapshot.canonical_root {
        return Err("扫描根目录在分析后发生变化".into());
    }

    validate_large_file_parent_chain(&snapshot.canonical_root, &snapshot.candidate.path)?;
    let before = fs::symlink_metadata(&snapshot.candidate.path)
        .map_err(|error| format!("文件在执行前已失效: {error}"))?;
    validate_large_file_metadata(&before, &snapshot.candidate)?;
    let canonical = fs::canonicalize(&snapshot.candidate.path)
        .map_err(|error| format!("无法复检文件路径: {error}"))?;
    if canonical == snapshot.canonical_root
        || !path_is_same_or_descendant(&canonical, &snapshot.canonical_root)
    {
        return Err("文件已移出最近一次扫描的本地磁盘范围".into());
    }
    match has_only_default_data_stream(&snapshot.candidate.path) {
        Ok(true) => {}
        Ok(false) => return Err("文件新增了额外数据流，已安全保留".into()),
        Err(_) => return Err("无法复检文件数据流，已安全保留".into()),
    }

    let file = File::open(&snapshot.candidate.path)
        .map_err(|error| format!("无法打开文件进行身份复检: {error}"))?;
    let handle_metadata = file
        .metadata()
        .map_err(|error| format!("无法读取打开文件的身份信息: {error}"))?;
    validate_large_file_metadata(&handle_metadata, &snapshot.candidate)?;
    let current_identity = file_identity_from_file(&file, &handle_metadata)
        .map_err(|_| "无法确认文件身份，已安全保留".to_string())?;
    if snapshot.candidate.identity != Some(current_identity) {
        return Err("文件身份与最近一次扫描不一致，已安全保留".into());
    }
    let link_count = hard_link_count_from_file(&file, &handle_metadata)
        .map_err(|_| "无法确认文件硬链接状态，已安全保留".to_string())?;
    if link_count != 1 {
        return Err("文件硬链接状态不允许安全删除，已安全保留".into());
    }
    drop(file);

    let after = fs::symlink_metadata(&snapshot.candidate.path)
        .map_err(|error| format!("文件在身份复检后已失效: {error}"))?;
    validate_large_file_metadata(&after, &snapshot.candidate)?;
    validate_large_file_parent_chain(&snapshot.canonical_root, &snapshot.candidate.path)?;
    let canonical_after = fs::canonicalize(&snapshot.candidate.path)
        .map_err(|error| format!("无法完成删除前路径复检: {error}"))?;
    if canonical_after != canonical {
        return Err("文件路径在复检期间发生变化，已安全保留".into());
    }

    fs::remove_file(&snapshot.candidate.path).map_err(|error| format!("永久删除失败: {error}"))?;
    Ok(snapshot.candidate.size_bytes)
}

fn validate_large_file_metadata(
    metadata: &Metadata,
    candidate: &FileCandidate,
) -> Result<(), String> {
    if !metadata.is_file() || is_link_or_reparse(metadata) || is_offline_or_recall(metadata) {
        return Err("文件已变为链接、云占位或非普通文件，已安全保留".into());
    }
    if metadata.len() != candidate.size_bytes || metadata.modified().ok() != candidate.modified {
        return Err("文件大小或修改时间与最近一次扫描不一致，已安全保留".into());
    }
    Ok(())
}

fn validate_large_file_parent_chain(root: &Path, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "文件没有有效父目录".to_string())?;
    let relative = strip_component_prefix(parent, root)
        .ok_or_else(|| "文件父目录不在最近一次扫描范围内".to_string())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&current).map_err(|error| format!("文件父目录已失效: {error}"))?;
        if !metadata.is_dir() || is_link_or_reparse(&metadata) || is_offline_or_recall(&metadata) {
            return Err("文件父目录已变为链接、云占位或非普通目录".into());
        }
    }
    Ok(())
}

/// Finds byte-for-byte duplicate files using a bounded three-stage pipeline:
/// logical size, head/tail SHA-256 sample, then full-file SHA-256.
///
/// Cloud placeholders, links, junctions and all reparse points are excluded
/// before any content is opened. Every reported group has a complete SHA-256
/// match; names and timestamps are never used as duplicate evidence.
pub fn scan_duplicates(
    root: &Path,
    options: DuplicateScanOptions,
    cancel: &AtomicBool,
) -> Result<DuplicateScanResult, String> {
    validate_duplicate_options(&options)?;
    let canonical_root = validate_root(root)?;
    let excluded_paths = normalize_excluded_paths(&canonical_root, &options.excluded_paths)?;
    let visible_root = visible_root(&canonical_root);
    let display_root = visible_root.display().to_string();
    let mut stats = ScanStats::default();
    if is_cancelled(cancel, &mut stats) {
        return Ok(empty_duplicate_result(display_root, stats));
    }

    let mut by_size: HashMap<u64, Vec<FileCandidate>> = HashMap::new();
    walk_regular_files(
        &canonical_root,
        options.max_files,
        options.max_groups,
        &excluded_paths,
        cancel,
        &mut stats,
        |_, _| {},
        |path, metadata, _| {
            if metadata.len() < options.min_size_bytes {
                return;
            }
            by_size
                .entry(metadata.len())
                .or_default()
                .push(FileCandidate {
                    path: path.to_path_buf(),
                    size_bytes: metadata.len(),
                    modified: metadata.modified().ok(),
                    identity: None,
                });
        },
    );

    let candidate_files = by_size
        .values()
        .filter(|files| files.len() > 1)
        .map(|files| files.len() as u64)
        .sum();
    if stats.cancelled {
        let mut result = empty_duplicate_result(display_root, stats);
        result.candidate_files = candidate_files;
        return Ok(result);
    }

    by_size.retain(|_, files| files.len() > 1);
    let mut size_groups = by_size.into_iter().collect::<Vec<_>>();
    size_groups.sort_by(|left, right| right.0.cmp(&left.0));

    let mut sampled_files = 0u64;
    let mut hashed_files = 0u64;
    let mut verified_groups: Vec<([u8; 32], Vec<FileCandidate>)> = Vec::new();

    'sizes: for (_, mut candidates) in size_groups {
        if is_cancelled(cancel, &mut stats) {
            break;
        }
        candidates.sort_by(|left, right| path_cmp(&left.path, &right.path));
        let mut identities = HashSet::new();
        candidates.retain_mut(|candidate| {
            if cancel.load(AtomicOrdering::Relaxed) {
                return false;
            }
            match file_identity(candidate) {
                Ok(identity) if identities.insert(identity) => {
                    candidate.identity = Some(identity);
                    true
                }
                Ok(_) | Err(_) => {
                    // A second directory entry for the same file is a hard-link
                    // alias, not an independently reclaimable duplicate. Identity
                    // failures are also excluded rather than guessed.
                    stats.skipped = stats.skipped.saturating_add(1);
                    false
                }
            }
        });
        if cancel.load(AtomicOrdering::Relaxed) {
            stats.cancelled = true;
            break;
        }
        if candidates.len() < 2 {
            continue;
        }
        let mut by_sample: HashMap<[u8; 32], Vec<FileCandidate>> = HashMap::new();
        for candidate in candidates {
            match sample_hash(&candidate, &canonical_root, options.sample_bytes, cancel) {
                Ok(hash) => {
                    sampled_files = sampled_files.saturating_add(1);
                    by_sample.entry(hash).or_default().push(candidate);
                }
                Err(HashFailure::Skipped) => {
                    stats.skipped = stats.skipped.saturating_add(1);
                }
                Err(HashFailure::Cancelled) => {
                    stats.cancelled = true;
                    break 'sizes;
                }
            }
        }

        let mut sample_groups = by_sample.into_iter().collect::<Vec<_>>();
        sample_groups.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, candidates) in sample_groups {
            if candidates.len() < 2 {
                continue;
            }
            let mut by_full_hash: HashMap<[u8; 32], Vec<FileCandidate>> = HashMap::new();
            for candidate in candidates {
                match full_hash(&candidate, &canonical_root, cancel) {
                    Ok(hash) => {
                        hashed_files = hashed_files.saturating_add(1);
                        by_full_hash.entry(hash).or_default().push(candidate);
                    }
                    Err(HashFailure::Skipped) => {
                        stats.skipped = stats.skipped.saturating_add(1);
                    }
                    Err(HashFailure::Cancelled) => {
                        stats.cancelled = true;
                        break 'sizes;
                    }
                }
            }
            verified_groups.extend(
                by_full_hash
                    .into_iter()
                    .filter(|(_, members)| members.len() > 1),
            );
        }
    }

    verified_groups.sort_by(|left, right| {
        right
            .1
            .first()
            .map(|file| file.size_bytes)
            .unwrap_or(0)
            .cmp(&left.1.first().map(|file| file.size_bytes).unwrap_or(0))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut groups = Vec::new();
    let mut returned_members = 0usize;
    let mut reclaimable_bytes = 0u64;
    for (hash, mut candidates) in verified_groups {
        if groups.len() >= options.max_groups {
            stats.limit_reached = true;
            break;
        }
        let remaining_members = options.max_members.saturating_sub(returned_members);
        if remaining_members < 2 {
            stats.limit_reached = true;
            break;
        }

        candidates.sort_by(|left, right| path_cmp(&left.path, &right.path));
        let total_members = candidates.len();
        if candidates.len() > remaining_members {
            candidates.truncate(remaining_members);
            stats.limit_reached = true;
        }
        if candidates.len() < 2 {
            continue;
        }

        let keep_index = choose_keep_candidate(&candidates);
        let size_bytes = candidates[0].size_bytes;
        let group_reclaimable =
            size_bytes.saturating_mul(candidates.len().saturating_sub(1) as u64);
        let hash_hex = hex_bytes(&hash);
        let members = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let (_, sensitivity, _) = classify_large_file(&candidate.path);
                DuplicateMember {
                    id: stable_id("duplicate-member", &candidate.path),
                    name: candidate
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| candidate.path.display().to_string()),
                    path: visible_path(&canonical_root, &visible_root, &candidate.path)
                        .display()
                        .to_string(),
                    modified_at: format_system_time(candidate.modified),
                    suggested_keep: index == keep_index,
                    protected: sensitivity == Sensitivity::Protected,
                }
            })
            .collect::<Vec<_>>();

        returned_members = returned_members.saturating_add(members.len());
        reclaimable_bytes = reclaimable_bytes.saturating_add(group_reclaimable);
        groups.push(DuplicateGroup {
            id: format!("duplicate-{}-{size_bytes}", &hash_hex[..16]),
            hash: format!("SHA-256 {hash_hex}"),
            size_bytes,
            reclaimable_bytes: group_reclaimable,
            match_kind: DuplicateMatch::FullHash,
            total_members,
            members,
        });
    }

    Ok(DuplicateScanResult {
        root: display_root,
        groups,
        candidate_files,
        sampled_files,
        hashed_files,
        reclaimable_bytes,
        stats,
    })
}

fn empty_duplicate_result(root: String, stats: ScanStats) -> DuplicateScanResult {
    DuplicateScanResult {
        root,
        groups: Vec::new(),
        candidate_files: 0,
        sampled_files: 0,
        hashed_files: 0,
        reclaimable_bytes: 0,
        stats,
    }
}

fn validate_limits(max_files: usize, max_results: usize) -> Result<(), String> {
    if max_files == 0 || max_files > HARD_MAX_FILES {
        return Err(format!("maxFiles 必须在 1 到 {HARD_MAX_FILES} 之间"));
    }
    if max_results == 0 || max_results > HARD_MAX_RESULTS {
        return Err(format!("maxResults 必须在 1 到 {HARD_MAX_RESULTS} 之间"));
    }
    Ok(())
}

fn validate_duplicate_options(options: &DuplicateScanOptions) -> Result<(), String> {
    validate_limits(options.max_files, options.max_groups)?;
    if options.min_size_bytes == 0 {
        return Err("minSizeBytes 必须大于 0，空文件不参与重复文件扫描".into());
    }
    if options.max_members < 2 || options.max_members > HARD_MAX_RESULTS {
        return Err(format!("maxMembers 必须在 2 到 {HARD_MAX_RESULTS} 之间"));
    }
    if options.sample_bytes == 0 || options.sample_bytes > HARD_MAX_SAMPLE_BYTES {
        return Err(format!(
            "sampleBytes 必须在 1 到 {HARD_MAX_SAMPLE_BYTES} 之间"
        ));
    }
    Ok(())
}

fn validate_root(root: &Path) -> Result<PathBuf, String> {
    if root.as_os_str().is_empty() {
        return Err("扫描路径不能为空".into());
    }
    validate_local_root(root)?;
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| format!("无法读取扫描路径 {}: {error}", root.display()))?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return Err("扫描路径必须是普通本地目录，不能是链接、联接点或重解析点".into());
    }
    if is_offline_or_recall(&metadata) {
        return Err("扫描路径是脱机或按需下载目录，为避免触发下载已拒绝扫描".into());
    }
    fs::canonicalize(root).map_err(|error| format!("无法验证扫描路径: {error}"))
}

#[cfg(windows)]
fn validate_local_root(root: &Path) -> Result<(), String> {
    use std::path::Prefix;
    use windows::{core::PCWSTR, Win32::Storage::FileSystem::GetDriveTypeW};

    let drive = match root.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            Prefix::UNC(..) | Prefix::VerbatimUNC(..) => {
                return Err("不允许扫描 UNC 或网络共享，只能分析本地卷".into());
            }
            _ => return Err("扫描路径必须位于本地盘符卷".into()),
        },
        _ => return Err("扫描路径必须是带盘符的绝对本地路径".into()),
    };

    let drive_root = [drive as u16, b':' as u16, b'\\' as u16, 0];
    // DRIVE_REMOTE = 4. The constant lives behind an unrelated generated
    // WindowsProgramming feature, while GetDriveTypeW itself is in FileSystem.
    if unsafe { GetDriveTypeW(PCWSTR(drive_root.as_ptr())) } == 4 {
        return Err("不允许扫描映射网络驱动器，只能分析本地卷".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn validate_local_root(root: &Path) -> Result<(), String> {
    if root.is_absolute() {
        Ok(())
    } else {
        Err("扫描路径必须是绝对路径".into())
    }
}

fn visible_root(canonical_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let text = canonical_root.to_string_lossy();
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(local) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(local);
        }
    }
    canonical_root.to_path_buf()
}

fn visible_path(canonical_root: &Path, display_root: &Path, internal_path: &Path) -> PathBuf {
    internal_path
        .strip_prefix(canonical_root)
        .map(|relative| display_root.join(relative))
        .unwrap_or_else(|_| visible_root(internal_path))
}

fn normalize_excluded_paths(
    canonical_root: &Path,
    requested_paths: &[String],
) -> Result<Vec<PathBuf>, String> {
    if requested_paths.len() > HARD_MAX_EXCLUDED_PATHS {
        return Err(format!(
            "excludedPaths 最多允许 {HARD_MAX_EXCLUDED_PATHS} 项"
        ));
    }

    let display_root = visible_root(canonical_root);
    let mut normalized_paths = Vec::with_capacity(requested_paths.len());
    for requested in requested_paths {
        let requested = requested.trim();
        if requested.is_empty() {
            return Err("excludedPaths 不能包含空路径".into());
        }
        if requested.encode_utf16().count() > HARD_MAX_EXCLUDED_PATH_LENGTH {
            return Err(format!(
                "excludedPaths 单项不能超过 {HARD_MAX_EXCLUDED_PATH_LENGTH} 个 UTF-16 代码单元"
            ));
        }

        let requested_path = PathBuf::from(requested);
        if requested_path.components().next().is_some_and(|component| {
            matches!(component, Component::Prefix(_)) && !requested_path.is_absolute()
        }) {
            return Err(format!("排除路径不是绝对路径: {requested}"));
        }
        let display_candidate = if requested_path.is_absolute() {
            requested_path
        } else {
            display_root.join(requested_path)
        };
        let display_candidate = lexical_normalize_absolute(&display_candidate)
            .ok_or_else(|| format!("排除路径无法规范化或试图越过卷根: {requested}"))?;
        let relative = strip_component_prefix(&display_candidate, &display_root)
            .ok_or_else(|| format!("排除路径不在扫描根目录内: {requested}"))?;
        if relative.as_os_str().is_empty() {
            return Err("不能把扫描根目录本身加入 excludedPaths".into());
        }

        let lexical_internal = canonical_root.join(relative);
        let internal = match fs::symlink_metadata(&lexical_internal) {
            Ok(_) => fs::canonicalize(&lexical_internal)
                .map_err(|error| format!("无法验证排除路径 {requested}: {error}"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => lexical_internal,
            Err(error) => return Err(format!("无法读取排除路径 {requested}: {error}")),
        };
        if internal == canonical_root || !path_is_same_or_descendant(&internal, canonical_root) {
            return Err(format!("排除路径解析后不在扫描根目录内: {requested}"));
        }
        normalized_paths.push(internal);
    }

    normalized_paths.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| path_cmp(left, right))
    });
    let mut minimal_paths: Vec<PathBuf> = Vec::with_capacity(normalized_paths.len());
    for path in normalized_paths {
        if minimal_paths
            .iter()
            .any(|ancestor| path_is_same_or_descendant(&path, ancestor))
        {
            continue;
        }
        minimal_paths.push(path);
    }
    Ok(minimal_paths)
}

fn lexical_normalize_absolute(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    let mut normal_components = 0usize;
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(value) => {
                normalized.push(value);
                normal_components = normal_components.saturating_add(1);
            }
            Component::ParentDir => {
                if normal_components == 0 || !normalized.pop() {
                    return None;
                }
                normal_components -= 1;
            }
        }
    }
    Some(normalized)
}

fn strip_component_prefix(path: &Path, prefix: &Path) -> Option<PathBuf> {
    let mut path_components = path.components();
    for prefix_component in prefix.components() {
        let path_component = path_components.next()?;
        if !components_equal(path_component, prefix_component) {
            return None;
        }
    }
    Some(path_components.collect())
}

fn components_equal(left: Component<'_>, right: Component<'_>) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn path_is_same_or_descendant(path: &Path, ancestor: &Path) -> bool {
    strip_component_prefix(path, ancestor).is_some()
}

fn walk_regular_files<DirectoryVisitor, FileVisitor>(
    root: &Path,
    max_files: usize,
    result_limit: usize,
    excluded_paths: &[PathBuf],
    cancel: &AtomicBool,
    stats: &mut ScanStats,
    mut visit_directory: DirectoryVisitor,
    mut visit_file: FileVisitor,
) where
    DirectoryVisitor: FnMut(&Path, usize),
    FileVisitor: FnMut(&Path, &Metadata, usize),
{
    let filtered_skipped = Cell::new(0u64);
    let entries = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if excluded_paths
                .iter()
                .any(|excluded| path_is_same_or_descendant(entry.path(), excluded))
            {
                return false;
            }
            match fs::symlink_metadata(entry.path()) {
                Ok(metadata) if !is_link_or_reparse(&metadata) => true,
                Ok(_) | Err(_) => {
                    filtered_skipped.set(filtered_skipped.get().saturating_add(1));
                    false
                }
            }
        });
    let max_entries = max_files
        .saturating_mul(4)
        .saturating_add(result_limit.saturating_mul(2))
        .saturating_add(1024)
        .min(HARD_MAX_ENTRIES);

    for entry in entries {
        if is_cancelled(cancel, stats) {
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                stats.skipped = stats.skipped.saturating_add(1);
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }

        if stats.visited_entries >= max_entries as u64 {
            stats.limit_reached = true;
            break;
        }
        stats.visited_entries = stats.visited_entries.saturating_add(1);

        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(_) => {
                stats.skipped = stats.skipped.saturating_add(1);
                continue;
            }
        };
        if is_link_or_reparse(&metadata) || is_offline_or_recall(&metadata) {
            stats.skipped = stats.skipped.saturating_add(1);
            continue;
        }
        if metadata.is_dir() {
            visit_directory(entry.path(), entry.depth());
            continue;
        }
        if !metadata.is_file() {
            stats.skipped = stats.skipped.saturating_add(1);
            continue;
        }
        if stats.scanned_files >= max_files as u64 {
            stats.limit_reached = true;
            break;
        }
        stats.scanned_files = stats.scanned_files.saturating_add(1);
        visit_file(entry.path(), &metadata, entry.depth());
    }

    stats.skipped = stats.skipped.saturating_add(filtered_skipped.get());
    if cancel.load(AtomicOrdering::Relaxed) {
        stats.cancelled = true;
    }
}

fn is_cancelled(cancel: &AtomicBool, stats: &mut ScanStats) -> bool {
    if cancel.load(AtomicOrdering::Relaxed) {
        stats.cancelled = true;
        true
    } else {
        false
    }
}

fn classify_storage_category(path: &Path) -> CategoryDefinition {
    if contains_any_component(
        path,
        &[
            "windows",
            "recovery",
            "system volume information",
            "$windows.~bt",
            "$windows.~ws",
        ],
    ) {
        return CATEGORY_SYSTEM;
    }
    if contains_any_component(
        path,
        &[
            "program files",
            "program files (x86)",
            "programdata",
            "appdata",
            "windowsapps",
            "steamapps",
        ],
    ) {
        return CATEGORY_APPS;
    }

    let extension = extension_lower(path);
    if matches_extension(
        &extension,
        &[
            "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "tif", "tiff", "raw", "cr2", "nef",
            "dng", "svg", "mp4", "mkv", "mov", "avi", "wmv", "webm", "m4v", "mpeg", "mpg", "ts",
            "mts", "mp3", "wav", "flac", "aac", "m4a", "ogg", "wma",
        ],
    ) {
        CATEGORY_MEDIA
    } else if matches_extension(
        &extension,
        &[
            "exe", "dll", "msi", "msix", "appx", "sys", "ocx", "cpl", "jar", "apk", "pak", "cab",
            "nupkg",
        ],
    ) {
        CATEGORY_APPS
    } else if matches_extension(
        &extension,
        &[
            "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf", "txt", "md", "rtf", "odt", "ods",
            "csv", "json", "xml", "yaml", "yml", "toml", "rs", "c", "cc", "cpp", "h", "hpp", "cs",
            "java", "kt", "go", "py", "js", "jsx", "ts", "tsx", "vue", "html", "css", "scss",
            "sql", "psd", "ai", "sketch", "fig", "dwg", "dxf", "blend", "zip", "7z", "rar", "tar",
            "gz", "bz2", "xz", "bak",
        ],
    ) {
        CATEGORY_DOCUMENTS
    } else if matches_extension(&extension, &["efi", "mui", "cat", "manifest", "mum", "dat"]) {
        CATEGORY_SYSTEM
    } else {
        CATEGORY_OTHER
    }
}

fn classify_directory(path: &Path) -> (&'static str, &'static str) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match name.as_str() {
        "users" => ("用户文件", "#265dff"),
        "windows" => ("系统", "#f0a02f"),
        "program files" | "program files (x86)" => ("应用", "#10a37f"),
        "programdata" => ("应用数据", "#7b67d8"),
        "recovery" | "system volume information" => ("受保护", "#82909f"),
        _ => ("文件夹", directory_color(&name)),
    }
}

fn directory_color(name: &str) -> &'static str {
    const COLORS: [&str; 6] = [
        "#265dff", "#10a37f", "#f0a02f", "#7b67d8", "#e05d6f", "#3a8ca8",
    ];
    let index = name
        .bytes()
        .fold(0usize, |value, byte| value.wrapping_mul(31) ^ byte as usize)
        % COLORS.len();
    COLORS[index]
}

fn classify_large_file(path: &Path) -> (String, Sensitivity, Option<String>) {
    let extension = extension_lower(path);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    let critical_path = contains_any_component(
        path,
        &[
            "windows",
            "program files",
            "program files (x86)",
            "programdata",
            "recovery",
            "system volume information",
            "windowsapps",
        ],
    );
    let cloud_path = path_components_lower(path)
        .iter()
        .any(|component| component == "onedrive" || component.ends_with("drivefs"));
    let critical_name = matches!(
        file_name.as_str(),
        "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "ntuser.dat" | "usrclass.dat"
    );

    let file_type = match extension.as_str() {
        "iso" | "img" | "wim" | "esd" => "磁盘镜像",
        "vhd" | "vhdx" | "vmdk" | "vdi" => "虚拟磁盘",
        "pst" | "ost" => "邮件存档",
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => "压缩包",
        "bak" | "backup" => "备份文件",
        "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" | "mpeg" | "mpg" | "ts" | "mts" => {
            "视频"
        }
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "wma" => "音频",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "tif" | "tiff" | "raw"
        | "cr2" | "nef" | "dng" => "图片",
        "psd" | "ai" | "sketch" | "fig" | "dwg" | "dxf" | "blend" => "设计文件",
        "db" | "sqlite" | "sqlite3" | "mdb" | "accdb" | "edb" => "数据库",
        "exe" | "msi" | "msix" | "appx" => "程序或安装包",
        "dll" | "sys" | "ocx" => "系统或程序文件",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pdf" | "txt" | "md" => "文档",
        "" => "无扩展名文件",
        _ => "其他文件",
    }
    .to_string();

    if critical_path || critical_name {
        return (
            file_type,
            Sensitivity::Protected,
            Some("系统或应用关键路径中的文件，不应作为垃圾直接删除".into()),
        );
    }
    if matches_extension(
        &extension,
        &["vhd", "vhdx", "vmdk", "vdi", "pst", "ost", "edb"],
    ) {
        return (
            file_type,
            Sensitivity::Protected,
            Some("可能承载虚拟机、子系统、邮件或数据库数据".into()),
        );
    }
    if cloud_path {
        return (
            file_type,
            Sensitivity::Protected,
            Some("位于云同步目录；仅分析本地可用文件，操作前需确认同步状态".into()),
        );
    }
    if matches_extension(
        &extension,
        &[
            "iso", "img", "wim", "esd", "zip", "7z", "rar", "tar", "gz", "bz2", "xz", "bak",
            "backup", "exe", "msi", "msix", "appx",
        ],
    ) || file_name.contains("backup")
        || file_name.contains("备份")
    {
        return (
            file_type,
            Sensitivity::Attention,
            Some("安装介质、压缩包或备份并不等同于垃圾文件".into()),
        );
    }
    (file_type, Sensitivity::Normal, None)
}

fn sort_large_files(files: &mut [LargeFileEntry]) {
    files.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| path_text_cmp(&left.path, &right.path))
    });
}

#[derive(Clone, Copy, Debug)]
enum HashFailure {
    Skipped,
    Cancelled,
}

impl From<io::Error> for HashFailure {
    fn from(_error: io::Error) -> Self {
        Self::Skipped
    }
}

fn sample_hash(
    candidate: &FileCandidate,
    root: &Path,
    sample_bytes: usize,
    cancel: &AtomicBool,
) -> Result<[u8; 32], HashFailure> {
    let (mut file, before) = open_hash_candidate(candidate, root, cancel)?;
    let mut hasher = Sha256::new();
    hasher.update(b"QINGPAN-SAMPLE-V1\0");
    hasher.update(candidate.size_bytes.to_le_bytes());

    if candidate.size_bytes <= sample_bytes.saturating_mul(2) as u64 {
        let length = usize::try_from(candidate.size_bytes).map_err(|_| HashFailure::Skipped)?;
        let mut bytes = vec![0u8; length];
        read_exact_cancelled(&mut file, &mut bytes, cancel)?;
        hasher.update(&bytes);
    } else {
        let mut head = vec![0u8; sample_bytes];
        read_exact_cancelled(&mut file, &mut head, cancel)?;
        hasher.update(&head);
        file.seek(SeekFrom::End(-(sample_bytes as i64)))
            .map_err(|_| HashFailure::Skipped)?;
        let mut tail = vec![0u8; sample_bytes];
        read_exact_cancelled(&mut file, &mut tail, cancel)?;
        hasher.update(&tail);
    }

    validate_after_hash(&file, &before, candidate)?;
    Ok(hasher.finalize().into())
}

fn full_hash(
    candidate: &FileCandidate,
    root: &Path,
    cancel: &AtomicBool,
) -> Result<[u8; 32], HashFailure> {
    let (mut file, before) = open_hash_candidate(candidate, root, cancel)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut read_bytes = 0u64;
    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            return Err(HashFailure::Cancelled);
        }
        let count = match file.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(HashFailure::Skipped),
        };
        if count == 0 {
            break;
        }
        read_bytes = read_bytes.saturating_add(count as u64);
        if read_bytes > candidate.size_bytes {
            return Err(HashFailure::Skipped);
        }
        hasher.update(&buffer[..count]);
    }
    if read_bytes != candidate.size_bytes {
        return Err(HashFailure::Skipped);
    }
    validate_after_hash(&file, &before, candidate)?;
    Ok(hasher.finalize().into())
}

fn open_hash_candidate(
    candidate: &FileCandidate,
    root: &Path,
    cancel: &AtomicBool,
) -> Result<(File, Metadata), HashFailure> {
    if cancel.load(AtomicOrdering::Relaxed) {
        return Err(HashFailure::Cancelled);
    }
    let metadata = fs::symlink_metadata(&candidate.path).map_err(|_| HashFailure::Skipped)?;
    if !metadata.is_file()
        || is_link_or_reparse(&metadata)
        || is_offline_or_recall(&metadata)
        || metadata.len() != candidate.size_bytes
        || metadata.modified().ok() != candidate.modified
    {
        return Err(HashFailure::Skipped);
    }
    let canonical = fs::canonicalize(&candidate.path).map_err(|_| HashFailure::Skipped)?;
    if canonical == root || !canonical.starts_with(root) {
        return Err(HashFailure::Skipped);
    }
    if !has_only_default_data_stream(&candidate.path)? {
        return Err(HashFailure::Skipped);
    }
    let file = File::open(&candidate.path).map_err(|_| HashFailure::Skipped)?;
    let handle_metadata = file.metadata().map_err(|_| HashFailure::Skipped)?;
    if !handle_metadata.is_file() || handle_metadata.len() != candidate.size_bytes {
        return Err(HashFailure::Skipped);
    }
    let identity = file_identity_from_file(&file, &handle_metadata)?;
    if candidate.identity != Some(identity) {
        return Err(HashFailure::Skipped);
    }
    Ok((file, metadata))
}

fn validate_after_hash(
    file: &File,
    before: &Metadata,
    candidate: &FileCandidate,
) -> Result<(), HashFailure> {
    let after = file.metadata().map_err(|_| HashFailure::Skipped)?;
    if after.len() != candidate.size_bytes
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || !has_only_default_data_stream(&candidate.path)?
    {
        return Err(HashFailure::Skipped);
    }
    Ok(())
}

fn file_identity(candidate: &FileCandidate) -> Result<FileIdentity, HashFailure> {
    file_identity_and_link_count(candidate).map(|(identity, _)| identity)
}

fn file_identity_and_link_count(
    candidate: &FileCandidate,
) -> Result<(FileIdentity, u64), HashFailure> {
    let metadata = fs::symlink_metadata(&candidate.path).map_err(|_| HashFailure::Skipped)?;
    if !metadata.is_file()
        || is_link_or_reparse(&metadata)
        || is_offline_or_recall(&metadata)
        || metadata.len() != candidate.size_bytes
        || metadata.modified().ok() != candidate.modified
    {
        return Err(HashFailure::Skipped);
    }
    let file = File::open(&candidate.path).map_err(|_| HashFailure::Skipped)?;
    let handle_metadata = file.metadata().map_err(|_| HashFailure::Skipped)?;
    if !handle_metadata.is_file()
        || is_link_or_reparse(&handle_metadata)
        || is_offline_or_recall(&handle_metadata)
        || handle_metadata.len() != candidate.size_bytes
        || handle_metadata.modified().ok() != candidate.modified
    {
        return Err(HashFailure::Skipped);
    }
    Ok((
        file_identity_from_file(&file, &handle_metadata)?,
        hard_link_count_from_file(&file, &handle_metadata)?,
    ))
}

fn read_exact_cancelled(
    file: &mut File,
    mut buffer: &mut [u8],
    cancel: &AtomicBool,
) -> Result<(), HashFailure> {
    while !buffer.is_empty() {
        if cancel.load(AtomicOrdering::Relaxed) {
            return Err(HashFailure::Cancelled);
        }
        match file.read(buffer) {
            Ok(0) => return Err(HashFailure::Skipped),
            Ok(count) => {
                let (_, rest) = buffer.split_at_mut(count);
                buffer = rest;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Err(HashFailure::Skipped),
        }
    }
    Ok(())
}

fn choose_keep_candidate(candidates: &[FileCandidate]) -> usize {
    candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| {
            let (_, sensitivity, _) = classify_large_file(&candidate.path);
            (
                if sensitivity == Sensitivity::Protected {
                    0
                } else {
                    1
                },
                if is_disposable_location(&candidate.path) {
                    1
                } else {
                    0
                },
                system_time_seconds(candidate.modified).unwrap_or(u64::MAX),
                candidate.path.to_string_lossy().to_ascii_lowercase(),
            )
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn is_disposable_location(path: &Path) -> bool {
    contains_any_component(path, &["temp", "tmp", "cache", "downloads", "$recycle.bin"])
}

fn contains_any_component(path: &Path, expected: &[&str]) -> bool {
    path_components_lower(path)
        .iter()
        .any(|component| expected.iter().any(|item| component == item))
}

fn path_components_lower(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn matches_extension(extension: &str, expected: &[&str]) -> bool {
    expected.contains(&extension)
}

fn stable_id(prefix: &str, path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().to_ascii_lowercase().as_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    format!("{prefix}-{}", &hex_bytes(&digest)[..20])
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn path_cmp(left: &Path, right: &Path) -> Ordering {
    path_text_cmp(&left.to_string_lossy(), &right.to_string_lossy())
}

fn path_text_cmp(left: &str, right: &str) -> Ordering {
    left.to_ascii_lowercase()
        .cmp(&right.to_ascii_lowercase())
        .then_with(|| left.cmp(right))
}

fn system_time_seconds(value: Option<SystemTime>) -> Option<u64> {
    value?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time| time.as_secs())
}

fn format_system_time(value: Option<SystemTime>) -> String {
    let Some(seconds) = system_time_seconds(value) else {
        return "未知".into();
    };
    let days = (seconds / 86_400) as i64;
    let seconds_in_day = seconds % 86_400;
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day % 3_600 / 60;
    let second = seconds_in_day % 60;
    let (year, month, day) = civil_date_from_unix_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Gregorian conversion adapted from the public-domain civil calendar formula
// by Howard Hinnant. It avoids adding a date dependency solely for JSON text.
fn civil_date_from_unix_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering as TestOrdering},
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = TEST_COUNTER.fetch_add(1, TestOrdering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "qingpan-storage-{label}-{}-{nanos}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent directory");
            }
            let mut file = File::create(&path).expect("create test file");
            file.write_all(bytes).expect("write test file");
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn directory_usage_reports_only_direct_children_and_categories() {
        let root = TestDirectory::new("usage");
        root.write("Media/Photos/image.jpg", b"image-data");
        root.write("Projects/source.rs", b"fn main() {}");
        root.write("root.bin", b"root");

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan usage");

        assert_eq!(result.total_file_count, 3);
        assert_eq!(result.directories.len(), 2);
        assert!(result
            .directories
            .iter()
            .any(|directory| directory.name == "Media" && directory.file_count == 1));
        assert_eq!(
            result
                .categories
                .iter()
                .map(|category| category.file_count)
                .sum::<u64>(),
            3
        );
        assert_eq!(
            classify_storage_category(Path::new("Photos/image.jpg")).id,
            "media"
        );
        assert_eq!(
            classify_storage_category(Path::new("Projects/source.rs")).id,
            "docs"
        );
        #[cfg(windows)]
        {
            assert!(!result.root.starts_with(r"\\?\"));
            assert!(result
                .directories
                .iter()
                .all(|directory| !directory.path.starts_with(r"\\?\")));
        }
    }

    #[test]
    fn linked_directory_is_not_followed() {
        let root = TestDirectory::new("links");
        root.write("real/only-once.bin", b"12345678");
        let linked = root.path.join("linked");
        if create_directory_link(&root.path.join("real"), &linked).is_err() {
            // Windows without Developer Mode may deny symlink creation. The
            // production reparse-point check is still compiled on that host.
            return;
        }

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan usage");

        assert_eq!(result.total_file_count, 1);
        assert_eq!(result.total_size_bytes, 8);
        assert!(result.stats.skipped >= 1);
        assert!(!result
            .directories
            .iter()
            .any(|directory| directory.name == "linked"));
    }

    #[test]
    fn exclusions_prune_directories_and_skip_exact_files() {
        let root = TestDirectory::new("exclusions");
        root.write("Excluded/nested/hidden.bin", b"hidden-directory-content");
        let exact_file = root.write("Exact/hidden.bin", b"hidden-file-content");
        root.write("Keep/visible.bin", b"visible");

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: vec![
                    root.path.join("Excluded").display().to_string(),
                    exact_file.display().to_string(),
                ],
            },
            &AtomicBool::new(false),
        )
        .expect("scan with exclusions");

        assert_eq!(result.total_file_count, 1);
        assert_eq!(result.total_size_bytes, 7);
        assert!(!result
            .directories
            .iter()
            .any(|directory| directory.name == "Excluded"));
        assert!(result
            .directories
            .iter()
            .any(|directory| directory.name == "Keep" && directory.file_count == 1));
    }

    #[test]
    fn exclusion_outside_root_is_rejected() {
        let root = TestDirectory::new("exclusion-root");
        let outside = TestDirectory::new("exclusion-outside");
        let outside_file = outside.write("outside.bin", b"outside");

        let error = scan_large_files(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: vec![outside_file.display().to_string()],
            },
            &AtomicBool::new(false),
        )
        .expect_err("outside exclusion must fail");

        assert!(error.contains("不在扫描根目录内"));
    }

    #[test]
    fn unchanged_large_file_snapshot_is_deleted_after_identity_recheck() {
        let root = TestDirectory::new("large-delete");
        let path = root.write("recording.mp4", b"large-video-content");
        let (result, mut snapshots) = scan_large_files_with_snapshots(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan large files with snapshots");
        let entry = result.files.first().expect("large file result");
        let snapshot = snapshots.remove(&entry.id).expect("executable snapshot");

        let deleted = delete_large_file(&snapshot).expect("delete unchanged large file");

        assert_eq!(deleted, 19);
        assert!(!path.exists());
    }

    #[test]
    fn changed_large_file_is_retained_by_delete_recheck() {
        let root = TestDirectory::new("large-changed");
        let path = root.write("recording.mp4", b"before");
        let (result, mut snapshots) = scan_large_files_with_snapshots(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan large files with snapshots");
        let entry = result.files.first().expect("large file result");
        let snapshot = snapshots.remove(&entry.id).expect("executable snapshot");
        fs::write(&path, b"changed and larger").expect("change large file after scan");

        let error = delete_large_file(&snapshot).expect_err("changed file must be retained");

        assert!(error.contains("大小或修改时间"));
        assert!(path.exists());
    }

    #[test]
    fn protected_large_file_never_receives_an_executable_snapshot() {
        let root = TestDirectory::new("large-protected");
        root.write("machine.vhdx", b"virtual-disk");
        let (result, snapshots) = scan_large_files_with_snapshots(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan protected large file");

        assert_eq!(result.files[0].sensitivity, Sensitivity::Protected);
        assert!(snapshots.is_empty());
    }

    #[test]
    fn duplicate_scan_requires_equal_full_content() {
        let root = TestDirectory::new("duplicates");
        let same = b"abcdefgh-duplicate-content";
        root.write("one/a.bin", same);
        root.write("two/b.bin", same);
        root.write("three/c.bin", b"abcdefgh-different-content");

        let result = scan_duplicates(
            &root.path,
            DuplicateScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_groups: 10,
                max_members: 20,
                sample_bytes: 4,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan duplicates");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert_eq!(result.groups[0].size_bytes, same.len() as u64);
        assert_eq!(result.groups[0].reclaimable_bytes, same.len() as u64);
        assert!(result.groups[0].hash.starts_with("SHA-256 "));
        assert_eq!(
            result.groups[0]
                .members
                .iter()
                .filter(|member| member.suggested_keep)
                .count(),
            1
        );
    }

    #[test]
    fn same_size_different_content_is_not_a_duplicate() {
        let root = TestDirectory::new("different");
        root.write("a.bin", b"AAAA1111");
        root.write("b.bin", b"BBBB2222");

        let result = scan_duplicates(
            &root.path,
            DuplicateScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_groups: 10,
                max_members: 20,
                sample_bytes: 2,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan duplicates");

        assert!(result.groups.is_empty());
        assert_eq!(result.hashed_files, 0);
    }

    #[test]
    fn hard_links_are_not_counted_as_reclaimable_copies() {
        let root = TestDirectory::new("hard-links");
        let original = root.write("one/original.bin", b"identical-content");
        let alias = root.path.join("two/alias.bin");
        fs::create_dir_all(alias.parent().expect("alias parent")).expect("create alias parent");
        fs::hard_link(&original, &alias).expect("create hard link");
        root.write("three/real-copy.bin", b"identical-content");

        let result = scan_duplicates(
            &root.path,
            DuplicateScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_groups: 10,
                max_members: 20,
                sample_bytes: 4,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan hard links");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert_eq!(result.groups[0].reclaimable_bytes, 17);
        assert!(result.stats.skipped >= 1);
    }

    #[cfg(windows)]
    #[test]
    fn file_with_named_data_stream_is_excluded_from_duplicates() {
        let root = TestDirectory::new("ads");
        let first = root.write("first.bin", b"same-default-stream");
        root.write("second.bin", b"same-default-stream");
        let stream_path = PathBuf::from(format!("{}:qingpan-test", first.display()));
        let Ok(mut stream) = File::create(stream_path) else {
            // Non-NTFS temporary volumes do not support alternate streams.
            return;
        };
        stream.write_all(b"additional-stream").expect("write ADS");
        drop(stream);

        let result = scan_duplicates(
            &root.path,
            DuplicateScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_groups: 10,
                max_members: 20,
                sample_bytes: 4,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan ADS candidates");

        assert!(result.groups.is_empty());
        assert!(result.stats.skipped >= 1);
    }

    /// A hard link exposes already-counted clusters through a second path.
    /// Charging it again is exactly the inflation that made shared stores such as
    /// WinSxS and the pnpm store report more than their real occupancy.
    #[test]
    fn hard_linked_content_is_counted_once_in_directory_usage() {
        let root = TestDirectory::new("usage-hard-links");
        // Must clear HARD_LINK_CHECK_MINIMUM_BYTES to be eligible for the check.
        let payload = vec![7u8; (HARD_LINK_CHECK_MINIMUM_BYTES as usize) * 2];
        let original = root.write("one/original.bin", &payload);
        let alias = root.path.join("two/alias.bin");
        fs::create_dir_all(alias.parent().expect("alias parent")).expect("create alias parent");
        if fs::hard_link(&original, &alias).is_err() {
            // Filesystems without hard-link support cannot exercise this path.
            return;
        }

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan hard-linked usage");

        // Both paths are walked, but the shared clusters are charged a single time.
        assert_eq!(result.total_file_count, 1);
        assert_eq!(result.total_size_bytes, payload.len() as u64);
        assert_eq!(result.stats.deduplicated_hard_links, 1);
        // Deleting one alias frees nothing, so only one directory owns the bytes.
        let charged = result
            .directories
            .iter()
            .filter(|directory| directory.size_bytes > 0)
            .count();
        assert_eq!(charged, 1);
    }

    /// Small hard links are deliberately left counted more than once, because
    /// bounding handle opens matters more than perfect accounting for bytes that
    /// cannot add up to a meaningful saving. This pins that tradeoff so it cannot
    /// change silently.
    #[test]
    fn hard_links_below_the_size_floor_are_not_deduplicated() {
        let root = TestDirectory::new("usage-small-links");
        let original = root.write("one/small.bin", b"tiny-shared-content");
        let alias = root.path.join("two/small-alias.bin");
        fs::create_dir_all(alias.parent().expect("alias parent")).expect("create alias parent");
        if fs::hard_link(&original, &alias).is_err() {
            return;
        }

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan small hard links");

        assert_eq!(result.total_file_count, 2);
        assert_eq!(result.stats.deduplicated_hard_links, 0);
    }

    /// Physical occupancy is the number that predicts freed space. A sparse file
    /// reserves no clusters, so reporting its logical length would promise a
    /// saving that deleting it cannot deliver.
    #[cfg(windows)]
    #[test]
    fn sparse_file_reports_physical_allocation_below_logical_size() {
        let root = TestDirectory::new("usage-sparse");
        let sparse = root.write("sparse.bin", b"");
        let status = std::process::Command::new("fsutil")
            .args(["sparse", "setflag"])
            .arg(&sparse)
            .status();
        if !matches!(status, Ok(code) if code.success()) {
            // Requires NTFS; skip on volumes that do not support sparse files.
            return;
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&sparse)
            .expect("open sparse file");
        file.set_len(64 * 1024 * 1024).expect("extend sparse file");
        drop(file);

        let metadata = fs::symlink_metadata(&sparse).expect("sparse metadata");
        let allocated = allocated_size(&sparse, &metadata).expect("query allocation");
        assert_eq!(metadata.len(), 64 * 1024 * 1024);
        // Zero is a legitimate measurement here and must not be replaced by the
        // logical length, otherwise the reported saving is fabricated.
        assert!(
            allocated < metadata.len(),
            "sparse allocation {allocated} should stay below logical size"
        );

        let result = scan_directory_usage(
            &root.path,
            DirectoryScanOptions {
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect("scan sparse usage");
        assert_eq!(result.total_size_bytes, 64 * 1024 * 1024);
        assert!(result.total_allocated_bytes < result.total_size_bytes);
    }

    /// An ordinary file must report identical logical and physical sizes, so the
    /// new allocation query cannot silently understate common content.
    #[test]
    fn ordinary_file_reports_matching_logical_and_physical_size() {
        let root = TestDirectory::new("usage-ordinary");
        let path = root.write("plain.bin", &[3u8; 8192]);
        let metadata = fs::symlink_metadata(&path).expect("plain metadata");

        let allocated = allocated_size(&path, &metadata).expect("query allocation");
        assert_eq!(metadata.len(), 8192);
        assert!(
            allocated >= metadata.len(),
            "ordinary allocation {allocated} must not understate 8192 bytes"
        );
    }

    /// A missing path cannot be measured. Surfacing the error lets the caller
    /// fall back to the logical length instead of recording a bogus zero.
    #[test]
    fn allocation_query_fails_for_a_missing_path() {
        let root = TestDirectory::new("usage-missing");
        let present = root.write("present.bin", b"content");
        let metadata = fs::symlink_metadata(&present).expect("metadata");
        let missing = root.path.join("absent.bin");

        assert!(allocated_size(&missing, &metadata).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn unc_roots_are_rejected_before_access() {
        let error = scan_large_files(
            Path::new(r"\\server\share"),
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect_err("UNC root must fail");
        assert!(error.contains("UNC") || error.contains("网络"));
    }

    #[test]
    fn pre_cancelled_scan_returns_cancelled_without_results() {
        let root = TestDirectory::new("cancelled");
        root.write("large.bin", b"content");
        let cancel = AtomicBool::new(true);

        let large = scan_large_files(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &cancel,
        )
        .expect("cancel large scan");
        let duplicates = scan_duplicates(
            &root.path,
            DuplicateScanOptions {
                min_size_bytes: 1,
                max_files: 100,
                max_groups: 10,
                max_members: 20,
                sample_bytes: 2,
                excluded_paths: Vec::new(),
            },
            &cancel,
        )
        .expect("cancel duplicate scan");

        assert!(large.stats.cancelled);
        assert!(large.files.is_empty());
        assert!(duplicates.stats.cancelled);
        assert!(duplicates.groups.is_empty());
    }

    #[test]
    fn invalid_limits_are_rejected() {
        let root = TestDirectory::new("limits");
        let error = scan_large_files(
            &root.path,
            LargeFileScanOptions {
                min_size_bytes: 1,
                max_files: 0,
                max_results: 10,
                excluded_paths: Vec::new(),
            },
            &AtomicBool::new(false),
        )
        .expect_err("zero max files must fail");
        assert!(error.contains("maxFiles"));
    }

    #[cfg(unix)]
    fn create_directory_link(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_directory_link(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }
}
