import type { ModeId } from "../app-shell/AppShell";

/** One colour band: texture values below `until` take this RGB colour. */
export type PaletteBand = readonly [
  until: number,
  red: number,
  green: number,
  blue: number,
];

export interface PlanetPalette {
  readonly seed: number;
  /** `terrain` maps noise height to bands; `banded` maps latitude warped by noise. */
  readonly pattern: "terrain" | "banded";
  readonly bands: readonly PaletteBand[];
  /** Noise frequency on the unit sphere. */
  readonly scale: number;
  /** Light floor on the night side, 0..1. */
  readonly ambient: number;
}

/** Original DevSweep palettes. The values were chosen for this project and
 * were not sampled from any third-party image. */
export const PLANET_PALETTES: Readonly<Record<ModeId, PlanetPalette>> = {
  clean: {
    seed: 1307,
    pattern: "terrain",
    scale: 2.1,
    ambient: 0.12,
    bands: [
      [0.47, 22, 52, 96],
      [0.53, 38, 92, 138],
      [0.55, 196, 184, 140],
      [0.66, 70, 122, 72],
      [0.76, 98, 132, 78],
      [1.01, 214, 219, 206],
    ],
  },
  software: {
    seed: 2711,
    pattern: "terrain",
    scale: 2.6,
    ambient: 0.1,
    bands: [
      [0.4, 84, 32, 22],
      [0.5, 128, 52, 32],
      [0.6, 168, 80, 46],
      [0.7, 196, 114, 72],
      [1.01, 226, 158, 112],
    ],
  },
  optimize: {
    seed: 4093,
    pattern: "terrain",
    scale: 3.2,
    ambient: 0.1,
    bands: [
      [0.38, 68, 72, 80],
      [0.48, 102, 108, 118],
      [0.58, 138, 144, 152],
      [0.7, 176, 181, 188],
      [1.01, 212, 214, 218],
    ],
  },
  analyze: {
    seed: 5381,
    pattern: "banded",
    scale: 1.8,
    ambient: 0.1,
    bands: [
      [0.2, 104, 66, 26],
      [0.4, 152, 100, 38],
      [0.6, 200, 144, 64],
      [0.8, 228, 184, 110],
      [1.01, 178, 118, 52],
    ],
  },
  status: {
    seed: 7919,
    pattern: "terrain",
    scale: 1.6,
    ambient: 0.42,
    bands: [
      [0.42, 236, 200, 116],
      [0.54, 244, 218, 150],
      [0.66, 250, 234, 190],
      [1.01, 255, 246, 222],
    ],
  },
};
