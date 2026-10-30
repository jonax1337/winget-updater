import { Fragment, useState, type ReactNode } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { Check, Download, EyeOff, RefreshCw, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Combobox } from "@/components/combobox";
import { ResourceError } from "@/components/resource-error";
import {
  NumberStepper,
  SettingField,
  SwitchField,
} from "@/components/setting-field";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { FieldGroup, FieldSeparator } from "@/components/ui/field";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemGroup,
  ItemTitle,
} from "@/components/ui/item";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { useAppUpdate } from "@/hooks/use-app-update";
import { useResource } from "@/hooks/use-resource";
import { useUpdater } from "@/hooks/use-updater";
import { installWingetModule, toAppError } from "@/lib/api";
import { managerVersionLabel, packageKey, parsePackageKey } from "@/lib/packages";
import type { ManagerInfo, Settings, Theme } from "@/lib/types";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "Follow Windows" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function SettingsPage() {
  const { settings } = useUpdater();
  const current = settings.settings;
  return (
    <div className="flex max-w-4xl flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={settings.error} retry={settings.refresh} />
      {settings.loading && <Skeleton className="h-96" />}
      {current && (
        <>
          <Automation current={current} />
          <Managers current={current} />
          <Appearance current={current} />
          <Winget current={current} />
          <IgnoredPackages current={current} />
        </>
      )}
      {isTauri() && <AppUpdates />}
    </div>
  );
}

function Section({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  const { settings } = useUpdater();
  return (
    <Card>
      <CardHeader className="border-b">
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
        <CardAction className="text-muted-foreground" aria-live="polite">
          {settings.saving ? <Spinner /> : <Check className="size-4" aria-label="Saved" />}
        </CardAction>
      </CardHeader>
      <CardContent>
        <FieldGroup>{children}</FieldGroup>
      </CardContent>
    </Card>
  );
}

function Automation({ current }: { current: Settings }) {
  const { settings } = useUpdater();
  return (
    <Section title="Automation" description="When to check for and install updates.">
      <SwitchField
        label="Check at sign-in"
        description="Check for updates once after startup."
        checked={current.updateOnStartup}
        onChange={(updateOnStartup) => void settings.patch({ updateOnStartup })}
      />
      <FieldSeparator />
      <SettingField label="Startup delay" description="Let Windows finish starting.">
        {(ids) => (
          <NumberStepper
            id={ids.control}
            describedBy={ids.description}
            value={current.startupDelayMinutes}
            min={0}
            max={60}
            unit="min"
            disabled={!current.updateOnStartup}
            onChange={(startupDelayMinutes) => void settings.patch({ startupDelayMinutes })}
          />
        )}
      </SettingField>
      <FieldSeparator />
      <SwitchField
        label="Scheduled checks"
        description="Check periodically while the app is running."
        checked={current.periodicChecks}
        onChange={(periodicChecks) => void settings.patch({ periodicChecks })}
      />
      <FieldSeparator />
      <SettingField label="Check interval" description="Time between scheduled checks.">
        {(ids) => (
          <NumberStepper
            id={ids.control}
            describedBy={ids.description}
            value={current.checkIntervalHours}
            min={1}
            max={168}
            unit="hr"
            disabled={!current.periodicChecks}
            onChange={(checkIntervalHours) => void settings.patch({ checkIntervalHours })}
          />
        )}
      </SettingField>
      <FieldSeparator />
      <SwitchField
        label="Install updates automatically"
        description="When off, only check and notify."
        checked={current.autoUpdate}
        onChange={(autoUpdate) => void settings.patch({ autoUpdate })}
      />
      <FieldSeparator />
      <SwitchField
        label="Start with Windows"
        description="Start minimized in the system tray at sign-in."
        checked={current.autostart}
        onChange={(autostart) => void settings.patch({ autostart })}
      />
    </Section>
  );
}

function Managers({ current }: { current: Settings }) {
  const { managers, settings, busy } = useUpdater();
  const enabled = current.enabledManagers;
  return (
    <Section title="Package managers" description="Only installed, enabled sources are checked.">
      <ResourceError error={managers.error} retry={managers.refresh} />
      {managers.loading && <Skeleton className="h-48" />}
      {managers.managers.map((manager, index) => (
        <Fragment key={manager.id}>
          {index > 0 && <FieldSeparator />}
          <SwitchField
            label={`${manager.name} · ${managerVersionLabel(manager)}`}
            description={managerHint(manager)}
            checked={enabled.includes(manager.id)}
            disabled={!manager.available || busy || settings.saving}
            onChange={(checked) =>
              void settings.patch({
                enabledManagers: checked
                  ? [...enabled, manager.id]
                  : enabled.filter((id) => id !== manager.id),
              })
            }
          />
        </Fragment>
      ))}
    </Section>
  );
}

function managerHint(manager: ManagerInfo): string {
  if (!manager.available) return "Install this package manager to enable it.";
  if (manager.requiresAdmin) return "Updates require administrator privileges.";
  return "Include this source when checking for updates.";
}

function Appearance({ current }: { current: Settings }) {
  const { settings } = useUpdater();
  return (
    <Section title="Notifications & appearance" description="Choose notifications and a theme.">
      <SwitchField
        label="Windows notifications"
        description="Notify when updates are found or finished."
        checked={current.notify}
        onChange={(notify) => void settings.patch({ notify })}
      />
      <FieldSeparator />
      <SettingField label="Theme" description="Light, dark, or follow Windows.">
        {(ids) => (
          <Combobox
            label="Theme"
            aria-describedby={ids.description}
            value={current.theme}
            options={THEMES}
            onChange={(theme) => void settings.patch({ theme })}
            className="w-40"
          />
        )}
      </SettingField>
    </Section>
  );
}

function Winget({ current }: { current: Settings }) {
  const { settings } = useUpdater();
  return (
    <Section title="WinGet detection" description="Accurate package IDs, including long names.">
      <SwitchField
        label="Include unknown versions"
        description="Check packages whose installed version WinGet cannot identify."
        checked={current.includeUnknownVersions}
        onChange={(includeUnknownVersions) => void settings.patch({ includeUnknownVersions })}
      />
      <FieldSeparator />
      <SettingField
        label="PowerShell module"
        description="Use Microsoft.WinGet.Client for accurate IDs. Does not install updates."
      >
        {(ids) => <InstallWingetModule id={ids.control} describedBy={ids.description} />}
      </SettingField>
    </Section>
  );
}

function InstallWingetModule({ id, describedBy }: { id: string; describedBy: string }) {
  const { busy, managers } = useUpdater();
  const [installing, setInstalling] = useState(false);
  const install = async () => {
    setInstalling(true);
    try {
      await installWingetModule();
      toast.success("WinGet PowerShell module installed", {
        description: "The next check will use the improved detection.",
      });
      void managers.refresh();
    } catch (error) {
      toast.error("Module installation failed", {
        description: toAppError(error).message,
      });
    } finally {
      setInstalling(false);
    }
  };
  return (
    <Button
      id={id}
      variant="outline"
      aria-describedby={describedBy}
      disabled={installing || busy}
      onClick={() => void install()}
    >
      {installing ? <Spinner data-icon="inline-start" /> : <Download data-icon="inline-start" />}
      Install
    </Button>
  );
}

function AppUpdates() {
  const { data: version } = useResource(getVersion, "");
  const { update, checking, installing, checkManually } = useAppUpdate();
  const description = update
    ? `Version ${update.version} is available.`
    : `Current version ${version || "unknown"}.`;
  return (
    <Card>
      <CardHeader className="border-b">
        <CardTitle>App updates</CardTitle>
        <CardDescription>Winget Updater checks for new versions once a day.</CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <SettingField label="Winget Updater" description={description}>
            {(ids) => (
              <Button
                id={ids.control}
                variant="outline"
                aria-describedby={ids.description}
                disabled={checking || installing}
                onClick={() => void checkManually()}
              >
                {checking ? <Spinner data-icon="inline-start" /> : <RefreshCw data-icon="inline-start" />}
                Check for updates
              </Button>
            )}
          </SettingField>
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function IgnoredPackages({ current }: { current: Settings }) {
  const { settings, packages, busy } = useUpdater();
  const ignored = current.ignoredPackages;
  return (
    <Card>
      <CardHeader>
        <CardTitle>Ignored packages</CardTitle>
        <CardDescription>Excluded from automatic updates.</CardDescription>
      </CardHeader>
      <CardContent>
        {ignored.length ? (
          <ItemGroup className="gap-2">
            {ignored.map((key) => {
              const reference = parsePackageKey(key);
              const name =
                packages.find((candidate) => packageKey(candidate) === key)?.name ?? reference?.id ?? key;
              return (
                <Item key={key} variant="outline" size="sm">
                  <ItemContent>
                    <ItemTitle>{name}</ItemTitle>
                    <ItemDescription className="font-mono">{key}</ItemDescription>
                  </ItemContent>
                  <ItemActions>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Stop ignoring ${name}`}
                      disabled={busy || settings.saving}
                      onClick={() =>
                        reference
                          ? void settings.ignore(reference, false)
                          : void settings.patch({
                              ignoredPackages: ignored.filter((entry) => entry !== key),
                            })
                      }
                    >
                      <Trash2 data-icon="inline-start" />
                      Stop ignoring
                    </Button>
                  </ItemActions>
                </Item>
              );
            })}
          </ItemGroup>
        ) : (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <EyeOff />
              </EmptyMedia>
              <EmptyTitle>No ignored packages</EmptyTitle>
              <EmptyDescription>
                Ignore packages from the action menu on the Updates page.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </CardContent>
    </Card>
  );
}
