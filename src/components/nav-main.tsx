import type { ReactNode } from "react";
import { Download, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { useUpdater } from "@/hooks/use-updater";

export interface NavItem<T extends string> {
  id: T;
  title: string;
  icon: ReactNode;
  badge?: number;
}

export function NavMain<T extends string>({
  items,
  active,
  onNavigate,
}: {
  items: NavItem<T>[];
  active: T;
  onNavigate: (id: T) => void;
}) {
  const { busy, ready, scanning, actionable, settings, check, update } = useUpdater();
  const locked = busy || !ready || settings.saving;
  return (
    <SidebarGroup>
      <SidebarGroupContent className="flex flex-col gap-2">
        <SidebarMenu className="gap-1">
          <SidebarMenuItem className="flex items-center gap-2">
            <SidebarMenuButton
              tooltip="Update all"
              disabled={locked || !actionable.length}
              onClick={() => void update()}
              className="min-w-8 bg-brand text-brand-foreground duration-200 ease-linear hover:bg-brand-hover hover:text-brand-foreground active:bg-brand-hover active:text-brand-foreground"
            >
              <Download />
              <span>Update all</span>
            </SidebarMenuButton>
            <Button
              size="icon"
              variant="outline"
              className="size-8 group-data-[collapsible=icon]:opacity-0"
              aria-label="Check now"
              disabled={locked}
              onClick={() => void check()}
            >
              <RefreshCw className={scanning ? "animate-spin" : undefined} />
            </Button>
          </SidebarMenuItem>
        </SidebarMenu>
        <SidebarMenu className="gap-1">
          {items.map((item) => (
            <NavMainItem
              key={item.id}
              item={item}
              active={item.id === active}
              onSelect={() => onNavigate(item.id)}
            />
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}

export function NavMainItem<T extends string>({
  item,
  active,
  onSelect,
}: {
  item: NavItem<T>;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        tooltip={item.title}
        isActive={active}
        className="data-active:bg-sidebar-primary data-active:text-sidebar-primary-foreground"
        aria-current={active ? "page" : undefined}
        onClick={onSelect}
      >
        {item.icon}
        <span>{item.title}</span>
      </SidebarMenuButton>
      {Boolean(item.badge) && <SidebarMenuBadge>{item.badge}</SidebarMenuBadge>}
    </SidebarMenuItem>
  );
}
