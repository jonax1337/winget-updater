import { useState } from "react";
import type { RowSelectionState } from "@tanstack/react-table";
import { ArrowRight, Download, PackageCheck, ShieldCheck } from "lucide-react";
import {
  DataTable,
  dataTableColumns,
  selectColumn,
  type DataTableColumn,
} from "@/components/data-table";
import { ManagerBadge } from "@/components/manager-badge";
import { ManagerErrors } from "@/components/manager-errors";
import { NoMatches, PackageFilters, usePackageFilters } from "@/components/package-filters";
import { PackageIcon } from "@/components/package-icon";
import { PackageMenu } from "@/components/package-menu";
import { ResourceError } from "@/components/resource-error";
import { StatusBadge } from "@/components/status-badge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenuCheckboxItem,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { useUpdater } from "@/hooks/use-updater";
import { packageKey } from "@/lib/packages";
import type { Package, ScanResult } from "@/lib/types";

export function Updates() {
  const { scan, packages, actionable, busy, ready, settings, update } = useUpdater();
  const filters = usePackageFilters();
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const filtered = packages.filter(filters.matches);
  const actionableKeys = new Set(actionable.map(packageKey));
  const selection = actionable.filter((entry) => rowSelection[packageKey(entry)]);

  return (
    <div className="flex flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={scan.error} retry={scan.refresh} />
      <ManagerErrors errors={scan.scan?.errors ?? []} failure="could not be checked" />
      <div className="flex flex-wrap items-center gap-2">
        <PackageFilters filters={filters} />
        <Button
          className="ml-auto"
          disabled={busy || !ready || !selection.length || settings.saving}
          onClick={() => void update(selection)}
        >
          <Download data-icon="inline-start" />
          Update selected
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
    header: "App",
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
    header: "Source",
    cell: ({ row }) => <ManagerBadge manager={row.original.manager} />,
  }),
  column.display({
    id: "status",
    header: "Status",
    cell: ({ row }) => <PackageStatus entry={row.original} />,
  }),
  column.display({
    id: "actions",
    header: () => <span className="sr-only">Actions</span>,
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
  if (ignored) return "Ignored";
  return enabled ? "Available" : "Source paused";
}

function PackageActions({ entry }: { entry: Package }) {
  const { busy, ready, settings, actionable, update } = useUpdater();
  const locked = busy || !ready || settings.saving;
  const updatable = actionable.some((candidate) => packageKey(candidate) === packageKey(entry));
  return (
    <PackageMenu entry={entry}>
      <DropdownMenuItem disabled={locked || !updatable} onSelect={() => void update([entry])}>
        <Download />
        Update now
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuCheckboxItem
        checked={entry.ignored}
        disabled={locked}
        onCheckedChange={(value) => void settings.ignore(entry, value)}
      >
        Ignore automatic updates
      </DropdownMenuCheckboxItem>
    </PackageMenu>
  );
}

function NoUpdates({ scan }: { scan: ScanResult | null }) {
  const { busy, ready, settings, check } = useUpdater();
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">{scan ? <ShieldCheck /> : <PackageCheck />}</EmptyMedia>
        <EmptyTitle>{scan ? "Everything is up to date" : "Not checked yet"}</EmptyTitle>
        <EmptyDescription>
          {scan
            ? "No newer versions were found during the last check."
            : "Check your enabled package managers for updates."}
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <Button variant="outline" disabled={busy || !ready || settings.saving} onClick={() => void check()}>
          Check now
        </Button>
      </EmptyContent>
    </Empty>
  );
}

