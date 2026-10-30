import type { ReactNode } from "react";
import { CalendarClock, Clock3, PackageCheck, ShieldCheck } from "lucide-react";
import {
  Card,
  CardAction,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { RelativeTime } from "@/components/relative-time";
import { plural } from "@/lib/format";
import { useUpdater } from "@/hooks/use-updater";
import type { Settings } from "@/lib/types";

export function SectionCards() {
  const { run, scan, settings, actionable, packages, scanning } = useUpdater();
  const loading = run.loading || scan.loading;
  const ignored = packages.filter((entry) => entry.ignored).length;
  const current = settings.settings;
  return (
    <div className="grid grid-cols-1 gap-4 px-4 lg:px-6 @xl/main:grid-cols-2 @5xl/main:grid-cols-4">
      <SectionCard
        label="Available updates"
        icon={<PackageCheck />}
        loading={loading}
        value={scan.scan ? actionable.length : "—"}
        footer={ignored ? `${ignored} ignored` : "Ready to update"}
      />
      <SectionCard
        label="Last check"
        icon={<Clock3 />}
        loading={loading}
        value={
          <RelativeTime
            timestamp={run.state.lastScan ?? scan.scan?.scannedAt ?? null}
            fallback="Never"
          />
        }
        footer={scanning ? "Checking for new versions …" : "All enabled package managers"}
      />
      <SectionCard
        label="Next check"
        icon={<CalendarClock />}
        loading={run.loading}
        value={<RelativeTime timestamp={run.state.nextScan} />}
        footer={scheduleLabel(current)}
      />
      <SectionCard
        label="Automatic updates"
        icon={<ShieldCheck />}
        loading={settings.loading}
        value={autoUpdateLabel(current?.autoUpdate)}
        footer={
          current?.autoUpdate
            ? "Updates run automatically"
            : "You start each update"
        }
      />
    </div>
  );
}

function SectionCard({
  label,
  icon,
  value,
  footer,
  loading,
}: {
  label: string;
  icon: ReactNode;
  value: ReactNode;
  footer: string;
  loading: boolean;
}) {
  return (
    <Card className="@container/card">
      <CardHeader>
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-2xl font-semibold tabular-nums @[250px]/card:text-3xl">
          {loading ? <Skeleton className="h-8 w-24" /> : value}
        </CardTitle>
        <CardAction className="text-muted-foreground [&_svg]:size-4">{icon}</CardAction>
      </CardHeader>
      <CardFooter className="border-t-0 bg-transparent pt-0 text-sm text-muted-foreground">
        {footer}
      </CardFooter>
    </Card>
  );
}

function autoUpdateLabel(enabled?: boolean): string {
  if (enabled === undefined) return "Not connected";
  return enabled ? "Active" : "Off";
}

function scheduleLabel(settings?: Settings): string {
  if (!settings) return "Loading schedule";
  if (settings.periodicChecks) return `Every ${plural(settings.checkIntervalHours, "hour", "hours")}`;
  return settings.updateOnStartup ? "At sign-in only" : "Manual only";
}
