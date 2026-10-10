import { useEffect, useRef, useState } from 'react';
import type { ProjectAction } from '../projectActions';
import type { Translate } from '../i18n';
import { Dialog } from './Dialog';

export function ActionPalette({ name, actions, execute, onClose, t }: {
  name: string; actions: ProjectAction[]; execute: (id: string) => void; onClose: () => void; t: Translate;
}) {
  const [selected, setSelected] = useState(actions.find(action => action.enabled)?.id ?? '');
  const list = useRef<HTMLDivElement>(null);
  const enabled = actions.filter(action => action.enabled);
  const current = enabled.find(action => action.id === selected) ?? enabled[0];
  useEffect(() => { list.current?.focus(); }, []);
  useEffect(() => { list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }); }, [current?.id]);
  return <Dialog title={t('actionPalette')} t={t} onClose={onClose}>
    <div className="dialog-body"><p className="dialog-project">{name}</p>
      <div ref={list} className="palette-list" role="listbox" aria-label={t('actionPalette')} tabIndex={0} aria-activedescendant={current ? `action-${current.id}` : undefined} onKeyDown={event => {
        if (event.nativeEvent.isComposing || event.keyCode === 229) return;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault(); event.stopPropagation();
          const index = enabled.findIndex(action => action.id === current?.id);
          setSelected(enabled[(index + (event.key === 'ArrowDown' ? 1 : -1) + enabled.length) % enabled.length]?.id ?? '');
        } else if (event.key === 'Enter') { event.preventDefault(); event.stopPropagation(); if (current && !event.repeat) execute(current.id); }
        else if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); onClose(); }
      }}>
        {actions.map(action => <div id={`action-${action.id}`} key={action.id} role="option" aria-selected={current?.id === action.id} aria-disabled={!action.enabled} className={`palette-action ${current?.id === action.id ? 'selected' : ''}`} onMouseMove={() => { if (action.enabled) setSelected(action.id); }} onClick={() => { if (action.enabled) execute(action.id); }}><span>{action.label}</span>{action.shortcut && <kbd>{action.shortcut}</kbd>}</div>)}
      </div><p>{t('paletteHint')}</p>
    </div>
  </Dialog>;
}
