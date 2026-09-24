import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { DEV_SERVER_HOST, devServerPortFromEnv } from "./scripts/dev-server-port.mjs";

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        hud: fileURLToPath(new URL("./hud.html", import.meta.url)),
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
  },
  clearScreen: false,
  server: {
    host: DEV_SERVER_HOST,
    // scripts/tauri-dev.mjs sets this when the preferred port is already taken.
    port: devServerPortFromEnv(process.env),
    strictPort: true,
  },
});
