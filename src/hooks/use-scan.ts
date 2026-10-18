import { getLastScan, onScanFinished } from "@/lib/api";
import type { ScanResult } from "@/lib/types";
import { useBackendEvent } from "./use-backend-event";
import { useResource } from "./use-resource";

export function useScan() {
  const resource = useResource<ScanResult | null>(getLastScan, null);
  useBackendEvent(onScanFinished, resource.replace);
  return { ...resource, scan: resource.data };
}
