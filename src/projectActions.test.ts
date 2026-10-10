import { describe, expect, it } from 'vitest';
import { projectActions } from './projectActions';
import { defaultSettings } from './models';
import { translator } from './i18n';

import { fixtureEditors, fixtureProject } from './test/fixtures';
const t = translator('en');
describe('shared project actions', () => {
  it('resolves inheritance and overrides; Open With is a temporary explicit editor', () => {
    const settings = { ...defaultSettings, defaultEditorId: 'cursor' };
    const inherited = projectActions(fixtureProject, settings, fixtureEditors, null, false, false, t);
    expect(inherited[0].label).toContain('Cursor');
    expect(inherited.find(action => action.id === 'editor:vscode')?.intent).toEqual({ kind: 'launch', target: 'editor', editorId: 'vscode' });
    const overridden = projectActions({ ...fixtureProject, editorId: 'vscode' }, settings, fixtureEditors, null, false, false, t);
    expect(overridden[0].label).toContain('Visual Studio Code');
    expect(overridden.find(action => action.id === 'editor:cursor')).toBeDefined();
  });
  it('keeps copy and favorite usable for missing projects, but disables native launches', () => {
    const actions = projectActions({ ...fixtureProject, availability: 'missing' }, defaultSettings, fixtureEditors, { branch: null, dirty: null, repositoryUrl: 'https://example.com/repo' }, false, false, t);
    expect(actions.filter(action => action.enabled).map(action => action.id)).toEqual(['copy', 'favorite']);
  });
  it('allows unavailable roots to be retried and respects read-only storage', () => {
    const actions = projectActions({ ...fixtureProject, availability: 'unknown', favorite: true }, defaultSettings, fixtureEditors, null, true, false, t);
    expect(actions[0].enabled).toBe(true);
    expect(actions.find(action => action.id === 'favorite')).toMatchObject({ label: 'Unfavorite', enabled: false });
    expect(actions.find(action => action.id === 'repository')?.enabled).toBe(false);
  });
  it('only offers configured editors and disables invalid executables', () => {
    const editors = [...fixtureEditors.map(editor => ({ ...editor, available: false })), { ...fixtureEditors[1], id: 'windsurf', configured: false }];
    const actions = projectActions(fixtureProject, defaultSettings, editors, null, false, false, t);
    expect(actions.find(action => action.id === 'editor:cursor')?.enabled).toBe(false);
    expect(actions.find(action => action.id === 'editor:windsurf')).toBeUndefined();
    // Default open is retryable: Rust provides the precise error if detection changes.
    expect(actions[0].enabled).toBe(true);
  });
  it('disables launches during a pending launch without blocking copy', () => {
    const actions = projectActions(fixtureProject, defaultSettings, fixtureEditors, null, false, true, t);
    expect(actions[0].enabled).toBe(false);
    expect(actions.find(action => action.id === 'copy')?.enabled).toBe(true);
  });
});
