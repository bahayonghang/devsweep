// Native acceptance matrix driver for the release desktop binary.
//
// Launches the hashed release executable with an isolated LOCALAPPDATA, one
// WebView2 device scale per launch, and a seeded presentation language. It
// drives the page through the WebView2 DevTools protocol, records per-mode
// facts and screenshots, then closes the window with WM_CLOSE and records
// leftover processes. It only navigates and reads: it never starts a scan,
// dry run, execute, uninstall, startup toggle, or Recycle Bin move.
//
// Usage: node tools/native-acceptance.mjs <exe> <outDir> [matrix|cancel]
//
// "cancel" runs five start/cancel/join/restart cycles per read-only operation
// (Clean scan, Software inventory, Optimize catalogue) in one launch.

import { spawn, execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";

const [exeArg, outArg, runKind = "matrix"] = process.argv.slice(2);
if (!exeArg || !outArg) {
  console.error("usage: node tools/native-acceptance.mjs <exe> <outDir>");
  process.exit(2);
}
const exe = resolve(exeArg);
const out = resolve(outArg);
const shots = join(out, "screenshots");
mkdirSync(shots, { recursive: true });

const MODES = ["clean", "software", "optimize", "analyze", "status"];
const LOCALES = ["en", "zh-CN"];
// "actual" omits the scale argument, so the WebView follows the unchanged
// Windows display scale of the host.
const SCALES = ["actual", 1, 1.25, 1.5, 2];
const WIDTHS = [390, 800, 1024, 1440];
const PORT = 9333;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function processRows() {
  const text = execFileSync(
    "powershell.exe",
    [
      "-NoLogo",
      "-NoProfile",
      "-Command",
      "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath,CommandLine | ConvertTo-Json -Compress",
    ],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  return JSON.parse(text);
}

function treeOf(rows, rootPid) {
  const keep = new Set([rootPid]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const row of rows) {
      if (keep.has(row.ParentProcessId) && !keep.has(row.ProcessId)) {
        keep.add(row.ProcessId);
        grew = true;
      }
    }
  }
  return rows
    .filter((row) => keep.has(row.ProcessId))
    .map((row) => ({
      pid: row.ProcessId,
      ppid: row.ParentProcessId,
      name: row.Name,
      path: row.ExecutablePath,
    }));
}

// Processes that belong to this launch: the app itself or any process whose
// command line names the isolated profile (WebView2 user data lives there).
function launchProcesses(rows, profile) {
  const needle = profile.toLowerCase();
  return rows
    .filter(
      (row) =>
        (row.ExecutablePath ?? "").toLowerCase() === exe.toLowerCase() ||
        (row.CommandLine ?? "").toLowerCase().includes(needle),
    )
    .map((row) => ({ pid: row.ProcessId, name: row.Name }));
}

async function connect() {
  let targets = null;
  for (let i = 0; i < 150 && !targets; i += 1) {
    try {
      const res = await fetch(`http://127.0.0.1:${PORT}/json/list`);
      const list = await res.json();
      if (list.some((t) => t.type === "page" && /tauri\.localhost/.test(t.url)))
        targets = list;
    } catch {
      /* not up yet */
    }
    if (!targets) await sleep(200);
  }
  if (!targets) throw new Error("cdp timeout");
  const page = targets.find(
    (t) =>
      t.type === "page" &&
      /tauri\.localhost/.test(t.url) &&
      !/hud\.html/.test(t.url),
  );
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((ok, fail) => {
    ws.onopen = ok;
    ws.onerror = () => fail(new Error("ws"));
  });
  let seq = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { ok, fail } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) fail(new Error(msg.error.message));
      else ok(msg.result);
    }
  };
  const send = (method, params = {}) =>
    new Promise((ok, fail) => {
      const id = ++seq;
      pending.set(id, { ok, fail });
      ws.send(JSON.stringify({ id, method, params }));
      setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id);
          fail(new Error(`cdp timeout ${method}`));
        }
      }, 30000);
    });
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.text);
    return r.result.value;
  };
  await send("Page.enable");
  return { send, evaluate, ws };
}

async function screenshot(session, name) {
  const r = await session.send("Page.captureScreenshot", { format: "png" });
  const file = join(shots, `${name}.png`);
  writeFileSync(file, Buffer.from(r.data, "base64"));
  return `screenshots/${name}.png`;
}

// Facts about the current page that do not depend on host data.
const FACTS = `(() => {
  const root = document.documentElement;
  const tab = document.querySelector('[role="tab"][aria-selected="true"]');
  const mode = document.querySelector('[class$="-mode"][data-status]');
  const text = document.body.innerText;
  const rawKeys = (text.match(/\\b[a-z]+\\.v1\\.[a-z0-9_.]+/g) ?? []).slice(0, 5);
  const canvas = document.querySelector('canvas');
  return {
    hash: location.hash,
    locale: document.querySelector('.app-shell')?.getAttribute('data-locale') ?? null,
    title: document.title,
    dpr: window.devicePixelRatio,
    inner: [window.innerWidth, window.innerHeight],
    horizontal_overflow: root.scrollWidth > root.clientWidth + 1,
    active_tab: tab?.textContent?.trim() ?? null,
    mode_status: mode?.getAttribute('data-status') ?? null,
    raw_keys: rawKeys,
    has_undefined: /\\bundefined\\b|\\bNaN\\b/.test(text),
    planet_canvas: canvas ? [canvas.width, canvas.height] : null,
    han_chars: (text.match(/[\\u4e00-\\u9fff]/g) ?? []).length,
  };
})()`;

async function visit(session, mode) {
  await session.evaluate(`location.hash = '#/${mode}'`);
  await sleep(1500);
  return session.evaluate(FACTS);
}

async function pressKey(session, key, code, windowsVirtualKeyCode) {
  // A button activates on Enter only when the key event carries its text.
  const text = key === "Enter" ? String.fromCharCode(13) : undefined;
  await session.send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode, text });
  await session.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode });
}

async function keyboardRow(session) {
  // The capsule uses manual activation: arrows move focus, Enter activates.
  await session.evaluate(`location.hash = '#/clean'`);
  await sleep(800);
  const before = await session.evaluate(
    `(() => { const t = document.querySelector('[role="tab"][aria-selected="true"]'); t?.focus(); return location.hash; })()`,
  );
  const focusFacts = `({ hash: location.hash, focused: document.activeElement?.getAttribute('role') === 'tab' ? document.activeElement.textContent.trim() : null, focus_visible: document.activeElement?.matches(':focus-visible') ?? false })`;
  await pressKey(session, "ArrowRight", "ArrowRight", 39);
  await sleep(500);
  const after_arrow = await session.evaluate(focusFacts);
  await pressKey(session, "Enter", "Enter", 13);
  await sleep(1200);
  const after_enter = await session.evaluate(focusFacts);
  await pressKey(session, "End", "End", 35);
  await sleep(300);
  const after_end = await session.evaluate(focusFacts);
  return { before, after_arrow, after_enter, after_end };
}

function launch(locale, scale, label) {
  const profile = join(out, "profiles", label);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "DevSweep", "settings"), { recursive: true });
  writeFileSync(
    join(profile, "DevSweep", "settings", "presentation-v1.json"),
    JSON.stringify({ schema_version: 1, language: locale }),
    "utf8",
  );
  const extra = scale === "actual" ? "" : ` --force-device-scale-factor=${scale}`;
  const app = spawn(exe, [], {
    env: {
      ...process.env,
      LOCALAPPDATA: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT} --remote-allow-origins=*${extra}`,
    },
    stdio: "ignore",
    detached: true,
    windowsHide: false,
  });
  app.unref();
  return { pid: app.pid, profile };
}

async function closeAndCheck(pid, profile) {
  const started = Date.now();
  try {
    execFileSync("taskkill.exe", ["/PID", String(pid)], { stdio: "ignore" });
  } catch {
    /* reported below */
  }
  let alive = true;
  for (let i = 0; i < 100 && alive; i += 1) {
    await sleep(100);
    alive = processRows().some((row) => row.ProcessId === pid);
  }
  await sleep(1500);
  const leftovers = launchProcesses(processRows(), profile);
  return { exited: !alive, exit_ms: Date.now() - started, leftovers };
}

function windowTitle(pid) {
  return execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-Command", `(Get-Process -Id ${pid}).MainWindowTitle`], { encoding: "utf8" }).trim();
}

// Each operation: route, start control, and DOM predicates for running,
// acknowledgement (cancel requested), and join (terminal, not running).
const CANCEL_OPERATIONS = [
  {
    operation: "Clean scan",
    route: "clean",
    // After a cancel, the stopped stage restarts through its rescan link.
    start: `(document.querySelector('.clean-mode .stage-link:not(:disabled)') ?? document.querySelector('.clean-mode .stage-action'))?.click()`,
    cancel: `document.querySelector('.clean-mode .stage-action')?.click()`,
    running: `!!document.querySelector('.clean-stage[aria-busy="true"]')`,
    ack: `!!document.querySelector('.clean-stage[aria-busy="true"] .stage-action:disabled')`,
    joined: `!document.querySelector('.clean-stage[aria-busy="true"]')`,
  },
  {
    operation: "Software inventory",
    route: "software",
    start: `document.querySelector('.software-mode .stage-action')?.click()`,
    cancel: `document.querySelector('.software-mode .stage-action')?.click()`,
    running: `document.querySelector('.software-mode')?.getAttribute('data-status') === 'loading'`,
    ack: `document.querySelector('.software-mode')?.getAttribute('data-status') === 'canceling'`,
    joined: `!['loading', 'canceling'].includes(document.querySelector('.software-mode')?.getAttribute('data-status'))`,
  },
  {
    operation: "Optimize catalogue",
    route: "optimize",
    // A loaded catalogue offers no in-place refresh; re-enter the mode first.
    reenter: true,
    start: `document.querySelector('.optimize-mode .stage-action')?.click()`,
    cancel: `document.querySelector('.optimize-mode .stage-action')?.click()`,
    running: `!['idle', 'ready', 'canceling'].includes(document.querySelector('.optimize-mode')?.getAttribute('data-status'))`,
    ack: `document.querySelector('.optimize-mode')?.getAttribute('data-status') === 'canceling'`,
    joined: `['idle', 'ready'].includes(document.querySelector('.optimize-mode')?.getAttribute('data-status'))`,
  },
];

async function waitFor(session, expression, timeoutMs, stepMs = 5) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await session.evaluate(expression)) return Date.now() - start;
    await sleep(stepMs);
  }
  return null;
}

async function cancelRows(session, pid) {
  const rows = [];
  for (const op of CANCEL_OPERATIONS) {
    await session.evaluate(`location.hash = '#/${op.route}'`);
    await sleep(1500);
    await waitFor(session, op.joined, 30000, 50);
    const reps = [];
    for (let rep = 1; rep <= 5; rep += 1) {
      if (op.reenter && rep > 1) {
        await session.evaluate(`location.hash = '#/analyze'`);
        await sleep(800);
        await session.evaluate(`location.hash = '#/${op.route}'`);
        await sleep(1200);
      }
      await session.evaluate(op.start);
      const startedMs = await waitFor(session, op.running, 10000);
      if (startedMs === null) {
        // The operation finished before a cancel could be requested.
        reps.push({ rep, started: false, finished_before_cancel: await session.evaluate(op.joined) });
        await sleep(500);
        continue;
      }
      const requestedAt = Date.now();
      await session.evaluate(op.cancel);
      const ackMs = await waitFor(session, `(${op.ack}) || (${op.joined})`, 5000, 2);
      const ackAt = Date.now();
      const joinMs = await waitFor(session, op.joined, 20000, 10);
      await sleep(1000);
      const children = treeOf(processRows(), pid)
        .filter((row) => row.pid !== pid && !/msedgewebview2\.exe/i.test(row.name))
        .map((row) => row.name);
      reps.push({ rep, started: true, request_ack_ms: ackMs, ack_join_ms: joinMs === null ? null : Date.now() - ackAt - 1000, joined: joinMs !== null, requested_at: requestedAt, non_webview_children_after_join: children });
    }
    rows.push({ operation: op.operation, reps });
  }
  return rows;
}

const rows = [];
if (runKind === "cancel") {
  const label = "en-cancel";
  console.log(`launch ${label}`);
  const { pid, profile } = launch("en", "actual", label);
  const session = await connect();
  await sleep(1500);
  const row = { label, pid, window_title: windowTitle(pid), tree: treeOf(processRows(), pid) };
  row.cancel = await cancelRows(session, pid);
  session.ws.close();
  row.close = await closeAndCheck(pid, profile);
  writeFileSync(join(out, "cancel.json"), JSON.stringify(row, null, 2), "utf8");
  console.log("done: cancel");
  process.exit(0);
}
for (const locale of LOCALES) {
  for (const scale of SCALES) {
    const label = scale === "actual" ? `${locale}-actual` : `${locale}-s${Math.round(scale * 100)}`;
    console.log(`launch ${label}`);
    const { pid, profile } = launch(locale, scale, label);
    const session = await connect();
    await sleep(1500);
    const tree = treeOf(processRows(), pid);
    const launchRow = {
      label,
      locale,
      scale,
      pid,
      tree,
      modes: {},
      widths: {},
    };
    for (const mode of MODES) {
      const facts = await visit(session, mode);
      facts.screenshot = await screenshot(session, `${label}-${mode}`);
      launchRow.modes[mode] = facts;
    }
    if (scale === 1) {
      launchRow.keyboard = await keyboardRow(session);
      for (const width of WIDTHS) {
        await session.send("Emulation.setDeviceMetricsOverride", {
          width,
          height: 800,
          deviceScaleFactor: 0,
          mobile: false,
        });
        await sleep(500);
        launchRow.widths[width] = {};
        for (const mode of MODES) {
          const facts = await visit(session, mode);
          facts.screenshot = await screenshot(
            session,
            `${label}-w${width}-${mode}`,
          );
          launchRow.widths[width][mode] = facts;
        }
      }
      await session.send("Emulation.clearDeviceMetricsOverride");
    }
    session.ws.close();
    launchRow.close = await closeAndCheck(pid, profile);
    rows.push(launchRow);
    writeFileSync(
      join(out, "matrix.json"),
      JSON.stringify(rows, null, 2),
      "utf8",
    );
  }
}
console.log(`done: ${rows.length} launches`);
