import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// The three typed adapters allowed to reach Tauri. bridge.ts owns every
// five-mode and supporting-domain operation; the other two are the declared
// non-domain exceptions for presentation settings and window lifecycle.
const DECLARED_ADAPTERS = ["api/bridge.ts", "i18n/index.ts", "lifecycle.ts"].sort();

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(entryPath);
    if (!/\.tsx?$/.test(entry.name) || /\.test\.tsx?$/.test(entry.name)) return [];
    return [entryPath];
  });
}

describe("React to Tauri boundary", () => {
  const files = sourceFiles(sourceRoot);

  it("reads the whole desktop frontend", () => {
    expect(files.length).toBeGreaterThan(20);
  });

  it("imports Tauri only in the three declared typed adapters", () => {
    const importers = files
      .filter((file) => /from "@tauri-apps\//.test(readFileSync(file, "utf8")))
      .map((file) => path.relative(sourceRoot, file).replaceAll("\\", "/"))
      .sort();
    expect(importers).toEqual(DECLARED_ADAPTERS);
  });
});
