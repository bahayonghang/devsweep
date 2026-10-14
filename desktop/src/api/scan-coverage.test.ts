import { describe, expect, it } from "vitest";
import fixture from "./fixtures/clean/scan-coverage.json";
import legacy from "./fixtures/scan-report.json";
import preferences from "./fixtures/clean/scan-preferences.json";
import { decodeCleanScanPreferences, decodeScanReport } from "./contract";

describe("closed scan coverage", () => {
  it("retains all typed outcomes with zero targets and accepts legacy missing coverage", () => {
    const report = decodeScanReport(fixture);
    expect(report.plan.targets).toEqual([]);
    expect(report.health.coverage).toHaveLength(13);
    expect(decodeScanReport(legacy).health.coverage).toBeUndefined();
  });
  it("rejects duplicated units, unknown statuses, authority fields and complete partial coverage", () => {
    for (const mutation of [
      (value: typeof fixture) => value.health.coverage.push(value.health.coverage[0]),
      (value: typeof fixture) => { value.health.coverage[0].status = "deleted"; },
      (value: typeof fixture) => { Object.assign(value.health.coverage[0], { argv: ["clean"] }); },
      (value: typeof fixture) => { value.health.completeness = "complete"; },
    ]) {
      const value = structuredClone(fixture); mutation(value);
      expect(() => decodeScanReport(value)).toThrow();
    }
  });
  it("decodes only the versioned root snapshot", () => {
    expect(decodeCleanScanPreferences(preferences)).toEqual(preferences);
    for (const value of [{ ...preferences, version: 2 }, { ...preferences, exclude: [] }, { ...preferences, roots: ["x", "x"] }, { ...preferences, roots: [""] }]) {
      expect(() => decodeCleanScanPreferences(value)).toThrow();
    }
  });
});
