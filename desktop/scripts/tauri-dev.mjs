import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  DEV_PORT_ENV,
  DEV_SERVER_HOST,
  PREFERRED_DEV_PORT,
  findLoopbackPort,
} from "./dev-server-port.mjs";

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const tauriEntry = require.resolve("@tauri-apps/cli/tauri.js");
const PORT_ATTEMPTS = 100;

export function shouldSelectDevPort(args) {
  const split = args.indexOf("--");
  const command = split === -1 ? args : args.slice(0, split);
  if (command[0] !== "dev") {
    return false;
  }
  return !command.some((arg) => arg === "-h" || arg === "--help" || arg === "-V" || arg === "--version");
}

export function devUrlConfig(port) {
  return {
    build: {
      devUrl: `http://${DEV_SERVER_HOST}:${port}`,
    },
  };
}

export function withDevConfig(args, configPath) {
  const split = args.indexOf("--");
  const head = split === -1 ? args : args.slice(0, split);
  const tail = split === -1 ? [] : args.slice(split);
  return [...head, "--config", configPath, ...tail];
}

function runTauri(args, env, cleanup) {
  const child = spawn(process.execPath, [tauriEntry, ...args], {
    cwd: desktopRoot,
    env,
    stdio: "inherit",
  });
  const stop = () => {
    if (cleanup) {
      cleanup();
    }
  };
  process.on("exit", stop);
  child.on("error", (error) => {
    stop();
    console.error(error.message);
    process.exit(1);
  });
  child.on("exit", (code, signal) => {
    stop();
    process.exit(signal ? 1 : (code ?? 1));
  });
}

async function main() {
  const args = process.argv.slice(2);
  if (!shouldSelectDevPort(args)) {
    runTauri(args, process.env);
    return;
  }

  const port = await findLoopbackPort(PREFERRED_DEV_PORT, PORT_ATTEMPTS);
  if (port === PREFERRED_DEV_PORT) {
    runTauri(args, process.env);
    return;
  }

  const directory = mkdtempSync(path.join(tmpdir(), "devsweep-desktop-dev-"));
  const configPath = path.join(directory, "dev-url.json");
  writeFileSync(configPath, JSON.stringify(devUrlConfig(port)));
  const cleanup = () => {
    rmSync(directory, { recursive: true, force: true });
  };
  console.error(
    `Port ${PREFERRED_DEV_PORT} is in use. Desktop dev server will use http://${DEV_SERVER_HOST}:${port}.`,
  );
  runTauri(withDevConfig(args, configPath), { ...process.env, [DEV_PORT_ENV]: String(port) }, cleanup);
}

const invokedDirectly = process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
