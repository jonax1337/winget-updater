import { Download, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useUpdater } from "@/hooks/use-updater";

export function UpdateActions({ large = false }: { large?: boolean }) {
  const { busy, ready, scanning, actionable, settings, check, update } =
    useUpdater();
  const size = large ? "lg" : "default";
  const disabled = busy || !ready || !actionable.length || settings.saving;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        variant="outline"
        size={size}
        disabled={busy || !ready || settings.saving}
        onClick={() => void check()}
      >
        <RefreshCw className={scanning ? "animate-spin" : undefined} />
        {scanning ? "Wird geprüft …" : "Jetzt prüfen"}
      </Button>
      <Tooltip>
        <TooltipTrigger asChild>
          {/* eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- Focus is needed only to expose the disabled button's tooltip. */}
          <span className="inline-flex" tabIndex={disabled ? 0 : undefined}>
            <Button
              size={size}
              disabled={disabled}
              onClick={() => void update()}
            >
              <Download />
              Alle aktualisieren
              {actionable.length > 0 && (
                <span className="ml-1 rounded bg-primary-foreground/15 px-1.5 py-0.5 text-xs tabular-nums">
                  {actionable.length}
                </span>
              )}
            </Button>
          </span>
        </TooltipTrigger>
        <TooltipContent>
          Ignorierte Pakete werden nicht aktualisiert.
        </TooltipContent>
      </Tooltip>
    </div>
  );
}
