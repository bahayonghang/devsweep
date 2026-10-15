import { describe, expect, it } from "vitest";
import advisoryJson from "./fixtures/clean/advisories.json";
import preferencesJson from "./fixtures/clean/advisory-preferences.json";
import { decodeCleanAdvisories, decodeCleanAdvisoryPreferences } from "./contract";
import type { CleanAdvisoriesV1 } from "./types.gen";

describe("closed read-only Clean capacity contract", () => {
  it("decodes the core-produced exact, lower-bound and unknown observations", () => {
    const result = decodeCleanAdvisories(advisoryJson);
    expect(result.projects.map((project) => project.artifacts.confidence)).toEqual(["exact", "exact", "unknown", "lower_bound"]);
    expect(result.projects.map((project) => project.assessment)).toEqual(["below_threshold", "oversized", "incomplete", "oversized"]);
    expect(decodeCleanAdvisoryPreferences(preferencesJson).threshold_bytes).toBe(5368709120);
  });

  it.each([0, -1073741824, 1073741825, Number.MAX_SAFE_INTEGER + 1, 1.5, NaN, Infinity])("refuses invalid threshold bytes %s", (threshold_bytes) => {
    expect(() => decodeCleanAdvisoryPreferences({ version: 1, threshold_bytes })).toThrow();
  });

  it("preserves the whole GiB safe-integer limit and closed preference shape", () => {
    expect(decodeCleanAdvisoryPreferences({ version: 1, threshold_bytes: 8388607 * 1073741824 }).threshold_bytes).toBe(8388607 * 1073741824);
    expect(() => decodeCleanAdvisoryPreferences({ ...preferencesJson, version: 2 })).toThrow();
    expect(() => decodeCleanAdvisoryPreferences({ ...preferencesJson, roots: [] })).toThrow();
  });

  it("rejects invented fields, duplicate identities, inconsistent confidence and false healthy claims", () => {
    const mutate = (edit: (value: CleanAdvisoriesV1) => void) => {
      const value = decodeCleanAdvisories(structuredClone(advisoryJson));
      edit(value);
      expect(() => decodeCleanAdvisories(value)).toThrow();
    };
    expect(() => decodeCleanAdvisories({ ...advisoryJson, selected_ids: [] })).toThrow();
    mutate((value) => { value.version = 2; });
    mutate((value) => { value.report_id = "unbound"; });
    mutate((value) => { value.included_kinds.push("dependency_directory"); });
    mutate((value) => { value.projects.push(value.projects[0]); });
    mutate((value) => { value.projects[0].artifacts.known_bytes = Number.MAX_SAFE_INTEGER + 1; });
    mutate((value) => { value.projects[0].artifacts.confidence = "unknown"; });
    mutate((value) => { value.projects[0].artifacts.target_ids.push(value.projects[0].artifacts.target_ids[0]); });
    mutate((value) => { value.projects[0].excluded_target_ids.push(value.projects[0].artifacts.target_ids[0]); });
    mutate((value) => { value.projects[2].assessment = "below_threshold"; });
  });
});
