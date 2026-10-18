import { useDeferredValue, useState } from "react";
import type { RowSelectionState } from "@tanstack/react-table";
import {
  ArrowRight,
  Download,
  EllipsisVertical,
  PackageCheck,
  Search,
  SearchX,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import { Combobox } from "@/components/combobox";
import {
  DataTable,
  dataTableColumns,
  selectColumn,
  type DataTableColumn,
} from "@/components/data-table";
import { ManagerBadge } from "@/components/manager-badge";
import { PackageIcon } from "@/components/package-icon";
import { ResourceError } from "@/components/resource-error";
import { StatusBadge } from "@/components/status-badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { Skeleton } from "@/components/ui/skeleton";
import { useUpdater } from "@/hooks/use-updater";
import { MANAGER_NAMES, packageKey } from "@/lib/packages";
import type { ManagerId, Package, ScanResult } from "@/lib/types";

type ManagerFilter = ManagerId | "all";

const MANAGER_FILTERS = [
  { value: "all" as ManagerFilter, label: "Alle Quellen" },
  ...(Object.entries(MANAGER_NAMES) as [ManagerId, string][]).map(([value, label]) => ({
    value: value as ManagerFilter,
    label,
  })),
];

export function Updates() {
  const { scan, packages, actionable, busy, ready, settings, update } = useUpdater();
  const [search, setSearch] = useState("");
  const [manager, setManager] = useState<ManagerFilter>("all");
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const query = useDeferredValue(search.trim().toLocaleLowerCase("de-DE"));
  const filtered = packages.filter(
    (entry) =>
      (manager === "all" || entry.manager === manager) &&
      `${entry.name} ${entry.id}`.toLocaleLowerCase("de-DE").includes(query),
  );
  const actionableKeys = new Set(actionable.map(packageKey));
  const selection = actionable.filter((entry) => rowSelection[packageKey(entry)]);

  return (
    <div className="flex flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={scan.error} retry={scan.refresh} />
      {scan.scan?.errors.map((error) => (
        <Alert key={error.manager}>
          <TriangleAlert className="text-warning" />
          <AlertTitle>{MANAGER_NAMES[error.manager]} konnte nicht geprüft werden</AlertTitle>
          <AlertDescription className="break-words">{error.message}</AlertDescription>
        </Alert>
      ))}
      <div className="flex flex-wrap items-center gap-2">
        <InputGroup className="max-w-sm">
          <InputGroupAddon>
            <Search />
          </InputGroupAddon>
          <InputGroupInput
            aria-label="Pakete durchsuchen"
            placeholder="Name oder Paket-ID suchen …"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </InputGroup>
        <Combobox
          label="Nach Quelle filtern"
          value={manager}
          options={MANAGER_FILTERS}
          onChange={setManager}
          className="w-44"
        />
        <Button
          className="ml-auto"
          disabled={busy || !ready || !selection.length || settings.saving}
          onClick={() => void update(selection)}
        >
          <Download data-icon="inline-start" />
          Auswahl aktualisieren
          {selection.length > 0 && <Badge variant="secondary">{selection.length}</Badge>}
        </Button>
      </div>
      {scan.loading ? (
        <Skeleton className="h-96" />
      ) : (
        <DataTable
          data={filtered}
          columns={PACKAGE_COLUMNS}
          getRowId={packageKey}
          rowSelection={rowSelection}
          onRowSelectionChange={setRowSelection}
          canSelect={(entry) => !busy && actionableKeys.has(packageKey(entry))}
          empty={packages.length ? <NoMatches /> : <NoUpdates scan={scan.scan} />}
        />
      )}
    </div>
  );
}

const column = dataTableColumns<Package>();

const PACKAGE_COLUMNS: DataTableColumn<Package>[] = [
  selectColumn((entry) => entry.name),
  column.accessor("name", {
    header: "Anwendung",
    cell: ({ row }) => (
      <div className="flex items-center gap-3">
        <PackageIcon package={row.original} name={row.original.name} />
        <div className="min-w-0 max-w-64">
          <p className="truncate font-medium">{row.original.name}</p>
          <p className="truncate font-mono text-xs text-muted-foreground">{row.original.id}</p>
        </div>
      </div>
    ),
  }),
  column.accessor("availableVersion", {
    header: "Version",
    cell: ({ row }) => (
      <span className="flex items-center gap-1.5 font-mono text-xs">
        <span className="text-muted-foreground">{row.original.currentVersion || "?"}</span>
        <ArrowRight className="size-3 text-muted-foreground" />
        <span className="font-medium">{row.original.availableVersion}</span>
      </span>
    ),
  }),
  column.accessor("manager", {
    header: "Quelle",
    cell: ({ row }) => <ManagerBadge manager={row.original.manager} />,
  }),
  column.display({
    id: "status",
    header: "Status",
    cell: ({ row }) => <PackageStatus entry={row.original} />,
  }),
  column.display({
    id: "actions",
    header: () => <span className="sr-only">Aktionen</span>,
    cell: ({ row }) => <PackageActions entry={row.original} />,
  }),
];

function PackageStatus({ entry }: { entry: Package }) {
  const { progress, settings } = useUpdater();
  const event = progress.events.get(packageKey(entry));
  if (event)
    return (
      <div className="flex flex-col gap-1">
        <StatusBadge status={event.status} />
        {event.message && (
          <span className="max-w-56 truncate text-xs text-muted-foreground" title={event.message}>
            {event.message}
          </span>
        )}
      </div>
    );
  const enabled = settings.settings?.enabledManagers.includes(entry.manager);
  return (
    <Badge variant="outline" className="text-muted-foreground">
      {availabilityLabel(entry.ignored, enabled)}
    </Badge>
  );
}

function availabilityLabel(ignored: boolean, enabled?: boolean): string {
  if (ignored) return "Ignoriert";
  return enabled ? "Verfügbar" : "Quelle pausiert";
}

function PackageActions({ entry }: { entry: Package }) {
  const { busy, ready, settings, actionable, update } = useUpdater();
  const locked = busy || !ready || settings.saving;
  const updatable = actionable.some((candidate) => packageKey(candidate) === packageKey(entry));
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-8 text-muted-foreground data-[state=open]:bg-muted"
          aria-label={`Aktionen für ${entry.name}`}
        >
          <EllipsisVertical />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuItem disabled={locked || !updatable} onSelect={() => void update([entry])}>
          <Download />
          Jetzt aktualisieren
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuCheckboxItem
          checked={entry.ignored}
          disabled={locked}
          onCheckedChange={(value) => void settings.ignore(entry, value)}
        >
          Bei Auto-Updates ignorieren
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function NoUpdates({ scan }: { scan: ScanResult | null }) {
  const { busy, ready, settings, check } = useUpdater();
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">{scan ? <ShieldCheck /> : <PackageCheck />}</EmptyMedia>
        <EmptyTitle>{scan ? "Alles auf dem neuesten Stand" : "Noch nicht geprüft"}</EmptyTitle>
        <EmptyDescription>
          {scan
            ? "Bei der letzten Prüfung wurden keine neueren Versionen gefunden."
            : "Prüfe deine aktiven Paketmanager auf verfügbare Updates."}
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <Button variant="outline" disabled={busy || !ready || settings.saving} onClick={() => void check()}>
          Jetzt prüfen
        </Button>
      </EmptyContent>
    </Empty>
  );
}

function NoMatches() {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <SearchX />
        </EmptyMedia>
        <EmptyTitle>Keine passenden Pakete</EmptyTitle>
        <EmptyDescription>Anderen Suchbegriff oder eine andere Quelle wählen.</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
