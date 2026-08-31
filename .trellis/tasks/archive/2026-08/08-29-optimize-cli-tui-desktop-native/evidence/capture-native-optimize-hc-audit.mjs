// Extra independent native shots: forced-colors + audit history.
// Same protocol as capture-native-optimize.mjs (isolated LOCALAPPDATA, CDP, path+PID kill).
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync, copyFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-optimize-cli-tui-desktop-native/evidence/native-desktop");
const eventsPath = join(repo, ".trellis/tasks/08-29-optimize-cli-tui-desktop-native/evidence/independent-native-desktop-hc-audit-events.jsonl");
const log = (line) => {
  const text = JSON.stringify(line);
  appendFileSync(join(evidence, "capture-log.jsonl"), text + "\n");
  appendFileSync(eventsPath, text + "\n");
};
mkdirSync(join(evidence, "screenshots"), { recursive: true });
writeFileSync(eventsPath, "");

if (!existsSync(exe)) throw new Error("missing release desktop binary: " + exe);
const sha256 = createHash("sha256").update(readFileSync(exe)).digest("hex");
log({ event: "hc_audit_exe_hash", exe, sha256 });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const localAppData = join(evidence, "localappdata-hc-audit");
mkdirSync(localAppData, { recursive: true });
const auditSrc = join(repo, ".trellis/tasks/08-29-optimize-cli-tui-desktop-native/evidence/native-independent/lappdata/DevSweep/audit/v1/optimize.jsonl");
const auditDstDir = join(localAppData, "DevSweep", "audit", "v1");
mkdirSync(auditDstDir, { recursive: true });
if (existsSync(auditSrc)) copyFileSync(auditSrc, join(auditDstDir, "optimize.jsonl"));
const cdpPort = 9375;
const env = {
  ...process.env,
  LOCALAPPDATA: localAppData,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
};
const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
log({ event: "app_launched", pid: app.pid, cdpPort });
const deadline = Date.now() + 45000;
let targets = null;
while (Date.now() < deadline) {
  try {
    const r = await fetch(`http://127.0.0.1:${cdpPort}/json`);
    if (r.ok) {
      targets = await r.json();
      break;
    }
  } catch {}
  await sleep(500);
}
if (!targets) throw new Error("cdp timeout");
const chosen = targets.find((t) => t.type === "page" && /tauri\.localhost/i.test(t.url || "")) ?? targets.find((t) => t.type === "page");
if (!chosen) throw new Error("no page target");
const ws = new WebSocket(chosen.webSocketDebuggerUrl);
await new Promise((resolveWs, reject) => {
  ws.onopen = resolveWs;
  ws.onerror = () => reject(new Error("ws error"));
});
let seq = 0;
const pending = new Map();
const send = (method, params = {}) => {
  const id = ++seq;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve2, reject) => {
    pending.set(id, { resolve: resolve2, reject });
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("cdp timeout: " + method));
      }
    }, 30000);
  });
};
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
      const { resolve: resolve2, reject } = pending.get(message.id);
      pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve2(message.result);
  }
};
const evaluate = async (expression) => {
  const result = await send("Runtime.evaluate", { expression, returnByValue: true });
  if (result.exceptionDetails) {
    throw new Error("page exception: " + (result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
  }
  return result.result.value;
};
const screenshot = async (name) => {
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(evidence, "screenshots", name + ".png"), Buffer.from(shot.data, "base64"));
  log({ event: "screenshot", name });
};

await send("Runtime.enable");
await send("Page.enable");
await sleep(2200);
await send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
await sleep(900);
await evaluate(`(() => {
  const select = document.querySelector('.settings-panel select');
  if (!select) return null;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
  setter.call(select, "en");
  select.dispatchEvent(new Event('change', { bubbles: true }));
  return select.value;
})()`);
await sleep(1200);
await send("Runtime.evaluate", { expression: "location.hash = '#/optimize'" });
await sleep(1200);
await evaluate(`(() => { document.querySelector('.optimize-toolbar button.primary-button')?.click(); })()`);
for (let i = 0; i < 40; i += 1) {
  await sleep(400);
  const rows = await evaluate(`(() => document.querySelectorAll('.optimize-row').length)()`);
  if (rows === 8) break;
}
await send("Emulation.setEmulatedMedia", {
  features: [
    { name: "forced-colors", value: "active" },
    { name: "prefers-contrast", value: "more" },
  ],
});
await sleep(600);
const hc = await evaluate(`(() => ({
  forced: window.matchMedia('(forced-colors: active)').matches,
  contrast: window.matchMedia('(prefers-contrast: more)').matches,
  summary: document.querySelector('.optimize-summary-bar')?.textContent?.replace(/\\s+/g, ' ').slice(0, 200) ?? null,
}))()`);
log({ event: "high_contrast_probe", hc });
await screenshot("opt-09-high-contrast-en");
await send("Emulation.setEmulatedMedia", { features: [] });
await sleep(400);
await evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Refresh audit history|刷新审计记录/.test(b.textContent || ''))?.click(); })()`);
let audit = null;
for (let i = 0; i < 25; i += 1) {
  await sleep(400);
  audit = await evaluate(`(() => ({
    status: document.querySelector('.optimize-mode')?.getAttribute('data-status'),
    auditText: document.querySelector('.optimize-audit')?.textContent?.replace(/\\s+/g, ' ').slice(0, 280) ?? null,
  }))()`);
  if (audit?.auditText) break;
}
log({ event: "audit_probe", audit });
await evaluate(`(() => { document.querySelector('.optimize-audit')?.scrollIntoView({ block: 'center' }); })()`);
await sleep(400);
await screenshot("opt-10-audit-en");

try { await send("Browser.close"); } catch {}
await sleep(2000);
let remaining = "";
try {
  remaining = execSync(
    `powershell.exe -NoLogo -NoProfile -Command "Get-CimInstance Win32_Process -Filter \\"ProcessId=${app.pid}\\" | Select-Object -ExpandProperty ExecutablePath"`,
    { encoding: "utf8" },
  ).trim();
} catch { remaining = ""; }
if (remaining && remaining.toLowerCase() === exe.toLowerCase()) {
  try { execSync(`taskkill /PID ${app.pid} /F`, { stdio: "pipe" }); } catch {}
  log({ event: "force_kill_after_path_pid_match", pid: app.pid, path: remaining });
} else {
  log({ event: remaining ? "refuse_kill_path_mismatch" : "app_exited_before_kill", pid: app.pid, path: remaining });
}
log({ event: "hc_audit_done" });
process.exit(0);
