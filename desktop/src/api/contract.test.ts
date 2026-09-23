import { describe, expect, it } from "vitest";
import scanFixture from "./fixtures/scan-report.json";
import dryRunFixture from "./fixtures/dry-run-outcome.json";
import executionFixture from "./fixtures/execution-report.json";
import progressFixture from "./fixtures/scan-progress.json";
import analyzeSnapshotFixture from "./fixtures/analyze/snapshot.json";
import analyzeProgressFixture from "./fixtures/analyze/progress.json";
import analyzeCompleteFixture from "./fixtures/analyze/complete.json";
import analyzeTrashPreviewFixture from "./fixtures/analyze/trash-preview.json";
import analyzeTrashReportFixture from "./fixtures/analyze/trash-report.json";
import statusSnapshotFixture from "./fixtures/status/snapshot.json";
import statusCompletedFixture from "./fixtures/status/snapshot-completed.json";
import statusStartedFixture from "./fixtures/status/event-started.json";
import statusUnknownFixture from "./fixtures/status/unknown-event.json";
import commandErrorFixtures from "./fixtures/errors/command-errors.json";
import { decodeAnalyzeSnapshot, decodeAnalyzeTrashPreview, decodeAnalyzeTrashReport, decodeCommandError, decodeDesktopAnalyzeProgress, decodeDesktopAnalyzeResult, decodeDesktopScanProgress, decodeDesktopStatusSnapshotResult, decodeDryRunOutcome, decodeExecutedReport, decodeExecutionReport, decodeScanReport, decodeStatusEvent, decodeStatusSnapshot } from "./contract";

describe("IPC decoders", () => {
  it("decodes archived command and event fixtures", () => {
    expect(decodeScanReport(scanFixture).plan.targets).toHaveLength(3);
    expect(decodeDesktopScanProgress(progressFixture).phase).toBe("projects");
    expect(decodeDryRunOutcome(dryRunFixture).digest).toBe(dryRunFixture.digest);
    expect(decodeExecutionReport(executionFixture).outcomes).toHaveLength(2);
    expect(decodeAnalyzeSnapshot(analyzeSnapshotFixture).nodes).toHaveLength(2);
    expect(decodeDesktopAnalyzeProgress(analyzeProgressFixture).sequence).toBe(1);
    expect(decodeDesktopAnalyzeResult(analyzeCompleteFixture).type).toBe("completed");
  });

  it("rejects malformed Analyze identities, sequences, bounds, and authority fields", () => {
    expect(() => decodeDesktopAnalyzeProgress({ ...analyzeProgressFixture, sequence: 0 })).toThrow("progress bounds");
    expect(() => decodeDesktopAnalyzeProgress({ ...analyzeProgressFixture, queue_depth: 5 })).toThrow("progress bounds");
    expect(() => decodeDesktopAnalyzeProgress({ ...analyzeProgressFixture, operation_id: " " })).toThrow("operation_id");
    expect(() => decodeAnalyzeSnapshot({ ...analyzeSnapshotFixture, version: 2 })).toThrow("Unsupported analyze snapshot version");
    const badParent = structuredClone(analyzeSnapshotFixture);
    badParent.nodes[1].parent_id = 99;
    expect(() => decodeAnalyzeSnapshot(badParent)).toThrow("parent identity");
    expect(() => decodeAnalyzeSnapshot({ ...analyzeSnapshotFixture, cleanup: true })).toThrow("unknown field");
  });

  it("decodes Analyze Recycle Bin previews and reports as closed, id-bound shapes", () => {
    const preview = decodeAnalyzeTrashPreview(analyzeTrashPreviewFixture);
    expect(preview.items.map((item) => item.target_id)).toEqual(["analyze.trash:analyze-op-1:1"]);
    expect(preview.refused).toEqual([{ node_id: 0, reason_code: "analysis_root" }]);
    expect(decodeAnalyzeTrashReport(analyzeTrashReportFixture).moved_node_ids).toEqual([1]);

    const otherTarget = structuredClone(analyzeTrashPreviewFixture);
    otherTarget.items[0].target_id = "analyze.trash:other:1";
    expect(() => decodeAnalyzeTrashPreview(otherTarget)).toThrow("target identity");
    const badRefusal = structuredClone(analyzeTrashPreviewFixture);
    badRefusal.refused[0].reason_code = "permanent_delete";
    expect(() => decodeAnalyzeTrashPreview(badRefusal)).toThrow("reason_code");
    expect(() => decodeAnalyzeTrashPreview({ ...analyzeTrashPreviewFixture, digest: `sha256:${"a".repeat(64)}` })).toThrow("digest");
    expect(() => decodeAnalyzeTrashPreview({ ...analyzeTrashPreviewFixture, command: "cmd.exe" })).toThrow("unknown field");
    expect(() => decodeAnalyzeTrashReport({ ...analyzeTrashReportFixture, moved_node_ids: [1, 2] })).toThrow("moved nodes");
    const deleted = structuredClone(analyzeTrashReportFixture);
    (deleted.report.outcomes[0].action as { type: string }).type = "permanent_delete";
    expect(() => decodeAnalyzeTrashReport(deleted)).toThrow();
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

  it("rejects progress payloads that expose cleanup authority", () => {
    const unsafe = structuredClone(progressFixture) as typeof progressFixture & { preview: { targets: Array<Record<string, unknown>> } };
    unsafe.preview.targets[0].program = "cmd.exe";
    expect(() => decodeDesktopScanProgress(unsafe)).toThrow("unknown field");
    expect(() => decodeDesktopScanProgress({ ...progressFixture, partial: { version: 2, targets: [] } })).toThrow("unknown field");
  });

  it("rejects unsafe sequences, duplicate ids, and unknown preview fields", () => {
    expect(() => decodeDesktopScanProgress({ ...progressFixture, sequence: Number.MAX_SAFE_INTEGER + 1 })).toThrow("Invalid progress.sequence");
    expect(() => decodeDesktopScanProgress({ ...progressFixture, scan_id: " " })).toThrow("Invalid progress.scan_id");
    const duplicate = structuredClone(progressFixture);
    duplicate.preview.targets.push(structuredClone(duplicate.preview.targets[0]));
    duplicate.preview.totals.target_count = 2;
    expect(() => decodeDesktopScanProgress(duplicate)).toThrow("duplicate target id");
    const badTotals = structuredClone(progressFixture);
    badTotals.preview.totals.verified_bytes += 1;
    expect(() => decodeDesktopScanProgress(badTotals)).toThrow("capacity totals");
    expect(() => decodeDesktopScanProgress({ ...progressFixture, extra: true })).toThrow("unknown field");
  });

  it.each(commandErrorFixtures)("decodes structured error $code", (error) => {
    expect(decodeCommandError(error).code).toBe(error.code);
  });

  // desktop/src-tauri/src/wire_parity.rs decodes the same file into the Rust
  // CommandError, so one payload set covers both surfaces.
  it("carries one payload for every structured error the decoder accepts", () => {
    expect(new Set(commandErrorFixtures.map((error) => error.code)).size).toBe(commandErrorFixtures.length);
    expect(commandErrorFixtures).toHaveLength(28);
    expect(() => decodeCommandError({ code: "not_a_command_error" })).toThrow("Invalid error.code");
  });
});

describe("Status V1 decoders", () => {
  it("decodes the frozen snapshot and rejects process privacy fields", () => {
    const snapshot = decodeStatusSnapshot(statusSnapshotFixture);
    expect(snapshot.cpu.state).toBe("available");
    expect(snapshot.memory.state).toBe("partial");
    expect(snapshot.power.state === "available" && snapshot.power.value?.battery_present === false).toBe(true);
    expect(snapshot.processes.state === "partial" && snapshot.processes.value?.truncated_by_limit === true).toBe(true);
    expect(JSON.stringify(snapshot)).not.toMatch(/cmdline|executable_path|"user"/);
    expect(decodeDesktopStatusSnapshotResult(statusCompletedFixture).type).toBe("completed");
    expect(decodeStatusEvent(statusStartedFixture).event).toBe("status_started");
    const leaked = structuredClone(statusSnapshotFixture) as typeof statusSnapshotFixture & {
      processes: { value: { items: Array<Record<string, unknown>> } };
    };
    leaked.processes.value.items[0].cmdline = "secret";
    expect(() => decodeStatusSnapshot(leaked)).toThrow("unknown field");
    expect(() => decodeStatusEvent(statusUnknownFixture)).toThrow("status event.event");
    expect(() => decodeStatusSnapshot({ ...statusSnapshotFixture, gpu: 0 })).toThrow("unknown field");
  });
});
