use crate::models::{CleanupItem, DeleteMode, RiskLevel};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
#[derive(Clone)]
pub struct Rule {
    pub id: &'static str,
    pub category: &'static str,
    pub name: &'static str,
    pub relative: &'static str,
    pub risk: RiskLevel,
}
pub fn rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "temp",
            category: "系统缓存",
            name: "临时文件",
            relative: "Temp",
            risk: RiskLevel::Low,
        },
        Rule {
            id: "thumbs",
            category: "系统缓存",
            name: "缩略图缓存",
            relative: "Microsoft/Windows/Explorer",
            risk: RiskLevel::Low,
        },
        Rule {
            id: "edge",
            category: "应用缓存",
            name: "Edge 浏览器缓存",
            relative: "Microsoft/Edge/User Data/Default/Cache",
            risk: RiskLevel::Low,
        },
        Rule {
            id: "chrome",
            category: "应用缓存",
            name: "Chrome 浏览器缓存",
            relative: "Google/Chrome/User Data/Default/Cache",
            risk: RiskLevel::Low,
        },
    ]
}
pub fn local_root() -> Option<PathBuf> {
    dirs::data_local_dir()
}
pub fn path_for(rule: &Rule) -> Option<PathBuf> {
    let root = local_root()?;
    Some(root.join(rule.relative))
}
pub fn folder_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok().map(|m| m.len()))
        .sum()
}
pub fn scan() -> Vec<CleanupItem> {
    rules()
        .into_iter()
        .filter_map(|r| {
            let p = path_for(&r)?;
            if !p.exists() {
                return None;
            }
            Some(CleanupItem {
                id: r.id.into(),
                category: r.category.into(),
                name: r.name.into(),
                path: p.display().to_string(),
                description: "可由应用或 Windows 自动重新生成".into(),
                size_bytes: folder_size(&p),
                risk: r.risk,
                delete_mode: DeleteMode::Permanent,
            })
        })
        .collect()
}
pub fn validated_path(id: &str) -> Result<PathBuf, String> {
    let rule = rules()
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("未知清理条目")?;
    let path = path_for(&rule).ok_or("无法定位用户缓存目录")?;
    let root = local_root().ok_or("无法定位用户目录")?;
    let parent = path.parent().unwrap_or(&path);
    let canonical_parent = fs::canonicalize(parent).map_err(|_| "路径已失效")?;
    let canonical_root = fs::canonicalize(root).map_err(|_| "无法验证用户目录")?;
    if !canonical_parent.starts_with(canonical_root) {
        return Err("路径不在允许范围内".into());
    }
    Ok(path)
}
pub fn clear_contents(path: &Path) -> Result<u64, String> {
    let before = folder_size(path);
    for item in fs::read_dir(path).map_err(|e| e.to_string())? {
        let p = item.map_err(|e| e.to_string())?.path();
        let result = if p.is_dir() {
            fs::remove_dir_all(&p)
        } else {
            fs::remove_file(&p)
        };
        if result.is_err() {
            continue;
        }
    }
    Ok(before.saturating_sub(folder_size(path)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_unique() {
        let r = rules();
        let mut ids = r.iter().map(|x| x.id).collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), r.len())
    }
    #[test]
    fn rules_are_not_high_risk() {
        assert!(rules().iter().all(|r| !matches!(r.risk, RiskLevel::High)))
    }
    #[test]
    fn unknown_id_is_rejected() {
        assert!(validated_path("../../Windows").is_err())
    }
}
