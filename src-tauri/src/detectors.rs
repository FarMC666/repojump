use crate::model::{Availability, ProjectRecord};
use crate::paths;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Read;
use std::path::Path;

pub struct DirectoryEntries {
    pub names: HashSet<String>,
    pub child_directories: Vec<std::path::PathBuf>,
    pub unreadable_entries: Vec<std::path::PathBuf>,
}

impl DirectoryEntries {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let mut names = HashSet::new();
        let mut child_directories = Vec::new();
        let mut unreadable_entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    unreadable_entries.push(path.to_path_buf());
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(_) => {
                    unreadable_entries.push(entry.path());
                    continue;
                }
            };
            if metadata.is_file() || (name == ".git" && metadata.is_dir()) {
                names.insert(name);
            }
            if metadata.is_dir() && !paths::is_reparse(&metadata) {
                child_directories.push(entry.path());
            }
        }
        child_directories.sort();
        Ok(Self {
            names,
            child_directories,
            unreadable_entries,
        })
    }
    fn has(&self, name: &str) -> bool {
        self.names.contains(name)
    }
    fn extension(&self, extension: &str) -> bool {
        self.names.iter().any(|n| n.ends_with(extension))
    }
    fn config(&self, prefix: &str) -> bool {
        self.names.iter().any(|n| n.starts_with(prefix))
    }
}

struct ProjectDetector {
    markers: &'static [&'static str],
    extensions: &'static [&'static str],
    tags: &'static [&'static str],
}

const DETECTORS: &[ProjectDetector] = &[
    ProjectDetector {
        markers: &[".git"],
        extensions: &[],
        tags: &[],
    },
    ProjectDetector {
        markers: &[
            "package.json",
            "pnpm-workspace.yaml",
            "yarn.lock",
            "package-lock.json",
        ],
        extensions: &[],
        tags: &["JavaScript"],
    },
    ProjectDetector {
        markers: &["pyproject.toml", "requirements.txt"],
        extensions: &[],
        tags: &["Python"],
    },
    ProjectDetector {
        markers: &["Cargo.toml"],
        extensions: &[],
        tags: &["Rust"],
    },
    ProjectDetector {
        markers: &["go.mod"],
        extensions: &[],
        tags: &["Go"],
    },
    ProjectDetector {
        markers: &[],
        extensions: &[".sln", ".slnx", ".csproj"],
        tags: &[".NET"],
    },
    ProjectDetector {
        markers: &["pom.xml"],
        extensions: &[],
        tags: &["Java", "Maven"],
    },
    ProjectDetector {
        markers: &["build.gradle", "build.gradle.kts"],
        extensions: &[],
        tags: &["Java", "Gradle"],
    },
    ProjectDetector {
        markers: &["composer.json"],
        extensions: &[],
        tags: &["PHP", "Composer"],
    },
    ProjectDetector {
        markers: &["Gemfile"],
        extensions: &[],
        tags: &["Ruby"],
    },
    ProjectDetector {
        markers: &[],
        extensions: &[".code-workspace"],
        tags: &["VS Code Workspace"],
    },
];

fn package(path: &Path) -> Option<Value> {
    let path = path.join("package.json");
    let metadata = fs::symlink_metadata(&path).ok()?;
    if paths::is_reparse(&metadata) || metadata.len() > 1_048_576 {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn detect(path: &Path, entries: &DirectoryEntries, force: bool) -> Option<ProjectRecord> {
    let mut found = force;
    let mut tags = BTreeSet::new();
    for detector in DETECTORS {
        if detector
            .markers
            .iter()
            .any(|m| entries.has(&m.to_lowercase()))
            || detector.extensions.iter().any(|e| entries.extension(e))
        {
            found = true;
            tags.extend(detector.tags.iter().map(|t| t.to_string()));
        }
    }
    if !found {
        return None;
    }
    let package = package(path);
    let dependency = |name: &str| {
        package.as_ref().is_some_and(|p| {
            [
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ]
            .iter()
            .any(|section| p.get(section).and_then(|v| v.get(name)).is_some())
        })
    };
    if entries.config("tsconfig.") || dependency("typescript") {
        tags.insert("TypeScript".into());
    }
    if dependency("react") {
        tags.insert("React".into());
    }
    if dependency("next") || (entries.has("package.json") && entries.config("next.config.")) {
        tags.insert("Next.js".into());
    }
    if dependency("vite") || (entries.has("package.json") && entries.config("vite.config.")) {
        tags.insert("Vite".into());
    }
    let display = path.to_string_lossy().into_owned();
    Some(ProjectRecord {
        id: paths::identity(&display),
        name: path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned(),
        path: display,
        tags: tags.into_iter().collect(),
        is_git: entries.has(".git"),
        root_ids: BTreeSet::new(),
        manual: force,
        availability: Availability::Available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_required_markers() {
        for marker in [
            ".git",
            "package.json",
            "pnpm-workspace.yaml",
            "yarn.lock",
            "package-lock.json",
            "pyproject.toml",
            "requirements.txt",
            "Cargo.toml",
            "go.mod",
            "test.sln",
            "test.csproj",
            "pom.xml",
            "build.gradle",
            "composer.json",
            "Gemfile",
            "test.code-workspace",
        ] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(temp.path().join(marker), "").unwrap();
            assert!(
                detect(
                    temp.path(),
                    &DirectoryEntries::read(temp.path()).unwrap(),
                    false
                )
                .is_some(),
                "{marker}"
            );
        }
    }
    #[test]
    fn multiple_tags_and_malformed_package() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies":{"react":"*","vite":"*"},"devDependencies":{"typescript":"*"}}"#,
        )
        .unwrap();
        let entries = DirectoryEntries::read(temp.path()).unwrap();
        let project = detect(temp.path(), &entries, false).unwrap();
        assert!(project.tags.contains(&"React".into()));
        assert!(project.tags.contains(&"Vite".into()));
        assert!(project.tags.contains(&"TypeScript".into()));
        fs::write(temp.path().join("package.json"), "broken").unwrap();
        assert!(detect(temp.path(), &entries, false).is_some());
    }
    #[test]
    fn marker_named_folders_are_not_projects_but_git_directories_are() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("package.json")).unwrap();
        fs::create_dir(temp.path().join("fake.sln")).unwrap();
        assert!(detect(
            temp.path(),
            &DirectoryEntries::read(temp.path()).unwrap(),
            false
        )
        .is_none());
        fs::create_dir(temp.path().join(".git")).unwrap();
        assert!(
            detect(
                temp.path(),
                &DirectoryEntries::read(temp.path()).unwrap(),
                false
            )
            .unwrap()
            .is_git
        );
    }
}
