import { useCallback, useEffect, useRef, useState } from "react";
import { toAppError } from "@/lib/api";
import type { AppError } from "@/lib/types";

export function useResource<T>(load: () => Promise<T>, initial: T) {
  const [data, setData] = useState(initial);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<AppError>();
  const revision = useRef(0);
  const mounted = useRef(false);

  const update = useCallback((change: (current: T) => T) => {
    revision.current += 1;
    setData(change);
    setLoading(false);
    setError(undefined);
  }, []);

  const replace = useCallback((value: T) => update(() => value), [update]);

  const refresh = useCallback(async () => {
    const request = ++revision.current;
    setError(undefined);
    try {
      const value = await load();
      if (mounted.current && request === revision.current) replace(value);
    } catch (failure) {
      if (mounted.current && request === revision.current)
        setError(toAppError(failure));
    } finally {
      if (mounted.current && request === revision.current) setLoading(false);
    }
  }, [load, replace]);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => {
      mounted.current = false;
      revision.current += 1;
    };
  }, [refresh]);

  return { data, loading, error, refresh, replace, update };
}
