use crate::{editor_detection, launcher, model::*};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EditorProfileConfig {
    pub executable_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCapabilities {
    pub project_open: bool,
    pub specific_file: bool,
    pub startup_helper: bool,
    pub git_graph: bool,
}

pub struct EditorDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub executable: &'static str,
    pub cli: &'static str,
    pub folders: &'static [&'static str],
    pub capabilities: EditorCapabilities,
}

const BASIC: EditorCapabilities = EditorCapabilities {
    project_open: true,
    specific_file: false,
    startup_helper: false,
    git_graph: false,
};
pub const REGISTRY: &[EditorDefinition] = &[
    EditorDefinition {
        id: "vscode",
        display_name: "Visual Studio Code",
        executable: "Code.exe",
        cli: "code",
        folders: &["Microsoft VS Code"],
        capabilities: EditorCapabilities {
            specific_file: true,
            startup_helper: true,
            git_graph: true,
            ..BASIC
        },
    },
    EditorDefinition {
        id: "vscode-insiders",
        display_name: "Visual Studio Code Insiders",
        executable: "Code - Insiders.exe",
        cli: "code-insiders",
        folders: &["Microsoft VS Code Insiders"],
        capabilities: EditorCapabilities {
            specific_file: true,
            ..BASIC
        },
    },
    EditorDefinition {
        id: "cursor",
        display_name: "Cursor",
        executable: "Cursor.exe",
        cli: "cursor",
        folders: &["Cursor", "cursor"],
        capabilities: BASIC,
    },
    EditorDefinition {
        id: "windsurf",
        display_name: "Windsurf",
        executable: "Windsurf.exe",
        cli: "windsurf",
        folders: &["Windsurf", "windsurf"],
        capabilities: BASIC,
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorStatus {
    pub id: String,
    pub display_name: String,
    pub configured: bool,
    pub executable_path: Option<String>,
    pub available: bool,
    pub capabilities: EditorCapabilities,
    pub error: Option<AppError>,
}

pub fn definition(id: &str) -> AppResult<&'static EditorDefinition> {
    REGISTRY
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AppError::new("editorUnsupported", id))
}

pub fn statuses(settings: &Settings) -> Vec<EditorStatus> {
    let mut statuses: Vec<_> = REGISTRY
        .iter()
        .map(|d| {
            let configured = settings.editor_profiles.contains_key(d.id);
            let result = editor_detection::resolve(d, settings.editor_profiles.get(d.id));
            EditorStatus {
                id: d.id.into(),
                display_name: d.display_name.into(),
                configured,
                executable_path: result.as_ref().ok().map(|p| p.to_string_lossy().into()),
                available: result.is_ok(),
                capabilities: d.capabilities,
                error: result.err(),
            }
        })
        .collect();
    for id in settings
        .editor_profiles
        .keys()
        .filter(|id| !REGISTRY.iter().any(|d| d.id == id.as_str()))
    {
        statuses.push(EditorStatus {
            id: id.clone(),
            display_name: id.clone(),
            configured: true,
            executable_path: None,
            available: false,
            capabilities: EditorCapabilities {
                project_open: false,
                ..BASIC
            },
            error: Some(AppError::new("editorUnsupported", id)),
        });
    }
    statuses
}

pub fn validate_changes(next: &Settings, previous: &Settings) -> AppResult<()> {
    if next.default_editor_id != previous.default_editor_id {
        definition(&next.default_editor_id)?;
    }
    if !next.editor_profiles.contains_key(&next.default_editor_id) {
        return Err(AppError::new(
            "editorNotConfigured",
            &next.default_editor_id,
        ));
    }
    for (id, profile) in &next.editor_profiles {
        if previous.editor_profiles.get(id) == Some(profile) {
            continue;
        }
        let d = definition(id)?;
        if profile.executable_path.is_some() {
            editor_detection::resolve(d, Some(profile))?;
        }
    }
    Ok(())
}

pub fn command(executable: &Path, project: &Path) -> Command {
    // Profiles cannot provide arguments or shell strings.
    launcher::vscode_command(executable, project, None)
}

pub fn open(
    app: &tauri::AppHandle,
    data: &Path,
    project: &Path,
    settings: &Settings,
    editor_id: &str,
    startup: &VscodeStartup,
) -> AppResult<Vec<AppError>> {
    let d = definition(editor_id)?;
    let profile = settings
        .editor_profiles
        .get(editor_id)
        .ok_or_else(|| AppError::new("editorNotConfigured", editor_id))?;
    let executable = editor_detection::resolve(d, Some(profile))?;
    let supported = match startup {
        VscodeStartup::Default => true,
        VscodeStartup::File { .. } => d.capabilities.specific_file,
        VscodeStartup::GitGraph => d.capabilities.startup_helper && d.capabilities.git_graph,
    };
    if !supported {
        launcher::spawn(command(&executable, project))?;
        return Ok(vec![AppError::new(
            "editorStartupUnsupported",
            d.display_name,
        )]);
    }
    crate::vscode_startup::open(app, data, project, &executable, startup)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_editors_keep_literal_arguments_and_capabilities_are_conservative() {
        for d in REGISTRY {
            let command = command(Path::new(d.executable), Path::new(r"D:\代码\a & (b); $c"));
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [
                    std::ffi::OsStr::new("--new-window"),
                    std::ffi::OsStr::new(r"D:\代码\a & (b); $c")
                ]
            );
            assert_eq!(d.capabilities.git_graph, d.id == "vscode");
        }
    }
    #[test]
    fn stale_executable_does_not_block_unrelated_settings_and_new_invalid_path_is_rejected() {
        let mut previous = Settings::default();
        previous
            .editor_profiles
            .get_mut("vscode")
            .unwrap()
            .executable_path = Some("Z:/missing/Code.exe".into());
        let mut next = previous.clone();
        next.theme = "light".into();
        validate_changes(&next, &previous).unwrap();
        next.editor_profiles
            .get_mut("vscode")
            .unwrap()
            .executable_path = Some("Z:/other/Code.exe".into());
        assert_eq!(
            validate_changes(&next, &previous).unwrap_err().code,
            "editorInvalid"
        );
    }
}
