import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const baseline = process.env.PLANET_BASELINE_REF ?? "e75a29d";
const sources = ["Planet.tsx", "planet-renderer.ts", "planet-palettes.ts"];

export default defineConfig(({ mode }) => {
  const original = mode === "baseline";
  const readSource = (name: string) => original
    ? execFileSync("git", ["show", `${baseline}:desktop/src/stage/${name}`], { encoding: "utf8" })
    : readFileSync(`src/stage/${name}`, "utf8");
  const sourceHash = createHash("sha256").update(sources.map(readSource).join("\n") + (original ? "" : readFileSync("src/stage/earth-renderer.ts", "utf8") + readFileSync("src/stage/earth-texture.ts", "utf8") + readFileSync("src/lifecycle.ts", "utf8"))).digest("hex");
  return {
    base: "./",
    plugins: [{
      name: "preserved-planet-comparison",
      enforce: "pre",
      load(id) {
        const file = sources.find((name) => id.replaceAll("\\", "/").endsWith(`/stage/${name}`));
        if (file && original) return readSource(file);
      },
    }, react()],
    define: { __PLANET_BUILD__: JSON.stringify({ mode, baseline: original ? baseline : null, source_sha256: sourceHash }) },
    build: { outDir: `dist/planet-${mode}`, emptyOutDir: true, rollupOptions: { input: "planet-render-benchmark.html" } },
    preview: { host: "127.0.0.1", port: 4182, strictPort: true },
  };
});
