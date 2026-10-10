import type { EditorStatus, Settings } from '../models';
import { api } from '../api';
import type { Translate } from '../i18n';

export function EditorSettings({ draft, editors, setDraft, busy, report, t }: {
  draft: Settings; editors: EditorStatus[]; setDraft: (draft: Settings) => void; busy: boolean; report: (error: unknown) => void; t: Translate;
}) {
  const setPath = (id: string, path: string | null) => setDraft({ ...draft, editorProfiles: { ...draft.editorProfiles, [id]: { executablePath: path } } });
  return <><label htmlFor="default-editor">{t('defaultEditor')}</label><select id="default-editor" disabled={busy} value={draft.defaultEditorId} onChange={event => setDraft({ ...draft, defaultEditorId: event.target.value })}>
    {Object.keys(draft.editorProfiles).map(id => <option key={id} value={id}>{editors.find(editor => editor.id === id)?.displayName ?? id}</option>)}
  </select><p>{t('editorInheritance')}</p>
    {editors.map(editor => {
      const configured = editor.id in draft.editorProfiles;
      return <div key={editor.id} className="editor-profile"><label className="check-label"><input type="checkbox" disabled={busy || editor.id === draft.defaultEditorId} checked={configured} onChange={event => {
        const profiles = { ...draft.editorProfiles };
        if (event.target.checked) profiles[editor.id] = { executablePath: null }; else delete profiles[editor.id];
        setDraft({ ...draft, editorProfiles: profiles });
      }} />{editor.displayName}</label>
        {configured && <><label htmlFor={`editor-path-${editor.id}`}>{t('editorExecutable')}</label><div className="input-actions"><input id={`editor-path-${editor.id}`} disabled={busy} value={draft.editorProfiles[editor.id]?.executablePath ?? ''} placeholder={t('autoDetect')} onChange={event => setPath(editor.id, event.target.value || null)} /><button type="button" className="secondary-button" disabled={busy} onClick={() => { void api.pickCode().then(path => { if (path !== null) setPath(editor.id, path); }).catch(report); }}>{t('browse')}</button></div>
          <p>{t('detectedExecutable')}: {editor.executablePath ?? t('editorNotFound')}</p>
          {draft.editorProfiles[editor.id]?.executablePath && <button type="button" className="text-button" disabled={busy} onClick={() => setPath(editor.id, null)}>{t('autoDetect')}</button>}
          {!editor.capabilities.startupHelper && <p>{t(editor.capabilities.specificFile ? 'editorFileOnly' : 'editorBasicOnly')}</p>}
        </>}
      </div>;
    })}
  </>;
}
