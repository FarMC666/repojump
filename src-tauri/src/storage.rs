use crate::model::{AppError, AppResult, IndexCache, UserData, SCHEMA_VERSION};
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

fn decode_user(bytes: &[u8]) -> AppResult<UserData> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| AppError::new("storageCorrupt", e.to_string()))?;
    if !value.is_object() {
        return Err(AppError::new("storageCorrupt", "Expected an object"));
    }
    let version = value
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if version > u64::from(SCHEMA_VERSION) {
        return Err(AppError::new("storageNewerVersion", version.to_string()));
    }
    // Version zero used the same fields without a version. Preserve them and fill defaults.
    value["schemaVersion"] = SCHEMA_VERSION.into();
    serde_json::from_value(value).map_err(|e| AppError::new("storageCorrupt", e.to_string()))
}

fn atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file =
        AtomicWriteFile::open(path).map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    file.write_all(bytes)
        .map_err(|e| AppError::new("storageFailure", e.to_string()))?;
    file.commit()
        .map_err(|e| AppError::new("storageFailure", e.to_string()))
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
        let cache = fs::read(directory.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<IndexCache>(&b).ok())
            .filter(|c| c.schema_version == SCHEMA_VERSION)
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
            decode_user(&previous)?;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
