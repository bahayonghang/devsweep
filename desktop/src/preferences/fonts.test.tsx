import { StrictMode, type ReactNode } from "react";
import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DesktopFontsBridge } from "../api/bridge";
import { decodeDesktopFont, decodeDesktopFonts } from "../api/contract";
import type { DesktopFonts } from "../api/types.gen";
import { applyAppearance, fontStack, SYSTEM_FONT_STACK } from "./appearance";
import { DesktopFontsContext, useFontCatalogue } from "./fonts";
import { FontPicker } from "./FontPicker";
import { DEFAULT_DESKTOP_PREFERENCES } from "./store";
import { PreferencesProvider } from "./PreferencesProvider";
import { createFixturePreferencesBridge } from "./fixture";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const available = (family = "Host Family"): DesktopFonts => ({ status: "available", families: [{ family, names: [{ locale: "en-US", name: family }, { locale: "zh-CN", name: "本机字体" }] }] });
const wrapper = (bridge: DesktopFontsBridge) => ({ children }: { children: ReactNode }) => <DesktopFontsContext.Provider value={bridge}>{children}</DesktopFontsContext.Provider>;

describe("font contract and CSS boundary", () => {
  it("accepts Unicode scalar families and rejects invalid font shapes", () => {
    for (const family of ["中文字体", String.fromCharCode(34, 92), "😀".repeat(256), "\ufeffFamily"]) {
      expect(decodeDesktopFont({ kind: "installed", family })).toEqual({ kind: "installed", family });
    }
    for (const value of [
      { kind: "system", family: "extra" }, { kind: "installed" }, { kind: "download", family: "X" },
      ...["", " leading", "trailing ", "a\nb", "a\u0085b", "\ud800", "😀".repeat(257)].map((family) => ({ kind: "installed", family })),
    ]) expect(() => decodeDesktopFont(value)).toThrow();
  });

  it("decodes closed outcomes without paths, bytes, or duplicate families", () => {
    expect(decodeDesktopFonts(available())).toEqual(available());
    expect(decodeDesktopFonts({ status: "available", families: [] })).toEqual({ status: "available", families: [] });
    expect(decodeDesktopFonts({ status: "unavailable", reason: "unsupported_platform" }).status).toBe("unavailable");
    for (const value of [
      { status: "available", families: [], path: "x" }, { status: "unavailable", reason: "unknown" },
      { status: "available", families: [{ family: "A", names: [], bytes: [] }] },
      { status: "available", families: [{ family: "A", names: [] }, { family: "a", names: [] }] },
      { status: "available", families: [{ family: "A", names: [{ locale: "en", name: "A", path: "x" }] }] },
    ]) expect(() => decodeDesktopFonts(value)).toThrow();
  });

  it("quotes one CSS family and applies only appearance properties to main and HUD", () => {
    const family = "中文" + String.fromCharCode(34, 92);
    expect(fontStack({ kind: "installed", family })).toBe('"中文' + String.fromCharCode(92, 34, 92, 92, 34) + ", " + SYSTEM_FONT_STACK);
    expect(fontStack({ kind: "system" })).toBe(SYSTEM_FONT_STACK);
    for (const root of [document.createElement("main"), document.createElement("aside")]) {
      const font = { kind: "installed" as const, family: 'Missing"; color: red; --other: url(x); \\ 中文' };
      applyAppearance(root, { ...DEFAULT_DESKTOP_PREFERENCES, font }, false, false);
      expect(root.style.getPropertyValue("--font-ui")).toBe(fontStack(font));
      expect(root.style.getPropertyValue("color")).toBe("");
      expect(root.style.getPropertyValue("--other")).toBe("");
    }
  });
});

describe("session font catalogue", () => {
  it("does not discover fonts when main or HUD only consumes preferences", async () => {
    const fonts = { list: vi.fn<DesktopFontsBridge["list"]>().mockResolvedValue(available()) };
    const preferences = createFixturePreferencesBridge();
    render(<PreferencesProvider bridge={preferences} fonts={fonts}><span>Appearance consumer</span></PreferencesProvider>);
    await act(async () => {});
    expect(fonts.list).not.toHaveBeenCalled();
  });
  it("coalesces StrictMode loads and caches success across Settings remounts", async () => {
    const pending = deferred<DesktopFonts>();
    const bridge = { list: vi.fn(() => pending.promise) };
    const strict = ({ children }: { children: ReactNode }) => <StrictMode><DesktopFontsContext.Provider value={bridge}>{children}</DesktopFontsContext.Provider></StrictMode>;
    const view = renderHook(useFontCatalogue, { wrapper: strict });
    expect(bridge.list).toHaveBeenCalledTimes(1);
    await act(async () => { pending.resolve(available()); });
    expect(view.result.current.families?.[0].family).toBe("Host Family");
    view.unmount();
    const next = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    expect(next.result.current.families?.[0].family).toBe("Host Family");
    expect(bridge.list).toHaveBeenCalledTimes(1);
  });

  it("coalesces refresh, retains choices after failure, and permits retry", async () => {
    const refresh = deferred<DesktopFonts>();
    const bridge = { list: vi.fn<DesktopFontsBridge["list"]>().mockResolvedValueOnce(available()).mockReturnValueOnce(refresh.promise).mockResolvedValueOnce(available("New Family")) };
    const view = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    await waitFor(() => expect(view.result.current.families).not.toBeNull());
    act(() => { void view.result.current.refresh(); void view.result.current.refresh(); });
    expect(bridge.list).toHaveBeenCalledTimes(2);
    await act(async () => { refresh.reject(new Error("fixture failure")); });
    expect(view.result.current.failure).toBe("enumeration_failed");
    expect(view.result.current.families?.[0].family).toBe("Host Family");
    await act(async () => { await view.result.current.refresh(); });
    expect(view.result.current.families?.[0].family).toBe("New Family");
  });

  it("discards disposed results and isolates request and bridge generations", async () => {
    const old = deferred<DesktopFonts>();
    const current = deferred<DesktopFonts>();
    const bridge = { list: vi.fn<DesktopFontsBridge["list"]>().mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise) };
    const view = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    view.unmount();
    await act(async () => {});
    const next = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    await act(async () => { current.resolve(available("Current")); old.resolve(available("Stale")); });
    expect(next.result.current.families?.[0].family).toBe("Current");
    expect(bridge.list).toHaveBeenCalledTimes(2);
    const other = renderHook(useFontCatalogue, { wrapper: wrapper({ list: async () => available("Other Bridge") }) });
    await waitFor(() => expect(other.result.current.families?.[0].family).toBe("Other Bridge"));
  });

  it("restarts after settlement and disposal occur in the same turn", async () => {
    const old = deferred<DesktopFonts>();
    const bridge = { list: vi.fn<DesktopFontsBridge["list"]>().mockReturnValueOnce(old.promise).mockResolvedValueOnce(available("Fresh")) };
    const view = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    await act(async () => { old.resolve(available("Disposed")); view.unmount(); });
    const next = renderHook(useFontCatalogue, { wrapper: wrapper(bridge) });
    await waitFor(() => expect(next.result.current.families?.[0].family).toBe("Fresh"));
    expect(bridge.list).toHaveBeenCalledTimes(2);
    expect(next.result.current.loading).toBe(false);
  });
});

describe("installed font picker", () => {
  it("keeps system usable during loading and distinguishes empty success", async () => {
    const request = deferred<DesktopFonts>();
    const bridge = { list: vi.fn(() => request.promise) };
    const user = userEvent.setup();
    const change = vi.fn();
    render(<FontPicker locale="en" font={{ kind: "installed", family: "Absent Family" }} disabled={false} onChange={change} />, { wrapper: wrapper(bridge) });
    expect(screen.getByRole("combobox")).toHaveValue("Absent Family");
    expect(screen.getByText("Loading installed fonts…")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Font family" }));
    await user.click(await screen.findByRole("option", { name: "System UI" }));
    expect(change).toHaveBeenCalledExactlyOnceWith({ kind: "system" });
    await act(async () => { request.resolve({ status: "available", families: [] }); });
    expect(screen.getByText(/No installed font families/)).toBeVisible();
    expect(screen.getByText(/The saved font/)).toHaveTextContent("Absent Family");
    expect(screen.getByRole("combobox")).toHaveValue("Absent Family");
  });

  it("keeps saved names after errors and selects canonical names on Refresh", async () => {
    const bridge = { list: vi.fn<DesktopFontsBridge["list"]>().mockResolvedValueOnce({ status: "unavailable", reason: "enumeration_failed" }).mockResolvedValueOnce(available("system:")) };
    const user = userEvent.setup();
    const change = vi.fn();
    render(<FontPicker locale="zh-CN" font={{ kind: "installed", family: "Unavailable 中文" }} disabled={false} onChange={change} />, { wrapper: wrapper(bridge) });
    await screen.findByText(/字体目录读取失败/);
    expect(screen.getByRole("combobox")).toHaveValue("Unavailable 中文");
    await user.click(screen.getByRole("button", { name: "刷新字体" }));
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "本机" } });
    await user.click(await screen.findByRole("option", { name: "本机字体 (system:)" }));
    expect(change).toHaveBeenCalledExactlyOnceWith({ kind: "installed", family: "system:" });
  });

  it("reaches all 2000 families and aliases without per-key enumeration", async () => {
    const families = Array.from({ length: 2000 }, (_, index) => ({ family: "Family " + String(index).padStart(4, "0"), names: [{ locale: "zh-CN", name: "中文字体 " + index }] }));
    const bridge = { list: vi.fn<DesktopFontsBridge["list"]>().mockResolvedValue({ status: "available", families }) };
    const change = vi.fn();
    const user = userEvent.setup();
    render(<FontPicker locale="en" font={{ kind: "system" }} disabled={false} onChange={change} />, { wrapper: wrapper(bridge) });
    await waitFor(() => expect(screen.getByRole("button", { name: "Refresh fonts" })).toBeEnabled());
    const started = performance.now();
    await user.click(screen.getByRole("button", { name: "Font family" }));
    expect(await screen.findAllByRole("option")).toHaveLength(2001);
    const openMs = performance.now() - started;
    const queryMs: number[] = [];
    for (const [query, family] of [["Family 0000", "Family 0000"], ["family 1000", "Family 1000"], ["中文字体 1999", "Family 1999"]]) {
      const start = performance.now();
      fireEvent.change(screen.getByRole("combobox"), { target: { value: query } });
      expect(await screen.findByRole("option", { name: family })).toBeVisible();
      queryMs.push(performance.now() - start);
    }
    expect(change).not.toHaveBeenCalled();
    await user.click(screen.getByRole("option", { name: "Family 1999" }));
    expect(change).toHaveBeenCalledExactlyOnceWith({ kind: "installed", family: "Family 1999" });
    expect(bridge.list).toHaveBeenCalledTimes(1);
    console.info(JSON.stringify({ method: "Vitest/jsdom; rendering and accessible queries; diagnostic only, not native latency", host: { platform: process.platform, arch: process.arch, node: process.version }, families: 2000, openMs, queryMs }));
  }, 30000);
});
