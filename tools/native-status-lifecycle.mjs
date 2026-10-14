// Read-only native Status lifecycle evidence for an owned measure-readonly launch.
// UI starts/stops real monitoring. The observer never replaces command results.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [launchArg, outputArg, hudOption] = process.argv.slice(2);
assert(launchArg && outputArg && (hudOption === undefined || ["--hud", "--hud-only"].includes(hudOption)), "usage: node tools/native-status-lifecycle.mjs <launch.json> <fresh-output> [--hud|--hud-only]");
const requestedStatusCycles = hudOption === "--hud-only" ? 0 : 5;
const launch = JSON.parse(readFileSync(resolve(launchArg), "utf8"));
assert.equal(typeof launch.created, "string");
assert(Number.isInteger(launch.debug_port) && launch.debug_port >= 1024 && launch.debug_port <= 65535);
const output = resolve(outputArg), repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
mkdirSync(output, { recursive: false });
const save = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: "wx" });
const pause = (ms) => new Promise((accept) => setTimeout(accept, ms));
function native(action, handle = null) {
  return JSON.parse(execFileSync("python", ["-c", `
import importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
s=module.WindowsSampler(); pid=int(sys.argv[2]); created=int(sys.argv[3]); action=sys.argv[4]
assert s.inspect({'pid':pid}).get('created')==created,'owned process identity changed'
if action in ('minimized','restored'):
 s.main_window_state(pid,created,action,int(sys.argv[5]))
if action=='toggle_hud': s.toggle_owned_hud(pid,created)
rows=s.tree(pid,{pid:created})
print(json.dumps({'windows':s.windows(pid,created),'owned_pids':[r['pid'] for r in rows], 'threads':s.named_threads(pid,created)}))
`, join(repo, "tools/measure-readonly.py"), String(launch.pid), launch.created, action, String(handle)], { encoding: "utf8", windowsHide: true }));
}
const ownerText = execFileSync("powershell.exe", ["-NoProfile", "-Command",
  `Get-NetTCPConnection -State Listen -LocalPort ${launch.debug_port} -ErrorAction Stop | Select-Object -ExpandProperty OwningProcess -Unique`], { encoding: "utf8", windowsHide: true }).trim();
assert.match(ownerText, /^\d+$/);
const owner = native("inspect");
assert(owner.owned_pids.includes(Number(ownerText)), "CDP listener must be owned by this exact launch");
const mainWindows = owner.windows.filter((w) => w.class === "Tauri Window" && w.title === "DevSweep");
assert.equal(mainWindows.length, 1, "start with fresh main-only process");
const mainHandle = mainWindows[0].handle;
const targets = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(10000) }).then((r) => r.json());
const page = targets.find((t) => t.type === "page" && /^https?:\/\/tauri\.localhost\//.test(t.url) && !t.url.includes("hud.html"));
assert(page, "packaged main page required");
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept, reject) => {
  const timeout = setTimeout(() => reject(new Error("CDP connect timeout")), 10000);
  socket.onopen = () => { clearTimeout(timeout); accept(); };
  socket.onerror = (error) => { clearTimeout(timeout); reject(error); };
});
assert(native("inspect").owned_pids.includes(Number(ownerText)));
let next = 0;
const pending = new Map();
const calls = [], requests = new Map();
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.method === "Network.requestWillBeSent") {
    const { requestId, request } = message.params;
    const url = new URL(request.url);
    const command = decodeURIComponent(url.pathname.slice(1));
    if (url.hostname === "ipc.localhost" && request.method === "POST" && ["status_snapshot", "status_live_start", "status_cancel"].includes(command)) {
      const args = JSON.parse(request.postData ?? "{}");
      const call = { command, id: args.operationId, request_ms: Date.now(), request_id: requestId, events: [] };
      calls.push(call); requests.set(requestId, call);
    }
  }
  if (message.method === "Network.loadingFinished" && requests.has(message.params.requestId)) {
    const call = requests.get(message.params.requestId);
    call.response_ms = Date.now();
    send("Network.getResponseBody", { requestId: message.params.requestId }).then((result) => {
      const body = result.base64Encoded ? Buffer.from(result.body, "base64").toString("utf8") : result.body;
      const value = body ? JSON.parse(body) : null;
      call.result_type = value?.type ?? null;
      call.settled_ms = call.response_ms;
      if (value?.code) call.error = JSON.stringify(value);
    }).catch((error) => { call.error = String(error); call.settled_ms = Date.now(); });
  }
  const waiter = pending.get(message.id);
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
  const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(expression) {
  const deadline = Date.now() + 30000;
  do { if (await evaluate(expression)) return; await pause(100); } while (Date.now() < deadline);
  throw new Error(`Condition timeout: ${expression}`);
}
async function waitUntil(condition) {
  const deadline = Date.now() + 30000;
  do { if (condition()) return; await pause(100); } while (Date.now() < deadline);
  throw new Error("Native IPC observation timeout");
}
async function click(expression) {
  const point = await evaluate(`(() => {const e=${expression};if(!e||e.disabled)throw Error('missing/disabled control');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, ...point, button: "left", clickCount: 1 });
}
const button = (label) => `[...document.querySelectorAll('button')].find(e=>e.textContent.trim()===${JSON.stringify(label)})`;
const cycles = [];
async function hudLifecycle() {
  const records = [];
  const samplers = (state) => state.threads.filter((thread) => thread.description === "devsweep-hud-sampler");
  const waitNative = async (condition) => {
    const deadline = Date.now() + 15000;
    do {
      const state = native("inspect");
      if (condition(state)) return state;
      await pause(100);
    } while (Date.now() < deadline);
    throw new Error("HUD native lifecycle timeout");
  };
  native("toggle_hud");
  await waitNative((state) => samplers(state).length === 1);
  let hudPage;
  for (let attempt = 0; attempt < 100 && !hudPage; attempt++) {
    const pages = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(5000) }).then((r) => r.json());
    hudPage = pages.find((target) => target.type === "page" && /^https?:\/\/tauri\.localhost\/hud\.html/.test(target.url));
    if (!hudPage) await pause(100);
  }
  assert(hudPage, "owned packaged HUD page required");
  const hudSocket = new WebSocket(hudPage.webSocketDebuggerUrl);
  await new Promise((accept, reject) => {
    const timer = setTimeout(() => reject(new Error("HUD CDP connect timeout")), 10000);
    hudSocket.onopen = () => { clearTimeout(timer); accept(); };
    hudSocket.onerror = (error) => { clearTimeout(timer); reject(error); };
  });
  let id = 0;
  // HUD calls are deliberately sequential; no response replacement or native invocation.
  const inspectHud = (expression) => new Promise((accept, reject) => {
    const requestId = ++id;
    const timer = setTimeout(() => reject(new Error("HUD observation timeout")), 10000);
    hudSocket.onmessage = ({ data }) => {
      const response = JSON.parse(data);
      if (response.id !== requestId) return;
      clearTimeout(timer);
      if (response.error || response.result.exceptionDetails) reject(new Error(JSON.stringify(response)));
      else accept(response.result.result.value);
    };
    hudSocket.send(JSON.stringify({ id: requestId, method: "Runtime.evaluate", params: { expression, returnByValue: true } }));
  });
  try {
    await inspectHud(`(() => {
      const callbacks=window.__TAURI_INTERNALS__.callbacks,original=callbacks.set;
      window.__hudEvents=[];
      const wrap=callback=>function(value){
        if(value?.event==='hud-status')window.__hudEvents.push({at_ms:Date.now(),type:value.payload?.type});
        return callback.apply(this,arguments);
      };
      callbacks.set=function(id,callback){return original.call(this,id,wrap(callback));};
      for(const [id,callback]of callbacks)original.call(callbacks,id,wrap(callback));
    })()`);
    for (let cycle = 0; cycle < 20; cycle++) {
      const started = Date.now();
      if (cycle > 0) native("toggle_hud");
      const visible = await waitNative((state) => samplers(state).length === 1);
      const deadline = Date.now() + 10000;
      let events;
      do {
        events = await inspectHud("window.__hudEvents");
        if (events.some((event) => event.type === "snapshot" && event.at_ms >= started)) break;
        assert(Date.now() < deadline, "visible HUD must publish an actual snapshot");
        await pause(100);
      } while (true);
      const hideRequest = Date.now();
      native("toggle_hud");
      const hidden = await waitNative((state) => samplers(state).length === 0);
      const joined = Date.now();
      const afterJoin = await inspectHud("window.__hudEvents.length");
      await pause(550);
      assert.equal(await inspectHud("window.__hudEvents.length"), afterJoin, "hidden joined HUD must publish no later event");
      const settled = native("inspect");
      assert.equal(samplers(settled).length, 0);
      records.push({ cycle, started_ms: started, hide_request_ms: hideRequest, joined_observed_ms: joined,
        snapshots: events.filter((event) => event.type === "snapshot" && event.at_ms >= started).length,
        visible, hidden, settled, no_later_publications: true });
    }
    save("hud-result.json", { pass: true, cycles: records,
      method: "real owned tray callback; callback Map observes unchanged HUD events; GetThreadDescription proves exactly one visible sampler and zero after hide in same PID" });
    return records.length;
  } catch (error) {
    save("hud-failure.json", { error: String(error), records });
    throw error;
  } finally {
    hudSocket.close();
  }
}
try {
  await waitFor("!!window.__TAURI_INTERNALS__ && !!document.querySelector('#primary-tab-status')");
  await send("Network.enable");
  await evaluate(`(() => {
    const callbacks=window.__TAURI_INTERNALS__.callbacks, original=callbacks.set;
    window.__statusEvents=[];
    const wrap=callback=>function(value){
      const event=value?.message;
      if(event?.operation_id && event.event)window.__statusEvents.push({id:event.operation_id,at_ms:Date.now(),event:event.event,sequence:event.sequence});
      return callback.apply(this,arguments);
    };
    callbacks.set=function(id,callback){return original.call(this,id,wrap(callback));};
    for(const [id,callback]of callbacks)original.call(callbacks,id,wrap(callback));
  })()`);
  await click("document.querySelector('#primary-tab-status')");
  await waitFor(`!!${button("Start live")} && !${button("Start live")}.disabled`);
  const preLiveStart = Date.now();
  await pause(5500);
  const preLiveEnd = Date.now();
  for (let cycle = 0; cycle < requestedStatusCycles; cycle++) {
    await click(button("Start live"));
    await waitUntil(() => calls.filter((c) => c.command === "status_live_start").length === cycle + 1);
    await waitFor(`!!${button("Stop live")}`);
    const live = calls.filter((c) => c.command === "status_live_start").at(-1);
    assert.equal(typeof live.id, "string", "observed request must include actual operation id");
    await pause(4400);
    if (cycle === 0) {
      native("minimized", mainHandle);
      const hidden = await evaluate("({visibility:document.visibilityState,at_ms:Date.now()})");
      const before = await evaluate("window.__statusEvents.length");
      await pause(4400);
      const after = await evaluate("window.__statusEvents.length");
      const minimized = native("inspect").windows.find((w) => w.handle === mainHandle);
      assert.equal(minimized.minimized, true);
      assert(after > before, "explicitly requested live sampling must continue while minimized");
      save("minimized.json", { hidden, native_window: minimized, events_before: before, events_after: after });
      native("restored", mainHandle);
      await waitFor("document.visibilityState==='visible'");
    }
    await click(button("Stop live"));
    await waitUntil(() => !!live.settled_ms);
    await waitFor(`!!${button("Start live")} && !${button("Start live")}.disabled`);
    assert.equal(live.error, undefined);
    assert.equal(live.result_type, "canceled");
    live.events = await evaluate(`window.__statusEvents.filter(e=>e.id===${JSON.stringify(live.id)})`);
    const eventCount = live.events.length;
    const postStart = Date.now();
    // The external 200 ms sampler supplies the first 25 scheduled post-join samples.
    await pause(5500);
    live.events = await evaluate(`window.__statusEvents.filter(e=>e.id===${JSON.stringify(live.id)})`);
    const cancel = calls.findLast((c) => c.command === "status_cancel" && c.id === live.id);
    assert.equal(live.events.length, eventCount, "no publication after joined command settlement");
    assert(cancel?.settled_ms && !cancel.error, "cancel acknowledgment required separately from join");
    assert(live.events.length > 0, "non-vacuous live evidence");
    assert(live.events.some((event) => event.event === "status_snapshot"), "at least one actual snapshot required");
    assert.equal(live.events.at(-1).event, "status_terminal");
    assert(live.events.every((event, index) => index === 0 || event.sequence > live.events[index - 1].sequence));
    cycles.push({ cycle, live, cancel, post_start_ms: postStart, post_end_ms: Date.now(), owned: native("inspect") });
  }
  let hudCycles = 0;
  if (hudOption) {
    await click(button("Start live"));
    await waitUntil(() => calls.filter((call) => call.command === "status_live_start").length === requestedStatusCycles + 1);
    const combined = calls.filter((call) => call.command === "status_live_start").at(-1);
    hudCycles = await hudLifecycle();
    await click(button("Stop live"));
    await waitUntil(() => !!combined.settled_ms);
    assert.equal(combined.result_type, "canceled");
    assert.equal(combined.error, undefined);
    combined.events = await evaluate(`window.__statusEvents.filter(e=>e.id===${JSON.stringify(combined.id)})`);
    assert(combined.events.filter((event) => event.event === "status_snapshot").length >= 2);
    save("combined-status-hud.json", combined);
  }
  await click("document.querySelector('#primary-tab-clean')");
  await waitFor("location.hash==='#/clean'");
  save("result.json", { pass: true, resource_gates: "pending sample analysis; this file proves lifecycle only", launch, main_handle: mainHandle,
    native_sha256: createHash("sha256").update(readFileSync(launch.executable)).digest("hex"),
    pre_live_start_ms: preLiveStart, pre_live_end_ms: preLiveEnd, cycles, requested_cycle_count: requestedStatusCycles, hud_cycles: hudCycles,
    method: "real UI Status start/stop; CDP observes native IPC request/completed response and callback Map records unchanged channel data; start response follows native join; no stop ack substituted for join" });
  console.log(JSON.stringify({ pass: true, cycles: cycles.length, output }));
} catch (error) {
  save("failure.json", { error: String(error), stack: error.stack, cycles, calls });
  throw error;
} finally {
  try { native("restored", mainHandle); } catch { /* Identity/exit failure remains a failed measurement. */ }
  socket.close();
}
