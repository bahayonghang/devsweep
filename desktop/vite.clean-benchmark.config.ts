import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

function sources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? sources(path) : /\.(tsx?|css|json)$/.test(path) ? [path] : [];
  }).sort();
}

export default defineConfig(({ mode }) => {
  if (!/^[a-zA-Z0-9_-]+$/.test(mode)) throw new Error("Invalid benchmark build label");
  const hash = createHash("sha256");
  for (const path of sources("src")) hash.update(path.replaceAll("\\", "/")).update("\0").update(readFileSync(path));
  return {
    base: "./", plugins: [react()],
    define: { __CLEAN_BUILD__: JSON.stringify({ label: mode, source_sha256: hash.digest("hex") }) },
    build: { outDir: `dist/clean-${mode}`, emptyOutDir: false, rollupOptions: { input: "clean-render-benchmark.html" } },
  };
});
