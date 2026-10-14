import type { PlanetTexture } from "./planet-renderer";

export const EARTH_TEXTURE_URL = `${import.meta.env.BASE_URL}earth/natural-earth.png`;
export const EARTH_FALLBACK_URL = `${import.meta.env.BASE_URL}earth/still.png`;
export const EARTH_INITIAL_TURN = 20 / 360;

// One decoded geographic source per webview. A failed load is remembered too;
// route changes must not repeatedly decode a corrupt packaged asset.
let earth: Promise<PlanetTexture> | undefined;

export function loadEarthTexture(): Promise<PlanetTexture> {
  earth ??= decodeEarth();
  return earth;
}

async function decodeEarth(): Promise<PlanetTexture> {
  const image = new Image();
  image.src = EARTH_TEXTURE_URL;
  await image.decode();
  if (image.naturalWidth !== 1024 || image.naturalHeight !== 512)
    throw new Error("Unexpected packaged Earth dimensions");
  const canvas = document.createElement("canvas");
  canvas.width = image.naturalWidth;
  canvas.height = image.naturalHeight;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("Earth decoding context unavailable");
  context.drawImage(image, 0, 0);
  const rgba = context.getImageData(0, 0, canvas.width, canvas.height).data;
  const rgb = new Uint8ClampedArray(canvas.width * canvas.height * 3);
  for (let input = 0, output = 0; input < rgba.length; input += 4, output += 3) {
    rgb[output] = rgba[input];
    rgb[output + 1] = rgba[input + 1];
    rgb[output + 2] = rgba[input + 2];
  }
  // Release the temporary canvas backing store; only the bounded RGB remains.
  canvas.width = canvas.height = 0;
  return { width: image.naturalWidth, height: image.naturalHeight, rgb };
}
