import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import type { DesktopBridge } from "../../api/bridge";
import { decodeStatusEvent } from "../../api/contract";
import startedJson from "../../api/fixtures/status/event-started.json";
import type { DesktopPreferencesSnapshot, DesktopStatusLiveResult } from "../../api/types.gen";
import { createFixturePreferencesBridge } from "../../preferences/fixture";
import { PreferencesProvider } from "../../preferences/PreferencesProvider";
import { usePreferences } from "../../preferences/context";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { StatusWorkbench } from "./StatusWorkbench";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((a, b) => { resolve = a; reject = b; });
  return { resolve, reject, promise };
}
function ReadyStatus({ bridge, coordinator }: { bridge: DesktopBridge; coordinator: OperationCoordinator }) {
  const preferences = usePreferences();
  return preferences.loading ? null : <StatusWorkbench bridge={bridge} coordinator={coordinator} locale="en" />;
}

describe("Status committed preferences", () => {
  it("passes rows to snapshot/live and commits before cancel, join, and replacement", async () => {
    const user = userEvent.setup();
    const preferences = createFixturePreferencesBridge();
    await preferences.update({ field: "status_process_limit", value: 50 });
    await preferences.update({ field: "status_interval_seconds", value: 5 });
    const events: string[] = [];
    const commit = deferred<DesktopPreferencesSnapshot>();
    const baseUpdate = preferences.update;
    preferences.update = vi.fn(async (patch) => { events.push("save"); await commit.promise; const result = await baseUpdate(patch); events.push("committed"); return result; });
    const runs: Array<{ operationId: string; terminal: ReturnType<typeof deferred<DesktopStatusLiveResult>> }> = [];
    const bridge: DesktopBridge = {
      ...fixtureBridge,
      statusSnapshot: vi.fn(fixtureBridge.statusSnapshot),
      statusCancel: vi.fn(async () => { events.push("cancel"); }),
      statusLiveStart: vi.fn((operationId, intervalMs, processLimit, onEvent) => {
        events.push("start " + intervalMs);
        const terminal = deferred<DesktopStatusLiveResult>();
        runs.push({ operationId, terminal });
        onEvent(decodeStatusEvent({ ...startedJson, operation_id: operationId, data: { interval_ms: intervalMs, process_limit: processLimit } }));
        return terminal.promise.then((result) => { events.push("joined"); return result; });
      }),
    };
    const coordinator = new OperationCoordinator();
    const view = render(<PreferencesProvider bridge={preferences}><ReadyStatus bridge={bridge} coordinator={coordinator} /></PreferencesProvider>);
    await user.click(await screen.findByRole("button", { name: "Show details" }));
    expect(bridge.statusSnapshot).toHaveBeenCalledWith(expect.any(String), 50);
    expect(screen.getByRole("combobox", { name: "Live interval" })).toHaveValue("5000");
    await user.click(screen.getByRole("button", { name: "Start live" }));
    expect(bridge.statusLiveStart).toHaveBeenCalledWith(expect.any(String), 5000, 50, expect.any(Function), expect.any(Function));
    await user.selectOptions(screen.getByRole("combobox", { name: "Live interval" }), "10000");
    expect(events).toEqual(["start 5000", "save"]);
    expect(screen.getByRole("combobox", { name: "Live interval" })).toHaveValue("5000");
    await act(async () => commit.resolve(await preferences.get()));
    await waitFor(() => expect(events).toEqual(["start 5000", "save", "committed", "cancel"]));
    expect(runs).toHaveLength(1);
    await act(async () => runs[0].terminal.resolve({ type: "canceled", operation_id: runs[0].operationId }));
    await waitFor(() => expect(runs).toHaveLength(2));
    expect(events).toEqual(["start 5000", "save", "committed", "cancel", "joined", "start 10000"]);
    expect(screen.getByRole("combobox", { name: "Live interval" })).toHaveValue("10000");
    view.unmount();
    runs[1].terminal.resolve({ type: "canceled", operation_id: runs[1].operationId });
    await coordinator.close();
  });

  it("leaves the running sampler and committed selector unchanged when saving fails", async () => {
    const user = userEvent.setup();
    const preferences = createFixturePreferencesBridge();
    preferences.update = vi.fn().mockRejectedValue(new Error("store failed"));
    const bridge = { ...fixtureBridge, statusLiveStart: vi.fn(fixtureBridge.statusLiveStart), statusCancel: vi.fn(fixtureBridge.statusCancel) };
    const coordinator = new OperationCoordinator();
    render(<PreferencesProvider bridge={preferences}><ReadyStatus bridge={bridge} coordinator={coordinator} /></PreferencesProvider>);
    await user.click(await screen.findByRole("button", { name: "Show details" }));
    await user.click(screen.getByRole("button", { name: "Start live" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Live interval" }), "10000");
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save");
    expect(screen.getByRole("combobox", { name: "Live interval" })).toHaveValue("2000");
    expect(bridge.statusCancel).not.toHaveBeenCalled();
    expect(bridge.statusLiveStart).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Stop live" }));
    await coordinator.close();
  });
});
