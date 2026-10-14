import type { PlanetTexture } from "./planet-renderer";

export interface EarthRenderer {
  draw(turn: number, diameter: number): void;
  dispose(): void;
}

const VERTEX = `
attribute vec2 position;
void main() { gl_Position = vec4(position, 0.0, 1.0); }
`;
const FRAGMENT = `
precision highp float;
uniform sampler2D earth;
uniform float turn;
uniform float diameter;
void main() {
  vec2 p = gl_FragCoord.xy / diameter * 2.0 - 1.0;
  float distance = length(p);
  float coverage = clamp((1.0 - distance) * diameter * 0.5 + 0.5, 0.0, 1.0);
  if (coverage <= 0.0) discard;
  float z = sqrt(max(0.0, 1.0 - min(1.0, distance * distance)));
  vec3 normal = vec3(p, z);
  vec2 uv = vec2(atan(p.x, z) / 6.28318530718 + 0.5 + turn,
    0.5 - asin(clamp(p.y, -1.0, 1.0)) / 3.14159265359);
  float diffuse = max(0.0, dot(normal, normalize(vec3(-0.55, 0.5, 0.67))));
  float shade = (0.42 + 0.58 * diffuse) * (0.85 + 0.15 * sqrt(z));
  gl_FragColor = vec4(texture2D(earth, uv).rgb * shade, coverage);
}
`;

/** One bounded texture and full-screen triangle; projection/filtering stay on GPU.
 * Context/shader failure is a static-image fallback, never another render loop. */
export function createEarthRenderer(canvas: HTMLCanvasElement, texture: PlanetTexture): EarthRenderer | null {
  const gl = canvas.getContext("webgl", {
    alpha: true, antialias: false, depth: false, stencil: false,
    premultipliedAlpha: false, preserveDrawingBuffer: false, powerPreference: "low-power",
  });
  if (!gl) return null;
  const shaders: WebGLShader[] = [];
  let program: WebGLProgram | null = null;
  let buffer: WebGLBuffer | null = null;
  let image: WebGLTexture | null = null;
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    if (image) gl.deleteTexture(image);
    if (buffer) gl.deleteBuffer(buffer);
    if (program) gl.deleteProgram(program);
    for (const shader of shaders) gl.deleteShader(shader);
    // Release the context on route changes too, rather than waiting for GC's
    // browser context limit during repeated mode switches.
    // A connected canvas can be reused by React effect replay or a preference
    // change. Its objects are released above; only detach retires the context.
    if (!canvas.isConnected) gl.getExtension("WEBGL_lose_context")?.loseContext();
  };
  try {
    const compile = (type: number, source: string) => {
      const shader = gl.createShader(type);
      if (!shader) throw new Error("Shader unavailable");
      shaders.push(shader);
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error("Shader compilation failed");
      return shader;
    };
    program = gl.createProgram();
    if (!program) throw new Error("Program unavailable");
    gl.attachShader(program, compile(gl.VERTEX_SHADER, VERTEX));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAGMENT));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error("Program link failed");
    gl.useProgram(program);
    buffer = gl.createBuffer();
    image = gl.createTexture();
    if (!buffer || !image) throw new Error("GPU resource unavailable");
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    gl.bindTexture(gl.TEXTURE_2D, image);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, texture.width, texture.height, 0, gl.RGB, gl.UNSIGNED_BYTE, new Uint8Array(texture.rgb.buffer, texture.rgb.byteOffset, texture.rgb.byteLength));
    const rotation = gl.getUniformLocation(program, "turn");
    const size = gl.getUniformLocation(program, "diameter");
    // Allocation/upload failures set GL error state rather than throwing.
    // Check once during initialization; never synchronize the per-frame path.
    if (gl.getError() !== gl.NO_ERROR) throw new Error("GPU initialization failed");
    return {
      draw(turn, diameter) {
        if (disposed) return;
        gl.viewport(0, 0, diameter, diameter);
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.uniform1f(rotation, turn % 1);
        gl.uniform1f(size, diameter);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
      },
      dispose,
    };
  } catch {
    dispose();
    return null;
  }
}
