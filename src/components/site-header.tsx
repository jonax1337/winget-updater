import { PAGE_TITLES, type Page } from "@/components/app-sidebar";
import { Separator } from "@/components/ui/separator";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { useUpdater } from "@/hooks/use-updater";

export function SiteHeader({ page }: { page: Page }) {
  return (
    <header className="flex h-header shrink-0 items-center gap-2 border-b">
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" aria-label="Toggle sidebar" />
        <Separator
          orientation="vertical"
          className="mx-2 data-[orientation=vertical]:h-4"
        />
        <h1 className="text-base font-medium">{PAGE_TITLES[page]}</h1>
        <ActivityStatus />
      </div>
    </header>
  );
}

function ActivityStatus() {
  const { scanning, updating } = useUpdater();
  if (!scanning && !updating) return null;
  return (
    <span role="status" className="ml-auto flex items-center gap-2 text-sm text-muted-foreground">
      <Spinner />
      {scanning ? "Checking for updates" : "Updating"}
    </span>
  );
}
