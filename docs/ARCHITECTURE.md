# Winget Updater – Architektur & Vertrag

Verbindlich für alle Implementierungs-Agenten. Abweichungen nur mit Begründung im Rückbericht.

## Ziel

Windows-Tray-App (Tauri 2), die beim Login startet, periodisch alle verfügbaren Paketmanager
nach Updates fragt und diese automatisch installiert. UI zur Kontrolle, Ausnahmen, Verlauf.
UI-Sprache: **Deutsch**.

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
| `managers/mod.rs` | **Vertrag** – Trait `PackageManager` (`detect() -> AppResult<Option<String>>`, `list_upgrades() -> AppResult<Upgrades { packages, warnings }>`), `registry()` |
| `managers/{winget,scoop,chocolatey,npm,pip}.rs` | je ein Manager inkl. Parser + Unit-Tests |
| `process.rs` | Prozess-Runner |
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
| `update_packages` | `packages: PackageRef[]` | `UpdateSummary` |
| `update_all` | – | `UpdateSummary` |
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

## UI (Frontend, `src/`)

shadcn-Sidebar-Layout, Hell/Dunkel/System. Seiten:

1. **Übersicht** – Kennzahlen-Karten (verfügbare Updates, letzter Scan, nächster Scan, Auto-Update an/aus),
   Paketmanager-Status (verfügbar/Version/aktiv), große Aktionen „Jetzt prüfen“ / „Alle aktualisieren“,
   Live-Fortschritt.
2. **Updates** – Tabelle: App-Icon (Fallback: farbiges Initialen-Avatar), Name, Id, aktuell → neu,
   Manager-Badge, Status (Live aus `update-event`), Ignorieren-Toggle, Einzel-Update; Mehrfachauswahl +
   „Auswahl aktualisieren“; Suche + Manager-Filter.
3. **Verlauf** – Tabelle der `HistoryEntry`, Status-Badges, Leeren-Button.
4. **Einstellungen** – alle `Settings`-Felder, Manager an/aus (nicht verfügbare ausgegraut),
   ignorierte Pakete verwalten, WinGet-Modul installieren.

Frontend-Dateien: `src/lib/types.ts` (**Vertrag**), `src/lib/api.ts` (typisierte `invoke`/`listen`-Wrapper),
`src/hooks/*`, `src/pages/*`, `src/components/*` (eigene), `src/components/ui/*` (shadcn, nicht editieren
außer nötig), `src/App.tsx`, `src/main.tsx`.
