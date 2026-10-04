use crate::{
    data_location::StorageManager,
    discovery::{self, ScanEvent},
    git, launcher,
    model::*,
    paths, settings, vscode_startup,
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

pub struct Inner {
    pub storage: StorageManager,
    pub user: UserData,
    pub index: IndexCache,
    pub revision: u64,
    pub epoch: u64,
    pub scan: ScanStatus,
    pub pending: bool,
    pub warnings: Vec<AppError>,
    pub active_shortcut: Option<String>,
}

pub struct AppState {
    pub inner: Mutex<Inner>,
    startup_directory: PathBuf,
    git_cache: Mutex<HashMap<String, (Instant, GitMetadata)>>,
    launches: Mutex<HashSet<String>>,
}

fn reconcile(user: &UserData, index: &mut IndexCache) {
    let roots: BTreeSet<_> = user.roots.iter().map(|r| r.id.clone()).collect();
    let manuals: HashSet<_> = user
        .manual_projects
        .iter()
        .map(|p| paths::identity(p))
        .collect();
    for project in index.projects.values_mut() {
        project.root_ids.retain(|r| roots.contains(r));
        project.manual = manuals.contains(&project.id);
    }
    index
        .projects
        .retain(|_, p| p.manual || !p.root_ids.is_empty());
    for path in &user.manual_projects {
        let id = paths::identity(path);
        index
            .projects
            .entry(id.clone())
            .or_insert_with(|| ProjectRecord {
                id,
                name: Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                path: path.clone(),
                tags: Vec::new(),
                is_git: false,
                root_ids: BTreeSet::new(),
                manual: true,
                availability: Availability::Unknown,
            });
    }
}

fn merge(index: &mut IndexCache, mut project: ProjectRecord) {
    if let Some(existing) = index.projects.get(&project.id) {
        project.root_ids.extend(existing.root_ids.iter().cloned());
        project.manual |= existing.manual;
    }
    index.projects.insert(project.id.clone(), project);
}

fn promote_manual_source(user: &mut UserData, path: String) {
    let id = paths::identity(&path);
    if !user
        .roots
        .iter()
        .any(|root| paths::identity(&root.path) == id)
    {
        user.roots.push(CodeRoot {
            id: uuid::Uuid::new_v4().to_string(),
            path,
        });
    }
    user.manual_projects
        .retain(|manual| paths::identity(manual) != id);
}

impl AppState {
    pub fn new(directory: PathBuf) -> Self {
        let (storage, user, mut index, warnings) = StorageManager::load(directory);
        let startup_directory = storage.bootstrap_directory().to_path_buf();
        vscode_startup::cleanup_expired(&startup_directory);
        reconcile(&user, &mut index);
        Self {
            startup_directory,
            inner: Mutex::new(Inner {
                storage,
                user,
                index,
                revision: 1,
                epoch: 0,
                scan: ScanStatus::default(),
                pending: false,
                warnings,
                active_shortcut: None,
            }),
            git_cache: Mutex::new(HashMap::new()),
            launches: Mutex::new(HashSet::new()),
        }
    }

    pub fn snapshot_locked(&self, inner: &Inner) -> AppSnapshot {
        AppSnapshot {
            revision: inner.revision,
            roots: inner.user.roots.clone(),
            projects: inner
                .index
                .projects
                .values()
                .map(|record| Project {
                    record: record.clone(),
                    category: inner
                        .user
                        .category_overrides
                        .get(&record.id)
                        .cloned()
                        .or_else(|| paths::category(&record.path, &inner.user.roots)),
                    category_override: inner.user.category_overrides.contains_key(&record.id),
                    favorite: inner.user.favorites.contains(&record.id),
                    last_opened_at: inner.user.recent.get(&record.id).copied(),
                    vscode_startup: inner
                        .user
                        .vscode_startup_overrides
                        .get(&record.id)
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect(),
            settings: inner.user.settings.clone(),
            scan: inner.scan.clone(),
            warnings: inner.warnings.clone(),
            data_directory: inner
                .storage
                .active
                .directory
                .to_string_lossy()
                .into_owned(),
            storage_read_only: inner.storage.active.read_only,
        }
    }

    pub fn snapshot(&self) -> AppSnapshot {
        self.snapshot_locked(&self.inner.lock().unwrap())
    }

    fn mutate(
        &self,
        app: &AppHandle,
        rescan: bool,
        edit: impl FnOnce(&mut UserData, &mut IndexCache) -> AppResult<()>,
    ) -> AppResult<AppSnapshot> {
        let snapshot = {
            let mut inner = self.inner.lock().unwrap();
            let mut user = inner.user.clone();
            let mut index = inner.index.clone();
            edit(&mut user, &mut index)?;
            reconcile(&user, &mut index);
            let relocate = user.roots != inner.user.roots
                || user.settings.data_location != inner.user.settings.data_location;
            let warnings = inner.storage.commit(&user, &index, relocate)?;
            if relocate {
                inner
                    .warnings
                    .retain(|warning| warning.code != "storageLocationFallback");
            }
            for warning in warnings {
                add_warning(&mut inner, warning);
            }
            inner.user = user;
            inner.index = index;
            inner.revision += 1;
            if rescan {
                inner.epoch += 1;
            }
            if let Err(e) = inner.storage.save_cache(&inner.index) {
                add_warning(&mut inner, e);
            }
            self.snapshot_locked(&inner)
        };
        let _ = app.emit("index-updated", &snapshot);
        if rescan {
            request_scan(app);
        }
        Ok(snapshot)
    }

    pub fn add_root(&self, app: &AppHandle, path: String) -> AppResult<AppSnapshot> {
        let path = paths::directory(&path)?.to_string_lossy().into_owned();
        self.mutate(app, true, |user, _| {
            if user
                .roots
                .iter()
                .any(|r| paths::identity(&r.path) == paths::identity(&path))
            {
                return Err(AppError::new("rootDuplicate", path));
            }
            user.roots.push(CodeRoot {
                id: uuid::Uuid::new_v4().to_string(),
                path,
            });
            Ok(())
        })
    }

    pub fn update_root(&self, app: &AppHandle, id: String, path: String) -> AppResult<AppSnapshot> {
        let path = paths::directory(&path)?.to_string_lossy().into_owned();
        self.mutate(app, true, |user, index| {
            if user
                .roots
                .iter()
                .any(|r| r.id != id && paths::identity(&r.path) == paths::identity(&path))
            {
                return Err(AppError::new("rootDuplicate", path));
            }
            let root = user
                .roots
                .iter_mut()
                .find(|r| r.id == id)
                .ok_or_else(|| AppError::new("rootNotFound", id.clone()))?;
            if paths::identity(&root.path) != paths::identity(&path) {
                for project in index.projects.values_mut() {
                    project.root_ids.remove(&id);
                }
            }
            root.path = path;
            Ok(())
        })
    }

    pub fn remove_root(&self, app: &AppHandle, id: String) -> AppResult<AppSnapshot> {
        self.mutate(app, true, |user, _| {
            user.roots.retain(|r| r.id != id);
            Ok(())
        })
    }

    pub fn add_manual(&self, app: &AppHandle, path: String) -> AppResult<AppSnapshot> {
        let path = paths::directory(&path)?.to_string_lossy().into_owned();
        let project = discovery::manual(&path)
            .map_err(|e| AppError::new("directoryUnavailable", e.to_string()))?;
        self.mutate(app, false, |user, index| {
            if !user
                .manual_projects
                .iter()
                .any(|p| paths::identity(p) == project.id)
            {
                user.manual_projects.push(path);
            }
            merge(index, project);
            Ok(())
        })
    }

    pub fn remove_manual(&self, app: &AppHandle, id: String) -> AppResult<AppSnapshot> {
        self.mutate(app, true, |user, _| {
            user.manual_projects.retain(|p| paths::identity(p) != id);
            Ok(())
        })
    }

    pub fn scan_manual_directory(&self, app: &AppHandle, id: &str) -> AppResult<AppSnapshot> {
        let path = {
            let inner = self.inner.lock().unwrap();
            inner
                .index
                .projects
                .get(id)
                .filter(|project| project.manual)
                .ok_or_else(|| AppError::new("projectNotFound", id))?
                .path
                .clone()
        };
        let path = paths::directory(&path)?.to_string_lossy().into_owned();
        self.mutate(app, true, |user, _| {
            promote_manual_source(user, path);
            Ok(())
        })
    }

    pub fn set_favorite(
        &self,
        app: &AppHandle,
        id: String,
        favorite: bool,
    ) -> AppResult<AppSnapshot> {
        self.mutate(app, false, |user, index| {
            if !index.projects.contains_key(&id) {
                return Err(AppError::new("projectNotFound", id));
            }
            if favorite {
                user.favorites.insert(id);
            } else {
                user.favorites.remove(&id);
            }
            Ok(())
        })
    }

    pub fn set_category(
        &self,
        app: &AppHandle,
        id: String,
        category: Option<String>,
    ) -> AppResult<AppSnapshot> {
        self.mutate(app, false, |user, index| {
            if !index.projects.contains_key(&id) {
                return Err(AppError::new("projectNotFound", id));
            }
            if let Some(category) = category {
                let category = category.trim();
                if category.is_empty() || category.chars().count() > 64 {
                    return Err(AppError::new("categoryInvalid", ""));
                }
                user.category_overrides.insert(id, category.into());
            } else {
                user.category_overrides.remove(&id);
            }
            Ok(())
        })
    }

    pub fn save_settings(&self, app: &AppHandle, mut next: Settings) -> AppResult<AppSnapshot> {
        settings::validate(&mut next)?;
        let (mut snapshot, rescan) = {
            let mut inner = self.inner.lock().unwrap();
            let previous = inner.user.settings.clone();
            let previous_active = inner.active_shortcut.clone();
            settings::change_shortcut(app, &previous_active, &next.global_shortcut)?;
            let mut user = inner.user.clone();
            user.settings = next;
            let index = inner.index.clone();
            let relocate = previous.data_location != user.settings.data_location;
            let warnings = match inner.storage.commit(&user, &index, relocate) {
                Ok(warnings) => warnings,
                Err(e) => {
                    if let Err(rollback) = settings::change_shortcut(
                        app,
                        &user.settings.global_shortcut,
                        &previous_active,
                    ) {
                        add_warning(&mut inner, rollback);
                    }
                    return Err(e);
                }
            };
            if relocate {
                inner
                    .warnings
                    .retain(|warning| warning.code != "storageLocationFallback");
            }
            for warning in warnings {
                add_warning(&mut inner, warning);
            }
            let rescan = previous.scan_depth != user.settings.scan_depth;
            inner.active_shortcut = user.settings.global_shortcut.clone();
            inner
                .warnings
                .retain(|warning| warning.code != "shortcutConflict");
            inner.user = user;
            inner.revision += 1;
            if rescan {
                inner.epoch += 1;
            }
            (self.snapshot_locked(&inner), rescan)
        };
        if let Err(error) = app
            .state::<crate::tray_menu::TrayMenu>()
            .apply(&snapshot.settings.language)
        {
            let mut inner = self.inner.lock().unwrap();
            add_warning(&mut inner, error);
            snapshot = self.snapshot_locked(&inner);
        }
        let _ = app.emit("index-updated", &snapshot);
        if rescan {
            request_scan(app);
        }
        Ok(snapshot)
    }

    pub fn set_vscode_startup(
        &self,
        app: &AppHandle,
        id: String,
        startup: VscodeStartup,
    ) -> AppResult<AppSnapshot> {
        self.mutate(app, false, |user, index| {
            let project = index
                .projects
                .get(&id)
                .ok_or_else(|| AppError::new("projectNotFound", &id))?;
            let startup = match startup {
                VscodeStartup::File { path } => {
                    let root = paths::directory(&project.path)?;
                    let file = paths::project_file(&root, &path)?;
                    VscodeStartup::File {
                        path: paths::relative_project_file(&root, &file)?,
                    }
                }
                startup => startup,
            };
            if startup == VscodeStartup::Default {
                user.vscode_startup_overrides.remove(&id);
            } else {
                user.vscode_startup_overrides.insert(id, startup);
            }
            Ok(())
        })
    }

    pub fn project_directory(&self, id: &str) -> AppResult<PathBuf> {
        paths::directory(&self.project(id)?.path)
    }

    fn project(&self, id: &str) -> AppResult<ProjectRecord> {
        self.inner
            .lock()
            .unwrap()
            .index
            .projects
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new("projectNotFound", id))
    }

    pub fn git_metadata(&self, id: &str) -> AppResult<GitMetadata> {
        let project = self.project(id)?;
        if !project.is_git {
            return Ok(GitMetadata {
                branch: None,
                dirty: None,
                repository_url: None,
            });
        }
        if let Some((time, cached)) = self.git_cache.lock().unwrap().get(id) {
            if time.elapsed() < Duration::from_secs(30) {
                return Ok(cached.clone());
            }
        }
        let metadata = git::metadata(Path::new(&project.path));
        self.git_cache
            .lock()
            .unwrap()
            .insert(id.into(), (Instant::now(), metadata.clone()));
        Ok(metadata)
    }

    pub fn copy_path(&self, app: &AppHandle, id: &str) -> AppResult<()> {
        let project = self.project(id)?;
        app.clipboard()
            .write_text(project.path)
            .map_err(|e| AppError::new("clipboardFailed", e.to_string()))
    }

    pub fn launch(
        &self,
        app: &AppHandle,
        id: &str,
        target: LaunchTarget,
    ) -> AppResult<LaunchResult> {
        {
            let mut pending = self.launches.lock().unwrap();
            if !pending.insert(id.into()) {
                return Err(AppError::new("launchPending", ""));
            }
        }
        let result = (|| {
            let project = self.project(id)?;
            let path = paths::directory(&project.path)?;
            let (settings, startup) = {
                let inner = self.inner.lock().unwrap();
                (
                    inner.user.settings.clone(),
                    inner.user.vscode_startup_for(id).clone(),
                )
            };
            let mut warnings = Vec::new();
            match target {
                LaunchTarget::Vscode => {
                    warnings = vscode_startup::open(
                        app,
                        &self.startup_directory,
                        &path,
                        &settings,
                        &startup,
                    )?
                }
                LaunchTarget::Terminal => launcher::open_terminal(&path, &settings)?,
                LaunchTarget::Explorer => app
                    .opener()
                    .open_path(&project.path, None::<&str>)
                    .map_err(|e| AppError::new("launchFailed", e.to_string()))?,
                LaunchTarget::Repository => {
                    let url = self
                        .git_metadata(id)?
                        .repository_url
                        .ok_or_else(|| AppError::new("repositoryUnavailable", ""))?;
                    app.opener()
                        .open_url(url, None::<&str>)
                        .map_err(|e| AppError::new("launchFailed", e.to_string()))?;
                }
            }
            if matches!(target, LaunchTarget::Vscode) {
                match self.mutate(app, false, |user, _| {
                    user.recent.insert(id.into(), now_ms());
                    Ok(())
                }) {
                    Ok(snapshot) => Ok(LaunchResult {
                        snapshot: Some(snapshot),
                        warnings,
                    }),
                    Err(error) => {
                        warnings.push(AppError::new(
                            "recentSaveFailed",
                            error.detail.unwrap_or_default(),
                        ));
                        Ok(LaunchResult {
                            snapshot: None,
                            warnings,
                        })
                    }
                }
            } else {
                Ok(LaunchResult {
                    snapshot: None,
                    warnings,
                })
            }
        })();
        self.launches.lock().unwrap().remove(id);
        result
    }
}

fn add_warning(inner: &mut Inner, warning: AppError) {
    if !inner.warnings.iter().any(|w| w.code == warning.code) {
        inner.warnings.push(warning);
    }
}

fn below(path: &str, ancestor: &str) -> bool {
    let path = paths::identity(path);
    let ancestor = paths::identity(ancestor);
    path == ancestor || path.starts_with(&(ancestor + "\\"))
}

fn finish_root(
    index: &mut IndexCache,
    root: &CodeRoot,
    seen: &HashSet<String>,
    issues: &[ScanIssue],
) {
    for project in index
        .projects
        .values_mut()
        .filter(|p| p.root_ids.contains(&root.id) && !seen.contains(&p.id))
    {
        if let Some(issue) = issues.iter().find(|i| below(&project.path, &i.path)) {
            project.availability = if issue.code == "directoryMissing" {
                Availability::Missing
            } else {
                Availability::Unknown
            };
        } else {
            match std::fs::metadata(&project.path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    project.availability = Availability::Missing
                }
                Err(_) => project.availability = Availability::Unknown,
                Ok(_) => {
                    project.root_ids.remove(&root.id);
                }
            }
        }
    }
}

pub fn request_scan(app: &AppHandle) {
    app.state::<AppState>().git_cache.lock().unwrap().clear();
    {
        let state = app.state::<AppState>();
        let mut inner = state.inner.lock().unwrap();
        if inner.scan.running {
            inner.pending = true;
            return;
        }
        inner.scan.running = true;
        inner.revision += 1;
    }
    let app = app.clone();
    std::thread::spawn(move || scan_worker(app));
}

fn scan_worker(app: AppHandle) {
    let state = app.state::<AppState>();
    loop {
        let (epoch, user) = {
            let mut inner = state.inner.lock().unwrap();
            inner.pending = false;
            inner.scan = ScanStatus {
                running: true,
                ..Default::default()
            };
            inner.revision += 1;
            let _ = app.emit("index-updated", state.snapshot_locked(&inner));
            (inner.epoch, inner.user.clone())
        };
        let mut last_emit = Instant::now();
        for root in &user.roots {
            let mut seen = HashSet::new();
            let mut issues = Vec::new();
            let completed = discovery::scan(root, user.settings.scan_depth, |event| {
                let mut inner = state.inner.lock().unwrap();
                if inner.epoch != epoch {
                    return false;
                }
                match event {
                    ScanEvent::Visited => inner.scan.visited += 1,
                    ScanEvent::Project(project) => {
                        seen.insert(project.id.clone());
                        inner.scan.discovered += 1;
                        merge(&mut inner.index, project);
                    }
                    ScanEvent::Issue(issue) => {
                        issues.push(issue.clone());
                        if inner.scan.issues.len() < 100 {
                            inner.scan.issues.push(issue);
                        }
                    }
                }
                if last_emit.elapsed() >= Duration::from_millis(120) {
                    inner.revision += 1;
                    let _ = app.emit("index-updated", state.snapshot_locked(&inner));
                    last_emit = Instant::now();
                }
                true
            });
            let mut inner = state.inner.lock().unwrap();
            if !completed || inner.epoch != epoch {
                break;
            }
            finish_root(&mut inner.index, root, &seen, &issues);
        }
        for path in &user.manual_projects {
            let result = discovery::manual(path);
            let mut inner = state.inner.lock().unwrap();
            if inner.epoch != epoch {
                break;
            }
            match result {
                Ok(project) => merge(&mut inner.index, project),
                Err(e) => {
                    if let Some(project) = inner.index.projects.get_mut(&paths::identity(path)) {
                        project.availability = if e.kind() == std::io::ErrorKind::NotFound {
                            Availability::Missing
                        } else {
                            Availability::Unknown
                        };
                    }
                }
            }
        }
        let mut inner = state.inner.lock().unwrap();
        if inner.epoch == epoch {
            let current_user = inner.user.clone();
            reconcile(&current_user, &mut inner.index);
            if let Err(e) = inner.storage.save_cache(&inner.index) {
                add_warning(&mut inner, e);
            }
        }
        if inner.pending || inner.epoch != epoch {
            drop(inner);
            continue;
        }
        inner.scan.running = false;
        inner.revision += 1;
        let _ = app.emit("index-updated", state.snapshot_locked(&inner));
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converting_a_manual_container_to_a_root_preserves_user_metadata_and_sources() {
        let path = r"D:\代码\My Code";
        let id = paths::identity(path);
        let mut user = UserData::default();
        user.manual_projects
            .extend([path.into(), r"D:\other\manual".into()]);
        user.favorites.insert(id.clone());
        user.recent.insert(id.clone(), 123);
        user.category_overrides
            .insert(id.clone(), "Projects".into());
        promote_manual_source(&mut user, path.into());
        assert_eq!(user.roots.len(), 1);
        assert_eq!(user.manual_projects, vec![r"D:\other\manual"]);
        assert!(user.favorites.contains(&id));
        assert_eq!(user.recent[&id], 123);
        assert_eq!(user.category_overrides[&id], "Projects");
        let root_id = user.roots[0].id.clone();
        promote_manual_source(&mut user, r"d:\代码\my code".into());
        assert_eq!(user.roots.len(), 1);
        assert_eq!(user.roots[0].id, root_id);
        let mut index = IndexCache::default();
        index.projects.insert(
            id.clone(),
            ProjectRecord {
                id: id.clone(),
                path: path.into(),
                name: "My Code".into(),
                tags: vec![],
                is_git: false,
                root_ids: BTreeSet::new(),
                manual: true,
                availability: Availability::Available,
            },
        );
        reconcile(&user, &mut index);
        assert!(!index.projects.contains_key(&id));
    }
    #[test]
    fn root_sources_manual_and_user_metadata_survive_reconciliation() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("go.mod"), "module example").unwrap();
        let mut record = discovery::manual(&temp.path().to_string_lossy()).unwrap();
        record.root_ids.extend(["removed".into(), "kept".into()]);
        let mut user = UserData::default();
        user.roots.push(CodeRoot {
            id: "kept".into(),
            path: temp.path().to_string_lossy().into_owned(),
        });
        user.favorites.insert(record.id.clone());
        user.recent.insert(record.id.clone(), 123);
        let mut index = IndexCache::default();
        index.projects.insert(record.id.clone(), record.clone());
        reconcile(&user, &mut index);
        assert_eq!(index.projects[&record.id].root_ids.len(), 1);
        user.roots.clear();
        user.manual_projects.push(record.path.clone());
        reconcile(&user, &mut index);
        assert!(index.projects[&record.id].manual);
        assert!(user.favorites.contains(&record.id));
        assert_eq!(user.recent[&record.id], 123);
    }
    #[test]
    fn missing_root_and_unreadable_subtree_preserve_cached_projects() {
        let root = CodeRoot {
            id: "r".into(),
            path: r"D:\code".into(),
        };
        let record = ProjectRecord {
            id: paths::identity(r"D:\code\apps\project"),
            path: r"D:\code\apps\project".into(),
            name: "project".into(),
            tags: vec![],
            is_git: false,
            root_ids: BTreeSet::from(["r".into()]),
            manual: false,
            availability: Availability::Available,
        };
        let mut index = IndexCache::default();
        index.projects.insert(record.id.clone(), record.clone());
        finish_root(
            &mut index,
            &root,
            &HashSet::new(),
            &[ScanIssue {
                path: root.path.clone(),
                code: "directoryUnreadable".into(),
            }],
        );
        assert_eq!(
            index.projects[&record.id].availability,
            Availability::Unknown
        );
        finish_root(
            &mut index,
            &root,
            &HashSet::new(),
            &[ScanIssue {
                path: root.path.clone(),
                code: "directoryMissing".into(),
            }],
        );
        assert_eq!(
            index.projects[&record.id].availability,
            Availability::Missing
        );
    }
}
