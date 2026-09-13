import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:net";
import { existsSync, mkdirSync, writeFileSync, appendFileSync, readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const evidenceRoot = resolve(here, "..");
const repoRoot = resolve(here, "../../../../../");
const desktopRoot = resolve(repoRoot, "desktop");
const webDir = resolve(evidenceRoot, "web");
const nativeDir = resolve(evidenceRoot, "native");
const logPath = resolve(evidenceRoot, "capture-log.jsonl");

const ROUTES = ["clean", "software", "optimize", "analyze", "status", "protection", "rules", "history"];
const WIDTHS = [390, 800, 1024, 1440];
const LOAD_CLICK = {
  software: /Refresh inventory|刷新软件清单/,
  optimize: /Reload catalogue|重新加载目录/,
  analyze: /Analyze path|分析路径/,
};

const chromeCandidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
];

const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));

function log(event, extra = {}) {
  appendFileSync(logPath, JSON.stringify({ event, ...extra, at: new Date().toISOString() }) + "\n");
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => {
        if (error) reject(error);
        else resolvePort(port);
      });
    });
  });
}

async function waitHttp(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { redirect: "manual" });
      if (response.ok || response.status === 304) return;
    } catch {
      // retry
    }
    await sleep(300);
  }
  throw new Error("timeout waiting for " + url);
}

async function connectCdp(port, urlMatch, fallback = true) {
  const deadline = Date.now() + 60000;
  let targets = null;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        targets = await response.json();
        const urls = (targets || []).map((target) => ({ type: target.type, url: target.url }));
        if (JSON.stringify(urls) !== connectCdp.lastUrls) {
          connectCdp.lastUrls = JSON.stringify(urls);
          log("cdp_targets", { port, urls });
        }
        const page = targets.find((target) => target.type === "page" && urlMatch.test(target.url || ""))
          ?? (fallback ? targets.find((target) => target.type === "page") : null);
        if (page?.webSocketDebuggerUrl) {
          const ws = new WebSocket(page.webSocketDebuggerUrl);
          await new Promise((resolveOpen, reject) => {
            ws.onopen = resolveOpen;
            ws.onerror = () => reject(new Error("ws error"));
          });
          let seq = 0;
          const pending = new Map();
          ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            if (message.id && pending.has(message.id)) {
              const { resolve: resolvePending, reject: rejectPending } = pending.get(message.id);
              pending.delete(message.id);
              if (message.error) rejectPending(new Error(message.error.message));
              else resolvePending(message.result);
            }
          };
          const send = (method, params = {}, timeoutMs = 30000) => {
            const id = ++seq;
            ws.send(JSON.stringify({ id, method, params }));
            return new Promise((resolveSend, rejectSend) => {
              pending.set(id, { resolve: resolveSend, reject: rejectSend });
              setTimeout(() => {
                if (pending.has(id)) {
                  pending.delete(id);
                  rejectSend(new Error("cdp timeout " + method));
                }
              }, timeoutMs);
            });
          };
          const evaluate = async (expression) => {
            const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
            if (result.exceptionDetails) throw new Error(result.exceptionDetails.text || "evaluate failed");
            return result.result.value;
          };
          await send("Runtime.enable");
          await send("Page.enable");
          return { send, evaluate, ws, target: page };
        }
      }
    } catch {
      // retry
    }
    await sleep(400);
  }
  throw new Error("cdp timeout port " + port);
}

async function setViewport(session, width, height) {
  await session.send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: width <= 430,
  });
}

async function screenshot(session, filePath) {
  const result = await session.send("Page.captureScreenshot", { format: "png", fromSurface: true });
  writeFileSync(filePath, Buffer.from(result.data, "base64"));
}

async function setHash(session, hash) {
  await session.evaluate(`location.hash = ${JSON.stringify(hash)}`);
  await sleep(700);
}

async function clickNamed(session, pattern) {
  return session.evaluate(`(() => {
    const re = ${pattern.toString()};
    const button = Array.from(document.querySelectorAll("button")).find((node) => re.test(node.textContent || ""));
    if (button) button.click();
    return !!button;
  })()`);
}

async function waitForShell(session) {
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    const ready = await session.evaluate("Boolean(document.querySelector('.app-shell'))");
    if (ready) return;
    await sleep(200);
  }
  throw new Error("app-shell did not mount");
}

async function setLocale(session, locale) {
  await setHash(session, "#/settings");
  const changed = await session.evaluate(`(() => {
    const select = document.querySelector(".settings-panel select");
    if (!select) return false;
    const descriptor = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value");
    descriptor.set.call(select, ${JSON.stringify(locale)});
    select.dispatchEvent(new Event("input", { bubbles: true }));
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  })()`);
  await sleep(900);
  const current = await session.evaluate("document.querySelector('.app-shell')?.getAttribute('data-locale')");
  return { changed, current };
}

async function loadRoute(session, route) {
  await setHash(session, `#/${route}`);
  const pattern = LOAD_CLICK[route];
  if (pattern) {
    const clicked = await clickNamed(session, pattern);
    log("load_click", { route, clicked });
    await sleep(1200);
  } else {
    await sleep(800);
  }
}

async function captureWeb() {
  mkdirSync(webDir, { recursive: true });
  const origin = "http://127.0.0.1:4180";
  let vite = null;
  let served = false;
  try {
    const probe = await fetch(origin);
    served = probe.ok;
  } catch {
    served = false;
  }
  if (!served) {
    vite = spawn("mise", ["exec", "node@22", "--", "npm", "run", "dev:fixture"], {
      cwd: desktopRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    log("vite_spawn", { pid: vite.pid });
    await waitHttp(origin, 30000);
    log("vite_ready", { origin });
  } else {
    log("vite_reused", { origin });
  }

  const binary = chromeCandidates.find((path) => existsSync(path));
  if (!binary) throw new Error("no Chrome/Edge binary found");
  const port = await freePort();
  const profile = resolve(process.env.TEMP || ".", `devsweep-sidebar-chrome-${Date.now()}`);
  const chrome = spawn(binary, [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    "--remote-allow-origins=*",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-sync",
    "--window-size=1440,900",
    `${origin}/#/clean`,
  ], { stdio: ["ignore", "pipe", "pipe"] });
  log("chrome_spawn", { pid: chrome.pid, port, binary });

  try {
    const session = await connectCdp(port, /127\.0\.0\.1:4180/);
    await waitForShell(session);
    log("shell_ready", { url: session.target.url });

    const locales = ["en", "zh-CN"];
    for (const locale of locales) {
      const localeResult = await setLocale(session, locale);
      log("locale", { requested: locale, ...localeResult });
      if (localeResult.current !== locale) {
        log("locale_unverified", { requested: locale, current: localeResult.current });
      }
      const prefix = locale === "zh-CN" ? "zh" : "en";
      for (const route of ROUTES) {
        await loadRoute(session, route);
        for (const width of WIDTHS) {
          const height = width <= 390 ? 844 : width <= 800 ? 720 : 900;
          await setViewport(session, width, height);
          await sleep(250);
          const file = resolve(webDir, `${prefix}-${route}-${width}.png`);
          await screenshot(session, file);
          log("screenshot", { file, locale, route, width, height });
        }
      }
    }

    await setLocale(session, "en");
    await setViewport(session, 1440, 900);
    await session.send("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-reduced-motion", value: "reduce" }],
    });
    for (const route of ROUTES) {
      await loadRoute(session, route);
      const file = resolve(webDir, `en-${route}-reduced-motion.png`);
      await screenshot(session, file);
      log("screenshot", { file, locale: "en", route, media: "prefers-reduced-motion" });
    }
    await session.send("Emulation.setEmulatedMedia", {
      features: [{ name: "forced-colors", value: "active" }],
    });
    for (const route of ROUTES) {
      await loadRoute(session, route);
      const file = resolve(webDir, `en-${route}-forced-colors.png`);
      await screenshot(session, file);
      log("screenshot", { file, locale: "en", route, media: "forced-colors" });
    }
    await session.send("Emulation.setEmulatedMedia", { features: [] });

    await loadRoute(session, "clean");
    await session.evaluate(`document.querySelector('.mode-tab')?.focus()`);
    const keyboard = [];
    for (let step = 0; step < 16; step += 1) {
      const info = await session.evaluate(`(() => {
        const el = document.activeElement;
        return {
          tag: el?.tagName ?? null,
          role: el?.getAttribute("role"),
          name: (el?.getAttribute("aria-label") || el?.textContent || "").trim().slice(0, 80),
          className: typeof el?.className === "string" ? el.className : "",
        };
      })()`);
      keyboard.push({ step, ...info });
      await session.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
      await session.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
      await sleep(80);
    }
    writeFileSync(resolve(webDir, "keyboard-walk.json"), JSON.stringify(keyboard, null, 2));
    log("keyboard_walk", { steps: keyboard.length });
    session.ws.close();
  } finally {
    try { chrome.kill(); } catch { /* ignore */ }
    if (vite) {
      try { vite.kill(); } catch { /* ignore */ }
    }
  }
}

async function captureNative() {
  mkdirSync(nativeDir, { recursive: true });
  const exe = resolve(repoRoot, "target/release/devsweep-desktop.exe");
  if (!existsSync(exe)) {
    log("native_build_start", { exe });
    const build = spawn("cargo", ["build", "-p", "devsweep-desktop", "--release"], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stderr = "";
    build.stderr.on("data", (chunk) => { stderr += String(chunk).slice(-4000); });
    const code = await new Promise((resolveExit) => build.on("exit", resolveExit));
    if (code !== 0) {
      log("native_build_failed", { code, stderr: stderr.slice(-2000) });
      writeFileSync(resolve(nativeDir, "UNVERIFIED.json"), JSON.stringify({
        status: "UNVERIFIED",
        reason: `cargo build -p devsweep-desktop --release exited ${code}`,
        stderr: stderr.slice(-2000),
      }, null, 2));
      return { status: "UNVERIFIED", reason: `cargo build exited ${code}` };
    }
  }
  const hash = createHash("sha256").update(readFileSync(exe)).digest("hex");
  writeFileSync(resolve(nativeDir, "binary-sha256.txt"), hash + "\n");
  log("native_binary", { exe, sha256: hash });

  const port = await freePort();
  const profile = resolve(evidenceRoot, "native-localappdata");
  mkdirSync(profile, { recursive: true });
  const app = spawn(exe, [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      LOCALAPPDATA: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-allow-origins=*`,
    },
    stdio: "ignore",
    detached: true,
    windowsHide: false,
  });
  app.unref();
  log("native_launch", { pid: app.pid, port });
  try {
    const session = await connectCdp(port, /tauri\.localhost/, false);
    const href = await session.evaluate("location.href").catch((error) => String(error));
    log("native_page", { href, title: session.target.title, url: session.target.url });
    await waitForShell(session);
    for (const [width, height] of [[1080, 720], [900, 600]]) {
      await setViewport(session, width, height);
      await sleep(400);
      for (const route of ROUTES) {
        await setHash(session, `#/${route}`);
        const file = resolve(nativeDir, `native-${route}-${width}x${height}.png`);
        await screenshot(session, file);
        log("native_screenshot", { file, route, width, height });
      }
    }
    try { await session.send("Browser.close"); } catch { /* ignore */ }
    session.ws.close();
    return { status: "PASS", sha256: hash };
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    log("native_failed", { reason });
    writeFileSync(resolve(nativeDir, "UNVERIFIED.json"), JSON.stringify({
      status: "UNVERIFIED",
      reason,
    }, null, 2));
    try { process.kill(app.pid); } catch { /* ignore */ }
    return { status: "UNVERIFIED", reason };
  }
}

mkdirSync(evidenceRoot, { recursive: true });
if (!existsSync(logPath)) writeFileSync(logPath, "");
const mode = process.argv[2] || "web";
if (mode === "web") {
  await captureWeb();
  log("web_complete", {});
} else if (mode === "native") {
  const result = await captureNative();
  log("native_complete", result);
} else {
  throw new Error("usage: capture.mjs web|native");
}
