import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });

describe("local Earth texture", () => {
  it("coalesces concurrent decoding and retains only one bounded RGB source", async () => {
    vi.resetModules();
    const decode = vi.fn(async () => undefined);
    vi.stubGlobal("Image", class {
      naturalWidth = 1024;
      naturalHeight = 512;
      src = "";
      decode = decode;
    });
    const bytes = new Uint8ClampedArray(1024 * 512 * 4).fill(42);
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({ drawImage: vi.fn(), getImageData: () => ({ data: bytes }) } as unknown as CanvasRenderingContext2D);
    const { loadEarthTexture } = await import("./earth-texture");
    const first = loadEarthTexture();
    expect(loadEarthTexture()).toBe(first);
    const texture = await first;
    expect(texture.width).toBe(1024);
    expect(texture.height).toBe(512);
    expect(texture.rgb.byteLength).toBe(1024 * 512 * 3);
    expect(texture.rgb[0]).toBe(42);
    expect(loadEarthTexture()).toBe(first);
    expect(decode).toHaveBeenCalledTimes(1);
  });

  it("remembers a corrupt asset failure instead of retrying on later mounts", async () => {
    vi.resetModules();
    const decode = vi.fn(async () => { throw new Error("corrupt"); });
    vi.stubGlobal("Image", class { src = ""; decode = decode; });
    const { loadEarthTexture } = await import("./earth-texture");
    const first = loadEarthTexture();
    await expect(first).rejects.toThrow("corrupt");
    expect(loadEarthTexture()).toBe(first);
    expect(decode).toHaveBeenCalledTimes(1);
  });

  it("rejects an unexpected image size before allocating its pixel buffer", async () => {
    vi.resetModules();
    vi.stubGlobal("Image", class {
      src = ""; naturalWidth = 8192; naturalHeight = 4096;
      decode = async () => undefined;
    });
    const context = vi.spyOn(HTMLCanvasElement.prototype, "getContext");
    const { loadEarthTexture } = await import("./earth-texture");
    await expect(loadEarthTexture()).rejects.toThrow("dimensions");
    expect(context).not.toHaveBeenCalled();
  });
});
