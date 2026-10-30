import { useCallback, useState } from "react";
import { listInstalled } from "@/lib/api";
import { packageKey } from "@/lib/packages";
import type { InstalledResult, PackageRef } from "@/lib/types";
import { useResource } from "./use-resource";

export function useInstalled() {
  const resource = useResource<InstalledResult | null>(listInstalled, null);
  const { data, update, refresh } = resource;
  const [reloading, setReloading] = useState(false);

  const reload = useCallback(async () => {
    setReloading(true);
    try {
      await refresh();
    } finally {
      setReloading(false);
    }
  }, [refresh]);

  const forget = useCallback(
    (reference: PackageRef) =>
      update(
        (current) =>
          current && {
            ...current,
            packages: current.packages.filter(
              (entry) => packageKey(entry) !== packageKey(reference),
            ),
          },
      ),
    [update],
  );

  return { ...resource, installed: data, reloading, reload, forget };
}
