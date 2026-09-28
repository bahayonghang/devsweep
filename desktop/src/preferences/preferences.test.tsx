import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopPreferencesBridge } from "../api/bridge";
import { decodeDesktopPreferencesSnapshot } from "../api/contract";
import defaultsJson from "../api/fixtures/desktop-preferences.json";
import updatedJson from "../api/fixtures/desktop-preferences-updated.json";
import type { DesktopPreferencesSnapshot } from "../api/types.gen";
import { applyAppearance, fontStack } from "./appearance";
import { createFixturePreferencesBridge, fixtureFontsBridge } from "./fixture";
import { PreferencesProvider } from "./PreferencesProvider";
import { SettingsPage } from "./SettingsPage";
import { useDesktopPreferences } from "./store";

const defaults = decodeDesktopPreferencesSnapshot(defaultsJson);
const updated = decodeDesktopPreferencesSnapshot(updatedJson);
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((a, b) => { resolve = a; reject = b; });
  return { promise, resolve, reject };
}
afterEach(() => { vi.unstubAllGlobals(); });

describe("desktop preference boundary", () => {
  it("decodes snapshots and rejects unknown fields, all unsupported values and unsafe order", () => {
    expect(decodeDesktopPreferencesSnapshot(defaultsJson)).toEqual(defaultsJson);
    expect(decodeDesktopPreferencesSnapshot(updatedJson)).toEqual(updatedJson);
    for (const patch of [{ schema_version: 3 }, { theme: "auto" }, { font_family: "url(remote)" }, { text_scale_percent: 101 }, { motion: "off" }, { planet_fps: 60 }, { status_interval_seconds: 3 }, { status_process_limit: 16 }, { hud_interval_seconds: 1 }, { extra: true }]) {
      expect(() => decodeDesktopPreferencesSnapshot({ ...defaults, preferences: { ...defaults.preferences, ...patch } })).toThrow();
    }
    for (const sequence of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1, "1"]) expect(() => decodeDesktopPreferencesSnapshot({ ...defaults, sequence })).toThrow();
    expect(() => decodeDesktopPreferencesSnapshot({ ...defaults, extra: true })).toThrow();
    expect(() => decodeDesktopPreferencesSnapshot({ sequence: 1, preferences: { theme: "dark" } })).toThrow();
  });
});

describe("committed desktop preference owner", () => {
  it("subscribes before get and keeps a new event over a late initial read", async () => {
    const initial = deferred<DesktopPreferencesSnapshot>();
    const events: string[] = [];
    let notify!: (value: DesktopPreferencesSnapshot) => void;
    const remove = vi.fn();
    const bridge: DesktopPreferencesBridge = { get: () => { events.push("get"); return initial.promise; }, update: vi.fn(), subscribe: async (listener) => { events.push("subscribe"); notify = listener; return remove; } };
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(events).toEqual(["subscribe", "get"]));
    act(() => notify(updated));
    await act(async () => initial.resolve(defaults));
    expect(hook.result.current.preferences.theme).toBe("light");
    act(() => notify({ ...defaults, sequence: 1 }));
    expect(hook.result.current.preferences.theme).toBe("light");
    hook.unmount();
    expect(remove).toHaveBeenCalledTimes(1);
  });

  it("removes a late subscription and never reads after disposal", async () => {
    const subscribed = deferred<() => void>();
    const remove = vi.fn();
    const bridge: DesktopPreferencesBridge = { ...createFixturePreferencesBridge(), subscribe: () => subscribed.promise, get: vi.fn() };
    const hook = renderHook(() => useDesktopPreferences(bridge));
    hook.unmount();
    await act(async () => subscribed.resolve(remove));
    expect(remove).toHaveBeenCalledTimes(1);
    expect(bridge.get).not.toHaveBeenCalled();
  });

  it("keeps the committed sequence while reloading and rejects an older event", async () => {
    const reloaded = deferred<DesktopPreferencesSnapshot>();
    let notify!: (value: DesktopPreferencesSnapshot) => void;
    const bridge: DesktopPreferencesBridge = {
      get: vi.fn().mockResolvedValueOnce({ ...updated, sequence: 5 }).mockReturnValueOnce(reloaded.promise),
      update: vi.fn(),
      subscribe: async (listener) => { notify = listener; return () => undefined; },
    };
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(hook.result.current.preferences.theme).toBe("light"));
    act(() => hook.result.current.reload());
    await waitFor(() => expect(bridge.get).toHaveBeenCalledTimes(2));
    act(() => notify(defaults));
    expect(hook.result.current.preferences.theme).toBe("light");
    await act(async () => reloaded.resolve({ ...updated, sequence: 6 }));
    expect(hook.result.current.preferences.theme).toBe("light");
    expect(hook.result.current.loading).toBe(false);
  });

  it("ignores an initial read after disposal and reloads on a visible-window notification", async () => {
    const initial = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    bridge.get = vi.fn().mockReturnValueOnce(initial.promise).mockResolvedValue(updated);
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    await waitFor(() => expect(hook.result.current.preferences.theme).toBe("light"));
    hook.unmount();
    const disposed = hook.result.current;
    await act(async () => initial.resolve(defaults));
    expect(hook.result.current).toBe(disposed);
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    expect(bridge.get).toHaveBeenCalledTimes(2);
  });

  it("keeps validated defaults on load failure and disables persistence until reload succeeds", async () => {
    const bridge = createFixturePreferencesBridge();
    bridge.get = vi.fn().mockRejectedValueOnce(new Error("unreadable")).mockResolvedValue(defaults);
    const write = vi.spyOn(bridge, "update");
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(hook.result.current.unavailable).toBe(true));
    expect(hook.result.current.preferences).toEqual(defaults.preferences);
    await act(async () => { await hook.result.current.update({ field: "theme", value: "light" }); });
    expect(write).not.toHaveBeenCalled();
    act(() => hook.result.current.reload());
    await waitFor(() => expect(hook.result.current.unavailable).toBe(false));
    await act(async () => { await hook.result.current.update({ field: "theme", value: "light" }); });
    expect(hook.result.current.preferences.theme).toBe("light");
  });

  it("holds the committed value during save and after failure, then permits retry", async () => {
    const write = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    bridge.update = vi.fn().mockReturnValueOnce(write.promise).mockResolvedValueOnce(updated);
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    let saving!: Promise<DesktopPreferencesSnapshot | null>;
    act(() => { saving = hook.result.current.update({ field: "theme", value: "light" }); });
    expect(hook.result.current.saving).toBe(true);
    expect(hook.result.current.preferences.theme).toBe("dark");
    await act(async () => { write.reject(new Error("write failed")); await saving; });
    expect(hook.result.current.saveError).toBe(true);
    expect(hook.result.current.preferences.theme).toBe("dark");
    await act(async () => { await hook.result.current.update({ field: "theme", value: "light" }); });
    expect(hook.result.current.preferences.theme).toBe("light");
    expect(hook.result.current.saveError).toBe(false);
  });

  it("uses the newest committed value when an older update response arrives and ignores disposed loads", async () => {
    const write = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    let notify!: (value: DesktopPreferencesSnapshot) => void;
    bridge.subscribe = async (listener) => { notify = listener; return () => undefined; };
    bridge.update = () => write.promise;
    const hook = renderHook(() => useDesktopPreferences(bridge));
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    let result!: Promise<DesktopPreferencesSnapshot | null>;
    act(() => { result = hook.result.current.update({ field: "theme", value: "light" }); });
    act(() => notify({ ...updated, sequence: 3 }));
    await act(async () => write.resolve({ ...defaults, sequence: 2 }));
    expect(await result).toEqual({ ...updated, sequence: 3 });
    const before = hook.result.current;
    hook.unmount();
    act(() => notify({ ...defaults, sequence: 4 }));
    expect(hook.result.current).toBe(before);
  });

  it("resets each group in one patch without changing the other group or language", async () => {
    const user = userEvent.setup();
    const bridge = createFixturePreferencesBridge();
    await bridge.update({ field: "theme", value: "light" });
    await bridge.update({ field: "status_process_limit", value: 50 });
    const writes = vi.spyOn(bridge, "update");
    const language = vi.fn();
    render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={false} onLocaleChange={language} /></PreferencesProvider>);
    await waitFor(() => expect(screen.getByRole("radio", { name: "Light" })).toBeChecked());
    await user.click(screen.getByRole("button", { name: "Reset appearance" }));
    expect(writes).toHaveBeenLastCalledWith({ field: "reset_appearance" });
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(screen.getByRole("combobox", { name: "Status process rows" })).toHaveTextContent("50 rows");
    await user.click(screen.getByRole("button", { name: "Reset performance" }));
    expect(writes).toHaveBeenLastCalledWith({ field: "reset_performance" });
    expect(screen.getByRole("combobox", { name: "Status process rows" })).toHaveTextContent("15 rows");
    expect(language).not.toHaveBeenCalled();
  });
});

describe("Settings committed controls", () => {
  it("retains the selected radio during save and failure, then permits retry", async () => {
    const user = userEvent.setup();
    const saving = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    bridge.update = vi.fn().mockReturnValueOnce(saving.promise).mockResolvedValueOnce(updated);
    render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={false} onLocaleChange={vi.fn()} /></PreferencesProvider>);
    const light = screen.getByRole("radio", { name: "Light" });
    await waitFor(() => expect(light).toBeEnabled());
    await user.hover(light.parentElement!);
    light.focus();
    expect(bridge.update).not.toHaveBeenCalled();
    await user.click(light);
    expect(bridge.update).toHaveBeenCalledExactlyOnceWith({ field: "theme", value: "light" });
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(light).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Language" })).toBeEnabled();
    await act(async () => saving.reject(new Error("failed")));
    expect(screen.getByRole("alert")).toHaveTextContent("Could not save");
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    await user.click(light);
    await waitFor(() => expect(light).toBeChecked());
    await user.click(light);
    expect(bridge.update).toHaveBeenCalledTimes(2);
  });

  it("keeps the saved font visible during a failed save", async () => {
    const user = userEvent.setup();
    const saving = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    bridge.update = vi.fn(() => saving.promise);
    render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={false} onLocaleChange={vi.fn()} /></PreferencesProvider>);
    const input = screen.getByRole("combobox", { name: "Font family" });
    await waitFor(() => expect(input).toBeEnabled());
    await user.click(input);
    await user.click(screen.getByRole("option", { name: "Segoe UI" }));
    expect(bridge.update).toHaveBeenCalledExactlyOnceWith({ field: "font", value: { kind: "installed", family: "Segoe UI" } });
    expect(input).toHaveValue("System UI");
    expect(input).toBeDisabled();
    await act(async () => saving.reject(new Error("failed")));
    expect(input).toHaveValue("System UI");
    expect(input).toBeEnabled();
  });

  it("disables desktop persistence during load or unavailability while language stays independent", async () => {
    const user = userEvent.setup();
    const initial = deferred<DesktopPreferencesSnapshot>();
    const bridge = createFixturePreferencesBridge();
    bridge.get = () => initial.promise;
    const language = vi.fn();
    render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={false} onLocaleChange={language} /></PreferencesProvider>);
    expect(screen.getByRole("combobox", { name: "Text size" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Language" })).toBeEnabled();
    await act(async () => initial.reject(new Error("unavailable")));
    expect(screen.getByRole("radio", { name: "Dark" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Reset performance" })).toBeDisabled();
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));
    expect(language).toHaveBeenCalledExactlyOnceWith("zh-CN");
  });

  it("disables the frame limit for reduced motion and exposes the reason", async () => {
    const bridge = createFixturePreferencesBridge();
    await bridge.update({ field: "motion", value: "reduced" });
    const view = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={true} onLocaleChange={vi.fn()} /></PreferencesProvider>);
    const fps = screen.getByRole("combobox", { name: "Planet frame limit" });
    await waitFor(() => expect(fps).toHaveAccessibleDescription("Animation is off while reduced motion is active."));
    expect(fps).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Language" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Text size" })).toBeEnabled();
    expect(view.container.querySelector("select")).toBeNull();
    expect(screen.getAllByRole("group", { name: "Theme" })).toHaveLength(1);
  });
});

describe("shared main and HUD appearance", () => {
  it("applies every local font and real scale token to either document root", () => {
    const main = document.createElement("div");
    const hud = document.createElement("div");
    for (const font of [{ kind: "system" }, { kind: "installed", family: "Segoe UI" }, { kind: "installed", family: "中文字体" }] as const) {
      for (const text_scale_percent of [100, 110, 125]) {
        for (const root of [main, hud]) {
          applyAppearance(root, { ...defaults.preferences, theme: "system", font, text_scale_percent }, true, true);
          expect(root.dataset.theme).toBe("light");
          expect(root.dataset.motion).toBe("reduced");
          expect(root.style.getPropertyValue("--font-ui")).toBe(fontStack(font));
          expect(root.style.getPropertyValue("--text-scale")).toBe(String(text_scale_percent / 100));
        }
      }
    }
  });

  it("follows OS theme only in system mode and removes media listeners on unmount", async () => {
    const listeners = new Map<string, Set<() => void>>();
    let light = false;
    vi.stubGlobal("matchMedia", (query: string) => {
      const handlers = listeners.get(query) ?? new Set();
      listeners.set(query, handlers);
      return { get matches() { return query.includes("color-scheme") && light; }, addEventListener: (_: string, listener: () => void) => handlers.add(listener), removeEventListener: (_: string, listener: () => void) => handlers.delete(listener) };
    });
    const bridge = createFixturePreferencesBridge();
    await bridge.update({ field: "theme", value: "system" });
    const view = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><span>Content</span></PreferencesProvider>);
    await waitFor(() => expect(listeners.get("(prefers-color-scheme: light)")?.size).toBe(1));
    expect(document.documentElement.dataset.theme).toBe("dark");
    act(() => { light = true; listeners.get("(prefers-color-scheme: light)")?.forEach((listener) => listener()); });
    expect(document.documentElement.dataset.theme).toBe("light");
    await act(async () => { await bridge.update({ field: "theme", value: "dark" }); });
    expect(listeners.get("(prefers-color-scheme: light)")?.size).toBe(0);
    expect(document.documentElement.dataset.theme).toBe("dark");
    view.unmount();
    expect([...listeners.values()].every((set) => set.size === 0)).toBe(true);
  });
});
