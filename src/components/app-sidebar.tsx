import type { ComponentProps } from "react";
import {
  ArrowDownToLine,
  History,
  LayoutDashboard,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import { NavMain, type NavItem } from "@/components/nav-main";
import { NavSecondary } from "@/components/nav-secondary";
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
} from "@/components/ui/sidebar";
import { useUpdater } from "@/hooks/use-updater";
import type { Settings } from "@/lib/types";

export type Page = "overview" | "updates" | "history" | "settings";

export const PAGE_TITLES: Record<Page, string> = {
  overview: "Übersicht",
  updates: "Updates",
  history: "Verlauf",
  settings: "Einstellungen",
};

export function AppSidebar({
  page,
  navigate,
  ...props
}: {
  page: Page;
  navigate: (page: Page) => void;
} & ComponentProps<typeof Sidebar>) {
  const { actionable } = useUpdater();
  const primary: NavItem<Page>[] = [
    { id: "overview", title: PAGE_TITLES.overview, icon: <LayoutDashboard /> },
    {
      id: "updates",
      title: PAGE_TITLES.updates,
      icon: <ArrowDownToLine />,
      badge: actionable.length,
    },
    { id: "history", title: PAGE_TITLES.history, icon: <History /> },
  ];
  const secondary: NavItem<Page>[] = [
    { id: "settings", title: PAGE_TITLES.settings, icon: <Settings2 /> },
  ];
  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarContent>
        <NavMain items={primary} active={page} onNavigate={navigate} />
        <NavSecondary
          items={secondary}
          active={page}
          onNavigate={navigate}
          className="mt-auto"
        />
      </SidebarContent>
      <SidebarFooter className="group-data-[collapsible=icon]:hidden">
        <ServiceStatus />
      </SidebarFooter>
    </Sidebar>
  );
}

function ServiceStatus() {
  const { settings, busy, ready } = useUpdater();
  return (
    <Item variant="outline" size="sm">
      <ItemMedia variant="icon">
        <ShieldCheck />
      </ItemMedia>
      <ItemContent>
        <ItemTitle>{serviceTitle(ready, busy)}</ItemTitle>
        <ItemDescription>{automationDescription(settings.settings)}</ItemDescription>
      </ItemContent>
    </Item>
  );
}

function serviceTitle(ready: boolean, busy: boolean): string {
  if (!ready) return "Nicht verbunden";
  if (busy) return "Gerade aktiv";
  return "Läuft im Hintergrund";
}

function automationDescription(settings?: Settings): string {
  if (!settings) return "Desktop-App nicht verbunden";
  return settings.autoUpdate ? "Automatische Updates an" : "Automatische Updates aus";
}
