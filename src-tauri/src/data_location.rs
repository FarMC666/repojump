use crate::{
    model::{AppError, AppResult, IndexCache, UserData},
    paths,
    storage::{atomic, decode_user, Storage},
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

fn existing_state(directory: &Path, profile: Option<&str>) -> AppResult<()> {
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
                return Ok(());
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
                    Ok(()) => true,
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
            legacy
        } else {
            recovery.clone()
        };
        let (mut active, user, index, initial) = Storage::load(source);
        active.read_only |= locator_read_only;
        warnings.extend(initial);
        let mut manager = Self {
            active,
            bootstrap,
            recovery,
            profile_id: user.profile_id.clone(),
        };
        if !manager.active.read_only {
            match manager.commit(&user, &index, true) {
                Ok(extra) => warnings.extend(extra),
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
        self.active.save_cache(index)?;
        if !same(&self.active.directory, &self.recovery) {
            // Cache is rebuildable; preferences were already mirrored by commit.
            if let Ok(target) = prepare(&self.recovery) {
                existing_state(&target.directory, Some(&self.profile_id))?;
                target.save_cache(index)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CodeRoot;

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
        user.settings.data_location = None;
        user.roots.clear();
        manager.commit(&user, &index, true).unwrap();
        assert_eq!(manager.active.directory, base.join(FOLDER));
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
