import { Badge } from "@/components/ui/badge";
import { MANAGER_NAMES } from "@/lib/packages";
import type { ManagerId } from "@/lib/types";

const MANAGER_DOTS: Record<ManagerId, string> = {
  winget: "bg-manager-winget",
  scoop: "bg-manager-scoop",
  chocolatey: "bg-manager-chocolatey",
  npm: "bg-manager-npm",
  pip: "bg-manager-pip",
};

export function ManagerBadge({ manager }: { manager: ManagerId }) {
  return (
    <Badge variant="outline" className="text-muted-foreground">
      <span aria-hidden="true" className={`size-1.5 rounded-full ${MANAGER_DOTS[manager]}`} />
      {MANAGER_NAMES[manager]}
    </Badge>
  );
}
