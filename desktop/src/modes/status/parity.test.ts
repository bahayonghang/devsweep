import { describe, expect, it } from "vitest";
import snapshotJson from "../../api/fixtures/status/snapshot.json";
import startedJson from "../../api/fixtures/status/event-started.json";
import { decodeStatusEvent, decodeStatusSnapshot } from "../../api/contract";

describe("status cross-surface fixtures", () => {
  it("keeps process rows on the closed privacy set and omits GPU as a measured card", () => {
    const snapshot = decodeStatusSnapshot(snapshotJson);
    expect(snapshot.unsupported_capabilities.map((item) => item.code)).toEqual([
      "gpu_utilization", "vram", "thermal", "fan", "smart", "physical_disk_activity",
    ]);
    expect(snapshot.processes.state === "partial" ? snapshot.processes.value?.items[0] : undefined).toEqual({
      pid: 42,
      name: "fixture.exe",
      cpu_basis_points_of_one_logical_core: 12500,
      private_bytes: 1048576,
      read_bytes_per_second: 2048,
      write_bytes_per_second: 1024,
    });
    expect(JSON.stringify(snapshot)).not.toMatch(/cmdline|gpu_utilization":0|"user"/);
  });

  it("starts live only after an explicit status_started event", () => {
    const started = decodeStatusEvent(startedJson);
    expect(started.event).toBe("status_started");
    expect(started.sequence).toBe(0);
    expect(started.data).toEqual({ interval_ms: 2000, process_limit: 15 });
  });
});
