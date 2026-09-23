import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import snapshotJson from "../api/fixtures/status/snapshot.json";
import hudSnapshotJson from "../api/fixtures/status/hud-snapshot.json";
import hudSamplingJson from "../api/fixtures/status/hud-sampling.json";
import { decodeHudStatusEvent, decodeStatusSnapshot } from "../api/contract";
import type { HudStatusEvent, StatusSnapshotV1 } from "../api/types.gen";
import { Hud, hudRows, type HudSubscribe } from "./Hud";

const snapshot = decodeStatusSnapshot(snapshotJson);

function harness() {
  let emit: ((event: HudStatusEvent) => void) | null = null;
  const stop = vi.fn();
  const subscribe: HudSubscribe = vi.fn(async (onEvent) => {
    emit = onEvent;
    return stop;
  });
  return {
    subscribe,
    stop,
    emit: (event: HudStatusEvent) => act(() => emit?.(event)),
  };
}

describe("HUD", () => {
  it("shows sampling first, then rows from hud-status snapshots", async () => {
    const hud = harness();
    const view = render(<Hud locale="en" subscribe={hud.subscribe} />);
    expect(screen.getByRole("status")).toHaveTextContent("Sampling…");
    await act(async () => undefined);
    hud.emit(decodeHudStatusEvent(hudSnapshotJson));
    expect(screen.getByText("CPU").nextSibling).toHaveTextContent("43.21%");
    expect(screen.getByText("GPU").nextSibling).toHaveTextContent("12.50%");
    expect(screen.getByText("Temperature").nextSibling).toHaveTextContent(
      "Unavailable",
    );
    expect(screen.getByText("Network")).toBeInTheDocument();
    expect(screen.getAllByText("Disk").length).toBeGreaterThan(0);
    expect(screen.queryByText("Battery")).not.toBeInTheDocument();
    hud.emit(decodeHudStatusEvent(hudSamplingJson));
    expect(screen.getByRole("status")).toHaveTextContent("Sampling…");
    view.unmount();
    expect(hud.stop).toHaveBeenCalled();
  });

  it("renders Simplified Chinese labels and battery when present", () => {
    const withBattery = {
      ...snapshot,
      power: {
        state: "available",
        sampled_at_unix_ms: 1,
        age_ms: 0,
        value: {
          ac_state: "offline",
          battery_present: true,
          charge_basis_points: 8_700,
          remaining_seconds: 5_400,
        },
      },
    } as StatusSnapshotV1;
    const rows = hudRows("zh-CN", withBattery);
    expect(rows.find((row) => row.key === "temperature")?.value).toBe("不可用");
    expect(rows.find((row) => row.key === "battery")).toMatchObject({
      label: "电池",
    });
    expect(rows.find((row) => row.key === "battery")?.value).toContain(
      "87.00%",
    );
    expect(rows.find((row) => row.key === "battery")?.value).toContain("90");
  });

  it("never shows a zero for a missing GPU value", () => {
    const missing = {
      ...snapshot,
      gpu: {
        state: "unavailable",
        sampled_at_unix_ms: null,
        reason_code: "counter_missing",
      },
    } as StatusSnapshotV1;
    expect(hudRows("en", missing).find((row) => row.key === "gpu")?.value).toBe(
      "Unavailable",
    );
  });

  it("uses no bridge commands and no Tauri import in the HUD entry", () => {
    for (const file of ["src/hud/main.tsx", "src/hud/Hud.tsx"]) {
      const source = readFileSync(resolve(process.cwd(), file), "utf8");
      expect(source).not.toMatch(/@tauri-apps|\btauriBridge\b|invoke\(/);
    }
  });
});
