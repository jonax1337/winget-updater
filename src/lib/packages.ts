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

const avatarColors = [
  "#276078",
  "#705796",
  "#84602f",
  "#336b60",
  "#8c4e68",
  "#515e86",
];

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

export function avatarColor(name: string): string {
  let hash = 0;
  for (const character of name)
    hash = (Math.imul(hash, 31) + character.codePointAt(0)!) >>> 0;
  return avatarColors[hash % avatarColors.length];
}
