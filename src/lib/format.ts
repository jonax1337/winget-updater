export function plural(count: number, one: string, other: string): string {
  const form = new Intl.PluralRules("en-US").select(count) === "one" ? one : other;
  return `${count} ${form}`;
}

const relativeFormatter = new Intl.RelativeTimeFormat("en-US", {
  numeric: "always",
  style: "short",
});
const dateFormatter = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
  timeStyle: "short",
});
const timeUnits: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * 60 * 60],
  ["month", 30 * 24 * 60 * 60],
  ["day", 24 * 60 * 60],
  ["hour", 60 * 60],
  ["minute", 60],
  ["second", 1],
];

export function relativeTime(
  timestamp: string | null,
  now = Date.now(),
): string {
  if (!timestamp) return "Not scheduled";
  const seconds = (Date.parse(timestamp) - now) / 1000;
  if (!Number.isFinite(seconds)) return "Unknown";
  const [unit, divisor] =
    timeUnits.find(([, duration]) => Math.abs(seconds) >= duration) ??
    timeUnits[5];
  return relativeFormatter.format(Math.round(seconds / divisor), unit);
}

export function formatDate(timestamp: string | null): string {
  if (!timestamp) return "No scans yet";
  const date = new Date(timestamp);
  return Number.isFinite(date.getTime())
    ? dateFormatter.format(date)
    : "Unknown";
}
