// Supplementary native capture: drill into the src directory so the list shows
// the access-denied locked child with its warning/incomplete evidence.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-20260831");
const fixtureRoot = join(evidence, "fixture");
const cdpPort = 9347;
const log = (line) => appendFileSync(join(evidence, "capture-log.jsonl"), JSON.stringify(line) + "\n");

const lockedDir = join(fixtureRoot, "src", "locked");
execSync(`icacls "${lockedDir}" /deny "*S-1-5-11:(OI)(CI)F" /inheritance:r`, { shell: "cmd.exe", stdio: "pipe" });
log({ event: "acl_deny_set", dir: lockedDir });

const env = { ...process.env, LOCALAPPDATA: join(evidence, "localappdata"), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}` };
const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
log({ event: "app_launched", pid: app.pid, cdpPort, exe_sha256: sha256(exe) });

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

await send("Runtime.enable");
await send("Page.enable");
await sleep(2500);
await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
await sleep(1200);
await evaluate(`(() => {
  const input = document.querySelector('.analyze-root-input input');
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
  setter.call(input, ${JSON.stringify(fixtureRoot.split(/\\/g).join("/"))});
  input.dispatchEvent(new Event('input', { bubbles: true }));
})()`);
await sleep(250);
await evaluate(`(() => { document.querySelector('.analyze-toolbar button.primary-button').click(); })()`);
let summary = null;
for (let i = 0; i < 40; i += 1) {
  await sleep(400);
  summary = await evaluate(`(() => document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 160) ?? null)()`);
  if (summary && /complete|完成/i.test(summary)) break;
}
log({ event: "scan_summary", summary });
// Drill into the src directory tile by dblclick.
const drilled = await evaluate(`(() => {
  const tile = Array.from(document.querySelectorAll('.analyze-tile')).find((t) => (t.getAttribute('aria-label') ?? '').startsWith('src'));
  if (!tile) return { ok: false };
  tile.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true, view: window }));
  return { ok: true };
})()`);
await sleep(1200);
log({ event: "drilled", drilled });
const listProbe = await evaluate(`(() => ({
  crumbCount: document.querySelectorAll('.analyze-breadcrumbs button').length,
  listText: document.querySelector('.analyze-list-panel')?.textContent?.replace(/\\s+/g, ' ').slice(0, 500) ?? null,
  listOptions: Array.from(document.querySelectorAll('.analyze-listbox option')).map((o) => o.textContent.trim()).slice(0, 12),
}))()`);
log({ event: "list_probe", listProbe });
await screenshot("13-analyze-src-list-warnings-en");
// Open the locked node in the list to show its warning detail.
const opened = await evaluate(`(() => {
  const listbox = document.querySelector('.analyze-listbox');
  if (!listbox) return { ok: false };
  const option = Array.from(listbox.options).find((o) => /locked/i.test(o.textContent));
  if (!option) return { ok: false, options: Array.from(listbox.options).map((o) => o.textContent.trim()) };
  listbox.value = option.value;
  listbox.dispatchEvent(new Event('change', { bubbles: true }));
  return { ok: true, label: option.textContent.trim() };
})()`);
await sleep(900);
log({ event: "locked_selected", opened });
await screenshot("14-analyze-locked-warning-detail-en");

try { await send("Browser.close"); } catch (e) { log({ event: "browser_close_error", error: String(e) }); }
await sleep(2500);
try { execSync(`taskkill /PID ${app.pid} /F 2>NUL`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
try { execSync(`icacls "${lockedDir}" /reset`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
log({ event: "done" });
process.exit(0);
