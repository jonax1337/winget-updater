import { describe, expect, it } from "vitest";
import { formatDate, plural, relativeTime } from "./format";

describe("plural", () => {
  it.each([0, 2, 5])("uses the plural for %i", (count) => {
    expect(plural(count, "Paket", "Pakete")).toBe(`${count} Pakete`);
  });
  it("uses the singular for one", () => {
    expect(plural(1, "Paket", "Pakete")).toBe("1 Paket");
  });
});

const now = Date.parse("2026-10-18T12:00:00Z");

describe("relativeTime", () => {
  it("formats past minutes in German", () => {
    expect(relativeTime("2026-10-18T11:55:00Z", now)).toBe("vor 5 Min.");
  });
  it("formats future hours in German", () => {
    expect(relativeTime("2026-10-18T15:00:00Z", now)).toBe("in 3 Std.");
  });
  it("handles missing and invalid timestamps", () => {
    expect(relativeTime(null, now)).toBe("Noch nicht geplant");
    expect(relativeTime("invalid", now)).toBe("Unbekannt");
  });
  it("uses seconds for recent events", () => {
    expect(relativeTime("2026-10-18T11:59:50Z", now)).toBe("vor 10 Sek.");
  });
  it("uses days for distant events", () => {
    expect(relativeTime("2026-10-16T12:00:00Z", now)).toBe("vor 2 Tagen");
  });
});

describe("formatDate", () => {
  it("handles missing dates without throwing", () => {
    expect(formatDate(null)).toBe("Noch kein Scan");
    expect(formatDate("invalid")).toBe("Unbekannt");
  });
  it("uses German date order", () => {
    expect(formatDate("2026-10-18T12:00:00Z")).toContain("18.10.2026");
  });
});
