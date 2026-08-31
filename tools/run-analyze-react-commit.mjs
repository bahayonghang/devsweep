// Collect Analyze React commit p95 from the product Vite benchmark page.
// Headless Chrome/Edge, free debugging port, DevTools-listening handshake.
// The page itself runs 5 warm-ups + 30 measured commits.
// Usage: node run-analyze-react-commit.mjs <pageUrl> <outJson> <outConsoleLog>
import { spawn } from "node:child_process";
import { existsSync, writeFileSync, appendFileSync, rmSync } from "node:fs";
import { createServer } from "node:net";

const [pageUrl, outJson, outConsole] = process.argv.slice(2);
if (!pageUrl || !outJson || !outConsole) {
  console.error("usage: node run-analyze-react-commit.mjs <pageUrl> <outJson> <outConsoleLog>");
  process.exit(2);
}

const candidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
];
const binary = candidates.find((path) => existsSync(path));
if (!binary) throw new Error("no Chrome/Edge binary found");

function log(event, extra = {}) {
  appendFileSync(outConsole, JSON.stringify({ event, ...extra, at: new Date().toISOString() }) + "\n");
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => {
        if (error) reject(error);
        else resolve(port);
      });
    });
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function collectOnce(attempt) {
  const port = await freePort();
  if (!port || port === 9223) throw new Error("refusing pinned or missing debug port");
  const profile = `${process.env.TEMP}\\devsweep-react-commit-${Date.now()}-${attempt}`;
  log("launch", { attempt, port, binary, profile });
  const chrome = spawn(binary, [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--remote-allow-origins=*`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-sync",
    "--window-size=1440,900",
    "about:blank",
  ], { stdio: ["ignore", "pipe", "pipe"] });

  let browserWs = null;
  try {
    browserWs = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("browser did not report DevTools port")), 20000);
      const onLine = (chunk) => {
        const text = String(chunk);
        for (const line of text.split(/\r?\n/)) {
          const match = /DevTools listening on (ws:\/\/\S+)/.exec(line);
          if (match) {
            clearTimeout(timer);
            resolve(match[1]);
          }
        }
      };
      chrome.stderr.on("data", onLine);
      chrome.stdout.on("data", onLine);
      chrome.on("exit", (code) => reject(new Error("browser exited early: " + code)));
    });
    log("browser_ws", { attempt, value: browserWs });

    const httpPort = new URL(browserWs).port;
    const newTab = await fetch(`http://127.0.0.1:${httpPort}/json/new?${encodeURIComponent(pageUrl)}`, { method: "PUT" })
      .then((response) => {
        if (!response.ok) throw new Error("json/new " + response.status);
        return response.json();
      });
    log("tab_created", { attempt, target: newTab.id, debuggerUrl: newTab.webSocketDebuggerUrl });

    const ws = new WebSocket(newTab.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = () => reject(new Error("ws error"));
    });

    let seq = 0;
    const pending = new Map();
    function send(method, params = {}) {
      const id = ++seq;
      ws.send(JSON.stringify({ id, method, params }));
      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        setTimeout(() => {
          if (pending.has(id)) {
            pending.delete(id);
            reject(new Error("cdp timeout: " + method));
          }
        }, 30000);
      });
    }
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.id && pending.has(message.id)) {
        const { resolve, reject } = pending.get(message.id);
        pending.delete(message.id);
        if (message.error) reject(new Error(message.error.message));
        else resolve(message.result);
      } else if (message.method === "Runtime.consoleAPICalled") {
        const text = (message.params.args ?? []).map((arg) => arg.value ?? arg.description ?? "").join(" ");
        log("console", { type: message.params.type, text });
      } else if (message.method === "Runtime.exceptionThrown") {
        log("exception", { detail: message.params.exceptionDetails });
      }
    };

    await send("Runtime.enable");
    await send("Page.enable");

    const deadline = Date.now() + 180000;
    let result = null;
    while (Date.now() < deadline) {
      await sleep(2000);
      const check = await send("Runtime.evaluate", {
        expression: "(() => ({ done: Boolean(window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__), value: window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__ ?? null }))()",
        returnByValue: true,
      });
      const value = check.result.value;
      if (value && value.done) {
        result = value.value;
        break;
      }
    }
    ws.close();
    if (!result) throw new Error("benchmark did not finish within 180s");
    return result;
  } finally {
    try { chrome.kill(); } catch {}
    try { rmSync(profile, { recursive: true, force: true }); } catch {}
  }
}

const attempts = 3;
let lastError = null;
for (let attempt = 1; attempt <= attempts; attempt++) {
  try {
    const result = await collectOnce(attempt);
    writeFileSync(outJson, JSON.stringify(result, null, 2));
    log("final_status", { status: result.status, p95: result.react_commit_p95_ms, samples: (result.react_commit_samples_ms || []).length });
    const samples = Array.isArray(result.react_commit_samples_ms) ? result.react_commit_samples_ms.length : 0;
    if (samples !== 30) {
      console.error("expected 30 measured samples, got " + samples);
      process.exit(1);
    }
    console.log("status:", result.status, "p95:", result.react_commit_p95_ms);
    process.exit(0);
  } catch (error) {
    lastError = error;
    log("attempt_failed", { attempt, error: String(error && error.message ? error.message : error) });
    console.error("react-commit attempt " + attempt + " failed: " + (error && error.message ? error.message : error));
    await sleep(2000);
  }
}

throw lastError || new Error("react-commit collection failed");
