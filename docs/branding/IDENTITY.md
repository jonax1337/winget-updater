# Winget Updater – visual identity

Final artwork based on the selected direction **03 · Monogram**, October 30, 2026.

![Winget Updater — Your software. Up to date.](readme-banner.svg)

## Mark and typography

The flat WU monogram ends in an upward arrow. Its continuous silhouette, open counters, and broad strokes keep it clear at small sizes. The app icon places the lime mark on a rounded graphite tile. Tray variants use the same geometry without the tile and with a tighter crop.

Geist is used for the interface and wordmark; Geist Mono is used for version numbers and technical details. The banner lettering is outlined, so the SVG requires no installed fonts and contains no embedded raster images.

All app-owned text is English, including accessible labels, dialogs, notifications, errors, and date/time formatting.

## Core colors

| Role | Color | Use |
| --- | --- | --- |
| Lime | `#C9F46A` | Monogram, main action, primary controls in dark mode |
| Graphite | `#181B19` | Icon tile, dark background, text on lime |
| Chalk | `#F3F5F0` | Light background, text on graphite |
| Moss | `#41651F` | Primary controls and small colored text in light mode |

Large surfaces use neutral shades with a slight green tint. Selected navigation and hover states use stepped green surfaces. Success, warning, error, and information retain distinct semantic colors; package managers retain separate chart colors.

## Design tokens

All UI colors are defined in `src/index.css`; components use the corresponding Tailwind utilities.

| Token | Purpose |
| --- | --- |
| `brand`, `brand-foreground` | Fixed lime and graphite pairing for the main action in both themes |
| `primary`, `primary-foreground` | Accessible UI colors: moss/chalk in light mode, lime/graphite in dark mode |
| `brand-hover`, `primary-hover` | Opaque hover colors with stable text contrast |
| `background`, `card`, `popover`, `sidebar` | Layered surfaces for both themes |
| `accent`, `secondary`, `muted` | Quiet interaction and background colors |
| `sidebar-primary`, `sidebar-primary-foreground` | Selected navigation item |
| `ring`, `sidebar-ring`, `input` | Visible focus and input boundaries |
| `success`, `warning`, `destructive`, `info` | Status colors independent of the brand accent |
| `chart-1` … `chart-5`, `manager-*` | Source colors coordinated with the palette |

The base radius is `0.5rem`. Typography, layout, and interaction patterns follow the existing app.

## Assets

| Source / export | Format / size |
| --- | --- |
| [App icon master](../../app-icon.svg) | Vector SVG with a 1024 × 1024 intrinsic size |
| [Title-bar logo and favicon](../../public/logo.svg) | Copy of the app icon master |
| [Standalone monogram](../../public/branding/monogram.svg) | SVG with a transparent background |
| [Tray for light taskbars](../../public/branding/tray-light.svg) | Dark mark, transparent background |
| [Tray for dark taskbars](../../public/branding/tray-dark.svg) | Light mark, transparent background |
| [README banner](readme-banner.svg) | SVG, 1600 × 480, vector paths only |
| [PNG banner](readme-banner.png) | 1600 × 480, rendered from the SVG |
| `src-tauri/icons/icon.png` | App icon, 1024 × 1024 |
| `src-tauri/icons/icon.ico` | Windows icon with multiple resolutions |
| `src-tauri/icons/icon.icns` | Desktop export from the Tauri icon generator |
| `src-tauri/icons/32x32.png` … `128x128@2x.png`, `Square*Logo.png`, `StoreLogo.png` | Tauri / Windows bundle exports |
| `src-tauri/icons/tray-{light,dark}-{16,20,24,32,48,64}.png` | Tray exports for different scaling factors |
| `src-tauri/icons/tray-{light,dark}.png` | Embedded 32-pixel tray icons |

The tray chooses its variant using the **Windows taskbar theme** (`SystemUsesLightTheme`), independently of the app theme. The variant is refreshed on Windows theme events and status updates. Pending updates add a lime dot with a graphite outline.

## Regenerating assets

`app-icon.svg` is the source of truth for the mark geometry. After editing it, run `pnpm icons` to regenerate the favicon, standalone mark, tray SVGs, and desktop icon exports. The generator uses the existing Tauri CLI and requires no additional npm dependencies.

Run `python scripts/generate-banner.py` to rebuild the outlined banner from the same mark and the project's installed Geist font. This authoring step requires `fonttools[woff]` in the local Python environment. The SVG itself can be used or edited in vector software without these tools. After changing it, render the PNG export again.
