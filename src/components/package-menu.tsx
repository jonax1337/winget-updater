import { useState, type ReactNode } from "react";
import { EllipsisVertical, Trash2 } from "lucide-react";
import { ConfirmAction } from "@/components/confirm-action";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useUpdater } from "@/hooks/use-updater";
import { MANAGER_NAMES, type NamedPackageRef } from "@/lib/packages";

/** The ⋮ menu of a package; `children` are the page-specific items above "Uninstall". */
export function PackageMenu({
  entry,
  onUninstalled,
  uninstallDisabled = false,
  children,
}: {
  entry: NamedPackageRef;
  onUninstalled?: () => void;
  uninstallDisabled?: boolean;
  children?: ReactNode;
}) {
  const { busy, ready, settings, uninstall } = useUpdater();
  const [confirming, setConfirming] = useState(false);
  const uninstallAndReport = async () => {
    if (await uninstall(entry)) onUninstalled?.();
  };
  // The dialog lives outside the menu content, which unmounts as soon as the menu closes.
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="size-8 text-muted-foreground data-[state=open]:bg-muted"
            aria-label={`Actions for ${entry.name}`}
          >
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-56">
          {children}
          {children && <DropdownMenuSeparator />}
          <DropdownMenuItem
            variant="destructive"
            disabled={busy || !ready || settings.saving || uninstallDisabled}
            onSelect={() => setConfirming(true)}
          >
            <Trash2 />
            Uninstall
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ConfirmAction
        open={confirming}
        onOpenChange={setConfirming}
        title={`Uninstall ${entry.name}?`}
        description={`This removes ${entry.name} using ${MANAGER_NAMES[entry.manager]}. This cannot be undone.`}
        confirm="Uninstall"
        onConfirm={() => void uninstallAndReport()}
      />
    </>
  );
}
