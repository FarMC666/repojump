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

/// Resolve a saved project-relative file, rejecting traversal and Windows drive-relative paths.
pub fn project_file(project: &Path, relative: &str) -> AppResult<PathBuf> {
    let invalid = || AppError::new("startupFileInvalid", relative);
    let relative = relative.trim();
    if relative.is_empty()
        || relative.contains([':', '\0'])
        || relative.starts_with(['/', '\\'])
        || relative.split(['/', '\\']).any(|part| part == "..")
    {
        return Err(invalid());
    }
    let candidate = project.join(relative.replace('\\', "/"));
    let root = dunce::canonicalize(project).map_err(|_| invalid())?;
    let file = dunce::canonicalize(candidate).map_err(|_| invalid())?;
    if !file.is_file() || !file.starts_with(&root) {
        return Err(invalid());
    }
    Ok(file)
}

pub fn relative_project_file(project: &Path, file: &Path) -> AppResult<String> {
    let invalid = || AppError::new("startupFileInvalid", file.display().to_string());
    let root = dunce::canonicalize(project).map_err(|_| invalid())?;
    let file = dunce::canonicalize(file).map_err(|_| invalid())?;
    if !file.is_file() {
        return Err(invalid());
    }
    file.strip_prefix(root)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .map_err(|_| invalid())
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
    fn startup_files_are_existing_files_inside_the_project() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/页面 & (test); $.html"), "hello").unwrap();
        std::fs::write(temp.path().join("outside.html"), "outside").unwrap();
        let relative = "src/页面 & (test); $.html";
        let file = project_file(&root, relative).unwrap();
        assert_eq!(relative_project_file(&root, &file).unwrap(), relative);
        assert!(project_file(&root, "src\\页面 & (test); $.html").is_ok());
        for invalid in [
            "",
            "src",
            "missing.html",
            "../outside.html",
            "src/../../outside.html",
            "C:outside.html",
            "C:\\outside.html",
            "/outside.html",
            "\\\\host\\share\\file",
        ] {
            assert!(project_file(&root, invalid).is_err(), "{invalid}");
        }
        assert!(relative_project_file(&root, &temp.path().join("outside.html")).is_err());
        std::fs::remove_file(&file).unwrap();
        assert!(project_file(&root, relative).is_err());
    }
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
