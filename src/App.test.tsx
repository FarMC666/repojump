// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { api } from './api';
import { defaultSettings, type AppSnapshot } from './models';
import { fixtureEditors, fixtureProject } from './test/fixtures';

vi.mock('./api', () => ({ isDesktop: () => true, api: { bootstrap: vi.fn(), editors: vi.fn(), settings: vi.fn(), subscribe: vi.fn(), onFocus: vi.fn(), syncTrayLanguage: vi.fn(), launch: vi.fn(), favorite: vi.fn(), copy: vi.fn(), hide: vi.fn() } }));
let root: Root;
let host: HTMLDivElement;
let focus: (quick: boolean) => void;
let state: AppSnapshot;
const search = () => document.getElementById('project-search') as HTMLInputElement;
async function key(key: string, options: KeyboardEventInit = {}) { await act(async () => { document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...options })); }); }
async function query(value: string) {
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(search(), value); search().dispatchEvent(new Event('input', { bubbles: true })); });
}
beforeEach(async () => {
  vi.clearAllMocks();
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true, matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }), requestAnimationFrame: (callback: FrameRequestCallback) => setTimeout(callback, 0) });
  Element.prototype.scrollIntoView = vi.fn();
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
  state = { revision: 1, roots: [], projects: [{ ...fixtureProject }], settings: { ...defaultSettings, language: 'en', editorProfiles: { vscode: { executablePath: null }, cursor: { executablePath: null } } }, scan: { running: false, visited: 0, discovered: 0, issues: [] }, warnings: [], dataDirectory: 'D:/test', storageReadOnly: false };
  vi.mocked(api.bootstrap).mockImplementation(async () => state);
  vi.mocked(api.editors).mockResolvedValue(fixtureEditors);
  vi.mocked(api.syncTrayLanguage).mockResolvedValue();
  vi.mocked(api.hide).mockResolvedValue();
  vi.mocked(api.copy).mockResolvedValue();
  vi.mocked(api.subscribe).mockResolvedValue(() => {});
  vi.mocked(api.onFocus).mockImplementation(async callback => { focus = callback; return () => {}; });
  vi.mocked(api.launch).mockResolvedValue({ snapshot: null, warnings: [] });
  vi.mocked(api.favorite).mockImplementation(async () => ({ ...state, revision: 2, projects: [{ ...fixtureProject, favorite: true }] }));
  vi.mocked(api.settings).mockImplementation(async settings => { state = { ...state, revision: state.revision + 1, settings }; return state; });
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => { root.render(<App />); });
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });
describe('keyboard-first project actions', () => {
  it('saves CMD independently of Windows Terminal and restores the selection on reopen', async () => {
    await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Settings"]')!.click());
    const terminal = document.getElementById('terminal') as HTMLSelectElement;
    expect(Array.from(terminal.options).find(option => option.value === 'windowsTerminal')?.text).toContain('default shell');
    await act(async () => { terminal.value = 'cmd'; terminal.dispatchEvent(new Event('change', { bubbles: true })); });
    await act(async () => document.querySelector('dialog form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
    expect(api.settings).toHaveBeenCalledWith(expect.objectContaining({ terminal: 'cmd', defaultEditorId: 'vscode', defaultVscodeStartup: { kind: 'default' } }));
    expect(document.querySelector('dialog')).toBeNull();
    await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Settings"]')!.click());
    expect((document.getElementById('terminal') as HTMLSelectElement).value).toBe('cmd');
  });
  it('Tab opens actions, skips disabled actions, Escape restores query, selection and caret', async () => {
    await query('项目'); search().setSelectionRange(1, 1);
    await key('Tab');
    const palette = document.querySelector('.palette-list')!;
    expect(document.activeElement).toBe(palette);
    await key('ArrowDown'); expect(palette.getAttribute('aria-activedescendant')).toBe('action-terminal');
    await key('ArrowDown'); await key('ArrowDown'); expect(palette.getAttribute('aria-activedescendant')).toBe('action-copy');
    await key('Escape');
    await act(async () => { await new Promise(resolve => setTimeout(resolve, 5)); });
    expect(document.querySelector('dialog')).toBeNull();
    expect(document.activeElement).toBe(search()); expect(search().value).toBe('项目'); expect(search().selectionStart).toBe(1);
    expect(document.querySelector('[role="option"]')?.getAttribute('aria-selected')).toBe('true');
  });
  it('Enter, quick shortcuts, menu and Open With execute the shared dispatcher', async () => {
    await key('Enter'); expect(api.launch).toHaveBeenLastCalledWith(fixtureProject.id, 'editor', undefined);
    await key('Enter', { ctrlKey: true }); expect(api.launch).toHaveBeenLastCalledWith(fixtureProject.id, 'terminal', undefined);
    await key('Enter', { altKey: true }); expect(api.launch).toHaveBeenLastCalledWith(fixtureProject.id, 'explorer', undefined);
    await key('Tab');
    await key('ArrowUp'); await key('Enter'); expect(api.launch).toHaveBeenLastCalledWith(fixtureProject.id, 'editor', 'cursor');
    await act(async () => { host.querySelector('.project-row')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true })); });
    const copy = Array.from(host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')).find(button => button.textContent === 'Copy path')!;
    await act(async () => copy.click()); expect(api.copy).toHaveBeenCalledWith(fixtureProject.id);
  });
  it('global shortcut activation closes dialogs and successful Open With hides quick mode', async () => {
    await key('Tab');
    await act(async () => { focus(true); await new Promise(resolve => setTimeout(resolve, 5)); });
    expect(document.querySelector('dialog')).toBeNull(); expect(document.activeElement).toBe(search());
    await key('Tab'); await key('ArrowUp'); await key('Enter');
    expect(api.launch).toHaveBeenLastCalledWith(fixtureProject.id, 'editor', 'cursor'); expect(api.hide).toHaveBeenCalledOnce();
  });
  it('composition, repeated Enter and Shift Tab preserve native input behavior', async () => {
    await key('Enter', { isComposing: true }); await key('Enter', { repeat: true }); await key('Tab', { shiftKey: true });
    expect(api.launch).not.toHaveBeenCalled(); expect(document.querySelector('dialog')).toBeNull();
  });
  it('expected startup degradation is an informational success and names the editor', async () => {
    vi.mocked(api.launch).mockResolvedValue({ snapshot: null, warnings: [{ code: 'editorStartupUnsupported', detail: 'Cursor' }] });
    await key('Tab'); await key('ArrowUp'); await key('Enter');
    expect(document.querySelector('.toast.error')).toBeNull();
    expect(document.querySelector('.toast[role="status"]')?.textContent).toContain('Opened in Cursor.');
    expect(document.querySelector('.toast')?.textContent).not.toContain('{editor}');
  });
  it.each([
    { warnings: [{ code: 'editorStartupUnsupported', detail: 'Cursor' }], hides: true },
    { warnings: [{ code: 'startupHelperFailed', detail: '' }], hides: false },
    { warnings: [{ code: 'editorStartupUnsupported', detail: 'Cursor' }, { code: 'recentSaveFailed', detail: '' }], hides: false },
  ])('quick launch hides only when warnings are expected capability degradation: $hides', async ({ warnings, hides }) => {
    vi.mocked(api.launch).mockResolvedValue({ snapshot: null, warnings });
    await act(async () => { focus(true); await new Promise(resolve => setTimeout(resolve, 5)); });
    await key('Tab'); await key('ArrowUp'); await key('Enter');
    expect(api.hide).toHaveBeenCalledTimes(hides ? 1 : 0);
    expect(!!document.querySelector('.toast.error')).toBe(!hides);
  });
});
