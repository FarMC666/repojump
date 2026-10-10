import { useCallback, useRef, useState } from 'react';
import { api } from './api';
import { errorText, type Translate } from './i18n';
import type { AppSnapshot, EditorStatus, GitMetadata } from './models';
import { projectActions } from './projectActions';

export function useProjectActions(snapshot: AppSnapshot | null, editors: EditorStatus[], gitInfo: { id: string; metadata: GitMetadata } | null, quick: boolean,
  update: (snapshot: AppSnapshot) => void, report: (error: unknown) => void, toast: (text: string, error: boolean) => void, hideQuick: () => void, t: Translate) {
  const [launching, setLaunching] = useState<string | null>(null);
  const lock = useRef(false);
  const last = useRef({ key: '', time: 0 });
  const actionsFor = useCallback((id: string) => {
    const project = snapshot?.projects.find(project => project.id === id);
    return project && snapshot ? projectActions(project, snapshot.settings, editors, gitInfo?.id === id ? gitInfo.metadata : null, snapshot.storageReadOnly, launching !== null, t) : [];
  }, [snapshot, editors, gitInfo, launching, t]);
  const execute = useCallback(async (id: string, actionId: string) => {
    const project = snapshot?.projects.find(project => project.id === id);
    const action = actionsFor(id).find(action => action.id === actionId);
    if (!project || !action?.enabled) return;
    const intent = action.intent;
    try {
      if (intent.kind === 'copy') { await api.copy(id); toast(t('copied'), false); }
      else if (intent.kind === 'favorite') { update(await api.favorite(id, !project.favorite)); }
      else {
        if (lock.current) return;
        const key = `${id}:${actionId}`;
        if (last.current.key === key && Date.now() - last.current.time < 500) return;
        last.current = { key, time: Date.now() }; lock.current = true; setLaunching(id);
        try {
          const result = await api.launch(id, intent.target, intent.editorId);
          if (result.snapshot) update(result.snapshot);
          const hasWarning = result.warnings.some(warning => warning.code !== 'editorStartupUnsupported');
          if (result.warnings.length) toast(result.warnings.map(warning => errorText(warning, t)).join(' '), hasWarning);
          if (!hasWarning && intent.target === 'editor' && quick) { await api.hide(); hideQuick(); }
        } finally { lock.current = false; setLaunching(null); }
      }
    } catch (error) { report(error); }
  }, [snapshot, actionsFor, update, report, toast, quick, hideQuick, t]);
  return { actionsFor, execute, launching };
}
