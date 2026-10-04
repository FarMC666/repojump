import { useState, type KeyboardEvent } from 'react';
import { FolderPlus, Pencil, Trash2 } from 'lucide-react';
import { api } from '../api';
import type { AppSnapshot, Settings } from '../models';
import type { Translate } from '../i18n';
import { errorText } from '../i18n';
import { Dialog } from './Dialog';

interface Props {
  snapshot: AppSnapshot; t: Translate; onClose: () => void;
  update: (snapshot: AppSnapshot) => void; addRoot: () => Promise<void>;
  removeRoot: (id: string) => void; report: (error: unknown) => void;
}

export function SettingsDialog({ snapshot, t, onClose, update, addRoot, removeRoot, report }: Props) {
  const [draft, setDraft] = useState<Settings>({ ...snapshot.settings });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [recording, setRecording] = useState(false);
  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => setDraft(previous => ({ ...previous, [key]: value }));
  const save = async () => {
    setBusy(true); setError(null);
    try { update(await api.settings({ ...draft, vscodePath: draft.vscodePath?.trim() || null, globalShortcut: draft.globalShortcut?.trim() || null })); onClose(); }
    catch (e) { setError(e); } finally { setBusy(false); }
  };
  const record = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!recording) return;
    e.preventDefault(); e.stopPropagation();
    if (e.key === 'Escape') { setRecording(false); return; }
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return;
    if (!e.ctrlKey && !e.altKey && !e.metaKey) return;
    const key = e.code.startsWith('Key') ? e.code.slice(3) : e.code.startsWith('Digit') ? e.code.slice(5) : e.key;
    set('globalShortcut', [...(e.ctrlKey ? ['Ctrl'] : []), ...(e.altKey ? ['Alt'] : []), ...(e.shiftKey ? ['Shift'] : []), ...(e.metaKey ? ['Super'] : []), key].join('+'));
    setRecording(false);
  };
  return <Dialog wide title={t('settings')} onClose={() => { if (!busy) onClose(); }} t={t}>
    <form onSubmit={e => { e.preventDefault(); void save(); }}>
      <div className="dialog-body settings-body">
        <section><h3>{t('roots')}</h3><p>{t('rootsBody')}</p>
          <div className="root-list">{snapshot.roots.length === 0 && <p>{t('noRoots')}</p>}{snapshot.roots.map(root => <div className="root-row" key={root.id}>
            <span title={root.path}>{root.path}</span>
            <button type="button" className="icon-button" title={t('changeRoot')} aria-label={`${t('changeRoot')} ${root.path}`} onClick={async () => { try { const path = await api.pickDirectory(); if (typeof path === 'string') update(await api.updateRoot(root.id, path)); } catch (e) { report(e); } }}><Pencil size={15} /></button>
            <button type="button" className="icon-button danger" title={t('removeRoot')} aria-label={`${t('removeRoot')} ${root.path}`} onClick={() => removeRoot(root.id)}><Trash2 size={15} /></button>
          </div>)}</div>
          <button type="button" className="secondary-button" onClick={() => { void addRoot(); }} disabled={busy || snapshot.storageReadOnly}><FolderPlus size={16} />{t('addRoot')}</button>
        </section>
        <section><h3>{t('appearance')}</h3><div className="setting-grid">
          <label htmlFor="theme">{t('theme')}</label><select id="theme" value={draft.theme} onChange={e => set('theme', e.target.value as Settings['theme'])}><option value="dark">{t('dark')}</option><option value="light">{t('light')}</option><option value="system">{t('system')}</option></select>
          <label htmlFor="language">{t('language')}</label><select id="language" value={draft.language} onChange={e => set('language', e.target.value as Settings['language'])}><option value="system">{t('system')}</option><option value="zh-CN">简体中文</option><option value="en">English</option></select>
        </div></section>
        <section><h3>{t('discovery')}</h3><div className="setting-grid"><label htmlFor="depth">{t('scanDepth')}</label><input id="depth" type="number" min={1} max={8} required value={draft.scanDepth} onChange={e => set('scanDepth', Number(e.target.value))} /></div><p>{t('depthHint')}</p></section>
        <section><h3>{t('launching')}</h3><label htmlFor="code-path">{t('codePath')}</label><div className="input-actions"><input id="code-path" placeholder={t('autoDetect')} value={draft.vscodePath ?? ''} onChange={e => set('vscodePath', e.target.value || null)} /><button type="button" className="secondary-button" onClick={async () => { try { const path = await api.pickCode(); if (typeof path === 'string') set('vscodePath', path); } catch (e) { setError(e); } }}>{t('browse')}</button></div>
          {draft.vscodePath && <button type="button" className="text-button" onClick={() => set('vscodePath', null)}>{t('reset')}</button>}
          <div className="setting-grid terminal-setting"><label htmlFor="terminal">{t('terminal')}</label><select id="terminal" value={draft.terminal} onChange={e => set('terminal', e.target.value as Settings['terminal'])}><option value="auto">{t('terminalAuto')}</option><option value="windowsTerminal">Windows Terminal</option><option value="powershell">PowerShell</option></select></div>
        </section>
        <section><h3>{t('shortcut')}</h3><label className="check-label"><input type="checkbox" checked={draft.globalShortcut !== null} onChange={e => set('globalShortcut', e.target.checked ? 'Ctrl+Alt+P' : null)} />{t('shortcutEnabled')}</label>
          {draft.globalShortcut !== null && <div className="input-actions"><input id="shortcut" aria-label={t('shortcut')} value={draft.globalShortcut} placeholder={t('shortcutPlaceholder')} onChange={e => set('globalShortcut', e.target.value)} onKeyDown={record} /><button type="button" className="secondary-button" onClick={() => { setRecording(true); document.getElementById('shortcut')?.focus(); }}>{t(recording ? 'recording' : 'capture')}</button></div>}
          <p>{t('shortcutHint')}</p><label className="check-label"><input type="checkbox" checked={draft.closeToTray} onChange={e => set('closeToTray', e.target.checked)} />{t('closeToTray')}</label>
        </section>
        <section><h3>{t('localData')}</h3><p>{t('localDataHint')}</p><code className="data-path">{snapshot.dataDirectory}</code><p className="version">RepoJump 0.1.0</p></section>
      </div>
      <div className="dialog-footer">{error !== null && <p className="form-error" role="alert">{errorText(error, t)}</p>}<button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{t('cancel')}</button><button className="primary-button" disabled={busy || snapshot.storageReadOnly}>{t('save')}</button></div>
    </form>
  </Dialog>;
}
