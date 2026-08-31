// Native Status workbench capture against the real release desktop app.
// Isolated LOCALAPPDATA, loopback WebView2 debugging, device-scale emulation
// (no user-global display change), bilingual screenshots at 1440/1024/800/390,
// keyboard focus, AX tree, reduced-motion and forced-colors, explicit live,
// interval change, leave/restart/close.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, appendFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-status-tui-desktop-native/evidence/native-desktop");
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
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/status'" });
  await sleep(1200);
}

async function waitForSnapshot(session, tag) {
  let state = null;
  for (let i = 0; i < 50; i += 1) {
    await sleep(400);
    state = await session.evaluate(`(() => {
      const mode = document.querySelector('.status-mode');
      const text = document.body.innerText || '';
      return {
        status: mode?.getAttribute('data-status') ?? null,
        hasMode: !!mode,
        snapshot: /Status snapshot|状态快照/.test(text),
        startLive: !!Array.from(document.querySelectorAll('button')).find((b) => /Start live|开始实时/.test(b.textContent || '')),
        stopLive: !!Array.from(document.querySelectorAll('button')).find((b) => /Stop live|停止实时/.test(b.textContent || '')),
        processRows: document.querySelectorAll('.status-table tbody tr').length,
        capability: Array.from(document.querySelectorAll('.status-capability p')).map((n) => n.textContent).slice(0, 4),
        gpuZero: /gpu 0/i.test(text),
        cmdline: /cmdline/i.test(text),
      };
    })()`);
    if (state.hasMode && state.snapshot && state.status && state.status !== "snapshot") break;
  }
  log({ event: "snapshot_state", tag, state });
  return state;
}

async function clickNamedButton(session, pattern) {
  return session.evaluate(`(() => {
    const button = Array.from(document.querySelectorAll('button')).find((b) => ${pattern}.test(b.textContent || ''));
    if (!button) return { clicked: false };
    button.click();
    return { clicked: true, label: button.textContent };
  })()`);
}

{
  const session = await launch(null, "localappdata", 9471);
  const { evaluate, screenshot, send } = session;
  await send("Runtime.enable");
  await send("Page.enable");
  await sleep(2200);
  await setLocale(session, "en");
  const storeMain = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_en", store: existsSync(storeMain) ? readFileSync(storeMain, "utf8").trim() : null });
  const snapshotEn = await waitForSnapshot(session, "en");
  await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(500);
  const metrics1440 = await evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
  log({ event: "width_probe", locale: "en", width: 1440, metrics: metrics1440 });
  await screenshot("status-snapshot-en-1440");
  writeFileSync(join(evidence, "snapshots", "status-en.txt"), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 1600)"), "utf8");
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`status-snapshot-en-${width}`);
    const metrics = await evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
    log({ event: "width_probe", locale: "en", width, metrics });
  }
  await send("Emulation.clearDeviceMetricsOverride");
  await sleep(400);

  await evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Start live/.test(b.textContent || ''))?.focus(); })()`);
  const focused = await evaluate(`(() => ({ tag: document.activeElement?.tagName, text: document.activeElement?.textContent, aria: document.activeElement?.getAttribute('aria-label') }))()`);
  log({ event: "keyboard_focus_start_live", focused });
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: " ", code: "Space", windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  let liveState = null;
  for (let i = 0; i < 40; i += 1) {
    await sleep(400);
    liveState = await evaluate(`(() => ({
      status: document.querySelector('.status-mode')?.getAttribute('data-status'),
      stopLive: !!Array.from(document.querySelectorAll('button')).find((b) => /Stop live|停止实时/.test(b.textContent || '')),
      chart: document.querySelectorAll('.status-chart').length,
      liveLabel: document.querySelector('.status-live-status')?.textContent ?? null,
    }))()`);
    if (liveState.status === "live" && liveState.stopLive) break;
  }
  log({ event: "live_started_keyboard", liveState, priorSnapshot: snapshotEn });
  await screenshot("status-live-started-en");

  await evaluate(`(() => {
    const select = document.querySelector('select[aria-label="Live interval"], select[aria-label="实时间隔"]');
    if (!select) return null;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, '5000');
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  await sleep(2500);
  const intervalState = await evaluate(`(() => ({
    status: document.querySelector('.status-mode')?.getAttribute('data-status'),
    interval: document.querySelector('select[aria-label="Live interval"], select[aria-label="实时间隔"]')?.value ?? null,
    stopLive: !!Array.from(document.querySelectorAll('button')).find((b) => /Stop live|停止实时/.test(b.textContent || '')),
  }))()`);
  log({ event: "interval_change", intervalState });
  await screenshot("status-interval-change-en");

  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await sleep(400);
  await screenshot("status-reduced-motion-en");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });

  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }, { name: "prefers-contrast", value: "more" }] });
  await sleep(400);
  const hcProbe = await evaluate(`(() => ({ forced: window.matchMedia('(forced-colors: active)').matches, contrast: window.matchMedia('(prefers-contrast: more)').matches }))()`);
  log({ event: "high_contrast_probe", hcProbe });
  await screenshot("status-high-contrast-en");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "none" }, { name: "prefers-contrast", value: "no-preference" }] });

  try {
    await send("Accessibility.enable");
    const ax = await send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", "status-en-full-axtree.json"), JSON.stringify(ax, null, 2));
    log({ event: "axtree_captured", nodes: ax.nodes?.length ?? 0 });
  } catch (error) {
    log({ event: "axtree_error", error: String(error) });
  }

  const stop = await clickNamedButton(session, /Stop live|停止实时/);
  log({ event: "stop_live", stop });
  await sleep(1500);

  await session.send("Runtime.evaluate", { expression: "location.hash = '#/clean'" });
  await sleep(1200);
  const left = await evaluate(`(() => ({ hash: location.hash, statusMode: !!document.querySelector('.status-mode') }))()`);
  log({ event: "leave", left });

  await session.send("Runtime.evaluate", { expression: "location.hash = '#/status'" });
  await sleep(1200);
  const restarted = await waitForSnapshot(session, "restart-en");
  log({ event: "restart", restarted });

  await setLocale(session, "zh-CN");
  const storeZh = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_zh", store: existsSync(storeZh) ? readFileSync(storeZh, "utf8").trim() : null });
  await waitForSnapshot(session, "zh-CN");
  await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(500);
  await screenshot("status-snapshot-zh-1440");
  writeFileSync(join(evidence, "snapshots", "status-zh.txt"), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 1600)"), "utf8");
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    await screenshot(`status-snapshot-zh-${width}`);
  }
  await send("Emulation.clearDeviceMetricsOverride");
  log({ event: "close" });
  await session.close();
}

for (const scalePercent of [100, 125, 150, 200]) {
  const session = await launch(scalePercent, `localappdata-dpi-${scalePercent}`, 9480 + scalePercent);
  await session.send("Runtime.enable");
  await session.send("Page.enable");
  await sleep(1800);
  await setLocale(session, "en");
  await waitForSnapshot(session, `dpi-${scalePercent}`);
  await session.screenshot(`status-dpi-${scalePercent}-en`);
  const dpr = await session.evaluate(`(() => ({ dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight] }))()`);
  log({ event: "dpi_probe", scalePercent, ...dpr });
  await session.close();
}

log({ event: "done" });
process.exit(0);
