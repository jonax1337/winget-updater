![Winget Updater — Your software. Up to date.](docs/branding/readme-banner.svg)

Winget Updater keeps Windows software up to date across **winget, Scoop, Chocolatey, npm, and pip**. It runs in the system tray with scheduled checks, automatic or manual updates, an ignore list, and update history.

## Development

Requires Windows, Node.js with pnpm, and the Rust and Windows build tools for Tauri 2.

```sh
pnpm install
pnpm tauri dev
```

`pnpm dev` opens the browser preview. Package managers, the tray, and saved settings are available in the desktop app.

```sh
pnpm build
pnpm test
pnpm lint
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Branding

The WU monogram combines the app's initials with an upward arrow. Lime, graphite, and chalk define the palette; Geist and Geist Mono are the brand fonts.

| File | Use |
| --- | --- |
| [app-icon.svg](app-icon.svg) | Vector master for the app icon |
| [monogram.svg](public/branding/monogram.svg) | Standalone transparent mark |
| [tray-light.svg](public/branding/tray-light.svg) / [tray-dark.svg](public/branding/tray-dark.svg) | Tray marks for light / dark taskbars |
| [readme-banner.svg](docs/branding/readme-banner.svg) | README banner with lettering converted to vector paths |
| [readme-banner.png](docs/branding/readme-banner.png) | PNG banner, 1600 × 480 pixels |

The [brand guide](docs/branding/IDENTITY.md) covers colors, theme tokens, and exports. Regenerate the PNG, ICO, and ICNS icons from the SVG master:

```sh
pnpm icons
```

See [Architecture](docs/ARCHITECTURE.md) for technical details.
