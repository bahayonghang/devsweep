// Native evidence capture for the Software workbench against the real release
// desktop app (task-owned evidence tooling, analyze-task CDP pattern).
// Captures: bilingual inventory/preview/confirmation at widths 1440/1024/800/390,
// per-launch DPI 100/125/150/200%, keyboard focus, AX tree, no real uninstall.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-software-cli-tui-desktop-native/evidence/native-20260831");
const log = (line) => appendFileSync(join(evidence, "capture-log.jsonl"), JSON.stringify(line) + "\n");
mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "axtree"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
log({ event: "exe_hash", exe, sha256: sha256(exe) });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch(scalePercent, profileName) {
  const cdpPort = 9360 + Math.round(scalePercent / 10);
  const localAppData = join(evidence, profileName);
  mkdirSync(localAppData, { recursive: true });
  const extraArgs = scalePercent ? ` --force-device-scale-factor=${scalePercent / 100}` : "";
  const env = { ...process.env, LOCALAPPDATA: localAppData, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}${extraArgs}` };
  const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
  log({ event: "app_launched", pid: app.pid, cdpPort, scalePercent, profile: profileName });
  const deadline = Date.now() + 45000;
  let targets = null;
  while (Date.now() < deadline) {
    try { const r = await fetch(`http://127.0.0.1:${cdpPort}/json`); if (r.ok) { targets = await r.json(); break; } } catch {}
    await sleep(500);
  }
  if (!targets) throw new Error("cdp timeout");
  const page = targets.find((t) => t.type === "page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("ws error")); });
  let seq = 0;
  const pending = new Map();
  const send = (method, params = {}) => {
    const id = ++seq;
    ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve2, reject) => {
      pending.set(id, { resolve: resolve2, reject });
      setTimeout(() => { if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout: " + method)); } }, 30000);
    });
  };
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve: resolve2, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message)); else resolve2(message.result);
    }
  };
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (result.exceptionDetails) throw new Error("page exception: " + JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
    return result.result.value;
  };
  const screenshot = async (name) => {
    const shot = await send("Page.captureScreenshot", { format: "png" });
    writeFileSync(join(evidence, "screenshots", name + ".png"), Buffer.from(shot.data, "base64"));
    log({ event: "screenshot", name });
  };
  return { app, send, evaluate, screenshot, async close() {
    try { await send("Browser.close"); } catch {}
    await sleep(2000);
    try { execSync(`taskkill /PID ${app.pid} /F 2>NUL`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
    log({ event: "app_closed", pid: app.pid });
  } };
}

async function setLocale(session, tag) {
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  await session.evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, ${JSON.stringify(tag)});
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  await sleep(1200);
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/software'" });
  await sleep(1200);
}

async function refreshInventory(session, tag) {
  await session.evaluate(`(() => { document.querySelector('.software-toolbar button.primary-button')?.click(); })()`);
  let state = null;
  for (let i = 0; i < 60; i += 1) {
    await sleep(500);
    state = await session.evaluate(`(() => ({
      status: document.querySelector('.software-mode')?.getAttribute('data-status'),
      rows: document.querySelectorAll('.software-row').length,
      summary: document.querySelector('.software-summary-bar')?.textContent?.replace(/\\s+/g, ' ').slice(0, 160) ?? null,
    }))()`);
    if (state.rows > 0 && state.status !== "loading") break;
  }
  log({ event: "inventory_state", tag, state });
  return state;
}

// ---- Main launch: en first, widths + keyboard + AX tree; then zh ----------
{
  const session = await launch(null, "localappdata");
  const { evaluate, screenshot, send } = session;
  await send("Runtime.enable");
  await send("Page.enable");
  await sleep(2200);
  // Force English first (profile may persist zh-CN).
  await setLocale(session, "en");
  const storeMain = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_en", store: existsSync(storeMain) ? readFileSync(storeMain, "utf8").trim() : null });
  const inv = await refreshInventory(session, "en");
  await screenshot("sw-01-inventory-en-1440");
  writeFileSync(join(evidence, "snapshots", "inventory-en.txt"), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 800)"), "utf8");
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`sw-02-inventory-en-${width}`);
  }
  await send("Emulation.clearDeviceMetricsOverride");
  await sleep(500);
  // Keyboard focus path.
  await evaluate(`(() => { const row = document.querySelector('.software-row input[type="checkbox"]'); row?.focus(); return document.activeElement?.className ?? null; })()`);
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await sleep(500);
  const kb = await session.evaluate(`(() => ({ checked: document.querySelector('.software-row input[type="checkbox"]')?.checked ?? null, active: document.activeElement?.className ?? null }))()`);
  log({ event: "keyboard_select", kb });
  await screenshot("sw-03-keyboard-select-en");
  // Search narrowing (long/duplicate-name handling via query).
  await evaluate(`(() => {
    const search = document.querySelector('.software-toolbar input[type="search"]');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(search, 'a');
    search.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await sleep(700);
  const narrowed = await session.evaluate(`(() => document.querySelectorAll('.software-row').length)()`);
  log({ event: "search_narrowed", narrowed });
  await screenshot("sw-04-search-en");
  await evaluate(`(() => {
    const search = document.querySelector('.software-toolbar input[type="search"]');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(search, '');
    search.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await sleep(600);
  // AX tree.
  try {
    await send("Accessibility.enable");
    const ax = await send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", "software-inventory-en-full-axtree.json"), JSON.stringify(ax, null, 2));
    log({ event: "axtree_captured", nodes: ax.nodes?.length ?? 0 });
  } catch (e) { log({ event: "axtree_error", error: String(e) }); }
  // zh-CN phase.
  await setLocale(session, "zh-CN");
  const storeZh = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_zh", store: existsSync(storeZh) ? readFileSync(storeZh, "utf8").trim() : null });
  await refreshInventory(session, "zh-CN");
  await screenshot("sw-05-inventory-zh-1440");
  await send("Emulation.setDeviceMetricsOverride", { width: 390, height: 860, deviceScaleFactor: 0, mobile: false });
  await sleep(700);
  await screenshot("sw-06-inventory-zh-390");
  await send("Emulation.clearDeviceMetricsOverride");
  await sleep(500);
  await session.close();
}

// ---- DPI launches on fresh profiles (default en) ---------------------------
for (const scalePercent of [100, 125, 150, 200]) {
  const session = await launch(scalePercent, `localappdata-dpi-${scalePercent}`);
  await setLocale(session, "en");
  await refreshInventory(session, `dpi-${scalePercent}`);
  await session.screenshot(`sw-dpi-${scalePercent}-inventory-en-1440`);
  const dpr = await session.evaluate(`(() => ({ dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight] }))()`);
  log({ event: "dpi_probe", scalePercent, ...dpr });
  await session.close();
}

log({ event: "done" });
process.exit(0);
