import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import * as api from "./api";
import type { Settings } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const reference = { manager: "npm" as const, id: "@scope/tool" };
const settings: Settings = {
  autoUpdate: true,
  checkIntervalHours: 6,
  updateOnStartup: true,
  startupDelayMinutes: 2,
  notify: true,
  autostart: true,
  includeUnknownVersions: false,
  enabledManagers: ["winget"],
  ignoredPackages: [],
  theme: "system",
};

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockResolvedValue(undefined);
});

describe("command adapter", () => {
  it("uses the contract names for commands without arguments", async () => {
    await api.getManagers();
    await api.scanUpdates();
    await api.getLastScan();
    await api.updateAll();
    await api.getRunState();
    await api.getSettings();
    await api.clearHistory();
    await api.installWingetModule();
    expect(vi.mocked(invoke).mock.calls.map(([name]) => name)).toEqual([
      "get_managers",
      "scan_updates",
      "get_last_scan",
      "update_all",
      "get_run_state",
      "get_settings",
      "clear_history",
      "install_winget_module",
    ]);
  });
  it("passes package arguments exactly as required", async () => {
    await api.updatePackages([reference]);
    expect(invoke).toHaveBeenLastCalledWith("update_packages", {
      packages: [reference],
    });
    await api.setPackageIgnored(reference, true);
    expect(invoke).toHaveBeenLastCalledWith("set_package_ignored", {
      package: reference,
      ignored: true,
    });
    await api.getPackageIcon(reference, "Tool");
    expect(invoke).toHaveBeenLastCalledWith("get_package_icon", {
      package: reference,
      name: "Tool",
    });
  });
  it("passes settings and history limits", async () => {
    await api.saveSettings(settings);
    expect(invoke).toHaveBeenLastCalledWith("save_settings", { settings });
    await api.getHistory(25);
    expect(invoke).toHaveBeenLastCalledWith("get_history", { limit: 25 });
  });
  it("preserves structured backend errors", async () => {
    vi.mocked(invoke).mockRejectedValue({
      kind: "busy",
      message: "Bereits aktiv",
    });
    await expect(api.scanUpdates()).rejects.toEqual({
      kind: "busy",
      message: "Bereits aktiv",
    });
  });
  it("does not invoke backend commands from a browser preview", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(api.updateAll()).rejects.toMatchObject({
      kind: "unavailable",
    });
    expect(invoke).not.toHaveBeenCalled();
  });
});

describe("event adapter", () => {
  it("registers all four contract events and returns their cleanup", async () => {
    const stop = vi.fn();
    vi.mocked(listen).mockResolvedValue(stop);
    const callback = vi.fn();
    const unsubscribe = await api.onRunState(callback);
    await api.onScanFinished(vi.fn());
    await api.onUpdateEvent(vi.fn());
    await api.onUpdateFinished(vi.fn());
    expect(vi.mocked(listen).mock.calls.map(([name]) => name)).toEqual([
      "run-state",
      "scan-finished",
      "update-event",
      "update-finished",
    ]);
    const payload = { scanning: true };
    const handler = vi.mocked(listen).mock.calls[0][1];
    handler({ event: "run-state", id: 1, payload });
    expect(callback).toHaveBeenCalledWith(payload);
    unsubscribe();
    expect(stop).toHaveBeenCalledOnce();
  });
  it("does not subscribe from a browser preview", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const stop = await api.onRunState(vi.fn());
    expect(listen).not.toHaveBeenCalled();
    expect(stop).toBeTypeOf("function");
  });
});

describe("toAppError", () => {
  it.each(["404", "true", "null", '"404"', "[]", '{}'])(
    "preserves the original string for JSON without an AppError: %s",
    (message) => {
      expect(api.toAppError(message)).toEqual({ kind: "other", message });
    },
  );
  it("normalizes serialized errors", () => {
    expect(api.toAppError('{"kind":"timeout","message":"Zu langsam"}')).toEqual(
      { kind: "timeout", message: "Zu langsam" },
    );
  });
  it("preserves plain errors and unexpected values", () => {
    expect(api.toAppError(new Error("Offline"))).toEqual({
      kind: "other",
      message: "Offline",
    });
    expect(api.toAppError("Kein JSON")).toEqual({
      kind: "other",
      message: "Kein JSON",
    });
    expect(api.toAppError(null).kind).toBe("other");
  });
});
