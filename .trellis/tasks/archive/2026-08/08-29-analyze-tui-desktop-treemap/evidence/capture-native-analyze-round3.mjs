// Round-3 native evidence capture: correctly labeled en/zh states and widths,
// real per-launch WebView2 scale factors (100/125/150/200%), a read-data-denied
// directory for genuine AccessDenied/partial evidence, and a CDP accessibility
// (screen-reader) tree capture — all against the real release desktop app.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-analyze-tui-desktop-treemap/evidence/native-20260831-round3");
const fixtureRoot = join(evidence, "fixture");
const captureLog = join(evidence, "capture-log.jsonl");
const log = (line) => appendFileSync(captureLog, JSON.stringify(line) + "\n");
mkdirSync(join(evidence, "localappdata"), { recursive: true });
mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });
mkdirSync(join(evidence, "axtree"), { recursive: true });
mkdirSync(join(fixtureRoot, "src", "small"), { recursive: true });

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

// Fixture: wide 3000-file dir + 120 small files + a read-data-denied dir.
if (!existsSync(join(fixtureRoot, "big", "gen"))) {
  mkdirSync(join(fixtureRoot, "big", "gen"), { recursive: true });
  for (let i = 0; i < 3000; i += 1) writeFileSync(join(fixtureRoot, "big", "gen", `gen-${String(i).padStart(4, "0")}.dat`), Buffer.alloc(64, 121));
  for (let i = 0; i < 120; i += 1) writeFileSync(join(fixtureRoot, "src", "small", `file-${String(i).padStart(3, "0")}.bin`), Buffer.alloc(i + 1, 120));
}
const lockedDir = join(fixtureRoot, "src", "locked");
mkdirSync(lockedDir, { recursive: true });
if (!existsSync(join(lockedDir, "hidden.bin"))) writeFileSync(join(lockedDir, "hidden.bin"), Buffer.alloc(4096, 7));
execSync(`icacls "${lockedDir}" /deny "*S-1-5-11:(RD)" /inheritance:r`, { shell: "cmd.exe", stdio: "pipe" });
writeFileSync(captureLog, "");
log({ event: "exe_hash", exe, sha256: sha256(exe) });
log({ event: "acl_deny_read_data_set", dir: lockedDir });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch(scalePercent) {
  const cdpPort = 9350 + (scalePercent ? Math.round(scalePercent / 10) : 0);
  const localAppData = scalePercent
    ? join(evidence, `localappdata-dpi-${scalePercent}`)
    : join(evidence, "localappdata");
  mkdirSync(localAppData, { recursive: true });
  const extraArgs = scalePercent ? ` --force-device-scale-factor=${scalePercent / 100}` : "";
  const env = {
    ...process.env,
    LOCALAPPDATA: localAppData,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}${extraArgs}`,
  };
  const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
  log({ event: "app_launched", pid: app.pid, cdpPort, scalePercent, extraArgs: extraArgs || null });
  const targets = await (async () => {
    const deadline = Date.now() + 45000;
    while (Date.now() < deadline) {
      try { const r = await fetch(`http://127.0.0.1:${cdpPort}/json`); if (r.ok) return await r.json(); } catch {}
      await sleep(500);
    }
    throw new Error("cdp timeout");
  })();
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
  return { app, send, evaluate, screenshot, cdpPort, async close() {
    try { await send("Browser.close"); } catch {}
    await sleep(2000);
    try { execSync(`taskkill /PID ${app.pid} /F 2>NUL`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
    log({ event: "app_closed", pid: app.pid });
  } };
}

async function runAnalyzeAndCapture(session, tag, { widths = [], locale = "en" } = {}) {
  const { evaluate, screenshot, send } = session;
  await send("Runtime.enable");
  await send("Page.enable");
  await sleep(2200);
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
  for (let i = 0; i < 50; i += 1) {
    await sleep(400);
    summary = await evaluate(`(() => document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 140) ?? null)()`);
    if (summary && /complete|完成/i.test(summary)) break;
  }
  log({ event: "scan_summary", tag, summary });
  if (widths.length === 0) {
    if (locale === "en") await screenshot(`${tag}-analyze-complete-en-native-window`);
    else await screenshot(`${tag}-analyze-complete-zh-native-window`);
  }
  for (const width of widths) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`${tag}-analyze-complete-${locale}-${width}`);
  }
  if (widths.length) { await send("Emulation.clearDeviceMetricsOverride"); await sleep(500); }
  return summary;
}

// ---- Main session (no forced scale): states, widths, a11y tree, cancel ----
{
  const session = await launch(null);
  const { evaluate, screenshot, send } = session;
  // The profile may persist zh-CN from a previous run; force English first so
  // the "en" captures are provably English (store audited in-log).
  await send("Runtime.enable");
  await send("Page.enable");
  await sleep(2200);
  await send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  const forcedEn = await evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, 'en');
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  await sleep(1500);
  const storePathMain = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "force_en", selectValue: forcedEn, store: existsSync(storePathMain) ? readFileSync(storePathMain, "utf8").trim() : null });
  await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
  await sleep(1200);
  await runAnalyzeAndCapture(session, "r3-01", { widths: [1440, 1024, 800, 390] });
  const partialProbe = await evaluate(`(() => {
    const tile = Array.from(document.querySelectorAll('.analyze-tile')).find((t) => (t.getAttribute('aria-label') ?? '').startsWith('src'));
    tile?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true, view: window }));
    return Boolean(tile);
  })()`);
  await sleep(1100);
  const listProbe = await evaluate(`(() => ({
    crumbs: document.querySelectorAll('.analyze-breadcrumbs button').length,
    options: Array.from(document.querySelectorAll('.analyze-listbox option')).map((o) => o.textContent.trim()).slice(0, 8),
    summary: document.querySelector('.analyze-summary')?.textContent?.replace(/\\s+/g, ' ').slice(0, 200) ?? null,
  }))()`);
  log({ event: "partial_probe", partialProbe, listProbe });
  await screenshot("r3-02-analyze-src-warnings-en");
  // Keyboard drill + focus restoration.
  await evaluate(`(() => { document.querySelector('.analyze-breadcrumbs button')?.click(); })()`);
  await sleep(700);
  await evaluate(`(() => { const tile = Array.from(document.querySelectorAll('.analyze-tile')).find((t) => (t.getAttribute('aria-label') ?? '').startsWith('big')) ?? document.querySelector('.analyze-tile'); tile.focus(); })()`);
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
  await sleep(1000);
  const kbAfter = await evaluate(`(() => ({ activeClass: document.activeElement?.className ?? null, crumbs: document.querySelectorAll('.analyze-breadcrumbs button').length }))()`);
  log({ event: "keyboard_focus_postfix", kbAfter });
  await screenshot("r3-03-analyze-keyboard-drill-en");
  // Reduced motion + forced colors.
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await sleep(400);
  await screenshot("r3-04-analyze-reduced-motion-en");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }, { name: "prefers-contrast", value: "more" }] });
  await sleep(400);
  await screenshot("r3-05-analyze-forced-colors-en");
  await send("Emulation.setEmulatedMedia", { features: [] });
  await sleep(300);
  // Accessibility (screen-reader) tree of the analyze workspace.
  try {
    await send("Accessibility.enable");
    const ax = await send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", "analyze-complete-en-full-axtree.json"), JSON.stringify(ax, null, 2));
    log({ event: "axtree_captured", nodes: ax.nodes?.length ?? 0 });
  } catch (e) { log({ event: "axtree_error", error: String(e) }); }
  // Cancel-on-leave (en — captured before any language switch in this run).
  await evaluate(`(() => { document.querySelector('.analyze-breadcrumbs button')?.click(); })()`);
  await sleep(600);
  await evaluate(`(() => {
    const input = document.querySelector('.analyze-root-input input');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(input, ${JSON.stringify(join(repo, "target").split(/\\/g).join("/"))});
    input.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await evaluate(`(() => { document.querySelector('.analyze-toolbar button.primary-button').click(); })()`);
  await sleep(600);
  const canceling = await evaluate(`(() => ({ status: document.querySelector('.analyze-live-status')?.textContent ?? null, cancelVisible: Boolean(document.querySelector('.analyze-toolbar button.secondary-button')) }))()`);
  log({ event: "canceling_state", canceling });
  await screenshot("r3-06-analyze-canceling-en");
  await send("Runtime.evaluate", { expression: "location.hash = '#/clean'" });
  await sleep(2500);
  await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
  await sleep(1500);
  const returnState = await evaluate(`(() => ({ empty: Boolean(document.querySelector('.analyze-empty')) }))()`);
  log({ event: "return_after_cancel", returnState });
  await screenshot("r3-07-analyze-after-cancel-return");
  // zh-CN via settings, then complete zh with width matrix.
  await send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  await evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, 'zh-CN');
    select.dispatchEvent(new Event('change', { bubbles: true }));
  })()`);
  await sleep(1500);
  const storePath = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "store_audit", exists: existsSync(storePath), content: existsSync(storePath) ? readFileSync(storePath, "utf8").trim() : null });
  await send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
  await sleep(1200);
  await runAnalyzeAndCapture(session, "r3-08", { widths: [1440, 390], locale: "zh" });
  await session.close();
}

// ---- DPI scale launches: real app window at 100/125/150/200% --------------
for (const scalePercent of [100, 125, 150, 200]) {
  const session = await launch(scalePercent);
  await runAnalyzeAndCapture(session, `r3-dpi-${scalePercent}`, {});
  const dpr = await session.evaluate(`(() => ({ dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight], ua: navigator.userAgent.slice(0, 60) }))()`);
  log({ event: "dpi_probe", scalePercent, ...dpr });
  await session.close();
}

try { execSync(`icacls "${lockedDir}" /reset`, { shell: "cmd.exe", stdio: "pipe" }); } catch {}
log({ event: "done" });
process.exit(0);
