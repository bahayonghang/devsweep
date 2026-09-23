import { useEffect, useRef } from "react";
import type { ModeId } from "../app-shell/AppShell";
import { PLANET_PALETTES } from "./planet-palettes";
import {
  buildPlanetGeometry,
  buildPlanetTexture,
  MAX_SOURCE_DIAMETER,
  renderPlanetFrame,
  type PlanetGeometry,
  type PlanetTexture,
} from "./planet-renderer";

/** Frame budget: at most 30 frames per second. */
const FRAME_INTERVAL_MS = 1000 / 30;
/** One full rotation takes this long. */
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

function mediaMatches(query: string): boolean {
  return globalThis.matchMedia?.(query)?.matches ?? false;
}

export interface PlanetProps {
  readonly mode: ModeId;
  /** False stops the loop and leaves one still frame. */
  readonly active?: boolean;
}

/** Original procedural planet. Decorative only: it carries no data. */
export function Planet({ mode, active = true }: PlanetProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || mediaMatches("(forced-colors: active)")) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const source = document.createElement("canvas");
    const sourceContext = source.getContext("2d");
    if (!sourceContext) return;
    const texture = textureFor(mode);
    const ambient = PLANET_PALETTES[mode].ambient;
    let geometry: PlanetGeometry | null = null;
    let frame: ImageData | null = null;
    let turn = 0;

    const draw = () => {
      const ratio = globalThis.devicePixelRatio || 1;
      const cssSize = canvas.clientWidth || FALLBACK_CSS_SIZE;
      const buffer = Math.max(1, Math.round(cssSize * ratio));
      if (canvas.width !== buffer || canvas.height !== buffer) {
        canvas.width = buffer;
        canvas.height = buffer;
      }
      if (!geometry || !frame || geometry.diameter !== Math.max(8, Math.min(MAX_SOURCE_DIAMETER, buffer))) {
        geometry = buildPlanetGeometry(buffer, texture, ambient);
        source.width = geometry.diameter;
        source.height = geometry.diameter;
        frame = sourceContext.createImageData(
          geometry.diameter,
          geometry.diameter,
        );
      }
      renderPlanetFrame(frame.data, geometry, texture, turn);
      sourceContext.putImageData(frame, 0, 0);
      context.clearRect(0, 0, buffer, buffer);
      context.imageSmoothingEnabled = true;
      context.drawImage(source, 0, 0, buffer, buffer);
    };

    draw();
    if (!active || mediaMatches("(prefers-reduced-motion: reduce)")) return;

    let handle: number | null = null;
    let previous: number | null = null;
    let lastDrawn = Number.NEGATIVE_INFINITY;
    let disposed = false;

    const tick = (now: number) => {
      handle = null;
      if (disposed || document.hidden) return;
      if (previous !== null)
        turn +=
          Math.min(now - previous, FRAME_INTERVAL_MS * 2) / TURN_DURATION_MS;
      previous = now;
      if (now - lastDrawn >= FRAME_INTERVAL_MS - 1) {
        lastDrawn = now;
        draw();
      }
      handle = requestAnimationFrame(tick);
    };
    const start = () => {
      if (disposed || document.hidden || handle !== null) return;
      previous = null;
      handle = requestAnimationFrame(tick);
    };
    const stop = () => {
      if (handle !== null) cancelAnimationFrame(handle);
      handle = null;
    };
    const onVisibilityChange = () => {
      if (document.hidden) stop();
      else start();
    };

    document.addEventListener("visibilitychange", onVisibilityChange);
    start();
    return () => {
      disposed = true;
      stop();
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [mode, active]);

  return (
    <span className="planet" data-planet={mode} aria-hidden="true">
      <canvas ref={canvasRef} className="planet-canvas" />
    </span>
  );
}
