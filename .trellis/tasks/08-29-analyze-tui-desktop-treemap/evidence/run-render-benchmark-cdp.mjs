// CDP driver for the DevSweep Analyze render benchmark (task evidence tooling).
// Launches the system Chrome/Edge in new-headless mode, loads the vite preview
// benchmark page, waits for the benchmark promise, and writes the raw result.
// Usage: node run-render-benchmark-cdp.mjs <pageUrl> <outJson> <outConsoleLog>
import { spawn } from "node:child_process";
import { writeFileSync, appendFileSync, rmSync } from "node:fs";

const [pageUrl, outJson, outConsole] = process.argv.slice(2);
if (!pageUrl || !outJson || !outConsole) {
  console.error("usage: node run-render-benchmark-cdp.mjs <pageUrl> <outJson> <outConsoleLog>");
  process.exit(2);
}

const candidate = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
];
const { existsSync } = await import("node:fs");
const binary = candidate.find((path) => existsSync(path));
if (!binary) throw new Error("no Chrome/Edge binary found");

const port = 9337;
const profile = process.env.TEMP + "\\devsweep-bench-profile-" + Date.now();
const chrome = spawn(binary, [
  "--headless=new",
  "--remote-debugging-port=" + port,
  "--user-data-dir=" + profile,
  "--no-first-run",
  "--disable-sync",
  "--window-size=1440,900",
  "about:blank",
], { stdio: ["ignore", "pipe", "pipe"] });

const listening = new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error("chrome did not report DevTools port")), 20000);
  const onLine = (line) => {
    const match = /DevTools listening on (ws:\/\/\S+)/.exec(line);
    if (match) { clearTimeout(timer); resolve(match[1]); }
  };
  chrome.stderr.on("data", (data) => String(data).split(/\r?\n/).forEach(onLine));
  chrome.stdout.on("data", (data) => String(data).split(/\r?\n/).forEach(onLine));
  chrome.on("exit", (code) => reject(new Error("chrome exited early: " + code)));
});

const browserWs = await listening;
appendFileSync(outConsole, JSON.stringify({ event: "browser_ws", value: browserWs }) + "\n");

// Discover the HTTP endpoint port from the ws URL.
const httpPort = new URL(browserWs).port;
const newTab = await fetch(`http://127.0.0.1:${httpPort}/json/new?${encodeURIComponent(pageUrl)}`, { method: "PUT" })
  .then((response) => response.json());
appendFileSync(outConsole, JSON.stringify({ event: "tab_created", target: newTab.id }) + "\n");

const ws = new WebSocket(newTab.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = (error) => reject(new Error("ws error")); });

let seq = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++seq;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    setTimeout(() => {
      if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout: " + method)); }
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
    appendFileSync(outConsole, JSON.stringify({ event: "console", type: message.params.type, text }) + "\n");
  } else if (message.method === "Runtime.exceptionThrown") {
    appendFileSync(outConsole, JSON.stringify({ event: "exception", detail: message.params.exceptionDetails }) + "\n");
  }
};

await send("Runtime.enable");
await send("Page.enable");

const done = await (async () => {
  const deadline = Date.now() + 300000;
  while (Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 3000));
    const check = await send("Runtime.evaluate", {
      expression: "(() => ({ done: Boolean(window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__), value: window.__DEVSWEEP_ANALYZE_BENCHMARK_RESULT__ ?? null }))()",
      returnByValue: true,
    });
    const value = check.result.value;
    if (value.done) return value;
  }
  throw new Error("benchmark did not finish within 300s");
})();

const result = done.value;
writeFileSync(outJson, JSON.stringify(result, null, 2));
appendFileSync(outConsole, JSON.stringify({ event: "final_status", status: result.status }) + "\n");

ws.close();
chrome.kill();
try { rmSync(profile, { recursive: true, force: true }); } catch {}
console.log("status:", result.status);
process.exit(result.status === "pass" ? 0 : 1);
