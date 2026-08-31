import { describe, expect, it } from "vitest";
import snapshotJson from "../../api/fixtures/status/snapshot.json";
import startedJson from "../../api/fixtures/status/event-started.json";
import liveSnapshotJson from "../../api/fixtures/status/event-snapshot.json";
import skippedJson from "../../api/fixtures/status/event-skipped.json";
import terminalJson from "../../api/fixtures/status/event-terminal.json";
import { decodeStatusEvent, decodeStatusSnapshot } from "../../api/contract";
import { MAX_CHART_POINTS, initialStatusState, sortedProcesses, statusReducer } from "./state";

const snapshot = decodeStatusSnapshot(snapshotJson);
const started = decodeStatusEvent(startedJson);
const liveSnapshot = decodeStatusEvent(liveSnapshotJson);
const skipped = decodeStatusEvent(skippedJson);
const terminal = decodeStatusEvent(terminalJson);

function liveState() {
  let state = statusReducer(initialStatusState, { type: "operation_requested", operationId: "live", operation: "live" });
  state = statusReducer(state, { type: "live_event", operationId: "live", event: { ...started, operation_id: "op-fixture" } });
  return state;
}

describe("status reducer", () => {
  it("loads a privacy-safe snapshot and keeps battery absent without a zero card", () => {
    let state = statusReducer(initialStatusState, { type: "operation_requested", operationId: "snap", operation: "snapshot" });
    state = statusReducer(state, { type: "snapshot_completed", operationId: "snap", snapshot });
    expect(state.status).toBe("ready");
    expect(state.snapshot?.cpu.state).toBe("available");
    expect(JSON.stringify(state.snapshot)).not.toMatch(/cmdline|"path"|executable/);
    const rows = sortedProcesses(state);
    expect(rows).toHaveLength(1);
    expect(rows[0].name).toBe("fixture.exe");
  });

  it("records live gaps, caps the chart at 60 points, and drops state on leave", () => {
    let state = liveState();
    state = statusReducer(state, { type: "live_event", operationId: "live", event: liveSnapshot });
    state = statusReducer(state, { type: "live_event", operationId: "live", event: skipped });
    expect(state.chart).toHaveLength(2);
    expect(state.chart[0].cpuBp).toBeNull();
    expect(state.chart[1].sampledAtUnixMs).toBeNull();
    for (let sequence = 3; sequence < 3 + MAX_CHART_POINTS; sequence += 1) {
      state = statusReducer(state, {
        type: "live_event",
        operationId: "live",
        event: {
          schema_version: 1,
          event: "tick_skipped",
          operation_id: "op-fixture",
          sequence,
          emitted_at_unix_ms: skipped.emitted_at_unix_ms,
          data: { reason: "sample_in_flight", skipped_total: sequence },
        },
      });
    }
    expect(state.chart).toHaveLength(MAX_CHART_POINTS);
    state = statusReducer(state, {
      type: "live_event",
      operationId: "live",
      event: {
        ...terminal,
        sequence: 3 + MAX_CHART_POINTS,
      },
    });
    expect(state.operationId).toBeNull();
    state = statusReducer(state, { type: "released" });
    expect(state.chart).toHaveLength(0);
    expect(state.snapshot).toBeNull();
  });

  it("rejects skipped sequences and extra live events from another operation", () => {
    let state = liveState();
    state = statusReducer(state, { type: "live_event", operationId: "live", event: { ...skipped, sequence: 2 } });
    expect(state.status).toBe("failed");
    state = liveState();
    const before = state;
    state = statusReducer(state, { type: "live_event", operationId: "live", event: { ...skipped, operation_id: "other-op", sequence: 1 } });
    expect(state.chart).toEqual(before.chart);
  });
});
