import type { ManagerId, ManagerInfo, PackageRef } from "./types";

export function managerVersionLabel(manager: ManagerInfo): string {
  return manager.available ? manager.version ?? "Installiert" : "Nicht installiert";
}

export const MANAGER_NAMES: Record<ManagerId, string> = {
  winget: "WinGet",
  scoop: "Scoop",
  chocolatey: "Chocolatey",
  npm: "npm",
  pip: "pip",
};


export type ManagerFilter = ManagerId | "all";

export const MANAGER_FILTERS = [
  { value: "all" as ManagerFilter, label: "Alle Quellen" },
  ...(Object.entries(MANAGER_NAMES) as [ManagerId, string][]).map(([value, label]) => ({
    value: value as ManagerFilter,
    label,
  })),
];

export interface NamedPackageRef extends PackageRef {
  name: string;
}

/** `query` must already be trimmed and lower-cased for the German locale. */
export function matchesFilter(entry: NamedPackageRef, manager: ManagerFilter, query: string): boolean {
  return (
    (manager === "all" || entry.manager === manager) &&
    `${entry.name} ${entry.id}`.toLocaleLowerCase("de-DE").includes(query)
  );
}

export function packageKey(reference: PackageRef): string {
  return `${reference.manager}:${reference.id}`;
}

export function parsePackageKey(key: string): PackageRef | undefined {
  const separator = key.indexOf(":");
  const manager = key.slice(0, separator);
  const id = key.slice(separator + 1);
  if (
    separator < 1 ||
    !id ||
    !Object.prototype.hasOwnProperty.call(MANAGER_NAMES, manager)
  )
    return undefined;
  return { manager: manager as ManagerId, id };
}

export function initials(name: string): string {
  const words = name.match(/[\p{L}\p{N}]+/gu) ?? [];
  if (!words.length) return "?";
  const letters =
    words.length > 1
      ? [Array.from(words[0]!)[0], Array.from(words[1]!)[0]]
      : Array.from(words[0]!).slice(0, 2);
  return letters.join("").toLocaleUpperCase("de-DE");
}

