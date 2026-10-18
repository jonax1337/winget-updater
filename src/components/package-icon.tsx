import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getPackageIcon } from "@/lib/api";
import { avatarColor, initials, packageKey } from "@/lib/packages";
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
    <Avatar className="size-10 rounded-xl after:rounded-xl" aria-hidden="true">
      {icon && (
        <AvatarImage
          src={icon}
          alt=""
          className="rounded-xl object-contain p-1"
        />
      )}
      <AvatarFallback
        className="rounded-xl text-xs font-semibold text-white"
        style={{ backgroundColor: avatarColor(name) }}
      >
        {initials(name)}
      </AvatarFallback>
    </Avatar>
  );
}
