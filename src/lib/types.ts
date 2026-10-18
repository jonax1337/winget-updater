// Mirror of src-tauri/src/models.rs – keep in sync.

export type ManagerId = "winget" | "scoop" | "chocolatey" | "npm" | "pip";

export interface ManagerInfo {
  id: ManagerId;
  name: string;
  available: boolean;
  version: string | null;
  enabled: boolean;
  requiresAdmin: boolean;
}

export interface PackageRef {
  manager: ManagerId;
  id: string;
}

export interface Package extends PackageRef {
  name: string;
  currentVersion: string;
  availableVersion: string;
  source: string | null;
  ignored: boolean;
}

export interface ManagerError {
  manager: ManagerId;
  message: string;
}

export interface ScanResult {
  packages: Package[];
  errors: ManagerError[];
  scannedAt: string;
}

export type UpdateStatus = "queued" | "running" | "succeeded" | "failed";

export interface UpdateEvent {
  package: PackageRef;
  status: UpdateStatus;
  message: string | null;
}

export interface UpdateSummary {
  succeeded: number;
  failed: number;
}

export interface HistoryEntry {
  package: PackageRef;
  name: string;
  fromVersion: string;
  toVersion: string;
  status: UpdateStatus;
  message: string | null;
  finishedAt: string;
}

export interface RunState {
  scanning: boolean;
  updating: boolean;
  lastScan: string | null;
  nextScan: string | null;
  pendingUpdates: number;
}

export type Theme = "system" | "light" | "dark";

export interface Settings {
  autoUpdate: boolean;
  checkIntervalHours: number;
  updateOnStartup: boolean;
  startupDelayMinutes: number;
  notify: boolean;
  autostart: boolean;
  includeUnknownVersions: boolean;
  enabledManagers: ManagerId[];
  ignoredPackages: string[];
  theme: Theme;
}

export interface AppError {
  kind: string;
  message: string;
}
