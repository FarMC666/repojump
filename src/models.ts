export interface AppError { code: string; detail?: string | null }
export interface Settings {
  theme: 'dark' | 'light' | 'system';
  language: 'en' | 'zh-CN' | 'system';
  scanDepth: number;
  vscodePath: string | null;
  terminal: 'auto' | 'powershell' | 'windowsTerminal';
  globalShortcut: string | null;
  closeToTray: boolean;
  dataLocation: string | null;
}
export interface CodeRoot { id: string; path: string }
export interface Project {
  id: string; name: string; path: string; tags: string[]; isGit: boolean;
  rootIds: string[]; manual: boolean; availability: 'available' | 'missing' | 'unknown';
  category: string | null; categoryOverride: boolean; favorite: boolean; lastOpenedAt: number | null;
}
export interface AppSnapshot {
  revision: number; roots: CodeRoot[]; projects: Project[]; settings: Settings;
  scan: { running: boolean; visited: number; discovered: number; issues: { path: string; code: string }[] };
  warnings: AppError[]; dataDirectory: string; storageReadOnly: boolean;
}
export interface GitMetadata { branch: string | null; dirty: boolean | null; repositoryUrl: string | null }
export type LaunchTarget = 'vscode' | 'terminal' | 'explorer' | 'repository';
export type View = { kind: 'all' | 'favorites' | 'recent' } | { kind: 'category'; category: string | null };
export interface LaunchResult { snapshot: AppSnapshot | null; warning: AppError | null }

export const defaultSettings: Settings = {
  theme: 'dark', language: 'system', scanDepth: 4, vscodePath: null,
  terminal: 'auto', globalShortcut: 'Ctrl+Alt+P', closeToTray: true, dataLocation: null,
};
