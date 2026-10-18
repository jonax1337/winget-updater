import { CheckCheck, LoaderCircle } from "lucide-react";
import { Progress } from "@/components/ui/progress";
import { useUpdater } from "@/hooks/use-updater";

export function UpdateProgress() {
  const { progress } = useUpdater();
  if (!progress.total && !progress.summary) return null;
  const updating = progress.total > 0 && !progress.summary;
  const total = Math.max(progress.total, progress.completed);
  const value = total ? (progress.completed / total) * 100 : 0;
  const running = Array.from(progress.events.values()).find(
    (event) => event.status === "running",
  );
  return (
    <section
      className="rounded-xl border bg-card px-5 py-4"
      aria-label="Update-Fortschritt"
      aria-live="polite"
    >
      <div className="mb-3 flex flex-wrap items-center gap-3">
        {updating ? (
          <LoaderCircle className="size-5 animate-spin text-primary" />
        ) : (
          <CheckCheck className="size-5 text-primary" />
        )}
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium">
            {updating
              ? "Anwendungen werden aktualisiert"
              : "Aktualisierung abgeschlossen"}
          </p>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">
            {running
              ? running.package.id
              : "Ein Paket nach dem anderen. Die App kann im Hintergrund weiterlaufen."}
          </p>
        </div>
        <span className="text-sm font-medium tabular-nums">
          {progress.completed} von {total || "…"}
        </span>
      </div>
      <Progress
        value={value}
        aria-label="Abgeschlossene Pakete"
        aria-valuenow={progress.completed}
        aria-valuemax={total || 1}
      />
      {progress.summary && (
        <p className="mt-2 text-xs text-muted-foreground">
          {progress.summary.succeeded} erfolgreich · {progress.summary.failed}{" "}
          fehlgeschlagen
        </p>
      )}
    </section>
  );
}
