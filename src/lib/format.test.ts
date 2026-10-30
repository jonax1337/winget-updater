import { describe, expect, it } from "vitest";
import { formatDate, plural, relativeTime } from "./format";

describe("plural", () => {
  it.each([0, 2, 5])("uses the plural for %i", (count) => {
    expect(plural(count, "package", "packages")).toBe(`${count} packages`);
  });
  it("uses the singular for one", () => {
    expect(plural(1, "package", "packages")).toBe("1 package");
  });
});

const now = Date.parse("2026-10-18T12:00:00Z");

describe("relativeTime", () => {
  it("formats past minutes in English", () => {
    expect(relativeTime("2026-10-18T11:55:00Z", now)).toBe("5 min. ago");
  });
  it("formats future hours in English", () => {
    expect(relativeTime("2026-10-18T15:00:00Z", now)).toBe("in 3 hr.");
  });
  it("handles missing and invalid timestamps", () => {
    expect(relativeTime(null, now)).toBe("Not scheduled");
    expect(relativeTime("invalid", now)).toBe("Unknown");
  });
  it("uses seconds for recent events", () => {
    expect(relativeTime("2026-10-18T11:59:50Z", now)).toBe("10 sec. ago");
  });
  it("uses days for distant events", () => {
    expect(relativeTime("2026-10-16T12:00:00Z", now)).toBe("2 days ago");
  });
});

describe("formatDate", () => {
  it("handles missing dates without throwing", () => {
    expect(formatDate(null)).toBe("No scans yet");
    expect(formatDate("invalid")).toBe("Unknown");
  });
  it("uses English date order", () => {
    expect(formatDate("2026-10-18T12:00:00Z")).toContain("Oct 18, 2026");
  });
});
