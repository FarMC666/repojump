import { useCallback, useEffect, useRef, useState } from 'react';
import { api, isDesktop } from './api';
import type { EditorStatus, Settings } from './models';

export function useEditors(settings: Settings, report: (error: unknown) => void) {
  const [editors, setEditors] = useState<EditorStatus[]>([]);
  const sequence = useRef(0);
  const key = JSON.stringify(settings.editorProfiles);
  const refresh = useCallback(() => {
    if (!isDesktop()) return;
    const request = ++sequence.current;
    void api.editors().then(next => { if (request === sequence.current) setEditors(next); }).catch(error => { if (request === sequence.current) report(error); });
  }, [key, report]);
  useEffect(() => { refresh(); return () => { sequence.current++; }; }, [refresh]);
  return { editors, refresh };
}
