import { Check, CircleDashed, LoaderCircle, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { UpdateStatus } from "@/lib/types";

const statuses = {
  queued: { label: "Wartend", icon: CircleDashed, className: "status-queued" },
  running: {
    label: "Wird aktualisiert",
    icon: LoaderCircle,
    className: "status-running",
  },
  succeeded: {
    label: "Aktualisiert",
    icon: Check,
    className: "status-succeeded",
  },
  failed: { label: "Fehlgeschlagen", icon: X, className: "status-failed" },
  skipped: { label: "Übersprungen", icon: CircleDashed, className: "text-muted-foreground" },
};

export function StatusBadge({
  status,
  message,
}: {
  status: UpdateStatus;
  message?: string | null;
}) {
  const { label, icon: Icon, className } = statuses[status];
  const badge = (
    <Badge variant="secondary" className={className}>
      <Icon
        className={
          status === "running" || status === "queued"
            ? "animate-spin"
            : undefined
        }
      />
      {label}
      {message && <span className="sr-only">{message}</span>}
    </Badge>
  );
  if (!message) return badge;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button type="button" aria-label={`${label}: Details`}>
          {badge}
        </button>
      </TooltipTrigger>
      <TooltipContent className="max-w-sm break-words">
        {message}
      </TooltipContent>
    </Tooltip>
  );
}
