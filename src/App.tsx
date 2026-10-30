import { useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Info } from "lucide-react";
import { AppSidebar, type Page } from "@/components/app-sidebar";
import { SiteHeader } from "@/components/site-header";
import { TitleBar } from "@/components/title-bar";
import { UpdateProgress } from "@/components/update-progress";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { UpdaterProvider, useUpdater } from "@/hooks/use-updater";
import { Catalog } from "@/pages/catalog";
import { History } from "@/pages/history";
import { Overview } from "@/pages/overview";
import { SettingsPage } from "@/pages/settings";
import { Software } from "@/pages/software";
import { Updates } from "@/pages/updates";

function App() {
  return (
    <TooltipProvider delayDuration={250}>
      <UpdaterProvider>
        <AppLayout />
      </UpdaterProvider>
    </TooltipProvider>
  );
}

function AppLayout() {
  const [page, setPage] = useState<Page>("overview");
  const { settings } = useUpdater();
  return (
    <div className="flex h-svh flex-col bg-sidebar">
      <TitleBar />
      <SidebarProvider className="min-h-0! flex-1">
        <AppSidebar
          page={page}
          navigate={setPage}
          variant="inset"
          className="top-titlebar h-[calc(100svh-var(--titlebar-height))]!"
        />
        <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
          <SiteHeader page={page} />
          <ScrollArea className="min-h-0 flex-1">
            <div className="@container/main flex flex-col gap-4 py-4 md:gap-6 md:py-6">
              <Notices />
              <CurrentPage page={page} navigate={setPage} />
            </div>
          </ScrollArea>
        </SidebarInset>
      </SidebarProvider>
      <Toaster
        theme={settings.settings?.theme ?? "system"}
        position="bottom-right"
        richColors
        closeButton
      />
    </div>
  );
}

function Notices() {
  return (
    <div className="flex flex-col gap-4 px-4 empty:hidden lg:px-6">
      {!isTauri() && (
        <Alert>
          <Info />
          <AlertTitle>Browser preview</AlertTitle>
          <AlertDescription>
            Package managers and settings are only available in the desktop app.
          </AlertDescription>
        </Alert>
      )}
      <UpdateProgress />
    </div>
  );
}

function CurrentPage({ page, navigate }: { page: Page; navigate: (page: Page) => void }) {
  switch (page) {
    case "overview":
      return <Overview navigate={navigate} />;
    case "updates":
      return <Updates />;
    case "software":
      return <Software />;
    case "catalog":
      return <Catalog />;
    case "history":
      return <History />;
    case "settings":
      return <SettingsPage />;
  }
}

export default App;
