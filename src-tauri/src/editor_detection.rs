use crate::{
    editors::{EditorDefinition, EditorProfileConfig},
    model::*,
};
use std::{
    env,
    path::{Path, PathBuf},
};

fn valid(path: &Path, d: &EditorDefinition) -> bool {
    path.is_absolute()
        && path.is_file()
        && path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(d.executable))
}

pub fn from_cli(path: &Path, d: &EditorDefinition) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd"))
    {
        let bin = path.parent()?;
        let mut folders = vec![bin.parent()?];
        // Cursor ships its CLI in resources/app/bin; never execute or parse the shim.
        if bin
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("bin"))
            && bin
                .parent()?
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("app"))
            && bin
                .parent()?
                .parent()?
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("resources"))
        {
            folders.push(bin.parent()?.parent()?.parent()?);
        }
        folders
            .into_iter()
            .map(|folder| folder.join(d.executable))
            .find(|candidate| valid(candidate, d))
    } else {
        valid(path, d).then(|| path.to_path_buf())
    }
}

fn installation_candidates(
    d: &EditorDefinition,
    name: &str,
    location: Option<&str>,
    icon: Option<&str>,
) -> Vec<PathBuf> {
    let name = name.trim().to_ascii_lowercase();
    let name = name
        .strip_suffix(" (user)")
        .or_else(|| name.strip_suffix(" (system)"))
        .unwrap_or(&name);
    let product_names: &[&str] = match d.id {
        "vscode" => &["microsoft visual studio code", "visual studio code"],
        "vscode-insiders" => &[
            "microsoft visual studio code insiders",
            "microsoft visual studio code - insiders",
            "visual studio code insiders",
            "visual studio code - insiders",
        ],
        "cursor" => &["cursor"],
        "windsurf" => &["windsurf"],
        _ => &[],
    };
    if !product_names.contains(&name) {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    if let Some(icon) = icon {
        let icon = icon.trim();
        let path = if let Some(quoted) = icon.strip_prefix('"') {
            quoted.split_once('"').map(|(path, _)| path)
        } else {
            Some(
                icon.rsplit_once(',')
                    .filter(|(_, index)| index.trim().parse::<i32>().is_ok())
                    .map_or(icon, |(path, _)| path)
                    .trim(),
            )
        };
        if let Some(path) = path {
            candidates.push(PathBuf::from(path));
        }
    }
    if let Some(location) = location {
        candidates.push(PathBuf::from(location.trim().trim_matches('"')).join(d.executable));
    }
    candidates
}

#[cfg(windows)]
fn from_registry(d: &EditorDefinition) -> Option<PathBuf> {
    use winreg::{
        enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        },
        RegKey,
    };
    let views = [KEY_WOW64_64KEY, KEY_WOW64_32KEY];
    let hives = [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE];
    for hive in hives {
        for view in views {
            if let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(
                format!(
                    "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{}",
                    d.executable
                ),
                KEY_READ | view,
            ) {
                if let Ok(value) = key.get_value::<String, _>("") {
                    let path = PathBuf::from(value.trim().trim_matches('"'));
                    if valid(&path, d) {
                        return Some(path);
                    }
                }
            }
        }
    }
    for hive in hives {
        for view in views {
            let Ok(products) = RegKey::predef(hive).open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
                KEY_READ | view,
            ) else {
                continue;
            };
            for name in products.enum_keys().flatten() {
                let Ok(key) = products.open_subkey_with_flags(name, KEY_READ) else {
                    continue;
                };
                let Ok(name) = key.get_value::<String, _>("DisplayName") else {
                    continue;
                };
                let location = key.get_value::<String, _>("InstallLocation").ok();
                let icon = key.get_value::<String, _>("DisplayIcon").ok();
                for candidate in
                    installation_candidates(d, &name, location.as_deref(), icon.as_deref())
                {
                    if valid(&candidate, d) {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

pub fn candidates(
    d: &EditorDefinition,
    path: Option<&std::ffi::OsStr>,
    bases: &[(PathBuf, bool)],
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = path {
        for folder in env::split_paths(path) {
            for name in [
                d.executable.to_string(),
                format!("{}.exe", d.cli),
                format!("{}.cmd", d.cli),
            ] {
                candidates.push(folder.join(name));
            }
        }
    }
    for (base, local) in bases {
        for folder in d.folders {
            let base = if *local {
                base.join("Programs")
            } else {
                base.clone()
            };
            candidates.push(base.join(folder).join(d.executable));
        }
    }
    candidates
}

pub fn resolve(d: &EditorDefinition, profile: Option<&EditorProfileConfig>) -> AppResult<PathBuf> {
    if let Some(path) = profile.and_then(|p| p.executable_path.as_deref()) {
        let path = PathBuf::from(path);
        return if valid(&path, d) {
            Ok(path)
        } else {
            Err(AppError::new("editorInvalid", path.to_string_lossy()))
        };
    }
    let bases: Vec<_> = [
        ("LOCALAPPDATA", true),
        ("ProgramFiles", false),
        ("ProgramFiles(x86)", false),
    ]
    .into_iter()
    .filter_map(|(key, local)| env::var_os(key).map(|p| (PathBuf::from(p), local)))
    .collect();
    let path = env::var_os("PATH");
    let candidates = candidates(d, path.as_deref(), &[]);
    for candidate in candidates {
        if let Some(executable) = from_cli(&candidate, d) {
            return Ok(executable);
        }
    }
    #[cfg(windows)]
    if let Some(path) = from_registry(d) {
        return Ok(path);
    }
    for candidate in candidates_from_bases(d, &bases) {
        if valid(&candidate, d) {
            return Ok(candidate);
        }
    }
    Err(AppError::new("editorNotFound", d.display_name))
}

fn candidates_from_bases(d: &EditorDefinition, bases: &[(PathBuf, bool)]) -> Vec<PathBuf> {
    candidates(d, None, bases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editors::REGISTRY;
    #[test]
    fn each_cli_maps_only_to_its_own_executable_without_running_shims() {
        let temp = tempfile::tempdir().unwrap();
        for d in REGISTRY {
            let folder = temp.path().join(d.id);
            std::fs::create_dir_all(folder.join("bin")).unwrap();
            std::fs::write(folder.join(d.executable), "").unwrap();
            let shim = folder.join("bin").join(format!("{}.cmd", d.cli));
            std::fs::write(&shim, "not executed").unwrap();
            assert_eq!(from_cli(&shim, d), Some(folder.join(d.executable)));
            let profile = EditorProfileConfig {
                executable_path: Some(folder.join(d.executable).to_string_lossy().into()),
            };
            assert_eq!(
                resolve(d, Some(&profile)).unwrap(),
                folder.join(d.executable)
            );
        }
    }
    #[test]
    fn common_candidates_include_user_and_system_installs() {
        for d in REGISTRY {
            let candidates = candidates(
                d,
                None,
                &[
                    (PathBuf::from("D:/user"), true),
                    (PathBuf::from("D:/system"), false),
                ],
            );
            assert!(candidates.iter().any(|p| p
                == &PathBuf::from("D:/user")
                    .join("Programs")
                    .join(d.folders[0])
                    .join(d.executable)));
            assert!(candidates.iter().any(|p| p
                == &PathBuf::from("D:/system")
                    .join(d.folders[0])
                    .join(d.executable)));
        }
    }

    #[test]
    fn nested_cursor_cli_resolves_the_installation_root_without_executing_it() {
        let temp = tempfile::tempdir().unwrap();
        let d = crate::editors::definition("cursor").unwrap();
        let root = temp.path().join("Cursor 代码 & (app); $");
        let bin = root.join("resources/app/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(root.join(d.executable), "").unwrap();
        let shim = bin.join("cursor.cmd");
        std::fs::write(&shim, "this must never execute").unwrap();
        assert_eq!(from_cli(&shim, d), Some(root.join(d.executable)));
        assert!(from_cli(&shim, crate::editors::definition("vscode").unwrap()).is_none());
        let unrelated = root.join("other/app/bin");
        std::fs::create_dir_all(&unrelated).unwrap();
        let other = unrelated.join("cursor.cmd");
        std::fs::write(&other, "").unwrap();
        assert!(from_cli(&other, d).is_none());
    }

    #[test]
    fn installation_records_resolve_custom_paths_and_literal_icon_indices() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("编辑器 & (apps), 1; $");
        std::fs::create_dir(&root).unwrap();
        for d in REGISTRY {
            let executable = root.join(d.executable);
            std::fs::write(&executable, "").unwrap();
            let name = format!("{} (User)", d.display_name);
            for icon in [
                format!("\"{}\",0", executable.display()),
                format!("{},-1", executable.display()),
            ] {
                let candidates = installation_candidates(d, &name, None, Some(&icon));
                assert_eq!(
                    candidates.into_iter().find(|path| valid(path, d)),
                    Some(executable.clone())
                );
            }
            let candidates = installation_candidates(d, &name, Some(root.to_str().unwrap()), None);
            assert_eq!(
                candidates.into_iter().find(|path| valid(path, d)),
                Some(executable)
            );
        }
    }

    #[test]
    fn unrelated_installer_names_and_icon_commands_are_not_editor_executables() {
        let d = crate::editors::definition("cursor").unwrap();
        assert!(
            installation_candidates(d, "Cursor extension", Some("D:/apps/cursor"), None).is_empty()
        );
        let paths = installation_candidates(
            d,
            "Cursor",
            None,
            Some("D:/apps/Cursor.exe --execute something"),
        );
        assert!(paths.iter().all(|path| !valid(path, d)));
    }
}
