const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$/;
const JSON_VERSION = /^(  "version": )"([^"]*)"/m;
const CARGO_VERSION = /^(version\s*=\s*)"([^"]*)"/;
const SECTION_HEADER = /^\[[^\]]+\]/;

export function assertSemver(version) {
  if (!SEMVER.test(version)) {
    throw new Error(`"${version}" is not a valid version; expected x.y.z or x.y.z-prerelease`);
  }
}

export function readJsonVersion(text) {
  const match = JSON_VERSION.exec(text);
  if (!match) throw new Error('No top-level "version" field found');
  return match[2];
}

export function setJsonVersion(text, version) {
  readJsonVersion(text);
  return text.replace(JSON_VERSION, `$1"${version}"`);
}

function packageVersionLine(lines) {
  let inPackage = false;
  for (const [index, line] of lines.entries()) {
    if (SECTION_HEADER.test(line)) inPackage = line.startsWith("[package]");
    if (inPackage && CARGO_VERSION.test(line)) return index;
  }
  throw new Error("No version found in the [package] section of Cargo.toml");
}

export function readCargoPackageVersion(text) {
  const lines = text.split("\n");
  return CARGO_VERSION.exec(lines[packageVersionLine(lines)])[2];
}

export function setCargoPackageVersion(text, version) {
  const lines = text.split("\n");
  const index = packageVersionLine(lines);
  lines[index] = lines[index].replace(CARGO_VERSION, `$1"${version}"`);
  return lines.join("\n");
}

export function setCargoLockVersion(text, crateName, version) {
  const entry = new RegExp(`(name = "${crateName}"\r?\nversion = )"[^"]*"`);
  if (!entry.test(text)) throw new Error(`No ${crateName} entry found in Cargo.lock`);
  return text.replace(entry, `$1"${version}"`);
}

export const VERSION_FILES = [
  { path: "package.json", read: readJsonVersion, set: setJsonVersion },
  { path: "src-tauri/tauri.conf.json", read: readJsonVersion, set: setJsonVersion },
  { path: "src-tauri/Cargo.toml", read: readCargoPackageVersion, set: setCargoPackageVersion },
];
