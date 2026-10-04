use crate::model::{AppError, AppResult, CodeRoot};
use std::path::{Path, PathBuf};

pub fn identity(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

pub fn directory(path: &str) -> AppResult<PathBuf> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(AppError::new("invalidPath", path.display().to_string()));
    }
    let canonical = dunce::canonicalize(path)
        .map_err(|e| AppError::new("directoryUnavailable", format!("{}: {e}", path.display())))?;
    if !canonical.is_dir() {
        return Err(AppError::new("invalidPath", path.display().to_string()));
    }
    Ok(canonical)
}

pub fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub fn category(path: &str, roots: &[CodeRoot]) -> Option<String> {
    let key = identity(path);
    let root = roots
        .iter()
        .filter(|r| {
            let base = identity(&r.path);
            key == base || key.starts_with(&(base + "\\"))
        })
        .max_by_key(|r| identity(&r.path).len())?;
    if key == identity(&root.path) {
        return None;
    }
    // Count components instead of byte offsets: Unicode case-folding may change byte length.
    let root_display = root.path.replace('/', "\\");
    let root_components = root_display.split('\\').filter(|p| !p.is_empty()).count();
    let display = path.replace('/', "\\");
    let parts: Vec<_> = display
        .split('\\')
        .filter(|p| !p.is_empty())
        .skip(root_components)
        .collect();
    (parts.len() > 1).then(|| parts[0].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_identity_and_category() {
        assert_eq!(identity("D:/My Code/测试/"), identity("d:\\My Code\\测试"));
        let roots = vec![CodeRoot {
            id: "r".into(),
            path: "D:\\code".into(),
        }];
        assert_eq!(
            category("D:\\code\\apps\\测试项目", &roots),
            Some("apps".into())
        );
        assert_eq!(category("D:\\code\\project", &roots), None);
        assert_eq!(category("D:\\code2\\apps\\project", &roots), None);
        let mut nested = roots;
        nested.push(CodeRoot {
            id: "r2".into(),
            path: "D:\\code\\apps".into(),
        });
        assert_eq!(category("D:\\code\\apps\\project", &nested), None);
    }
}
