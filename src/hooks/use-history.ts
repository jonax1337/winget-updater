import { useCallback, useState } from "react";
import { toast } from "sonner";
import {
  clearHistory,
  getHistory,
  onUpdateFinished,
  toAppError,
} from "@/lib/api";
import type { HistoryEntry } from "@/lib/types";
import { useBackendEvent } from "./use-backend-event";
import { useResource } from "./use-resource";

const loadHistory = () => getHistory(500);

export function useHistory() {
  const resource = useResource<HistoryEntry[]>(loadHistory, []);
  const { refresh } = resource;
  const [clearing, setClearing] = useState(false);
  useBackendEvent(onUpdateFinished, () => {
    void refresh();
  });

  const clear = useCallback(async () => {
    setClearing(true);
    try {
      await clearHistory();
      await refresh();
      toast.success("Verlauf geleert");
    } catch (error) {
      toast.error("Verlauf konnte nicht geleert werden", {
        description: toAppError(error).message,
      });
    } finally {
      setClearing(false);
    }
  }, [refresh]);

  return { ...resource, entries: resource.data, clearing, clear };
}
