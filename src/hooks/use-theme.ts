import { useEffect } from "react";
import type { Theme } from "@/lib/types";

export function useTheme(theme: Theme = "system") {
  useEffect(() => {
    const system = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && system.matches);
      document.documentElement.classList.toggle("dark", dark);
      document.documentElement.style.colorScheme = dark ? "dark" : "light";
    };
    apply();
    system.addEventListener("change", apply);
    return () => system.removeEventListener("change", apply);
  }, [theme]);
}
