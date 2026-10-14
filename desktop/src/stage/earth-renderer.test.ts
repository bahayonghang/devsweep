import { afterEach, describe, expect, it, vi } from "vitest";
import { createEarthRenderer } from "./earth-renderer";

afterEach(() => vi.restoreAllMocks());
const texture = { width: 8, height: 4, rgb: new Uint8ClampedArray(96) };
function context() {
  const gl = {
    createShader: vi.fn(() => ({})), shaderSource: vi.fn(), compileShader: vi.fn(),
    getShaderParameter: vi.fn(() => true), createProgram: vi.fn(() => ({})),
    attachShader: vi.fn(), linkProgram: vi.fn(), getProgramParameter: vi.fn(() => true),
    useProgram: vi.fn(), createBuffer: vi.fn(() => ({})), createTexture: vi.fn(() => ({})),
    bindBuffer: vi.fn(), bufferData: vi.fn(), getAttribLocation: vi.fn(() => 0),
    enableVertexAttribArray: vi.fn(), vertexAttribPointer: vi.fn(), bindTexture: vi.fn(),
    pixelStorei: vi.fn(), texParameteri: vi.fn(), texImage2D: vi.fn(),
    getUniformLocation: vi.fn(() => ({})), viewport: vi.fn(), clearColor: vi.fn(),
    NO_ERROR: 0, getError: vi.fn(() => 0),
    clear: vi.fn(), uniform1f: vi.fn(), drawArrays: vi.fn(),
    deleteTexture: vi.fn(), deleteBuffer: vi.fn(), deleteProgram: vi.fn(), deleteShader: vi.fn(),
    getExtension: vi.fn(() => ({ loseContext: lose })),
  };
  const lose = vi.fn();
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(gl as unknown as WebGLRenderingContext);
  return { gl, lose };
}
describe("bounded Earth GPU resources", () => {
  it("uploads once, only changes uniforms on draws, and releases every owned object", () => {
    const { gl, lose } = context();
    const renderer = createEarthRenderer(document.createElement("canvas"), texture)!;
    renderer.draw(0, 464);
    renderer.draw(.1, 464);
    expect(gl.texImage2D).toHaveBeenCalledTimes(1);
    expect(gl.bufferData).toHaveBeenCalledTimes(1);
    expect(gl.drawArrays).toHaveBeenCalledTimes(2);
    renderer.dispose();
    renderer.dispose();
    renderer.draw(.2, 464);
    expect(gl.drawArrays).toHaveBeenCalledTimes(2);
    expect(gl.getError).toHaveBeenCalledTimes(1);
    expect(gl.deleteTexture).toHaveBeenCalledTimes(1);
    expect(gl.deleteBuffer).toHaveBeenCalledTimes(1);
    expect(gl.deleteProgram).toHaveBeenCalledTimes(1);
    expect(gl.deleteShader).toHaveBeenCalledTimes(2);
    expect(lose).toHaveBeenCalledTimes(1);
  });

  it("retains a connected context for React effect replay after releasing objects", () => {
    const { gl, lose } = context();
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    const first = createEarthRenderer(canvas, texture)!;
    first.dispose();
    expect(gl.deleteTexture).toHaveBeenCalledTimes(1);
    expect(lose).not.toHaveBeenCalled();
    const second = createEarthRenderer(canvas, texture)!;
    second.draw(0, 232);
    canvas.remove();
    second.dispose();
    expect(lose).toHaveBeenCalledTimes(1);
  });

  it("cleans up a partial shader failure and declines animation", () => {
    const { gl, lose } = context();
    gl.getShaderParameter.mockReturnValue(false);
    expect(createEarthRenderer(document.createElement("canvas"), texture)).toBeNull();
    expect(gl.deleteShader).toHaveBeenCalledTimes(1);
    expect(gl.deleteProgram).toHaveBeenCalledTimes(1);
    expect(gl.drawArrays).not.toHaveBeenCalled();
    expect(lose).toHaveBeenCalledTimes(1);
  });

  it("releases uploaded resources and declines animation when initialization records a GL error", () => {
    const { gl, lose } = context();
    gl.getError.mockReturnValue(0x0505); // OUT_OF_MEMORY is reported without throwing.
    expect(createEarthRenderer(document.createElement("canvas"), texture)).toBeNull();
    expect(gl.getError).toHaveBeenCalledTimes(1);
    expect(gl.deleteTexture).toHaveBeenCalledTimes(1);
    expect(gl.deleteBuffer).toHaveBeenCalledTimes(1);
    expect(gl.deleteProgram).toHaveBeenCalledTimes(1);
    expect(gl.deleteShader).toHaveBeenCalledTimes(2);
    expect(gl.drawArrays).not.toHaveBeenCalled();
    expect(lose).toHaveBeenCalledTimes(1);
  });
});
