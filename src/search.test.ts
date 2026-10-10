import { describe, expect, it } from 'vitest';
import { searchProjects, score } from './search';
import type { Project } from './models';

const project = (name: string, values: Partial<Project> = {}): Project => ({
  id: name, name, path: `D:\\代码\\apps\\${name}`, category: 'apps', categoryOverride: false,
  tags: ['TypeScript', 'React', 'Vite'], isGit: true, rootIds: ['root'], manual: false,
  favorite: false, lastOpenedAt: null, availability: 'available', editorId: null, vscodeStartup: { kind: 'default' }, ...values,
});

describe('project search and ordering', () => {
  it('matches all fields, fuzzy names, multiple words and unicode paths', () => {
    const p = project('silent-translator');
    for (const query of ['sil', 'slt', 'REACT', 'apps', '代码', 'react translator']) expect(score(p, query)).toBeGreaterThan(0);
    expect(score(p, 'react missing')).toBe(-1);
    expect(score(p, 'silent-translator')).toBeGreaterThan(score(p, 'sil'));
  });
  it('finds git-only projects and accepts either Windows path separator', () => {
    const p = project('Silent-Translator', { path: 'D:\\code\\mods\\Silent-Translator', category: 'mods', tags: [] });
    for (const query of ['sil', 'silent translator', 'mods', 'D:\\code\\mods', 'd:/code/mods']) {
      expect(searchProjects([p], query, { kind: 'all' })).toEqual([p]);
    }
    expect(searchProjects([project('中文项目')], 'D:/代码/apps', { kind: 'all' })).toHaveLength(1);
  });
  it('pins favorites and ranks by name relevance inside each group', () => {
    const list = [project('a-react'), project('react-core'), project('z', { favorite: true })];
    expect(searchProjects(list, 'react', { kind: 'all' }).map(p => p.name)).toEqual(['z', 'react-core', 'a-react']);
  });
  it('recent stays chronological, categories combine with queries', () => {
    const list = [project('old', { lastOpenedAt: 10, favorite: true }), project('new', { lastOpenedAt: 20, category: 'web' }), project('unused')];
    expect(searchProjects(list, '', { kind: 'recent' }).map(p => p.name)).toEqual(['new', 'old']);
    expect(searchProjects(list, 'react', { kind: 'category', category: 'web' }).map(p => p.name)).toEqual(['new']);
  });
  it('searches 500 projects within the local performance target', () => {
    const list = Array.from({ length: 500 }, (_, i) => project(`project-${i}`));
    const timings = Array.from({ length: 30 }, () => {
      const start = performance.now(); searchProjects(list, 'react project', { kind: 'all' });
      return performance.now() - start;
    }).sort((a, b) => a - b);
    expect(timings[28]).toBeLessThan(50);
    console.info(`500-project search P95: ${timings[28].toFixed(2)}ms`);
  });
});
