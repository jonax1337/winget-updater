import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppError,
  HistoryEntry,
  InstalledResult,
  ManagerInfo,
  PackageRef,
  RunState,
  ScanResult,
  Settings,
  UpdateEvent,
  UpdateSummary,
} from "./types";
import type { CatalogSearchResult } from "./catalog";

export function toAppError(error: unknown): AppError {
  if (
    typeof error === "object" &&
    error !== null &&
    "kind" in error &&
    "message" in error
  ) {
    return { kind: String(error.kind), message: String(error.message) };
  }
  if (typeof error === "string") {
    try {
      const parsed: unknown = JSON.parse(error);
      if (typeof parsed === "object" && parsed !== null && "kind" in parsed && "message" in parsed) {
        return { kind: String(parsed.kind), message: String(parsed.message) };
      }
      return { kind: "other", message: error };
    } catch {
      return { kind: "other", message: error };
    }
  }
  return {
    kind: "other",
    message:
      error instanceof Error
        ? error.message
        : "Ein unbekannter Fehler ist aufgetreten.",
  };
}

async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      kind: "unavailable",
      message:
        "Bitte die Desktop-App öffnen. Die Browseransicht hat keinen Zugriff auf Paketmanager.",
    } satisfies AppError;
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    throw toAppError(error);
  }
}

async function onEvent<T>(
  name: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<T>(name, (event) => handler(event.payload));
  } catch (error) {
    throw toAppError(error);
  }
}

export const getManagers = () => command<ManagerInfo[]>("get_managers");
export const scanUpdates = () => command<ScanResult>("scan_updates");
export const getLastScan = () => command<ScanResult | null>("get_last_scan");
export const listInstalled = () => command<InstalledResult>("list_installed");
export const uninstallPackage = (reference: PackageRef) =>
  command<void>("uninstall_package", { package: reference });
export const updatePackages = (packages: PackageRef[]) =>
  command<UpdateSummary>("update_packages", { packages });
export const updateAll = () => command<UpdateSummary>("update_all");
export const getRunState = () => command<RunState>("get_run_state");
export const getSettings = () => command<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  command<Settings>("save_settings", { settings });
export const setPackageIgnored = (reference: PackageRef, ignored: boolean) =>
  command<Settings>("set_package_ignored", { package: reference, ignored });
export const getHistory = (limit = 500) =>
  command<HistoryEntry[]>("get_history", { limit });
export const clearHistory = () => command<void>("clear_history");
export const getPackageIcon = (reference: PackageRef, name: string) =>
  command<string | null>("get_package_icon", { package: reference, name });
export const installWingetModule = () => command<void>("install_winget_module");
export const searchCatalog = (query: string) =>
  command<CatalogSearchResult>("search_catalog", { query });
export const installCatalogPackage = (id: string) =>
  command<void>("install_catalog_package", { id });

export const onRunState = (handler: (payload: RunState) => void) =>
  onEvent("run-state", handler);
export const onScanFinished = (handler: (payload: ScanResult) => void) =>
  onEvent("scan-finished", handler);
export const onUpdateEvent = (handler: (payload: UpdateEvent) => void) =>
  onEvent("update-event", handler);
export const onUpdateFinished = (handler: (payload: UpdateSummary) => void) =>
  onEvent("update-finished", handler);
