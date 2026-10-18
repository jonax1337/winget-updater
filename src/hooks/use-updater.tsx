import {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { toast } from "sonner";
import {
  onUpdateFinished,
  scanUpdates,
  toAppError,
  updateAll,
  updatePackages,
} from "@/lib/api";
import { packageKey } from "@/lib/packages";
import { plural } from "@/lib/format";
import type { Package, PackageRef, Settings, UpdateSummary } from "@/lib/types";

import { useBackendEvent } from "./use-backend-event";
import { useHistory } from "./use-history";
import { useManagers } from "./use-managers";
import { useRunState } from "./use-run-state";
import { useScan } from "./use-scan";
import { useSettings } from "./use-settings";
import { useTheme } from "./use-theme";
import { useUpdateProgress } from "./use-update-progress";

function availablePackages(packages: Package[], settings?: Settings): Package[] {
  return packages.filter(
    (entry) => !entry.ignored && settings?.enabledManagers.includes(entry.manager),
  );
}

function notifySummary(summary: UpdateSummary) {
  const message = `${summary.succeeded} aktualisiert, ${summary.failed} fehlgeschlagen`;
  const options = {
    id: "update-result",
    description: "Details stehen im Verlauf.",
  };
  if (summary.failed) toast.error(message, options);
  else toast.success(message, options);
}

function useUpdaterState() {
  const run = useRunState();
  const scan = useScan();
  const settings = useSettings();
  const history = useHistory();
  const managers = useManagers();
  const progress = useUpdateProgress();
  const lock = useRef(false);
  const [operation, setOperation] = useState<"scan" | "update">();
  useTheme(settings.settings?.theme);
  useBackendEvent(onUpdateFinished, notifySummary);

  const packages = (scan.scan?.packages ?? []).map((entry) => ({
    ...entry,
    ignored: settings.settings
      ? settings.settings.ignoredPackages.includes(packageKey(entry))
      : entry.ignored,
  }));
  const actionable = availablePackages(packages, settings.settings).filter(
    (entry) => progress.events.get(packageKey(entry))?.status !== "succeeded",
  );
  const scanning = run.state.scanning || operation === "scan";
  const updating = run.state.updating || operation === "update";
  const busy = scanning || updating;
  const ready =
    !run.loading && !run.error && !settings.loading && !settings.error;

  const { scanning: backendScanning, updating: backendUpdating } = run.state;
  const { refresh: refreshRun } = run;
  const { refresh: refreshManagers } = managers;
  const { replace: replaceScan } = scan;
  const { saving: settingsSaving, settings: currentSettings } = settings;

  const check = useCallback(async () => {
    if (lock.current || backendScanning || backendUpdating || settingsSaving) return;
    lock.current = true;
    setOperation("scan");
    try {
      const result = await scanUpdates();
      replaceScan(result);
      const count = availablePackages(result.packages, currentSettings).length;
      toast.success("Prüfung abgeschlossen", {
        description: `${plural(count, "Update", "Updates")} gefunden.`,
      });
    } catch (error) {
      toast.error("Prüfung fehlgeschlagen", {
        description: toAppError(error).message,
      });
    } finally {
      lock.current = false;
      setOperation(undefined);
      void refreshRun();
      void refreshManagers();
    }
  }, [
    backendScanning,
    backendUpdating,
    replaceScan,
    settingsSaving,
    currentSettings,
    refreshRun,
    refreshManagers,
  ]);

  const update = async (selection?: PackageRef[]) => {
    if (lock.current || busy || settings.saving) return;
    lock.current = true;
    setOperation("update");
    progress.begin(selection ?? actionable);
    try {
      await (selection ? updatePackages(selection) : updateAll());
    } catch (error) {
      progress.abandon();
      const { kind, message: description } = toAppError(error);
      if (kind === "storage") {
        toast.warning("Verlauf konnte nicht gespeichert werden", { description });
      } else {
        toast.error("Aktualisierung fehlgeschlagen", { description });
      }
    } finally {
      lock.current = false;
      setOperation(undefined);
      void run.refresh();
    }
  };

  return {
    run,
    scan,
    settings,
    history,
    managers,
    progress,
    packages,
    actionable,
    scanning,
    updating,
    busy,
    ready,
    check,
    update,
  };
}

type Updater = ReturnType<typeof useUpdaterState>;
const UpdaterContext = createContext<Updater | undefined>(undefined);

export function UpdaterProvider({ children }: { children: ReactNode }) {
  return (
    <UpdaterContext.Provider value={useUpdaterState()}>
      {children}
    </UpdaterContext.Provider>
  );
}

export function useUpdater(): Updater {
  const context = useContext(UpdaterContext);
  if (!context) throw new Error("UpdaterProvider fehlt");
  return context;
}
