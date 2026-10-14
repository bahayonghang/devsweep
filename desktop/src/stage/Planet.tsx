import { usePreferences } from "../preferences/context";
import { useMediaPreference } from "../preferences/appearance";
import { useEffect, useRef } from "react";
import { observeWindowPresentation } from "../lifecycle";
import type { ModeId } from "../app-shell/AppShell";
import { PLANET_PALETTES } from "./planet-palettes";
import { EARTH_FALLBACK_URL, EARTH_INITIAL_TURN, loadEarthTexture } from "./earth-texture";
import { createEarthRenderer, type EarthRenderer } from "./earth-renderer";
import { buildPlanetGeometry, buildPlanetTexture, MAX_SOURCE_DIAMETER, renderPlanetFrame, type PlanetGeometry, type PlanetTexture } from "./planet-renderer";

const TURN_DURATION_MS = 96_000;
const FALLBACK_CSS_SIZE = 232;
const textures = new Map<ModeId, PlanetTexture>();

function textureFor(mode: ModeId): PlanetTexture {
  let texture = textures.get(mode);
  if (!texture) {
    texture = buildPlanetTexture(PLANET_PALETTES[mode]);
    textures.set(mode, texture);
  }
  return texture;
}

export interface PlanetProps {
  readonly mode: ModeId;
  /** False stops the loop and leaves one still frame. */
  readonly active?: boolean;
}

/** Decorative geography for Clean; the other modes retain their seeded planets. */
export function Planet({ mode, active = true }: PlanetProps) {
  const { preferences } = usePreferences();
  const systemReduced = useMediaPreference("(prefers-reduced-motion: reduce)");
  const forcedColors = useMediaPreference("(forced-colors: active)");
  const reduced = preferences.motion === "reduced" || systemReduced;
  const frameIntervalMs = 1000 / preferences.planet_fps;
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const hostRef = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const host = hostRef.current;
    if (!canvas || !host) return;
    host.dataset.renderState = "fallback";
    if (forcedColors) return;
    const geographic = mode === "clean";
    const context = geographic ? null : canvas.getContext("2d");
    const source = document.createElement("canvas");
    const sourceContext = geographic ? null : source.getContext("2d");
    if (!geographic && (!context || !sourceContext)) return;
    let earthRenderer: EarthRenderer | null = null;
    let texture: PlanetTexture | null = null;
    let geometry: PlanetGeometry | null = null;
    let frame: ImageData | null = null;
    let turn = geographic ? EARTH_INITIAL_TURN : 0;
    let handle: number | null = null;
    let previous: number | null = null;
    let lastDrawn = Number.NEGATIVE_INFINITY;
    let disposed = false;
    let failed = false;
    let loading = false;
    let dirty = true;
    let windowVisible = false;
    const hidden = () => document.hidden || !windowVisible;

    const stop = () => {
      if (handle !== null) cancelAnimationFrame(handle);
      handle = null;
      previous = null;
    };
    const fallback = () => {
      failed = true;
      stop();
      host.dataset.renderState = "fallback";
    };
    const bufferSize = () => Math.max(8, Math.min(512, Math.round(
      (canvas.clientWidth || FALLBACK_CSS_SIZE) * Math.min(2, globalThis.devicePixelRatio || 1),
    )));
    const draw = () => {
      if (disposed || failed || hidden() || !texture) return;
      try {
        const buffer = bufferSize();
        if (canvas.width !== buffer || canvas.height !== buffer) {
          canvas.width = buffer;
          canvas.height = buffer;
        }
        if (earthRenderer) {
          earthRenderer.draw(turn, buffer);
        } else if (sourceContext && context) {
          if (!geometry || !frame || geometry.diameter !== Math.min(MAX_SOURCE_DIAMETER, buffer)) {
            geometry = buildPlanetGeometry(buffer, texture, PLANET_PALETTES[mode].ambient);
            source.width = geometry.diameter;
            source.height = geometry.diameter;
            frame = sourceContext.createImageData(geometry.diameter, geometry.diameter);
          }
          renderPlanetFrame(frame.data, geometry, texture, turn);
          sourceContext.putImageData(frame, 0, 0);
          context.clearRect(0, 0, buffer, buffer);
          context.imageSmoothingEnabled = true;
          context.drawImage(source, 0, 0, buffer, buffer);
        }
        host.dataset.renderState = "ready";
        dirty = false;
        lastDrawn = performance.now();
      } catch {
        fallback();
      }
    };
    const tick = (now: number) => {
      handle = null;
      if (disposed || failed || hidden()) return;
      if (previous !== null)
        turn += Math.min(now - previous, frameIntervalMs * 2) / TURN_DURATION_MS;
      previous = now;
      if (now - lastDrawn >= frameIntervalMs - 0.5) {
        draw();
        lastDrawn = now;
      }
      if (!failed) handle = requestAnimationFrame(tick);
    };
    const start = () => {
      if (disposed || failed || hidden()) return;
      if (!texture) {
        if (loading) return;
        if (!geographic) {
          texture = textureFor(mode);
        } else {
          loading = true;
          void loadEarthTexture().then((loaded) => {
            if (disposed) return;
            texture = loaded;
            start();
          }, () => { if (!disposed) fallback(); });
          return;
        }
      }
      if (geographic && !earthRenderer) {
        earthRenderer = createEarthRenderer(canvas, texture);
        if (!earthRenderer) { fallback(); return; }
      }
      if (dirty) draw();
      if (active && !reduced && !failed && handle === null)
        handle = requestAnimationFrame(tick);
    };
    const onVisibilityChange = () => {
      if (hidden()) stop();
      else start();
    };
    const onResize = () => {
      if (canvas.width !== bufferSize()) {
        dirty = true;
        if (!hidden()) draw();
      }
    };
    let resolution: MediaQueryList | undefined;
    const watchResolution = () => {
      resolution?.removeEventListener?.("change", onResolutionChange);
      resolution = globalThis.matchMedia?.(`(resolution: ${globalThis.devicePixelRatio || 1}dppx)`);
      resolution?.addEventListener?.("change", onResolutionChange);
    };
    const onResolutionChange = () => { watchResolution(); onResize(); };
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(onResize);
    observer?.observe(canvas);
    watchResolution();
    window.addEventListener("resize", onResize);
    document.addEventListener("visibilitychange", onVisibilityChange);
    canvas.addEventListener("contextlost", fallback);
    canvas.addEventListener("webglcontextlost", fallback);
    const unobserveWindow = observeWindowPresentation((visible) => {
      windowVisible = visible;
      onVisibilityChange();
    });
    start();
    return () => {
      disposed = true;
      unobserveWindow();
      stop();
      observer?.disconnect();
      resolution?.removeEventListener?.("change", onResolutionChange);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      canvas.removeEventListener("contextlost", fallback);
      canvas.removeEventListener("webglcontextlost", fallback);
      earthRenderer?.dispose();
      source.width = source.height = 0;
    };
  }, [mode, active, reduced, forcedColors, frameIntervalMs]);

  return (
    <span ref={hostRef} className="planet" data-planet={mode} aria-hidden="true">
      {mode === "clean" && !forcedColors ? <img className="planet-fallback" src={EARTH_FALLBACK_URL} alt="" onError={(event) => { event.currentTarget.hidden = true; }} /> : null}
      <canvas key={mode} ref={canvasRef} className="planet-canvas" />
    </span>
  );
}
