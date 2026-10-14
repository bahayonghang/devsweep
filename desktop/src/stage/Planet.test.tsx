import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MODE_IDS } from "../app-shell/AppShell";
import { Planet } from "./Planet";
import { DesktopPreferencesContext } from "../preferences/context";
import { DEFAULT_DESKTOP_PREFERENCES } from "../preferences/store";
import { PLANET_PALETTES } from "./planet-palettes";
import { loadEarthTexture } from "./earth-texture";
import { createEarthRenderer } from "./earth-renderer";
import * as lifecycle from "../lifecycle";
vi.mock("./earth-renderer", () => ({ createEarthRenderer: vi.fn() }));
vi.mock("./earth-texture", async (original) => ({
  ...await original<typeof import("./earth-texture")>(),
  loadEarthTexture: vi.fn(),
}));
import {
  MAX_SOURCE_DIAMETER,
  buildPlanetGeometry,
  buildPlanetTexture,
  renderPlanetFrame,
} from "./planet-renderer";

const stageStyles = readFileSync(
  resolve(process.cwd(), "src/stage/styles.css"),
  "utf8",
);

interface FrameQueue {
  readonly callbacks: Map<number, FrameRequestCallback>;
  run(now: number): void;
}

let putImageData: ReturnType<typeof vi.fn>;
let frames: FrameQueue;
let requestFrame: ReturnType<typeof vi.fn>;
let cancelFrame: ReturnType<typeof vi.fn>;
let hidden = false;
let clock = 0;
let media: Record<string, boolean> = {};
const originalGetContext = HTMLCanvasElement.prototype.getContext;

function installMocks() {
  putImageData = vi.fn();
  const context = {
    createImageData: (width: number, height: number) => ({
      width,
      height,
      data: new Uint8ClampedArray(width * height * 4),
    }),
    putImageData,
    clearRect: vi.fn(),
    drawImage: vi.fn(),
    imageSmoothingEnabled: false,
  };
  HTMLCanvasElement.prototype.getContext = function getContext() {
    return context;
  } as unknown as typeof HTMLCanvasElement.prototype.getContext;

  const callbacks = new Map<number, FrameRequestCallback>();
  let nextId = 1;
  requestFrame = vi.fn((callback: FrameRequestCallback) => {
    const id = nextId;
    nextId += 1;
    callbacks.set(id, callback);
    return id;
  });
  cancelFrame = vi.fn((id: number) => {
    callbacks.delete(id);
  });
  frames = {
    callbacks,
    run(now: number) {
      clock = now;
      const pending = [...callbacks.entries()];
      callbacks.clear();
      for (const [, callback] of pending) callback(now);
    },
  };
  vi.stubGlobal("requestAnimationFrame", requestFrame);
  vi.stubGlobal("cancelAnimationFrame", cancelFrame);
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: media[query] ?? false,
    media: query,
  }));
  Object.defineProperty(document, "hidden", {
    configurable: true,
    get: () => hidden,
  });
}

function setHidden(next: boolean) {
  hidden = next;
  act(() => {
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

beforeEach(() => {
  hidden = false;
  media = {};
  clock = 0;
  vi.spyOn(performance, "now").mockImplementation(() => clock);
  vi.mocked(loadEarthTexture).mockReset().mockResolvedValue({ width: 8, height: 4, rgb: new Uint8ClampedArray(8 * 4 * 3).fill(128) });
  installMocks();
  vi.mocked(createEarthRenderer).mockReset().mockImplementation(() => ({ draw: putImageData, dispose: vi.fn() }));
});

afterEach(() => {
  HTMLCanvasElement.prototype.getContext = originalGetContext;
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, "hidden");
});

describe("Planet", () => {
  it("suspends on native minimize even when document.hidden stays false, without resetting its renderer", async () => {
    let visible: (value: boolean) => void = () => undefined;
    const release = vi.fn();
    vi.spyOn(lifecycle, "observeWindowPresentation").mockImplementation((handler) => {
      visible = handler;
      handler(false);
      return release;
    });
    const view = render(<Planet mode="clean" />);
    expect(document.hidden).toBe(false);
    expect(loadEarthTexture).not.toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    await act(async () => visible(true));
    expect(createEarthRenderer).toHaveBeenCalledOnce();
    frames.run(100);
    act(() => visible(false));
    const drawn = putImageData.mock.calls.length;
    expect(frames.callbacks.size).toBe(0);
    frames.run(5000);
    window.dispatchEvent(new Event("resize"));
    expect(putImageData).toHaveBeenCalledTimes(drawn);
    act(() => visible(true));
    frames.run(5100);
    expect(putImageData.mock.calls.length).toBeGreaterThan(drawn);
    expect(createEarthRenderer).toHaveBeenCalledOnce();
    view.unmount();
    expect(release).toHaveBeenCalledOnce();
    act(() => visible(true));
    expect(frames.callbacks.size).toBe(0);
  });

  it("cannot revive native-hidden animation after a late texture decode", async () => {
    let complete!: (value: Awaited<ReturnType<typeof loadEarthTexture>>) => void;
    vi.mocked(loadEarthTexture).mockReturnValue(new Promise((resolve) => { complete = resolve; }));
    let visible: (value: boolean) => void = () => undefined;
    vi.spyOn(lifecycle, "observeWindowPresentation").mockImplementation((handler) => {
      visible = handler; handler(true); return () => undefined;
    });
    render(<Planet mode="clean" />);
    act(() => visible(false));
    await act(async () => complete({ width: 8, height: 4, rgb: new Uint8ClampedArray(96) }));
    expect(createEarthRenderer).not.toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    act(() => visible(true));
    expect(createEarthRenderer).toHaveBeenCalledOnce();
    expect(frames.callbacks.size).toBe(1);
  });

  it("uses the committed 15 FPS budget and stops the old loop when app motion changes", () => {
    const context = {
      preferences: { ...DEFAULT_DESKTOP_PREFERENCES, planet_fps: 15 },
      loading: false, unavailable: false, saving: false, saveError: false,
      update: async () => null, reload: () => undefined,
    };
    const view = render(<DesktopPreferencesContext.Provider value={context}><Planet mode="software" /></DesktopPreferencesContext.Provider>);
    frames.run(1000);
    expect(putImageData).toHaveBeenCalledTimes(2);
    frames.run(1040);
    expect(putImageData).toHaveBeenCalledTimes(2);
    frames.run(1070);
    expect(putImageData).toHaveBeenCalledTimes(3);
    view.rerender(<DesktopPreferencesContext.Provider value={{ ...context, preferences: { ...context.preferences, motion: "reduced" } }}><Planet mode="software" /></DesktopPreferencesContext.Provider>);
    expect(frames.callbacks.size).toBe(0);
    expect(putImageData).toHaveBeenCalledTimes(4);
    frames.run(1200);
    expect(putImageData).toHaveBeenCalledTimes(4);
  });
  it("is decorative and draws a first frame, then animates at no more than 30 fps", () => {
    const view = render(<Planet mode="software" />);
    const host = view.container.querySelector(".planet");
    expect(host).toHaveAttribute("aria-hidden", "true");
    expect(host?.querySelector("canvas")).toBeInTheDocument();
    expect(putImageData).toHaveBeenCalledTimes(1);
    expect(requestFrame).toHaveBeenCalledTimes(1);

    frames.run(1000);
    expect(putImageData).toHaveBeenCalledTimes(2);
    frames.run(1010);
    expect(putImageData).toHaveBeenCalledTimes(2);
    frames.run(1040);
    expect(putImageData).toHaveBeenCalledTimes(3);
    expect(frames.callbacks.size).toBe(1);
  });

  it("stops when the document is hidden and resumes when it is visible", () => {
    render(<Planet mode="software" />);
    frames.run(100);
    setHidden(true);
    expect(cancelFrame).toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    const drawn = putImageData.mock.calls.length;
    frames.run(500);
    expect(putImageData).toHaveBeenCalledTimes(drawn);

    setHidden(false);
    expect(frames.callbacks.size).toBe(1);
    frames.run(600);
    expect(putImageData).toHaveBeenCalledTimes(drawn + 1);
  });

  it("stops the loop when the route makes it inactive and leaves one still frame", () => {
    const view = render(<Planet mode="optimize" />);
    frames.run(100);
    const drawn = putImageData.mock.calls.length;
    const requested = requestFrame.mock.calls.length;
    view.rerender(<Planet mode="optimize" active={false} />);
    expect(cancelFrame).toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    expect(putImageData).toHaveBeenCalledTimes(drawn + 1);
    expect(requestFrame).toHaveBeenCalledTimes(requested);
    setHidden(true);
    setHidden(false);
    expect(frames.callbacks.size).toBe(0);
  });

  it("stops on unmount and ignores later visibility changes", () => {
    const view = render(<Planet mode="analyze" />);
    view.unmount();
    expect(cancelFrame).toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    setHidden(true);
    setHidden(false);
    expect(frames.callbacks.size).toBe(0);
    expect(putImageData).toHaveBeenCalledTimes(1);
  });

  it("draws exactly one frame under reduced motion", () => {
    media["(prefers-reduced-motion: reduce)"] = true;
    render(<Planet mode="status" />);
    expect(putImageData).toHaveBeenCalledTimes(1);
    expect(requestFrame).not.toHaveBeenCalled();
    setHidden(true);
    setHidden(false);
    expect(requestFrame).not.toHaveBeenCalled();
    expect(putImageData).toHaveBeenCalledTimes(1);
  });

  it("does not draw under forced colors; CSS shows an outlined circle instead", () => {
    media["(forced-colors: active)"] = true;
    render(<Planet mode="clean" />);
    expect(putImageData).not.toHaveBeenCalled();
    expect(requestFrame).not.toHaveBeenCalled();
    expect(loadEarthTexture).not.toHaveBeenCalled();
    const forced = stageStyles.slice(
      stageStyles.indexOf("@media (forced-colors: active)"),
    );
    expect(forced).toMatch(/\.planet-canvas\s*\{\s*display:\s*none;/);
    expect(forced).toMatch(/\.planet\s*\{[^}]*border:\s*2px solid CanvasText;/);
  });

  it("defers the first texture and draw while initially hidden", async () => {
    hidden = true;
    render(<Planet mode="clean" />);
    expect(loadEarthTexture).not.toHaveBeenCalled();
    expect(putImageData).not.toHaveBeenCalled();
    setHidden(false);
    await act(async () => undefined);
    expect(loadEarthTexture).toHaveBeenCalledTimes(1);
    expect(putImageData).toHaveBeenCalledTimes(1);
    expect(frames.callbacks.size).toBe(1);
  });

  it("ignores late texture completion after a route unmount", async () => {
    let resolve!: (value: Awaited<ReturnType<typeof loadEarthTexture>>) => void;
    vi.mocked(loadEarthTexture).mockReturnValue(new Promise((done) => { resolve = done; }));
    const view = render(<Planet mode="clean" />);
    view.unmount();
    await act(async () => resolve({ width: 8, height: 4, rgb: new Uint8ClampedArray(96) }));
    expect(putImageData).not.toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    setHidden(true);
    setHidden(false);
    expect(putImageData).not.toHaveBeenCalled();
  });

  it("suspends a texture that finishes decoding while hidden until shown", async () => {
    let resolve!: (value: Awaited<ReturnType<typeof loadEarthTexture>>) => void;
    vi.mocked(loadEarthTexture).mockReturnValue(new Promise((done) => { resolve = done; }));
    render(<Planet mode="clean" />);
    setHidden(true);
    await act(async () => resolve({ width: 8, height: 4, rgb: new Uint8ClampedArray(96) }));
    expect(putImageData).not.toHaveBeenCalled();
    expect(frames.callbacks.size).toBe(0);
    setHidden(false);
    expect(putImageData).toHaveBeenCalledTimes(1);
  });

  it("retains the packaged still after decode failure without a retry loop", async () => {
    vi.mocked(loadEarthTexture).mockRejectedValue(new Error("corrupt texture"));
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    expect(view.container.querySelector(".planet")).toHaveAttribute("data-render-state", "fallback");
    expect(view.container.querySelector("img")).toHaveAttribute("src", "/earth/still.png");
    setHidden(true);
    setHidden(false);
    frames.run(1000);
    expect(loadEarthTexture).toHaveBeenCalledTimes(1);
    expect(frames.callbacks.size).toBe(0);
  });

  it("keeps a fallback when the GPU context is unavailable", async () => {
    vi.mocked(createEarthRenderer).mockReturnValue(null);
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    expect(view.container.querySelector(".planet")).toHaveAttribute("data-render-state", "fallback");
    expect(loadEarthTexture).toHaveBeenCalledTimes(1);
    expect(frames.callbacks.size).toBe(0);
  });

  it("redraws a reduced-motion still only for an actual size or DPR change", async () => {
    media["(prefers-reduced-motion: reduce)"] = true;
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    const canvas = view.container.querySelector("canvas")!;
    expect(putImageData).toHaveBeenCalledTimes(1);
    act(() => window.dispatchEvent(new Event("resize")));
    expect(putImageData).toHaveBeenCalledTimes(1);
    vi.stubGlobal("devicePixelRatio", 3);
    act(() => window.dispatchEvent(new Event("resize")));
    expect(canvas.width).toBe(464);
    expect(putImageData).toHaveBeenCalledTimes(2);
    expect(frames.callbacks.size).toBe(0);
    Object.defineProperty(canvas, "clientWidth", { configurable: true, value: 168 });
    act(() => window.dispatchEvent(new Event("resize")));
    expect(canvas.width).toBe(336);
    expect(putImageData).toHaveBeenCalledTimes(3);
    view.unmount();
    act(() => window.dispatchEvent(new Event("resize")));
    expect(putImageData).toHaveBeenCalledTimes(3);
  });

  it("stops a lost context and keeps a local still", async () => {
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    act(() => view.container.querySelector("canvas")!.dispatchEvent(new Event("webglcontextlost")));
    expect(frames.callbacks.size).toBe(0);
    expect(view.container.querySelector(".planet")).toHaveAttribute("data-render-state", "fallback");
    setHidden(true);
    setHidden(false);
    expect(frames.callbacks.size).toBe(0);
  });

  it("replaces a canvas when switching between GPU and 2D modes", async () => {
    const dispose = vi.fn();
    vi.mocked(createEarthRenderer).mockReturnValue({ draw: putImageData, dispose });
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    const earthCanvas = view.container.querySelector("canvas");
    view.rerender(<Planet mode="software" />);
    expect(view.container.querySelector("canvas")).not.toBe(earthCanvas);
    expect(dispose).toHaveBeenCalledTimes(1);
    expect(view.container.querySelector(".planet")).toHaveAttribute("data-render-state", "ready");
  });

  it("keeps an outline when the fallback image also fails", async () => {
    vi.mocked(loadEarthTexture).mockRejectedValue(new Error("missing"));
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    const fallback = view.container.querySelector("img")!;
    act(() => fallback.dispatchEvent(new Event("error")));
    expect(fallback.hidden).toBe(true);
    expect(view.container.querySelector(".planet")).toHaveAttribute("data-render-state", "fallback");
    expect(frames.callbacks.size).toBe(0);
  });

  it("disconnects the sizing observer and releases the GPU renderer on unmount", async () => {
    const observe = vi.fn();
    const disconnect = vi.fn();
    const dispose = vi.fn();
    vi.stubGlobal("ResizeObserver", class { observe = observe; disconnect = disconnect; });
    vi.mocked(createEarthRenderer).mockReturnValue({ draw: putImageData, dispose });
    const view = render(<Planet mode="clean" />);
    await act(async () => undefined);
    expect(observe).toHaveBeenCalledTimes(1);
    view.unmount();
    expect(disconnect).toHaveBeenCalledTimes(1);
    expect(dispose).toHaveBeenCalledTimes(1);
    expect(frames.callbacks.size).toBe(0);
  });

  it("keeps reduced-motion and forced-colors selectors in the stage styles without CSS shading", () => {
    expect(stageStyles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(stageStyles).toContain("@media (forced-colors: active)");
    expect(stageStyles).not.toContain("linear-gradient");
    expect(stageStyles).not.toContain("radial-gradient");
    expect(stageStyles).not.toContain("backdrop-filter");
  });
});

describe("planet renderer", () => {
  it("builds one original palette per mode and caps the source diameter", () => {
    expect(Object.keys(PLANET_PALETTES)).toEqual([...MODE_IDS]);
    const texture = buildPlanetTexture(PLANET_PALETTES.clean);
    const geometry = buildPlanetGeometry(1024, texture, 0.1);
    expect(geometry.diameter).toBe(MAX_SOURCE_DIAMETER);
    expect(geometry.pixels.length).toBeLessThanOrEqual(
      MAX_SOURCE_DIAMETER * MAX_SOURCE_DIAMETER,
    );
  });

  it("is deterministic for a seed, leaves corners transparent, and rotates", () => {
    const texture = buildPlanetTexture(PLANET_PALETTES.analyze);
    expect(buildPlanetTexture(PLANET_PALETTES.analyze).rgb).toEqual(
      texture.rgb,
    );
    const geometry = buildPlanetGeometry(64, texture, 0.1);
    const first = new Uint8ClampedArray(64 * 64 * 4);
    const turned = new Uint8ClampedArray(64 * 64 * 4);
    renderPlanetFrame(first, geometry, texture, 0);
    renderPlanetFrame(turned, geometry, texture, 0.25);
    expect(first[3]).toBe(0);
    const center = (32 * 64 + 32) * 4;
    expect(first[center + 3]).toBe(255);
    expect(turned).not.toEqual(first);
  });
});
