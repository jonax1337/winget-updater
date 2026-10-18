import { Badge } from "@/components/ui/badge";
import { MANAGER_NAMES } from "@/lib/packages";
import type { ManagerId } from "@/lib/types";

export function ManagerBadge({ manager }: { manager: ManagerId }) {
  return (
    <Badge variant="outline" className={`manager-badge manager-${manager}`}>
      {MANAGER_NAMES[manager]}
    </Badge>
  );
}
