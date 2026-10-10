import { Code2, Copy, FolderOpen, Globe, Star, Terminal } from 'lucide-react';
import type { ActionIntent } from '../projectActions';

export function ProjectActionIcon({ intent }: { intent: ActionIntent }) {
  const Icon = intent.kind === 'copy' ? Copy : intent.kind === 'favorite' ? Star
    : intent.target === 'terminal' ? Terminal : intent.target === 'explorer' ? FolderOpen
    : intent.target === 'repository' ? Globe : Code2;
  return <Icon size={16} />;
}
