use crate::model::{
    AppError, AppResult, IndexCache, UserData, INDEX_SCHEMA_VERSION, STATE_SCHEMA_VERSION,
};
use atomic_write_file::AtomicWriteFile;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct Storage {
    pub directory: PathBuf,
    pub read_only: bool,
}

pub(crate) fn decode_user(bytes: &[u8]) -> AppResult<UserData> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| AppError::new("storageCorrupt", e.to_string()))?;
    if !value.is_object() {
        return Err(AppError::new("storageCorrupt", "Expected an object"));
    }
    let version = value
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if version > u64::from(STATE_SCHEMA_VERSION) {
        return Err(AppError::new("storageNewerVersion", version.to_string()));
    }
    // Version zero used the same fields without a version. Preserve them and fill defaults.

    if version < 2 {
        if value.get("settings").is_none() {
            value["settings"] = serde_json::json!({});
        }
        if let Some(settings) = value["settings"].as_object_mut() {
            let legacy = settings
                .remove("vscodePath")
                .unwrap_or(serde_json::Value::Null);
            settings
                .entry("defaultEditorId")
                .or_insert_with(|| "vscode".into());
            settings
                .entry("editorProfiles")
                .or_insert_with(|| serde_json::json!({"vscode": {"executablePath": legacy}}));
        }
    }
    value["schemaVersion"] = STATE_SCHEMA_VERSION.into();
    serde_json::from_value(value).map_err(|e| AppError::new("storageCorrupt", e.to_string()))
}

pub(crate) fn atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    // Windows can redirect individual existing files even when their parent
    // directory resolves to the original volume. Create the temporary sibling
    // beside the actual destination so replacement stays on the same volume.
    let destination = match dunce::canonicalize(path) {
        Ok(actual) => actual,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(AppError::new("storageFailure", error.to_string())),
    };
    let mut file = AtomicWriteFile::open(&destination)
        .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    file.write_all(bytes)
        .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    file.commit()
        .map_err(|e| AppError::new("storageFailure", e.to_string()))
}

pub(crate) fn preserve_migration_checkpoint(directory: &Path, previous: &[u8]) -> AppResult<()> {
    let value: serde_json::Value = serde_json::from_slice(previous)
        .map_err(|e| AppError::new("storageCorrupt", e.to_string()))?;
    let version = value
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let path = directory.join("state.pre-v2.json");
    if version < 2 && !path.exists() {
        atomic(&path, previous)?;
    }
    Ok(())
}

impl Storage {
    pub fn load(directory: PathBuf) -> (Self, UserData, IndexCache, Vec<AppError>) {
        let mut warnings = Vec::new();
        let mut read_only = false;
        if let Err(e) = fs::create_dir_all(&directory) {
            warnings.push(AppError::new("storageFailure", e.to_string()));
            read_only = true;
        }
        // Resolve junctions and Windows packaged-process path redirection before making
        // temporary files. A logical destination can otherwise refer to a different
        // volume than the newly created file, breaking an atomic rename.
        let directory = match dunce::canonicalize(&directory) {
            Ok(actual) => actual,
            Err(e) => {
                if !read_only {
                    warnings.push(AppError::new("storageFailure", e.to_string()));
                }
                read_only = true;
                directory
            }
        };
        let primary = directory.join("state.json");
        let user = match fs::read(&primary) {
            Ok(bytes) => match decode_user(&bytes) {
                Ok(user) => user,
                Err(e) if e.code == "storageNewerVersion" => {
                    warnings.push(e);
                    read_only = true;
                    UserData::default()
                }
                Err(_) => {
                    let recovered = fs::read(directory.join("state.json.bak"))
                        .ok()
                        .and_then(|b| decode_user(&b).ok());
                    let quarantine =
                        directory.join(format!("state.corrupt.{}.json", crate::model::now_ms()));
                    if let Err(e) = fs::rename(&primary, &quarantine) {
                        warnings.push(AppError::new("storageFailure", e.to_string()));
                        read_only = true;
                    }
                    warnings.push(AppError::new(
                        if recovered.is_some() {
                            "storageRecovered"
                        } else {
                            "storageReset"
                        },
                        quarantine.display().to_string(),
                    ));
                    recovered.unwrap_or_default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // A valid backup can also recover an interrupted first replacement.
                fs::read(directory.join("state.json.bak"))
                    .ok()
                    .and_then(|b| decode_user(&b).ok())
                    .unwrap_or_default()
            }
            Err(e) => {
                warnings.push(AppError::new("storageFailure", e.to_string()));
                read_only = true;
                UserData::default()
            }
        };
        if !read_only {
            if let Some(previous) = fs::read(&primary)
                .or_else(|_| fs::read(directory.join("state.json.bak")))
                .ok()
                .filter(|bytes| decode_user(bytes).is_ok())
            {
                if let Err(error) = preserve_migration_checkpoint(&directory, &previous) {
                    warnings.push(error);
                    read_only = true;
                }
            }
        }
        let cache = fs::read(directory.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<IndexCache>(&b).ok())
            .filter(|c| c.schema_version == INDEX_SCHEMA_VERSION)
            .unwrap_or_default();
        (
            Self {
                directory,
                read_only,
            },
            user,
            cache,
            warnings,
        )
    }

    pub fn save_user(&self, user: &UserData) -> AppResult<()> {
        if self.read_only {
            return Err(AppError::new(
                "storageReadOnly",
                self.directory.display().to_string(),
            ));
        }
        let path = self.directory.join("state.json");
        if let Ok(previous) = fs::read(&path) {
            // Never replace a valid backup with a corrupt primary.
            let decoded = decode_user(&previous)?;
            let has_profile = serde_json::from_slice::<serde_json::Value>(&previous)
                .ok()
                .is_some_and(|value| value.get("profileId").is_some());
            if has_profile && decoded.profile_id != user.profile_id {
                return Err(AppError::new(
                    "storageLocationConflict",
                    path.display().to_string(),
                ));
            }
            preserve_migration_checkpoint(&self.directory, &previous)?;
            atomic(&self.directory.join("state.json.bak"), &previous)?;
        }
        let bytes = serde_json::to_vec_pretty(user)
            .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
        atomic(&path, &bytes)
    }

    pub fn save_cache(&self, cache: &IndexCache) -> AppResult<()> {
        if self.read_only {
            return Ok(());
        }
        let bytes =
            serde_json::to_vec(cache).map_err(|e| AppError::new("cacheFailure", e.to_string()))?;
        atomic(&self.directory.join("index.json"), &bytes)
            .map_err(|error| AppError::new("cacheFailure", error.detail.unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v1_editor_migration_preserves_every_preference_and_original_checkpoint() {
        let original = serde_json::json!({
            "schemaVersion": 1, "profileId": "released", "roots": [{"id":"r", "path":"D:/代码"}],
            "manualProjects":["D:/manual"], "favorites":["p"], "recent":{"p":123}, "categoryOverrides":{"p":"apps"},
            "vscodeStartupOverrides":{"p":{"kind":"gitGraph"}},
            "settings":{"vscodePath":"D:/My Editor/Code.exe", "defaultVscodeStartup":{"kind":"file","path":"首页.html"},
                "theme":"light", "language":"zh-CN", "scanDepth":3, "terminal":"powershell", "globalShortcut":"Ctrl+Alt+J", "closeToTray":false, "dataLocation":"D:/custom"}
        });
        let bytes = serde_json::to_vec(&original).unwrap();
        let mut migrated = decode_user(&bytes).unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.settings.default_editor_id, "vscode");
        assert_eq!(
            migrated.settings.editor_profiles["vscode"]
                .executable_path
                .as_deref(),
            Some("D:/My Editor/Code.exe")
        );
        assert!(migrated.editor_overrides.is_empty());
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("state.json"), &bytes).unwrap();
        let (store, _, _, _) = Storage::load(temp.path().into());
        store.save_user(&migrated).unwrap();
        migrated
            .editor_overrides
            .insert("p".into(), "cursor".into());
        store.save_user(&migrated).unwrap();
        let (_, restored, _, warnings) = Storage::load(temp.path().into());
        assert!(warnings.is_empty());
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(migrated).unwrap()
        );
        assert_eq!(
            fs::read(temp.path().join("state.pre-v2.json")).unwrap(),
            bytes
        );
        let saved = serde_json::from_slice::<serde_json::Value>(
            &fs::read(temp.path().join("state.json")).unwrap(),
        )
        .unwrap();
        for key in [
            "roots",
            "manualProjects",
            "favorites",
            "recent",
            "categoryOverrides",
            "vscodeStartupOverrides",
        ] {
            assert_eq!(saved[key], original[key], "{key}");
        }
        for key in [
            "theme",
            "language",
            "scanDepth",
            "terminal",
            "globalShortcut",
            "closeToTray",
            "dataLocation",
            "defaultVscodeStartup",
        ] {
            assert_eq!(saved["settings"][key], original["settings"][key], "{key}");
        }
        let unknown = decode_user(br#"{"schemaVersion":2,"settings":{"defaultEditorId":"future","editorProfiles":{"future":{"executablePath":"X:/future.exe"}}},"editorOverrides":{"p":"future"}}"#).unwrap();
        assert_eq!(unknown.editor_for("p"), "future");
        store.save_cache(&IndexCache::default()).unwrap();
        let (_, _, cache, warnings) = Storage::load(temp.path().into());
        assert!(warnings.is_empty());
        assert_eq!(cache.schema_version, 1);
    }
    #[test]
    fn global_startup_is_compatible_with_old_settings_and_persists_with_overrides() {
        use crate::model::VscodeStartup;
        let legacy = decode_user(br#"{"schemaVersion":1,"profileId":"existing","settings":{"scanDepth":2},"vscodeStartupOverrides":{"project":{"kind":"gitGraph"}}}"#).unwrap();
        assert_eq!(
            legacy.settings.default_vscode_startup,
            VscodeStartup::Default
        );
        assert_eq!(legacy.vscode_startup_for("other"), &VscodeStartup::Default);
        assert_eq!(
            legacy.vscode_startup_for("project"),
            &VscodeStartup::GitGraph
        );
        let temp = tempfile::tempdir().unwrap();
        let (store, _, _, _) = Storage::load(temp.path().into());
        let mut user = legacy;
        for startup in [
            VscodeStartup::File {
                path: "首页 & (index); $.html".into(),
            },
            VscodeStartup::GitGraph,
            VscodeStartup::Default,
        ] {
            user.settings.default_vscode_startup = startup.clone();
            store.save_user(&user).unwrap();
            let (_, restored, _, warnings) = Storage::load(temp.path().into());
            assert!(warnings.is_empty());
            assert_eq!(restored.profile_id, "existing");
            assert_eq!(restored.settings, user.settings);
            assert_eq!(restored.vscode_startup_for("other"), &startup);
            assert_eq!(
                restored.vscode_startup_for("project"),
                &VscodeStartup::GitGraph
            );
        }
    }
    #[test]
    fn startup_overrides_load_legacy_data_and_round_trip_independently() {
        use crate::model::VscodeStartup;
        let legacy = decode_user(br#"{"schemaVersion":1,"favorites":["one"]}"#).unwrap();
        assert!(legacy.vscode_startup_overrides.is_empty());
        assert!(legacy.favorites.contains("one"));
        let temp = tempfile::tempdir().unwrap();
        let (store, mut user, _, _) = Storage::load(temp.path().into());
        user.vscode_startup_overrides.insert(
            "one".into(),
            VscodeStartup::File {
                path: "src/index.html".into(),
            },
        );
        user.vscode_startup_overrides
            .insert("two".into(), VscodeStartup::GitGraph);
        store.save_user(&user).unwrap();
        let (_, restored, _, _) = Storage::load(temp.path().into());
        assert_eq!(
            restored.vscode_startup_overrides,
            user.vscode_startup_overrides
        );
        user.vscode_startup_overrides.remove("one");
        store.save_user(&user).unwrap();
        let (_, restored, _, _) = Storage::load(temp.path().into());
        assert!(!restored.vscode_startup_overrides.contains_key("one"));
        assert_eq!(
            restored.vscode_startup_overrides["two"],
            VscodeStartup::GitGraph
        );
    }
    #[test]
    fn startup_updates_preserve_v014_profile_and_storage_preferences() {
        use crate::model::{CodeRoot, VscodeStartup};
        let temp = tempfile::tempdir().unwrap();
        let (store, mut user, _, _) = Storage::load(temp.path().into());
        user.roots.push(CodeRoot {
            id: "existing-root".into(),
            path: "D:/代码 & (projects); $".into(),
        });
        user.settings.data_location = Some("D:/Custom 数据".into());
        user.favorites.insert("existing-project".into());
        user.recent.insert("existing-project".into(), 123);
        user.category_overrides
            .insert("existing-project".into(), "App".into());
        store.save_user(&user).unwrap();
        let (_, mut loaded, _, _) = Storage::load(temp.path().into());
        loaded
            .vscode_startup_overrides
            .insert("existing-project".into(), VscodeStartup::GitGraph);
        store.save_user(&loaded).unwrap();
        let (_, restored, _, _) = Storage::load(temp.path().into());
        assert_eq!(restored.profile_id, user.profile_id);
        assert_eq!(restored.roots, user.roots);
        assert_eq!(restored.settings, user.settings);
        assert_eq!(restored.favorites, user.favorites);
        assert_eq!(restored.recent, user.recent);
        assert_eq!(restored.category_overrides, user.category_overrides);
        assert_eq!(
            restored.vscode_startup_overrides["existing-project"],
            VscodeStartup::GitGraph
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replacement_follows_individually_redirected_files() {
        let temp = tempfile::tempdir().unwrap();
        let actual = temp.path().join("actual");
        let logical = temp.path().join("logical");
        fs::create_dir(&actual).unwrap();
        fs::create_dir(&logical).unwrap();
        let destination = actual.join("state.json");
        let alias = logical.join("state.json");
        atomic(&destination, b"previous").unwrap();
        std::os::unix::fs::symlink(&destination, &alias).unwrap();
        atomic(&alias, b"updated").unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"updated");
        assert_eq!(fs::read(&alias).unwrap(), b"updated");
        assert!(fs::symlink_metadata(&alias)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_dir(&actual).unwrap().count(), 1);
        assert_eq!(fs::read_dir(&logical).unwrap().count(), 1);
    }

    #[test]
    fn persistence_resolves_the_actual_parent_before_atomic_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let logical = temp.path().join("data").join("..").join("data");
        let (store, user, _, warnings) = Storage::load(logical);
        assert!(warnings.is_empty());
        assert_eq!(
            store.directory,
            dunce::canonicalize(temp.path().join("data")).unwrap()
        );
        store.save_user(&user).unwrap();
        store.save_user(&user).unwrap();
        assert!(store.directory.join("state.json.bak").is_file());
    }
    #[test]
    fn persistence_backup_recovery_and_future_version() {
        let temp = tempfile::tempdir().unwrap();
        let (store, mut user, _, _) = Storage::load(temp.path().into());
        user.favorites.insert("d:\\代码\\project".into());
        store.save_user(&user).unwrap();
        user.recent.insert("d:\\代码\\project".into(), 12);
        store.save_user(&user).unwrap();
        let (_, restored, _, _) = Storage::load(temp.path().into());
        assert_eq!(restored.recent.values().next(), Some(&12));
        fs::write(temp.path().join("state.json"), "bad").unwrap();
        let (recovered, data, _, warnings) = Storage::load(temp.path().into());
        assert!(data.favorites.contains("d:\\代码\\project"));
        assert_eq!(warnings[0].code, "storageRecovered");
        recovered.save_user(&data).unwrap();
        fs::write(temp.path().join("state.json"), r#"{"schemaVersion":99}"#).unwrap();
        let (future, _, _, warnings) = Storage::load(temp.path().into());
        assert!(future.read_only);
        assert_eq!(warnings[0].code, "storageNewerVersion");
        assert!(future.save_user(&data).is_err());
        assert!(fs::read_to_string(temp.path().join("state.json"))
            .unwrap()
            .contains("99"));
    }
    #[test]
    fn version_zero_preserves_fields() {
        let user = decode_user(br#"{"favorites":["path"],"settings":{"scanDepth":3}}"#).unwrap();
        assert!(user.favorites.contains("path"));
        assert_eq!(user.settings.scan_depth, 3);
        assert_eq!(user.settings.theme, "dark");
    }
}
