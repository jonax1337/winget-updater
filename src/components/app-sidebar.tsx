import type { ComponentProps } from "react";
import {
  ArrowDownToLine,
  History,
  LayoutDashboard,
  Settings2,
} from "lucide-react";
import { NavMain, type NavItem } from "@/components/nav-main";
import { NavSecondary } from "@/components/nav-secondary";
import { Sidebar, SidebarContent } from "@/components/ui/sidebar";
import { useUpdater } from "@/hooks/use-updater";

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
    </Sidebar>
  );
}
