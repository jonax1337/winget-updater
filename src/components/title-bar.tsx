import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

export function TitleBar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-titlebar shrink-0 items-center justify-between bg-sidebar pl-3 text-sidebar-foreground"
    >
      <div data-tauri-drag-region className="flex items-center gap-2 text-xs font-medium">
        <AppLogo />
        Winget Updater
      </div>
      {isTauri() && <WindowControls />}
    </header>
  );
}

/** Same motif as public/logo.svg, drawn with theme tokens so it follows light and dark mode. */
function AppLogo() {
  return (
    <svg viewBox="0 0 1024 1024" className="size-5" aria-hidden="true">
      <rect width="1024" height="1024" rx="160" className="fill-primary" />
      <g className="fill-primary-foreground">
        <rect x="224" y="224" width="248" height="248" rx="40" />
        <rect x="552" y="224" width="248" height="248" rx="40" />
        <rect x="224" y="552" width="248" height="248" rx="40" />
      </g>
      <path
        d="M676 576V768M604 696L676 768L748 696"
        className="fill-none stroke-primary-foreground"
        strokeWidth={64}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function WindowControls() {
  const appWindow = getCurrentWindow();
  const maximized = useMaximized();
  return (
    <div className="flex h-full">
      <Button
        variant="ghost"
        className="h-full w-11 rounded-none"
        aria-label="Minimieren"
        onClick={() => void appWindow.minimize()}
      >
        <Minus />
      </Button>
      <Button
        variant="ghost"
        className="h-full w-11 rounded-none"
        aria-label={maximized ? "Wiederherstellen" : "Maximieren"}
        onClick={() => void appWindow.toggleMaximize()}
      >
        {maximized ? <Copy className="size-3.5" /> : <Square className="size-3.5" />}
      </Button>
      <Button
        variant="ghost"
        className="h-full w-11 rounded-none hover:bg-destructive hover:text-white dark:hover:bg-destructive"
        aria-label="In den Infobereich schließen"
        onClick={() => void appWindow.close()}
      >
        <X />
      </Button>
    </div>
  );
}

function useMaximized(): boolean {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const sync = () => void appWindow.isMaximized().then(setMaximized);
    sync();
    const unlisten = appWindow.onResized(sync);
    return () => void unlisten.then((stop) => stop());
  }, []);
  return maximized;
}
