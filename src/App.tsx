import { useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Info } from "lucide-react";
import { AppSidebar, navigation, type Page } from "@/components/app-sidebar";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { UpdateActions } from "@/components/update-actions";
import { UpdateProgress } from "@/components/update-progress";
import { UpdaterProvider, useUpdater } from "@/hooks/use-updater";
import { Overview } from "@/pages/overview";
import { Updates } from "@/pages/updates";
import { History } from "@/pages/history";
import { SettingsPage } from "@/pages/settings";

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
  const { settings, scanning, updating } = useUpdater();
  const title = navigation.find((item) => item.id === page)!.title;
  return (
    <SidebarProvider>
      <a href="#main-content" className="skip-link">
        Zum Inhalt springen
      </a>
      <AppSidebar page={page} navigate={setPage} />
      <SidebarInset className="min-w-0">
        <header className="sticky top-0 z-20 flex min-h-19 flex-wrap items-center justify-between gap-3 border-b bg-background/95 px-4 py-4 backdrop-blur-sm lg:px-8">
          <div className="flex items-center gap-3">
            <SidebarTrigger
              className="md:hidden"
              aria-label="Navigation öffnen"
            />
            <h1 className="text-sm font-semibold">{title}</h1>
            <span className="hidden h-4 border-l sm:block" />
            <span
              className="hidden text-xs text-muted-foreground sm:inline"
              role="status"
            >
              {scanning
                ? "Prüfung läuft"
                : updating
                  ? "Aktualisierung läuft"
                  : "Dein Software-Cockpit"}
            </span>
          </div>
          <UpdateActions />
        </header>
        <div
          id="main-content"
          tabIndex={-1}
          className="mx-auto w-full max-w-360 space-y-6 px-4 py-7 outline-none lg:px-8 lg:py-8"
        >
          {!isTauri() && (
            <div
              role="status"
              className="flex items-start gap-3 rounded-xl border bg-card p-4 text-sm"
            >
              <Info className="mt-0.5 size-4 shrink-0 text-primary" />
              <p className="leading-relaxed">
                Browseransicht · Paketmanager und Einstellungen sind nur in der
                Desktop-App verfügbar. Es werden keine Beispieldaten angezeigt.
              </p>
            </div>
          )}
          <UpdateProgress />
          {page === "overview" && <Overview navigate={setPage} />}
          {page === "updates" && <Updates />}
          {page === "history" && <History />}
          {page === "settings" && <SettingsPage />}
        </div>
      </SidebarInset>
      <Toaster
        theme={settings.settings?.theme ?? "system"}
        position="bottom-right"
        richColors
        closeButton
      />
    </SidebarProvider>
  );
}

export default App;
