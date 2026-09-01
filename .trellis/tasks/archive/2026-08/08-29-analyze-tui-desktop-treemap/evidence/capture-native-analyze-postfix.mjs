// Post-fix native evidence capture: fresh screenshots after the independent
// checker's fixes, including the OS-DPI-scale matrix (renderer-level
// deviceScaleFactor emulation of 100/125/150/200% on the real desktop app).
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-20260831-postfix");
const fixtureRoot = join(evidence, "fixture");
const cdpPort = 9348;
const log = (line) => appendFileSync(join(evidence, "capture-log.jsonl"), JSON.stringify(line) + "\n");
mkdirSync(join(evidence, "localappdata"), { recursive: true });
mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });
mkdirSync(join(fixtureRoot, "src", "small"), { recursive: true });

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
log({ event: "exe_hash", exe, sha256: sha256(exe) });

// Fixture: 120 small files + a wide 3000-file directory + an ACL-denied dir.
if (!existsSync(join(fixtureRoot, "big", "gen"))) {
  mkdirSync(join(fixtureRoot, "big", "gen"), { recursive: true });
  for (let i = 0; i < 3000; i += 1) writeFileSync(join(fixtureRoot, "big", "gen", `gen-${String(i).padStart(4, "0")}.dat`), Buffer.alloc(64, 121));
  for (let i = 0; i < 120; i += 1) writeFileSync(join(fixtureRoot, "src", "small", `file-${String(i).padStart(3, "0")}.bin`), Buffer.alloc(i + 1, 120));
}
const lockedDir = join(fixtureRoot, "src", "locked");
mkdirSync(lockedDir, { recursive: true });
if (!existsSync(join(lockedDir, "hidden.bin"))) writeFileSync(join(lockedDir, "hidden.bin"), Buffer.alloc(4096, 7));
execSync(`icacls "${lockedDir}" /deny "*S-1-5-11:(OI)(CI)F" /inheritance:r`, { shell: "cmd.exe", stdio: "pipe" });
log({ event: "acl_deny_set" });

const env = { ...process.env, LOCALAPPDATA: join(evidence, "localappdata"), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}` };
const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
log({ event: "app_launched", pid: app.pid, cdpPort });

async function waitHttp(url, deadlineMs) {
  const deadline = Date.now() + deadlineMs;
  while (Date.now() < deadline) {
    try { const r = await fetch(url); if (r.ok) return await r.json(); } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("timeout: " + url);
}
const page = (await waitHttp(`http://127.0.0.1:${cdpPort}/json`, 45000)).find((t) => t.type === "page");
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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function startAnalysis(rootPath) {
  await evaluate(`(() => {
    const input = document.querySelector('.analyze-root-input input');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(input, ${JSON.stringify(rootPath.split(/\\/g).join("/"))});
    input.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await sleep(250);
  await evaluate(`(() => { document.querySelector('.analyze-toolbar button.primary-button').click(); })()`);
  log({ event: "analyze_start_clicked", rootPath });
}
async function waitForWorkspace() {
  for (let i = 0; i < 50; i += 1) {
    const state = await evaluate(`(() => ({
      hasWorkspace: Boolean(document.querySelector('.analyze-workspace')),
      summary: document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 120) ?? null,
    }))()`);
    if (state.hasWorkspace && state.summary && /complete|完成/i.test(state.summary)) return state;
    await sleep(400);
  }
  return null;
}

await send("Runtime.enable");
await send("Page.enable");
await sleep(2500);
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1200);
await startAnalysis(fixtureRoot);
const complete = await waitForWorkspace();
log({ event: "complete_state", complete });
if (!complete) throw new Error("workspace missing");
await screenshot("postfix-01-analyze-complete-en-1440");

// Keyboard drill with focus restoration probe (post-fix behavior).
const kb = await evaluate(`(() => {
  const tile = Array.from(document.querySelectorAll('.analyze-tile')).find((t) => (t.getAttribute('aria-label') ?? '').startsWith('big')) ?? document.querySelector('.analyze-tile');
  if (!tile) return { ok: false };
  tile.focus();
  return { ok: true, label: tile.getAttribute('aria-label'), before: document.activeElement?.getAttribute?.('aria-label') ?? document.activeElement?.tagName };
})()`);
await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await sleep(1000);
const kbAfter = await evaluate(`(() => ({
  active: document.activeElement?.getAttribute?.('aria-label') ?? document.activeElement?.tagName,
  activeClass: document.activeElement?.className ?? null,
  crumbs: document.querySelectorAll('.analyze-breadcrumbs button').length,
}))()`);
log({ event: "keyboard_drill_postfix", kb, kbAfter });
await screenshot("postfix-02-analyze-keyboard-drill-en");
await evaluate(`(() => { document.querySelector('.analyze-breadcrumbs button')?.click(); })()`);
await sleep(700);

// OS DPI scaling matrix: 100/125/150/200% via renderer deviceScaleFactor.
for (const [label, dsf] of [["100", 1], ["125", 1.25], ["150", 1.5], ["200", 2]]) {
  await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 860, deviceScaleFactor: dsf, mobile: false });
  await sleep(700);
  await screenshot(`postfix-03-analyze-complete-en-dpi-${label}`);
}
await send("Emulation.clearDeviceMetricsOverride");
await sleep(500);

// Warnings list render (drill into src).
await evaluate(`(() => {
  const tile = Array.from(document.querySelectorAll('.analyze-tile')).find((t) => (t.getAttribute('aria-label') ?? '').startsWith('src'));
  tile?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true, view: window }));
})()`);
await sleep(1000);
const listProbe = await evaluate(`(() => ({
  options: Array.from(document.querySelectorAll('.analyze-listbox option')).map((o) => o.textContent.trim()).slice(0, 8),
}))()`);
log({ event: "list_probe_postfix", listProbe });
await screenshot("postfix-04-analyze-src-warnings-en");
await evaluate(`(() => { document.querySelector('.analyze-breadcrumbs button')?.click(); })()`);
await sleep(600);

// Reduced motion + forced colors (fresh).
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
await sleep(400);
await screenshot("postfix-05-analyze-reduced-motion-en");
await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }, { name: "prefers-contrast", value: "more" }] });
await sleep(400);
await screenshot("postfix-06-analyze-forced-colors-en");
await send("Emulation.setEmulatedMedia", { features: [] });

// zh-CN complete state (app restores persisted language; this profile starts en).
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
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1200);
await startAnalysis(fixtureRoot);
const completeZh = await waitForWorkspace();
log({ event: "complete_state_zh", completeZh });
await screenshot("postfix-07-analyze-complete-zh-1440");

// Cancel-on-leave (fresh).
await startAnalysis(join(repo, "target"));
await sleep(600);
const canceling = await evaluate(`(() => ({ status: document.querySelector('.analyze-live-status')?.textContent ?? null, cancelVisible: Boolean(document.querySelector('.analyze-toolbar button.secondary-button')) }))()`);
log({ event: "canceling_state", canceling });
await screenshot("postfix-08-analyze-canceling-en");
await send("Runtime.evaluate", { expression: "location.hash = '#/clean'" });
await sleep(2500);
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1500);
const returnState = await evaluate(`(() => ({ empty: Boolean(document.querySelector('.analyze-empty')) }))()`);
log({ event: "return_after_cancel", returnState });
await screenshot("postfix-09-analyze-after-cancel-return");

try { await send("Browser.close"); } catch (e) { log({ event: "browser_close_error", error: String(e) }); }
await sleep(2500);
try { execSync(`taskkill /PID ${app.pid} /F 2>NUL`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
try { execSync(`icacls "${lockedDir}" /reset`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
log({ event: "done" });
process.exit(0);
