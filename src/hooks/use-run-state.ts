import { getRunState, onRunState } from "@/lib/api";
import type { RunState } from "@/lib/types";
import { useBackendEvent } from "./use-backend-event";
import { useResource } from "./use-resource";

const initialState: RunState = {
  scanning: false,
  updating: false,
  lastScan: null,
  nextScan: null,
  pendingUpdates: 0,
};

export function useRunState() {
  const resource = useResource(getRunState, initialState);
  useBackendEvent(onRunState, resource.replace);
  return { ...resource, state: resource.data };
}
