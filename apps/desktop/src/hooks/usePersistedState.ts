import { useEffect, useState } from 'react';

export function usePersistedState<T>(key: string, defaultValue: T) {
  const [state, setState] = useState<T>(() => {
    if (typeof window === 'undefined') {
      return defaultValue;
    }
    try {
      const stored = window.localStorage.getItem(key);
      if (stored === null) {
        return defaultValue;
      }
      return JSON.parse(stored) as T;
    } catch {
      return defaultValue;
    }
  });

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    try {
      window.localStorage.setItem(key, JSON.stringify(state));
    } catch {
      // Ignore storage errors (quota, disabled storage).
    }
  }, [key, state]);

  return [state, setState] as const;
}
