// Independent native Optimize workbench capture against the real release
// desktop app. Mirrors the Software presentation child's CDP pattern:
// isolated LOCALAPPDATA, loopback WebView2 debugging, device-scale emulation
// (no user-global display change), bilingual screenshots at 1440/1024/800/390,
// keyboard focus, AX tree, reduced-motion emulation.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-optimize-cli-tui-desktop-native/evidence/native-desktop");
const log = (line) => appendFileSync(join(evidence, "capture-log.jsonl"), JSON.stringify(line) + "\n");
mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "axtree"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });

if (!existsSync(exe)) {
  throw new Error("missing release desktop binary: " + exe);
}

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
log({ event: "exe_hash", exe, sha256: sha256(exe) });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch(scalePercent, profileName, cdpPort) {
  const localAppData = join(evidence, profileName);
  mkdirSync(localAppData, { recursive: true });
  const extraArgs = scalePercent ? ` --force-device-scale-factor=${scalePercent / 100}` : "";
  const env = {
    ...process.env,
    LOCALAPPDATA: localAppData,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}${extraArgs}`,
  };
  const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
  log({ event: "app_launched", pid: app.pid, cdpPort, scalePercent, profile: profileName });
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
  if (!targets) throw new Error("cdp timeout port " + cdpPort);
  const page = targets.find((t) => t.type === "page" && /tauri\.localhost/i.test(t.url || ""));
  const chosen = page ?? targets.find((t) => t.type === "page");
  if (!chosen) throw new Error("no page target");
  log({ event: "cdp_target", url: chosen.url, title: chosen.title, id: chosen.id });
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
      throw new Error("page exception: " + JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
    }
    return result.result.value;
  };
  const screenshot = async (name) => {
    const shot = await send("Page.captureScreenshot", { format: "png" });
    writeFileSync(join(evidence, "screenshots", name + ".png"), Buffer.from(shot.data, "base64"));
    log({ event: "screenshot", name });
  };
  return {
    app,
    send,
    evaluate,
    screenshot,
    async close() {
      try { await send("Browser.close"); } catch {}
      await sleep(2000);
      let remaining = null;
      try {
        remaining = execSync(
          `powershell.exe -NoLogo -NoProfile -Command "Get-CimInstance Win32_Process -Filter \\"ProcessId=${app.pid}\\" | Select-Object -ExpandProperty ExecutablePath"`,
          { encoding: "utf8" },
        ).trim();
      } catch {
        remaining = "";
      }
      if (remaining && remaining.toLowerCase() === exe.toLowerCase()) {
        try { execSync(`taskkill /PID ${app.pid} /F`, { stdio: "pipe" }); } catch {}
        log({ event: "force_kill_after_path_pid_match", pid: app.pid, path: remaining });
      } else if (remaining) {
        log({ event: "refuse_kill_path_mismatch", pid: app.pid, path: remaining });
      } else {
        log({ event: "app_exited_before_kill", pid: app.pid });
      }
      log({ event: "app_closed", pid: app.pid });
    },
  };
}

async function setLocale(session, tag) {
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  await session.evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    if (!select) return null;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, ${JSON.stringify(tag)});
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  await sleep(1200);
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/optimize'" });
  await sleep(1200);
}

async function loadCatalogue(session, tag) {
  await session.evaluate(`(() => { document.querySelector('.optimize-toolbar button.primary-button')?.click(); })()`);
  let state = null;
  for (let i = 0; i < 40; i += 1) {
    await sleep(400);
    state = await session.evaluate(`(() => ({
      status: document.querySelector('.optimize-mode')?.getAttribute('data-status'),
      rows: document.querySelectorAll('.optimize-row').length,
      summary: document.querySelector('.optimize-summary-bar')?.textContent?.replace(/\\s+/g, ' ').slice(0, 240) ?? null,
      noRun: Array.from(document.querySelectorAll('.optimize-no-run')).map((n) => n.textContent),
      badges: Array.from(document.querySelectorAll('.optimize-badge')).map((n) => n.textContent),
    }))()`);
    if (state.rows === 8 && state.status !== "checking") break;
  }
  log({ event: "catalogue_state", tag, state });
  return state;
}

async function emulateReducedMotion(session, reduce) {
  await session.send("Emulation.setEmulatedMedia", {
    features: [{ name: "prefers-reduced-motion", value: reduce ? "reduce" : "no-preference" }],
  });
}

{
  const session = await launch(null, "localappdata", 9371);
  const { evaluate, screenshot, send } = session;
  await send("Runtime.enable");
  await send("Page.enable");
  await sleep(2200);
  await setLocale(session, "en");
  const storeMain = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_en", store: existsSync(storeMain) ? readFileSync(storeMain, "utf8").trim() : null });
  const catalogue = await loadCatalogue(session, "en");
  await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(500);
  const metrics1440 = await evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
  log({ event: "width_probe", locale: "en", width: 1440, metrics: metrics1440 });
  await screenshot("opt-01-catalogue-en-1440");
  writeFileSync(join(evidence, "snapshots", "catalogue-en.txt"), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 1200)"), "utf8");
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`opt-02-catalogue-en-${width}`);
    const metrics = await evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
    log({ event: "width_probe", locale: "en", width, metrics });
  }
  await send("Emulation.clearDeviceMetricsOverride");
  await sleep(400);

  await evaluate(`(() => { document.querySelector('input[aria-label*="guidance.drive_optimize"]')?.focus(); })()`);
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await sleep(500);
  const guidanceKb = await evaluate(`(() => ({
    checked: document.querySelector('input[aria-label*="guidance.drive_optimize"]')?.checked ?? null,
    preview: !!document.querySelector('.optimize-summary-actions button'),
    summary: document.querySelector('.optimize-summary-bar')?.textContent?.replace(/\\s+/g, ' ').slice(0, 240) ?? null,
    noRun: Array.from(document.querySelectorAll('.optimize-no-run')).map((n) => n.textContent),
  }))()`);
  log({ event: "keyboard_guidance", guidanceKb });
  await screenshot("opt-03-guidance-keyboard-en");

  await evaluate(`(() => { document.querySelector('input[aria-label*="settings.search"]')?.click(); })()`);
  await sleep(400);
  await evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Review preview|审阅预览/.test(b.textContent || ''))?.click(); })()`);
  for (let i = 0; i < 20; i += 1) {
    await sleep(300);
    const ready = await evaluate(`(() => document.querySelector('.optimize-mode')?.getAttribute('data-status'))()`);
    if (ready === "preview_ready") break;
  }
  await screenshot("opt-04-preview-search-en");
  await evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Continue to confirmation|进入二次确认/.test(b.textContent || ''))?.click(); })()`);
  await sleep(500);
  const dialog = await evaluate(`(() => ({
    open: document.querySelector('dialog.optimize-confirm-dialog')?.open ?? false,
    title: document.querySelector('#optimize-confirm-title')?.textContent ?? null,
    runDns: !!Array.from(document.querySelectorAll('button')).find((b) => (b.textContent || '').includes('Run DNS flush')),
    openSettings: !!Array.from(document.querySelectorAll('button')).find((b) => /Open Windows Settings|打开 Windows 设置/.test(b.textContent || '')),
  }))()`);
  log({ event: "confirm_dialog", dialog });
  await screenshot("opt-05-confirm-settings-en");
  await evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Close|关闭/.test(b.textContent || '') && b.closest('dialog'))?.click(); })()`);
  await sleep(400);

  await emulateReducedMotion(session, true);
  await sleep(400);
  await screenshot("opt-06-reduced-motion-en");
  await emulateReducedMotion(session, false);

  try {
    await send("Accessibility.enable");
    const ax = await send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", "optimize-en-full-axtree.json"), JSON.stringify(ax, null, 2));
    log({ event: "axtree_captured", nodes: ax.nodes?.length ?? 0 });
  } catch (error) {
    log({ event: "axtree_error", error: String(error) });
  }

  await setLocale(session, "zh-CN");
  const storeZh = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_zh", store: existsSync(storeZh) ? readFileSync(storeZh, "utf8").trim() : null });
  await loadCatalogue(session, "zh-CN");
  await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(500);
  await screenshot("opt-07-catalogue-zh-1440");
  writeFileSync(join(evidence, "snapshots", "catalogue-zh.txt"), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 1200)"), "utf8");
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`opt-08-catalogue-zh-${width}`);
  }
  await send("Emulation.clearDeviceMetricsOverride");
  await session.close();
}

for (const scalePercent of [100, 125, 150, 200]) {
  const session = await launch(scalePercent, `localappdata-dpi-${scalePercent}`, 9380 + scalePercent);
  await session.send("Runtime.enable");
  await session.send("Page.enable");
  await sleep(1800);
  await setLocale(session, "en");
  await loadCatalogue(session, `dpi-${scalePercent}`);
  await session.screenshot(`opt-dpi-${scalePercent}-catalogue-en`);
  const dpr = await session.evaluate(`(() => ({ dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight] }))()`);
  log({ event: "dpi_probe", scalePercent, ...dpr });
  await session.close();
}

log({ event: "done" });
process.exit(0);
