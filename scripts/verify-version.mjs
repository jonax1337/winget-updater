import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { VERSION_FILES, assertSemver } from "./version-edits.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));

async function findMismatches(expected) {
  const versions = await Promise.all(
    VERSION_FILES.map(async ({ path: file, read }) => ({
      file,
      actual: read(await readFile(path.join(root, file), "utf8")),
    })),
  );
  return versions.filter(({ actual }) => actual !== expected);
}

try {
  const expected = process.argv[2] ?? "";
  assertSemver(expected);
  const mismatches = await findMismatches(expected);
  if (mismatches.length) {
    const details = mismatches.map(({ file, actual }) => `${file} is ${actual}`).join("; ");
    throw new Error(`Expected version ${expected}, but ${details}`);
  }
  console.log(`All versions are ${expected}`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
