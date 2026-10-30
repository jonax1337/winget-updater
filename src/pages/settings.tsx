import { Fragment, useState, type ReactNode } from "react";
import { Check, Download, EyeOff, Trash2 } from "lucide-react";
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
import { useUpdater } from "@/hooks/use-updater";
import { installWingetModule, toAppError } from "@/lib/api";
import { managerVersionLabel, packageKey, parsePackageKey } from "@/lib/packages";
import type { ManagerInfo, Settings, Theme } from "@/lib/types";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "Wie Windows" },
  { value: "light", label: "Hell" },
  { value: "dark", label: "Dunkel" },
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
          {settings.saving ? <Spinner /> : <Check className="size-4" aria-label="Gespeichert" />}
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
    <Section title="Automatisierung" description="Wann geprüft und aktualisiert wird.">
      <SwitchField
        label="Updates automatisch installieren"
        description="Nach jeder Prüfung alle nicht ignorierten Pakete aktualisieren."
        checked={current.autoUpdate}
        onChange={(autoUpdate) => void settings.patch({ autoUpdate })}
      />
      <FieldSeparator />
      <SettingField label="Prüfintervall" description="Abstand zwischen zwei automatischen Prüfungen.">
        {(ids) => (
          <NumberStepper
            id={ids.control}
            describedBy={ids.description}
            value={current.checkIntervalHours}
            min={1}
            max={168}
            unit="Std."
            onChange={(checkIntervalHours) => void settings.patch({ checkIntervalHours })}
          />
        )}
      </SettingField>
      <FieldSeparator />
      <SwitchField
        label="Nach dem App-Start prüfen"
        description="Nach der Startverzögerung eine erste Prüfung ausführen."
        checked={current.updateOnStartup}
        onChange={(updateOnStartup) => void settings.patch({ updateOnStartup })}
      />
      <FieldSeparator />
      <SettingField label="Startverzögerung" description="Windows Zeit zum Hochfahren lassen.">
        {(ids) => (
          <NumberStepper
            id={ids.control}
            describedBy={ids.description}
            value={current.startupDelayMinutes}
            min={0}
            max={60}
            unit="Min."
            onChange={(startupDelayMinutes) => void settings.patch({ startupDelayMinutes })}
          />
        )}
      </SettingField>
      <FieldSeparator />
      <SwitchField
        label="Mit Windows starten"
        description="Beim Anmelden minimiert im Infobereich starten."
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
    <Section title="Paketmanager" description="Nur installierte und aktivierte Quellen werden geprüft.">
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
  if (!manager.available) return "Installiere diesen Paketmanager, um ihn zu aktivieren.";
  if (manager.requiresAdmin) return "Aktualisierungen benötigen Administratorrechte.";
  return "Verfügbare Updates in die Prüfung einbeziehen.";
}

function Appearance({ current }: { current: Settings }) {
  const { settings } = useUpdater();
  return (
    <Section title="Benachrichtigungen & Darstellung" description="Informiert bleiben, ohne gestört zu werden.">
      <SwitchField
        label="Windows-Benachrichtigungen"
        description="Über gefundene Updates und abgeschlossene Aktualisierungen informieren."
        checked={current.notify}
        onChange={(notify) => void settings.patch({ notify })}
      />
      <FieldSeparator />
      <SettingField label="Erscheinungsbild" description="Hell, dunkel oder passend zu Windows.">
        {(ids) => (
          <Combobox
            label="Erscheinungsbild"
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
    <Section title="WinGet-Erkennung" description="Zuverlässige Paket-IDs, auch bei langen Namen.">
      <SwitchField
        label="Unbekannte Versionen einbeziehen"
        description="Auch Pakete prüfen, deren installierte Version WinGet nicht kennt."
        checked={current.includeUnknownVersions}
        onChange={(includeUnknownVersions) => void settings.patch({ includeUnknownVersions })}
      />
      <FieldSeparator />
      <SettingField
        label="PowerShell-Modul"
        description="Microsoft.WinGet.Client für genauere Erkennung ohne abgeschnittene IDs. Startet keine Updates."
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
    <Button
      id={id}
      variant="outline"
      aria-describedby={describedBy}
      disabled={installing || busy}
      onClick={() => void install()}
    >
      {installing ? <Spinner data-icon="inline-start" /> : <Download data-icon="inline-start" />}
      Installieren
    </Button>
  );
}

function IgnoredPackages({ current }: { current: Settings }) {
  const { settings, packages, busy } = useUpdater();
  const ignored = current.ignoredPackages;
  return (
    <Card>
      <CardHeader>
        <CardTitle>Ignorierte Pakete</CardTitle>
        <CardDescription>Werden nie automatisch aktualisiert.</CardDescription>
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
                      aria-label={`${name} wieder freigeben`}
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
                      Freigeben
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
              <EmptyTitle>Keine Ausnahmen</EmptyTitle>
              <EmptyDescription>
                Pakete lassen sich auf der Updates-Seite über das Aktionsmenü ignorieren.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </CardContent>
    </Card>
  );
}
