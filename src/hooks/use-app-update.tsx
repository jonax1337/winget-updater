import {
  createContext,
  useContext,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";
import { toAppError } from "@/lib/api";
import {
  CHECK_INTERVAL_MS,
  NO_DOWNLOAD,
  STARTUP_CHECK_DELAY_MS,
  applyDownloadEvent,
  downloadPercent,
  type DownloadProgress,
} from "@/lib/app-update";

type Status = "idle" | "checking" | "installing";

function useAppUpdateState() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [download, setDownload] = useState<DownloadProgress>(NO_DOWNLOAD);
  const announced = useRef<Update | null>(null);

  const announce = (next: Update | null) => {
    void announced.current?.close();
    announced.current = next;
    setUpdate(next);
  };

  const findUpdate = async () => {
    setStatus("checking");
    try {
      const found = await check();
      announce(found);
      return found;
    } finally {
      setStatus("idle");
    }
  };

  const checkInBackground = useEffectEvent(async () => {
    if (status !== "idle") return;
    try {
      await findUpdate();
    } catch {
      // Offline or no release yet: the next scheduled check tries again.
    }
  });

  const checkManually = async () => {
    try {
      if (!(await findUpdate())) toast.success("Winget Updater is up to date");
    } catch (error) {
      toast.error("Could not check for app updates", {
        description: toAppError(error).message,
      });
    }
  };

  const install = async () => {
    if (!update) return;
    setStatus("installing");
    setDownload(NO_DOWNLOAD);
    try {
      await update.downloadAndInstall((event) =>
        setDownload((progress) => applyDownloadEvent(progress, event)),
      );
      await relaunch();
    } catch (error) {
      setStatus("idle");
      toast.error("Could not install the update", {
        description: toAppError(error).message,
      });
    }
  };

  useEffect(() => {
    if (!isTauri()) return;
    const startup = window.setTimeout(() => void checkInBackground(), STARTUP_CHECK_DELAY_MS);
    const recurring = window.setInterval(() => void checkInBackground(), CHECK_INTERVAL_MS);
    return () => {
      window.clearTimeout(startup);
      window.clearInterval(recurring);
    };
  }, []);

  return {
    update,
    checking: status === "checking",
    installing: status === "installing",
    downloadPercent: downloadPercent(download),
    checkManually,
    install,
  };
}

type AppUpdate = ReturnType<typeof useAppUpdateState>;
const AppUpdateContext = createContext<AppUpdate | undefined>(undefined);

export function AppUpdateProvider({ children }: { children: ReactNode }) {
  return (
    <AppUpdateContext.Provider value={useAppUpdateState()}>
      {children}
    </AppUpdateContext.Provider>
  );
}

export function useAppUpdate(): AppUpdate {
  const context = useContext(AppUpdateContext);
  if (!context) throw new Error("AppUpdateProvider is missing");
  return context;
}
