import { describe, expect, it } from "vitest";
import catalogueJson from "../../api/fixtures/optimize/catalogue.json";
import previewJson from "../../api/fixtures/optimize/preview-dns.json";
import settingsPreviewJson from "../../api/fixtures/optimize/preview-settings-search.json";
import executionJson from "../../api/fixtures/optimize/execution-dns-succeeded.json";
import launchedJson from "../../api/fixtures/optimize/execution-settings-launched.json";
import auditJson from "../../api/fixtures/optimize/audit.json";
import {
  decodeDesktopOptimizeAuditResult,
  decodeDesktopOptimizeListResult,
  decodeDesktopOptimizePreviewResult,
  decodeDesktopOptimizeRunResult,
} from "../../api/contract";
import { initialOptimizeState, optimizeReducer } from "./state";

const catalogue = decodeDesktopOptimizeListResult(catalogueJson);
const preview = decodeDesktopOptimizePreviewResult(previewJson);
const settingsPreview = decodeDesktopOptimizePreviewResult(settingsPreviewJson);
const execution = decodeDesktopOptimizeRunResult(executionJson);
const launched = decodeDesktopOptimizeRunResult(launchedJson);
const audit = decodeDesktopOptimizeAuditResult(auditJson);

function loadedState() {
  const started = optimizeReducer(initialOptimizeState, { type: "operation_requested", operationId: "list", operation: "list" });
  if (catalogue.type !== "completed") throw new Error("fixture catalogue must complete");
  return optimizeReducer(started, { type: "list_completed", operationId: "list", entries: catalogue.entries });
}

describe("optimize reducer", () => {
  it("loads the closed eight-id catalogue and refuses guidance preview", () => {
    let state = loadedState();
    expect(state.entries?.map((entry) => entry.id)).toEqual([
      "dns.flush",
      "settings.storage_recommendations",
      "settings.search",
      "settings.energy_recommendations",
      "guidance.drive_optimize",
      "guidance.system_integrity",
      "guidance.filesystem_check",
      "guidance.network_reset",
    ]);
    state = optimizeReducer(state, { type: "selection_changed", catalogueId: "guidance.drive_optimize" });
    expect(state.status).toBe("selected");
    expect(optimizeReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" })).toBe(state);
  });

  it("rejects stale preview and run completions", () => {
    let state = loadedState();
    state = optimizeReducer(state, { type: "selection_changed", catalogueId: "dns.flush" });
    state = optimizeReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" });
    expect(optimizeReducer(state, { type: "preview_completed", operationId: "stale", result: preview })).toBe(state);
    state = optimizeReducer(state, { type: "preview_completed", operationId: "preview", result: { ...preview, operation_id: "preview" } });
    expect(state.status).toBe("preview_ready");
    state = optimizeReducer(state, { type: "selection_changed", catalogueId: "settings.search" });
    expect(state.preview).toBeNull();
    expect(state.plan).toBeNull();
  });

  it("requires confirmation, launches Settings without completion, and preserves unknown", () => {
    let state = loadedState();
    state = optimizeReducer(state, { type: "selection_changed", catalogueId: "settings.search" });
    state = optimizeReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" });
    state = optimizeReducer(state, { type: "preview_completed", operationId: "preview", result: { ...settingsPreview, operation_id: "preview" } });
    expect(optimizeReducer(state, { type: "operation_requested", operationId: "run", operation: "run" })).toBe(state);
    state = optimizeReducer(state, { type: "confirmation_opened" });
    state = optimizeReducer(state, { type: "operation_requested", operationId: "run", operation: "run" });
    expect(state.status).toBe("launching");
    state = optimizeReducer(state, { type: "run_completed", operationId: "run", result: { ...launched, operation_id: "run" } });
    expect(state.status).toBe("terminal");
    expect(state.report?.outcomes[0].outcome).toBe("launched");

    state = loadedState();
    state = optimizeReducer(state, { type: "selection_changed", catalogueId: "dns.flush" });
    state = optimizeReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" });
    state = optimizeReducer(state, { type: "preview_completed", operationId: "preview", result: { ...preview, operation_id: "preview" } });
    state = optimizeReducer(state, { type: "confirmation_opened" });
    state = optimizeReducer(state, { type: "operation_requested", operationId: "run", operation: "run" });
    expect(state.status).toBe("running");
    const unknown = {
      operation_id: "run",
      report: {
        ...execution.report,
        outcomes: [{ ...execution.report.outcomes[0], outcome: "unknown_after_dispatch" as const }],
      },
    };
    state = optimizeReducer(state, { type: "run_completed", operationId: "run", result: unknown });
    expect(state.status).toBe("unknown");
  });

  it("accepts only matching audit recovery operations", () => {
    let state = loadedState();
    state = optimizeReducer(state, { type: "operation_requested", operationId: "audit", operation: "audit" });
    expect(optimizeReducer(state, { type: "audit_completed", operationId: "stale", result: audit })).toBe(state);
    state = optimizeReducer(state, { type: "audit_completed", operationId: "audit", result: { ...audit, operation_id: "audit" } });
    expect(state.audit?.recovered[0].outcome).toBe("unknown_after_dispatch");
    expect(state.audit?.records.filter((record) => record.transition.kind === "dispatch_started")).toHaveLength(1);
  });
});
