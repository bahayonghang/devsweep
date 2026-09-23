import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MODE_IDS } from "../app-shell/AppShell";
import { Planet } from "./Planet";
import { PLANET_PALETTES } from "./planet-palettes";
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
  installMocks();
});

afterEach(() => {
  HTMLCanvasElement.prototype.getContext = originalGetContext;
  vi.unstubAllGlobals();
  Reflect.deleteProperty(document, "hidden");
});

describe("Planet", () => {
  it("is decorative and draws a first frame, then animates at no more than 30 fps", () => {
    const view = render(<Planet mode="clean" />);
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
    const forced = stageStyles.slice(
      stageStyles.indexOf("@media (forced-colors: active)"),
    );
    expect(forced).toMatch(/\.planet-canvas\s*\{\s*display:\s*none;/);
    expect(forced).toMatch(/\.planet\s*\{[^}]*border:\s*2px solid CanvasText;/);
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
