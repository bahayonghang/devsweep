import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const generator = path.join(desktopRoot, "scripts/generate-types.mjs");
const fixtures = path.join(desktopRoot, "src/api/fixtures");

function generate(fixtureRoot = fixtures): string {
  return execFileSync(process.execPath, [generator, "--fixtures", fixtureRoot, "--stdout"], {
    cwd: desktopRoot,
    encoding: "utf8",
  });
}

describe("IPC type generation", () => {
  it("is deterministic, committed, ASCII, and type-only", () => {
    const first = generate();
    const second = generate();
    expect(second).toBe(first);
    expect(first).toBe(readFileSync(path.join(desktopRoot, "src/api/types.gen.ts"), "utf8"));
    expect([...first].every((character) => character.charCodeAt(0) <= 127)).toBe(true);
    expect(first).not.toContain("export class Convert");
  });

  it("changes when a named fixture changes", () => {
    const fixtureCopy = mkdtempSync(path.join(tmpdir(), "devsweep-types-"));
    try {
      cpSync(fixtures, fixtureCopy, { recursive: true });
      const progressPath = path.join(fixtureCopy, "scan-progress.json");
      const progress = JSON.parse(readFileSync(progressPath, "utf8"));
      progress.phase = "fixture_mutation";
      writeFileSync(progressPath, `${JSON.stringify(progress, null, 2)}\n`, "utf8");
      const changed = generate(fixtureCopy);
      expect(changed).not.toBe(generate());
      expect(changed).toContain('"fixture_mutation"');
    } finally {
      rmSync(fixtureCopy, { recursive: true, force: true });
    }
  });
});
