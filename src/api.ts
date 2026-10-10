import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AppSnapshot, EditorStatus, GitMetadata, LaunchResult, LaunchTarget, Settings, VscodeStartup } from './models';

export const isDesktop = () => '__TAURI_INTERNALS__' in window;
export const api = {
  bootstrap: () => invoke<AppSnapshot>('bootstrap'),
  rescan: () => invoke<void>('rescan'),
  addRoot: (path: string) => invoke<AppSnapshot>('add_root', { path }),
  updateRoot: (id: string, path: string) => invoke<AppSnapshot>('update_root', { id, path }),
  removeRoot: (id: string) => invoke<AppSnapshot>('remove_root', { id }),
  addProject: (path: string) => invoke<AppSnapshot>('add_manual_project', { path }),
  inspectDirectory: (path: string) => invoke<boolean>('inspect_directory', { path }),
  scanDirectory: (id: string) => invoke<AppSnapshot>('scan_manual_directory', { id }),
  removeProject: (id: string) => invoke<AppSnapshot>('remove_manual_project', { id }),
  favorite: (id: string, favorite: boolean) => invoke<AppSnapshot>('set_favorite', { id, favorite }),
  category: (id: string, category: string | null) => invoke<AppSnapshot>('set_category_override', { id, category }),
  vscodeStartup: (id: string, startup: VscodeStartup) => invoke<AppSnapshot>('set_vscode_startup', { id, startup }),
  editors: () => invoke<EditorStatus[]>('get_editors'),
  projectEditor: (id: string, editorId: string | null) => invoke<AppSnapshot>('set_project_editor', { id, editorId }),
  pickProjectFile: (id: string) => invoke<string | null>('pick_project_file', { id }),
  settings: (settings: Settings) => invoke<AppSnapshot>('update_settings', { settings }),
  appearance: (appearance: Pick<Settings, 'theme' | 'language'>) => invoke<AppSnapshot>('update_appearance', appearance),
  syncTrayLanguage: (language: 'en' | 'zh-CN') => invoke<void>('sync_tray_language', { language }),
  git: (id: string) => invoke<GitMetadata>('get_git_metadata', { id }),
  launch: (id: string, target: LaunchTarget, editorId?: string) => invoke<LaunchResult>('launch_project', { id, target, editorId }),
  copy: (id: string) => invoke<void>('copy_project_path', { id }),
  pickDirectory: () => invoke<string | null>('pick_path', { kind: 'directory' }),
  pickRoot: () => invoke<string | null>('pick_path', { kind: 'codeRoot' }),
  pickDataLocation: () => invoke<string | null>('pick_path', { kind: 'dataLocation' }),
  pickCode: () => invoke<string | null>('pick_path', { kind: 'code' }),
  subscribe: (update: (snapshot: AppSnapshot) => void) => listen<AppSnapshot>('index-updated', e => update(e.payload)),
  onFocus: (focus: (quick: boolean) => void) => listen<boolean>('launcher-focus', e => focus(e.payload)),
  hide: () => getCurrentWindow().hide(),
};
