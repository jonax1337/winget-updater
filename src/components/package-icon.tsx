import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getPackageIcon } from "@/lib/api";
import { initials, packageKey } from "@/lib/packages";
import type { PackageRef } from "@/lib/types";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";

const icons = new Map<string, Promise<string | null>>();

function loadIcon(reference: PackageRef, name: string) {
  const key = packageKey(reference);
  if (!icons.has(key)) {
    icons.set(
      key,
      getPackageIcon(reference, name).catch(() => {
        icons.delete(key);
        return null;
      }),
    );
  }
  return icons.get(key)!;
}

export function PackageIcon({
  package: reference,
  name,
}: {
  package: PackageRef;
  name: string;
}) {
  const [icon, setIcon] = useState<string | null>(null);
  const { manager, id } = reference;
  useEffect(() => {
    let active = true;
    setIcon(null);
    if (isTauri())
      void loadIcon({ manager, id }, name).then((value) => {
        if (active) setIcon(value);
      });
    return () => {
      active = false;
    };
  }, [manager, id, name]);
  return (
    <Avatar className="size-8 rounded-lg after:rounded-lg" aria-hidden="true">
      {icon && <AvatarImage src={icon} alt="" className="rounded-lg object-contain p-0.5" />}
      <AvatarFallback className="rounded-lg text-xs font-medium">
        {initials(name)}
      </AvatarFallback>
    </Avatar>
  );
}
