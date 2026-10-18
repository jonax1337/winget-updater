import { useDeferredValue, useState } from "react";
import {
  ArrowRight,
  Download,
  PackageCheck,
  Search,
  SearchX,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { EmptyState } from "@/components/empty-state";
import { ManagerBadge } from "@/components/manager-badge";
import { PackageIcon } from "@/components/package-icon";
import { ResourceError } from "@/components/resource-error";
import { StatusBadge } from "@/components/status-badge";
import { useUpdater } from "@/hooks/use-updater";
import { MANAGER_NAMES, packageKey } from "@/lib/packages";
import { plural } from "@/lib/format";
import { usePackageSelection } from "@/hooks/use-package-selection";
import type { Package, ScanResult, UpdateEvent } from "@/lib/types";

function emptyStateTitle(scan: ScanResult | null): string {
  if (!scan) return "Bereit für die erste Prüfung";
  if (scan.errors.length) return "Keine Updates ermittelt";
  return "Alles auf dem neuesten Stand";
}

function emptyStateDescription(scan: ScanResult | null): string {
  if (!scan) return "Prüfe deine aktiven Paketmanager auf verfügbare Updates.";
  if (scan.errors.length) {
    return "Einige Quellen konnten nicht geprüft werden. Prüfe die Hinweise oben und versuche es erneut.";
  }
  return "Bei der letzten Prüfung wurden keine neueren Versionen gefunden.";
}

export function Updates() {
  const {
    scan,
    packages,
    actionable,
    busy,
    ready,
    settings,
    progress,
    update,
    check,
  } = useUpdater();
  const [search, setSearch] = useState("");
  const query = useDeferredValue(search.trim().toLocaleLowerCase("de-DE"));
  const [manager, setManager] = useState("all");
  const filtered = packages.filter(
    (entry) =>
      (manager === "all" || entry.manager === manager) &&
      `${entry.name} ${entry.id}`.toLocaleLowerCase("de-DE").includes(query),
  );
  const { eligible, selection, checked, selectVisible, toggle, selected } =
    usePackageSelection(filtered, actionable);
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="page-title">Neue Versionen. Weniger Aufwand.</h2>
          <p className="page-description">
            Prüfen, auswählen und aktualisieren — über alle Paketmanager hinweg.
          </p>
        </div>
        <Badge variant="outline" className="mt-1">
          {actionable.length} verfügbar
        </Badge>
      </div>
      <ResourceError error={scan.error} retry={scan.refresh} />
      {scan.scan?.errors.map((error, index) => (
        <div
          key={`${error.manager}-${index}`}
          role="alert"
          className="flex items-start gap-3 rounded-xl border bg-card p-4 text-sm"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
          <div className="min-w-0">
            <span className="font-medium">{MANAGER_NAMES[error.manager]}</span>
            <p className="mt-1 break-words text-muted-foreground">
              {error.message}
            </p>
          </div>
        </div>
      ))}
      <Card className="gap-0 pb-0">
        <CardContent className="flex flex-wrap items-center gap-3 border-b pb-4">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute left-3 top-2.5 size-4 text-muted-foreground" />
            <Input
              className="h-9 pl-9"
              aria-label="Pakete durchsuchen"
              placeholder="Name oder Paket-ID suchen …"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
          </div>
          <Select value={manager} onValueChange={setManager}>
            <SelectTrigger
              className="h-9! min-w-44"
              aria-label="Nach Paketmanager filtern"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Alle Paketmanager</SelectItem>
              {Object.entries(MANAGER_NAMES).map(([id, name]) => (
                <SelectItem key={id} value={id}>
                  {name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </CardContent>
        {scan.loading ? (
          <div className="space-y-4 p-5" aria-label="Updates werden geladen">
            {Array.from({ length: 5 }, (_, index) => (
              <Skeleton key={index} className="h-12 w-full" />
            ))}
          </div>
        ) : packages.length === 0 ? (
          <EmptyState
            icon={scan.scan ? ShieldCheck : PackageCheck}
            title={emptyStateTitle(scan.scan)}
            description={emptyStateDescription(scan.scan)}
          >
            <Button
              variant="outline"
              onClick={() => void check()}
              disabled={busy || !ready || settings.saving}
            >
              Jetzt prüfen
            </Button>
          </EmptyState>
        ) : filtered.length === 0 ? (
          <EmptyState
            icon={SearchX}
            title="Keine passenden Pakete"
            description="Versuche einen anderen Suchbegriff oder wähle einen anderen Paketmanager."
          />
        ) : (
          <Table className="min-w-215">
            <TableHeader>
              <TableRow className="bg-muted/35 hover:bg-muted/35">
                <TableHead className="w-12 pl-5">
                  <Checkbox
                    aria-label="Alle sichtbaren, aktualisierbaren Pakete auswählen"
                    disabled={busy || !eligible.length}
                    checked={
                      checked > 0 && checked < eligible.length
                        ? "indeterminate"
                        : eligible.length > 0 && checked === eligible.length
                    }
                    onCheckedChange={(value) => selectVisible(value === true)}
                  />
                </TableHead>
                <TableHead>Anwendung</TableHead>
                <TableHead>Version</TableHead>
                <TableHead>Quelle</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Ignorieren</TableHead>
                <TableHead className="pr-5 text-right">Aktion</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((entry) => (
                <PackageRow
                  key={packageKey(entry)}
                  entry={entry}
                  event={progress.events.get(packageKey(entry))}
                  selected={selected.has(packageKey(entry))}
                  toggle={() => toggle(packageKey(entry))}
                />
              ))}
            </TableBody>
          </Table>
        )}
        <div className="flex flex-wrap items-center justify-between gap-3 border-t bg-muted/25 px-5 py-3">
          <p className="text-xs text-muted-foreground">
            {plural(filtered.length, "Paket", "Pakete")} · {selection.length} ausgewählt
          </p>
          <Button
            size="sm"
            disabled={busy || !ready || !selection.length || settings.saving}
            onClick={() => void update(selection)}
          >
            <Download />
            Auswahl aktualisieren
          </Button>
        </div>
      </Card>
    </div>
  );
}

function PackageRow({
  entry,
  event,
  selected,
  toggle,
}: {
  entry: Package;
  event?: UpdateEvent;
  selected: boolean;
  toggle: () => void;
}) {
  const { busy, ready, settings, update } = useUpdater();
  const done = event?.status === "succeeded";
  const enabled = settings.settings?.enabledManagers.includes(entry.manager);
  const disabled =
    busy || !ready || !enabled || entry.ignored || done || settings.saving;
  return (
    <TableRow
      className={entry.ignored ? "opacity-65" : undefined}
      data-state={selected && !entry.ignored ? "selected" : undefined}
    >
      <TableCell className="pl-5">
        <Checkbox
          aria-label={`${entry.name} auswählen`}
          checked={selected && !entry.ignored && !done}
          disabled={disabled}
          onCheckedChange={toggle}
        />
      </TableCell>
      <TableCell className="py-4">
        <div className="flex items-center gap-3">
          <PackageIcon package={entry} name={entry.name} />
          <div className="max-w-64">
            <p className="truncate font-medium" title={entry.name}>
              {entry.name}
            </p>
            <p
              className="mt-1 truncate text-xs text-muted-foreground"
              title={entry.id}
            >
              {entry.id}
            </p>
          </div>
        </div>
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-2 text-xs">
          <span
            className="max-w-28 truncate text-muted-foreground"
            title={entry.currentVersion}
          >
            {entry.currentVersion || "Unbekannt"}
          </span>
          <ArrowRight className="size-3 shrink-0 text-muted-foreground" />
          <span
            className="max-w-28 truncate font-medium"
            title={entry.availableVersion}
          >
            {entry.availableVersion}
          </span>
        </div>
      </TableCell>
      <TableCell>
        <ManagerBadge manager={entry.manager} />
      </TableCell>
      <TableCell>
        {event ? (
          <StatusBadge status={event.status} message={event.message} />
        ) : (
          <Badge variant="outline" className="text-muted-foreground">
            {entry.ignored
              ? "Ignoriert"
              : enabled
                ? "Verfügbar"
                : "Quelle pausiert"}
          </Badge>
        )}
      </TableCell>
      <TableCell>
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex p-2">
              <Switch
                aria-label={`${entry.name} bei automatischen Updates ignorieren`}
                checked={entry.ignored}
                disabled={busy || !ready || settings.saving}
                onCheckedChange={(value) => void settings.ignore(entry, value)}
              />
            </span>
          </TooltipTrigger>
          <TooltipContent>
            Ignorierte Pakete bleiben sichtbar, werden aber nicht automatisch
            aktualisiert.
          </TooltipContent>
        </Tooltip>
      </TableCell>
      <TableCell className="pr-5 text-right">
        <Button
          variant="outline"
          size="icon"
          aria-label={`${entry.name} aktualisieren`}
          disabled={disabled}
          onClick={() => void update([entry])}
        >
          <Download />
        </Button>
      </TableCell>
    </TableRow>
  );
}
