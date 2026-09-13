import { describe, expect, it } from "vitest";
import variants from "../../api/fixtures/contract-variants.json";
import type { RiskLevel, TargetKind } from "../../api/types.gen";
import { kindLabel, riskLabel } from "./labels";

const locales = ["en", "zh-CN"] as const;
const kinds = variants.string_enums.TargetKind as TargetKind[];
const risks = variants.string_enums.RiskLevel as RiskLevel[];

describe("Clean kind and risk labels", () => {
  it("maps every generated TargetKind to a catalogue key in both locales", () => {
    expect(kinds).toEqual([
      "package_cache",
      "build_artifacts",
      "dependency_directory",
      "virtual_env",
      "test_cache",
      "tool_cache",
    ]);
    expect(kindLabel("en", "build_artifacts")).toBe("Build artifacts");
    expect(kindLabel("zh-CN", "build_artifacts")).toBe("构建产物");
    for (const locale of locales) {
      for (const kind of kinds) {
        const label = kindLabel(locale, kind);
        expect(label.length).toBeGreaterThan(0);
        expect(label).not.toBe(kind);
        expect(label).not.toContain("_");
      }
    }
  });

  it("maps every generated RiskLevel to a catalogue key in both locales", () => {
    expect(risks).toEqual(["low", "medium", "high", "dangerous"]);
    expect(riskLabel("en", "dangerous")).toBe("Dangerous");
    expect(riskLabel("zh-CN", "dangerous")).toBe("危险");
    for (const locale of locales) {
      for (const risk of risks) {
        const label = riskLabel(locale, risk);
        expect(label.length).toBeGreaterThan(0);
        expect(label).not.toBe(risk);
      }
    }
  });
});
