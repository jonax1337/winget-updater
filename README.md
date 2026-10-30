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

## Releasing

Installed copies update themselves: the app checks GitHub Releases once a day and offers a one-click update. Releases are built by the [release workflow](.github/workflows/release.yml) when a `v*` tag is pushed.

```sh
pnpm version:set 1.2.3
git commit -am "chore: release v1.2.3"
git tag v1.2.3
git push origin main v1.2.3
```

`pnpm version:set` updates `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. The workflow fails unless the tag matches the version in `package.json`, `Cargo.toml` and `tauri.conf.json`, then publishes the NSIS installer, its signature, and `latest.json` as a GitHub Release.

One-time setup in the repository settings (Secrets and variables, Actions):

- `TAURI_SIGNING_PRIVATE_KEY`: the content of the updater signing key file (`%USERPROFILE%\.tauri\winget-updater.key`). Its public half is `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: not needed while the key has no password. Leave it unset.

Keep the private key out of the repository. Losing it means installed copies can no longer verify updates, so back it up.

The installer is not Authenticode-signed, so Windows SmartScreen may warn when it is run for the first time.

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
