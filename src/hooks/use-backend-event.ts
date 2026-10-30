import { useEffect, useEffectEvent } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { toAppError } from "@/lib/api";

type Subscribe<T> = (handler: (payload: T) => void) => Promise<UnlistenFn>;

export function useBackendEvent<T>(
  subscribe: Subscribe<T>,
  handler: (payload: T) => void,
) {
  const receive = useEffectEvent(handler);
  useEffect(() => {
    let disposed = false;
    let unsubscribe: UnlistenFn | undefined;
    subscribe((payload) => {
      if (!disposed) receive(payload);
    })
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch((error: unknown) => {
        if (!disposed)
          toast.error("Live connection failed", {
            description: toAppError(error).message,
          });
      });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [subscribe]);
}
