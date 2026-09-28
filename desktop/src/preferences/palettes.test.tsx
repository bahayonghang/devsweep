import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { decodeDesktopPreferencesSnapshot } from "../api/contract";
import snapshotsJson from "../api/fixtures/desktop-preferences-themes.json";
import patches from "../api/fixtures/desktop-preferences-patches.json";
import type { DesktopPreferencesSnapshot, DesktopTheme } from "../api/types.gen";
import { message } from "../i18n";
import { Hud } from "../hud/Hud";
import { applyAppearance, THEME_CATALOGUE } from "./appearance";
import { createFixturePreferencesBridge, fixtureFontsBridge } from "./fixture";
import { PreferencesProvider } from "./PreferencesProvider";
import { SettingsPage } from "./SettingsPage";

const themes = Object.values(THEME_CATALOGUE);
const snapshots = snapshotsJson.map(decodeDesktopPreferencesSnapshot);
const defaults = snapshots[0].preferences;
const named = themes.filter((theme) => !["dark", "light", "system"].includes(theme.id));
afterEach(() => { vi.unstubAllGlobals(); });

describe("closed shared palette catalogue", () => {
  it("matches all seven snapshot and patch wire variants", () => {
    expect(themes.map((theme) => theme.id)).toEqual(["dark", "light", "system", "catppuccin_latte", "catppuccin_mocha", "codex", "claude"]);
    expect(snapshots.map((snapshot) => snapshot.preferences.theme)).toEqual(themes.map((theme) => theme.id));
    expect(new Set(patches.filter((patch) => patch.field === "theme").map((patch) => patch.value))).toEqual(new Set(themes.map((theme) => theme.id)));
    for (const theme of ["latte", "mocha", "catppuccin_frappe", "claude_dark", "CODEX", null]) {
      expect(() => decodeDesktopPreferencesSnapshot({ ...snapshots[0], preferences: { ...defaults, theme } })).toThrow();
    }
  });

  it("applies exact palettes with only light/dark schemes in main and HUD roots", () => {
    const roots = [document.createElement("main"), document.createElement("aside")];
    for (const snapshot of snapshots) for (const systemLight of [false, true]) for (const scale of [100, 110, 125]) {
      const preferences = { ...snapshot.preferences, text_scale_percent: scale, motion: "reduced" as const };
      const before = structuredClone(preferences);
      const resolved = preferences.theme === "system" ? (systemLight ? "light" : "dark") : preferences.theme;
      for (const root of roots) {
        applyAppearance(root, preferences, systemLight, false);
        expect(root.dataset.theme).toBe(resolved);
        expect(root.style.colorScheme).toBe(THEME_CATALOGUE[resolved].scheme);
        expect(["dark", "light"]).toContain(root.style.colorScheme);
        expect(root.dataset.motion).toBe("reduced");
        expect(root.style.getPropertyValue("--text-scale")).toBe(String(scale / 100));
      }
      expect(preferences).toEqual(before);
    }
  });

  it.each(["en", "zh-CN"] as const)("offers seven labeled, commit-only radio previews in %s", async (locale) => {
    const user = userEvent.setup();
    const bridge = createFixturePreferencesBridge();
    const save = vi.spyOn(bridge, "update");
    const view = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale={locale} localeSaving={false} onLocaleChange={vi.fn()} /></PreferencesProvider>);
    await waitFor(() => expect(screen.getAllByRole("radio")[0]).toBeEnabled());
    expect(screen.getAllByRole("radio")).toHaveLength(7);
    expect(screen.getAllByRole("radio", { checked: true })).toHaveLength(1);
    for (const theme of themes) {
      const radio = screen.getByRole("radio", { name: message(locale, theme.label) });
      expect(radio).toHaveAttribute("value", theme.id);
      expect(radio.parentElement!.querySelectorAll("[data-palette-preview]")).toHaveLength(theme.preview.length);
      const calls = save.mock.calls.length;
      await user.hover(radio.parentElement!);
      radio.focus();
      expect(save).toHaveBeenCalledTimes(calls);
      if (theme.id === "dark") continue;
      await user.click(radio);
      await waitFor(() => expect(radio).toBeChecked());
      expect(save).toHaveBeenLastCalledWith({ field: "theme", value: theme.id });
      expect(screen.getAllByRole("radio", { checked: true })).toHaveLength(1);
    }
    expect(view.container.querySelectorAll("[data-palette-preview]")).toHaveLength(8);
    await user.click(screen.getByRole("button", { name: message(locale, "preferences.v1.reset.appearance") }));
    await waitFor(() => expect(screen.getByRole("radio", { name: message(locale, "preferences.v1.theme.dark") })).toBeChecked());
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it.each(named)("keeps the committed palette when saving $id fails", async (theme) => {
    const user = userEvent.setup();
    let reject!: (error: Error) => void;
    const pending = new Promise<DesktopPreferencesSnapshot>((_, fail) => { reject = fail; });
    const bridge = createFixturePreferencesBridge();
    bridge.update = vi.fn(() => pending);
    render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><SettingsPage locale="en" localeSaving={false} onLocaleChange={vi.fn()} /></PreferencesProvider>);
    const radio = screen.getByRole("radio", { name: message("en", theme.label) });
    await waitFor(() => expect(radio).toBeEnabled());
    await user.click(radio);
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    await act(async () => reject(new Error("write failed")));
    expect(radio).not.toBeChecked();
    expect(radio).toBeEnabled();
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(screen.getByRole("alert")).toHaveTextContent("Could not save");
  });

  it.each(themes)("loads the saved $id snapshot into HUD after a main-window commit", async (theme) => {
    const bridge = createFixturePreferencesBridge();
    const main = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><span>Main</span></PreferencesProvider>);
    await act(async () => { await bridge.update({ field: "theme", value: theme.id }); });
    const committed = (await bridge.get()).preferences;
    const mainTheme = document.documentElement.dataset.theme;
    const mainScheme = document.documentElement.style.colorScheme;
    main.unmount();
    document.documentElement.dataset.theme = "stale";
    const hud = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><Hud locale="en" subscribe={async () => () => undefined} /></PreferencesProvider>);
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe(mainTheme));
    expect(document.documentElement.style.colorScheme).toBe(mainScheme);
    const nextTheme: DesktopTheme = theme.id === "claude" ? "codex" : "claude";
    await act(async () => { await bridge.update({ field: "theme", value: nextTheme }); });
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe(nextTheme));
    expect((await bridge.get()).preferences).toEqual({ ...committed, theme: nextTheme });
    hud.unmount();
  });

  it("listens for OS scheme changes only in System and preserves reduced motion", async () => {
    let light = false;
    const listeners = new Set<() => void>();
    vi.stubGlobal("matchMedia", (query: string) => ({
      get matches() { return query.includes("color-scheme") ? light : true; },
      addEventListener: (_: string, callback: () => void) => { if (query.includes("color-scheme")) listeners.add(callback); },
      removeEventListener: (_: string, callback: () => void) => listeners.delete(callback),
    }));
    const bridge = createFixturePreferencesBridge();
    const view = render(<PreferencesProvider bridge={bridge} fonts={fixtureFontsBridge}><span>Main</span></PreferencesProvider>);
    for (const theme of named) {
      await act(async () => { await bridge.update({ field: "theme", value: theme.id }); });
      expect(listeners.size).toBe(0);
      act(() => { light = !light; listeners.forEach((callback) => callback()); });
      expect(document.documentElement.dataset.theme).toBe(theme.id);
      expect(document.documentElement.dataset.motion).toBe("reduced");
    }
    await act(async () => { await bridge.update({ field: "theme", value: "system" }); });
    expect(listeners.size).toBe(1);
    for (const next of [true, false]) {
      act(() => { light = next; listeners.forEach((callback) => callback()); });
      expect(document.documentElement.dataset.theme).toBe(next ? "light" : "dark");
      expect(document.documentElement.style.colorScheme).toBe(next ? "light" : "dark");
    }
    view.unmount();
    expect(listeners.size).toBe(0);
  });
});
