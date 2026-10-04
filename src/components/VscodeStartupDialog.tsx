import { useState } from 'react';
import { api } from '../api';
import type { AppSnapshot, Project, VscodeStartup } from '../models';
import { errorText, type Translate } from '../i18n';
import { Dialog } from './Dialog';

export function VscodeStartupDialog({ project, t, onClose, update }: {
  project: Project; t: Translate; onClose: () => void; update: (snapshot: AppSnapshot) => void;
}) {
  const [kind, setKind] = useState(project.vscodeStartup.kind);
  const [path, setPath] = useState(project.vscodeStartup.kind === 'file' ? project.vscodeStartup.path : '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const browse = async () => {
    setBusy(true); setError(null);
    try { const file = await api.pickProjectFile(project.id); if (file !== null) setPath(file); }
    catch (e) { setError(e); } finally { setBusy(false); }
  };
  const save = async () => {
    setBusy(true); setError(null);
    const startup: VscodeStartup = kind === 'file' ? { kind, path: path.trim() } : { kind };
    try { update(await api.vscodeStartup(project.id, startup)); onClose(); }
    catch (e) { setError(e); } finally { setBusy(false); }
  };
  return <Dialog title={t('vscodeStartup')} t={t} onClose={() => { if (!busy) onClose(); }}>
    <form onSubmit={e => { e.preventDefault(); void save(); }}>
      <div className="dialog-body startup-body">
        <p className="dialog-project">{project.name}</p>
        <label htmlFor="startup-kind">{t('startupContent')}</label>
        <select id="startup-kind" autoFocus value={kind} disabled={busy} onChange={e => { setKind(e.target.value as VscodeStartup['kind']); setError(null); }}>
          <option value="default">{t('startupDefault')}</option>
          <option value="file">{t('startupFile')}</option>
          <option value="gitGraph">Git Graph</option>
        </select>
        {kind === 'default' && <p>{t('startupDefaultHint')}</p>}
        {kind === 'file' && <>
          <label htmlFor="startup-file">{t('startupFilePath')}</label>
          <div className="input-actions"><input id="startup-file" placeholder="index.html" required value={path} disabled={busy} onChange={e => setPath(e.target.value)} /><button type="button" className="secondary-button" disabled={busy} onClick={() => { void browse(); }}>{t('browse')}</button></div>
          <p>{t('startupFileHint')}</p>
        </>}
        {kind === 'gitGraph' && <p>{t('startupGitGraphHint')}</p>}
      </div>
      <div className="dialog-footer">
        {error != null && <p className="form-error" role="alert">{errorText(error, t)}</p>}
        <button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{t('cancel')}</button>
        <button className="primary-button" disabled={busy}>{t('save')}</button>
      </div>
    </form>
  </Dialog>;
}
