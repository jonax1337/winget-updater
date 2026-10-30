import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";
import {
  getSettings,
  saveSettings,
  setPackageIgnored,
  toAppError,
} from "@/lib/api";
import { packageKey } from "@/lib/packages";
import type { PackageRef, Settings } from "@/lib/types";
import { useResource } from "./use-resource";

type Change = (settings: Settings) => Settings;
type Persist = (settings: Settings) => Promise<Settings>;

export function useSettings() {
  const resource = useResource<Settings | undefined>(getSettings, undefined);
  const { data, replace } = resource;
  const confirmed = useRef<Settings | undefined>(undefined);
  const queue = useRef(Promise.resolve());
  const optimistic = useRef<Settings | undefined>(undefined);
  const pendingCount = useRef(0);
  const version = useRef(0);
  const [pending, setPending] = useState(0);

  const enqueue = useCallback(
    (change: Change, persist: Persist = saveSettings) => {
      if (!data) return Promise.resolve();
      if (!pendingCount.current) {
        confirmed.current = data;
        optimistic.current = data;
      }
      const request = ++version.current;
      optimistic.current = change(optimistic.current!);
      replace(optimistic.current);
      pendingCount.current += 1;
      setPending(pendingCount.current);
      queue.current = queue.current.then(async () => {
        try {
          confirmed.current = await persist(change(confirmed.current!));
          if (request === version.current) {
            replace(confirmed.current);
            toast.success("Settings saved", {
              id: "settings-saved",
            });
          }
        } catch (error) {
          if (request === version.current) replace(confirmed.current);
          toast.error("Not saved", {
            description: toAppError(error).message,
          });
        } finally {
          pendingCount.current -= 1;
          setPending(pendingCount.current);
        }
      });
      return queue.current;
    },
    [data, replace],
  );

  const patch = useCallback(
    (changes: Partial<Settings>) =>
      enqueue((settings) => ({ ...settings, ...changes })),
    [enqueue],
  );

  const ignore = useCallback(
    (reference: PackageRef, ignored: boolean) => {
      const key = packageKey(reference);
      return enqueue(
        (settings) => ({
          ...settings,
          ignoredPackages: ignored
            ? Array.from(new Set([...settings.ignoredPackages, key]))
            : settings.ignoredPackages.filter((entry) => entry !== key),
        }),
        () => setPackageIgnored(reference, ignored),
      );
    },
    [enqueue],
  );

  return {
    ...resource,
    settings: data,
    saving: pending > 0,
    patch,
    ignore,
  };
}
