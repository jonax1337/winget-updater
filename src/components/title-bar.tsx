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
        <img src="/logo.svg" alt="" className="pointer-events-none size-5" />
        Winget Updater
      </div>
      {isTauri() && <WindowControls />}
    </header>
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
        aria-label="Minimize"
        onClick={() => void appWindow.minimize()}
      >
        <Minus />
      </Button>
      <Button
        variant="ghost"
        className="h-full w-11 rounded-none"
        aria-label={maximized ? "Restore" : "Maximize"}
        onClick={() => void appWindow.toggleMaximize()}
      >
        {maximized ? <Copy className="size-3.5" /> : <Square className="size-3.5" />}
      </Button>
      <Button
        variant="ghost"
        className="h-full w-11 rounded-none hover:bg-destructive hover:text-white dark:hover:bg-destructive"
        aria-label="Close to tray"
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
