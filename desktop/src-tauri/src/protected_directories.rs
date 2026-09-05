use std::path::{Path, PathBuf};

/// Resolve protected user-data directories from the current profile instead of
/// embedding machine-specific paths in the frontend bundle.
pub fn discover() -> Vec<String> {
    let mut candidates = vec![dirs::desktop_dir(), dirs::document_dir()];

    #[cfg(windows)]
    for variable in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        candidates.push(std::env::var_os(variable).map(PathBuf::from));
    }

    unique_absolute_paths(candidates.into_iter().flatten())
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|path| path.display().to_string())
        .collect()
}

fn unique_absolute_paths(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    let mut keys = Vec::new();

    for path in paths {
        if !path.is_absolute() || !has_non_root_component(&path) {
            continue;
        }
        let key = path_key(&path);
        if keys.iter().any(|existing| existing == &key) {
            continue;
        }
        keys.push(key);
        unique.push(path);
    }
    unique
}

fn has_non_root_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::Normal(_)))
}

fn path_key(path: &Path) -> String {
    let normalized = path
        .as_os_str()
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_string();

    #[cfg(windows)]
    {
        normalized.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_unique_absolute_non_root_paths() {
        let root = std::env::temp_dir();
        let desktop = root.join("profile").join("Desktop");
        let documents = root.join("profile").join("Documents");
        let paths = unique_absolute_paths([
            desktop.clone(),
            desktop.clone(),
            documents.clone(),
            PathBuf::from("relative"),
        ]);

        assert_eq!(paths, [desktop, documents]);
    }
}
