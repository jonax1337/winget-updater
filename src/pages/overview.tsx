import {
  ArrowRight,
  Blocks,
  CalendarClock,
  Clock3,
  PackageCheck,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ManagerBadge } from "@/components/manager-badge";
import { RelativeTime } from "@/components/relative-time";
import { ResourceError } from "@/components/resource-error";
import { UpdateActions } from "@/components/update-actions";
import { useUpdater } from "@/hooks/use-updater";
import type { Page } from "@/components/app-sidebar";
import type { ReactNode } from "react";
import { plural } from "@/lib/format";
import { managerVersionLabel } from "@/lib/packages";

function updateHeadline(scanning: boolean, count: number, hasScan: boolean): string {
  if (scanning) return "Wir suchen nach neuen Versionen.";
  if (count) return plural(count, "Update wartet auf dich.", "Updates warten auf dich.");
  if (hasScan) return "Auf dem neuesten Stand.";
  return "Ein guter Zeitpunkt zum Prüfen.";
}

export function Overview({ navigate }: { navigate: (page: Page) => void }) {
  const { run, scan, settings, actionable, packages, scanning } = useUpdater();
  const loading = run.loading || scan.loading;
  return (
    <div className="space-y-6">
      <div>
        <h2 className="page-title">Alles im Blick.</h2>
        <p className="page-description">
          Deine Anwendungen aktuell halten. Ganz nebenbei.
        </p>
      </div>
      <ResourceError
        error={run.error ?? scan.error ?? settings.error}
        retry={() => {
          void run.refresh();
          void scan.refresh();
          void settings.refresh();
        }}
      />
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Metric
          icon={PackageCheck}
          label="Verfügbare Updates"
          loading={loading}
          value={
            <span className="text-3xl tabular-nums">
              {scan.scan ? actionable.length : "—"}
            </span>
          }
          detail={
            packages.some((entry) => entry.ignored)
              ? `${packages.filter((entry) => entry.ignored).length} ignoriert`
              : "Bereit zur Aktualisierung"
          }
        />
        <Metric
          icon={Clock3}
          label="Letzte Prüfung"
          loading={loading}
          value={
            <RelativeTime
              timestamp={run.state.lastScan ?? scan.scan?.scannedAt ?? null}
              fallback="Noch keine"
            />
          }
          detail={
            scanning
              ? "Neue Versionen werden gesucht …"
              : "Alle aktiven Paketmanager"
          }
        />
        <Metric
          icon={CalendarClock}
          label="Nächste Prüfung"
          loading={run.loading}
          value={<RelativeTime timestamp={run.state.nextScan} />}
          detail={
            settings.settings
              ? `Automatisch alle ${settings.settings.checkIntervalHours} Stunden`
              : "Zeitplan wird geladen"
          }
        />
        <Metric
          icon={ShieldCheck}
          label="Automatische Updates"
          loading={settings.loading}
          value={
            !settings.settings
              ? "Nicht verbunden"
              : settings.settings.autoUpdate
                ? "Aktiviert"
                : "Deaktiviert"
          }
          detail={
            !settings.settings
              ? "Einstellungen nicht verfügbar"
              : settings.settings.autoUpdate
                ? "Updates ohne manuelles Eingreifen"
                : "Du startest jede Aktualisierung"
          }
        />
      </div>
      <Card className="overview-action [--card-spacing:--spacing(6)]">
        <CardContent className="flex flex-col gap-6 md:flex-row md:items-center md:justify-between">
          <div className="flex items-start gap-4">
            <div className="flex size-14 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary">
              <ShieldCheck className="size-7" strokeWidth={1.4} />
            </div>
            <div>
              <h3 className="text-xl font-semibold tracking-tight">
                {updateHeadline(scanning, actionable.length, Boolean(scan.scan))}
              </h3>
              <p className="mt-2 max-w-lg text-sm leading-relaxed text-muted-foreground">
                {scan.scan?.errors.length
                  ? "Nicht alle Paketmanager konnten geprüft werden. Details findest du unter Updates."
                  : "Ein gemeinsamer Ort für deine Paketmanager. Prüfe neue Versionen oder aktualisiere alle Anwendungen auf einmal."}
              </p>
              <div className="mt-5">
                <UpdateActions large />
              </div>
            </div>
          </div>
          <Button
            variant="ghost"
            className="self-start md:self-center"
            onClick={() => navigate("updates")}
          >
            Updates ansehen
            <ArrowRight />
          </Button>
        </CardContent>
      </Card>
      <ManagerStatus />
      <div className="flex items-start gap-2 text-xs leading-relaxed text-muted-foreground">
        <RefreshCw className="mt-0.5 size-3.5 shrink-0" />
        <p>
          Fenster schließen beendet die App nicht. Im Infobereich von Windows
          laufen Prüfungen und Updates weiter.
        </p>
      </div>
    </div>
  );
}

function Metric({
  icon: Icon,
  label,
  value,
  detail,
  loading,
}: {
  icon: LucideIcon;
  label: string;
  value: ReactNode;
  detail: string;
  loading: boolean;
}) {
  return (
    <Card className="[--card-spacing:--spacing(5)]">
      <CardContent>
        <div className="flex items-center justify-between gap-3 text-muted-foreground">
          <p className="text-xs font-medium">{label}</p>
          <Icon className="size-4" strokeWidth={1.7} />
        </div>
        {loading ? (
          <Skeleton className="my-4 h-8 w-28" />
        ) : (
          <div className="mb-3 mt-4 text-xl font-semibold tracking-tight">
            {value}
          </div>
        )}
        <p className="text-xs leading-relaxed text-muted-foreground">
          {detail}
        </p>
      </CardContent>
    </Card>
  );
}

function ManagerStatus() {
  const { managers, settings } = useUpdater();
  return (
    <Card className="[--card-spacing:--spacing(5)]">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Blocks className="size-4 text-muted-foreground" />
          Paketmanager
        </CardTitle>
        <p className="text-sm text-muted-foreground">
          Deine verbundenen Quellen für neue Versionen.
        </p>
      </CardHeader>
      <CardContent>
        <ResourceError error={managers.error} retry={managers.refresh} />
        <div className="divide-y">
          {managers.loading
            ? Array.from({ length: 5 }, (_, index) => (
                <div key={index} className="flex items-center gap-4 py-4">
                  <Skeleton className="h-6 w-24" />
                  <Skeleton className="h-4 w-32" />
                </div>
              ))
            : managers.managers.map((manager) => {
                const enabled =
                  settings.settings?.enabledManagers.includes(manager.id) ??
                  manager.enabled;
                return (
                  <div
                    key={manager.id}
                    className="flex flex-wrap items-center gap-3 py-4 first:pt-0 last:pb-0"
                  >
                    <ManagerBadge manager={manager.id} />
                    <span className="min-w-24 flex-1 text-sm text-muted-foreground">
                      {managerVersionLabel(manager)}
                    </span>
                    {manager.requiresAdmin && (
                      <span className="text-xs text-muted-foreground">
                        Administratorrechte nötig
                      </span>
                    )}
                    <Badge
                      variant="outline"
                      className={
                        manager.available && enabled
                          ? "status-succeeded"
                          : "text-muted-foreground"
                      }
                    >
                      {!manager.available
                        ? "Nicht verfügbar"
                        : enabled
                          ? "Aktiv"
                          : "Pausiert"}
                    </Badge>
                  </div>
                );
              })}
        </div>
      </CardContent>
    </Card>
  );
}
