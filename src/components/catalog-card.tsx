import type { ReactNode } from "react";
import { Check, Download, RefreshCw } from "lucide-react";
import { PackageIcon } from "@/components/package-icon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import type { CatalogEntry, CatalogStatus } from "@/lib/catalog";

export interface CatalogActions {
  statusOf: (id: string) => CatalogStatus;
  installingId: string | undefined;
  installDisabled: boolean;
  install: (entry: CatalogEntry) => void;
}

const SKELETON_COUNT = 9;

export function CatalogGrid({
  entries,
  actions,
}: {
  entries: readonly CatalogEntry[];
  actions: CatalogActions;
}) {
  return (
    <Cards>
      {entries.map((entry) => (
        <CatalogCard key={entry.id} entry={entry} actions={actions} />
      ))}
    </Cards>
  );
}

export function CatalogGridSkeleton() {
  return (
    <Cards>
      {Array.from({ length: SKELETON_COUNT }, (_, index) => (
        <Skeleton key={index} className="h-28 rounded-xl" />
      ))}
    </Cards>
  );
}

function Cards({ children }: { children: ReactNode }) {
  return <div className="grid gap-3 @2xl/main:grid-cols-2 @5xl/main:grid-cols-3">{children}</div>;
}

function CatalogCard({ entry, actions }: { entry: CatalogEntry; actions: CatalogActions }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="flex min-w-0 items-center gap-3">
          <PackageIcon package={{ manager: "winget", id: entry.id }} name={entry.name} />
          <span className="min-w-0">
            <span className="block truncate">{entry.name}</span>
            <span className="block truncate font-mono text-xs font-normal text-muted-foreground">
              {entry.id}
            </span>
          </span>
        </CardTitle>
        <CardAction>
          <CatalogAction entry={entry} actions={actions} />
        </CardAction>
      </CardHeader>
      <CardContent>
        <p className="line-clamp-2 text-muted-foreground" title={entry.description}>
          {entry.description}
        </p>
      </CardContent>
    </Card>
  );
}

function CatalogAction({ entry, actions }: { entry: CatalogEntry; actions: CatalogActions }) {
  const status = actions.statusOf(entry.id);
  if (status === "update-available") {
    return (
      <Badge variant="secondary">
        <RefreshCw />
        Update available
      </Badge>
    );
  }
  if (status === "installed") {
    return (
      <Badge variant="outline" className="text-success">
        <Check />
        Installed
      </Badge>
    );
  }
  const installing = actions.installingId === entry.id;
  return (
    <Button
      variant="outline"
      size="sm"
      disabled={actions.installDisabled}
      onClick={() => actions.install(entry)}
      aria-label={`Install ${entry.name}`}
    >
      {installing ? <Spinner data-icon="inline-start" /> : <Download data-icon="inline-start" />}
      {installing ? "Installing" : "Install"}
    </Button>
  );
}
