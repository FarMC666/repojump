import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ArrowUpRight, Check, ChevronRight, Clock3, Code2, Copy, Folder, FolderCode, FolderOpen, FolderPlus, GitBranch, Globe, Loader2, MoreHorizontal, Plus, RefreshCw, Search, Settings2, Star, Terminal, X } from 'lucide-react';
import { api, isDesktop } from './api';
import { defaultSettings, type AppSnapshot, type Project, type View, type GitMetadata, type LaunchTarget } from './models';
import { displayCategory, searchProjects } from './search';
import { errorText, locale, translator } from './i18n';
import { Dialog } from './components/Dialog';
import { SettingsDialog } from './components/SettingsDialog';
import { VscodeStartupDialog } from './components/VscodeStartupDialog';

type Confirmation = { title: string; body: string; path: string; run: () => Promise<unknown> };

export function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [loadError, setLoadError] = useState<unknown>(null);
  const [retry, setRetry] = useState(0);
  const [query, setQuery] = useState('');
  const [view, setView] = useState<View>({ kind: 'all' });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [quick, setQuick] = useState(false);
  const [menu, setMenu] = useState<{ project: Project; x: number; y: number } | null>(null);
  const [categoryEdit, setCategoryEdit] = useState<Project | null>(null);
  const [startupEdit, setStartupEdit] = useState<Project | null>(null);
  const [categoryDraft, setCategoryDraft] = useState('');
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [folderChoice, setFolderChoice] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [launching, setLaunching] = useState<string | null>(null);
  const [toast, setToast] = useState<{ text: string; error: boolean } | null>(null);
  const [gitInfo, setGitInfo] = useState<{ id: string; metadata: GitMetadata } | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const composing = useRef(false);
  const launchLock = useRef(false);
  const lastLaunch = useRef({ key: '', time: 0 });
  const settings = snapshot?.settings ?? defaultSettings;
  const language = locale(settings);
  const t = useMemo(() => translator(language), [language]);
  const report = useCallback((error: unknown) => setToast({ text: errorText(error, t), error: true }), [t]);
  const update = useCallback((next: AppSnapshot) => setSnapshot(previous => !previous || next.revision >= previous.revision ? next : previous), []);
  const results = useMemo(() => searchProjects(snapshot?.projects ?? [], query, view), [snapshot?.projects, query, view]);
  const selected = results.find(project => project.id === selectedId) ?? results[0];
  const selectedIndex = selected ? results.indexOf(selected) : -1;
  const metadata = selected && gitInfo?.id === selected.id ? gitInfo.metadata : null;
  const categories = useMemo(() => Array.from(new Set((snapshot?.projects ?? []).map(p => p.category))).sort((a, b) => (a ?? '').localeCompare(b ?? '')), [snapshot?.projects]);
  const categoryName = (category: string | null) => displayCategory(category, t('uncategorized'));
  const viewTitle = view.kind === 'category' ? categoryName(view.category) : t(view.kind);

  useEffect(() => {
    let cancelled = false;
    const cleanups: (() => void)[] = [];
    if (!isDesktop()) return;
    void (async () => {
      try {
        const stopUpdates = await api.subscribe(next => { if (!cancelled) update(next); });
        if (cancelled) { stopUpdates(); return; } cleanups.push(stopUpdates);
        const stopFocus = await api.onFocus(isQuick => {
          setQuick(isQuick); setQuery(''); setView({ kind: 'all' }); setMenu(null);
          setSettingsOpen(false); setConfirmation(null); setCategoryEdit(null); setStartupEdit(null); setFolderChoice(null);
          requestAnimationFrame(() => searchRef.current?.focus());
        });
        if (cancelled) { stopFocus(); return; } cleanups.push(stopFocus);
        const initial = await api.bootstrap();
        if (!cancelled) { update(initial); setLoadError(null); searchRef.current?.focus(); }
      } catch (e) { if (!cancelled) setLoadError(e); }
    })();
    return () => { cancelled = true; cleanups.forEach(cleanup => cleanup()); };
  }, [retry, update]);

  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)');
    const apply = () => { document.documentElement.dataset.theme = settings.theme === 'system' ? media.matches ? 'dark' : 'light' : settings.theme; };
    apply(); media.addEventListener('change', apply); return () => media.removeEventListener('change', apply);
  }, [settings.theme]);
  useEffect(() => { document.documentElement.lang = language; }, [language]);
  useEffect(() => { if (!toast) return; const timer = setTimeout(() => setToast(null), toast.error ? 9000 : 2400); return () => clearTimeout(timer); }, [toast]);
  useEffect(() => {
    if (!selected?.isGit) return;
    let cancelled = false;
    const timer = setTimeout(() => { void api.git(selected.id).then(data => { if (!cancelled) setGitInfo({ id: selected.id, metadata: data }); }).catch(() => {}); }, 120);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [selected?.id, selected?.isGit, snapshot?.scan.running]);
  useEffect(() => { document.getElementById(`project-${selectedIndex}`)?.scrollIntoView({ block: 'nearest' }); }, [selectedIndex]);
  useEffect(() => {
    if (!menu) return;
    menuRef.current?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    const dismiss = (event: PointerEvent) => { if (!menuRef.current?.contains(event.target as Node)) setMenu(null); };
    window.addEventListener('pointerdown', dismiss); return () => window.removeEventListener('pointerdown', dismiss);
  }, [menu]);

  const run = async (operation: () => Promise<AppSnapshot | void>) => {
    try { const next = await operation(); if (next) update(next); } catch (e) { report(e); }
  };
  const updateDiscovery = (next: AppSnapshot) => {
    update(next); setView({ kind: 'all' }); setQuery(''); setSelectedId(null);
    requestAnimationFrame(() => searchRef.current?.focus());
  };
  const scanFolder = async (path: string) => {
    const key = (value: string) => value.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase();
    if (snapshot?.roots.some(root => key(root.path) === key(path))) {
      await api.rescan(); return api.bootstrap();
    }
    return api.addRoot(path);
  };
  const addRoot = async () => {
    setPending(true);
    try { const path = await api.pickRoot(); if (typeof path === 'string') updateDiscovery(await scanFolder(path)); }
    catch (e) { report(e); } finally { setPending(false); }
  };
  const addProject = async () => {
    setPending(true);
    try {
      const path = await api.pickDirectory();
      if (typeof path === 'string') {
        if (await api.inspectDirectory(path)) updateDiscovery(await api.addProject(path));
        else setFolderChoice(path);
      }
    } catch (e) { report(e); } finally { setPending(false); }
  };
  const addChosenFolder = async (asRoot: boolean) => {
    if (!folderChoice) return;
    setPending(true);
    try { updateDiscovery(await (asRoot ? scanFolder(folderChoice) : api.addProject(folderChoice))); setFolderChoice(null); }
    catch (e) { report(e); } finally { setPending(false); }
  };
  const launch = useCallback(async (project: Project, target: LaunchTarget = 'vscode') => {
    if (launchLock.current) return;
    const key = `${target}:${project.id}`;
    if (lastLaunch.current.key === key && Date.now() - lastLaunch.current.time < 500) return;
    lastLaunch.current = { key, time: Date.now() };
    launchLock.current = true; setLaunching(project.id); setMenu(null);
    try {
      const result = await api.launch(project.id, target);
      if (result.snapshot) update(result.snapshot);
      if (result.warnings.length) setToast({ text: result.warnings.map(warning => errorText(warning, t)).join(' '), error: true });
      else if (target === 'vscode' && quick) { await api.hide(); setQuick(false); }
    } catch (e) { report(e); }
    finally { launchLock.current = false; setLaunching(null); }
  }, [quick, update, report, t]);

  useEffect(() => {
    const keyboard = (event: KeyboardEvent) => {
      if (event.isComposing || event.keyCode === 229 || composing.current) return;
      if (event.key === 'Escape') {
        if (menu) { event.preventDefault(); setMenu(null); searchRef.current?.focus(); }
        else if (!settingsOpen && !categoryEdit && !startupEdit && !confirmation && !folderChoice) {
          if (query) setQuery(''); else if (quick) { void api.hide(); setQuick(false); }
        }
        return;
      }
      if (settingsOpen || categoryEdit || startupEdit || confirmation || folderChoice || menu) return;
      if (event.ctrlKey && event.key.toLowerCase() === 'k') { event.preventDefault(); searchRef.current?.focus(); searchRef.current?.select(); return; }
      const target = event.target as HTMLElement;
      if (target !== searchRef.current && target.closest('input, select, textarea, button')) return;
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        const next = Math.max(0, Math.min(results.length - 1, selectedIndex + (event.key === 'ArrowDown' ? 1 : -1)));
        setSelectedId(results[next]?.id ?? null);
      } else if (event.key === 'Enter' && selected && !event.repeat) { event.preventDefault(); void launch(selected); }
    };
    window.addEventListener('keydown', keyboard); return () => window.removeEventListener('keydown', keyboard);
  }, [results, selected, selectedIndex, menu, query, quick, settingsOpen, categoryEdit, startupEdit, confirmation, folderChoice, launch]);

  const chooseView = (next: View) => { setView(next); setSelectedId(null); setQuery(''); searchRef.current?.focus(); };
  const confirmRoot = (id: string) => {
    const path = snapshot?.roots.find(root => root.id === id)?.path ?? '';
    setConfirmation({ title: t('removeRoot'), body: t('removeRootBody'), path, run: () => api.removeRoot(id).then(updateDiscovery) });
  };
  const confirm = async () => {
    if (!confirmation) return;
    setPending(true); try { await confirmation.run(); setConfirmation(null); } catch (e) { report(e); } finally { setPending(false); }
  };
  const showMenu = (project: Project, x: number, y: number) => {
    setSelectedId(project.id); setMenu({ project, x: Math.max(8, Math.min(x, window.innerWidth - 254)), y: Math.max(8, Math.min(y, window.innerHeight - 410)) });
  };
  const emptyKey = query ? 'noResults' : view.kind === 'favorites' ? 'noFavorites' : view.kind === 'recent' ? 'noRecent' : !snapshot?.roots.length && !snapshot?.projects.length ? 'welcome' : 'noProjects';

  return <div className="app-shell">
    <header className="app-header"><div className="brand"><span className="brand-mark"><ArrowUpRight size={21} strokeWidth={2.8} /></span><span>RepoJump</span>{quick && <span className="quick-label">{t('quick')}</span>}</div>
      <div className="header-actions"><button className="primary-button" title={t('addRootHint')} onClick={() => { void addRoot(); }} disabled={!snapshot || snapshot.storageReadOnly || pending}><FolderPlus size={16} />{t('addRoot')}</button><button className="secondary-button" title={t('addProjectHint')} onClick={() => { void addProject(); }} disabled={!snapshot || snapshot.storageReadOnly || pending}><Plus size={16} />{t('addProject')}</button><button className="icon-button" aria-label={t('settings')} title={t('settings')} disabled={!snapshot || pending} onClick={() => setSettingsOpen(true)}><Settings2 size={19} /></button></div>
    </header>
    <div className="search-area"><Search size={20} /><input ref={searchRef} id="project-search" type="search" autoComplete="off" spellCheck={false} aria-label={t('search')} placeholder={t('search')} value={query} onChange={e => { setQuery(e.target.value); setSelectedId(null); }} onCompositionStart={() => { composing.current = true; }} onCompositionEnd={() => { composing.current = false; }} aria-controls="project-list" aria-activedescendant={selectedIndex >= 0 ? `project-${selectedIndex}` : undefined} />{query ? <button className="icon-button" aria-label={t('close')} onClick={() => { setQuery(''); searchRef.current?.focus(); }}><X size={16} /></button> : <kbd>Ctrl K</kbd>}</div>
    <div className="workspace">
      <aside className="sidebar"><nav aria-label={t('all')}>
        {([{ kind: 'all' }, { kind: 'favorites' }, { kind: 'recent' }] as const).map(item => {
          const Icon = item.kind === 'all' ? FolderCode : item.kind === 'favorites' ? Star : Clock3;
          const count = snapshot?.projects.filter(p => item.kind === 'all' || (item.kind === 'favorites' ? p.favorite : p.lastOpenedAt !== null)).length ?? 0;
          return <button key={item.kind} className={`nav-item ${view.kind === item.kind ? 'active' : ''}`} aria-current={view.kind === item.kind ? 'page' : undefined} onClick={() => chooseView(item)}><Icon size={17} /><span>{t(item.kind)}</span><span className="nav-count">{count}</span></button>;
        })}
        <div className="nav-label">{t('categories')}</div>{categories.map(category => <button key={category ?? '__uncategorized'} className={`nav-item ${view.kind === 'category' && view.category === category ? 'active' : ''}`} onClick={() => chooseView({ kind: 'category', category })}><Folder size={16} /><span>{categoryName(category)}</span><span className="nav-count">{snapshot!.projects.filter(p => p.category === category).length}</span></button>)}
      </nav><div className="sidebar-bottom"><button className="nav-item" title={t('addRootHint')} onClick={() => { void addRoot(); }} disabled={!snapshot || snapshot.storageReadOnly || pending}><FolderPlus size={17} /><span>{t('addRoot')}</span></button><div className="sidebar-version">LOCAL · v0.1.1</div></div>
      </aside>
      <main className="project-main">
        <div className="list-toolbar"><div><h1>{query ? t('searching') : viewTitle}</h1><span>{results.length} {t('projects')}</span></div><button className="text-button" disabled={!snapshot} onClick={() => { void run(api.rescan); }}><RefreshCw size={14} className={snapshot?.scan.running ? 'spinning' : ''} />{t('rescan')}</button></div>
        {(snapshot?.warnings.length ?? 0) > 0 && <div className="warning-banner" role="status">{snapshot!.warnings.map((warning, i) => <details key={i}><summary>{errorText(warning, t)}</summary><p>{warning.detail}</p></details>)}</div>}
        {(snapshot?.scan.issues.length ?? 0) > 0 && <div className="warning-banner"><details><summary>{t('scanIssues')} ({snapshot!.scan.issues.length})</summary>{snapshot!.scan.issues.map((issue, i) => <p key={i}>{errorText(issue, t)} · {issue.path}</p>)}</details></div>}
        <div id="project-list" className="project-list" role="listbox" aria-label={viewTitle}>
          {!isDesktop() ? <div className="empty-state"><Code2 size={38} /><h2>RepoJump</h2><p>{t('desktopOnly')}</p></div> : !snapshot ? <div className="empty-state">{loadError ? <><h2>{errorText(loadError, t)}</h2><button className="secondary-button" onClick={() => setRetry(previous => previous + 1)}>{t('retry')}</button></> : <><Loader2 className="spinning" size={26} /><p>{t('loading')}</p></>}</div> : !results.length ? <div className="empty-state"><span className="empty-symbol"><FolderCode size={34} /></span><h2>{t(emptyKey)}</h2><p>{t(`${emptyKey}Body`)}</p>{emptyKey === 'welcome' && <button className="primary-button" onClick={() => { void addRoot(); }}><FolderPlus size={17} />{t('addRoot')}</button>}{emptyKey === 'noProjects' && <button className="secondary-button" onClick={() => { void addProject(); }}><Plus size={16} />{t('addProject')}</button>}</div> : results.map((project, index) => <div id={`project-${index}`} data-project-id={project.id} role="option" aria-selected={project.id === selected?.id} key={project.id} className={`project-row ${project.id === selected?.id ? 'selected' : ''} ${project.availability !== 'available' ? 'unavailable' : ''}`} onClick={() => { setSelectedId(project.id); searchRef.current?.focus(); }} onDoubleClick={() => { void launch(project); }} onContextMenu={e => { e.preventDefault(); showMenu(project, e.clientX, e.clientY); }}>
            <div className="project-symbol"><FolderCode size={21} /></div><div className="project-content"><div className="project-title"><span>{project.name}</span>{project.isGit && <GitBranch size={13} aria-label={t('git')} />}{project.manual && <span className="manual-badge">{t('manual')}</span>}{project.availability !== 'available' && <span className="unavailable-badge">{t(project.availability === 'missing' ? 'missing' : 'unknown')}</span>}</div><div className="project-tags">{project.tags.length ? project.tags.map(tag => <span key={tag}>{tag}</span>) : <span>—</span>}<span className="category-tag">{categoryName(project.category)}</span></div><div className="project-path" title={project.path}>{project.path}</div></div>
            <div className="project-actions"><button className={`icon-button star-button ${project.favorite ? 'starred' : ''}`} aria-label={`${t(project.favorite ? 'unfavorite' : 'favorite')} ${project.name}`} title={t(project.favorite ? 'unfavorite' : 'favorite')} disabled={snapshot.storageReadOnly} onClick={e => { e.stopPropagation(); void run(() => api.favorite(project.id, !project.favorite)); }}><Star size={17} fill={project.favorite ? 'currentColor' : 'none'} /></button><button className="row-open" aria-label={`${t('openCode')} ${project.name}`} title={t('openCode')} disabled={launching !== null || project.availability === 'missing'} onClick={e => { e.stopPropagation(); void launch(project); }}>{launching === project.id ? <Loader2 className="spinning" size={16} /> : <ArrowUpRight size={17} />}</button><button className="icon-button" aria-label={`${t('more')} ${project.name}`} onClick={e => { e.stopPropagation(); const rect = e.currentTarget.getBoundingClientRect(); showMenu(project, rect.right - 242, rect.bottom + 5); }}><MoreHorizontal size={18} /></button></div>
          </div>)}
        </div>
        {selected && <div className="selection-details"><div className="selection-path"><Folder size={14} /><span title={selected.path}>{selected.path}</span><button className="icon-button" aria-label={t('copyPath')} onClick={() => { void api.copy(selected.id).then(() => setToast({ text: t('copied'), error: false })).catch(report); }}><Copy size={14} /></button></div><div className="selection-meta">{selected.isGit && <span><GitBranch size={13} />{metadata?.branch ?? t('gitUnknown')}{metadata?.dirty !== null && metadata && <span className={metadata.dirty ? 'dirty-dot' : 'clean-dot'} title={t(metadata.dirty ? 'dirty' : 'clean')} aria-label={t(metadata.dirty ? 'dirty' : 'clean')} />}</span>}<span>{t('lastOpened')}: {selected.lastOpenedAt ? new Intl.DateTimeFormat(language, { dateStyle: 'medium', timeStyle: 'short' }).format(selected.lastOpenedAt) : t('never')}</span></div></div>}
      </main>
    </div>
    <footer className="status-bar"><span className="status"><span className={snapshot?.scan.running ? 'status-dot busy' : 'status-dot'} />{snapshot?.scan.running ? `${t('scanning')} · ${snapshot.scan.visited} ${t('visited')}` : t('ready')}</span><div className="keyboard-hints"><span><kbd>↑</kbd><kbd>↓</kbd>{t('navigateHint')}</span><span><kbd>Enter</kbd>{t('enterHint')}</span><span><kbd>Ctrl K</kbd>{t('searchHint')}</span></div></footer>
    {menu && <div ref={menuRef} className="project-menu" role="menu" style={{ left: menu.x, top: menu.y }} onKeyDown={e => {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); const buttons = Array.from(menuRef.current!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')); const index = buttons.indexOf(document.activeElement as HTMLButtonElement); buttons[(index + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]?.focus(); }
    }}>
      {([{ target: 'vscode', label: 'openCode', Icon: Code2 }, { target: 'terminal', label: 'openTerminal', Icon: Terminal }, { target: 'explorer', label: 'openExplorer', Icon: FolderOpen }, { target: 'repository', label: 'openRepository', Icon: Globe }] as const).map(({ target, label, Icon }) => <button role="menuitem" key={target} disabled={menu.project.availability === 'missing' || (target === 'repository' && !(gitInfo?.id === menu.project.id && gitInfo.metadata.repositoryUrl))} onClick={() => { void launch(menu.project, target); }}><Icon size={16} />{t(label)}{target === 'vscode' && <kbd>Enter</kbd>}</button>)}
      <button role="menuitem" onClick={() => { void api.copy(menu.project.id).then(() => setToast({ text: t('copied'), error: false })).catch(report); setMenu(null); }}><Copy size={16} />{t('copyPath')}</button><hr />
      <button role="menuitem" disabled={snapshot?.storageReadOnly} onClick={() => { void run(() => api.favorite(menu.project.id, !menu.project.favorite)); setMenu(null); }}><Star size={16} />{t(menu.project.favorite ? 'unfavorite' : 'favorite')}</button>
      <button role="menuitem" disabled={snapshot?.storageReadOnly} onClick={() => { setCategoryEdit(menu.project); setCategoryDraft(menu.project.category ?? ''); setMenu(null); }}><Folder size={16} />{t('editCategory')}<ChevronRight size={14} /></button>
      <button role="menuitem" disabled={snapshot?.storageReadOnly} onClick={() => { setStartupEdit(menu.project); setMenu(null); }}><Code2 size={16} />{t('vscodeStartup')}<ChevronRight size={14} /></button>
      {menu.project.categoryOverride && <button role="menuitem" onClick={() => { void run(() => api.category(menu.project.id, null)); setMenu(null); }}><RefreshCw size={16} />{t('autoCategory')}</button>}
      {menu.project.manual && <><hr />{!menu.project.isGit && menu.project.tags.length === 0 && <button role="menuitem" disabled={snapshot?.storageReadOnly || menu.project.availability === 'missing'} onClick={() => { const project = menu.project; setMenu(null); void run(() => api.scanDirectory(project.id).then(next => { updateDiscovery(next); })); }}><FolderPlus size={16} />{t('scanDirectory')}</button>}<button role="menuitem" className="danger" disabled={snapshot?.storageReadOnly} onClick={() => { const project = menu.project; setConfirmation({ title: t('removeManual'), body: t('removeManualBody'), path: project.path, run: () => api.removeProject(project.id).then(update) }); setMenu(null); }}><X size={16} />{t('removeManual')}</button></>}
    </div>}
    {settingsOpen && snapshot && <SettingsDialog snapshot={snapshot} t={t} onClose={() => setSettingsOpen(false)} update={update} updateRoots={updateDiscovery} addRoot={addRoot} removeRoot={confirmRoot} report={report} />}
    {startupEdit && <VscodeStartupDialog project={startupEdit} t={t} update={update} onClose={() => { setStartupEdit(null); requestAnimationFrame(() => searchRef.current?.focus()); }} />}
    {folderChoice && <Dialog title={t('folderChoice')} t={t} onClose={() => { if (!pending) setFolderChoice(null); }}><form onSubmit={e => { e.preventDefault(); void addChosenFolder(true); }}><div className="dialog-body"><p>{t('folderChoiceBody')}</p><code className="data-path">{folderChoice}</code></div><div className="dialog-footer"><button type="button" className="secondary-button" disabled={pending} onClick={() => { void addChosenFolder(false); }}>{t('addOnlyFolder')}</button><button className="primary-button" disabled={pending}>{t('scanDirectory')}</button></div></form></Dialog>}
    {categoryEdit && <Dialog title={t('editCategory')} t={t} onClose={() => { if (!pending) setCategoryEdit(null); }}><form onSubmit={async e => { e.preventDefault(); setPending(true); try { update(await api.category(categoryEdit.id, categoryDraft)); setCategoryEdit(null); } catch (error) { report(error); } finally { setPending(false); } }}><div className="dialog-body"><p className="dialog-project">{categoryEdit.name}</p><label htmlFor="category-name">{t('category')}</label><input id="category-name" autoFocus maxLength={64} required value={categoryDraft} onChange={e => setCategoryDraft(e.target.value)} /></div><div className="dialog-footer"><button type="button" className="secondary-button" disabled={pending} onClick={() => setCategoryEdit(null)}>{t('cancel')}</button><button className="primary-button" disabled={pending}>{t('save')}</button></div></form></Dialog>}
    {confirmation && <Dialog title={confirmation.title} t={t} onClose={() => { if (!pending) setConfirmation(null); }}><div className="dialog-body"><p>{confirmation.body}</p><code className="data-path">{confirmation.path}</code></div><div className="dialog-footer"><button type="button" className="secondary-button" disabled={pending} onClick={() => setConfirmation(null)}>{t('cancel')}</button><button className="danger-button" disabled={pending} onClick={() => { void confirm(); }}>{t('remove')}</button></div></Dialog>}
    {toast && createPortal(<div className={`toast ${toast.error ? 'error' : ''}`} role={toast.error ? 'alert' : 'status'}>{toast.error ? <X size={16} /> : <Check size={16} />}<span>{toast.text}</span><button className="icon-button" aria-label={t('close')} onClick={() => setToast(null)}><X size={14} /></button></div>, Array.from(document.querySelectorAll('dialog[open]')).at(-1) ?? document.body)}
  </div>;
}
