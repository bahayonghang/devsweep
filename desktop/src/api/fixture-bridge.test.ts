import { describe, expect, it, vi } from "vitest";
import scanJson from "./fixtures/scan-report.json";
import replays from "./fixtures/clean/scan-advisory-replays.json";
import { decodeDesktopScanProgress, decodeScanReport } from "./contract";
import { fixtureBridge } from "./fixture-bridge";

const plan = decodeScanReport(scanJson).plan;
const cargo = "cargo.target:C:/work/app/target";
const npm = "npm.cache.clean:global";

describe("fixtureBridge selection contract", () => {
  it("emits invocation-correlated preview and explicit cancellation", async () => {
    const progress = vi.fn();
    const pending = fixtureBridge.scanStart(
      "fixture-cancel",
      { include_projects: true, include_global: true, roots: ["."] },
      progress,
      vi.fn(),
    );
    await fixtureBridge.scanCancel("stale");
    await fixtureBridge.scanCancel("fixture-cancel");

    await expect(pending).resolves.toEqual({ type: "canceled", scan_id: "fixture-cancel" });
    expect(progress).toHaveBeenCalledWith(expect.objectContaining({ scan_id: "fixture-cancel", sequence: 1 }));
  });

  it("changes the dry-run report and digest with selection", async () => {
    const one = await fixtureBridge.planDryRun(plan, [cargo]);
    const two = await fixtureBridge.planDryRun(plan, [cargo, npm]);
    expect(one.report.selected).toBe(1);
    expect(two.report.selected).toBe(2);
    expect(two.report.outcomes.map((outcome) => outcome.target_id)).toEqual([cargo, npm]);
    expect(two.report.estimated_recoverable).toEqual({ verified_bytes: 524288000, partial_lower_bound_bytes: 134217728, unknown_target_count: 0 });
    expect(two.digest).not.toBe(one.digest);
  });

  it("keeps global-only rows, progress and totals free of project data", async () => {
    const progress = vi.fn();
    const result = await fixtureBridge.scanStart("global-only", { include_projects: false, include_global: true, roots: [] }, progress, vi.fn());
    expect(result.type).toBe("completed");
    if (result.type !== "completed") throw new Error("expected fixture completion");
    expect(result.report.plan.targets.map((target) => target.id)).toEqual(["cargo.home.inspect:C:/Users/dev/.cargo", npm]);
    expect(result.report.health.totals).toEqual({ verified_bytes: 0, partial_lower_bound_bytes: 134217728, unknown_target_count: 1 });
    for (const [raw] of progress.mock.calls) {
      const event = decodeDesktopScanProgress(raw);
      expect(event.phase).toBe("global");
      expect(event.preview?.targets.every((target) => target.scope.type === "global")).toBe(true);
      expect(event.preview?.totals).toEqual({ target_count: 2, ...result.report.health.totals });
    }
    expect((await fixtureBridge.cleanAdvisories(result.report, 5)).projects).toEqual([]);
    expect((await fixtureBridge.cleanAdvisories(result.report, 5)).report_id).toBe(replays.global.advisories.report_id);
  });

  it("emits only the explicit fixture project and leaves it opt-in", async () => {
    const progress = vi.fn();
    const result = await fixtureBridge.scanStart("projects-only", { include_projects: true, include_global: false, roots: ["C:\\work\\app\\"] }, progress, vi.fn());
    if (result.type !== "completed") throw new Error("expected fixture completion");
    expect(result.report.plan.targets.map((target) => target.id)).toEqual([cargo]);
    expect(result.report.plan.targets[0].selected_by_default).toBe(false);
    expect(result.report.health.totals).toEqual({ verified_bytes: 524288000, partial_lower_bound_bytes: 0, unknown_target_count: 0 });
    expect(result.report.health.diagnostics).toEqual([]);
    expect(result.report.health.coverage).toBeUndefined();
    for (const [raw] of progress.mock.calls) {
      const event = decodeDesktopScanProgress(raw);
      expect(event.phase).toBe("projects");
      expect(event.preview?.targets.map((target) => target.id)).toEqual([cargo]);
      expect(event.preview?.totals).toEqual({ target_count: 1, ...result.report.health.totals });
    }
    const advice = await fixtureBridge.cleanAdvisories(result.report, 5);
    expect(advice.projects).toHaveLength(1);
    expect(advice.projects[0].artifacts.known_bytes).toBe(524288000);
    expect(advice.projects[0].assessment).toBe("incomplete");
    expect(advice.report_id).toBe(replays.projects.advisories.report_id);
    expect(advice.report_id).not.toBe(replays.global.advisories.report_id);
    expect((await fixtureBridge.cleanAdvisories(result.report, 7)).report_id).toBe(advice.report_id);
    expect(plan.targets[0].selected_by_default).toBe(true); // archived compatibility input is unchanged
  });

  it("does not invent project discoveries for unsupported explicit fixture roots", async () => {
    const progress = vi.fn();
    const result = await fixtureBridge.scanStart("unsupported-root", { include_projects: true, include_global: false, roots: ["C:/somewhere-else"] }, progress, vi.fn());
    if (result.type !== "completed") throw new Error("expected fixture completion");
    expect(result.report.plan.targets).toEqual([]);
    expect(result.report.health.completeness).toBe("partial");
    expect(result.report.health.diagnostics).toEqual([expect.objectContaining({ stage: "discovery", outcome: "skipped", path: "C:/somewhere-else" })]);
    expect(progress.mock.calls.every(([raw]) => decodeDesktopScanProgress(raw).preview?.targets.length === 0)).toBe(true);
    await expect(fixtureBridge.cleanAdvisories(result.report, 5)).rejects.toThrow("archived matching report");
  });

  it("executes only the digest matching the current selection", async () => {
    const two = await fixtureBridge.planDryRun(plan, [cargo, npm]);
    const report = await fixtureBridge.planExecute(plan, [cargo, npm], two.digest);
    expect(report.confirmation_digest).toBe(two.digest);
    expect(report.outcomes.map((outcome) => outcome.target_id)).toEqual([cargo, npm]);
    await expect(fixtureBridge.planExecute(plan, [cargo], two.digest)).rejects.toMatchObject({ code: "stale_confirmation" });
  });

  it("supports global-only npm review and requires its own fixture confirmation token", async () => {
    const globalPlan = decodeScanReport(replays.global.report).plan;
    const preview = await fixtureBridge.planDryRun(globalPlan, [npm]);
    expect(preview.report.selected).toBe(1);
    expect(preview.report.outcomes).toEqual([expect.objectContaining({ target_id: npm, action: { type: "command", irreversible: true } })]);
    expect(preview.report.estimated_recoverable).toEqual({ verified_bytes: 0, partial_lower_bound_bytes: 134217728, unknown_target_count: 0 });
    await expect(fixtureBridge.planExecute(globalPlan, [npm], (await fixtureBridge.planDryRun(plan, [cargo])).digest)).rejects.toMatchObject({ code: "stale_confirmation" });
    const outcome = await fixtureBridge.planExecute(globalPlan, [npm], preview.digest);
    expect(outcome.dry_run).toBe(false);
    expect(outcome.outcomes.map((item) => item.target_id)).toEqual([npm]);
    expect(outcome.confirmation_digest).toBe(preview.digest);
  });
});
