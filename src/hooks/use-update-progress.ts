import { useCallback, useRef, useState } from "react";
import {
  onScanFinished,
  onUpdateEvent,
  onUpdateFinished,
} from "@/lib/api";
import { packageKey } from "@/lib/packages";
import type { PackageRef, UpdateEvent, UpdateSummary } from "@/lib/types";
import { useBackendEvent } from "./use-backend-event";

export function useUpdateProgress() {
  const [events, setEvents] = useState<Map<string, UpdateEvent>>(new Map());
  const [runKeys, setRunKeys] = useState<Set<string>>(new Set());
  const [summary, setSummary] = useState<UpdateSummary>();
  const active = useRef(false);

  const begin = useCallback((packages: PackageRef[] = []) => {
    active.current = true;
    setSummary(undefined);
    setRunKeys(new Set(packages.map(packageKey)));
    setEvents((previous) => {
      const next = new Map(previous);
      for (const reference of packages) {
        next.set(packageKey(reference), {
          package: reference,
          status: "queued",
          message: null,
        });
      }
      return next;
    });
  }, []);

  const finish = useCallback((result: UpdateSummary) => {
    active.current = false;
    setSummary(result);
  }, []);

  useBackendEvent(onUpdateEvent, (event) => {
    if (!active.current) begin();
    const key = packageKey(event.package);
    setRunKeys((previous) => new Set(previous).add(key));
    setEvents((previous) => new Map(previous).set(key, event));
  });
  useBackendEvent(onUpdateFinished, finish);

  const reset = useCallback(() => {
    active.current = false;
    setEvents(new Map());
    setRunKeys(new Set());
    setSummary(undefined);
  }, []);
  const abandon = useCallback(() => {
    if (active.current) reset();
  }, [reset]);
  useBackendEvent(onScanFinished, () => {
    if (!active.current) reset();
  });

  const current = Array.from(runKeys, (key) => events.get(key));
  const completed = current.filter(
    (event) =>
      event?.status === "succeeded" ||
      event?.status === "failed" ||
      event?.status === "skipped",
  ).length;
  return {
    events,
    summary,
    completed,
    total: runKeys.size,
    begin,
    finish,
    reset,
    abandon,
  };
}
