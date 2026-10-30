import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { installCatalogPackage, searchCatalog } from "./api";
import {
  appsInCategory,
  CATALOG_APPS,
  CATEGORY_FILTERS,
  catalogStatus,
  searchResultEntry,
  wingetIds,
} from "./catalog";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

describe("catalog data", () => {
  it("lists every package ID once, ignoring case", () => {
    const ids = CATALOG_APPS.map((app) => app.id.toLowerCase());
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("uses IDs the backend accepts", () => {
    for (const { id } of CATALOG_APPS) expect(id).toMatch(/^[\p{L}\p{N}._\-/@+]+$/u);
  });

  it("gives every app a name and a description", () => {
    for (const app of CATALOG_APPS) {
      expect(app.name.trim(), app.id).not.toBe("");
      expect(app.description.trim(), app.id).not.toBe("");
    }
  });

  it("assigns every app to a known category", () => {
    const categories = CATEGORY_FILTERS.map((filter) => filter.value);
    for (const app of CATALOG_APPS) expect(categories, app.id).toContain(app.category);
  });

  it("fills every category with apps", () => {
    for (const { value } of CATEGORY_FILTERS) {
      expect(appsInCategory(value).length, value).toBeGreaterThan(0);
    }
  });

  it("shows all apps for the all filter and only matching ones otherwise", () => {
    expect(appsInCategory("all")).toBe(CATALOG_APPS);
    expect(appsInCategory("browsers").every((app) => app.category === "browsers")).toBe(true);
  });
});

describe("installed and update state", () => {
  const installed = wingetIds([
    { manager: "winget", id: "Git.Git" },
    { manager: "winget", id: "VideoLAN.VLC" },
    { manager: "scoop", id: "Mozilla.Firefox" },
  ]);
  const updatable = wingetIds([{ manager: "winget", id: "VideoLAN.VLC" }]);

  it("collects lower-cased winget IDs only", () => {
    expect([...installed]).toEqual(["git.git", "videolan.vlc"]);
  });

  it("reports installed, update available and not installed", () => {
    expect(catalogStatus("Git.Git", installed, updatable)).toBe("installed");
    expect(catalogStatus("VideoLAN.VLC", installed, updatable)).toBe("update-available");
    expect(catalogStatus("Mozilla.Firefox", installed, updatable)).toBe("not-installed");
  });

  it("matches IDs case-insensitively", () => {
    expect(catalogStatus("git.GIT", installed, updatable)).toBe("installed");
  });
});

describe("search results", () => {
  it("shows the version as description", () => {
    expect(
      searchResultEntry({ id: "VideoLAN.VLC", name: "VLC", version: "3.0.24", source: "winget" }),
    ).toEqual({ id: "VideoLAN.VLC", name: "VLC", description: "Version 3.0.24" });
  });
});

describe("catalog commands", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("passes the query to the search command", async () => {
    await searchCatalog("visual studio");
    expect(invoke).toHaveBeenCalledWith("search_catalog", { query: "visual studio" });
  });

  it("passes the package ID to the install command", async () => {
    await installCatalogPackage("Git.Git");
    expect(invoke).toHaveBeenCalledWith("install_catalog_package", { id: "Git.Git" });
  });
});
