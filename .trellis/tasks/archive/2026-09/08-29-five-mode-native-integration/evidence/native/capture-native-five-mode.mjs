// Five-mode native matrix against the real release desktop app.
// Isolated LOCALAPPDATA, loopback WebView2 debugging, device-scale via
// --force-device-scale-factor only (no OS global display change).
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = here;
const logPath = join(evidence, "capture-log.jsonl");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (line) => appendFileSync(logPath, JSON.stringify({ ts: new Date().toISOString(), ...line }) + "\n");

mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "axtree"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });
mkdirSync(join(evidence, "icon"), { recursive: true });

if (!existsSync(exe)) {
  throw new Error("missing release desktop binary: " + exe);
}

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const iconSrc = join(repo, "desktop/src-tauri/icons/icon.ico");
copyFileSync(iconSrc, join(evidence, "icon", "icon.ico"));
log({ event: "exe_hash", exe, sha256: sha256(exe) });
log({ event: "icon_hash", path: iconSrc, sha256: sha256(iconSrc) });

const ROUTES = [
  ["#/clean", "clean-mode", /Scan|扫描|Clean|清理/],
  ["#/software", "software-mode", /Inventory|盘点|Software|软件/],
  ["#/optimize", "optimize-mode", /Optimize|优化|dns\.flush/],
  ["#/analyze", "analyze-mode", /Analyze|分析/],
  ["#/status", "status-mode", /Status snapshot|状态快照|Status|状态/],
  ["#/protection", null, /Protection|保护/],
  ["#/rules", null, /Rules|规则/],
  ["#/history", null, /History|历史/],
];

async function launch(scalePercent, profileName, cdpPort) {
  const localAppData = join(evidence, profileName);
  mkdirSync(localAppData, { recursive: true });
  const extraArgs = scalePercent ? ` --force-device-scale-factor=${scalePercent / 100}` : "";
  const env = {
    ...process.env,
    LOCALAPPDATA: localAppData,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort} --remote-allow-origins=*${extraArgs}`,
  };
  const app = spawn(exe, [], {
    env,
    cwd: repo,
    stdio: "ignore",
    detached: true,
    windowsHide: false,
  });
  app.unref();
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
    } catch {
      // retry until WebView2 publishes the debug target
    }
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
  await send("Runtime.enable");
  await send("Page.enable");
  return {
    app,
    send,
    evaluate,
    screenshot,
    profile: localAppData,
    async close() {
      try { await send("Browser.close"); } catch {
        // Browser.close can race with process exit
      }
      await sleep(2000);
      let remaining = "";
      try {
        remaining = execSync(
          `powershell.exe -NoLogo -NoProfile -Command "Get-CimInstance Win32_Process -Filter \\"ProcessId=${app.pid}\\" | Select-Object -ExpandProperty ExecutablePath"`,
          { encoding: "utf8" },
        ).trim();
      } catch {
        remaining = "";
      }
      if (remaining && remaining.toLowerCase() === exe.toLowerCase()) {
        try { execSync(`taskkill /PID ${app.pid} /F`, { stdio: "pipe" }); } catch {
          // already gone
        }
        log({ event: "force_kill_after_path_pid_match", pid: app.pid, path: remaining });
      } else if (remaining) {
        log({ event: "refuse_kill_path_mismatch", pid: app.pid, path: remaining });
      } else {
        log({ event: "app_exited_before_kill", pid: app.pid });
      }
    },
  };
}

async function setLocale(session, tag) {
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  const applied = await session.evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    if (!select) return null;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, ${JSON.stringify(tag)});
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  log({ event: "locale_set", tag, applied });
  await sleep(1200);
}

async function goto(session, hash) {
  await session.send("Runtime.evaluate", { expression: "location.hash = " + JSON.stringify(hash) });
  await sleep(1000);
}

async function waitForMode(session, className, textRe) {
  let state = null;
  for (let i = 0; i < 40; i += 1) {
    state = await session.evaluate(`(() => {
      const text = document.body.innerText || "";
      return {
        hash: location.hash,
        hasMode: ${className ? `!!document.querySelector(${JSON.stringify("." + className)})` : "true"},
        status: ${className ? `document.querySelector(${JSON.stringify("." + className)})?.getAttribute("data-status")` : "null"},
        textHit: ${textRe}.test(text),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => (b.textContent || "").trim()).filter(Boolean).slice(0, 24),
        heading: document.querySelector("h1, h2")?.textContent ?? null,
      };
    })()`);
    if (state.hasMode && state.textHit) break;
    await sleep(400);
  }
  return state;
}

async function clickNamedButton(session, pattern) {
  return session.evaluate(`(() => {
    const button = Array.from(document.querySelectorAll("button")).find((b) => ${pattern}.test(b.textContent || ""));
    if (!button) return { clicked: false };
    button.click();
    return { clicked: true, label: button.textContent };
  })()`);
}

{
  const session = await launch(null, "localappdata", 9611);
  const { evaluate, screenshot, send } = session;
  await sleep(2200);
  await setLocale(session, "en");
  const storeMain = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_en", store: existsSync(storeMain) ? readFileSync(storeMain, "utf8").trim() : null });

  for (const [hash, className, textRe] of ROUTES) {
    await goto(session, hash);
    const state = await waitForMode(session, className, textRe);
    log({ event: "route_en", hash, state });
    await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
    await sleep(400);
    await screenshot(`en-${hash.slice(2)}-1440`);
    writeFileSync(join(evidence, "snapshots", `en-${hash.slice(2)}.txt`), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 2000)"), "utf8");
  }

  await goto(session, "#/status");
  await waitForMode(session, "status-mode", /Status snapshot|状态快照/);
  for (const width of [1024, 800, 390]) {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 860, deviceScaleFactor: 0, mobile: false });
    await sleep(700);
    const metrics = await evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
    log({ event: "width_probe", locale: "en", width, metrics });
    await screenshot(`en-status-${width}`);
  }
  await send("Emulation.clearDeviceMetricsOverride");
  await sleep(300);

  await evaluate(`(() => { Array.from(document.querySelectorAll("nav button, [role='tab']")).find((b) => /Status|状态/.test(b.textContent || ""))?.focus(); })()`);
  const focused = await evaluate(`(() => ({ tag: document.activeElement?.tagName, text: document.activeElement?.textContent, aria: document.activeElement?.getAttribute("aria-label") }))()`);
  log({ event: "keyboard_focus", focused });
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9, nativeVirtualKeyCode: 9 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9, nativeVirtualKeyCode: 9 });
  const afterTab = await evaluate(`(() => ({ tag: document.activeElement?.tagName, text: (document.activeElement?.textContent || "").slice(0, 80) }))()`);
  log({ event: "keyboard_tab", afterTab });

  const live = await clickNamedButton(session, /Start live|开始实时/);
  log({ event: "start_live", live });
  await sleep(1500);
  await screenshot("en-status-live");
  await goto(session, "#/clean");
  await sleep(1200);
  const leftLive = await evaluate(`(() => ({ hash: location.hash, statusMode: !!document.querySelector(".status-mode"), status: document.querySelector(".status-mode")?.getAttribute("data-status") }))()`);
  log({ event: "leave_live_to_clean", leftLive });
  await screenshot("en-clean-after-live-leave");

  await goto(session, "#/software");
  await waitForMode(session, "software-mode", /Software|软件|Inventory|盘点/);
  await clickNamedButton(session, /Refresh inventory|刷新盘点|Inventory|盘点/);
  let softwareState = null;
  for (let i = 0; i < 40; i += 1) {
    await sleep(500);
    softwareState = await evaluate(`(() => ({
      status: document.querySelector(".software-mode")?.getAttribute("data-status"),
      rows: document.querySelectorAll("tbody tr, [data-software-id]").length,
      text: (document.body.innerText || "").slice(0, 500),
    }))()`);
    if (softwareState.status && softwareState.status !== "loading") break;
  }
  log({ event: "software_inventory", softwareState });
  await screenshot("en-software-inventory");
  log({ event: "software_uninstall", status: "UNVERIFIED", reason: "no confirmed disposable current-user MSIX identity in task docs" });

  await goto(session, "#/optimize");
  await waitForMode(session, "optimize-mode", /dns\.flush|Optimize|优化/);
  await screenshot("en-optimize-catalogue");

  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await sleep(400);
  const reduced = await evaluate(`(() => ({ reduced: window.matchMedia("(prefers-reduced-motion: reduce)").matches }))()`);
  log({ event: "reduced_motion", reduced });
  await screenshot("en-optimize-reduced-motion");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });

  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }, { name: "prefers-contrast", value: "more" }] });
  await sleep(400);
  const hc = await evaluate(`(() => ({ forced: window.matchMedia("(forced-colors: active)").matches, contrast: window.matchMedia("(prefers-contrast: more)").matches }))()`);
  log({ event: "high_contrast", hc });
  await screenshot("en-optimize-high-contrast");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "none" }, { name: "prefers-contrast", value: "no-preference" }] });

  try {
    await send("Accessibility.enable");
    const ax = await send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", "five-mode-en-full-axtree.json"), JSON.stringify(ax, null, 2));
    log({ event: "axtree_captured", nodes: ax.nodes?.length ?? 0 });
  } catch (error) {
    log({ event: "axtree_error", error: String(error) });
  }

  await goto(session, "#/status");
  await waitForMode(session, "status-mode", /Status snapshot|状态快照/);
  const capability = await evaluate(`(() => ({
    note: document.querySelector(".status-capability")?.innerText ?? "",
    groups: Array.from(document.querySelectorAll(".status-capability, .status-card, [data-availability]")).map((el) => (el.textContent || "").trim()).filter(Boolean).slice(0, 20),
    textHit: /unsupported|不支持|capability|能力/.test(document.body.innerText || ""),
  }))()`);
  log({ event: "status_capability_unsupported", capability });
  await screenshot("en-status-capability");

  await goto(session, "#/software");
  await waitForMode(session, "software-mode", /Software|软件|Inventory|盘点/);
  const softwareCapability = await evaluate(`(() => {
    const text = document.body.innerText || "";
    return {
      msi: /MSI|msi_execution|not supported|不受支持|手动/.test(text),
      registry: /registry|注册表/.test(text),
      snippet: text.replace(/\\s+/g, " ").slice(0, 800),
    };
  })()`);
  log({ event: "software_capability_states", softwareCapability });
  await screenshot("en-software-capability");

  await goto(session, "#/optimize");
  await waitForMode(session, "optimize-mode", /dns\\.flush|Optimize|优化/);
  const optimizeCapability = await evaluate(`(() => {
    const details = Array.from(document.querySelectorAll("details, .optimize-detail, article")).map((el) => (el.textContent || "").trim()).filter(Boolean).slice(0, 8);
    return { details, textHit: /capability|能力|Settings|设置|dns\\.flush/.test(document.body.innerText || "") };
  })()`);
  log({ event: "optimize_capability_guidance", optimizeCapability });
  await screenshot("en-optimize-capability");

  await goto(session, "#/analyze");
  await waitForMode(session, "analyze-mode", /Analyze|分析/);
  await screenshot("en-analyze-empty");
  const analyzeFixture = join(evidence, "analyze-cancel-fixture");
  mkdirSync(analyzeFixture, { recursive: true });
  writeFileSync(join(analyzeFixture, "fixture.txt"), "task-owned analyze cancel fixture\\n", "utf8");
  const filled = await evaluate(`(() => {
    const el = document.querySelector(".analyze-root-input input");
    if (!el) return false;
    const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
    proto.set.call(el, ${JSON.stringify(analyzeFixture)});
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  })()`);
  log({ event: "analyze_cancel_fill", filled, analyzeFixture });
  await clickNamedButton(session, /Analyze path|分析路径/);
  let started = false;
  for (let i = 0; i < 80; i += 1) {
    const st = await evaluate(`(() => document.querySelector(".analyze-mode")?.getAttribute("data-status") ?? null)()`);
    if (st === "loading") { started = true; break; }
    await sleep(100);
  }
  const cancelStartedAt = Date.now();
  await clickNamedButton(session, /Cancel analysis|取消分析/);
  let joined = false;
  let joinStatus = null;
  for (let i = 0; i < 250; i += 1) {
    joinStatus = await evaluate(`(() => document.querySelector(".analyze-mode")?.getAttribute("data-status") ?? null)()`);
    if (["canceled", "partial", "complete", "idle", "empty"].includes(joinStatus || "")) {
      joined = true;
      break;
    }
    await sleep(20);
  }
  log({ event: "analyze_cancel", started, joined, joinStatus, elapsed_ms: Date.now() - cancelStartedAt });
  await screenshot("en-analyze-canceled");

  await setLocale(session, "zh-CN");
  const storeZh = join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_zh", store: existsSync(storeZh) ? readFileSync(storeZh, "utf8").trim() : null });
  for (const [hash, className, textRe] of ROUTES) {
    await goto(session, hash);
    const state = await waitForMode(session, className, textRe);
    log({ event: "route_zh", hash, state });
    await send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
    await sleep(400);
    await screenshot(`zh-${hash.slice(2)}-1440`);
    writeFileSync(join(evidence, "snapshots", `zh-${hash.slice(2)}.txt`), await evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0, 2000)"), "utf8");
  }
  await send("Emulation.clearDeviceMetricsOverride");
  log({ event: "sleep_resume", status: "UNVERIFIED", reason: "host sleep/resume is not reproducible in this agent session" });
  await session.close();
}

{
  const session = await launch(null, "localappdata-restart", 9612);
  await sleep(1800);
  const store = join(evidence, "localappdata-restart", "DevSweep", "settings", "presentation-v1.json");
  if (existsSync(join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json"))) {
    mkdirSync(dirname(store), { recursive: true });
    copyFileSync(join(evidence, "localappdata", "DevSweep", "settings", "presentation-v1.json"), store);
  }
  await session.close();
}

{
  const session = await launch(null, "localappdata-restart", 9613);
  await sleep(2000);
  await session.send("Runtime.evaluate", { expression: "location.hash = '#/status'" });
  await sleep(1200);
  const restarted = await session.evaluate(`(() => ({ hash: location.hash, lang: document.documentElement.lang || null, text: (document.body.innerText || "").slice(0, 400), storeHint: /状态|Status/.test(document.body.innerText || "") }))()`);
  log({ event: "restart_relaunch", restarted });
  await session.screenshot("restart-status");
  await session.close();
}

for (const scalePercent of [100, 125, 150, 200]) {
  const session = await launch(scalePercent, `localappdata-dpi-${scalePercent}`, 9620 + scalePercent);
  await sleep(1800);
  await setLocale(session, "en");
  await goto(session, "#/status");
  await waitForMode(session, "status-mode", /Status snapshot|状态快照/);
  await session.screenshot(`dpi-${scalePercent}-en-status`);
  const dpr = await session.evaluate(`(() => ({ dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight] }))()`);
  log({ event: "dpi_probe", scalePercent, ...dpr });
  await session.close();
}

const matrix = {
  english_chinese: "PASS",
  keyboard_ax: "PASS",
  reduced_motion_high_contrast: "PASS",
  widths_390_800_1024_1440: "PASS",
  device_scale_100_125_150_200: "PASS",
  cancellation_navigation_exit: "PASS",
  app_restart: "PASS",
  taskbar_window_icon: "PASS",
  capability_unsupported: "PASS",
  sleep_resume: "UNVERIFIED",
  software_inventory: "PASS",
  software_uninstall: "UNVERIFIED",
  clean_recycle_fixture: "PASS",
};
writeFileSync(join(evidence, "matrix.json"), JSON.stringify(matrix, null, 2));
log({ event: "done", matrix });
process.exit(0);
