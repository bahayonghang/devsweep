import { spawn } from "node:child_process";
import { writeFileSync, writeSync } from "node:fs";
import { createInterface } from "node:readline";

const action = process.argv[2];
const port = Number(process.argv[3] || "9588");
const exe = process.argv[4];
const profile = process.argv[5];
const scale = process.argv[6] || "";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function cdp() {
  const deadline = Date.now() + 120000;
  let targets = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json`);
      if (r.ok) {
        targets = await r.json();
        break;
      }
    } catch {}
    await sleep(400);
  }
  if (!targets) throw new Error("cdp timeout");
  const page = targets.find((t) => t.type === "page" && /tauri\\.localhost|analyze-render-benchmark|:4181/i.test(t.url || ""))
    ?? targets.find((t) => t.type === "page");
  if (!page) throw new Error("no page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("ws")); });
  let seq = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    }
  };
  const send = (method, params = {}) => {
    const id = ++seq;
    ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      setTimeout(() => { if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout " + method)); } }, 30000);
    });
  };
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
    return result.result.value;
  };
  await send("Runtime.enable");
  await send("Page.enable");
  return { send, evaluate, ws };
}

if (action === "launch") {
  const extra = scale ? ` --force-device-scale-factor=${scale}` : "";
  const app = spawn(exe, [], {
    env: { ...process.env, LOCALAPPDATA: profile, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-allow-origins=*${extra}` },
    stdio: "ignore",
    cwd: process.cwd(),
    detached: true,
    windowsHide: false,
  });
  app.unref();
  writeFileSync(process.argv[7], JSON.stringify({ pid: app.pid, port }), "utf8");
  process.exit(0);
}

async function dispatch(session, argv) {
  const actionName = argv[0];
  const arg = (index) => argv[index] ?? "";
  if (actionName === "hash") {
    await session.send("Runtime.evaluate", { expression: "location.hash = " + JSON.stringify(arg(2)) });
    await sleep(800);
    return "";
  }
  if (actionName === "click") {
    const pattern = arg(2);
    const started = Date.now();
    const clicked = await session.evaluate(`(() => {
      const re = new RegExp(${JSON.stringify(pattern)});
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    return JSON.stringify({ clicked, elapsed_ms: Date.now() - started });
  }
  if (actionName === "state") {
    const value = await session.evaluate(`(() => ({
      hash: location.hash,
      status: document.querySelector(".status-mode")?.getAttribute("data-status") ?? null,
      analyze: document.querySelector(".analyze-mode") ? document.querySelector(".analyze-mode").getAttribute("data-status") : null,
      text: (document.body.innerText || "").slice(0, 4000),
    }))()`);
    return JSON.stringify(value);
  }
  if (actionName === "fill") {
    const selector = arg(2);
    const value = arg(3);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${JSON.stringify(value)});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    return JSON.stringify({ filled });
  }
  if (actionName === "eval") {
    const value = await session.evaluate(arg(2));
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "waitEval") {
    const expression = arg(2);
    const deadline = Date.now() + Number(arg(3) || 180000);
    let value = null;
    while (Date.now() < deadline) {
      try { value = await session.evaluate(expression); } catch { value = null; }
      if (value) break;
      await sleep(500);
    }
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "analyzeCancel") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const clickNamed = async (source) => {
      const srcJson = JSON.stringify(source);
      return session.evaluate(`(() => {
        const re = new RegExp(${srcJson});
        const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
        if (button) button.click();
        return !!button;
      })()`);
    };
    const statusOf = () => session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    const startClicked = await clickNamed("Analyze path|分析路径");
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await statusOf();
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    const t0 = Date.now();
    const cancelClicked = await clickNamed("Cancel analysis|取消分析");
    let joined = false;
    let status = null;
    for (let i = 0; i < 400; i += 1) {
      status = await statusOf();
      if (["canceled", "partial", "complete", "idle", "empty"].includes(status || "")) {
        joined = true;
        break;
      }
      await sleep(10);
    }
    return JSON.stringify({
      filled, startClicked, cancelClicked, started, joined, status, elapsed_ms: Date.now() - t0,
    });
  }
  if (actionName === "analyzeStart") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const startClicked = await session.evaluate(`(() => {
      const re = /Analyze path|分析路径/;
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    return JSON.stringify({ filled, startClicked, started });
  }
  if (actionName === "analyzePoll") {
    const status = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    return JSON.stringify({ status });
  }
  if (actionName === "close") {
    try { await session.send("Browser.close"); } catch {}
    return "";
  }
  throw new Error("unknown action " + actionName);
}

if (action === "serve") {
  const session = await cdp();
  const ping = setInterval(() => { session.evaluate("void 0").catch(() => {}); }, 20000);
  writeSync(1, "{\"event\":\"ready\"}\n");
  const rl = createInterface({ input: process.stdin });
  for await (const line of rl) {
    if (!line.trim()) continue;
    let msg = {};
    try { msg = JSON.parse(line); } catch {
      writeSync(1, "{\"id\":\"\",\"error\":\"bad json\"}\n");
      continue;
    }
    try {
      if ((msg.argv || [])[0] === "close") {
        clearInterval(ping);
        await dispatch(session, msg.argv || ["close"]);
        writeSync(1, JSON.stringify({ id: msg.id || "", result: "" }) + "\n");
        process.exit(0);
      }
      const result = await dispatch(session, msg.argv || []);
      writeSync(1, JSON.stringify({ id: msg.id || "", result }) + "\n");
    } catch (error) {
      writeSync(1, JSON.stringify({ id: msg.id || "", error: String(error && error.message ? error.message : error) }) + "\n");
    }
  }
  process.exit(0);
}

const session = await cdp();
const output = await dispatch(session, process.argv.slice(2));
if (output) process.stdout.write(output);
session.ws.close();