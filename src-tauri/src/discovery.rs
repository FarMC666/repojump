use crate::detectors::{detect, DirectoryEntries};
use crate::model::{CodeRoot, ProjectRecord, ScanIssue};
use std::path::Path;

const IGNORED: &[&str] = &[
    ".repojump",
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".cache",
    "venv",
    ".venv",
    "__pycache__",
    "vendor",
    "coverage",
    ".turbo",
    ".vs",
    ".pytest_cache",
    ".mypy_cache",
    "$recycle.bin",
    "system volume information",
];

pub enum ScanEvent {
    Visited,
    Project(ProjectRecord),
    Issue(ScanIssue),
}

// The callback returns false to cancel an obsolete scan. No filesystem writes occur here.
pub fn scan(root: &CodeRoot, max_depth: u8, mut emit: impl FnMut(ScanEvent) -> bool) -> bool {
    let mut stack = vec![(std::path::PathBuf::from(&root.path), 0_u8)];
    while let Some((path, depth)) = stack.pop() {
        if !emit(ScanEvent::Visited) {
            return false;
        }
        let entries = match DirectoryEntries::read(&path) {
            Ok(entries) => entries,
            Err(e) => {
                let issue = ScanIssue {
                    path: path.to_string_lossy().into_owned(),
                    code: if e.kind() == std::io::ErrorKind::NotFound {
                        "directoryMissing"
                    } else {
                        "directoryUnreadable"
                    }
                    .into(),
                };
                if !emit(ScanEvent::Issue(issue)) {
                    return false;
                }
                continue;
            }
        };
        for unreadable in &entries.unreadable_entries {
            if !emit(ScanEvent::Issue(ScanIssue {
                path: unreadable.to_string_lossy().into_owned(),
                code: "directoryUnreadable".into(),
            })) {
                return false;
            }
        }
        if let Some(mut project) = detect(&path, &entries, false) {
            project.root_ids.insert(root.id.clone());
            if !emit(ScanEvent::Project(project)) {
                return false;
            }
            continue;
        }
        if depth < max_depth {
            for child in entries.child_directories.into_iter().rev() {
                let name = child
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if !IGNORED.contains(&name.as_str()) {
                    stack.push((child, depth + 1));
                }
            }
        }
    }
    true
}

pub fn manual(path: &str) -> std::io::Result<ProjectRecord> {
    let path = Path::new(path);
    let entries = DirectoryEntries::read(path)?;
    Ok(detect(path, &entries, true).expect("forced detection always returns a project"))
}

pub fn is_project_directory(path: &str) -> std::io::Result<bool> {
    let path = Path::new(path);
    let entries = DirectoryEntries::read(path)?;
    Ok(detect(path, &entries, false).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn code_root_traverses_categories_and_finds_git_only_projects() {
        let temp = tempfile::tempdir().unwrap();
        for dir in [
            "apps/桌面项目/.git",
            "web/Test Project",
            "mods/Silent-Translator/.git",
        ] {
            fs::create_dir_all(temp.path().join(dir)).unwrap();
        }
        fs::write(temp.path().join("web/Test Project/package.json"), "{}").unwrap();
        let root = CodeRoot {
            id: "code".into(),
            path: temp.path().to_string_lossy().into_owned(),
        };
        let mut projects = Vec::new();
        assert!(scan(&root, 4, |event| {
            if let ScanEvent::Project(project) = event {
                projects.push(project);
            }
            true
        }));
        assert_eq!(projects.len(), 3);
        assert!(!is_project_directory(&root.path).unwrap());
        for category in ["apps", "web", "mods"] {
            assert!(!is_project_directory(&temp.path().join(category).to_string_lossy()).unwrap());
            assert_eq!(
                projects
                    .iter()
                    .filter(
                        |p| crate::paths::category(&p.path, std::slice::from_ref(&root)).as_deref()
                            == Some(category)
                    )
                    .count(),
                1
            );
        }
        let git_only = projects
            .iter()
            .find(|p| p.name == "Silent-Translator")
            .unwrap();
        assert!(git_only.is_git);
        assert!(git_only.tags.is_empty());
        assert!(is_project_directory(&git_only.path).unwrap());
        let mods = CodeRoot {
            id: "mods".into(),
            path: temp.path().join("mods").to_string_lossy().into_owned(),
        };
        let mut nested_ids = Vec::new();
        scan(&mods, 4, |event| {
            if let ScanEvent::Project(project) = event {
                nested_ids.push(project.id);
            }
            true
        });
        assert_eq!(nested_ids, vec![git_only.id.clone()]);
    }
    #[test]
    fn parent_stop_ignore_depth_and_manual_child() {
        let temp = tempfile::tempdir().unwrap();
        for dir in [
            "apps/parent/child",
            "node_modules/ignored",
            ".repojump/ignored",
            "a/b/c/deep",
        ] {
            fs::create_dir_all(temp.path().join(dir)).unwrap();
        }
        for file in [
            "apps/parent/package.json",
            "apps/parent/child/Cargo.toml",
            "node_modules/ignored/go.mod",
            ".repojump/ignored/go.mod",
            "a/b/c/deep/go.mod",
        ] {
            fs::write(temp.path().join(file), "{}").unwrap();
        }
        let root = CodeRoot {
            id: "r".into(),
            path: temp.path().to_string_lossy().into_owned(),
        };
        let mut projects = Vec::new();
        scan(&root, 3, |event| {
            if let ScanEvent::Project(p) = event {
                projects.push(p);
            }
            true
        });
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "parent");
        let child = manual(&temp.path().join("apps/parent/child").to_string_lossy()).unwrap();
        assert!(child.tags.contains(&"Rust".into()));
        assert!(child.manual);
    }
    #[test]
    fn root_detection_and_cancellation() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(".git"), "gitdir: elsewhere").unwrap();
        let root = CodeRoot {
            id: "r".into(),
            path: temp.path().to_string_lossy().into_owned(),
        };
        let mut count = 0;
        assert!(scan(&root, 4, |event| {
            if matches!(event, ScanEvent::Project(_)) {
                count += 1;
            }
            true
        }));
        assert_eq!(count, 1);
        assert!(!scan(&root, 4, |_| false));
    }
}
