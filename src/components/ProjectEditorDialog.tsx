import { useState } from 'react';
import { api } from '../api';
import type { AppSnapshot, EditorStatus, Project } from '../models';
import { errorText, type Translate } from '../i18n';
import { Dialog } from './Dialog';

export function ProjectEditorDialog({ project, editors, defaultId, t, update, onClose }: {
  project: Project; editors: EditorStatus[]; defaultId: string; t: Translate; update: (snapshot: AppSnapshot) => void; onClose: () => void;
}) {
  const [id, setId] = useState(project.editorId ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const configured = editors.filter(editor => editor.configured);
  return <Dialog title={t('projectEditor')} t={t} onClose={() => { if (!busy) onClose(); }}><form onSubmit={event => {
    event.preventDefault(); if (busy) return; setBusy(true);
    void api.projectEditor(project.id, id || null).then(next => { update(next); onClose(); }).catch(setError).finally(() => setBusy(false));
  }}><div className="dialog-body"><p className="dialog-project">{project.name}</p><label htmlFor="project-editor">{t('projectEditor')}</label><select id="project-editor" autoFocus disabled={busy} value={id} onChange={event => setId(event.target.value)}><option value="">{t('startupInherit')} · {editors.find(editor => editor.id === defaultId)?.displayName ?? defaultId}</option>{id && !configured.some(editor => editor.id === id) && <option value={id}>{id} · {t('editorNotFound')}</option>}{configured.map(editor => <option key={editor.id} value={editor.id}>{editor.displayName}{editor.available ? '' : ` · ${t('editorNotFound')}`}</option>)}</select><p>{t('editorInheritance')}</p></div><div className="dialog-footer">{error !== null && <p role="alert" className="form-error">{errorText(error, t)}</p>}<button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{t('cancel')}</button><button className="primary-button" disabled={busy}>{t('save')}</button></div></form></Dialog>;
}
