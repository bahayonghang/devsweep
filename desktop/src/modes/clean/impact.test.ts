import { describe, expect, it } from "vitest";
import fixture from "../../api/fixtures/scan-report.json";
import { decodeScanReport } from "../../api/contract";
import type { TargetKind, UntrustedTarget } from "../../api/types.gen";
import { IMPACT_RANK, orderGroups } from "./impact";

const [base] = decodeScanReport(fixture).plan.targets;

function target(
  id: string,
  kind: TargetKind,
  bytes: number,
  complete: boolean,
  scope: "project" | "global" = "project",
): UntrustedTarget {
  return {
    ...base,
    id,
    kind,
    estimated_bytes: bytes,
    size_complete: complete,
    scope:
      scope === "project"
        ? { type: "project", root: "C:/work/app" }
        : { type: "global" },
  };
}

describe("impact order", () => {
  it("ranks regenerable caches first, then build artifacts, then dependencies and environments", () => {
    expect(IMPACT_RANK.package_cache).toBe(IMPACT_RANK.tool_cache);
    expect(IMPACT_RANK.tool_cache).toBe(IMPACT_RANK.test_cache);
    expect(IMPACT_RANK.test_cache).toBeLessThan(IMPACT_RANK.build_artifacts);
    expect(IMPACT_RANK.build_artifacts).toBeLessThan(
      IMPACT_RANK.dependency_directory,
    );
    expect(IMPACT_RANK.dependency_directory).toBe(IMPACT_RANK.virtual_env);

    const groups = orderGroups([
      target("venv", "virtual_env", 9_000, true),
      target("build", "build_artifacts", 9_000, true),
      target("deps", "dependency_directory", 1, true),
      target("test", "test_cache", 10, true),
      target("tool", "tool_cache", 20, true),
      target("pkg", "package_cache", 5, true, "global"),
    ]);
    expect(groups.map((group) => group.kind)).toEqual([
      "tool_cache",
      "test_cache",
      "package_cache",
      "build_artifacts",
      "virtual_env",
      "dependency_directory",
    ]);
  });

  it("sorts groups and rows by verified bytes, then lower bound, then unknown", () => {
    const groups = orderGroups([
      target("unknown", "build_artifacts", 0, false),
      target("lower-large", "build_artifacts", 900, false),
      target("verified-small", "build_artifacts", 10, true),
      target("lower-small", "build_artifacts", 100, false),
      target("verified-large", "build_artifacts", 500, true),
      target("global-lower", "build_artifacts", 10_000, false, "global"),
    ]);
    expect(groups.map((group) => group.scope)).toEqual(["project", "global"]);
    expect(groups[0].targets.map((row) => row.id)).toEqual([
      "verified-large",
      "verified-small",
      "lower-large",
      "lower-small",
      "unknown",
    ]);
    expect(
      groups[0].targets.find((row) => row.id === "lower-large")?.size_complete,
    ).toBe(false);
    expect(
      groups[0].targets.find((row) => row.id === "unknown")?.estimated_bytes,
    ).toBe(0);
  });

  it("places a group with only lower-bound bytes after one with verified bytes in the same rank", () => {
    const groups = orderGroups([
      target("pkg-lower", "package_cache", 9_000, false, "global"),
      target("tool-verified", "tool_cache", 1, true),
    ]);
    expect(groups.map((group) => group.kind)).toEqual([
      "tool_cache",
      "package_cache",
    ]);
  });
});
