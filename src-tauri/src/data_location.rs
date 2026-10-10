use crate::{
    model::{AppError, AppResult, IndexCache, UserData},
    paths,
    storage::{atomic, decode_user, preserve_migration_checkpoint, Storage},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const FOLDER: &str = ".repojump";
const LOCATOR: &str = "storage-location.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    schema_version: u32,
    directory: PathBuf,
    profile_id: String,
}

/// Only this manager changes the active location. The bootstrap directory keeps
/// a locator and recovery copy so an offline code drive cannot lose preferences.
pub struct StorageManager {
    pub active: Storage,
    bootstrap: PathBuf,
    recovery: PathBuf,
    profile_id: String,
}

fn same(a: &Path, b: &Path) -> bool {
    paths::identity(&a.to_string_lossy()) == paths::identity(&b.to_string_lossy())
}

fn hide(directory: &Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{
                SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES,
            },
        };
        let metadata =
            fs::metadata(directory).map_err(|e| AppError::new("storageFailure", e.to_string()))?;
        let name: Vec<u16> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            SetFileAttributesW(
                PCWSTR(name.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(metadata.file_attributes() | FILE_ATTRIBUTE_HIDDEN.0),
            )
        }
        .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    }
    #[cfg(not(windows))]
    let _ = directory;
    Ok(())
}

fn prepare(directory: &Path) -> AppResult<Storage> {
    // Do not create a missing root or follow a junction at the data folder.
    let parent = directory
        .parent()
        .ok_or_else(|| AppError::new("invalidPath", ""))?;
    if !parent.is_dir() {
        return Err(AppError::new(
            "directoryUnavailable",
            parent.display().to_string(),
        ));
    }
    if let Ok(metadata) = fs::symlink_metadata(directory) {
        if paths::is_reparse(&metadata) || !metadata.is_dir() {
            return Err(AppError::new(
                "storageLocationConflict",
                directory.display().to_string(),
            ));
        }
    }
    fs::create_dir_all(directory).map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    hide(directory)?;
    Ok(Storage {
        directory: dunce::canonicalize(directory)
            .map_err(|e| AppError::new("storageFailure", e.to_string()))?,
        read_only: false,
    })
}

fn existing_state(directory: &Path, profile: Option<&str>) -> AppResult<UserData> {
    let metadata = fs::symlink_metadata(directory)
        .map_err(|e| AppError::new("directoryUnavailable", e.to_string()))?;
    if paths::is_reparse(&metadata) || !metadata.is_dir() {
        return Err(AppError::new(
            "storageLocationConflict",
            directory.display().to_string(),
        ));
    }
    let mut found = false;
    for name in ["state.json", "state.json.bak"] {
        let bytes = match fs::read(directory.join(name)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(AppError::new("storageFailure", e.to_string())),
        };
        found = true;
        match decode_user(&bytes) {
            Ok(user) => {
                if let Some(profile) = profile {
                    if user.profile_id != profile {
                        return Err(AppError::new(
                            "storageLocationConflict",
                            directory.display().to_string(),
                        ));
                    }
                }
                return Ok(user);
            }
            Err(e) if e.code == "storageNewerVersion" => return Err(e),
            Err(_) => {}
        }
    }
    Err(AppError::new(
        if found {
            "storageCorrupt"
        } else {
            "directoryUnavailable"
        },
        directory.display().to_string(),
    ))
}

fn equivalent_preferences(left: &UserData, right: &UserData) -> bool {
    // Root IDs and profile IDs are internal identities. Separate upgrades of the
    // same legacy configuration can generate different IDs without changing data.
    left.schema_version == right.schema_version
        && left
            .roots
            .iter()
            .map(|root| paths::identity(&root.path))
            .eq(right.roots.iter().map(|root| paths::identity(&root.path)))
        && left.manual_projects == right.manual_projects
        && left.favorites == right.favorites
        && left.recent == right.recent
        && left.category_overrides == right.category_overrides
        // v0.1.4 did not store startup overrides. Compatible additions made in
        // separate AppData views must not undo duplicate-profile recovery.
        && left.vscode_startup_overrides.iter().all(|(id, startup)| {
            right.vscode_startup_overrides.get(id).is_none_or(|other| other == startup)
        })
        && left.editor_overrides.iter().all(|(id, editor)| right.editor_overrides.get(id).is_none_or(|other| other == editor))
        && left.settings == right.settings
}

fn check_destination(store: &Storage, user: &UserData) -> AppResult<()> {
    for name in ["state.json", "state.json.bak"] {
        let path = store.directory.join(name);
        if path.exists() {
            let bytes =
                fs::read(&path).map_err(|e| AppError::new("storageFailure", e.to_string()))?;
            let old = decode_user(&bytes)?;
            if old.profile_id != user.profile_id {
                return Err(AppError::new(
                    "storageLocationConflict",
                    path.display().to_string(),
                ));
            }
            return Ok(());
        }
    }
    if store.directory.join("index.json").exists() {
        return Err(AppError::new(
            "storageLocationConflict",
            store.directory.display().to_string(),
        ));
    }
    Ok(())
}

impl StorageManager {
    /// Runtime launch files stay in app data when preferences move to a code root.
    pub fn bootstrap_directory(&self) -> &Path {
        &self.bootstrap
    }

    pub fn load(bootstrap: PathBuf) -> (Self, UserData, IndexCache, Vec<AppError>) {
        let _ = fs::create_dir_all(&bootstrap);
        let legacy = bootstrap.clone();
        let mut warnings = Vec::new();
        // A packaged parent can redirect writes while reads of an existing
        // AppData directory still resolve to its original volume. Anchor the
        // locator to the newly created recovery folder's actual parent.
        let (bootstrap, recovery) = match prepare(&bootstrap.join(FOLDER)) {
            Ok(store) => (
                store.directory.parent().unwrap().to_path_buf(),
                store.directory,
            ),
            Err(error) => {
                warnings.push(error);
                (bootstrap.clone(), bootstrap.join(FOLDER))
            }
        };
        let location = match fs::read(bootstrap.join(LOCATOR)) {
            Ok(bytes) => match serde_json::from_slice::<Location>(&bytes) {
                Ok(location) => Some(location),
                Err(_) => {
                    let preserved = bootstrap.join(format!(
                        "storage-location.corrupt.{}.json",
                        crate::model::now_ms()
                    ));
                    if let Err(e) = fs::rename(bootstrap.join(LOCATOR), &preserved) {
                        warnings.push(AppError::new("storageFailure", e.to_string()));
                    }
                    warnings.push(AppError::new(
                        "storageLocationRecovered",
                        preserved.display().to_string(),
                    ));
                    None
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                warnings.push(AppError::new("storageFailure", e.to_string()));
                None
            }
        };
        let mut locator_read_only = false;
        let source = if let Some(location) = location {
            if location.schema_version > 1 {
                locator_read_only = true;
                warnings.push(AppError::new(
                    "storageNewerVersion",
                    location.schema_version.to_string(),
                ));
                location.directory
            } else if location.directory.is_absolute()
                && match existing_state(&location.directory, Some(&location.profile_id)) {
                    Ok(_) => true,
                    Err(error) => error.code == "storageNewerVersion",
                }
            {
                location.directory
            } else {
                warnings.push(AppError::new(
                    "storageLocationFallback",
                    location.directory.display().to_string(),
                ));
                recovery.clone()
            }
        } else if recovery.join("state.json").exists() || recovery.join("state.json.bak").exists() {
            recovery.clone()
        } else if legacy.join("state.json").exists() || legacy.join("state.json.bak").exists() {
            // Upgrade from the original AppData-only layout without dropping fields.
            legacy.clone()
        } else {
            recovery.clone()
        };
        let migrating_legacy = same(&source, &legacy)
            && fs::read(source.join("state.json"))
                .or_else(|_| fs::read(source.join("state.json.bak")))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .is_some_and(|value| value.is_object() && value.get("profileId").is_none());
        let (mut active, mut user, mut index, initial) = Storage::load(source);
        active.read_only |= locator_read_only;
        warnings.extend(initial);
        let mut manager = Self {
            active,
            bootstrap,
            recovery,
            profile_id: user.profile_id.clone(),
        };
        if !manager.active.read_only {
            let desired = manager.desired(&user);
            if user.settings.data_location.is_none()
                && !user.roots.is_empty()
                && !same(&manager.active.directory, &desired)
            {
                if let Ok(existing) = existing_state(&desired, None) {
                    let shared_legacy_root = migrating_legacy
                        && user.roots.iter().any(|root| {
                            existing.roots.iter().any(|other| {
                                root.id == other.id
                                    && same(Path::new(&root.path), Path::new(&other.path))
                            })
                        });
                    let duplicate = equivalent_preferences(&user, &existing);
                    if existing.profile_id != user.profile_id && (shared_legacy_root || duplicate) {
                        // Adopt the existing state instead of overwriting it with
                        // a second migration made through a different AppData view.
                        // Divergent profiles and custom locations remain protected.
                        let duplicate_recovery =
                            duplicate && same(&manager.active.directory, &manager.recovery);
                        let startup = user.vscode_startup_overrides.clone();
                        let editor_overrides = user.editor_overrides.clone();
                        let (active, restored, cache, extra) = Storage::load(desired);
                        manager.active = active;
                        manager.profile_id = restored.profile_id.clone();
                        user = restored;
                        if duplicate {
                            // Existing root identities and user records remain
                            // authoritative; preserve nonconflicting new options.
                            user.vscode_startup_overrides.extend(startup);
                            user.editor_overrides.extend(editor_overrides);
                        }
                        index = cache;
                        warnings.extend(extra);
                        if duplicate_recovery && !manager.active.read_only {
                            if let Err(error) = manager.align_duplicate_recovery(&user) {
                                warnings.push(AppError::new(
                                    "storageBackupFailure",
                                    error.detail.unwrap_or_default(),
                                ));
                            }
                        }
                    }
                }
            }
            if manager.active.read_only {
                return (manager, user, index, warnings);
            }
            match manager.commit(&user, &index, true) {
                Ok(extra) => {
                    // Only the final location matters: recovering from a stale
                    // locator can successfully return to the preferred directory.
                    warnings.retain(|warning| warning.code != "storageLocationFallback");
                    warnings.extend(extra);
                }
                Err(error) => {
                    warnings.push(error);
                    // A previously configured custom drive may now be offline.
                    match manager.move_to(&manager.recovery.clone(), &user, &index) {
                        Ok(()) => warnings.push(AppError::new(
                            "storageLocationFallback",
                            manager.active.directory.display().to_string(),
                        )),
                        Err(error) => {
                            manager.active.read_only = true;
                            warnings.push(error);
                        }
                    }
                }
            }
        }
        (manager, user, index, warnings)
    }

    fn desired(&self, user: &UserData) -> PathBuf {
        user.settings
            .data_location
            .as_deref()
            .or_else(|| user.roots.first().map(|root| root.path.as_str()))
            .map(PathBuf::from)
            .unwrap_or_else(|| self.bootstrap.clone())
            .join(FOLDER)
    }

    fn align_duplicate_recovery(&self, user: &UserData) -> AppResult<()> {
        let target = prepare(&self.recovery)?;
        let primary = target.directory.join("state.json");
        let previous =
            fs::read(&primary).map_err(|e| AppError::new("storageFailure", e.to_string()))?;
        // Recheck compatibility before aligning identities and startup additions.
        // Keep the previous state as a backup, just as with ordinary persistence.
        if !equivalent_preferences(&decode_user(&previous)?, user) {
            return Err(AppError::new(
                "storageLocationConflict",
                primary.display().to_string(),
            ));
        }
        preserve_migration_checkpoint(&target.directory, &previous)?;
        atomic(&target.directory.join("state.json.bak"), &previous)?;
        let bytes = serde_json::to_vec_pretty(user)
            .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
        atomic(&primary, &bytes)
    }

    fn locator(&self, directory: &Path, user: &UserData) -> AppResult<()> {
        let bytes = serde_json::to_vec_pretty(&Location {
            schema_version: 1,
            directory: directory.into(),
            profile_id: user.profile_id.clone(),
        })
        .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
        atomic(&self.bootstrap.join(LOCATOR), &bytes)
    }

    fn move_to(&mut self, directory: &Path, user: &UserData, index: &IndexCache) -> AppResult<()> {
        let target = prepare(directory)?;
        if same(&target.directory, &self.active.directory) {
            // Updating the same location must not save settings and then report
            // failure because the locator or rebuildable cache could not be saved.
            self.locator(&target.directory, user)?;
            target.save_user(user)?;
            self.active = target;
            return Ok(());
        }
        check_destination(&target, user)?;
        target.save_user(user)?;
        target.save_cache(index)?;
        // The old source remains authoritative until the locator replacement succeeds.
        self.locator(&target.directory, user)?;
        self.active = target;
        Ok(())
    }

    pub fn commit(
        &mut self,
        user: &UserData,
        index: &IndexCache,
        relocate: bool,
    ) -> AppResult<Vec<AppError>> {
        if self.active.read_only {
            return Err(AppError::new(
                "storageReadOnly",
                self.active.directory.display().to_string(),
            ));
        }
        let mut warnings = Vec::new();
        if relocate {
            let desired = self.desired(user);
            if let Err(error) = self.move_to(&desired, user, index) {
                if user.settings.data_location.is_some() {
                    return Err(error);
                }
                // Automatic storage follows the first root; an inaccessible root
                // uses the default location rather than preventing root management.
                self.move_to(&self.recovery.clone(), user, index)?;
                warnings.push(AppError::new(
                    "storageLocationFallback",
                    error.detail.unwrap_or_default(),
                ));
            }
        } else if let Err(error) = self.active.save_user(user) {
            if same(&self.active.directory, &self.recovery) {
                return Err(error);
            }
            // A code drive can disappear while the application is already open.
            self.move_to(&self.recovery.clone(), user, index)?;
            warnings.push(AppError::new(
                "storageLocationFallback",
                error.detail.unwrap_or_default(),
            ));
        }
        if !same(&self.active.directory, &self.recovery) {
            let mirrored = prepare(&self.recovery).and_then(|target| {
                check_destination(&target, user)?;
                target.save_user(user)?;
                target.save_cache(index)
            });
            if let Err(error) = mirrored {
                warnings.push(AppError::new(
                    "storageBackupFailure",
                    error.detail.unwrap_or_default(),
                ));
            }
        }
        Ok(warnings)
    }

    pub fn save_cache(&self, index: &IndexCache) -> AppResult<()> {
        if self.active.read_only {
            return Ok(());
        }
        let active = self.active.save_cache(index);
        if !same(&self.active.directory, &self.recovery) {
            // Cache is rebuildable; preferences were already mirrored by commit.
            if let Ok(target) = prepare(&self.recovery) {
                existing_state(&target.directory, Some(&self.profile_id))?;
                target.save_cache(index)?;
            }
        }
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CodeRoot, VscodeStartup};

    #[test]
    fn editor_profiles_and_overrides_survive_custom_storage_offline_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let bootstrap = temp.path().join("local");
        let custom = temp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(bootstrap.clone());
        user.settings.data_location = Some(custom.to_string_lossy().into());
        user.settings.default_editor_id = "cursor".into();
        for definition in crate::editors::REGISTRY {
            user.settings.editor_profiles.insert(
                definition.id.into(),
                crate::editors::EditorProfileConfig {
                    executable_path: Some(format!("Z:/Editors/{}", definition.executable)),
                },
            );
        }
        user.editor_overrides
            .insert("project".into(), "vscode-insiders".into());
        user.vscode_startup_overrides
            .insert("project".into(), VscodeStartup::GitGraph);
        user.favorites.insert("project".into());
        manager.commit(&user, &index, true).unwrap();
        let offline = temp.path().join("offline");
        fs::rename(&custom, &offline).unwrap();
        user.editor_overrides
            .insert("project".into(), "windsurf".into());
        user.recent.insert("project".into(), 456);
        let warnings = manager.commit(&user, &index, false).unwrap();
        assert!(warnings
            .iter()
            .any(|warning| warning.code == "storageLocationFallback"));
        fs::rename(&offline, &custom).unwrap();
        let (manager, restored, _, warnings) = StorageManager::load(bootstrap);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(manager.active.directory, custom.join(FOLDER));
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&user).unwrap()
        );
        assert_eq!(restored.editor_for("project"), "windsurf");
        assert_eq!(restored.editor_for("other"), "cursor");
    }

    #[test]
    fn separate_appdata_views_reuse_equivalent_root_data_and_its_identities() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("desktop-appdata");
        let root = temp.path().join("代码 & (projects);");
        fs::create_dir(&root).unwrap();
        let (mut desktop, mut user, index, _) = StorageManager::load(base.clone());
        user.roots.push(CodeRoot {
            id: "desktop-root".into(),
            path: root.to_string_lossy().into_owned(),
        });
        user.favorites.insert("project".into());
        user.recent.insert("project".into(), 123);
        user.vscode_startup_overrides
            .insert("project".into(), VscodeStartup::GitGraph);
        desktop.commit(&user, &index, false).unwrap();
        let mut existing = user.clone();
        existing.profile_id = "original-profile".into();
        existing.roots[0].id = "original-root".into();
        let root_store = prepare(&root.join(FOLDER)).unwrap();
        root_store.save_user(&existing).unwrap();
        root_store.save_cache(&index).unwrap();

        let (manager, restored, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert_eq!(restored.profile_id, existing.profile_id);
        assert_eq!(restored.roots, existing.roots);
        assert!(restored.favorites.contains("project"));
        assert_eq!(restored.recent["project"], 123);
        assert_eq!(
            restored.vscode_startup_overrides,
            user.vscode_startup_overrides
        );
        let recovery =
            decode_user(&fs::read(base.join(FOLDER).join("state.json")).unwrap()).unwrap();
        assert_eq!(recovery.profile_id, existing.profile_id);
        let (manager, restarted, _, warnings) = StorageManager::load(base);
        assert!(warnings.is_empty());
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert_eq!(restarted.profile_id, existing.profile_id);
    }

    #[test]
    fn legacy_without_locator_adopts_newer_data_with_the_same_root_identity() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&base).unwrap();
        fs::create_dir(&root).unwrap();
        let mut existing = UserData::default();
        existing.roots.push(CodeRoot {
            id: "shared-legacy-root".into(),
            path: root.to_string_lossy().into_owned(),
        });
        let mut legacy = serde_json::to_value(&existing).unwrap();
        legacy.as_object_mut().unwrap().remove("profileId");
        let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
        fs::write(base.join("state.json"), &legacy_bytes).unwrap();
        existing.favorites.insert("added-after-upgrade".into());
        existing.recent.insert("added-after-upgrade".into(), 456);
        prepare(&root.join(FOLDER))
            .unwrap()
            .save_user(&existing)
            .unwrap();

        let (manager, restored, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert_eq!(restored.profile_id, existing.profile_id);
        assert!(restored.favorites.contains("added-after-upgrade"));
        assert_eq!(restored.recent["added-after-upgrade"], 456);
        assert_eq!(fs::read(base.join("state.json")).unwrap(), legacy_bytes);
    }

    #[test]
    fn divergent_profiles_remain_protected_during_automatic_startup() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&root).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        user.roots.push(CodeRoot {
            id: "root".into(),
            path: root.to_string_lossy().into_owned(),
        });
        user.favorites.insert("local-favorite".into());
        manager.commit(&user, &index, false).unwrap();
        let mut other = user.clone();
        other.profile_id = "another-profile".into();
        other.favorites.clear();
        let destination = prepare(&root.join(FOLDER)).unwrap();
        destination.save_user(&other).unwrap();
        let original = fs::read(destination.directory.join("state.json")).unwrap();

        let (manager, restored, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings
            .iter()
            .any(|warning| warning.code == "storageLocationFallback"));
        assert_eq!(manager.active.directory, base.join(FOLDER));
        assert_eq!(restored.profile_id, user.profile_id);
        assert!(restored.favorites.contains("local-favorite"));
        assert_eq!(
            fs::read(destination.directory.join("state.json")).unwrap(),
            original
        );
    }

    #[test]
    fn duplicate_v014_profiles_merge_nonconflicting_startup_preferences() {
        for legacy in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let base = temp.path().join("appdata");
            let root = temp.path().join("代码 & (projects);");
            fs::create_dir(&root).unwrap();
            let (mut desktop, mut user, index, _) = StorageManager::load(base.clone());
            user.roots.push(CodeRoot {
                id: "duplicate-root".into(),
                path: root.to_string_lossy().into_owned(),
            });
            user.recent.insert("project".into(), 123);
            if !legacy {
                user.vscode_startup_overrides.insert(
                    "source-only".into(),
                    VscodeStartup::File {
                        path: "index.html".into(),
                    },
                );
            }
            desktop.commit(&user, &index, false).unwrap();
            if legacy {
                let primary = base.join(FOLDER).join("state.json");
                let mut old = serde_json::to_value(&user).unwrap();
                old.as_object_mut()
                    .unwrap()
                    .remove("vscodeStartupOverrides");
                fs::write(primary, serde_json::to_vec(&old).unwrap()).unwrap();
            }
            let mut existing = user.clone();
            existing.profile_id = "original-profile".into();
            existing.roots[0].id = "original-root".into();
            existing.vscode_startup_overrides.clear();
            existing
                .vscode_startup_overrides
                .insert("target-only".into(), VscodeStartup::GitGraph);
            let target = prepare(&root.join(FOLDER)).unwrap();
            target.save_user(&existing).unwrap();
            let mut expected = existing.vscode_startup_overrides.clone();
            expected.extend(user.vscode_startup_overrides.clone());

            let (manager, restored, _, warnings) = StorageManager::load(base.clone());
            assert!(warnings.is_empty(), "legacy={legacy}: {warnings:?}");
            assert_eq!(manager.active.directory, root.join(FOLDER));
            assert_eq!(restored.profile_id, existing.profile_id);
            assert_eq!(restored.roots, existing.roots);
            assert_eq!(restored.vscode_startup_overrides, expected);
            assert_eq!(restored.recent, user.recent);
            let recovery =
                decode_user(&fs::read(base.join(FOLDER).join("state.json")).unwrap()).unwrap();
            assert_eq!(recovery.profile_id, existing.profile_id);
            assert_eq!(recovery.vscode_startup_overrides, expected);
            let (manager, restarted, _, warnings) = StorageManager::load(base);
            assert!(warnings.is_empty());
            assert_eq!(manager.active.directory, root.join(FOLDER));
            assert_eq!(restarted.vscode_startup_overrides, expected);
        }
    }

    #[test]
    fn different_startup_preferences_do_not_adopt_another_profile() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&root).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        user.roots.push(CodeRoot {
            id: "root".into(),
            path: root.to_string_lossy().into_owned(),
        });
        user.vscode_startup_overrides
            .insert("project".into(), VscodeStartup::GitGraph);
        manager.commit(&user, &index, false).unwrap();
        let mut other = user.clone();
        other.profile_id = "another-profile".into();
        other.vscode_startup_overrides.insert(
            "project".into(),
            VscodeStartup::File {
                path: "index.html".into(),
            },
        );
        let destination = prepare(&root.join(FOLDER)).unwrap();
        destination.save_user(&other).unwrap();
        let original = fs::read(destination.directory.join("state.json")).unwrap();

        let (manager, restored, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings
            .iter()
            .any(|warning| warning.code == "storageLocationFallback"));
        assert_eq!(manager.active.directory, base.join(FOLDER));
        assert_eq!(
            restored.vscode_startup_overrides,
            user.vscode_startup_overrides
        );
        assert_eq!(
            fs::read(destination.directory.join("state.json")).unwrap(),
            original
        );
    }

    #[test]
    fn recovering_a_stale_locator_does_not_keep_a_false_fallback_warning() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&root).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        user.roots.push(CodeRoot {
            id: "root".into(),
            path: root.to_string_lossy().into_owned(),
        });
        manager.commit(&user, &index, true).unwrap();
        manager
            .locator(&temp.path().join("missing"), &user)
            .unwrap();

        let (manager, restored, _, warnings) = StorageManager::load(base);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert_eq!(restored.profile_id, user.profile_id);
    }

    #[test]
    fn first_root_custom_location_restart_and_removal_preserve_data() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("My Code 代码 & (root);");
        let second = temp.path().join("other");
        let custom = temp.path().join("Custom 数据");
        for folder in [&root, &second, &custom] {
            fs::create_dir(folder).unwrap();
        }
        let (mut manager, mut user, index, warnings) = StorageManager::load(base.clone());
        assert!(warnings.is_empty());
        assert_eq!(manager.active.directory, base.join(FOLDER));
        let bootstrap = manager.bootstrap_directory().to_path_buf();
        user.vscode_startup_overrides.insert(
            "project".into(),
            VscodeStartup::File {
                path: "src/首页 & (index); $.html".into(),
            },
        );
        user.vscode_startup_overrides
            .insert("other".into(), VscodeStartup::GitGraph);
        let startup = user.vscode_startup_overrides.clone();
        user.settings.default_vscode_startup = VscodeStartup::GitGraph;
        user.favorites.insert("project".into());
        user.recent.insert("project".into(), 123);
        user.roots.push(CodeRoot {
            id: "first".into(),
            path: root.to_string_lossy().into_owned(),
        });
        manager.commit(&user, &index, true).unwrap();
        user.roots.push(CodeRoot {
            id: "second".into(),
            path: second.to_string_lossy().into_owned(),
        });
        manager.commit(&user, &index, true).unwrap();
        assert_eq!(manager.active.directory, root.join(FOLDER));
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_ne!(
                fs::metadata(&manager.active.directory)
                    .unwrap()
                    .file_attributes()
                    & 2,
                0
            );
        }
        user.settings.data_location = Some(custom.to_string_lossy().into_owned());
        manager.commit(&user, &index, true).unwrap();
        let (mut manager, mut user, index, warnings) = StorageManager::load(base.clone());
        assert!(warnings.is_empty());
        assert_eq!(manager.active.directory, custom.join(FOLDER));
        assert!(user.favorites.contains("project"));
        assert_eq!(user.recent["project"], 123);
        assert_eq!(user.vscode_startup_overrides, startup);
        assert_eq!(
            user.settings.default_vscode_startup,
            VscodeStartup::GitGraph
        );
        assert_eq!(manager.bootstrap_directory(), bootstrap);
        user.settings.data_location = None;
        user.roots.clear();
        manager.commit(&user, &index, true).unwrap();
        assert_eq!(manager.active.directory, base.join(FOLDER));
        let (_, restored, _, warnings) = StorageManager::load(base);
        assert!(warnings.is_empty());
        assert_eq!(restored.vscode_startup_overrides, startup);
        assert_eq!(
            restored.settings.default_vscode_startup,
            VscodeStartup::GitGraph
        );
        assert!(root.join(FOLDER).join("state.json").is_file());
    }

    #[test]
    fn legacy_upgrade_offline_fallback_and_reconnection_keep_latest_preferences() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&base).unwrap();
        fs::write(
            base.join("state.json"),
            serde_json::to_vec(&serde_json::json!({
                "roots": [{"id":"root", "path":root}], "favorites":["old"], "recent":{"old":12}
            }))
            .unwrap(),
        )
        .unwrap();
        let (manager, user, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings.is_empty());
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert_eq!(user.recent["old"], 12);
        fs::rename(&root, temp.path().join("offline")).unwrap();
        let (mut manager, mut user, index, warnings) = StorageManager::load(base.clone());
        assert!(warnings.iter().any(|e| e.code == "storageLocationFallback"));
        assert_eq!(manager.active.directory, base.join(FOLDER));
        user.favorites.insert("offline-new".into());
        manager.commit(&user, &index, false).unwrap();
        fs::rename(temp.path().join("offline"), &root).unwrap();
        let (manager, user, _, warnings) = StorageManager::load(base);
        assert!(warnings.is_empty());
        assert_eq!(manager.active.directory, root.join(FOLDER));
        assert!(user.favorites.contains("offline-new"));
    }

    #[test]
    fn conflicting_profile_and_failed_locator_leave_source_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let custom = temp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        let original = fs::read(manager.active.directory.join("state.json")).unwrap();
        let other = prepare(&custom.join(FOLDER)).unwrap();
        other.save_user(&UserData::default()).unwrap();
        let other_bytes = fs::read(other.directory.join("state.json")).unwrap();
        user.settings.data_location = Some(custom.to_string_lossy().into_owned());
        assert_eq!(
            manager.commit(&user, &index, true).unwrap_err().code,
            "storageLocationConflict"
        );
        assert_eq!(
            fs::read(other.directory.join("state.json")).unwrap(),
            other_bytes
        );
        assert_eq!(
            fs::read(manager.active.directory.join("state.json")).unwrap(),
            original
        );
        let empty = temp.path().join("empty");
        fs::create_dir(&empty).unwrap();
        user.settings.data_location = Some(empty.to_string_lossy().into_owned());
        fs::remove_file(base.join(LOCATOR)).unwrap();
        fs::create_dir(base.join(LOCATOR)).unwrap();
        assert!(manager.commit(&user, &index, true).is_err());
        assert_eq!(manager.active.directory, base.join(FOLDER));
        assert_eq!(
            fs::read(manager.active.directory.join("state.json")).unwrap(),
            original
        );
    }

    #[test]
    fn runtime_offline_and_damaged_locator_recover_without_losing_favorites() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let custom = temp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        user.settings.data_location = Some(custom.to_string_lossy().into_owned());
        manager.commit(&user, &index, true).unwrap();
        fs::rename(&custom, temp.path().join("offline")).unwrap();
        user.favorites.insert("while-offline".into());
        let warnings = manager.commit(&user, &index, false).unwrap();
        assert_eq!(warnings[0].code, "storageLocationFallback");
        assert_eq!(manager.active.directory, base.join(FOLDER));
        fs::write(base.join(LOCATOR), "damaged").unwrap();
        let (manager, user, _, warnings) = StorageManager::load(base.clone());
        assert!(warnings
            .iter()
            .any(|e| e.code == "storageLocationRecovered"));
        assert!(user.favorites.contains("while-offline"));
        assert!(!manager.active.read_only);
        assert!(fs::read_dir(base).unwrap().any(|file| file
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("storage-location.corrupt.")));
    }

    #[test]
    fn future_state_at_relocated_directory_is_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("appdata");
        let root = temp.path().join("code");
        fs::create_dir(&root).unwrap();
        let (mut manager, mut user, index, _) = StorageManager::load(base.clone());
        user.roots.push(CodeRoot {
            id: "r".into(),
            path: root.to_string_lossy().into_owned(),
        });
        manager.commit(&user, &index, true).unwrap();
        let primary = manager.active.directory.join("state.json");
        fs::write(&primary, r#"{"schemaVersion":99}"#).unwrap();
        let (manager, _, _, warnings) = StorageManager::load(base);
        assert!(manager.active.read_only);
        assert!(warnings.iter().any(|e| e.code == "storageNewerVersion"));
        assert_eq!(
            fs::read_to_string(primary).unwrap(),
            r#"{"schemaVersion":99}"#
        );
    }
}
