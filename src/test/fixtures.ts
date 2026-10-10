import type { EditorStatus, Project } from '../models';
export const fixtureProject: Project = { id: 'project', name: '项目 & (test); $', path: 'D:/My Code/项目 & (test); $', tags: [], isGit: false, rootIds: ['root'], manual: false, availability: 'available', category: null, categoryOverride: false, favorite: false, lastOpenedAt: null, vscodeStartup: { kind: 'default' }, editorId: null };
export const fixtureEditors: EditorStatus[] = [
  { id: 'vscode', displayName: 'Visual Studio Code', configured: true, available: true, executablePath: 'C:/Code.exe', capabilities: { projectOpen: true, specificFile: true, startupHelper: true, gitGraph: true }, error: null },
  { id: 'cursor', displayName: 'Cursor', configured: true, available: true, executablePath: 'C:/Cursor.exe', capabilities: { projectOpen: true, specificFile: false, startupHelper: false, gitGraph: false }, error: null },
];
