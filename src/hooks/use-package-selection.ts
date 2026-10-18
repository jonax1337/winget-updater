import { useState } from "react";
import { packageKey } from "@/lib/packages";
import type { Package } from "@/lib/types";

export function usePackageSelection(filtered: Package[], actionable: Package[]) {
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const actionableKeys = new Set(actionable.map(packageKey));
  const eligible = filtered.filter((entry) => actionableKeys.has(packageKey(entry)));
  const selection = actionable.filter((entry) => selected.has(packageKey(entry)));
  const checked = eligible.filter((entry) => selected.has(packageKey(entry))).length;

  const selectVisible = (value: boolean) => {
    setSelected((previous) => {
      const next = new Set(previous);
      for (const entry of eligible) {
        if (value) next.add(packageKey(entry));
        else next.delete(packageKey(entry));
      }
      return next;
    });
  };

  const toggle = (key: string) => {
    setSelected((previous) => {
      const next = new Set(previous);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return { selected, eligible, selection, checked, selectVisible, toggle };
}
