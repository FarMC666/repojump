import type { Project, View } from './models';

const normalize = (text: string) => text.normalize('NFKC').replace(/\\/g, '/').toLocaleLowerCase();
function subsequence(needle: string, haystack: string): number {
  let position = 0, first = -1, last = -1;
  for (const char of needle) {
    const index = haystack.indexOf(char, position);
    if (index < 0) return 0;
    if (first < 0) first = index;
    last = index; position = index + char.length;
  }
  return Math.max(1, 35 - (last - first - needle.length + 1) - first * 0.25);
}

export function score(project: Project, query: string): number {
  const name = normalize(project.name);
  const fields = [project.path, project.category ?? '', ...project.tags].map(normalize);
  let total = 0;
  for (const token of normalize(query).trim().split(/\s+/).filter(Boolean)) {
    const nameScore = name === token ? 150 : name.startsWith(token) ? 110 : name.includes(token) ? 80 : subsequence(token, name);
    const fieldScore = fields.some(field => field === token) ? 60 : fields.some(field => field.includes(token)) ? 45 : 0;
    const value = Math.max(nameScore, fieldScore);
    if (!value) return -1;
    total += value;
  }
  return total;
}

export function searchProjects(projects: Project[], query: string, view: View): Project[] {
  return projects.filter(project => {
    if (view.kind === 'favorites') return project.favorite;
    if (view.kind === 'recent') return project.lastOpenedAt !== null;
    if (view.kind === 'category') return project.category === view.category;
    return true;
  }).map(project => ({ project, relevance: score(project, query) }))
    .filter(result => result.relevance >= 0)
    .sort((a, b) => {
      if (view.kind !== 'recent') {
        const favorite = Number(b.project.favorite) - Number(a.project.favorite);
        if (favorite) return favorite;
        if (query.trim() && a.relevance !== b.relevance) return b.relevance - a.relevance;
      }
      return (b.project.lastOpenedAt ?? 0) - (a.project.lastOpenedAt ?? 0)
        || a.project.name.localeCompare(b.project.name, undefined, { numeric: true })
        || a.project.path.localeCompare(b.project.path);
    }).map(result => result.project);
}

export function displayCategory(category: string | null, fallback: string): string {
  if (!category) return fallback;
  return category.replace(/^[a-z]/, char => char.toUpperCase());
}
