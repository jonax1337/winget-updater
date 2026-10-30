import { ArrowUpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemFooter,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { useAppUpdate } from "@/hooks/use-app-update";
import { useUpdater } from "@/hooks/use-updater";
import { summarizeReleaseNotes } from "@/lib/app-update";

export function AppUpdateNotice() {
  const { update, checking, installing, downloadPercent, install } = useAppUpdate();
  const { busy } = useUpdater();
  if (!update) return null;
  const description = busy
    ? "Available once the running package operation has finished."
    : (summarizeReleaseNotes(update.body) ?? "A new version is ready to install.");
  return (
    <Item variant="outline" aria-label="App update" aria-live="polite">
      <ItemMedia variant="icon">
        <ArrowUpCircle />
      </ItemMedia>
      <ItemContent>
        <ItemTitle>Winget Updater {update.version} is available</ItemTitle>
        <ItemDescription>{description}</ItemDescription>
      </ItemContent>
      <ItemActions>
        <Button disabled={busy || checking || installing} onClick={() => void install()}>
          {installing && <Spinner data-icon="inline-start" />}
          {installing ? "Installing…" : "Install and restart"}
        </Button>
      </ItemActions>
      {installing && (
        <ItemFooter>
          <Progress value={downloadPercent ?? 0} aria-label="Update download" />
        </ItemFooter>
      )}
    </Item>
  );
}
