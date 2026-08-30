// Native evidence driver for the Analyze mode against the real release Tauri
// desktop app (task-owned evidence tooling). Second revision with product-exact
// selectors: .analyze-root-input input, .analyze-toolbar .primary-button,
// .analyze-tile SVG rects, .analyze-breadcrumbs buttons, settings <select>.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-20260831");
const fixtureRoot = join(evidence, "fixture");
const cdpPort = 9346;
const log = (line) => appendFileSync(join(evidence, "capture-log.jsonl"), JSON.stringify(line) + "\n");

mkdirSync(join(evidence, "localappdata"), { recursive: true });
mkdirSync(join(evidence, "screenshots"), { recursive: true });

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}
const exeHash = sha256(exe);
log({ event: "exe_hash", exe, sha256: exeHash });

// Deny ACL on the locked subtree so the real core records access-denied
// warnings and the snapshot surfaces a partial/permission state.
const lockedDir = join(fixtureRoot, "src", "locked");
if (!existsSync(join(lockedDir, "hidden.bin"))) writeFileSync(join(lockedDir, "hidden.bin"), Buffer.alloc(4096, 7));
execSync(`icacls "${lockedDir}" /deny "*S-1-5-11:(OI)(CI)F" /inheritance:r`, { shell: "cmd.exe", stdio: "pipe" });
log({ event: "acl_deny_set", dir: lockedDir });

const env = {
  ...process.env,
  LOCALAPPDATA: join(evidence, "localappdata"),
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
};
const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
log({ event: "app_launched", pid: app.pid, cdpPort });

async function waitHttp(url, deadlineMs) {
  const deadline = Date.now() + deadlineMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return await response.json();
    } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("timeout waiting for " + url);
}

const targets = await waitHttp(`http://127.0.0.1:${cdpPort}/json`, 45000);
const page = targets.find((t) => t.type === "page");
log({ event: "cdp_targets", count: targets.length, pageUrl: page?.url });

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("ws error")); });
let seq = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++seq;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve2, reject) => {
    pending.set(id, { resolve: resolve2, reject });
    setTimeout(() => { if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout: " + method)); } }, 30000);
  });
}
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    const { resolve: resolve2, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message)); else resolve2(message.result);
  }
};
async function evaluate(expression) {
  const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: false });
  if (result.exceptionDetails) throw new Error("page exception: " + JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
  return result.result.value;
}
async function screenshot(name) {
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(evidence, "screenshots", name + ".png"), Buffer.from(shot.data, "base64"));
  log({ event: "screenshot", name });
}
async function snapshotText(name) {
  const text = await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 600)");
  writeFileSync(join(evidence, "snapshots", name + ".txt"), text, "utf8");
  return text;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function startAnalysis(rootPath) {
  const probe = await evaluate(`(() => ({
    input: Boolean(document.querySelector('.analyze-root-input input')),
    start: Array.from(document.querySelectorAll('button')).map((b) => b.textContent.trim()),
  }))()`);
  log({ event: "analyze_probe", probe });
  await evaluate(`(() => {
    const input = document.querySelector('.analyze-root-input input');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(input, ${JSON.stringify(rootPath.split(/\\/g).join("/"))});
    input.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await sleep(250);
  const clicked = await evaluate(`(() => {
    const button = document.querySelector('.analyze-toolbar button.primary-button');
    if (!button || button.disabled) return { clicked: false, disabled: button ? button.disabled : null };
    button.click();
    return { clicked: true };
  })()`);
  log({ event: "analyze_start_clicked", clicked, rootPath });
  return clicked;
}

async function waitForWorkspace(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const state = await evaluate(`(() => ({
      hasWorkspace: Boolean(document.querySelector('.analyze-workspace')),
      tiles: document.querySelectorAll('.analyze-tile').length,
      status: document.querySelector('.analyze-live-status')?.textContent ?? null,
      summary: document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 200) ?? null,
    }))()`);
    if (state.hasWorkspace && state.summary && !/analyz/i.test(state.status ?? "")) return state;
    await sleep(400);
  }
  return null;
}

await send("Runtime.enable");
await send("Page.enable");
await sleep(2500);

// ---- Round A: complete state, en, four widths ----------------------------
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1200);
await startAnalysis(fixtureRoot);
const complete = await waitForWorkspace(30000);
log({ event: "complete_state", complete });
if (!complete) throw new Error("analyze workspace did not render");
await snapshotText("complete-en-1440");
await screenshot("01-analyze-complete-en-1440");
for (const width of [1024, 800, 390]) {
  await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
  await sleep(700);
  await screenshot(`02-analyze-complete-en-${width}`);
}
await send("Emulation.clearDeviceMetricsOverride");
await sleep(500);

// ---- Keyboard drill-down --------------------------------------------------
const keyboardBefore = await evaluate(`(() => {
  const tile = document.querySelector('.analyze-tile[role=\"button\"]');
  if (!tile) return { ok: false };
  tile.focus();
  return { ok: true, label: tile.getAttribute('aria-label') };
})()`);
await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await sleep(900);
const keyboardAfter = await evaluate(`(() => ({
  focused: document.activeElement?.getAttribute?.('aria-label') ?? document.activeElement?.tagName,
  crumbCount: document.querySelectorAll('.analyze-breadcrumbs button').length,
}))()`);
log({ event: "keyboard_drill", keyboardBefore, keyboardAfter });
await snapshotText("keyboard-drill-en");
await screenshot("03-analyze-keyboard-drill-en");
await evaluate(`(() => { document.querySelector('.analyze-breadcrumbs button')?.click(); })()`);
await sleep(600);

// ---- Reduced motion + high contrast emulation -----------------------------
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
await sleep(400);
await screenshot("04-analyze-reduced-motion-en");
await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }, { name: "prefers-contrast", value: "more" }] });
await sleep(400);
await screenshot("05-analyze-forced-colors-en");
await send("Emulation.setEmulatedMedia", { features: [] });

// ---- Partial / warning state ---------------------------------------------
const warningProbe = await evaluate(`(() => {
  const text = document.body.textContent ?? '';
  return {
    hasPartial: /partial|lower bound/i.test(text),
    hasDenied: /denied|permission/i.test(text),
    hasUnknown: /unknown/i.test(text),
    summary: document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 300) ?? null,
  };
})()`);
log({ event: "warning_probe", warningProbe });
await screenshot("06-analyze-partial-warnings-en");

// ---- Language switch to zh-CN via settings select -------------------------
await send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
await sleep(900);
await evaluate(`(() => {
  const select = document.querySelector('.settings-panel select');
  const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
  setter.call(select, 'zh-CN');
  select.dispatchEvent(new Event('change', { bubbles: true }));
  return select.value;
})()`);
await sleep(1500);
const storePath = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
log({ event: "store_audit", exists: existsSync(storePath), content: existsSync(storePath) ? readFileSync(storePath, "utf8").trim() : null });
await screenshot("07-settings-zh-CN");
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1200);
await startAnalysis(fixtureRoot);
const completeZh = await waitForWorkspace(30000);
log({ event: "complete_state_zh", completeZh });
await snapshotText("complete-zh-1440");
await screenshot("08-analyze-complete-zh-1440");
await send("Emulation.setDeviceMetricsOverride", { width: 390, height: 860, deviceScaleFactor: 0, mobile: false });
await sleep(700);
await screenshot("09-analyze-complete-zh-390");
await send("Emulation.clearDeviceMetricsOverride");
await sleep(400);

// ---- Cancel-on-leave against a large real tree ----------------------------
await startAnalysis(join(repo, "target"));
await sleep(600);
const cancelingState = await evaluate(`(() => ({
  status: document.querySelector('.analyze-live-status')?.textContent ?? null,
  cancelVisible: Boolean(document.querySelector('.analyze-toolbar button.secondary-button')),
}))()`);
log({ event: "canceling_state", cancelingState });
await screenshot("10-analyze-canceling-en");
await send("Runtime.evaluate", { expression: "location.hash = '#/clean'" });
await sleep(2500);
const leaveState = await evaluate(`(() => ({ hash: location.hash }))()`);
log({ event: "left_mode", leaveState });
await screenshot("11-after-leave-mode");
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1500);
const returnState = await evaluate(`(() => ({
  empty: Boolean(document.querySelector('.analyze-empty')),
  status: document.querySelector('.analyze-live-status')?.textContent ?? null,
}))()`);
log({ event: "return_after_cancel", returnState });
await screenshot("12-analyze-after-cancel-return");

// ---- Graceful close and residue check -------------------------------------
try { await send("Browser.close"); } catch (e) { log({ event: "browser_close_error", error: String(e) }); }
await sleep(2500);
let exited = true;
try {
  const out = execSync(`tasklist /FI "PID eq ${app.pid}" /NH`, { encoding: "utf8" });
  exited = !out.includes(String(app.pid));
} catch { exited = true; }
log({ event: "app_close", pid: app.pid, exited });
if (!exited) { try { execSync(`taskkill /PID ${app.pid} /F`, { stdio: "pipe" }); log({ event: "forced_kill", pid: app.pid }); } catch {} }

// Restore access to the locked dir so the evidence tree stays deletable.
try { execSync(`icacls "${lockedDir}" /reset`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
log({ event: "done" });
process.exit(0);
