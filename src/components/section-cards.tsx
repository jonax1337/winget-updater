import type { ReactNode } from "react";
import { CalendarClock, Clock3, PackageCheck, ShieldCheck } from "lucide-react";
import { Badge } from "@/components/ui/badge";
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
import { useUpdater } from "@/hooks/use-updater";

export function SectionCards() {
  const { run, scan, settings, actionable, packages, scanning } = useUpdater();
  const loading = run.loading || scan.loading;
  const ignored = packages.filter((entry) => entry.ignored).length;
  const current = settings.settings;
  return (
    <div className="grid grid-cols-1 gap-4 px-4 *:data-[slot=card]:bg-gradient-to-t *:data-[slot=card]:from-primary/5 *:data-[slot=card]:to-card *:data-[slot=card]:shadow-xs lg:px-6 @xl/main:grid-cols-2 @5xl/main:grid-cols-4 dark:*:data-[slot=card]:bg-card">
      <SectionCard
        label="Verfügbare Updates"
        icon={<PackageCheck />}
        loading={loading}
        value={scan.scan ? actionable.length : "—"}
        footer={ignored ? `${ignored} ignoriert` : "Bereit zur Aktualisierung"}
      />
      <SectionCard
        label="Letzte Prüfung"
        icon={<Clock3 />}
        loading={loading}
        value={
          <RelativeTime
            timestamp={run.state.lastScan ?? scan.scan?.scannedAt ?? null}
            fallback="Noch keine"
          />
        }
        footer={scanning ? "Neue Versionen werden gesucht …" : "Alle aktiven Paketmanager"}
      />
      <SectionCard
        label="Nächste Prüfung"
        icon={<CalendarClock />}
        loading={run.loading}
        value={<RelativeTime timestamp={run.state.nextScan} />}
        footer={
          current
            ? `Automatisch alle ${current.checkIntervalHours} Stunden`
            : "Zeitplan wird geladen"
        }
      />
      <SectionCard
        label="Automatische Updates"
        icon={<ShieldCheck />}
        loading={settings.loading}
        value={autoUpdateLabel(current?.autoUpdate)}
        footer={
          current?.autoUpdate
            ? "Updates ohne manuelles Eingreifen"
            : "Du startest jede Aktualisierung"
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
        <CardAction>
          <Badge variant="outline">{icon}</Badge>
        </CardAction>
      </CardHeader>
      <CardFooter className="text-sm text-muted-foreground">{footer}</CardFooter>
    </Card>
  );
}

function autoUpdateLabel(enabled?: boolean): string {
  if (enabled === undefined) return "Nicht verbunden";
  return enabled ? "Aktiv" : "Aus";
}
