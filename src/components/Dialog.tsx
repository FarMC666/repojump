import { useEffect, useId, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';
import type { Translate } from '../i18n';

export function Dialog({ title, children, onClose, t, wide = false }: { title: string; children: ReactNode; onClose: () => void; t: Translate; wide?: boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => { const dialog = ref.current!; dialog.showModal(); return () => dialog.close(); }, []);
  return <dialog ref={ref} className={wide ? 'dialog settings-dialog' : 'dialog'} aria-labelledby={titleId} onCancel={e => { e.preventDefault(); onClose(); }} onClick={e => { if (e.target === ref.current) { const rect = ref.current!.getBoundingClientRect(); if (e.clientX < rect.left || e.clientX > rect.right || e.clientY < rect.top || e.clientY > rect.bottom) onClose(); } }}>
    <div className="dialog-header"><h2 id={titleId}>{title}</h2><button type="button" className="icon-button" aria-label={t('close')} onClick={onClose}><X size={18} /></button></div>
    {children}
  </dialog>;
}
