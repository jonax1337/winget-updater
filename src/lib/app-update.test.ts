import { describe, expect, it } from "vitest";
import {
  NO_DOWNLOAD,
  applyDownloadEvent,
  downloadPercent,
  summarizeReleaseNotes,
} from "./app-update";

describe("applyDownloadEvent", () => {
  it("starts with the announced size", () => {
    const started = applyDownloadEvent(NO_DOWNLOAD, {
      event: "Started",
      data: { contentLength: 400 },
    });
    expect(started).toEqual({ downloaded: 0, total: 400 });
  });

  it("accumulates downloaded chunks", () => {
    const first = applyDownloadEvent(
      { downloaded: 0, total: 400 },
      { event: "Progress", data: { chunkLength: 100 } },
    );
    const second = applyDownloadEvent(first, { event: "Progress", data: { chunkLength: 50 } });
    expect(second).toEqual({ downloaded: 150, total: 400 });
  });

  it("keeps the progress when the download finishes", () => {
    const progress = { downloaded: 400, total: 400 };
    expect(applyDownloadEvent(progress, { event: "Finished" })).toBe(progress);
  });
});

describe("downloadPercent", () => {
  it("rounds to whole percent", () => {
    expect(downloadPercent({ downloaded: 1, total: 3 })).toBe(33);
  });

  it("never exceeds 100", () => {
    expect(downloadPercent({ downloaded: 500, total: 400 })).toBe(100);
  });

  it("is unknown without a total size", () => {
    expect(downloadPercent({ downloaded: 100 })).toBeUndefined();
  });
});

describe("summarizeReleaseNotes", () => {
  it("is empty without notes", () => {
    expect(summarizeReleaseNotes(undefined)).toBeUndefined();
    expect(summarizeReleaseNotes("  \n ")).toBeUndefined();
  });

  it("skips headings and list markers", () => {
    const notes = "## What's changed\n- Faster scans\n* Fixed tray icon\n";
    expect(summarizeReleaseNotes(notes)).toBe("Faster scans · Fixed tray icon");
  });

  it("leaves out the generated changelog link", () => {
    const notes = "* Faster scans\n\n**Full Changelog**: https://github.com/o/r/compare/v1...v2";
    expect(summarizeReleaseNotes(notes)).toBe("Faster scans");
  });

  it("keeps only the first lines", () => {
    expect(summarizeReleaseNotes("a\nb\nc\nd")).toBe("a · b · c");
  });

  it("truncates long notes with an ellipsis", () => {
    const summary = summarizeReleaseNotes("x".repeat(300));
    expect(summary).toHaveLength(200);
    expect(summary?.endsWith("…")).toBe(true);
  });
});
