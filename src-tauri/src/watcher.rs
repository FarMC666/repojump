use crate::{
    detectors, discovery, indexer,
    model::ScanIssue,
    paths,
    service::{self, AppState},
};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

type Hint = Result<Event, notify::Error>;
struct Native {
    watcher: Option<RecommendedWatcher>,
    watched: BTreeSet<PathBuf>,
    anchors: BTreeSet<PathBuf>,
    sources: BTreeMap<String, BTreeSet<PathBuf>>,
    epoch: u64,
}

pub struct WatchManager {
    native: Mutex<Native>,
    receiver: Mutex<Option<Receiver<Hint>>>,
    sender: SyncSender<Hint>,
    overflow: std::sync::Arc<AtomicBool>,
    allowed: Arc<RwLock<BTreeSet<String>>>,
    stop: AtomicBool,
}

impl WatchManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(2048);
        let overflow = std::sync::Arc::new(AtomicBool::new(false));
        let allowed = Arc::new(RwLock::new(BTreeSet::new()));
        let watcher = create(tx.clone(), overflow.clone(), allowed.clone()).ok();
        Self {
            native: Mutex::new(Native {
                watcher,
                watched: BTreeSet::new(),
                anchors: BTreeSet::new(),
                sources: BTreeMap::new(),
                epoch: 0,
            }),
            receiver: Mutex::new(Some(rx)),
            sender: tx,
            overflow,
            allowed,
            stop: AtomicBool::new(false),
        }
    }
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.native.lock().unwrap().watcher.take();
    }
    pub fn reset_epoch(&self, epoch: u64) {
        let mut native = self.native.lock().unwrap();
        if native.epoch == epoch {
            return;
        }
        let old = std::mem::take(&mut native.anchors);
        if let Some(watcher) = &mut native.watcher {
            for path in old {
                let _ = watcher.unwatch(&path);
            }
        }
        native.sources.clear();
        native.watched.clear();
        self.allowed.write().unwrap().clear();
        native.epoch = epoch;
    }
    pub fn watch(&self, path: &Path) {
        if self.stopped() {
            return;
        }
        let mut native = self.native.lock().unwrap();
        if native.watched.contains(path) {
            return;
        }
        // Windows cannot rename an ancestor with open descendant directory handles.
        // Keep one native subtree handle per disjoint root, but accept hints only
        // from directories that discovery actually visited. No target is traversed here.
        if !native
            .anchors
            .iter()
            .any(|anchor| indexer::below(&path.to_string_lossy(), &anchor.to_string_lossy()))
        {
            let descendants: Vec<_> = native
                .anchors
                .iter()
                .filter(|anchor| indexer::below(&anchor.to_string_lossy(), &path.to_string_lossy()))
                .cloned()
                .collect();
            for old in descendants {
                if let Some(watcher) = &mut native.watcher {
                    let _ = watcher.unwatch(&old);
                }
                native.anchors.remove(&old);
            }
            if native
                .watcher
                .as_mut()
                .is_none_or(|watcher| watcher.watch(path, RecursiveMode::Recursive).is_err())
            {
                return;
            }
            native.anchors.insert(path.into());
        }
        native.watched.insert(path.into());
        self.allowed
            .write()
            .unwrap()
            .insert(paths::identity(&path.to_string_lossy()));
    }
    fn restart(&self) {
        let mut native = self.native.lock().unwrap();
        native.watcher.take();
        native.watched.clear();
        native.anchors.clear();
        self.allowed.write().unwrap().clear();
        if !self.stopped() {
            native.watcher = create(
                self.sender.clone(),
                self.overflow.clone(),
                self.allowed.clone(),
            )
            .ok();
        }
    }
    fn invalidate(&self, path: &Path) {
        let mut native = self.native.lock().unwrap();
        let removed: Vec<_> = native
            .watched
            .iter()
            .filter(|candidate| {
                indexer::below(&candidate.to_string_lossy(), &path.to_string_lossy())
            })
            .cloned()
            .collect();
        for candidate in removed {
            if native.anchors.remove(&candidate) {
                if let Some(watcher) = &mut native.watcher {
                    let _ = watcher.unwatch(&candidate);
                }
            }
            native.watched.remove(&candidate);
            self.allowed
                .write()
                .unwrap()
                .remove(&paths::identity(&candidate.to_string_lossy()));
        }
    }
    pub fn finish_scope(
        &self,
        source: &str,
        scope: &Path,
        paths: BTreeSet<PathBuf>,
        issues: &[ScanIssue],
        epoch: u64,
    ) {
        let mut native = self.native.lock().unwrap();
        if native.epoch != epoch {
            return;
        }
        let source = native.sources.entry(source.into()).or_default();
        source.retain(|p| {
            !indexer::below(&p.to_string_lossy(), &scope.to_string_lossy())
                || issues
                    .iter()
                    .any(|i| indexer::below(&p.to_string_lossy(), &i.path))
        });
        source.extend(paths);
        prune(&mut native, &self.allowed);
    }
    pub fn keep_manuals(&self, paths: &[String], epoch: u64) {
        let mut native = self.native.lock().unwrap();
        if native.epoch != epoch {
            return;
        }
        native
            .sources
            .insert("@manual".into(), paths.iter().map(PathBuf::from).collect());
        prune(&mut native, &self.allowed);
    }
    pub fn start(app: &AppHandle) {
        let rx = app
            .state::<AppState>()
            .watcher
            .receiver
            .lock()
            .unwrap()
            .take();
        if let Some(rx) = rx {
            let app = app.clone();
            std::thread::spawn(move || monitor(app, rx));
        }
    }
}

fn create(
    tx: SyncSender<Hint>,
    overflow: std::sync::Arc<AtomicBool>,
    allowed: Arc<RwLock<BTreeSet<String>>>,
) -> notify::Result<RecommendedWatcher> {
    RecommendedWatcher::new(
        move |mut event: Hint| {
            if let Ok(event) = &mut event {
                if !event.need_rescan() {
                    let allowed = allowed.read().unwrap();
                    event.paths.retain(|path| {
                        relevant(event.kind, path)
                            && (allowed.contains(&paths::identity(&path.to_string_lossy()))
                                || path.parent().is_some_and(|parent| {
                                    allowed.contains(&paths::identity(&parent.to_string_lossy()))
                                }))
                    });
                    if event.paths.is_empty() {
                        return;
                    }
                }
            }
            if tx.try_send(event).is_err() {
                overflow.store(true, Ordering::Relaxed);
            }
        },
        Config::default().with_follow_symlinks(false),
    )
}

fn prune(native: &mut Native, allowed: &RwLock<BTreeSet<String>>) {
    let desired: BTreeSet<_> = native.sources.values().flatten().cloned().collect();
    let remove: Vec<_> = native.watched.difference(&desired).cloned().collect();
    for path in remove {
        native.watched.remove(&path);
        allowed
            .write()
            .unwrap()
            .remove(&paths::identity(&path.to_string_lossy()));
    }
    let obsolete: Vec<_> = native
        .anchors
        .iter()
        .filter(|anchor| {
            !desired
                .iter()
                .any(|path| indexer::below(&path.to_string_lossy(), &anchor.to_string_lossy()))
        })
        .cloned()
        .collect();
    for anchor in obsolete {
        if let Some(watcher) = &mut native.watcher {
            let _ = watcher.unwatch(&anchor);
        }
        native.anchors.remove(&anchor);
    }
}

pub fn relevant(kind: EventKind, path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if discovery::ignored(&name) && !name.eq_ignore_ascii_case(".git") {
        return false;
    }
    if detectors::relevant_file(&name) {
        return !matches!(kind, EventKind::Access(_));
    }
    match kind {
        EventKind::Create(notify::event::CreateKind::File)
        | EventKind::Remove(notify::event::RemoveKind::File) => false,
        EventKind::Create(_)
        | EventKind::Remove(_)
        | EventKind::Modify(notify::event::ModifyKind::Name(_))
        | EventKind::Any
        | EventKind::Other => true,
        _ => false,
    }
}

fn monitor(app: AppHandle, rx: Receiver<Hint>) {
    let state = app.state::<AppState>();
    let mut dirty = BTreeSet::new();
    let mut first = None;
    let mut pending_overflow = false;
    let mut restart = false;
    let mut last = Instant::now();
    let mut health = Instant::now();
    while !state.watcher.stopped() {
        if let Ok(hint) = rx.recv_timeout(Duration::from_millis(100)) {
            match hint {
                Ok(event) if !event.need_rescan() => {
                    for path in &event.paths {
                        if matches!(
                            event.kind,
                            EventKind::Remove(_)
                                | EventKind::Modify(notify::event::ModifyKind::Name(_))
                        ) && !path.is_dir()
                        {
                            state.watcher.invalidate(path);
                        }
                        if relevant(event.kind, path) {
                            dirty.insert(path.clone());
                            if dirty.len() > 2048 {
                                dirty.clear();
                                state.watcher.overflow.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
                hint => {
                    restart |= hint.is_err();
                    state.watcher.overflow.store(true, Ordering::Relaxed);
                }
            }
            if !dirty.is_empty() {
                first.get_or_insert(Instant::now());
                last = Instant::now();
            }
        }
        let overflow = state.watcher.overflow.swap(false, Ordering::Relaxed);
        if overflow {
            pending_overflow = true;
            first.get_or_insert(Instant::now());
            last = Instant::now();
        }
        if first.is_some_and(|start: Instant| {
            last.elapsed() >= Duration::from_millis(300)
                || start.elapsed() >= Duration::from_secs(2)
        }) {
            if restart {
                state.watcher.restart();
                restart = false;
            }
            service::request_changes(&app, std::mem::take(&mut dirty), pending_overflow);
            pending_overflow = false;
            first = None;
        }
        if health.elapsed() >= Duration::from_secs(30) {
            health = Instant::now();
            if state.watcher.native.lock().unwrap().watcher.is_none() {
                state.watcher.restart();
            }
            let (roots, manuals) = {
                let inner = state.inner.lock().unwrap();
                (inner.user.roots.clone(), inner.user.manual_projects.clone())
            };
            let mut unavailable = Vec::new();
            let mut refresh = BTreeSet::new();
            let failed: Vec<_> = {
                let native = state.watcher.native.lock().unwrap();
                native
                    .sources
                    .values()
                    .flatten()
                    .filter(|path| !native.watched.contains(*path))
                    .cloned()
                    .collect()
            };
            for path in failed {
                unavailable.push(path.to_string_lossy().into_owned());
                refresh.insert(path);
            }
            for root in &roots {
                let path = Path::new(&root.path);
                let safe = paths::safe_scan_directory(path, path).is_ok();
                if !safe {
                    state.watcher.invalidate(path);
                }
                let watched = state.watcher.native.lock().unwrap().watched.contains(path);
                // UNC roots are reconciled periodically: native delivery is not guaranteed.
                if !safe || !watched || root.path.starts_with("\\\\") {
                    refresh.insert(path.to_path_buf());
                    if !safe || !watched {
                        unavailable.push(root.path.clone());
                    }
                }
            }
            for manual in manuals {
                let path = Path::new(&manual);
                if paths::safe_scan_directory(path, path).is_err() {
                    state.watcher.invalidate(path);
                }
                if !state.watcher.native.lock().unwrap().watched.contains(path) {
                    refresh.insert(path.into());
                    unavailable.push(manual);
                }
            }
            {
                let mut inner = state.inner.lock().unwrap();
                let next = if unavailable.is_empty() {
                    None
                } else {
                    Some(unavailable.join("\n"))
                };
                let previous = inner
                    .warnings
                    .iter()
                    .find(|e| e.code == "watcherUnavailable")
                    .and_then(|e| e.detail.clone());
                if next != previous {
                    inner.warnings.retain(|e| e.code != "watcherUnavailable");
                    if let Some(detail) = next {
                        service::add_warning(
                            &mut inner,
                            crate::model::AppError::new("watcherUnavailable", detail),
                        );
                    }
                    inner.revision += 1;
                    let _ = app.emit("index-updated", state.snapshot_locked(&inner));
                }
            }
            if !refresh.is_empty() {
                service::request_changes(&app, refresh, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restarting_and_source_changes_recreate_and_prune_handles() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("project");
        std::fs::create_dir(&nested).unwrap();
        let manager = WatchManager::new();
        manager.watch(temp.path());
        manager.watch(&nested);
        manager.finish_scope(
            "root",
            temp.path(),
            BTreeSet::from([temp.path().into(), nested.clone()]),
            &[],
            0,
        );
        assert_eq!(manager.native.lock().unwrap().watched.len(), 2);
        manager.invalidate(&nested);
        assert_eq!(manager.native.lock().unwrap().watched.len(), 1);
        manager.watch(&nested);
        manager.restart();
        assert!(manager.native.lock().unwrap().watched.is_empty());
        manager.watch(temp.path());
        manager.reset_epoch(1);
        assert!(manager.native.lock().unwrap().sources.is_empty());
        assert!(manager.native.lock().unwrap().watched.is_empty());
        manager.stop();
        manager.watch(temp.path());
        assert!(manager.native.lock().unwrap().watcher.is_none());
    }
    #[test]
    fn markers_and_tag_configs_but_not_generated_files_trigger_hints() {
        let event = Event::new(EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any,
        )));
        for name in [
            ".git",
            ".GIT",
            "package.json",
            "Cargo.toml",
            "app.slnx",
            "build.gradle.kts",
            "tsconfig.json",
            "vite.config.ts",
        ] {
            assert!(relevant(event.kind, Path::new(name)), "{name}");
        }
        for name in ["main.ts", "node_modules", "target", ".repojump", "dist"] {
            assert!(!relevant(event.kind, Path::new(name)));
        }
    }
    #[test]
    fn native_subtree_watch_filters_ignored_child_content_before_queueing() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("node_modules")).unwrap();
        let (tx, rx) = mpsc::sync_channel(64);
        let mut watcher = create(
            tx,
            std::sync::Arc::new(AtomicBool::new(false)),
            Arc::new(RwLock::new(BTreeSet::from([paths::identity(
                &temp.path().to_string_lossy(),
            )]))),
        )
        .unwrap();
        watcher
            .watch(temp.path(), RecursiveMode::NonRecursive)
            .unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while Instant::now() < deadline {
            if let Ok(Ok(event)) = rx.recv_timeout(Duration::from_millis(100)) {
                found |= event
                    .paths
                    .iter()
                    .any(|p| p.file_name().is_some_and(|n| n == "package.json"));
                if found {
                    break;
                }
            }
        }
        assert!(found);
        std::fs::write(temp.path().join("node_modules/ignored.json"), "{}").unwrap();
        while let Ok(Ok(event)) = rx.recv_timeout(Duration::from_millis(300)) {
            assert!(!event.paths.iter().any(|p| p.ends_with("ignored.json")));
        }
        watcher.unwatch(temp.path()).unwrap();
    }
    #[test]
    fn native_handles_do_not_block_parent_directory_rename() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let child = root.join("child");
        std::fs::create_dir_all(&child).unwrap();
        let manager = WatchManager::new();
        manager.watch(&root);
        manager.watch(&child);
        std::fs::rename(&root, temp.path().join("renamed")).unwrap();
        manager.stop();
    }
}
