import { getManagers } from "@/lib/api";
import type { ManagerInfo } from "@/lib/types";
import { useResource } from "./use-resource";

export function useManagers() {
  const resource = useResource<ManagerInfo[]>(getManagers, []);
  return { ...resource, managers: resource.data };
}
