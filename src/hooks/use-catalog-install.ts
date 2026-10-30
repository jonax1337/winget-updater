import { useState } from "react";
import { toast } from "sonner";
import { installCatalogPackage, toAppError } from "@/lib/api";
import type { CatalogEntry } from "@/lib/catalog";

export function useCatalogInstall(onInstalled: () => void) {
  const [installingId, setInstallingId] = useState<string>();

  const install = async ({ id, name }: CatalogEntry) => {
    setInstallingId(id);
    try {
      await installCatalogPackage(id);
      toast.success(`${name} installed`);
      onInstalled();
    } catch (error) {
      toast.error(`Installing ${name} failed`, {
        description: toAppError(error).message,
      });
    } finally {
      setInstallingId(undefined);
    }
  };

  return { installingId, install };
}
