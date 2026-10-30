import { Check, CircleDashed, X, type LucideIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import type { UpdateStatus } from "@/lib/types";

const STATUSES: Record<UpdateStatus, { label: string; icon?: LucideIcon; tone: string }> = {
  queued: { label: "Queued", icon: CircleDashed, tone: "text-muted-foreground" },
  running: { label: "Updating", tone: "text-info" },
  succeeded: { label: "Updated", icon: Check, tone: "text-success" },
  failed: { label: "Failed", icon: X, tone: "text-destructive" },
  skipped: { label: "Skipped", icon: CircleDashed, tone: "text-muted-foreground" },
};

export function StatusBadge({ status }: { status: UpdateStatus }) {
  const { label, icon: Icon, tone } = STATUSES[status];
  return (
    <Badge variant="outline" className={tone}>
      {Icon ? <Icon /> : <Spinner />}
      {label}
    </Badge>
  );
}
