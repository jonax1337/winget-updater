import type { ManagerError, PackageRef } from "./types";

export type CatalogCategory =
  | "browsers"
  | "development"
  | "communication"
  | "media"
  | "productivity"
  | "utilities"
  | "gaming"
  | "security";

export type CatalogFilter = CatalogCategory | "all";

export interface CatalogEntry {
  id: string;
  name: string;
  description: string;
}

export interface CatalogApp extends CatalogEntry {
  category: CatalogCategory;
}

// Mirror of src-tauri/src/catalog.rs – keep in sync.
export interface CatalogPackage {
  id: string;
  name: string;
  version: string;
  source: string;
}

export interface CatalogSearchResult {
  packages: CatalogPackage[];
  errors: ManagerError[];
}

export type CatalogStatus = "installed" | "update-available" | "not-installed";

/** The backend rejects longer queries. */
export const SEARCH_MAX_LENGTH = 100;

export const CATEGORY_FILTERS: { value: CatalogFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "browsers", label: "Browsers" },
  { value: "development", label: "Development" },
  { value: "communication", label: "Communication" },
  { value: "media", label: "Media & Creative" },
  { value: "productivity", label: "Productivity" },
  { value: "utilities", label: "Utilities" },
  { value: "gaming", label: "Gaming" },
  { value: "security", label: "Security & Privacy" },
];

export const CATALOG_APPS: readonly CatalogApp[] = [
  { category: "browsers", id: "Google.Chrome", name: "Google Chrome", description: "Fast, widely supported browser with Google account sync." },
  { category: "browsers", id: "Mozilla.Firefox", name: "Mozilla Firefox", description: "Open-source browser focused on privacy and customization." },
  { category: "browsers", id: "Brave.Brave", name: "Brave", description: "Chromium-based browser that blocks ads and trackers by default." },
  { category: "browsers", id: "Vivaldi.Vivaldi", name: "Vivaldi", description: "Highly customizable Chromium-based browser with built-in tools." },
  { category: "browsers", id: "Opera.Opera", name: "Opera", description: "Browser with a built-in VPN, sidebar apps and workspaces." },
  { category: "browsers", id: "TorProject.TorBrowser", name: "Tor Browser", description: "Browse anonymously through the Tor network." },
  { category: "browsers", id: "Zen-Team.Zen-Browser", name: "Zen Browser", description: "Calm, Firefox-based browser with vertical tabs and workspaces." },

  { category: "development", id: "Git.Git", name: "Git", description: "Distributed version control system." },
  { category: "development", id: "Microsoft.VisualStudioCode", name: "Visual Studio Code", description: "Lightweight, extensible code editor from Microsoft." },
  { category: "development", id: "Microsoft.PowerShell", name: "PowerShell", description: "Cross-platform shell and scripting language." },
  { category: "development", id: "Microsoft.WindowsTerminal", name: "Windows Terminal", description: "Modern tabbed terminal for Command Prompt, PowerShell and WSL." },
  { category: "development", id: "Docker.DockerDesktop", name: "Docker Desktop", description: "Build, run and share containerized applications." },
  { category: "development", id: "OpenJS.NodeJS.LTS", name: "Node.js (LTS)", description: "JavaScript runtime, long-term support release." },
  { category: "development", id: "Python.Python.3.13", name: "Python 3.13", description: "Python interpreter and standard library." },
  { category: "development", id: "GitHub.cli", name: "GitHub CLI", description: "Use pull requests, issues and releases from the command line." },
  { category: "development", id: "GitHub.GitHubDesktop", name: "GitHub Desktop", description: "Graphical client for Git and GitHub." },
  { category: "development", id: "JetBrains.Toolbox", name: "JetBrains Toolbox", description: "Install and update JetBrains IDEs in one place." },
  { category: "development", id: "Postman.Postman", name: "Postman", description: "Design, test and document APIs." },
  { category: "development", id: "Rustlang.Rustup", name: "Rustup", description: "Installer and version manager for the Rust toolchain." },
  { category: "development", id: "Notepad++.Notepad++", name: "Notepad++", description: "Fast, free source code and text editor." },

  { category: "communication", id: "Discord.Discord", name: "Discord", description: "Voice, video and text chat for communities and friends." },
  { category: "communication", id: "Microsoft.Teams", name: "Microsoft Teams", description: "Chat, meetings and collaboration for work and school." },
  { category: "communication", id: "SlackTechnologies.Slack", name: "Slack", description: "Team messaging organized into channels." },
  { category: "communication", id: "Zoom.Zoom", name: "Zoom Workplace", description: "Video meetings, webinars and team chat." },
  { category: "communication", id: "Telegram.TelegramDesktop", name: "Telegram", description: "Fast cloud-based messenger with large groups and channels." },
  { category: "communication", id: "OpenWhisperSystems.Signal", name: "Signal", description: "Private messenger with end-to-end encryption." },
  { category: "communication", id: "Mozilla.Thunderbird", name: "Mozilla Thunderbird", description: "Free email, calendar and contacts client." },

  { category: "media", id: "VideoLAN.VLC", name: "VLC media player", description: "Plays nearly any audio or video format." },
  { category: "media", id: "Spotify.Spotify", name: "Spotify", description: "Music and podcast streaming." },
  { category: "media", id: "OBSProject.OBSStudio", name: "OBS Studio", description: "Record and live-stream video with scenes and sources." },
  { category: "media", id: "Audacity.Audacity", name: "Audacity", description: "Multi-track audio recorder and editor." },
  { category: "media", id: "HandBrake.HandBrake", name: "HandBrake", description: "Convert video to modern formats." },
  { category: "media", id: "Gyan.FFmpeg", name: "FFmpeg", description: "Command-line toolkit for converting audio and video." },
  { category: "media", id: "GIMP.GIMP", name: "GIMP", description: "Free image editor for photo retouching and composition." },
  { category: "media", id: "Inkscape.Inkscape", name: "Inkscape", description: "Vector graphics editor for illustrations and logos." },
  { category: "media", id: "KDE.Krita", name: "Krita", description: "Digital painting and illustration studio." },
  { category: "media", id: "BlenderFoundation.Blender", name: "Blender", description: "3D modeling, animation and rendering suite." },
  { category: "media", id: "clsid2.mpc-hc", name: "MPC-HC", description: "Lightweight classic media player." },

  { category: "productivity", id: "Notion.Notion", name: "Notion", description: "Notes, docs and databases in one workspace." },
  { category: "productivity", id: "Obsidian.Obsidian", name: "Obsidian", description: "Linked notes stored as local Markdown files." },
  { category: "productivity", id: "TheDocumentFoundation.LibreOffice", name: "LibreOffice", description: "Free office suite with word processor, spreadsheets and slides." },
  { category: "productivity", id: "Adobe.Acrobat.Reader.64-bit", name: "Adobe Acrobat Reader", description: "View, sign and annotate PDF documents." },
  { category: "productivity", id: "SumatraPDF.SumatraPDF", name: "SumatraPDF", description: "Minimal, fast reader for PDF, ePub and comic books." },
  { category: "productivity", id: "Joplin.Joplin", name: "Joplin", description: "Open-source note taking with sync and end-to-end encryption." },
  { category: "productivity", id: "Doist.Todoist", name: "Todoist", description: "Task manager and to-do list." },
  { category: "productivity", id: "Dropbox.Dropbox", name: "Dropbox", description: "Cloud storage with file sync and sharing." },
  { category: "productivity", id: "Google.GoogleDrive", name: "Google Drive", description: "Sync and back up files with Google Drive." },

  { category: "utilities", id: "7zip.7zip", name: "7-Zip", description: "File archiver with a high compression ratio." },
  { category: "utilities", id: "Microsoft.PowerToys", name: "PowerToys", description: "Microsoft's power-user tools for window management, search and more." },
  { category: "utilities", id: "voidtools.Everything", name: "Everything", description: "Instant file name search for Windows." },
  { category: "utilities", id: "WinDirStat.WinDirStat", name: "WinDirStat", description: "See which files and folders use your disk space." },
  { category: "utilities", id: "Rufus.Rufus", name: "Rufus", description: "Create bootable USB drives from ISO images." },
  { category: "utilities", id: "BleachBit.BleachBit", name: "BleachBit", description: "Free disk space and clean up temporary files." },
  { category: "utilities", id: "CPUID.CPU-Z", name: "CPU-Z", description: "Detailed information about CPU, memory and mainboard." },
  { category: "utilities", id: "TeamViewer.TeamViewer", name: "TeamViewer", description: "Remote access and remote support." },
  { category: "utilities", id: "AnyDesk.AnyDesk", name: "AnyDesk", description: "Lightweight remote desktop software." },
  { category: "utilities", id: "Balena.Etcher", name: "balenaEtcher", description: "Flash OS images to USB drives and SD cards." },
  { category: "utilities", id: "WinSCP.WinSCP", name: "WinSCP", description: "SFTP, FTP and SCP file transfer client." },
  { category: "utilities", id: "PuTTY.PuTTY", name: "PuTTY", description: "SSH and Telnet client." },
  { category: "utilities", id: "ShareX.ShareX", name: "ShareX", description: "Screenshots, screen recording and quick sharing." },
  { category: "utilities", id: "qBittorrent.qBittorrent", name: "qBittorrent", description: "Open-source BitTorrent client without ads." },
  { category: "utilities", id: "Microsoft.Sysinternals.Suite", name: "Sysinternals Suite", description: "Advanced system utilities from Microsoft." },

  { category: "gaming", id: "Valve.Steam", name: "Steam", description: "Game store and library from Valve." },
  { category: "gaming", id: "EpicGames.EpicGamesLauncher", name: "Epic Games Launcher", description: "Epic Games Store with free weekly games." },
  { category: "gaming", id: "GOG.Galaxy", name: "GOG Galaxy", description: "DRM-free game store that unifies your game libraries." },
  { category: "gaming", id: "Ubisoft.Connect", name: "Ubisoft Connect", description: "Launcher for Ubisoft games." },
  { category: "gaming", id: "ElectronicArts.EADesktop", name: "EA app", description: "Launcher for Electronic Arts games." },
  { category: "gaming", id: "PrismLauncher.PrismLauncher", name: "Prism Launcher", description: "Manage multiple Minecraft instances and mods." },

  { category: "security", id: "Bitwarden.Bitwarden", name: "Bitwarden", description: "Open-source password manager." },
  { category: "security", id: "KeePassXCTeam.KeePassXC", name: "KeePassXC", description: "Offline password manager with a local encrypted database." },
  { category: "security", id: "Malwarebytes.Malwarebytes", name: "Malwarebytes", description: "Detects and removes malware." },
  { category: "security", id: "Proton.ProtonVPN", name: "Proton VPN", description: "VPN with a strict no-logs policy and a free tier." },
  { category: "security", id: "WireGuard.WireGuard", name: "WireGuard", description: "Fast, modern VPN protocol and client." },
  { category: "security", id: "MullvadVPN.MullvadVPN", name: "Mullvad VPN", description: "Privacy-focused VPN that needs no email address." },
  { category: "security", id: "AgileBits.1Password", name: "1Password", description: "Password manager for individuals and teams." },
  { category: "security", id: "IDRIX.VeraCrypt", name: "VeraCrypt", description: "Disk and container encryption." },
  { category: "security", id: "WiresharkFoundation.Wireshark", name: "Wireshark", description: "Network protocol analyzer." },
  { category: "security", id: "Tailscale.Tailscale", name: "Tailscale", description: "Zero-config mesh VPN between your devices." },
];

export function appsInCategory(filter: CatalogFilter): readonly CatalogApp[] {
  return filter === "all" ? CATALOG_APPS : CATALOG_APPS.filter((app) => app.category === filter);
}

export function searchResultEntry(result: CatalogPackage): CatalogEntry {
  return { id: result.id, name: result.name, description: `Version ${result.version}` };
}

/** IDs are compared case-insensitively, so they are stored lower-cased. */
export function wingetIds(packages: readonly PackageRef[]): Set<string> {
  return new Set(
    packages
      .filter((entry) => entry.manager === "winget")
      .map((entry) => entry.id.toLowerCase()),
  );
}

export function catalogStatus(
  id: string,
  installedIds: ReadonlySet<string>,
  updatableIds: ReadonlySet<string>,
): CatalogStatus {
  const key = id.toLowerCase();
  if (updatableIds.has(key)) return "update-available";
  return installedIds.has(key) ? "installed" : "not-installed";
}
