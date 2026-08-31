// Evidence-only Vitest configuration for the managed sandbox. The ordinary
// project config remains authoritative. This substitutes TypeScript's in-process
// transpiler for esbuild because Node child-process creation is denied here.
import { resolve } from "node:path";
import { createRequire } from "node:module";
import { defineConfig } from "../../../../desktop/node_modules/vitest/dist/config.js";

const desktopRoot = resolve(import.meta.dirname, "../../../../desktop");
const require = createRequire(import.meta.url);
const ts = require(resolve(desktopRoot, "node_modules/typescript/lib/typescript.js"));

export default defineConfig({
  root: desktopRoot,
  esbuild: false,
  plugins: [{
    name: "sandbox-typescript-transpile",
    enforce: "pre",
    transform(code, id) {
      if (!/[\\/]desktop[\\/]src[\\/].*\.tsx?$/.test(id)) return null;
      const result = ts.transpileModule(code, {
        fileName: id,
        compilerOptions: {
          target: 9,
          module: 99,
          moduleResolution: 100,
          jsx: 4,
          resolveJsonModule: true,
          esModuleInterop: true,
          sourceMap: false,
        },
      });
      return { code: result.outputText, map: result.sourceMapText ?? null };
    },
  }],
  test: {
    environment: "jsdom",
    setupFiles: resolve(desktopRoot, "src/test/setup.ts"),
  },
});
