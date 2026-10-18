import { describe, expect, it } from "vitest";
import { initials, managerVersionLabel, packageKey, parsePackageKey } from "./packages";
import type { ManagerInfo } from "./types";

describe("managerVersionLabel", () => {
  const manager: ManagerInfo = {
    id: "winget", name: "WinGet", available: true, version: null,
    enabled: true, requiresAdmin: false,
  };
  it("labels an installed manager without a version", () => {
    expect(managerVersionLabel(manager)).toBe("Installiert");
  });
  it("shows the version of an available manager", () => {
    expect(managerVersionLabel({ ...manager, version: "1.2.3" })).toBe("1.2.3");
  });
  it("does not show a stale version for an unavailable manager", () => {
    expect(managerVersionLabel({ ...manager, available: false, version: "1.2.3" })).toBe("Nicht installiert");
  });
});

describe("packageKey", () => {
  it("separates managers for the same ID", () => {
    expect(packageKey({ manager: "npm", id: "python" })).toBe("npm:python");
    expect(packageKey({ manager: "pip", id: "python" })).toBe("pip:python");
  });
  it("preserves scoped IDs and embedded colons", () => {
    const reference = { manager: "npm" as const, id: "@scope/package:channel" };
    expect(parsePackageKey(packageKey(reference))).toEqual(reference);
  });
  it("rejects invalid keys", () => {
    expect(parsePackageKey("other:app")).toBeUndefined();
    expect(parsePackageKey("winget:")).toBeUndefined();
    expect(parsePackageKey("winget")).toBeUndefined();
  });
});

describe("initials", () => {
  it("takes two words or two letters from a single word", () => {
    expect(initials("Visual Studio Code")).toBe("VS");
    expect(initials("Firefox")).toBe("FI");
  });
  it("handles whitespace, scopes and Unicode", () => {
    expect(initials("  ")).toBe("?");
    expect(initials("@scope/tool")).toBe("ST");
    expect(initials("Änderung Übernehmen")).toBe("ÄÜ");
  });
});

