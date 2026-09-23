import type { PlanetPalette } from "./planet-palettes";

/** Largest source diameter in pixels. One frame costs at most this squared. */
export const MAX_SOURCE_DIAMETER = 256;
const TEXTURE_WIDTH = 512;
const TEXTURE_HEIGHT = 256;
/** Half width of the blend across a band boundary, in texture-value units. */
const BAND_BLEND = 0.03;
const OCTAVES = 5;
/** Strength of the domain warp that breaks up the value-noise lattice. */
const WARP = 1.4;
/** Fixed light direction (x right, y up, z toward the viewer), normalized. */
const LIGHT = (() => {
  const x = -0.55;
  const y = 0.5;
  const z = 0.67;
  const length = Math.hypot(x, y, z);
  return [x / length, y / length, z / length] as const;
})();

function hash(x: number, y: number, z: number, seed: number): number {
  let h =
    Math.imul(x, 374761393) ^
    Math.imul(y, 668265263) ^
    Math.imul(z, 2147483647) ^
    Math.imul(seed, 144665);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

function smooth(t: number): number {
  return t * t * (3 - 2 * t);
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/** Seeded 3D value noise in 0..1. */
function valueNoise(x: number, y: number, z: number, seed: number): number {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const z0 = Math.floor(z);
  const tx = smooth(x - x0);
  const ty = smooth(y - y0);
  const tz = smooth(z - z0);
  const c000 = hash(x0, y0, z0, seed);
  const c100 = hash(x0 + 1, y0, z0, seed);
  const c010 = hash(x0, y0 + 1, z0, seed);
  const c110 = hash(x0 + 1, y0 + 1, z0, seed);
  const c001 = hash(x0, y0, z0 + 1, seed);
  const c101 = hash(x0 + 1, y0, z0 + 1, seed);
  const c011 = hash(x0, y0 + 1, z0 + 1, seed);
  const c111 = hash(x0 + 1, y0 + 1, z0 + 1, seed);
  return lerp(
    lerp(lerp(c000, c100, tx), lerp(c010, c110, tx), ty),
    lerp(lerp(c001, c101, tx), lerp(c011, c111, tx), ty),
    tz,
  );
}

function fractalNoise(x: number, y: number, z: number, seed: number): number {
  let total = 0;
  let amplitude = 0.5;
  let frequency = 1;
  let weight = 0;
  for (let octave = 0; octave < OCTAVES; octave += 1) {
    total +=
      valueNoise(
        x * frequency,
        y * frequency,
        z * frequency,
        seed + octave * 101,
      ) * amplitude;
    weight += amplitude;
    amplitude *= 0.5;
    frequency *= 2;
  }
  return total / weight;
}

/** Colour for one texture value. Each band boundary blends over
 * `2 * BAND_BLEND` so edges stay soft after rotation. */
function bandColour(
  bands: PlanetPalette["bands"],
  value: number,
): readonly [number, number, number] {
  const found = bands.findIndex(([until]) => value < until);
  const index = found === -1 ? bands.length - 1 : found;
  const band = bands[index];
  let neighbour = band;
  let weight = 0;
  if (index > 0 && value - bands[index - 1][0] < BAND_BLEND) {
    neighbour = bands[index - 1];
    weight = 0.5 - 0.5 * smooth(Math.min(1, (value - bands[index - 1][0]) / BAND_BLEND));
  } else if (index < bands.length - 1 && band[0] - value < BAND_BLEND) {
    neighbour = bands[index + 1];
    weight = 0.5 - 0.5 * smooth(Math.min(1, (band[0] - value) / BAND_BLEND));
  }
  return [
    lerp(band[1], neighbour[1], weight),
    lerp(band[2], neighbour[2], weight),
    lerp(band[3], neighbour[3], weight),
  ];
}

/** Equirectangular RGB texture sampled from 3D noise on the unit sphere, so
 * the longitude seam matches. */
export interface PlanetTexture {
  readonly width: number;
  readonly height: number;
  readonly rgb: Uint8ClampedArray;
}

export function buildPlanetTexture(palette: PlanetPalette): PlanetTexture {
  const rgb = new Uint8ClampedArray(TEXTURE_WIDTH * TEXTURE_HEIGHT * 3);
  for (let row = 0; row < TEXTURE_HEIGHT; row += 1) {
    const latitude = (0.5 - (row + 0.5) / TEXTURE_HEIGHT) * Math.PI;
    const cosLatitude = Math.cos(latitude);
    const y = Math.sin(latitude);
    for (let column = 0; column < TEXTURE_WIDTH; column += 1) {
      const longitude = ((column + 0.5) / TEXTURE_WIDTH) * Math.PI * 2;
      const x = cosLatitude * Math.cos(longitude);
      const z = cosLatitude * Math.sin(longitude);
      const px = (x + 2) * palette.scale;
      const py = (y + 2) * palette.scale;
      const pz = (z + 2) * palette.scale;
      const warp = (fractalNoise(px + 5.2, py + 1.3, pz + 2.8, palette.seed + 7) - 0.5) * WARP;
      const noise = fractalNoise(px + warp, py - warp, pz + warp, palette.seed);
      const value =
        palette.pattern === "banded"
          ? 0.5 + 0.5 * Math.sin(y * 9 + (noise - 0.5) * 3.2)
          : noise;
      const [red, green, blue] = bandColour(palette.bands, value);
      const offset = (row * TEXTURE_WIDTH + column) * 3;
      rgb[offset] = red;
      rgb[offset + 1] = green;
      rgb[offset + 2] = blue;
    }
  }
  return { width: TEXTURE_WIDTH, height: TEXTURE_HEIGHT, rgb };
}

/** Per-pixel sphere geometry for one source diameter. Computed once; each
 * frame only shifts the texture longitude. */
export interface PlanetGeometry {
  readonly diameter: number;
  readonly pixels: Uint32Array;
  readonly rowOffsets: Uint32Array;
  readonly columns: Float32Array;
  readonly shade: Float32Array;
  readonly alpha: Uint8ClampedArray;
}

export function buildPlanetGeometry(
  diameter: number,
  texture: PlanetTexture,
  ambient: number,
): PlanetGeometry {
  const size = Math.max(8, Math.min(MAX_SOURCE_DIAMETER, Math.round(diameter)));
  const radius = size / 2;
  const pixels: number[] = [];
  const rowOffsets: number[] = [];
  const columns: number[] = [];
  const shade: number[] = [];
  const alpha: number[] = [];
  for (let py = 0; py < size; py += 1) {
    for (let px = 0; px < size; px += 1) {
      const nx = (px + 0.5 - radius) / radius;
      const ny = (radius - (py + 0.5)) / radius;
      const distance = Math.hypot(nx, ny);
      const coverage = Math.min(1, Math.max(0, (1 - distance) * radius + 0.5));
      if (coverage <= 0) continue;
      const nz = Math.sqrt(Math.max(0, 1 - Math.min(1, distance * distance)));
      const latitude = Math.asin(Math.max(-1, Math.min(1, ny)));
      const longitude = Math.atan2(nx, nz);
      const row = Math.min(
        texture.height - 1,
        Math.max(0, Math.floor((0.5 - latitude / Math.PI) * texture.height)),
      );
      const lambert = Math.max(
        0,
        nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2],
      );
      const rim = 0.55 + 0.45 * Math.sqrt(nz);
      pixels.push(py * size + px);
      rowOffsets.push(row * texture.width);
      columns.push((longitude / (Math.PI * 2) + 0.5) * texture.width);
      shade.push((ambient + (1 - ambient) * lambert) * rim);
      alpha.push(Math.round(coverage * 255));
    }
  }
  return {
    diameter: size,
    pixels: Uint32Array.from(pixels),
    rowOffsets: Uint32Array.from(rowOffsets),
    columns: Float32Array.from(columns),
    shade: Float32Array.from(shade),
    alpha: Uint8ClampedArray.from(alpha),
  };
}

/** Writes one frame into `target` (RGBA, diameter x diameter). `turn` is the
 * rotation in whole turns; only its fractional part matters. */
export function renderPlanetFrame(
  target: Uint8ClampedArray,
  geometry: PlanetGeometry,
  texture: PlanetTexture,
  turn: number,
): void {
  target.fill(0);
  const shift = (turn - Math.floor(turn)) * texture.width;
  for (let index = 0; index < geometry.pixels.length; index += 1) {
    const position = geometry.columns[index] + shift;
    const base = Math.floor(position);
    const fraction = position - base;
    const left = (geometry.rowOffsets[index] + (base % texture.width)) * 3;
    const right = (geometry.rowOffsets[index] + ((base + 1) % texture.width)) * 3;
    const light = geometry.shade[index];
    const offset = geometry.pixels[index] * 4;
    target[offset] = lerp(texture.rgb[left], texture.rgb[right], fraction) * light;
    target[offset + 1] = lerp(texture.rgb[left + 1], texture.rgb[right + 1], fraction) * light;
    target[offset + 2] = lerp(texture.rgb[left + 2], texture.rgb[right + 2], fraction) * light;
    target[offset + 3] = geometry.alpha[index];
  }
}
