import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { VERSION_FILES, assertSemver, setCargoLockVersion } from "./version-edits.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const CRATE_NAME = "winget-updater";

const edits = [
  ...VERSION_FILES.map(({ path: file, set }) => ({ file, edit: set })),
  {
    file: "src-tauri/Cargo.lock",
    edit: (text, version) => setCargoLockVersion(text, CRATE_NAME, version),
  },
];

async function setVersion(version) {
  assertSemver(version);
  const updated = await Promise.all(
    edits.map(async ({ file, edit }) => {
      const filePath = path.join(root, file);
      return [filePath, edit(await readFile(filePath, "utf8"), version)];
    }),
  );
  await Promise.all(updated.map(([filePath, text]) => writeFile(filePath, text)));
  console.log(`Version set to ${version} in ${edits.map(({ file }) => file).join(", ")}`);
}

try {
  await setVersion(process.argv[2] ?? "");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
