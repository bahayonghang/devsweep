import { spawn } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "node:net";

const here = dirname(fileURLToPath(import.meta.url));
const webDir = resolve(here, "../web");
const desktopRoot = resolve(here, "../../../../../desktop");
const origin = "http://127.0.0.1:4180";
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => error ? reject(error) : resolvePort(port));
    });
  });
}

async function waitHttp(url) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { redirect: "manual" });
      if (response.ok || response.status === 304) return;
    } catch { /* retry */ }
    await sleep(300);
  }
  throw new Error("timeout waiting for " + url);
}

async function connectCdp(port) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find((target) => target.type === "page" && /127\.0\.0\.1:4180/.test(target.url || ""))
          ?? targets.find((target) => target.type === "page");
        if (page?.webSocketDebuggerUrl) {
          const ws = new WebSocket(page.webSocketDebuggerUrl);
          await new Promise((open, fail) => {
            ws.onopen = open;
            ws.onerror = () => fail(new Error("ws error"));
          });
          let seq = 0;
          const pending = new Map();
          ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            if (message.id && pending.has(message.id)) {
              const wait = pending.get(message.id);
              pending.delete(message.id);
              if (message.error) wait.reject(new Error(message.error.message));
              else wait.resolve(message.result);
            }
          };
          const send = (method, params = {}) => {
            const id = ++seq;
            ws.send(JSON.stringify({ id, method, params }));
            return new Promise((resolveSend, rejectSend) => {
              pending.set(id, { resolve: resolveSend, reject: rejectSend });
              setTimeout(() => {
                if (pending.has(id)) {
                  pending.delete(id);
                  rejectSend(new Error("cdp timeout " + method));
                }
              }, 30000);
            });
          };
          await send("Runtime.enable");
          await send("Page.enable");
          return { send, ws };
        }
      }
    } catch { /* retry */ }
    await sleep(400);
  }
  throw new Error("cdp timeout port " + port);
}

const chrome = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].find((path) => existsSync(path));
if (!chrome) throw new Error("no Chrome/Edge");

let vite = null;
try {
  await fetch(origin, { signal: AbortSignal.timeout(2000) });
} catch {
  process.stdout.write("starting fixture vite\n");
  vite = spawn("mise", ["exec", "node@22", "--", "npm", "run", "dev:fixture"], {
    cwd: desktopRoot,
    stdio: ["ignore", "pipe", "pipe"],
  });
  await waitHttp(origin);
  process.stdout.write("vite ready\n");
}

const port = await freePort();
const browser = spawn(chrome, [
  "--headless=new",
  `--remote-debugging-port=${port}`,
  "--remote-allow-origins=*",
  `--user-data-dir=${resolve(process.env.TEMP || ".", `devsweep-narrow-${Date.now()}`)}`,
  "--no-first-run",
  `${origin}/#/clean`,
], { stdio: ["ignore", "pipe", "pipe"] });

try {
  const session = await connectCdp(port);
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    const ready = await session.send("Runtime.evaluate", { expression: "Boolean(document.querySelector('.app-shell'))", returnByValue: true });
    if (ready.result.value) break;
    await sleep(200);
  }
  for (const [width, height] of [[390, 844], [800, 720]]) {
    await session.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width <= 430 });
    await sleep(300);
    const shot = await session.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    const file = resolve(webDir, `en-clean-${width}.png`);
    writeFileSync(file, Buffer.from(shot.data, "base64"));
    process.stdout.write(`wrote ${file}\n`);
  }
  session.ws.close();
} finally {
  browser.kill();
  if (vite) vite.kill();
}
