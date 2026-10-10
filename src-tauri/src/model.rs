use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

pub const STATE_SCHEMA_VERSION: u32 = 2;
pub const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: Some(detail.into()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VscodeStartup {
    #[default]
    Default,
    File {
        path: String,
    },
    GitGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub scan_depth: u8,
    pub default_editor_id: String,
    pub editor_profiles: BTreeMap<String, crate::editors::EditorProfileConfig>,
    pub default_vscode_startup: VscodeStartup,
    pub terminal: String,
    pub global_shortcut: Option<String>,
    pub close_to_tray: bool,
    pub data_location: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "system".into(),
            scan_depth: 4,
            default_editor_id: "vscode".into(),
            editor_profiles: BTreeMap::from([(
                "vscode".into(),
                crate::editors::EditorProfileConfig::default(),
            )]),
            default_vscode_startup: VscodeStartup::Default,
            terminal: "auto".into(),
            global_shortcut: Some("Ctrl+Alt+P".into()),
            close_to_tray: true,
            data_location: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeRoot {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserData {
    pub schema_version: u32,
    pub profile_id: String,
    pub roots: Vec<CodeRoot>,
    pub manual_projects: Vec<String>,
    pub favorites: BTreeSet<String>,
    pub recent: BTreeMap<String, u64>,
    pub category_overrides: BTreeMap<String, String>,
    pub editor_overrides: BTreeMap<String, String>,
    pub vscode_startup_overrides: BTreeMap<String, VscodeStartup>,
    pub settings: Settings,
}

impl Default for UserData {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            profile_id: uuid::Uuid::new_v4().to_string(),
            roots: Vec::new(),
            manual_projects: Vec::new(),
            favorites: BTreeSet::new(),
            recent: BTreeMap::new(),
            category_overrides: BTreeMap::new(),
            editor_overrides: BTreeMap::new(),
            vscode_startup_overrides: BTreeMap::new(),
            settings: Settings::default(),
        }
    }
}

impl UserData {
    pub fn editor_for(&self, id: &str) -> &str {
        self.editor_overrides
            .get(id)
            .map(String::as_str)
            .unwrap_or(&self.settings.default_editor_id)
    }
    pub fn vscode_startup_for(&self, id: &str) -> &VscodeStartup {
        self.vscode_startup_overrides
            .get(id)
            .filter(|startup| **startup != VscodeStartup::Default)
            .unwrap_or(&self.settings.default_vscode_startup)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    Available,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub tags: Vec<String>,
    pub is_git: bool,
    #[serde(default)]
    pub root_ids: BTreeSet<String>,
    #[serde(default)]
    pub manual: bool,
    pub availability: Availability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexCache {
    pub schema_version: u32,
    pub projects: BTreeMap<String, ProjectRecord>,
}

impl Default for IndexCache {
    fn default() -> Self {
        Self {
            schema_version: INDEX_SCHEMA_VERSION,
            projects: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(flatten)]
    pub record: ProjectRecord,
    pub category: Option<String>,
    pub category_override: bool,
    pub favorite: bool,
    pub last_opened_at: Option<u64>,
    pub vscode_startup: VscodeStartup,
    pub editor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    pub path: String,
    pub code: String,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub running: bool,
    pub visited: usize,
    pub discovered: usize,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub revision: u64,
    pub roots: Vec<CodeRoot>,
    pub projects: Vec<Project>,
    pub settings: Settings,
    pub scan: ScanStatus,
    pub warnings: Vec<AppError>,
    pub data_directory: String,
    pub storage_read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitMetadata {
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    pub repository_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum LaunchTarget {
    Vscode,
    Editor,
    Terminal,
    Explorer,
    Repository,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub snapshot: Option<AppSnapshot>,
    pub warnings: Vec<AppError>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_inherit_global_startup_unless_overridden() {
        let mut user = UserData::default();
        assert_eq!(
            user.vscode_startup_for("inherited"),
            &VscodeStartup::Default
        );
        user.settings.default_vscode_startup = VscodeStartup::GitGraph;
        user.vscode_startup_overrides.insert(
            "file".into(),
            VscodeStartup::File {
                path: "index.html".into(),
            },
        );
        user.vscode_startup_overrides
            .insert("legacy-default".into(), VscodeStartup::Default);
        assert_eq!(
            user.vscode_startup_for("inherited"),
            &VscodeStartup::GitGraph
        );
        assert_eq!(
            user.vscode_startup_for("legacy-default"),
            &VscodeStartup::GitGraph
        );
        assert_eq!(
            user.vscode_startup_for("file"),
            &VscodeStartup::File {
                path: "index.html".into()
            }
        );
        user.settings.default_vscode_startup = VscodeStartup::File {
            path: "src/main.ts".into(),
        };
        assert_eq!(
            user.vscode_startup_for("inherited"),
            &user.settings.default_vscode_startup
        );
        assert_eq!(
            user.vscode_startup_for("file"),
            &VscodeStartup::File {
                path: "index.html".into()
            }
        );
        user.vscode_startup_overrides.remove("file");
        assert_eq!(
            user.vscode_startup_for("file"),
            &user.settings.default_vscode_startup
        );
    }
}
