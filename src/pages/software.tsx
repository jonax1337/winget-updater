import { useMemo } from "react";
import { AppWindow, RefreshCw } from "lucide-react";
import {
  DataTable,
  dataTableColumns,
  type DataTableColumn,
} from "@/components/data-table";
import { ManagerBadge } from "@/components/manager-badge";
import { ManagerErrors } from "@/components/manager-errors";
import { NoMatches, PackageFilters, usePackageFilters } from "@/components/package-filters";
import { PackageIcon } from "@/components/package-icon";
import { PackageMenu } from "@/components/package-menu";
import { ResourceError } from "@/components/resource-error";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { useInstalled } from "@/hooks/use-installed";
import { useUpdater } from "@/hooks/use-updater";
import { packageKey } from "@/lib/packages";
import type { InstalledPackage, PackageRef } from "@/lib/types";

export function Software() {
  const { busy } = useUpdater();
  const installed = useInstalled();
  const filters = usePackageFilters();
  const packages = installed.installed?.packages ?? [];
  const columns = useMemo(
    () => softwareColumns(installed.forget, installed.reloading),
    [installed.forget, installed.reloading],
  );

  return (
    <div className="flex flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={installed.error} retry={installed.reload} />
      <ManagerErrors
        errors={installed.installed?.errors ?? []}
        failure="konnte nicht gelesen werden"
      />
      <div className="flex flex-wrap items-center gap-2">
        <PackageFilters filters={filters} />
        <Button
          variant="outline"
          className="ml-auto"
          disabled={busy || installed.loading || installed.reloading}
          onClick={() => void installed.reload()}
        >
          <RefreshCw
            data-icon="inline-start"
            className={installed.reloading ? "animate-spin" : undefined}
          />
          Neu laden
        </Button>
      </div>
      {installed.loading ? (
        <Skeleton className="h-96" />
      ) : (
        <DataTable
          data={packages.filter(filters.matches)}
          columns={columns}
          getRowId={installedKey}
          empty={packages.length ? <NoMatches /> : <NoSoftware />}
        />
      )}
    </div>
  );
}

// winget lists one package ID several times when different versions are installed.
const installedKey = (entry: InstalledPackage) => `${packageKey(entry)}@${entry.version}`;

const column = dataTableColumns<InstalledPackage>();

function softwareColumns(
  forget: (reference: PackageRef) => void,
  reloading: boolean,
): DataTableColumn<InstalledPackage>[] {
  return [
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
    column.accessor("version", {
      header: "Version",
      cell: ({ row }) => <span className="font-mono text-xs">{row.original.version}</span>,
    }),
    column.accessor("manager", {
      header: "Quelle",
      cell: ({ row }) => <ManagerBadge manager={row.original.manager} />,
    }),
    column.display({
      id: "actions",
      header: () => <span className="sr-only">Aktionen</span>,
      cell: ({ row }) => (
        <PackageMenu
          entry={row.original}
          onUninstalled={() => forget(row.original)}
          uninstallDisabled={reloading}
        />
      ),
    }),
  ];
}

function NoSoftware() {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <AppWindow />
        </EmptyMedia>
        <EmptyTitle>Keine Software gefunden</EmptyTitle>
        <EmptyDescription>
          Die aktiven Paketmanager haben keine installierte Software gemeldet.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
