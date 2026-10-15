// Production-only Clean overview measurements in a fresh, task-owned browser.
// node tools/clean-home-benchmark.mjs NEW_OUTPUT_DIRECTORY BUILD_LABEL [PRESERVED_BUILD_DIRECTORY] [measure|visual]
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
const root = resolve(preservedBuild ?? `desktop/dist/clean-home-${label}`);
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
  await command("Page.navigate", {url:`http://127.0.0.1:${port}/clean-home-benchmark.html`});
  let ready = false;
  for(let attempt=0;attempt<100;attempt++) { if(await evaluate("Boolean(window.cleanHomeBenchmark)")){ready=true;break;} await sleep(50); }
  if(!ready) throw Error("Production Clean home harness not loaded");
  await evaluate("document.documentElement.dataset.theme = 'dark'");
  const result = await evaluate("window.cleanHomeBenchmark.run()");
  if(result.retained_targets!==10000 || result.retained_owners!==10000 || result.rendered_owners!==10) throw Error("Full report retention or DOM owner bound violated");
  await writeFile(join(out,"10000-home.json"),JSON.stringify(result,null,2));
  const png=await command("Page.captureScreenshot",{format:"png"});
  await writeFile(join(out,"measured-home.png"),Buffer.from(png.data,"base64"));
  // Supplemental visual state only, after all measured samples have completed.
  await command("Emulation.setDeviceMetricsOverride", { width:1350, height:900, deviceScaleFactor:1, mobile:false });
  await evaluate("document.querySelector('.clean-owner-list .target-pagination button').click(); document.querySelector('#clean-threshold').value='5'; document.querySelector('.clean-threshold').requestSubmit()");
  let warningReady=false;
  for(let attempt=0;attempt<100;attempt++) { if(await evaluate("document.querySelectorAll('.clean-assessment.oversized').length===10")){warningReady=true;break;} await sleep(50); }
  if(!warningReady) throw Error("Supplemental oversized visual state unavailable");
  await evaluate("document.querySelector('.clean-threshold-settings').open=false; window.scrollTo(0,0)");
  const warningPng=await command("Page.captureScreenshot",{format:"png"});
  await writeFile(join(out,"oversized-home.png"),Buffer.from(warningPng.data,"base64"));
  const html=await readFile(join(root,"clean-home-benchmark.html"));
  await writeFile(join(out,"record.json"),JSON.stringify({timestamp:new Date().toISOString(),label,result,
    html_sha256:createHash("sha256").update(html).digest("hex"),node:process.version,theme:"dark",
    host:{platform:platform(),release:release(),cpu:cpus()[0]?.model,logical_processors:cpus().length}},null,2));
  console.log(JSON.stringify({owners:result.retained_owners,mounted:result.rendered_owners,p95_ms:result.p95_ms,budgets:result.budgets}));
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
