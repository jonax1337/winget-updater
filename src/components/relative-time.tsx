import { useEffect, useState } from "react";
import { formatDate, relativeTime } from "@/lib/format";

export function RelativeTime({
  timestamp,
  fallback = "Not scheduled",
}: {
  timestamp: string | null;
  fallback?: string;
}) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, []);
  if (!timestamp) return <span>{fallback}</span>;
  return (
    <time dateTime={timestamp} title={formatDate(timestamp)}>
      {relativeTime(timestamp, now)}
    </time>
  );
}
