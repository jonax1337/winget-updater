import { describe, expect, it } from "vitest";
import {
  VERSION_FILES,
  assertSemver,
  readCargoPackageVersion,
  readJsonVersion,
  setCargoLockVersion,
  setCargoPackageVersion,
  setJsonVersion,
} from "./version-edits.mjs";

describe("assertSemver", () => {
  it.each(["0.1.0", "1.20.3", "2.0.0-beta.1", "1.0.0-rc1"])("accepts %s", (version) => {
    expect(() => assertSemver(version)).not.toThrow();
  });

  it.each(["", "1", "1.2", "v1.2.3", "1.2.3.4", "1.2.3-", "1.2.3+build", "01.2.3"])(
    "rejects %j",
    (version) => {
      expect(() => assertSemver(version)).toThrow("not a valid version");
    },
  );
});

describe("readJsonVersion", () => {
  it("reads the top-level version", () => {
    const manifest = '{\n  "version": "1.4.0",\n  "nested": {\n    "version": "9"\n  }\n}';
    expect(readJsonVersion(manifest)).toBe("1.4.0");
  });
});

describe("setJsonVersion", () => {
  const manifest = '{\n  "name": "app",\n  "version": "0.1.0",\n  "nested": {\n    "version": "9.9.9"\n  }\n}\n';

  it("replaces only the top-level version", () => {
    expect(setJsonVersion(manifest, "1.0.0")).toBe(
      '{\n  "name": "app",\n  "version": "1.0.0",\n  "nested": {\n    "version": "9.9.9"\n  }\n}\n',
    );
  });

  it("keeps CRLF line endings", () => {
    const windows = manifest.replaceAll("\n", "\r\n");
    expect(setJsonVersion(windows, "1.0.0")).toBe(windows.replace("0.1.0", "1.0.0"));
  });

  it("fails without a version field", () => {
    expect(() => setJsonVersion('{\n  "name": "app"\n}\n', "1.0.0")).toThrow("version");
  });
});

describe("setCargoPackageVersion", () => {
  const manifest = [
    "[package]",
    'name = "app"',
    'version = "0.1.0"',
    'authors = ["someone"]',
    "",
    "[dependencies]",
    'serde = { version = "1" }',
    'version = "3.0.0"',
    "",
  ].join("\n");

  it("replaces the package version only", () => {
    expect(setCargoPackageVersion(manifest, "1.0.0")).toBe(
      manifest.replace('version = "0.1.0"', 'version = "1.0.0"'),
    );
  });

  it("ignores version keys of other sections", () => {
    const withoutPackageVersion = manifest.replace('version = "0.1.0"\n', "");
    expect(() => setCargoPackageVersion(withoutPackageVersion, "1.0.0")).toThrow("[package]");
  });

  it("keeps CRLF line endings", () => {
    const windows = manifest.replaceAll("\n", "\r\n");
    expect(setCargoPackageVersion(windows, "1.0.0")).toBe(
      windows.replace('version = "0.1.0"', 'version = "1.0.0"'),
    );
  });
});

describe("readCargoPackageVersion", () => {
  it("reads the package version, not a dependency version", () => {
    const manifest = '[package]\nname = "app"\nversion = "2.1.0"\n\n[dependencies]\nversion = "3.0.0"\n';
    expect(readCargoPackageVersion(manifest)).toBe("2.1.0");
  });
});

describe("VERSION_FILES", () => {
  it("reads back what it wrote", () => {
    const fixtures = {
      "package.json": '{\n  "version": "0.1.0"\n}\n',
      "src-tauri/tauri.conf.json": '{\n  "version": "0.1.0"\n}\n',
      "src-tauri/Cargo.toml": '[package]\nversion = "0.1.0"\n',
    };
    for (const { path, read, set } of VERSION_FILES) {
      expect(read(set(fixtures[path], "3.2.1"))).toBe("3.2.1");
    }
  });
});

describe("setCargoLockVersion", () => {
  const lock = [
    "[[package]]",
    'name = "other"',
    'version = "0.1.0"',
    "",
    "[[package]]",
    'name = "app"',
    'version = "0.1.0"',
    'dependencies = ["other"]',
    "",
  ].join("\n");

  it("updates the entry of the named crate", () => {
    expect(setCargoLockVersion(lock, "app", "1.0.0")).toBe(
      lock.replace('name = "app"\nversion = "0.1.0"', 'name = "app"\nversion = "1.0.0"'),
    );
  });

  it("fails when the crate is missing", () => {
    expect(() => setCargoLockVersion(lock, "missing", "1.0.0")).toThrow("missing");
  });
});
