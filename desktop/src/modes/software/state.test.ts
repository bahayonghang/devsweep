import { describe, expect, it } from "vitest";
import inventoryJson from "../../api/fixtures/software/inventory.json";
import previewJson from "../../api/fixtures/software/preview.json";
import executionJson from "../../api/fixtures/software/execution-five-terminal.json";
import auditJson from "../../api/fixtures/software/audit-restart.json";
import { decodeDesktopSoftwareAuditResult, decodeDesktopSoftwarePreviewResult, decodeDesktopSoftwareUninstallResult, decodeSoftwareInventory } from "../../api/contract";
import { initialSoftwareState, softwareReducer } from "./state";

const inventory = decodeSoftwareInventory(inventoryJson);
const preview = decodeDesktopSoftwarePreviewResult(previewJson);
const execution = decodeDesktopSoftwareUninstallResult(executionJson);
const audit = decodeDesktopSoftwareAuditResult(auditJson);

function loadedState() {
  const state = softwareReducer(initialSoftwareState, { type: "operation_requested", operationId: "inventory", operation: "inventory" });
  return softwareReducer(state, { type: "inventory_completed", operationId: "inventory", inventory });
}

describe("software reducer", () => {
  it("selects only exact eligible current-user MSIX identities", () => {
    let state = loadedState();
    state = softwareReducer(state, { type: "selection_changed", softwareId: "software:v1:msi:user-unmanaged", selected: true });
    expect(state.selectedIds.size).toBe(0);
    state = softwareReducer(state, { type: "selection_changed", softwareId: "software:v1:msix:eligible-contoso", selected: true });
    expect([...state.selectedIds]).toEqual(["software:v1:msix:eligible-contoso"]);
  });

  it("rejects stale completions and invalidates preview on selection changes", () => {
    let state = loadedState();
    state = softwareReducer(state, { type: "selection_changed", softwareId: "software:v1:msix:eligible-contoso", selected: true });
    state = softwareReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" });
    expect(softwareReducer(state, { type: "preview_completed", operationId: "stale", result: preview })).toBe(state);
    state = softwareReducer(state, { type: "preview_completed", operationId: "preview", result: { ...preview, operation_id: "preview" } });
    expect(state.status).toBe("preview_ready");
    state = softwareReducer(state, { type: "selection_changed", softwareId: "software:v1:msix:eligible-contoso", selected: false });
    expect(state.preview).toBeNull();
    expect(state.plan).toBeNull();
  });

  it("requires preview confirmation and preserves unknown-after-dispatch", () => {
    let state = loadedState();
    state = softwareReducer(state, { type: "selection_changed", softwareId: "software:v1:msix:eligible-contoso", selected: true });
    state = softwareReducer(state, { type: "operation_requested", operationId: "preview", operation: "preview" });
    state = softwareReducer(state, { type: "preview_completed", operationId: "preview", result: { ...preview, operation_id: "preview" } });
    expect(softwareReducer(state, { type: "operation_requested", operationId: "execute", operation: "uninstall" })).toBe(state);
    state = softwareReducer(state, { type: "confirmation_opened" });
    state = softwareReducer(state, { type: "operation_requested", operationId: "execute", operation: "uninstall" });
    const matching = {
      operation_id: "execute",
      report: {
        ...execution.report,
        outcomes: [execution.report.outcomes.find((item) => item.outcome === "unknown_after_dispatch")!]
          .map((item) => ({ ...item, software_id: "software:v1:msix:eligible-contoso" })),
      },
    };
    state = softwareReducer(state, { type: "uninstall_completed", operationId: "execute", result: matching });
    expect(state.status).toBe("unknown");
    expect(state.report?.outcomes[0].outcome).toBe("unknown_after_dispatch");
  });

  it("accepts only matching audit recovery operations", () => {
    let state = loadedState();
    state = softwareReducer(state, { type: "operation_requested", operationId: "audit", operation: "audit" });
    expect(softwareReducer(state, { type: "audit_completed", operationId: "stale", result: audit })).toBe(state);
    state = softwareReducer(state, { type: "audit_completed", operationId: "audit", result: { ...audit, operation_id: "audit" } });
    expect(state.audit?.recovered[0].outcome).toBe("unknown_after_dispatch");
    expect(state.audit?.records.filter((record) => record.transition.kind === "dispatch_started")).toHaveLength(1);
  });
});
