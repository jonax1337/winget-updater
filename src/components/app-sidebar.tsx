import {
  ArrowDownToLine,
  Blocks,
  History,
  LayoutDashboard,
  Settings2,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { useUpdater } from "@/hooks/use-updater";

import type { Settings } from "@/lib/types";

function backgroundStatus(ready: boolean, busy: boolean): string {
  if (!ready) return "Desktop-Verbindung";
  if (busy) return "Gerade für dich aktiv";
  return "Läuft im Hintergrund";
}

function automationDescription(settings?: Settings): string {
  if (!settings) return "Einstellungen sind erst verfügbar, wenn die Desktop-App verbunden ist.";
  if (settings.autoUpdate) return "Updates werden automatisch installiert. Du behältst die Kontrolle.";
  return "Automatische Updates sind aus. Neue Versionen bleiben hier sichtbar.";
}

export type Page = "overview" | "updates" | "history" | "settings";
export const navigation: { id: Page; title: string; icon: LucideIcon }[] = [
  { id: "overview", title: "Übersicht", icon: LayoutDashboard },
  { id: "updates", title: "Updates", icon: ArrowDownToLine },
  { id: "history", title: "Verlauf", icon: History },
  { id: "settings", title: "Einstellungen", icon: Settings2 },
];

export function AppSidebar({
  page,
  navigate,
}: {
  page: Page;
  navigate: (page: Page) => void;
}) {
  const { actionable, settings, busy, ready } = useUpdater();
  const { isMobile, setOpenMobile } = useSidebar();
  const select = (next: Page) => {
    navigate(next);
    if (isMobile) setOpenMobile(false);
  };
  return (
    <Sidebar className="border-r" collapsible="offcanvas">
      <SidebarHeader className="px-5 pb-8 pt-7">
        <div className="flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-xl bg-primary text-primary-foreground">
            <Blocks className="size-5" strokeWidth={1.8} />
          </div>
          <div>
            <p className="text-sm font-semibold tracking-tight">
              Winget Updater
            </p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Software, auf dem neuesten Stand.
            </p>
          </div>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup className="px-3">
          <SidebarGroupContent>
            <nav aria-label="Hauptnavigation">
              <SidebarMenu className="gap-1">
                {navigation.map(({ id, title, icon: Icon }) => (
                  <SidebarMenuItem key={id}>
                    <SidebarMenuButton
                      className="h-11 gap-3 px-3"
                      isActive={page === id}
                      aria-current={page === id ? "page" : undefined}
                      onClick={() => select(id)}
                    >
                      <Icon strokeWidth={1.7} />
                      <span>{title}</span>
                    </SidebarMenuButton>
                    {id === "updates" && actionable.length > 0 && (
                      <SidebarMenuBadge className="right-3 top-3! bg-primary/10 text-primary">
                        {actionable.length}
                      </SidebarMenuBadge>
                    )}
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </nav>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter className="gap-5 px-5 pb-6">
        <div className="rounded-xl border bg-background/60 p-3.5">
          <div className="flex items-center gap-2 text-xs font-medium">
            <ShieldCheck className="size-4 text-primary" />
            {backgroundStatus(ready, busy)}
          </div>
          <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
            {automationDescription(settings.settings)}
          </p>
        </div>
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>Winget Updater</span>
          <span>v0.1.0</span>
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
