// Production-only Clean component measurements in a fresh, task-owned browser.
// node tools/clean-render-benchmark.mjs NEW_OUTPUT_DIRECTORY BUILD_LABEL [PRESERVED_BUILD_DIRECTORY] [measure|visual]
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, join, extname, sep } from "node:path";
import { createHash } from "node:crypto";
import { cpus, release, platform } from "node:os";

const [directory, label, preservedBuild, taskMode = "measure"] = process.argv.slice(2);
if (!directory || !/^[a-zA-Z0-9_-]+$/.test(label ?? "")) throw new Error("New output directory and safe build label required");
if (!["measure", "visual"].includes(taskMode)) throw new Error("Unsupported benchmark task mode");
const out = resolve(directory);
await mkdir(out); // Existing profiles/evidence are never reused.
const profile = join(out, "profile");
await mkdir(profile);
const root = resolve(preservedBuild ?? `desktop/dist/clean-${label}`);
const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, "http://127.0.0.1").pathname);
    const path = resolve(root, `.${pathname}`);
    if (!path.startsWith(`${root}${sep}`)) { response.writeHead(403).end(); return; }
    const bytes = await readFile(path);
    response.setHeader("Content-Type", ({ ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".png": "image/png" })[extname(path)] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store"); response.end(bytes);
  } catch { response.writeHead(404).end(); }
});
await new Promise((resolve, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", resolve); });
const port = server.address().port;
const browser = spawn("C:/Program Files/Google/Chrome/Application/chrome.exe", [
  "--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
  "--no-first-run", "--no-default-browser-check", "--disable-extensions", "--disable-background-networking",
  "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "about:blank",
], { windowsHide: true, stdio: "ignore" });
let browserError;
browser.on("error", (error) => { browserError = error; });
const exit = new Promise((resolve) => browser.once("exit", resolve));
const alive = () => browser.pid && !browserError && browser.exitCode === null && browser.signalCode === null;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let socket;
let id = 0;
const pending = new Map();
function command(method, params = {}) {
  const key = ++id;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(key); reject(new Error(`CDP timeout: ${method}`)); }, 120_000);
    pending.set(key, { resolve: (value) => { clearTimeout(timer); resolve(value); }, reject: (error) => { clearTimeout(timer); reject(error); } });
    socket.send(JSON.stringify({ id: key, method, params }));
  });
}
async function evaluate(expression) {
  const result = await command("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
try {
  let page;
  for (let attempt = 0; attempt < 100 && !page; attempt += 1) {
    if (!alive()) throw new Error("Owned browser exited before debugging startup", { cause: browserError });
    try {
      const [debugPort, identity] = (await readFile(join(profile, "DevToolsActivePort"), "utf8")).trim().split(/\r?\n/);
      if (!/^\d+$/.test(debugPort) || !identity?.startsWith("/devtools/browser/")) throw new Error("Invalid owned endpoint");
      const origin = `http://127.0.0.1:${debugPort}`;
      const version = await (await fetch(`${origin}/json/version`, { signal: AbortSignal.timeout(1000) })).json();
      if (new URL(version.webSocketDebuggerUrl).pathname !== identity) throw new Error("Browser identity mismatch");
      const targets = await (await fetch(`${origin}/json/list`, { signal: AbortSignal.timeout(1000) })).json();
      page = targets.find((target) => target.type === "page" && new URL(target.webSocketDebuggerUrl).hostname === "127.0.0.1" && new URL(target.webSocketDebuggerUrl).port === debugPort);
    } catch { await sleep(100); }
  }
  if (!page || !alive()) throw new Error("Owned debugging page unavailable");
  socket = new WebSocket(page.webSocketDebuggerUrl);
  socket.addEventListener("close", () => { for (const value of pending.values()) value.reject(new Error("CDP closed")); pending.clear(); });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => { socket.close(); reject(new Error("CDP connection timeout")); }, 5000);
    socket.addEventListener("open", () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP connection failed")); }, { once: true });
  });
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data), request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    if (message.error) request.reject(new Error(JSON.stringify(message.error))); else request.resolve(message.result);
  });
  await command("Page.enable");
  await command("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1.25, mobile: false });
  const results = [];
  if (taskMode === "visual") {
    await command("Page.navigate", { url: `http://127.0.0.1:${port}/clean-render-benchmark.html` });
    for (let attempt = 0; attempt < 100 && !await evaluate("Boolean(window.cleanBenchmark)"); attempt += 1) await sleep(50);
    for (const locale of ["en", "zh-CN"]) for (const width of [390, 800, 1024, 1440]) for (const mode of ["review", "preview"]) {
      await command("Emulation.setDeviceMetricsOverride", { width, height: 900, deviceScaleFactor: 1.25, mobile: false });
      const shown = await evaluate(`window.cleanBenchmark.show(${JSON.stringify(mode)}, ${JSON.stringify(locale)})`);
      const layout = await evaluate(`({overflow:document.documentElement.scrollWidth>innerWidth,rows:document.querySelectorAll('tbody tr').length,buttons:[...document.querySelectorAll('.target-pagination button')].map(b=>({label:b.textContent,disabled:b.disabled}))})`);
      const name = `${locale}-${width}-${mode}`;
      const png = await command("Page.captureScreenshot", { format: "png" });
      await writeFile(join(out, `${name}.png`), Buffer.from(png.data, "base64"));
      results.push({ name, locale, width, mode, ...shown, ...layout, png_sha256: createHash("sha256").update(Buffer.from(png.data, "base64")).digest("hex") });
    }
    console.log(JSON.stringify({ visual_cases: results.length, overflow_cases: results.filter((result) => result.overflow).length }));
  } else for (const count of [100, 1000, 10000]) for (const mode of ["review", "preview"]) {
    await command("Page.navigate", { url: `http://127.0.0.1:${port}/clean-render-benchmark.html?count=${count}&mode=${mode}` });
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt += 1) {
      if (await evaluate("Boolean(window.cleanBenchmark)")) { ready = true; break; }
      await sleep(50);
    }
    if (!ready) throw new Error("Production harness not loaded");
    const result = await evaluate(`window.cleanBenchmark.run(${count}, ${JSON.stringify(mode)})`);
    if (result.retained_targets !== count) throw new Error("Benchmark dropped targets");
    results.push(result);
    await writeFile(join(out, `${count}-${mode}.json`), JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ count, mode, p95_ms: result.p95_ms, dom_elements: result.dom_elements, rendered_rows: result.rendered_rows }));
  }
  const html = await readFile(join(root, "clean-render-benchmark.html"));
  await writeFile(join(out, "record.json"), JSON.stringify({ timestamp: new Date().toISOString(), label, task_mode: taskMode, results,
    html_sha256: createHash("sha256").update(html).digest("hex"),
    host: { platform: platform(), release: release(), cpu: cpus()[0]?.model, logical_processors: cpus().length },
  }, null, 2));
} finally {
  if (socket?.readyState === WebSocket.OPEN && alive()) { await Promise.race([command("Browser.close").catch(() => {}), sleep(1000)]); socket.close(); }
  server.close();
  if (alive()) await Promise.race([exit, sleep(5000)]);
  const forced = Boolean(alive());
  if (forced) { browser.kill(); await Promise.race([exit, sleep(5000)]); }
  const teardown = { pid: browser.pid, forced, exit_code: browser.exitCode, signal: browser.signalCode, error: browserError?.message, lifecycle_pass: !forced && !browserError && browser.exitCode === 0 };
  await writeFile(join(out, "teardown.json"), JSON.stringify(teardown, null, 2));
  if (!teardown.lifecycle_pass) process.exitCode = 1;
}
