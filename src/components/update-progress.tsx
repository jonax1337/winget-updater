import { CheckCheck } from "lucide-react";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemFooter,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { useUpdater } from "@/hooks/use-updater";
import type { UpdateSummary } from "@/lib/types";

export function UpdateProgress() {
  const { progress } = useUpdater();
  if (!progress.total && !progress.summary) return null;
  const updating = progress.total > 0 && !progress.summary;
  const total = Math.max(progress.total, progress.completed);
  const running = Array.from(progress.events.values()).find(
    (event) => event.status === "running",
  );
  return (
    <Item variant="outline" aria-label="Update progress" aria-live="polite">
      <ItemMedia variant="icon">{updating ? <Spinner /> : <CheckCheck />}</ItemMedia>
      <ItemContent>
        <ItemTitle>
          {updating ? "Updating apps" : "Update complete"}
        </ItemTitle>
        <ItemDescription>{progressDetail(running?.package.id, progress.summary)}</ItemDescription>
      </ItemContent>
      <ItemActions className="text-sm font-medium tabular-nums">
        {progress.completed} of {total || "…"}
      </ItemActions>
      <ItemFooter>
        <Progress
          value={total ? (progress.completed / total) * 100 : 0}
          aria-label="Completed packages"
        />
      </ItemFooter>
    </Item>
  );
}

function progressDetail(runningId?: string, summary?: UpdateSummary | null): string {
  if (summary) return `${summary.succeeded} succeeded · ${summary.failed} failed`;
  return runningId ?? "One package at a time. The app can keep running in the background.";
}
