// Production component benchmark over local builds. Owns only its isolated
// browser/profile and loopback server; no user browser, app or cleanup operation.
// node tools/planet-render-benchmark.mjs OUTPUT_DIR [quick|full|matrix|interaction|navigation] [modes...]
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve, join, extname, sep } from "node:path";
import { cpus, platform, release } from "node:os";
import { createHash } from "node:crypto";

const [outArg, length = "full", ...selectedModes] = process.argv.slice(2);
if (!outArg) throw new Error("Output directory required");
const out = resolve(outArg);
// Refuse reused evidence/profile directories; their contents establish ownership.
await mkdir(out);
const profile = join(out, "profile");
await mkdir(profile);
const modes = selectedModes.length ? selectedModes : ["baseline", "production"];
const root = resolve("desktop/dist");
const server = createServer(async (request, response) => {
  const pathname = decodeURIComponent(new URL(request.url, "http://127.0.0.1").pathname);
  const path = resolve(root, `.${pathname}`);
  if (!path.startsWith(`${root}${sep}`)) { response.writeHead(403).end(); return; }
  try {
    const bytes = await readFile(path);
    response.setHeader("Content-Type", ({ ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".png": "image/png" })[extname(path)] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store");
    response.end(bytes);
  } catch { response.writeHead(404).end(); }
});
await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});
const port = server.address().port;
const browser = spawn("C:/Program Files/Google/Chrome/Application/chrome.exe", [
  "--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
  "--no-first-run", "--no-default-browser-check", "--disable-extensions",
  "--enable-precise-memory-info",
  "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "about:blank",
], { windowsHide: true, stdio: "ignore" });
let browserError;
browser.on("error", (error) => { browserError = error; });
const browserExited = new Promise((resolve) => browser.once("exit", resolve));
const isOwnedBrowserAlive = () => browser.pid && !browserError && browser.exitCode === null && browser.signalCode === null;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let socket;
const pending = new Map();
let nextId = 0;
async function command(method, params = {}) {
  const id = ++nextId;
  const result = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP command timed out: ${method}`)); }, 30000);
    pending.set(id, { resolve: (value) => { clearTimeout(timer); resolve(value); }, reject: (error) => { clearTimeout(timer); reject(error); } });
  });
  socket.send(JSON.stringify({ id, method, params }));
  return result;
}
async function evaluate(expression) {
  const result = await command("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
const percentile = (values, q) => [...values].sort((a, b) => a - b)[Math.max(0, Math.ceil(values.length * q) - 1)] ?? null;
async function loaded() {
  for (let count = 0; count < 300; count += 1) {
    const ready = await evaluate("window.planetBenchmark?.snapshot().first_frame_ms != null");
    if (ready) return;
    await sleep(50);
  }
  throw new Error("Planet did not draw a frame in 15 seconds");
}
try {
  let targets;
  for (let attempt = 0; attempt < 100 && !targets; attempt += 1) {
    if (!isOwnedBrowserAlive()) throw new Error("Owned browser exited before its debugging endpoint was ready", { cause: browserError });
    try {
      const [debuggingPort, browserPath] = (await readFile(join(profile, "DevToolsActivePort"), "utf8")).trim().split(/\r?\n/);
      if (!/^\d+$/.test(debuggingPort) || !browserPath?.startsWith("/devtools/browser/")) throw new Error("Invalid owned debugging endpoint");
      const origin = `http://127.0.0.1:${debuggingPort}`;
      const version = await (await fetch(`${origin}/json/version`, { signal: AbortSignal.timeout(1000) })).json();
      if (new URL(version.webSocketDebuggerUrl).pathname !== browserPath) throw new Error("Debugging endpoint identity mismatch");
      const listed = await (await fetch(`${origin}/json/list`, { signal: AbortSignal.timeout(1000) })).json();
      if (!isOwnedBrowserAlive()) throw new Error("Owned browser exited during endpoint discovery");
      targets = listed.filter((target) => {
        const url = new URL(target.webSocketDebuggerUrl);
        return url.hostname === "127.0.0.1" && url.port === debuggingPort;
      });
    } catch { await sleep(100); }
  }
  if (!targets?.some((target) => target.type === "page") || !isOwnedBrowserAlive()) throw new Error("Isolated browser unavailable");
  socket = new WebSocket(targets.find((target) => target.type === "page").webSocketDebuggerUrl);
  socket.addEventListener("close", () => {
    for (const request of pending.values()) request.reject(new Error("Owned CDP socket closed"));
    pending.clear();
  });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => { socket.close(); reject(new Error("Owned CDP socket connection timed out")); }, 5000);
    socket.addEventListener("open", () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("Owned CDP socket connection failed")); }, { once: true });
  });
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  });
  await command("Page.enable");
  await command("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });
  await command("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 2, mobile: false });
  const results = [];
  for (const mode of modes) {
    const base = `http://127.0.0.1:${port}/planet-${mode}/planet-render-benchmark.html`;
    if (length === "navigation") {
      await command("Page.navigate", { url: `${base}?navigation=1` });
      await sleep(100);
      await loaded();
      for (let iteration = 0; iteration < 105; iteration += 1) {
        await evaluate("window.planetBenchmark.home()");
        await evaluate("window.planetBenchmark.scan()");
        await sleep(150);
        await evaluate("window.planetBenchmark.navigate()");
        await sleep(100);
        const samples = await evaluate("window.planetBenchmark.snapshot().navigation_samples");
        if (samples.length !== iteration + 1 || !samples.at(-1).destination_committed)
          throw new Error(`Navigation destination did not commit for ${mode}, sample ${iteration}`);
      }
      const facts = await evaluate("window.planetBenchmark.snapshot()");
      const measured = facts.navigation_samples.slice(5);
      const result = { mode, facts, warmup_samples: facts.navigation_samples.slice(0, 5), samples: measured, navigation_p95_ms: percentile(measured.map((sample) => sample.duration_ms), .95) };
      results.push(result);
      await writeFile(join(out, `${mode}-navigation.json`), JSON.stringify(result, null, 2));
      console.log(JSON.stringify({ mode, samples: measured.length, navigation_p95_ms: result.navigation_p95_ms }));
      continue;
    }
    if (length === "matrix") {
      const captures = [];
      await mkdir(join(out, mode), { recursive: true });
      for (const locale of ["en", "zh-CN"]) for (const width of [390, 800, 1024, 1440]) for (const dpr of [1, 1.25, 1.5, 2]) {
        await command("Emulation.setDeviceMetricsOverride", { width, height: 900, deviceScaleFactor: dpr, mobile: false });
        await command("Page.navigate", { url: `${base}?still=1&locale=${locale}` });
        await sleep(100);
        await loaded();
        const facts = await evaluate("({...window.planetBenchmark.snapshot(), overflow:document.documentElement.scrollWidth>innerWidth,font:getComputedStyle(document.body).fontFamily})");
        const capture = await command("Page.captureScreenshot", { format: "png" });
        const name = `${locale}-${width}-${dpr}.png`;
        await writeFile(join(out, mode, name), Buffer.from(capture.data, "base64"));
        captures.push({ name, facts });
      }
      await command("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 2, mobile: false });
      for (const scenario of ["light", "forced", "texture-failure", "both-assets-failed", "context-unavailable", "fps15"]) {
        await command("Network.enable");
        await command("Network.setBlockedURLs", { urls: scenario === "texture-failure" ? ["*earth/natural-earth.png"] : scenario === "both-assets-failed" ? ["*earth/*.png"] : [] });
        await command("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }, { name: "forced-colors", value: scenario === "forced" ? "active" : "none" }] });
        let script;
        if (scenario === "context-unavailable") script = await command("Page.addScriptToEvaluateOnNewDocument", { source: "const get=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){return type==='webgl'?null:get.call(this,type,...args)}" });
        await command("Page.navigate", { url: `${base}?still=${scenario === "fps15" ? 0 : 1}&fps=15&theme=${scenario === "light" ? "light" : "dark"}` });
        await sleep(500);
        if (scenario === "fps15") { await evaluate("window.planetBenchmark.reset()"); await sleep(5000); }
        const facts = await evaluate("({...window.planetBenchmark.snapshot(),fallbackHidden:document.querySelector('.planet-fallback')?.hidden,canvasDisplay:getComputedStyle(document.querySelector('canvas')).display})");
        const capture = await command("Page.captureScreenshot", { format: "png" });
        await writeFile(join(out, mode, `${scenario}.png`), Buffer.from(capture.data, "base64"));
        captures.push({ name: scenario, facts });
        if (script) await command("Page.removeScriptToEvaluateOnNewDocument", { identifier: script.identifier });
      }
      const result = { mode, captures };
      results.push(result);
      await writeFile(join(out, `${mode}-matrix.json`), JSON.stringify(result, null, 2));
      console.log(JSON.stringify({ mode, captures: captures.length }));
      continue;
    }
    const cold = [];
    const coldLongTasks = [];
    for (let iteration = 0; iteration < (length === "interaction" ? 1 : length === "quick" ? 3 : 20); iteration += 1) {
      await command("Page.navigate", { url: `${base}?cold=${iteration}` });
      await sleep(100);
      await loaded();
      const sample = await evaluate("window.planetBenchmark.snapshot()");
      cold.push(sample.first_frame_ms);
      coldLongTasks.push(sample.long_tasks);
    }
    await sleep(3000);
    const windows = [];
    for (let iteration = 0; iteration < (length === "interaction" ? 0 : length === "quick" ? 1 : 5); iteration += 1) {
      await evaluate("window.planetBenchmark.reset()");
      await sleep(length === "quick" ? 5000 : 30000);
      const sample = await evaluate("window.planetBenchmark.snapshot()");
      if (sample.painted_raf_durations_ms.length < (length === "quick" ? 50 : 500))
        throw new Error(`Invalid animated window for ${mode}: insufficient painted samples`);
      const gaps = sample.draw_timestamps_ms.slice(1).map((value, index) => value - sample.draw_timestamps_ms[index]);
      windows.push({ ...sample, painted_raf_p95_ms: percentile(sample.painted_raf_durations_ms, .95), painted_raf_total_ms: sample.painted_raf_durations_ms.reduce((a, b) => a + b, 0), gaps_over_twice_interval: gaps.filter((value) => value > 1000 / 30 * 2).length, gaps_count: gaps.length, frame_gap_p95_ms: percentile(gaps, .95) });
    }
    // Real Stage/ScanPage progress updates and local cancel state through the
    // production coordinator. This does not claim native worker cancellation.
    for (let iteration = 0; iteration < (length === "interaction" ? 100 : 20); iteration += 1) {
      await evaluate("window.planetBenchmark.scan()");
      await sleep(150);
      await evaluate("window.planetBenchmark.cancel()");
      await sleep(50);
    }
    const interactions = await evaluate("window.planetBenchmark.snapshot().interaction_ms");
    const cycleSamples = [];
    for (let iteration = 0; iteration < 20; iteration += 1) {
      await evaluate("window.planetBenchmark.mount(false)");
      await sleep(50);
      const before = await evaluate("window.planetBenchmark.snapshot()");
      await sleep(50);
      const after = await evaluate("window.planetBenchmark.snapshot()");
      cycleSamples.push({ handles: after.owned_frames, unexpected_draws: after.draw_timestamps_ms.length - before.draw_timestamps_ms.length, memory: after.memory });
      await evaluate("window.planetBenchmark.mount(true)");
      await sleep(100);
    }
    await command("Page.navigate", { url: `${base}?still=1` });
    await sleep(100);
    await loaded();
    await sleep(50);
    const screenshot = await command("Page.captureScreenshot", { format: "png" });
    await writeFile(join(out, `${mode}.png`), Buffer.from(screenshot.data, "base64"));
    const html = await readFile(resolve(root, `planet-${mode}/planet-render-benchmark.html`));
    const result = { mode, cold_ms: cold, cold_p95_ms: percentile(cold, .95), cold_long_tasks: coldLongTasks, windows, interaction_ms: interactions, interaction_p95_ms: percentile(interactions, .95), cycle_samples: cycleSamples, html_sha256: createHash("sha256").update(html).digest("hex") };
    results.push(result);
    await writeFile(join(out, `${mode}.json`), JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ mode, cold_p95_ms: result.cold_p95_ms, warm_p95_ms: windows.map((w) => w.painted_raf_p95_ms), painted_ms: windows.map((w) => w.painted_raf_total_ms), interaction_p95_ms: result.interaction_p95_ms }));
  }
  await writeFile(join(out, "record.json"), JSON.stringify({ length, timestamp: new Date().toISOString(), host: { platform: platform(), release: release(), cpu: cpus()[0]?.model, logical_processors: cpus().length }, results }, null, 2));
} finally {
  if (socket?.readyState === WebSocket.OPEN && isOwnedBrowserAlive()) {
    await Promise.race([command("Browser.close").catch(() => undefined), sleep(1000)]);
    socket.close();
  }
  server.close();
  if (isOwnedBrowserAlive()) await Promise.race([browserExited, sleep(5000)]);
  const forced = Boolean(isOwnedBrowserAlive());
  if (forced) { browser.kill(); await Promise.race([browserExited, sleep(5000)]); }
  const teardown = { pid: browser.pid, forced, exit_code: browser.exitCode, signal: browser.signalCode, error: browserError?.message, lifecycle_pass: !forced && !browserError && browser.exitCode === 0 };
  await writeFile(join(out, "teardown.json"), JSON.stringify(teardown, null, 2));
  if (!teardown.lifecycle_pass) process.exitCode = 1;
}
