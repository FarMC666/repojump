use crate::{
    discovery::{self, ScanEvent},
    model::*,
    paths,
    service::{self, AppState},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

pub fn below(path: &str, ancestor: &str) -> bool {
    let path = paths::identity(path);
    let ancestor = paths::identity(ancestor);
    path == ancestor || path.starts_with(&(ancestor + "\\"))
}

#[derive(Default)]
pub struct WorkQueue {
    pub full: bool,
    pub scopes: BTreeMap<String, BTreeSet<PathBuf>>,
    pub manuals: BTreeSet<String>,
}

impl WorkQueue {
    pub fn full(&mut self) {
        self.full = true;
        self.scopes.clear();
        self.manuals.clear();
    }
    pub fn scope(&mut self, root: &CodeRoot, path: PathBuf) {
        if self.full {
            return;
        }
        let scopes = self.scopes.entry(root.id.clone()).or_default();
        if scopes
            .iter()
            .any(|p| below(&path.to_string_lossy(), &p.to_string_lossy()))
        {
            return;
        }
        scopes.retain(|p| !below(&p.to_string_lossy(), &path.to_string_lossy()));
        scopes.insert(path);
        if scopes.len() > 128 {
            scopes.clear();
            scopes.insert(PathBuf::from(&root.path));
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.full && self.scopes.is_empty() && self.manuals.is_empty()
    }
}

/// Build removal decisions without holding the service mutex or overwriting user metadata.
pub fn missing_records(
    index: &IndexCache,
    root: &CodeRoot,
    scope: &Path,
    seen: &HashSet<String>,
    issues: &[ScanIssue],
) -> Vec<(String, Option<Availability>)> {
    index
        .projects
        .values()
        .filter(|p| {
            p.root_ids.contains(&root.id)
                && below(&p.path, &scope.to_string_lossy())
                && !seen.contains(&p.id)
        })
        .map(|p| {
            let availability = if let Some(issue) = issues.iter().find(|i| below(&p.path, &i.path))
            {
                Some(if issue.code == "directoryMissing" {
                    Availability::Missing
                } else {
                    Availability::Unknown
                })
            } else {
                match paths::safe_scan_directory(Path::new(&root.path), Path::new(&p.path)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        Some(Availability::Missing)
                    }
                    Err(_) => Some(Availability::Unknown),
                    Ok(()) => None,
                }
            };
            (p.id.clone(), availability)
        })
        .collect()
}

pub fn apply_missing(
    index: &mut IndexCache,
    root: &CodeRoot,
    decisions: Vec<(String, Option<Availability>)>,
) {
    for (id, availability) in decisions {
        if let Some(p) = index.projects.get_mut(&id) {
            if let Some(availability) = availability {
                p.availability = availability;
            } else {
                p.root_ids.remove(&root.id);
            }
        }
    }
}

pub fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    let mut cache_dirty = false;
    let mut last_save = Instant::now() - Duration::from_secs(2);
    loop {
        let (epoch, user, work, before) = {
            let mut inner = state.inner.lock().unwrap();
            let work = std::mem::take(&mut inner.work);
            inner.scan = ScanStatus {
                running: true,
                ..Default::default()
            };
            inner.revision += 1;
            let _ = app.emit("index-updated", state.snapshot_locked(&inner));
            (
                inner.epoch,
                inner.user.clone(),
                work,
                inner.index.projects.clone(),
            )
        };
        state.watcher.reset_epoch(epoch);
        let mut last_emit = Instant::now();
        for root in &user.roots {
            let scopes = if work.full {
                BTreeSet::from([PathBuf::from(&root.path)])
            } else {
                work.scopes.get(&root.id).cloned().unwrap_or_default()
            };
            for scope in scopes {
                let Ok(relative) = scope.strip_prefix(&root.path) else {
                    continue;
                };
                let depth = relative.components().count();
                if depth > usize::from(user.settings.scan_depth) {
                    continue;
                }
                let mut seen = HashSet::new();
                let mut visited = BTreeSet::new();
                let mut issues = Vec::new();
                let completed = discovery::scan_scope(
                    root,
                    &scope,
                    depth as u8,
                    user.settings.scan_depth,
                    |event| {
                        if let ScanEvent::Visited(path) = &event {
                            if paths::safe_scan_directory(Path::new(&root.path), path).is_ok() {
                                state.watcher.watch(path);
                                visited.insert(path.clone());
                            }
                        }
                        let mut inner = state.inner.lock().unwrap();
                        if inner.epoch != epoch || state.watcher.stopped() {
                            return false;
                        }
                        match event {
                            ScanEvent::Visited(_) => inner.scan.visited += 1,
                            ScanEvent::Project(project) => {
                                seen.insert(project.id.clone());
                                inner.scan.discovered += 1;
                                service::merge(&mut inner.index, project);
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
                    },
                );
                let index = {
                    let inner = state.inner.lock().unwrap();
                    if !completed || inner.epoch != epoch {
                        break;
                    }
                    inner.index.clone()
                };
                let decisions = missing_records(&index, root, &scope, &seen, &issues);
                {
                    let mut inner = state.inner.lock().unwrap();
                    if inner.epoch != epoch {
                        break;
                    }
                    apply_missing(&mut inner.index, root, decisions);
                }
                state
                    .watcher
                    .finish_scope(&root.id, &scope, visited, &issues, epoch);
            }
        }
        for path in &user.manual_projects {
            if !work.full && !work.manuals.contains(path) {
                continue;
            }
            if paths::safe_scan_directory(Path::new(path), Path::new(path)).is_ok() {
                state.watcher.watch(Path::new(path));
            }
            let result = discovery::manual(path);
            let mut inner = state.inner.lock().unwrap();
            if inner.epoch != epoch {
                break;
            }
            match result {
                Ok(project) => service::merge(&mut inner.index, project),
                Err(e) => {
                    if let Some(p) = inner.index.projects.get_mut(&paths::identity(path)) {
                        p.availability = if e.kind() == std::io::ErrorKind::NotFound {
                            Availability::Missing
                        } else {
                            Availability::Unknown
                        };
                    }
                }
            }
        }
        state.watcher.keep_manuals(&user.manual_projects, epoch);
        let mut inner = state.inner.lock().unwrap();
        if inner.epoch == epoch {
            let user = inner.user.clone();
            service::reconcile(&user, &mut inner.index);
            cache_dirty |= work.full || inner.index.projects != before;
        } else {
            inner.work.full();
        }
        if cache_dirty && (inner.work.is_empty() || last_save.elapsed() >= Duration::from_secs(2)) {
            match inner.storage.save_cache(&inner.index) {
                Err(e) => service::add_warning(&mut inner, e),
                Ok(()) => inner
                    .warnings
                    .retain(|warning| warning.code != "cacheFailure"),
            }
            cache_dirty = false;
            last_save = Instant::now();
        }
        if !inner.work.is_empty() && !state.watcher.stopped() {
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
    fn queue_coalesces_scopes_and_bounds_storms() {
        let root = CodeRoot {
            id: "r".into(),
            path: "D:/code".into(),
        };
        let mut queue = WorkQueue::default();
        for i in 0..1000 {
            queue.scope(&root, PathBuf::from(format!("D:/code/p{i}")));
        }
        assert_eq!(
            queue.scopes["r"],
            BTreeSet::from([PathBuf::from("D:/code")])
        );
        queue.full();
        queue.scope(&root, PathBuf::from("D:/code/new"));
        assert!(queue.scopes.is_empty());
    }
    #[test]
    fn partial_removal_preserves_other_scopes_manuals_and_missing_records() {
        let temp = tempfile::tempdir().unwrap();
        let root = CodeRoot {
            id: "r".into(),
            path: temp.path().to_string_lossy().into(),
        };
        let mut index = IndexCache::default();
        for name in ["changed", "unaffected", "missing"] {
            let path = temp.path().join(name);
            std::fs::create_dir(&path).unwrap();
            let mut p = discovery::manual(&path.to_string_lossy()).unwrap();
            p.root_ids.extend(["r".into(), "other".into()]);
            index.projects.insert(p.id.clone(), p);
        }
        std::fs::remove_dir(temp.path().join("missing")).unwrap();
        let decisions = missing_records(
            &index,
            &root,
            &temp.path().join("changed"),
            &HashSet::new(),
            &[],
        );
        apply_missing(&mut index, &root, decisions);
        let changed =
            &index.projects[&paths::identity(&temp.path().join("changed").to_string_lossy())];
        assert!(!changed.root_ids.contains("r"));
        assert!(changed.manual && changed.root_ids.contains("other"));
        assert!(index.projects
            [&paths::identity(&temp.path().join("unaffected").to_string_lossy())]
            .root_ids
            .contains("r"));
        let decisions = missing_records(
            &index,
            &root,
            &temp.path().join("missing"),
            &HashSet::new(),
            &[],
        );
        apply_missing(&mut index, &root, decisions);
        assert_eq!(
            index.projects[&paths::identity(&temp.path().join("missing").to_string_lossy())]
                .availability,
            Availability::Missing
        );
    }
}
