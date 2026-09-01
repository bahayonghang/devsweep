import { describe, expect, it } from "vitest";
import catalogueJson from "../../api/fixtures/optimize/catalogue.json";
import executionJson from "../../api/fixtures/optimize/execution-five-terminal.json";
import launchedJson from "../../api/fixtures/optimize/execution-settings-launched.json";
import {
  decodeDesktopOptimizeListResult,
  decodeDesktopOptimizeRunResult,
} from "../../api/contract";

describe("optimize cross-surface fixtures", () => {
  it("covers the closed eight ids and omits executable facts", () => {
    const catalogue = decodeDesktopOptimizeListResult(catalogueJson);
    if (catalogue.type !== "completed") throw new Error("fixture catalogue must complete");
    expect(catalogue.entries.map((entry) => entry.id)).toEqual([
      "dns.flush",
      "settings.storage_recommendations",
      "settings.search",
      "settings.energy_recommendations",
      "guidance.drive_optimize",
      "guidance.system_integrity",
      "guidance.filesystem_check",
      "guidance.network_reset",
    ]);
    expect(catalogue.entries.map((entry) => entry.action_class)).toEqual([
      "execute", "settings_handoff", "settings_handoff", "settings_handoff",
      "guidance", "guidance", "guidance", "guidance",
    ]);
    expect(JSON.stringify(catalogue)).not.toMatch(/ipconfig|ms-settings|"program"\s*:|"argv"\s*:/i);
  });

  it("keeps launched distinct from success and closes the five post-dispatch terminals", () => {
    const launched = decodeDesktopOptimizeRunResult(launchedJson).report;
    expect(launched.outcomes[0].outcome).toBe("launched");
    expect(launched.outcomes[0].action_class).toBe("settings_handoff");
    expect(JSON.stringify(launched)).not.toContain("succeeded");
    const report = decodeDesktopOptimizeRunResult(executionJson).report;
    expect(new Set(report.outcomes.map((item) => item.outcome))).toEqual(new Set([
      "canceled_before_start", "succeeded", "launched", "failed", "unknown_after_dispatch",
    ]));
  });
});
