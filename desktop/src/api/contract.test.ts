import { describe, expect, it } from "vitest";
import scanFixture from "./fixtures/scan-report.json";
import dryRunFixture from "./fixtures/dry-run-outcome.json";
import executionFixture from "./fixtures/execution-report.json";
import progressFixture from "./fixtures/scan-progress.json";
import { decodeCommandError, decodeDryRunOutcome, decodeExecutedReport, decodeExecutionReport, decodeScanProgress, decodeScanReport } from "./contract";

describe("IPC decoders", () => {
  it("decodes archived command and event fixtures", () => {
    expect(decodeScanReport(scanFixture).plan.targets).toHaveLength(3);
    expect(decodeScanProgress(progressFixture).phase).toBe("projects");
    expect(decodeDryRunOutcome(dryRunFixture).digest).toBe(dryRunFixture.digest);
    expect(decodeExecutionReport(executionFixture).outcomes).toHaveLength(2);
  });

  it("rejects missing safety fields", () => {
    const malformed = structuredClone(scanFixture) as Record<string, unknown>;
    const plan = malformed.plan as { targets: Array<Record<string, unknown>> };
    delete plan.targets[0].intent;
    expect(() => decodeScanReport(malformed)).toThrow("Invalid intent");
  });

  it("accepts the Serde-omitted empty sizing warnings field", () => {
    const serialized = structuredClone(scanFixture);
    delete (serialized.plan.targets[0] as { sizing_warnings?: unknown }).sizing_warnings;
    expect(decodeScanReport(serialized).plan.targets[0].sizing_warnings).toBeUndefined();
  });

  it("rejects unknown fields and unsafe integer payloads", () => {
    const unknown = structuredClone(scanFixture) as Record<string, unknown>;
    (unknown.plan as { targets: Array<Record<string, unknown>> }).targets[0].program = "cmd.exe";
    expect(() => decodeScanReport(unknown)).toThrow("unknown field");

    const unsafe = structuredClone(scanFixture) as Record<string, unknown>;
    (unsafe.health as { totals: { verified_bytes: number } }).totals.verified_bytes = Number.MAX_SAFE_INTEGER + 1;
    expect(() => decodeScanReport(unsafe)).toThrow("Invalid totals.verified_bytes");
  });

  it("enforces dry-run digest, report counts, and execution mode", () => {
    const staleDigest = structuredClone(dryRunFixture);
    staleDigest.digest = "different";
    expect(() => decodeDryRunOutcome(staleDigest)).toThrow("dry-run outcome invariant");

    const badCounts = structuredClone(executionFixture);
    badCounts.succeeded = 1;
    expect(() => decodeExecutionReport(badCounts)).toThrow("execution report counts");
    expect(() => decodeExecutedReport(dryRunFixture.report)).toThrow("execution result mode");
  });

  it("rejects progress payloads that expose a trusted partial plan", () => {
    expect(() => decodeScanProgress({ ...progressFixture, partial: { version: 2, targets: [{ action: { type: "command", program: "cmd.exe" } }] } })).toThrow("progress.partial authority");
  });

  it.each([
    { code: "scan_already_running" },
    { code: "scan_failed", message: "failed" },
    { code: "invalid_plan", issues: ["bad version"] },
    { code: "stale_confirmation", expected_digest: "old", actual_digest: "new" },
    { code: "unknown_target", target_id: "missing" },
    { code: "inspect_only_target", target_id: "inspect" },
    { code: "io", message: "disk" },
  ])("decodes structured error $code", (error) => {
    expect(decodeCommandError(error).code).toBe(error.code);
  });
});
