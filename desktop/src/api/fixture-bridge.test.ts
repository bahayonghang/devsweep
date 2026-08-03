import { describe, expect, it } from "vitest";
import scanJson from "./fixtures/scan-report.json";
import { decodeScanReport } from "./contract";
import { fixtureBridge } from "./fixture-bridge";

const plan = decodeScanReport(scanJson).plan;
const cargo = "cargo.target:C:/work/app/target";
const npm = "npm.cache.clean:global";

describe("fixtureBridge selection contract", () => {
  it("changes the dry-run report and digest with selection", async () => {
    const one = await fixtureBridge.planDryRun(plan, [cargo]);
    const two = await fixtureBridge.planDryRun(plan, [cargo, npm]);
    expect(one.report.selected).toBe(1);
    expect(two.report.selected).toBe(2);
    expect(two.report.outcomes.map((outcome) => outcome.target_id)).toEqual([cargo, npm]);
    expect(two.report.estimated_recoverable).toEqual({ verified_bytes: 524288000, partial_lower_bound_bytes: 134217728, unknown_target_count: 0 });
    expect(two.digest).not.toBe(one.digest);
  });

  it("executes only the digest matching the current selection", async () => {
    const two = await fixtureBridge.planDryRun(plan, [cargo, npm]);
    const report = await fixtureBridge.planExecute(plan, [cargo, npm], two.digest);
    expect(report.confirmation_digest).toBe(two.digest);
    expect(report.outcomes.map((outcome) => outcome.target_id)).toEqual([cargo, npm]);
    await expect(fixtureBridge.planExecute(plan, [cargo], two.digest)).rejects.toMatchObject({ code: "stale_confirmation" });
  });
});
