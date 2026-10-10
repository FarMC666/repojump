export interface AppError { code: string; detail?: string | null }
export interface Settings {
  theme: 'dark' | 'light' | 'system';
  language: 'en' | 'zh-CN' | 'system';
  scanDepth: number;
  defaultEditorId: string;
  editorProfiles: Record<string, { executablePath: string | null }>;
  defaultVscodeStartup: VscodeStartup;
  terminal: 'auto' | 'powershell' | 'windowsTerminal' | 'cmd';
  globalShortcut: string | null;
  closeToTray: boolean;
  dataLocation: string | null;
}
export interface CodeRoot { id: string; path: string }
export type VscodeStartup = { kind: 'default' } | { kind: 'file'; path: string } | { kind: 'gitGraph' };
export interface Project {
  id: string; name: string; path: string; tags: string[]; isGit: boolean;
  rootIds: string[]; manual: boolean; availability: 'available' | 'missing' | 'unknown';
  category: string | null; categoryOverride: boolean; favorite: boolean; lastOpenedAt: number | null;
  vscodeStartup: VscodeStartup;
  editorId: string | null;
}
export interface AppSnapshot {
  revision: number; roots: CodeRoot[]; projects: Project[]; settings: Settings;
  scan: { running: boolean; visited: number; discovered: number; issues: { path: string; code: string }[] };
  warnings: AppError[]; dataDirectory: string; storageReadOnly: boolean;
}
export interface GitMetadata { branch: string | null; dirty: boolean | null; repositoryUrl: string | null }
export interface EditorCapabilities { projectOpen: boolean; specificFile: boolean; startupHelper: boolean; gitGraph: boolean }
export interface EditorStatus { id: string; displayName: string; configured: boolean; executablePath: string | null; available: boolean; capabilities: EditorCapabilities; error: AppError | null }
export type LaunchTarget = 'editor' | 'vscode' | 'terminal' | 'explorer' | 'repository';
export type View = { kind: 'all' | 'favorites' | 'recent' } | { kind: 'category'; category: string | null };
export interface LaunchResult { snapshot: AppSnapshot | null; warnings: AppError[] }

export const defaultSettings: Settings = {
  theme: 'dark', language: 'system', scanDepth: 4, defaultEditorId: 'vscode', editorProfiles: { vscode: { executablePath: null } },
  defaultVscodeStartup: { kind: 'default' },
  terminal: 'auto', globalShortcut: 'Ctrl+Alt+P', closeToTray: true, dataLocation: null,
};
