import type { EditorStatus, GitMetadata, LaunchTarget, Project, Settings } from './models';
import type { Translate } from './i18n';

export type ActionIntent = { kind: 'launch'; target: LaunchTarget; editorId?: string } | { kind: 'copy' } | { kind: 'favorite' };
export interface ProjectAction { id: string; label: string; enabled: boolean; intent: ActionIntent; shortcut?: string }
export function projectActions(project: Project, settings: Settings, editors: EditorStatus[], metadata: GitMetadata | null, readOnly: boolean, launching: boolean, t: Translate): ProjectAction[] {
  const editorId = project.editorId ?? settings.defaultEditorId;
  const resolved = editors.find(editor => editor.id === editorId);
  const local = project.availability !== 'missing' && !launching;
  return [
    { id: 'open', label: `${t('openEditor')} · ${resolved?.displayName ?? editorId}`, enabled: local, intent: { kind: 'launch', target: 'editor' }, shortcut: 'Enter' },
    { id: 'terminal', label: t('openTerminal'), enabled: local, intent: { kind: 'launch', target: 'terminal' }, shortcut: 'Ctrl Enter' },
    { id: 'explorer', label: t('openExplorer'), enabled: local, intent: { kind: 'launch', target: 'explorer' }, shortcut: 'Alt Enter' },
    { id: 'repository', label: t('openRepository'), enabled: local && !!metadata?.repositoryUrl, intent: { kind: 'launch', target: 'repository' } },
    { id: 'copy', label: t('copyPath'), enabled: true, intent: { kind: 'copy' } },
    { id: 'favorite', label: t(project.favorite ? 'unfavorite' : 'favorite'), enabled: !readOnly, intent: { kind: 'favorite' } },
    ...editors.filter(editor => editor.configured && editor.capabilities.projectOpen && editor.id !== editorId).map(editor => ({
      id: `editor:${editor.id}`, label: `${t('openWith')} · ${editor.displayName}`, enabled: local && editor.available,
      intent: { kind: 'launch' as const, target: 'editor' as const, editorId: editor.id },
    })),
  ];
}
