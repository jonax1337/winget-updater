# Winget Updater – Architektur & Vertrag

Verbindlich für alle Implementierungs-Agenten. Abweichungen nur mit Begründung im Rückbericht.

## Ziel

Windows-Tray-App (Tauri 2), die beim Login startet, periodisch alle verfügbaren Paketmanager
nach Updates fragt und diese automatisch installiert. UI zur Kontrolle, Ausnahmen, Verlauf.
UI language: **English**. App-owned UI text, accessibility labels, tray menus, notifications,
and errors must be English. Package-manager parsers still accept localized output.

## Stack (entschieden)

| Bereich   | Wahl |
|-----------|------|
| Shell     | Tauri 2, Rust edition 2024 |
| Frontend  | React 19 + TypeScript + Vite, Tailwind v4, **shadcn/ui (radix-nova)**, lucide-react, sonner |
| Plugins   | autostart, notification, store, single-instance |
| Async     | tokio; Paketmanager-Aufrufe sind blockierend und laufen in `spawn_blocking` |

## Paketmanager

| Id          | Erkennung              | Updates listen | Update ausführen | Default |
|-------------|------------------------|----------------|------------------|---------|
| winget      | `winget --version`     | PowerShell-Modul `Microsoft.WinGet.Client` (`Get-WinGetPackage \| ? IsUpdateAvailable \| ConvertTo-Json`) wenn vorhanden, sonst `winget upgrade` Tabellen-Parser | `winget upgrade --id <id> --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity` | an |
| scoop       | `scoop --version`      | `scoop update` (Buckets) dann `scoop status` | `scoop update <app>` | an |
| chocolatey  | `choco --version`      | `choco outdated -r` (`name\|current\|available\|pinned`) | `choco upgrade <id> -y --no-progress` (braucht Admin) | an |
| npm         | `npm --version`        | `npm outdated -g --json` | `npm install -g <name>@latest` | an |
| pip         | `python -m pip --version` | `python -m pip list --outdated --format=json --user` | `python -m pip install -U --user <name>` | **aus** |

### winget-Tabellen-Parser (wichtig)

- Ausgabe ist **lokalisiert** (siehe `src-tauri/tests/fixtures/winget_upgrade.txt`, deutsche Köpfe).
  Niemals an Header-*Namen* parsen. Spaltengrenzen aus den Startpositionen der Header-Wörter in der
  Zeile direkt über der `-----`-Trennzeile ableiten; Spaltenreihenfolge ist fest:
  Name, Id, Version, Available, Source.
- Positionen in **Zeichen** (chars), nicht Bytes (Umlaute!).
- Fortschritts-/Spinnerzeilen (`\r`, `-\|/`, Balken `█▒`) vor dem Header verwerfen.
- Zeilen nach der Tabelle (z. B. „1 Aktualisierungen verfügbar.“, Pins-Hinweise) beenden die Tabelle:
  eine Datenzeile muss mindestens bis in die Source-Spalte reichen.
- IDs mit `…` sind abgeschnitten → Paket nicht übernehmen, stattdessen eine `ManagerError`-Warnung
  „N Pakete mit abgeschnittener ID – WinGet-PowerShell-Modul installieren“.

## Prozess-Ausführung (`process.rs`)

Ein einziger Runner für alle Manager: `CREATE_NO_WINDOW` (0x08000000), stdout/stderr lossy UTF-8,
Timeout (Liste 120 s, Update 30 min), Rückgabe `CommandOutput { status, stdout, stderr }`.
Kein Manager ruft `std::process::Command` direkt auf.

## Modul-Zuständigkeiten (Rust, `src-tauri/src/`)

| Datei | Inhalt |
|-------|--------|
| `models.rs` | **Vertrag** – alle serialisierbaren Typen (nicht ohne Not ändern) |
| `error.rs` | **Vertrag** – `AppError` |
| `managers/mod.rs` | **Vertrag** – Trait `PackageManager` (`detect() -> AppResult<Option<String>>`, `list_upgrades() -> AppResult<Upgrades { packages, warnings }>`, `list_installed() -> AppResult<Installed { packages, warnings }>`, `uninstall(id)`), `registry()` |
| `managers/{winget,scoop,chocolatey,npm,pip}.rs` | je ein Manager inkl. Parser + Unit-Tests |
| `process.rs` | Prozess-Runner |
| `installed.rs` | Registry-Index installierter Apps (`Uninstall`-Keys, Name-Matching, Scoop-Pfade) – gemeinsam genutzt von `icons.rs` und `running.rs` |
| `running.rs` | Laufende-App-Gate: Installationsverzeichnisse eines Pakets + Prozessliste (Toolhelp32) → laufende EXEs; Vollbild/Präsentation via `SHQueryUserNotificationState` |
| `icons.rs` | App-Icons aus Registry (`Uninstall`-Keys HKLM/HKCU/WOW6432, `DisplayIcon`) → PNG 32×32 → Data-URL, Disk-Cache im App-Cache-Dir |
| `settings.rs` | Laden/Speichern `Settings` via tauri-plugin-store (`settings.json`), Defaults |
| `history.rs` | Verlauf via store (`history.json`), max. 500 Einträge, neueste zuerst |
| `engine.rs` | Zentraler Zustand (`AppState`): Scan, Update-Queue (sequenziell, ein Paket gleichzeitig), Events, Benachrichtigungen |
| `scheduler.rs` | Startverzögerung + Intervall-Loop: Scan, bei `auto_update` danach Update aller nicht ignorierten Pakete |
| `tray.rs` | Tray-Icon + Menü: Öffnen, Jetzt prüfen, Alle aktualisieren, —, Beenden; Tooltip „Winget Updater – N Updates“ |
| `commands.rs` | Alle `#[tauri::command]` (dünn, delegieren an engine/settings/history/icons) |
| `lib.rs` | Verdrahtung: Plugins, State, Tray, Scheduler, Fenster-Close → verstecken, `--minimized` beim Autostart |

## Tauri-Commands (Namen exakt so)

| Command | Args | Rückgabe |
|---------|------|----------|
| `get_managers` | – | `ManagerInfo[]` |
| `scan_updates` | – | `ScanResult` |
| `get_last_scan` | – | `ScanResult \| null` |
| `list_installed` | – | `InstalledResult` (alle installierten Programme der aktiven Paketmanager, bei Bedarf geladen) |
| `uninstall_package` | `package: PackageRef` | – (nur über den Manager des Pakets; entfernt es aus dem letzten Scan) |
| `update_packages` | `packages: PackageRef[], skipRunning: boolean` | `UpdateSummary` |
| `update_all` | `skipRunning: boolean` | `UpdateSummary` |
| `check_running` | `packages: PackageRef[]` | `RunningApp[]` (nur Pakete mit laufenden Prozessen) |
| `get_run_state` | – | `RunState` |
| `get_settings` | – | `Settings` |
| `save_settings` | `settings: Settings` | `Settings` (wendet Autostart an, plant Scheduler neu) |
| `set_package_ignored` | `package: PackageRef, ignored: boolean` | `Settings` |
| `get_history` | `limit: number` | `HistoryEntry[]` |
| `clear_history` | – | – |
| `get_package_icon` | `package: PackageRef, name: string` | `string \| null` (Data-URL) |
| `install_winget_module` | – | – |

Alle Commands: `Result<T, AppError>`; `AppError` serialisiert als `{ kind, message }`.

## Events (Backend → Frontend)

| Event | Payload |
|-------|---------|
| `run-state` | `RunState` |
| `scan-finished` | `ScanResult` |
| `update-event` | `UpdateEvent` |
| `update-finished` | `UpdateSummary` |

## Verhalten

- Autostart (Default an) startet mit `--minimized` → nur Tray, Fenster verborgen.
- Fenster schließen = verstecken; Beenden nur über Tray.
- Single-Instance: zweiter Start fokussiert das vorhandene Fenster.
- Scheduler: nach `startup_delay_minutes` erster Lauf (wenn `update_on_startup`), danach alle
  `check_interval_hours`. Settings-Änderung startet den Loop neu.
- Während ein Scan/Update läuft, werden weitere Starts abgewiesen (`AppError::Busy`).
- Benachrichtigung (wenn `notify`): nach Scan mit Treffern ohne Auto-Update, nach jedem Update-Lauf
  (Anzahl erfolgreich/fehlgeschlagen).
- Pakete, die zwischen Planung und Ausführung eines automatischen Laufs ignoriert oder deaktiviert werden, erhalten den Status `skipped`.
- Manager-Warnungen (z. B. abgeschnittene winget-IDs) landen als `ManagerError` im `ScanResult`.
- Ignorierte Pakete: Schlüssel `"<manager>:<id>"` in `Settings.ignored_packages`; werden gelistet
  (`ignored: true`), aber nie automatisch aktualisiert.

## Laufende-Apps-Gate

- **Erkennung** (`running.rs`): Installationsverzeichnisse je Paket – winget/chocolatey über den Registry-Index
  (`InstallLocation`, sonst Ordner der `DisplayIcon`-EXE), scoop `~/scoop/apps/<id>`, chocolatey zusätzlich
  `%ChocolateyInstall%\lib\<id>`; npm/pip werden nie geprüft. Zu allgemeine Verzeichnisse (Laufwerkswurzel,
  `Program Files*`, `%WINDIR%`, Benutzerprofil, `AppData\Local|Roaming`, `AppData\Local\Programs`,
  `ProgramData`) werden verworfen. Ein Prozess „läuft“, wenn sein Image-Pfad (case-insensitiv) unter einem
  dieser Verzeichnisse liegt; der eigene Prozess zählt nie. Ergebnis: EXE-Dateinamen, dedupliziert, sortiert.
- **`skipRunning = true`**: Paket mit laufenden Prozessen wird nicht installiert, sondern `skipped` mit Meldung
  „Läuft gerade (Discord.exe) – wird beim nächsten Durchlauf erneut versucht“; zählt in `UpdateSummary.skipped`;
  kein Verlaufseintrag. Die Prüfung erfolgt unmittelbar vor jedem einzelnen Paket.
- Scheduler und Tray-„Alle aktualisieren“ nutzen `skipRunning = settings.skip_running_apps`.
  Das Frontend ruft vor manuellen Updates `check_running` auf und fragt bei Treffern nach
  („Trotzdem aktualisieren“ → `false`, „Laufende überspringen“ → `true`, „Abbrechen“).
- **Vollbild-Gate**: Ist `defer_while_busy` an und meldet `SHQueryUserNotificationState` `QUNS_BUSY`,
  `QUNS_RUNNING_D3D_FULL_SCREEN` oder `QUNS_PRESENTATION_MODE`, führt der *Scheduler* nach dem Scan keine
  Updates aus und versucht es nach `min(15 min, Intervall)` erneut (`next_scan` entsprechend). Manuelle Aktionen
  sind nicht betroffen.
- Vertragstypen (`models.rs` ↔ `types.ts`): `RunningApp { package: PackageRef, processes: string[] }`;
  `UpdateSummary.skipped: u32`; `Settings.skip_running_apps` / `skipRunningApps` und
  `Settings.defer_while_busy` / `deferWhileBusy` (beide `bool`, Default `true`).
- Benachrichtigung nach Lauf: „X erfolgreich, Y fehlgeschlagen“ + bei `skipped > 0` „, Z zurückgestellt“.

## UI (Frontend, `src/`)

Layout aus dem shadcn-Block `dashboard-01` (Inset-Sidebar, einklappbar auf Icons, Site-Header mit
Breadcrumb) unter einer eigenen Titlebar (`decorations: false`, `data-tauri-drag-region`,
Fenster-Buttons über die Window-API). Hell/Dunkel/System.

UI-Regeln:
- Nur shadcn-Komponenten/-Blocks; keine eigenen CSS-Klassen, keine Inline-Styles.
- Design-System = Tokens in `src/index.css` (Farben inkl. `success`/`warning`/`info`, Manager-Farben
  `manager-*`, Fonts, Radien, `titlebar`/`header`-Höhen); Komponenten nutzen sie nur über Tailwind-Utilities.
- CI: WU-Monogramm, Limette/Graphit/Kreide; feste `brand`-Tokens und semantische Hell-/Dunkel-Tokens.
  SVG-Master `app-icon.svg`, reproduzierbare Desktop-Exporte über `pnpm icons`, Details in
  `docs/branding/IDENTITY.md`. Das Tray nutzt eigene transparente Hell-/Dunkel-Icons passend zur
  Windows-Taskleiste und einen limettenfarbenen Update-Punkt mit Graphitkontur.
- Auswahllisten immer als `Combobox` (Popover + Command), keine nativen Controls (`select`, `type=number`).

Seiten:

1. **Übersicht** – Section-Cards (verfügbare Updates, letzter Scan, nächster Scan, Auto-Update),
   Donut-Chart „Updates nach Quelle“, Paketmanager-Status. Aktionen „Alle aktualisieren“ / „Jetzt prüfen“
   stehen in der Sidebar; Live-Fortschritt über allen Seiten.
2. **Updates** – Data-Table (TanStack): App-Icon (Fallback: Initialen), Name, Id, aktuell → neu,
   Manager-Badge, Status (Live aus `update-event`), Aktionsmenü (Einzel-Update, Ignorieren);
   Mehrfachauswahl + „Auswahl aktualisieren“; Suche + Quellen-Combobox; Paginierung.
3. **Verlauf** – Data-Table der `HistoryEntry`, Ergebnis-Filter (Toggle-Group), Leeren mit Bestätigung.
4. **Einstellungen** – alle `Settings`-Felder als Field-Gruppen, Manager an/aus (nicht verfügbare
   ausgegraut), ignorierte Pakete verwalten, WinGet-Modul installieren.

Frontend-Dateien: `src/lib/types.ts` (**Vertrag**), `src/lib/api.ts` (typisierte `invoke`/`listen`-Wrapper),
`src/hooks/*`, `src/pages/*`, `src/components/*` (eigene), `src/components/ui/*` (shadcn, nicht editieren
außer nötig), `src/App.tsx`, `src/main.tsx`.
