// Instrumented lifecycle evidence for an explicitly owned packaged Tauri launch.
// This is not an uninstrumented CPU/GPU benchmark and never starts domain work.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [launchArg, outputArg, completedCyclesArg] = process.argv.slice(2);
assert(launchArg && outputArg, "usage: node tools/native-earth-smoke.mjs <launch.json> <fresh-output> [same-launch-completed-cycle-output|--matrix-only]");
const matrixOnly = completedCyclesArg === "--matrix-only";
const launch = JSON.parse(readFileSync(resolve(launchArg), "utf8"));
assert.equal(typeof launch.created, "string");
assert(Number.isInteger(launch.debug_port) && launch.debug_port >= 1024 && launch.debug_port <= 65535);
const output = resolve(outputArg), repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
mkdirSync(output, { recursive: false });
const save = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: "wx" });
const pause = (ms) => new Promise((accept) => setTimeout(accept, ms));
function native(action = "inspect", handle = null) {
  return JSON.parse(execFileSync("python", ["-c", `
import ctypes as ct,ctypes.wintypes as wt,importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
s=module.WindowsSampler();pid=int(sys.argv[2]);created=int(sys.argv[3]);action=sys.argv[4]
assert s.inspect({'pid':pid}).get('created')==created,'owned process identity changed'
if action in ('minimized','restored'):s.main_window_state(pid,created,action,int(sys.argv[5]))
windows=s.windows(pid,created)
user=ct.WinDLL('user32');user.GetDpiForWindow.argtypes=[wt.HWND];user.GetDpiForWindow.restype=wt.UINT
for window in windows:window['dpi']=user.GetDpiForWindow(window['handle'])
rows=s.tree(pid,{pid:created})
for row in rows:
 if 'created' in row:row['created']=str(row['created'])
print(json.dumps({'windows':windows,'processes':rows}))
`, join(repo, "tools/measure-readonly.py"), String(launch.pid), launch.created, action, String(handle)], { encoding: "utf8", windowsHide: true }));
}
const listener = execFileSync("powershell.exe", ["-NoProfile", "-Command",
  `Get-NetTCPConnection -State Listen -LocalPort ${launch.debug_port} -ErrorAction Stop | Select-Object -ExpandProperty OwningProcess -Unique`], { encoding: "utf8", windowsHide: true }).trim();
assert.match(listener, /^\d+$/);
const owner = native();
assert(owner.processes.some((row) => row.pid === Number(listener)), "debug listener must belong to the exact launch");
const windows = owner.windows.filter((window) => window.class === "Tauri Window" && window.title === "DevSweep");
assert.equal(windows.length, 1, "one owned main window required");
const mainHandle = windows[0].handle;
const targets = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(10000) }).then((r) => r.json());
const page = targets.find((target) => target.type === "page" && /^https?:\/\/tauri\.localhost\//.test(target.url) && !target.url.includes("hud.html"));
assert(page, "packaged main page required");
const debuggerUrl = new URL(page.webSocketDebuggerUrl);
assert.equal(debuggerUrl.hostname, "127.0.0.1");
assert.equal(debuggerUrl.port, String(launch.debug_port));
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept, reject) => {
  const timer = setTimeout(() => reject(new Error("CDP connect timeout")), 10000);
  socket.onopen = () => { clearTimeout(timer); accept(); };
  socket.onerror = (error) => { clearTimeout(timer); reject(error); };
});
let next = 0;
const pending = new Map();
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data), waiter = pending.get(message.id);
  if (!waiter) return;
  pending.delete(message.id); clearTimeout(waiter.timer);
  if (message.error) waiter.reject(new Error(message.error.message)); else waiter.accept(message.result);
};
function send(method, params = {}) {
  return new Promise((accept, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 30000);
    pending.set(id, { accept, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(expression) {
  const until = Date.now() + 30000;
  do { if (await evaluate(expression)) return; await pause(100); } while (Date.now() < until);
  throw new Error(`Native condition timed out: ${expression}`);
}
async function click(expression) {
  const point = await evaluate(`(() => {const e=${expression};if(!e||e.disabled)throw Error('control unavailable');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, ...point, button: "left", clickCount: 1 });
}
const tab = (id) => click(`document.getElementById(${JSON.stringify(id)})`);
const presentation = "window.__TAURI_INTERNALS__.invoke('main_presentation_visibility')";
async function snapshot() {
  return evaluate(`({at_ms:Date.now(),visibility:document.visibilityState,route:location.hash,dpr:devicePixelRatio,width:innerWidth,height:innerHeight,render:document.querySelector('[data-planet="clean"]')?.dataset.renderState,canvas:[...document.querySelectorAll('.planet-canvas')].map(c=>({width:c.width,height:c.height})),metrics:window.__earthSmoke,resources:performance.getEntriesByType('resource').map(r=>r.name),heap:performance.memory?{used:performance.memory.usedJSHeapSize,total:performance.memory.totalJSHeapSize}:null})`);
}
async function shot(name) {
  // Capture a neutral pointer pose; the existing active-tab hover contrast
  // defect is retained separately in the native acceptance diagnostics.
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 45 });
  await pause(50);
  const result = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(output, name), Buffer.from(result.data, "base64"), { flag: "wx" });
}
function memoryAssessment(rows) {
  const median = (values) => {
    const sorted = [...values].sort((a, b) => a - b), middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
  };
  const observations = rows.map((row) => {
    const processes = row.inactiveNative.processes.filter((process) => process.present);
    const totals = {};
    for (const key of ["private_bytes", "working_bytes", "handles", "threads"])
      totals[key] = processes.length && processes.every((process) => Number.isFinite(process[key]))
        ? processes.reduce((sum, process) => sum + process[key], 0) : null;
    return { cycle: row.cycle + 1, ...totals, heap_used_bytes: row.inactive.heap?.used ?? null };
  });
  const windows = [];
  for (let start = 0; start < observations.length; start += 5) {
    const block = observations.slice(start, start + 5), metrics = {};
    for (const key of ["private_bytes", "working_bytes", "handles", "threads", "heap_used_bytes"]) {
      const values = block.map((row) => row[key]);
      metrics[key] = values.every(Number.isFinite) ? { min: Math.min(...values), max: Math.max(...values), median: median(values) } : null;
    }
    windows.push({ cycles: [start + 1, start + block.length], metrics });
  }
  return { method: "Same unmounted route in each cycle; actual owned process-tree totals and browser-reported JS heap; five-cycle windows, no forced GC. GPU-object ownership is asserted separately. These descriptive windows require a plateau assessment and are not an invented numeric pass threshold.", observations, windows };
}
const cycles = completedCyclesArg && !matrixOnly ? JSON.parse(readFileSync(join(resolve(completedCyclesArg), "cycles.json"), "utf8")) : [];
if (completedCyclesArg && !matrixOnly) {
  const previous = JSON.parse(readFileSync(join(resolve(completedCyclesArg), "initial.json"), "utf8"));
  assert.equal(previous.launch.pid, launch.pid, "cycle evidence must belong to this same running app");
  assert.equal(previous.launch.created, launch.created);
  assert.equal(previous.launch.executable, launch.executable);
  assert.equal(cycles.length, 20, "only an already completed twenty-cycle record can be reused");
  save("reused-cycles.json", { source: resolve(completedCyclesArg), cycles_sha256: createHash("sha256").update(readFileSync(join(resolve(completedCyclesArg), "cycles.json"))).digest("hex") });
}
let instrumentationIdentifier;
try {
  await send("Page.enable");
  const systemMedia = await evaluate("({reduced:matchMedia('(prefers-reduced-motion: reduce)').matches,forced_colors:matchMedia('(forced-colors: active)').matches,motion:document.documentElement.dataset.motion})");
  save("system-media.json", systemMedia);
  // Exercise animation without changing Windows or the user's preference.
  // Actual system media is retained above; the later reduced-motion case
  // explicitly verifies the still pose again.
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });
  const instrumentation = await send("Page.addScriptToEvaluateOnNewDocument", { source: `(() => {
    const metrics=window.__earthSmoke={draws:0,rafCallbacks:0,pendingRaf:0,objects:{Texture:0,Buffer:0,Program:0,Shader:0},visibility:[]};
    const raf=requestAnimationFrame,cancel=cancelAnimationFrame,handles=new Set();
    window.requestAnimationFrame=callback=>{let id;id=raf.call(window,time=>{handles.delete(id);metrics.pendingRaf=handles.size;metrics.rafCallbacks++;callback(time);});handles.add(id);metrics.pendingRaf=handles.size;return id;};
    window.cancelAnimationFrame=id=>{handles.delete(id);metrics.pendingRaf=handles.size;return cancel.call(window,id);};
    const proto=WebGLRenderingContext.prototype,draw=proto.drawArrays;
    proto.drawArrays=function(...args){metrics.draws++;return draw.apply(this,args);};
    for(const kind of Object.keys(metrics.objects)){
      const create=proto['create'+kind],remove=proto['delete'+kind],live=new WeakSet();
      proto['create'+kind]=function(...args){const value=create.apply(this,args);if(value){live.add(value);metrics.objects[kind]++;}return value;};
      proto['delete'+kind]=function(value){if(value&&live.has(value)){live.delete(value);metrics.objects[kind]--;}return remove.call(this,value);};
    }
    document.addEventListener('visibilitychange',()=>metrics.visibility.push({at_ms:Date.now(),state:document.visibilityState}));
  })()` });
  instrumentationIdentifier = instrumentation.identifier;
  await send("Network.enable");
  await send("Network.emulateNetworkConditions", { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
  await send("Page.reload", { ignoreCache: true });
  await waitFor("!!document.querySelector('#primary-tab-clean')");
  await tab("primary-tab-clean");
  await waitFor("document.querySelector('[data-planet=clean]')?.dataset.renderState==='ready'");
  await pause(1000);
  const initial = await snapshot();
  assert.equal(await evaluate("navigator.onLine"), false);
  assert(initial.metrics.draws > 1, "non-vacuous animated Earth required");
  assert(initial.resources.some((url) => url.endsWith("/earth/natural-earth.png")));
  assert(initial.resources.every((url) => ["tauri.localhost", "ipc.localhost"].includes(new URL(url).hostname)));
  save("initial.json", { launch, native: owner, page: initial, browser: await send("Browser.getVersion"), method: "Instrumented draw/RAF/resource lifecycle only; API wrappers preserve arguments and return values. CSS viewport/DPR matrix is emulated inside actual WebView2; actual Windows DPI is recorded separately. No process performance claim." });
  await shot("actual-windows-scale.png");
  for (let cycle = matrixOnly ? 20 : cycles.length; cycle < 20; cycle++) {
    await tab("primary-tab-settings");
    await waitFor("location.hash==='#/settings' && !document.querySelector('.planet-canvas')");
    await pause(100);
    const unmounted = await snapshot();
    await pause(500);
    const inactive = await snapshot();
    assert.equal(inactive.metrics.draws, unmounted.metrics.draws, "no draw after unmount");
    assert.equal(inactive.metrics.pendingRaf, 0, "no pending application RAF on settings");
    assert(Object.values(inactive.metrics.objects).every((value) => value === 0), "GPU objects released on unmount");
    const inactiveNative = native();
    await tab("primary-tab-clean");
    await waitFor("document.querySelector('[data-planet=clean]')?.dataset.renderState==='ready'");
    const minimizedNative = native("minimized", mainHandle);
    assert.equal(minimizedNative.windows.find((window) => window.handle === mainHandle)?.minimized, true);
    // WebView2 can remain document-visible after the real HWND is minimized.
    // Query the shipped read-only command; do not replace any bridge or state.
    await waitFor(`${presentation}.then(value=>value===false)`);
    await waitFor("window.__earthSmoke.pendingRaf===0");
    const hidden = await snapshot();
    await pause(500);
    const suspended = await snapshot();
    assert.equal(suspended.metrics.draws, hidden.metrics.draws, "no hidden draw");
    assert.equal(suspended.metrics.rafCallbacks, hidden.metrics.rafCallbacks, "no hidden RAF callback");
    assert.equal(suspended.metrics.pendingRaf, 0, "hidden Planet cancels its frame owner");
    const restoredNative = native("restored", mainHandle);
    assert.equal(restoredNative.windows.find((window) => window.handle === mainHandle)?.minimized, false);
    await waitFor(`${presentation}.then(value=>value===true)`);
    await pause(150);
    const restored = await snapshot();
    assert(restored.metrics.draws > suspended.metrics.draws, "Earth resumes drawing");
    assert.deepEqual(restored.metrics.objects, { Texture: 1, Buffer: 1, Program: 1, Shader: 2 });
    cycles.push({ cycle, unmounted, inactive, inactiveNative, hidden, suspended, restored, minimizedNative, restoredNative, native: native(), native_presentation: { minimized: false, restored: true } });
  }
  save("cycles.json", cycles);
  save("memory-windows.json", memoryAssessment(cycles));
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await pause(300);
  const still = await snapshot(); await pause(600); const stillAfter = await snapshot();
  assert.equal(stillAfter.metrics.draws, still.metrics.draws);
  assert.equal(stillAfter.metrics.pendingRaf, 0);
  // Keep the desktop 232px CSS globe: the responsive 800px layout uses 168px.
  await send("Emulation.setDeviceMetricsOverride", { width: 1080, height: 900, deviceScaleFactor: 2, mobile: false });
  await waitFor("document.querySelector('.planet-canvas')?.width===464");
  const resized = await snapshot(); await pause(600); const resizedAfter = await snapshot();
  assert(resized.metrics.draws > stillAfter.metrics.draws, "DPR change redraws the still pose");
  assert.equal(resizedAfter.metrics.draws, resized.metrics.draws);
  assert.equal(resizedAfter.metrics.pendingRaf, 0);
  await shot("reduced-motion-dpr2.png");
  save("reduced-motion.json", { still, stillAfter, resized, resizedAfter });
  const matrix = [];
  for (const locale of ["en", "zh-CN"]) {
    if (locale === "zh-CN") {
      await tab("primary-tab-settings");
      await waitFor("!!document.querySelector('#settings-language')");
      await tab("settings-language");
      await waitFor("document.querySelectorAll('.settings-choice-item').length===2");
      await click("[...document.querySelectorAll('.settings-choice-item')].find(e=>e.textContent.includes('Simplified Chinese'))");
      await waitFor("document.querySelector('.app-shell')?.dataset.locale==='zh-CN'");
      await tab("primary-tab-clean");
      await waitFor("document.querySelector('[data-planet=clean]')?.dataset.renderState==='ready'");
    }
    for (const width of [390, 800, 1024, 1440]) {
      for (const dpr of [1, 1.25, 1.5, 2]) {
        await send("Emulation.setDeviceMetricsOverride", { width, height: 1100, deviceScaleFactor: dpr, mobile: false });
        // Fresh mounts establish each visual case independently. DPR-only
        // lifecycle behavior has its own same-component event-trace check.
        await send("Page.reload", { ignoreCache: true });
        await waitFor("document.querySelector('[data-planet=clean]')?.dataset.renderState==='ready'");
        await waitFor("(() => {const c=document.querySelector('.planet-canvas');return c && c.width===Math.max(8,Math.min(512,Math.round(c.clientWidth*Math.min(2,devicePixelRatio))));})()");
        await pause(150);
        const facts = await evaluate(`({locale:document.querySelector('.app-shell').dataset.locale,width:innerWidth,dpr:devicePixelRatio,overflow:document.documentElement.scrollWidth>innerWidth,font:getComputedStyle(document.body).fontFamily,render:document.querySelector('[data-planet=clean]').dataset.renderState,canvas:document.querySelector('.planet-canvas').width,css_canvas:document.querySelector('.planet-canvas').clientWidth,expected_canvas:Math.max(8,Math.min(512,Math.round(document.querySelector('.planet-canvas').clientWidth*Math.min(2,devicePixelRatio)))),roots:[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent),pendingRaf:window.__earthSmoke.pendingRaf})`);
        assert.equal(facts.locale, locale); assert.equal(facts.overflow, false);
        assert.equal(facts.render, "ready"); assert(facts.canvas <= 512); assert.equal(facts.pendingRaf, 0);
        assert.equal(facts.canvas, facts.expected_canvas);
        const screenshot = `${locale}-w${width}-dpr${dpr}.png`;
        await shot(screenshot); matrix.push({ ...facts, screenshot });
      }
    }
  }
  save("visual-matrix.json", matrix);
  await send("Emulation.setEmulatedMedia", { features: [{ name: "forced-colors", value: "active" }] });
  await send("Page.reload", { ignoreCache: true });
  await waitFor("!!document.querySelector('[data-planet=clean]')");
  await pause(600);
  const forced = await snapshot();
  assert.equal(forced.metrics.draws, 0);
  assert.equal(forced.metrics.pendingRaf, 0);
  assert(forced.resources.every((url) => !url.includes("/earth/")), "forced colors must not load texture or fallback");
  save("forced-colors.json", forced); await shot("forced-colors.png");
  save("accessibility.json", await send("Accessibility.getFullAXTree"));
  save("result.json", { pass: true, scope: matrixOnly ? "visual matrix and still/forced-colors only; native cycles are separate evidence" : "native lifecycle and visual matrix", cycles: cycles.length, visual_cases: matrix.length, native_sha256: createHash("sha256").update(readFileSync(launch.executable)).digest("hex"), actual_window_dpi: windows[0].dpi, actual_webview_dpr: initial.dpr, hidden_draws: matrixOnly ? null : 0, unmounted_draws: matrixOnly ? null : 0, bounded_gpu_objects: matrixOnly ? null : true, resource_gate: "raw process/heap rows retained; uninstrumented performance and memory plateau assessment separate", cleanup_executed: false });
  console.log(JSON.stringify({ pass: true, cycles: cycles.length, output }));
} catch (error) {
  save("failure.json", { error: String(error), stack: error.stack, cycles });
  try { save("failure-page.json", await snapshot()); await shot("failure.png"); } catch { /* Preserve original failure. */ }
  throw error;
} finally {
  try { native("restored", mainHandle); } catch { /* Native exit is recorded by the launch harness. */ }
  try { await send("Emulation.clearDeviceMetricsOverride"); await send("Emulation.setEmulatedMedia", { features: [] }); } catch { /* Preserve original failure. */ }
  if (instrumentationIdentifier) try { await send("Page.removeScriptToEvaluateOnNewDocument", { identifier: instrumentationIdentifier }); } catch { /* Preserve original failure. */ }
  socket.close();
}
