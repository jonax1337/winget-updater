import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const require = createRequire(import.meta.url);
const cli = path.join(path.dirname(require.resolve("@tauri-apps/cli/package.json")), "tauri.js");
const output = await mkdtemp(path.join(tmpdir(), "winget-branding-"));
const brandDirectory = path.join(root, "public/branding");
const iconDirectory = path.join(root, "src-tauri/icons");

function generate(source, destination, extra = []) {
  const result = spawnSync(process.execPath, [cli, "icon", source, "--output", destination, ...extra], {
    cwd: root,
    stdio: "inherit",
  });
  if (result.status !== 0) throw new Error("Icon export failed");
}

try {
  await mkdir(brandDirectory, { recursive: true });
  const icon = await readFile(path.join(root, "app-icon.svg"), "utf8");
  const mark = icon.match(/<path id="monogram" d="([^"]+)"/)?.[1];
  if (!mark) throw new Error("app-icon.svg must contain the canonical monogram path");
  await copyFile(path.join(root, "app-icon.svg"), path.join(root, "public/logo.svg"));
  const svg = (color, viewBox = "0 0 256 256", size = 256) =>
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="${viewBox}" fill="none">\n  <title>Winget Updater</title>\n  <path d="${mark}" fill="${color}"/>\n</svg>\n`;
  await writeFile(path.join(brandDirectory, "monogram.svg"), svg("#C9F46A"));
  await writeFile(path.join(brandDirectory, "tray-light.svg"), svg("#181B19", "16 16 224 224", 32));
  await writeFile(path.join(brandDirectory, "tray-dark.svg"), svg("#F3F5F0", "16 16 224 224", 32));

  const application = path.join(output, "app");
  generate("app-icon.svg", application);
  // Keep the desktop exports; this Windows app does not ship mobile assets.
  for (const file of await readdir(application, { withFileTypes: true })) {
    if (file.isFile() && /\.(png|ico|icns)$/.test(file.name)) {
      await copyFile(path.join(application, file.name), path.join(iconDirectory, file.name));
    }
  }
  generate("app-icon.svg", path.join(output, "master"), ["--png", "1024"]);
  await copyFile(path.join(output, "master/1024x1024.png"), path.join(iconDirectory, "icon.png"));

  for (const theme of ["light", "dark"]) {
    const trayOutput = path.join(output, theme);
    generate(`public/branding/tray-${theme}.svg`, trayOutput,
      [16, 20, 24, 32, 48, 64].flatMap((size) => ["--png", String(size)]));
    for (const size of [16, 20, 24, 32, 48, 64]) {
      await copyFile(path.join(trayOutput, `${size}x${size}.png`),
        path.join(iconDirectory, `tray-${theme}-${size}.png`));
    }
    await copyFile(path.join(trayOutput, "32x32.png"), path.join(iconDirectory, `tray-${theme}.png`));
  }
  await copyFile(path.join(iconDirectory, "tray-dark.png"), path.join(iconDirectory, "tray.png"));
} finally {
  // Delete only the unique temporary directory created by this invocation.
  const resolved = path.resolve(output);
  if (path.dirname(resolved) !== path.resolve(tmpdir()) ||
      !path.basename(resolved).startsWith("winget-branding-")) {
    throw new Error("Unexpected temporary export directory");
  }
  await rm(resolved, { recursive: true });
}
