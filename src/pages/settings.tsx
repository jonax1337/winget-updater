import { useState, type ReactNode } from "react";
import {
  Bell,
  Blocks,
  Check,
  Clock3,
  Download,
  EyeOff,
  LoaderCircle,
  Monitor,
  ShieldCheck,
  Terminal,
  Trash2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { EmptyState } from "@/components/empty-state";
import { ManagerBadge } from "@/components/manager-badge";
import { ResourceError } from "@/components/resource-error";
import {
  NumberSetting,
  SettingRow,
  ToggleSetting,
} from "@/components/settings-fields";
import { useUpdater } from "@/hooks/use-updater";
import { installWingetModule, toAppError } from "@/lib/api";
import { managerVersionLabel, packageKey, parsePackageKey } from "@/lib/packages";
import type { Settings, Theme } from "@/lib/types";

export function SettingsPage() {
  const { settings, busy } = useUpdater();
  const current = settings.settings;
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="page-title">So, wie es zu dir passt.</h2>
          <p className="page-description">
            Lege fest, wann und wie deine Software aktuell bleibt.
          </p>
        </div>
        <span
          className="flex items-center gap-1.5 text-xs text-muted-foreground"
          role="status"
        >
          {settings.saving ? (
            <>
              <LoaderCircle className="size-3.5 animate-spin" />
              Wird gespeichert …
            </>
          ) : (
            <>
              <Check className="size-3.5" />
              Änderungen werden sofort gespeichert
            </>
          )}
        </span>
      </div>
      <ResourceError error={settings.error} retry={settings.refresh} />
      {settings.loading ? (
        <div className="space-y-4">
          <Skeleton className="h-64 w-full" />
          <Skeleton className="h-48 w-full" />
        </div>
      ) : (
        current && (
          <>
            <SettingsSection
              icon={ShieldCheck}
              title="Automatisierung"
              description="Weniger manuell erledigen. Mit deinem Zeitplan."
            >
              <ToggleSetting
                title="Updates automatisch installieren"
                description="Alle verfügbaren, nicht ignorierten Pakete nach einer Prüfung aktualisieren."
                checked={current.autoUpdate}
                onChange={(autoUpdate) => void settings.patch({ autoUpdate })}
              />
              <SettingRow
                title="Prüfintervall"
                description="In diesem Abstand nach neuen Versionen suchen."
              >
                {(describedBy) => <NumberSetting
                  describedBy={describedBy}
                  label="Prüfintervall in Stunden"
                  value={current.checkIntervalHours}
                  min={1}
                  max={168}
                  unit="Stunden"
                  onChange={(checkIntervalHours) =>
                    void settings.patch({ checkIntervalHours })
                  }
                />}
              </SettingRow>
              <ToggleSetting
                title="Nach dem App-Start prüfen"
                description="Nach der Startverzögerung eine erste Prüfung ausführen."
                checked={current.updateOnStartup}
                onChange={(updateOnStartup) =>
                  void settings.patch({ updateOnStartup })
                }
              />
              <SettingRow
                title="Startverzögerung"
                description="Windows und deinen Anwendungen Zeit zum Starten geben."
              >
                {(describedBy) => <NumberSetting
                  describedBy={describedBy}
                  label="Startverzögerung in Minuten"
                  value={current.startupDelayMinutes}
                  min={0}
                  max={60}
                  unit="Minuten"
                  onChange={(startupDelayMinutes) =>
                    void settings.patch({ startupDelayMinutes })
                  }
                />}
              </SettingRow>
              <ToggleSetting
                title="Mit Windows starten"
                description="Die App beim Anmelden minimiert im Infobereich starten."
                checked={current.autostart}
                onChange={(autostart) => void settings.patch({ autostart })}
              />
            </SettingsSection>
            <SettingsSection
              icon={Blocks}
              title="Paketmanager"
              description="Nur installierte und aktivierte Quellen werden geprüft."
            >
              <ManagersSettings current={current} />
            </SettingsSection>
            <SettingsSection
              icon={Bell}
              title="Benachrichtigungen & Darstellung"
              description="Das Wichtigste mitbekommen, ohne unterbrochen zu werden."
            >
              <ToggleSetting
                title="Windows-Benachrichtigungen"
                description="Über gefundene Updates und abgeschlossene Aktualisierungen informieren."
                checked={current.notify}
                onChange={(notify) => void settings.patch({ notify })}
              />
              <SettingRow
                title="Erscheinungsbild"
                description="Hell, dunkel oder automatisch passend zu Windows."
              >
                {(describedBy) => <Select
                  value={current.theme}
                  onValueChange={(theme: Theme) =>
                    void settings.patch({ theme })
                  }
                >
                  <SelectTrigger
                    className="min-w-36"
                    aria-label="Erscheinungsbild"
                    aria-describedby={describedBy}
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="system">
                      <Monitor className="size-4" />
                      System
                    </SelectItem>
                    <SelectItem value="light">Hell</SelectItem>
                    <SelectItem value="dark">Dunkel</SelectItem>
                  </SelectContent>
                </Select>}
              </SettingRow>
            </SettingsSection>
            <SettingsSection
              icon={Terminal}
              title="WinGet-Erkennung"
              description="Zuverlässige Paket-IDs, auch bei langen Anwendungsnamen."
            >
              <ToggleSetting
                title="Unbekannte Versionen einbeziehen"
                description="Auch Pakete prüfen, deren installierte Version WinGet nicht ermitteln kann."
                checked={current.includeUnknownVersions}
                onChange={(includeUnknownVersions) =>
                  void settings.patch({ includeUnknownVersions })
                }
              />
              <WingetModule />
            </SettingsSection>
            <SettingsSection
              icon={EyeOff}
              title="Ignorierte Pakete"
              description="Diese Pakete werden nie automatisch aktualisiert. Du kannst sie jederzeit wieder freigeben."
            >
              <IgnoredPackages current={current} disabled={busy || settings.saving} />
            </SettingsSection>
          </>
        )
      )}
      <p className="flex items-center gap-2 text-xs text-muted-foreground">
        <Clock3 className="size-3.5" />
        Eine Änderung am Zeitplan plant die nächste Prüfung neu.
      </p>
    </div>
  );
}

function SettingsSection({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Card className="[--card-spacing:--spacing(5)]">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icon className="size-4 text-primary" />
          {title}
        </CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="divide-y">{children}</CardContent>
    </Card>
  );
}

function ManagersSettings({ current }: { current: Settings }) {
  const { managers, settings, busy } = useUpdater();
  const enabled = current.enabledManagers;
  return (
    <>
      <ResourceError error={managers.error} retry={managers.refresh} />
      {managers.loading ? (
        <Skeleton className="h-48" />
      ) : (
        managers.managers.map((manager) => (
          <div
            key={manager.id}
            className="flex items-center justify-between gap-5 py-4 first:pt-0 last:pb-0"
          >
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <ManagerBadge manager={manager.id} />
                <span className="text-xs text-muted-foreground">
                  {managerVersionLabel(manager)}
                </span>
                {!manager.available && (
                  <Badge variant="outline">Nicht verfügbar</Badge>
                )}
              </div>
              <p
                id={`manager-${manager.id}-hint`}
                className="mt-2 text-xs text-muted-foreground"
              >
                {!manager.available
                  ? "Installiere diesen Paketmanager, um ihn zu aktivieren."
                  : manager.requiresAdmin
                    ? "Aktualisierungen benötigen Administratorrechte."
                    : "Verfügbare Updates in die Prüfung einbeziehen."}
              </p>
            </div>
            <Switch
              aria-label={`${manager.name} aktivieren`}
              aria-describedby={`manager-${manager.id}-hint`}
              checked={enabled.includes(manager.id)}
              disabled={!manager.available || busy || settings.saving}
              onCheckedChange={(checked) =>
                void settings.patch({
                  enabledManagers: checked
                    ? [...enabled, manager.id]
                    : enabled.filter((id) => id !== manager.id),
                })
              }
            />
          </div>
        ))
      )}
    </>
  );
}

function WingetModule() {
  const { busy, managers } = useUpdater();
  const [installing, setInstalling] = useState(false);
  const install = async () => {
    setInstalling(true);
    try {
      await installWingetModule();
      toast.success("WinGet-PowerShell-Modul installiert", {
        description: "Die nächste Prüfung verwendet die genauere Erkennung.",
      });
      void managers.refresh();
    } catch (error) {
      toast.error("Modul konnte nicht installiert werden", {
        description: toAppError(error).message,
      });
    } finally {
      setInstalling(false);
    }
  };
  return (
    <div className="pt-5">
      <p className="mb-4 text-xs leading-relaxed text-muted-foreground">
        Das Modul Microsoft.WinGet.Client ermöglicht eine genauere Erkennung
        ohne abgeschnittene IDs. Es wird für dein Benutzerkonto installiert.
        Dies startet keine Paket-Updates.
      </p>
      <Button
        variant="outline"
        className="max-w-full whitespace-normal"
        disabled={installing || busy}
        onClick={() => void install()}
      >
        {installing ? <LoaderCircle className="animate-spin" /> : <Download />}
        {installing
          ? "Modul wird installiert …"
          : "WinGet-PowerShell-Modul installieren"}
      </Button>
    </div>
  );
}

function IgnoredPackages({ current, disabled }: { current: Settings; disabled: boolean }) {
  const { settings, packages } = useUpdater();
  const ignored = current.ignoredPackages;
  if (!ignored.length)
    return (
      <EmptyState
        icon={EyeOff}
        title="Keine Ausnahmen"
        description="Über den Schalter „Ignorieren“ auf der Updates-Seite kannst du einzelne Pakete von automatischen Updates ausnehmen."
      />
    );
  return (
    <div className="divide-y">
      {ignored.map((key) => {
        const reference = parsePackageKey(key);
        const entry = packages.find(
          (candidate) => packageKey(candidate) === key,
        );
        return (
          <div
            key={key}
            className="flex items-center justify-between gap-3 py-3 first:pt-0 last:pb-0"
          >
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">
                {entry?.name ?? reference?.id ?? key}
              </p>
              <p className="mt-1 truncate text-xs text-muted-foreground">
                {key}
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              aria-label={`${entry?.name ?? key} aus Ausnahmen entfernen`}
              disabled={disabled}
              onClick={() =>
                reference
                  ? void settings.ignore(reference, false)
                  : void settings.patch({
                      ignoredPackages: ignored.filter((entry) => entry !== key),
                    })
              }
            >
              <Trash2 />
              <span className="hidden sm:inline">Entfernen</span>
            </Button>
          </div>
        );
      })}
    </div>
  );
}
