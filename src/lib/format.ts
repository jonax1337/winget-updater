export function plural(count: number, one: string, other: string): string {
  const form = new Intl.PluralRules("de-DE").select(count) === "one" ? one : other;
  return `${count} ${form}`;
}

const relativeFormatter = new Intl.RelativeTimeFormat("de-DE", {
  numeric: "always",
  style: "short",
});
const dateFormatter = new Intl.DateTimeFormat("de-DE", {
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
  if (!timestamp) return "Noch nicht geplant";
  const seconds = (Date.parse(timestamp) - now) / 1000;
  if (!Number.isFinite(seconds)) return "Unbekannt";
  const [unit, divisor] =
    timeUnits.find(([, duration]) => Math.abs(seconds) >= duration) ??
    timeUnits[5];
  return relativeFormatter.format(Math.round(seconds / divisor), unit);
}

export function formatDate(timestamp: string | null): string {
  if (!timestamp) return "Noch kein Scan";
  const date = new Date(timestamp);
  return Number.isFinite(date.getTime())
    ? dateFormatter.format(date)
    : "Unbekannt";
}
