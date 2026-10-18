import { useState } from "react";
import { ArrowRight, History as HistoryIcon, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ConfirmAction } from "@/components/confirm-action";
import { EmptyState } from "@/components/empty-state";
import { ManagerBadge } from "@/components/manager-badge";
import { PackageIcon } from "@/components/package-icon";
import { RelativeTime } from "@/components/relative-time";
import { ResourceError } from "@/components/resource-error";
import { StatusBadge } from "@/components/status-badge";
import { useUpdater } from "@/hooks/use-updater";
import { formatDate } from "@/lib/format";
import { packageKey } from "@/lib/packages";

export function History() {
  const { history, updating } = useUpdater();
  const [filter, setFilter] = useState("all");
  const filtered = history.entries.filter(
    (entry) => filter === "all" || entry.status === filter,
  );
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="page-title">Jede Aktualisierung nachvollziehbar.</h2>
          <p className="page-description">
            Die letzten 500 Ergebnisse, mit der neuesten Aktualisierung zuerst.
          </p>
        </div>
        <ConfirmAction
          title="Verlauf wirklich leeren?"
          description="Alle gespeicherten Ergebnisse werden entfernt. Installierte Anwendungen bleiben unverändert."
          confirm="Verlauf leeren"
          onConfirm={() => void history.clear()}
          trigger={
            <Button
              variant="outline"
              disabled={updating || history.clearing || !history.entries.length}
            >
              <Trash2 />
              Verlauf leeren
            </Button>
          }
        />
      </div>
      <ResourceError error={history.error} retry={history.refresh} />
      <div
        className="flex flex-wrap gap-2"
        role="group"
        aria-label="Verlauf filtern"
      >
        {[
          ["all", "Alle Ergebnisse"],
          ["succeeded", "Erfolgreich"],
          ["failed", "Fehlgeschlagen"],
        ].map(([value, label]) => (
          <Button
            key={value}
            size="sm"
            variant={filter === value ? "secondary" : "ghost"}
            aria-pressed={filter === value}
            onClick={() => setFilter(value)}
          >
            {label}
          </Button>
        ))}
      </div>
      <Card className="gap-0 py-0">
        {history.loading ? (
          <div className="space-y-4 p-5">
            {Array.from({ length: 5 }, (_, index) => (
              <Skeleton key={index} className="h-12 w-full" />
            ))}
          </div>
        ) : filtered.length === 0 ? (
          <EmptyState
            icon={HistoryIcon}
            title={
              history.entries.length
                ? "Keine Ergebnisse für diesen Filter"
                : "Hier entsteht dein Update-Verlauf"
            }
            description={
              history.entries.length
                ? "Wähle einen anderen Filter, um weitere Ergebnisse zu sehen."
                : "Sobald eine Anwendung aktualisiert wird, findest du hier Versionen, Zeitpunkt und Ergebnis."
            }
          />
        ) : (
          <Table className="min-w-170">
            <TableHeader>
              <TableRow className="bg-muted/35">
                <TableHead className="pl-5">Anwendung</TableHead>
                <TableHead>Version</TableHead>
                <TableHead>Quelle</TableHead>
                <TableHead>Ergebnis</TableHead>
                <TableHead className="pr-5 text-right">Zeitpunkt</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((entry) => (
                <TableRow
                  key={`${packageKey(entry.package)}-${entry.finishedAt}-${entry.status}`}
                >
                  <TableCell className="py-4 pl-5">
                    <div className="flex items-center gap-3">
                      <PackageIcon package={entry.package} name={entry.name} />
                      <div className="max-w-64">
                        <p className="truncate font-medium">{entry.name}</p>
                        <p
                          className="mt-1 truncate text-xs text-muted-foreground"
                          title={entry.package.id}
                        >
                          {entry.package.id}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-muted-foreground">
                        {entry.fromVersion || "Unbekannt"}
                      </span>
                      <ArrowRight className="size-3 text-muted-foreground" />
                      <span>{entry.toVersion}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <ManagerBadge manager={entry.package.manager} />
                  </TableCell>
                  <TableCell>
                    <StatusBadge
                      status={entry.status}
                      message={entry.message}
                    />
                    {entry.status === "failed" && entry.message && (
                      <p className="mt-2 max-w-64 whitespace-normal break-words text-xs text-muted-foreground">
                        {entry.message}
                      </p>
                    )}
                  </TableCell>
                  <TableCell className="pr-5 text-right">
                    <div className="text-xs">
                      <RelativeTime timestamp={entry.finishedAt} />
                      <p className="mt-1 text-muted-foreground">
                        {formatDate(entry.finishedAt)}
                      </p>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </Card>
    </div>
  );
}
