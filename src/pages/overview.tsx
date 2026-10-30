import { ArrowRight, Blocks, ShieldCheck } from "lucide-react";
import { Label, Pie, PieChart } from "recharts";
import type { Page } from "@/components/app-sidebar";
import { ManagerDot } from "@/components/manager-badge";
import { ResourceError } from "@/components/resource-error";
import { SectionCards } from "@/components/section-cards";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemTitle,
} from "@/components/ui/item";
import { Skeleton } from "@/components/ui/skeleton";
import { useUpdater } from "@/hooks/use-updater";
import { MANAGER_NAMES, managerVersionLabel } from "@/lib/packages";
import type { ManagerId, ManagerInfo } from "@/lib/types";

const chartConfig = {
  updates: { label: "Updates" },
  winget: { label: MANAGER_NAMES.winget, color: "var(--color-manager-winget)" },
  scoop: { label: MANAGER_NAMES.scoop, color: "var(--color-manager-scoop)" },
  chocolatey: { label: MANAGER_NAMES.chocolatey, color: "var(--color-manager-chocolatey)" },
  npm: { label: MANAGER_NAMES.npm, color: "var(--color-manager-npm)" },
  pip: { label: MANAGER_NAMES.pip, color: "var(--color-manager-pip)" },
} satisfies ChartConfig;

export function Overview({ navigate }: { navigate: (page: Page) => void }) {
  const { run, scan, settings } = useUpdater();
  return (
    <>
      <div className="px-4 empty:hidden lg:px-6">
        <ResourceError
          error={run.error ?? scan.error ?? settings.error}
          retry={() => {
            void run.refresh();
            void scan.refresh();
            void settings.refresh();
          }}
        />
      </div>
      <SectionCards />
      <div className="grid grid-cols-1 gap-4 px-4 lg:px-6 @3xl/main:grid-cols-2">
        <UpdatesBySource onShowAll={() => navigate("updates")} />
        <ManagerStatus />
      </div>
    </>
  );
}

function UpdatesBySource({ onShowAll }: { onShowAll: () => void }) {
  const { actionable } = useUpdater();
  const data: SourceSlice[] = (Object.keys(MANAGER_NAMES) as ManagerId[])
    .map((manager) => ({
      manager,
      updates: actionable.filter((entry) => entry.manager === manager).length,
      fill: `var(--color-${manager})`,
    }))
    .filter((entry) => entry.updates > 0);
  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle>Updates by source</CardTitle>
        <CardDescription>Available updates, excluding ignored packages</CardDescription>
        <CardAction>
          <Button variant="ghost" size="sm" onClick={onShowAll}>
            View all
            <ArrowRight data-icon="inline-end" />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent className="flex-1">
        {data.length ? (
          <>
            <SourceChart data={data} total={actionable.length} />
            <SourceLegend data={data} />
          </>
        ) : (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <ShieldCheck />
              </EmptyMedia>
              <EmptyTitle>Up to date</EmptyTitle>
              <EmptyDescription>No pending updates found.</EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </CardContent>
    </Card>
  );
}

interface SourceSlice {
  manager: ManagerId;
  updates: number;
  fill: string;
}

function SourceChart({ data, total }: { data: SourceSlice[]; total: number }) {
  return (
    <ChartContainer config={chartConfig} className="mx-auto aspect-square max-h-56">
      <PieChart>
        <ChartTooltip cursor={false} content={<ChartTooltipContent hideLabel />} />
        <Pie data={data} dataKey="updates" nameKey="manager" innerRadius={60} strokeWidth={5}>
          <Label
            content={({ viewBox }) =>
              viewBox && "cx" in viewBox && "cy" in viewBox ? (
                <text x={viewBox.cx} y={viewBox.cy} textAnchor="middle" dominantBaseline="middle">
                  <tspan x={viewBox.cx} y={viewBox.cy} className="fill-foreground text-3xl font-bold">
                    {total}
                  </tspan>
                  <tspan x={viewBox.cx} y={(viewBox.cy ?? 0) + 24} className="fill-muted-foreground">
                    Updates
                  </tspan>
                </text>
              ) : null
            }
          />
        </Pie>
      </PieChart>
    </ChartContainer>
  );
}

function SourceLegend({ data }: { data: SourceSlice[] }) {
  return (
    <ul aria-label="Legend" className="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
      {data.map((entry) => (
        <li key={entry.manager} className="flex items-center gap-2">
          <ManagerDot manager={entry.manager} />
          {MANAGER_NAMES[entry.manager]}
          <span className="ml-auto text-muted-foreground tabular-nums">{entry.updates}</span>
        </li>
      ))}
    </ul>
  );
}

function ManagerStatus() {
  const { managers, settings } = useUpdater();
  return (
    <Card>
      <CardHeader>
        <CardTitle>Package managers</CardTitle>
        <CardDescription>Sources checked for new versions</CardDescription>
        <CardAction>
          <Blocks className="size-4 text-muted-foreground" />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <ResourceError error={managers.error} retry={managers.refresh} />
        <ItemGroup className="gap-2">
          {managers.loading
            ? Array.from({ length: 5 }, (_, index) => <Skeleton key={index} className="h-12" />)
            : managers.managers.map((manager) => (
                <Item key={manager.id} variant="outline" size="sm">
                  <ItemContent>
                    <ItemTitle>
                      <ManagerDot manager={manager.id} />
                      {manager.name}
                    </ItemTitle>
                    {manager.available && (
                      <ItemDescription className="font-mono text-xs">
                        {managerVersionLabel(manager)}
                      </ItemDescription>
                    )}
                  </ItemContent>
                  <ItemActions>
                    <ManagerState
                      manager={manager}
                      enabled={settings.settings?.enabledManagers.includes(manager.id) ?? manager.enabled}
                    />
                  </ItemActions>
                </Item>
              ))}
        </ItemGroup>
      </CardContent>
    </Card>
  );
}

function ManagerState({ manager, enabled }: { manager: ManagerInfo; enabled: boolean }) {
  if (!manager.available) return <Badge variant="outline">Not installed</Badge>;
  if (!enabled) return <Badge variant="secondary">Paused</Badge>;
  return (
    <Badge variant="outline" className="text-success">
      {manager.requiresAdmin ? "Active · Admin" : "Active"}
    </Badge>
  );
}
