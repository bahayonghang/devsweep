// Native Protection/Rules/History capture against the real release desktop app.
// Isolated LOCALAPPDATA + APPDATA, loopback WebView2 debugging, device-scale
// emulation (no user-global display change), bilingual screenshots at
// 1440/1024/800/390, keyboard focus, AX tree, forced-colors, concurrent CLI
// writers, corrupt/newer stores, mixed audit redaction, no history replay.
import { spawn, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  writeFileSync,
  appendFileSync,
  mkdirSync,
  existsSync,
  readFileSync,
  copyFileSync,
} from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const releaseCli = join(repo, "target/release/devsweep.exe");
const debugCli = join(repo, "target/debug/devsweep.exe");
const cli = existsSync(debugCli) ? debugCli : releaseCli;
const evidence = join(repo, ".trellis/tasks/08-29-protect-rules-history-surfaces/evidence/independent-native-desktop");
const logPath = join(evidence, "capture-log.jsonl");
const log = (line) => appendFileSync(logPath, JSON.stringify(line) + "\n");

mkdirSync(join(evidence, "screenshots"), { recursive: true });
mkdirSync(join(evidence, "axtree"), { recursive: true });
mkdirSync(join(evidence, "snapshots"), { recursive: true });
mkdirSync(join(evidence, "stores"), { recursive: true });
writeFileSync(logPath, "");

if (!existsSync(exe)) {
  throw new Error("missing release desktop binary: " + exe);
}
if (!existsSync(cli)) {
  throw new Error("missing CLI binary: " + cli);
}

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
log({ event: "exe_hash", exe, sha256: sha256(exe), cli, cliSha256: sha256(cli) });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function integrityProbe() {
  try {
    const whoami = execSync("whoami /groups", { encoding: "utf8" });
    const medium = /S-1-16-8192/.test(whoami);
    let consent = 0;
    try {
      const listed = execSync(
        'powershell.exe -NoLogo -NoProfile -Command "(Get-Process consent -ErrorAction SilentlyContinue | Measure-Object).Count"',
        { encoding: "utf8" },
      ).trim();
      consent = Number(listed) || 0;
    } catch {
      consent = 0;
    }
    return { mediumIntegrity: medium, consentExeCount: consent };
  } catch (error) {
    return { error: String(error) };
  }
}

function writeJson(path, value) {
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, typeof value === "string" ? value : JSON.stringify(value));
}

function seedSettings(localAppData, language) {
  const path = join(localAppData, "DevSweep", "settings", "presentation-v1.json");
  mkdirSync(join(localAppData, "DevSweep", "settings"), { recursive: true });
  writeFileSync(path, JSON.stringify({ schema_version: 1, language }));
  return path;
}

function seedHealthyProtection(appData, paths) {
  const dir = join(appData, "devsweep");
  mkdirSync(dir, { recursive: true });
  const store = join(dir, "protected-paths.json");
  writeFileSync(store, JSON.stringify({ version: 1, paths }));
  return store;
}

function seedMixedHistory(localAppData) {
  const dir = join(localAppData, "DevSweep", "audit", "v1");
  mkdirSync(dir, { recursive: true });
  const fixtures = join(repo, "crates/devsweep-core/tests/fixtures/history/clean-v1");
  const clean = [
    readFileSync(join(fixtures, "protection-mutation.jsonl"), "utf8").trim(),
    readFileSync(join(fixtures, "execution-transition.jsonl"), "utf8").trim(),
    readFileSync(join(fixtures, "unknown-version.jsonl"), "utf8").trim(),
    JSON.stringify({
      schema_version: 1,
      domain: "clean",
      record_kind: "execution_transition",
      operation_id: "op-forbidden-must-redact",
      path: "C:/secret-keep-native",
      command: "cmd.exe",
      argv: ["cmd.exe", "/c", "rd /s /q C:\\secret-keep-native"],
    }),
  ].join("\n") + "\n";
  const software = [
    JSON.stringify({
      schema_version: 1,
      domain: "software",
      operation_id: "op-sw-partial-v1",
      timestamp_unix_ms: 1711929601000,
    }),
    JSON.stringify({
      schema_version: 2,
      domain: "software",
      operation_id: "op-sw-future",
      path: "C:/secret-software.exe",
      argv: ["msiexec", "/x", "{SECRET-GUID}"],
    }),
  ].join("\n") + "\n";
  const optimize = [
    JSON.stringify({
      schema_version: 1,
      domain: "optimize",
      operation_id: "op-opt-partial-v1",
      timestamp_unix_ms: 1711929602000,
    }),
    JSON.stringify({
      schema_version: 2,
      domain: "optimize",
      operation_id: "op-opt-future",
      command: "ipconfig",
      argv: ["/flushdns"],
      path: "C:/Windows/System32/ipconfig.exe",
    }),
  ].join("\n") + "\n";
  writeFileSync(join(dir, "clean.jsonl"), clean);
  writeFileSync(join(dir, "software.jsonl"), software);
  writeFileSync(join(dir, "optimize.jsonl"), optimize);
  return dir;
}

function forbiddenVisible(text) {
  const hay = (text || "").toLowerCase();
  return {
    secretKeep: hay.includes("secret-keep-native"),
    secretSoftware: hay.includes("secret-software.exe"),
    msiexec: hay.includes("msiexec"),
    flushdns: hay.includes("flushdns") || hay.includes("/flushdns"),
    ipconfig: hay.includes("ipconfig"),
    rawArgv: hay.includes("\"argv\"") || hay.includes("rd /s"),
    runRetry: /\b(run this operation|retry this|replay this)\b/.test(hay),
  };
}

async function launch({ scalePercent, profileName, cdpPort, appDataName, language, protectionPaths, history }) {
  const localAppData = join(evidence, profileName);
  const appData = join(evidence, appDataName);
  mkdirSync(localAppData, { recursive: true });
  mkdirSync(appData, { recursive: true });
  seedSettings(localAppData, language);
  if (protectionPaths) seedHealthyProtection(appData, protectionPaths);
  if (history) seedMixedHistory(localAppData);
  const extraArgs = scalePercent ? ` --force-device-scale-factor=${scalePercent / 100}` : "";
  const env = {
    ...process.env,
    LOCALAPPDATA: localAppData,
    APPDATA: appData,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}${extraArgs}`,
  };
  const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
  log({ event: "app_launched", pid: app.pid, cdpPort, scalePercent, profile: profileName, appData: appDataName, language });
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
  log({ event: "cdp_target", url: chosen.url, title: chosen.title, id: chosen.id, pid: app.pid });
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
    log({ event: "screenshot", name, pid: app.pid });
  };
  return {
    app,
    localAppData,
    appData,
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

async function waitForSupport(session, route) {
  await session.send("Runtime.evaluate", { expression: `location.hash = ${JSON.stringify("#/" + route)}` });
  let state = null;
  for (let i = 0; i < 40; i += 1) {
    await sleep(300);
    state = await session.evaluate(`(() => {
      const heading = document.querySelector('.support-page h1');
      const text = document.body.innerText || '';
      const buttons = Array.from(document.querySelectorAll('button')).map((b) => (b.textContent || '').trim()).filter(Boolean);
      return {
        hash: location.hash,
        heading: heading?.textContent ?? null,
        hasPage: !!document.querySelector('.support-page'),
        unavailable: /protection store unavailable|保护存储不可用/i.test(text),
        inspectOnly: /inspect only|仅供检查/i.test(text),
        noReplay: /cannot execute or replay|不能执行或重放/i.test(text),
        buttons,
        textSample: text.replace(/\\s+/g, ' ').slice(0, 800),
      };
    })()`);
    if (state.hasPage && state.hash === `#/${route}`) break;
  }
  log({ event: "support_state", route, state });
  return state;
}

async function key(session, keyName, code, vk) {
  await session.send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key: keyName,
    code,
    windowsVirtualKeyCode: vk,
    nativeVirtualKeyCode: vk,
  });
  await session.send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key: keyName,
    code,
    windowsVirtualKeyCode: vk,
    nativeVirtualKeyCode: vk,
  });
}

async function captureAx(session, name) {
  try {
    await session.send("Accessibility.enable");
    const ax = await session.send("Accessibility.getFullAXTree", {});
    writeFileSync(join(evidence, "axtree", name + ".json"), JSON.stringify(ax, null, 2));
    const names = (ax.nodes ?? []).map((node) => node.name?.value ?? node.role?.value ?? "").filter(Boolean);
    const replay = names.filter((value) => /\b(run|retry|replay|execute)\b/i.test(value)
      && !/inspect-only|cannot execute/i.test(value));
    log({ event: "axtree_captured", name, nodes: ax.nodes?.length ?? 0, replayLike: replay.slice(0, 20) });
    return { nodes: ax.nodes?.length ?? 0, replayLike: replay };
  } catch (error) {
    log({ event: "axtree_error", name, error: String(error) });
    return { error: String(error) };
  }
}

async function widths(session, locale, prefix) {
  for (const width of [1440, 1024, 800, 390]) {
    await session.send("Emulation.setDeviceMetricsOverride", {
      width,
      height: width === 390 ? 844 : 900,
      deviceScaleFactor: 0,
      mobile: false,
    });
    await sleep(600);
    const metrics = await session.evaluate(`(() => ({ innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))()`);
    log({ event: "width_probe", locale, prefix, width, metrics });
    await session.screenshot(`${prefix}-${locale}-${width}`);
  }
  await session.send("Emulation.clearDeviceMetricsOverride");
  await sleep(300);
}

{
  log({ event: "integrity", ...integrityProbe() });

  const session = await launch({
    profileName: "localappdata",
    appDataName: "appdata",
    cdpPort: 9671,
    language: "en",
    protectionPaths: [join(evidence, "keep-display")],
    history: true,
  });
  mkdirSync(join(evidence, "keep-display"), { recursive: true });
  seedHealthyProtection(session.appData, [join(evidence, "keep-display")]);
  await session.send("Runtime.enable");
  await session.send("Page.enable");
  await sleep(2200);

  const protectEn = await waitForSupport(session, "protection");
  writeFileSync(join(evidence, "snapshots", "protection-en.txt"), protectEn.textSample ?? "", "utf8");
  await session.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(400);
  await session.screenshot("protect-en-1440");

  const draft = join(evidence, "keep-keyboard");
  mkdirSync(draft, { recursive: true });
  const addLabel = await session.evaluate(`(() => {
    const input = document.querySelector('.support-page form input[name="path"], .support-page form input');
    if (!input) return null;
    input.focus();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
    setter.call(input, ${JSON.stringify(draft)});
    input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: ${JSON.stringify(draft)} }));
    return {
      tag: document.activeElement?.tagName,
      name: input.getAttribute("name"),
      value: input.value,
      label: input.labels?.[0]?.textContent ?? null,
    };
  })()`);
  log({ event: "keyboard_focus_add", addLabel });
  await session.evaluate(`(() => { document.querySelector('.support-page form .primary-button')?.focus(); })()`);
  await key(session, " ", "Space", 32);
  await sleep(600);
  const confirm = await session.evaluate(`(() => {
    const dialog = document.querySelector('dialog[open]');
    return {
      open: !!dialog,
      title: dialog?.querySelector('h2')?.textContent ?? null,
      buttons: Array.from(dialog?.querySelectorAll('button') ?? []).map((b) => (b.textContent || '').trim()),
    };
  })()`);
  log({ event: "keyboard_confirm_add", confirm });
  if (!confirm.open) throw new Error("keyboard confirm dialog did not open");
  await session.screenshot("protect-en-keyboard-confirm");
  await key(session, "Escape", "Escape", 27);
  await sleep(400);

  await captureAx(session, "protection-en");

  const rulesEn = await waitForSupport(session, "rules");
  writeFileSync(join(evidence, "snapshots", "rules-en.txt"), rulesEn.textSample ?? "", "utf8");
  await session.screenshot("rules-en-1440");
  await session.evaluate(`(() => { document.querySelector('.support-page table button')?.click(); })()`);
  await sleep(400);
  await session.screenshot("rules-en-detail");
  const rulesAx = await captureAx(session, "rules-en");

  const historyEn = await waitForSupport(session, "history");
  writeFileSync(join(evidence, "snapshots", "history-en.txt"), historyEn.textSample ?? "", "utf8");
  const historyProbe = await session.evaluate(`(() => {
    const text = document.body.innerText || '';
    const buttons = Array.from(document.querySelectorAll('button')).map((b) => (b.textContent || '').trim());
    return {
      buttons,
      forbidden: ${forbiddenVisible.toString()}(text),
      executeControl: buttons.some((label) => /^(run|retry|replay|execute)$/i.test(label)),
    };
  })()`);
  log({ event: "history_redaction_en", historyProbe });
  await session.screenshot("history-en-1440");
  await session.evaluate(`(() => { document.querySelector('.support-page table button')?.focus(); })()`);
  await key(session, "Enter", "Enter", 13);
  await sleep(600);
  const historyDetail = await session.evaluate(`(() => {
    const text = document.body.innerText || '';
    return { forbidden: ${forbiddenVisible.toString()}(text), sample: text.replace(/\\s+/g, ' ').slice(0, 900) };
  })()`);
  log({ event: "history_detail_en", historyDetail });
  writeFileSync(join(evidence, "snapshots", "history-detail-en.txt"), historyDetail.sample ?? "", "utf8");
  await session.screenshot("history-en-detail-no-replay");
  const historyAx = await captureAx(session, "history-en");

  await session.send("Emulation.setEmulatedMedia", {
    features: [
      { name: "forced-colors", value: "active" },
      { name: "prefers-contrast", value: "more" },
    ],
  });
  await sleep(400);
  const hcProbe = await session.evaluate(`(() => ({
    forced: window.matchMedia('(forced-colors: active)').matches,
    contrast: window.matchMedia('(prefers-contrast: more)').matches,
  }))()`);
  log({ event: "high_contrast_probe", hcProbe });
  await session.screenshot("history-en-high-contrast");
  await waitForSupport(session, "protection");
  await session.screenshot("protect-en-high-contrast");
  await session.send("Emulation.setEmulatedMedia", {
    features: [
      { name: "forced-colors", value: "none" },
      { name: "prefers-contrast", value: "no-preference" },
    ],
  });

  await waitForSupport(session, "protection");
  await widths(session, "en", "protect");
  await waitForSupport(session, "rules");
  await widths(session, "en", "rules");
  await waitForSupport(session, "history");
  await widths(session, "en", "history");

  await session.send("Runtime.evaluate", { expression: "location.hash = '#/settings'" });
  await sleep(900);
  await session.evaluate(`(() => {
    const select = document.querySelector('.settings-panel select');
    if (!select) return null;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
    setter.call(select, 'zh-CN');
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value;
  })()`);
  await sleep(1400);
  const storeZh = join(session.localAppData, "DevSweep", "settings", "presentation-v1.json");
  log({ event: "locale_audit_zh", store: existsSync(storeZh) ? readFileSync(storeZh, "utf8").trim() : null });

  const protectZh = await waitForSupport(session, "protection");
  writeFileSync(join(evidence, "snapshots", "protection-zh.txt"), protectZh.textSample ?? "", "utf8");
  await session.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 0, mobile: false });
  await sleep(400);
  await session.screenshot("protect-zh-1440");
  await widths(session, "zh-CN", "protect");
  const rulesZh = await waitForSupport(session, "rules");
  writeFileSync(join(evidence, "snapshots", "rules-zh.txt"), rulesZh.textSample ?? "", "utf8");
  await session.screenshot("rules-zh-1440");
  await widths(session, "zh-CN", "rules");
  const historyZh = await waitForSupport(session, "history");
  writeFileSync(join(evidence, "snapshots", "history-zh.txt"), historyZh.textSample ?? "", "utf8");
  const historyZhProbe = await session.evaluate(`(() => {
    const text = document.body.innerText || '';
    const buttons = Array.from(document.querySelectorAll('button')).map((b) => (b.textContent || '').trim());
    return {
      buttons,
      forbidden: ${forbiddenVisible.toString()}(text),
      executeControl: buttons.some((label) => /执行|重放|重试|run|replay/i.test(label)),
    };
  })()`);
  log({ event: "history_redaction_zh", historyZhProbe });
  await session.screenshot("history-zh-1440");
  await widths(session, "zh-CN", "history");
  await captureAx(session, "history-zh");
  await session.close();

  const corruptBytes = Buffer.from("{not-json");
  const corruptApp = join(evidence, "appdata-corrupt", "devsweep");
  mkdirSync(corruptApp, { recursive: true });
  const corruptStore = join(corruptApp, "protected-paths.json");
  writeFileSync(corruptStore, corruptBytes);
  const corruptSession = await launch({
    profileName: "localappdata-corrupt",
    appDataName: "appdata-corrupt",
    cdpPort: 9672,
    language: "en",
    history: false,
  });
  await corruptSession.send("Runtime.enable");
  await corruptSession.send("Page.enable");
  await sleep(2000);
  const corruptUi = await waitForSupport(corruptSession, "protection");
  const afterCorrupt = readFileSync(corruptStore);
  const corruptPreserved = Buffer.compare(afterCorrupt, corruptBytes) === 0;
  log({ event: "corrupt_store", preserved: corruptPreserved, bytes: afterCorrupt.toString("utf8"), ui: corruptUi });
  copyFileSync(corruptStore, join(evidence, "stores", "corrupt-after-ui.json"));
  await corruptSession.screenshot("protect-en-corrupt-fail-closed");
  await corruptSession.close();
  if (!corruptPreserved) throw new Error("corrupt store bytes were not preserved");
  if (!corruptUi.unavailable) throw new Error("corrupt store did not fail closed in UI");

  const newerBytes = readFileSync(join(repo, "crates/devsweep-core/tests/fixtures/protection/newer-version.json"));
  const newerApp = join(evidence, "appdata-newer", "devsweep");
  mkdirSync(newerApp, { recursive: true });
  const newerStore = join(newerApp, "protected-paths.json");
  writeFileSync(newerStore, newerBytes);
  const newerSession = await launch({
    profileName: "localappdata-newer",
    appDataName: "appdata-newer",
    cdpPort: 9673,
    language: "zh-CN",
    history: false,
  });
  await newerSession.send("Runtime.enable");
  await newerSession.send("Page.enable");
  await sleep(2000);
  const newerUi = await waitForSupport(newerSession, "protection");
  const afterNewer = readFileSync(newerStore);
  const newerPreserved = Buffer.compare(afterNewer, newerBytes) === 0;
  log({ event: "newer_store", preserved: newerPreserved, bytes: afterNewer.toString("utf8"), ui: newerUi });
  copyFileSync(newerStore, join(evidence, "stores", "newer-after-ui.json"));
  await newerSession.screenshot("protect-zh-newer-fail-closed");
  await newerSession.close();
  if (!newerPreserved) throw new Error("newer store bytes were not preserved");
  if (!newerUi.unavailable) throw new Error("newer store did not fail closed in UI");

  const concurrentApp = join(evidence, "appdata-concurrent");
  const concurrentLocal = join(evidence, "localappdata-concurrent");
  mkdirSync(join(concurrentApp, "devsweep"), { recursive: true });
  mkdirSync(join(concurrentLocal, "DevSweep", "audit", "v1"), { recursive: true });
  const keepA = join(evidence, "keep-a");
  const keepB = join(evidence, "keep-b");
  mkdirSync(keepA, { recursive: true });
  mkdirSync(keepB, { recursive: true });
  const spawnProtect = (path, label) => {
    const child = spawn(cli, ["clean", "protect", "add", "--path", path, "--confirm", "--format", "json"], {
      env: { ...process.env, APPDATA: concurrentApp, LOCALAPPDATA: concurrentLocal },
      cwd: repo,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const chunks = [];
    const err = [];
    child.stdout.on("data", (d) => chunks.push(d));
    child.stderr.on("data", (d) => err.push(d));
    return new Promise((resolveChild) => {
      child.on("close", (code) => {
        resolveChild({
          label,
          pid: child.pid,
          code,
          stdout: Buffer.concat(chunks).toString("utf8"),
          stderr: Buffer.concat(err).toString("utf8"),
        });
      });
    });
  };
  const started = Date.now();
  const [writerA, writerB] = await Promise.all([spawnProtect(keepA, "a"), spawnProtect(keepB, "b")]);
  log({ event: "concurrent_writers", elapsedMs: Date.now() - started, writerA, writerB });
  const concurrentStore = join(concurrentApp, "devsweep", "protected-paths.json");
  const concurrentBytes = existsSync(concurrentStore) ? readFileSync(concurrentStore, "utf8") : "";
  writeFileSync(join(evidence, "stores", "concurrent-after-writers.json"), concurrentBytes);
  const listed = concurrentBytes ? JSON.parse(concurrentBytes) : { paths: [] };
  log({ event: "concurrent_store", listed, pathCount: listed.paths?.length ?? 0 });
  if ((listed.paths?.length ?? 0) !== 2) {
    throw new Error("concurrent writers lost an update; store has " + (listed.paths?.length ?? 0) + " paths");
  }

  const concurrentSession = await launch({
    profileName: "localappdata-concurrent",
    appDataName: "appdata-concurrent",
    cdpPort: 9674,
    language: "en",
    history: false,
  });
  await concurrentSession.send("Runtime.enable");
  await concurrentSession.send("Page.enable");
  await sleep(2000);
  const concurrentUi = await waitForSupport(concurrentSession, "protection");
  const rows = await concurrentSession.evaluate(`(() => Array.from(document.querySelectorAll('.support-page tbody tr td:first-child')).map((n) => n.textContent))()`);
  log({ event: "concurrent_ui", rows, ui: concurrentUi });
  await concurrentSession.screenshot("protect-en-concurrent-two-writers");
  await concurrentSession.close();
  if (!rows.some((row) => /keep-a/i.test(row)) || !rows.some((row) => /keep-b/i.test(row))) {
    throw new Error("desktop protection list did not show both concurrent writes");
  }

  if (historyProbe.executeControl || historyZhProbe.executeControl) {
    throw new Error("history exposed an execute/replay control");
  }
  if (historyProbe.forbidden.secretKeep || historyDetail.forbidden.secretKeep || historyZhProbe.forbidden.secretKeep) {
    throw new Error("history leaked a raw secret path");
  }
  if ((historyAx.replayLike ?? []).length || (rulesAx.replayLike ?? []).length) {
    log({ event: "ax_replay_warning", historyAx, rulesAx });
  }

  log({ event: "done", integrity: integrityProbe() });
  process.exit(0);
}
