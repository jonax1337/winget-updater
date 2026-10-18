import { useState } from "react";
import { ArrowRight, History as HistoryIcon, Trash2 } from "lucide-react";
import { ConfirmAction } from "@/components/confirm-action";
import {
  DataTable,
  dataTableColumns,
  type DataTableColumn,
} from "@/components/data-table";
import { ManagerBadge } from "@/components/manager-badge";
import { PackageIcon } from "@/components/package-icon";
import { RelativeTime } from "@/components/relative-time";
import { ResourceError } from "@/components/resource-error";
import { StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useUpdater } from "@/hooks/use-updater";
import { formatDate } from "@/lib/format";
import { packageKey } from "@/lib/packages";
import type { HistoryEntry } from "@/lib/types";

type ResultFilter = "all" | "succeeded" | "failed";

export function History() {
  const { history, updating } = useUpdater();
  const [filter, setFilter] = useState<ResultFilter>("all");
  const filtered = history.entries.filter(
    (entry) => filter === "all" || entry.status === filter,
  );
  return (
    <div className="flex flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={history.error} retry={history.refresh} />
      <div className="flex flex-wrap items-center gap-2">
        <ToggleGroup
          type="single"
          variant="outline"
          aria-label="Verlauf filtern"
          value={filter}
          onValueChange={(value: ResultFilter) => value && setFilter(value)}
        >
          <ToggleGroupItem value="all">Alle</ToggleGroupItem>
          <ToggleGroupItem value="succeeded">Erfolgreich</ToggleGroupItem>
          <ToggleGroupItem value="failed">Fehlgeschlagen</ToggleGroupItem>
        </ToggleGroup>
        <ConfirmAction
          title="Verlauf wirklich leeren?"
          description="Alle gespeicherten Ergebnisse werden entfernt. Installierte Anwendungen bleiben unverändert."
          confirm="Verlauf leeren"
          onConfirm={() => void history.clear()}
          trigger={
            <Button
              variant="outline"
              className="ml-auto"
              disabled={updating || history.clearing || !history.entries.length}
            >
              <Trash2 data-icon="inline-start" />
              Verlauf leeren
            </Button>
          }
        />
      </div>
      {history.loading ? (
        <Skeleton className="h-96" />
      ) : (
        <DataTable
          data={filtered}
          columns={HISTORY_COLUMNS}
          getRowId={(entry) => `${packageKey(entry.package)}-${entry.finishedAt}-${entry.status}`}
          empty={<NoHistory filtered={history.entries.length > 0} />}
        />
      )}
    </div>
  );
}

const column = dataTableColumns<HistoryEntry>();

const HISTORY_COLUMNS: DataTableColumn<HistoryEntry>[] = [
  column.accessor("name", {
    header: "Anwendung",
    cell: ({ row }) => (
      <div className="flex items-center gap-3">
        <PackageIcon package={row.original.package} name={row.original.name} />
        <div className="min-w-0 max-w-64">
          <p className="truncate font-medium">{row.original.name}</p>
          <p className="truncate font-mono text-xs text-muted-foreground">{row.original.package.id}</p>
        </div>
      </div>
    ),
  }),
  column.accessor("toVersion", {
    header: "Version",
    cell: ({ row }) => (
      <span className="flex items-center gap-1.5 font-mono text-xs">
        <span className="text-muted-foreground">{row.original.fromVersion || "?"}</span>
        <ArrowRight className="size-3 text-muted-foreground" />
        <span>{row.original.toVersion}</span>
      </span>
    ),
  }),
  column.display({
    id: "manager",
    header: "Quelle",
    cell: ({ row }) => <ManagerBadge manager={row.original.package.manager} />,
  }),
  column.accessor("status", {
    header: "Ergebnis",
    cell: ({ row }) => (
      <div className="flex flex-col gap-1">
        <StatusBadge status={row.original.status} />
        {row.original.status === "failed" && row.original.message && (
          <span className="max-w-64 truncate text-xs text-muted-foreground" title={row.original.message}>
            {row.original.message}
          </span>
        )}
      </div>
    ),
  }),
  column.accessor("finishedAt", {
    header: "Zeitpunkt",
    cell: ({ row }) => (
      <div className="flex flex-col text-xs">
        <RelativeTime timestamp={row.original.finishedAt} />
        <span className="text-muted-foreground">{formatDate(row.original.finishedAt)}</span>
      </div>
    ),
  }),
];

function NoHistory({ filtered }: { filtered: boolean }) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <HistoryIcon />
        </EmptyMedia>
        <EmptyTitle>{filtered ? "Keine Ergebnisse für diesen Filter" : "Noch kein Verlauf"}</EmptyTitle>
        <EmptyDescription>
          {filtered
            ? "Anderen Filter wählen, um weitere Ergebnisse zu sehen."
            : "Nach der ersten Aktualisierung stehen hier Versionen, Zeitpunkt und Ergebnis."}
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
