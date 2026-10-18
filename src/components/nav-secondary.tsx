import type { ComponentProps } from "react";
import { NavMainItem, type NavItem } from "@/components/nav-main";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
} from "@/components/ui/sidebar";

export function NavSecondary<T extends string>({
  items,
  active,
  onNavigate,
  ...props
}: {
  items: NavItem<T>[];
  active: T;
  onNavigate: (id: T) => void;
} & ComponentProps<typeof SidebarGroup>) {
  return (
    <SidebarGroup {...props}>
      <SidebarGroupContent>
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
