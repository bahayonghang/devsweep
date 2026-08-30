import { describe, expect, it } from "vitest";
import fixture from "../../api/fixtures/scan-report.json";
import { decodeScanReport } from "../../api/contract";
import { cleanReducer, initialCleanState } from "./reducer";

const report = decodeScanReport(fixture);

describe("cleanReducer", () => {
  it("rejects inspect-only selection and ignores selection during scan", () => {
    let state = cleanReducer(initialCleanState, { type: "scan_requested", scanId: "s1" });
    state = cleanReducer(state, { type: "selection_changed", targetId: "cargo.target:C:/work/app/target", selected: true });
    expect(state.selectedIds.size).toBe(0);
    state = cleanReducer(state, { type: "scan_completed", scanId: "s1", report });
    const before = [...state.selectedIds];
    state = cleanReducer(state, { type: "selection_changed", targetId: "cargo.home.inspect:C:/Users/dev/.cargo", selected: true });
    expect([...state.selectedIds]).toEqual(before);
  });
});
