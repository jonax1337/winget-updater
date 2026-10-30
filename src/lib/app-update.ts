import type { DownloadEvent } from "@tauri-apps/plugin-updater";

export const STARTUP_CHECK_DELAY_MS = 15_000;
export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

const NOTES_LINE_LIMIT = 3;
const NOTES_LENGTH_LIMIT = 200;
const CHANGELOG_LINK_PREFIX = "**Full Changelog**";

export type DownloadProgress = { downloaded: number; total?: number };

export const NO_DOWNLOAD: DownloadProgress = { downloaded: 0 };

export function applyDownloadEvent(
  progress: DownloadProgress,
  event: DownloadEvent,
): DownloadProgress {
  switch (event.event) {
    case "Started":
      return { downloaded: 0, total: event.data.contentLength };
    case "Progress":
      return { ...progress, downloaded: progress.downloaded + event.data.chunkLength };
    case "Finished":
      return progress;
  }
}

export function downloadPercent({ downloaded, total }: DownloadProgress): number | undefined {
  if (!total) return undefined;
  return Math.min(100, Math.round((downloaded / total) * 100));
}

export function summarizeReleaseNotes(notes?: string): string | undefined {
  const lines = (notes ?? "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#") && !line.startsWith(CHANGELOG_LINK_PREFIX))
    .map((line) => line.replace(/^[-*+]\s+/, ""))
    .slice(0, NOTES_LINE_LIMIT);
  if (!lines.length) return undefined;
  const summary = lines.join(" · ");
  return summary.length > NOTES_LENGTH_LIMIT
    ? `${summary.slice(0, NOTES_LENGTH_LIMIT - 1).trimEnd()}…`
    : summary;
}
