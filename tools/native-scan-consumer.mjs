// Read-only real native scan with a deliberately paused WebView consumer.
// Observes unchanged progress and complete IPC results; never invokes cleanup.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [launchArg, outputArg, rootArg, pauseArg = "3000", mode = "pause"] = process.argv.slice(2);
assert(launchArg && outputArg && rootArg, "usage: node tools/native-scan-consumer.mjs <launch.json> <fresh-output> <fixture-root> [pause-ms] [pause|minimize|reload|cancel|close]");
assert(["pause", "minimize", "reload", "cancel", "close"].includes(mode));
const pauseMs = Number(pauseArg);
assert(Number.isInteger(pauseMs) && pauseMs >= 100 && pauseMs <= 30000);
const launchPath = resolve(launchArg), output = resolve(outputArg);
const root = resolve(rootArg);
const launch = JSON.parse(readFileSync(launchPath, "utf8"));
const port = launch.debug_port;
assert(Number.isInteger(port) && port >= 1024 && port <= 65535);
assert.equal(typeof launch.created, "string", "FILETIME must not lose precision in JSON");
mkdirSync(output, { recursive: false });
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const write = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: "wx" });
const digest = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
function verifyOwner() {
  const listeners = JSON.parse(execFileSync("powershell.exe", ["-NoProfile", "-Command",
    `Get-NetTCPConnection -State Listen -LocalPort ${port} -ErrorAction Stop | Select-Object LocalAddress,OwningProcess | ConvertTo-Json -Compress`], { encoding: "utf8", windowsHide: true }));
  const listener = [listeners].flat().find((row) => ["127.0.0.1", "::1"].includes(row.LocalAddress));
  assert(listener, "owned loopback listener required");
  const owner = String(listener.OwningProcess);
  assert.match(owner, /^\d+$/);
  const proof = JSON.parse(execFileSync("python", ["-c", `
import importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
sampler=module.WindowsSampler(); pid=int(sys.argv[2]); created=int(sys.argv[3])
assert sampler.inspect({'pid':pid}).get('created')==created,'root process identity changed'
rows=sampler.tree(pid,{pid:created}); owner=int(sys.argv[4])
assert any(row['pid']==owner for row in rows),'debug port is not owned by this launch'
print(json.dumps({'root_pid':pid,'created':str(created),'listener_pid':owner}))
`, join(repo, "tools/measure-readonly.py"), String(launch.pid), launch.created, owner], { encoding: "utf8", windowsHide: true }));
  return { ...proof, address: listener.LocalAddress };
}
const owner = verifyOwner();
function native(action) {
  return JSON.parse(execFileSync("python", ["-c", `
import importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1]);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
s=m.WindowsSampler();pid=int(sys.argv[2]);created=int(sys.argv[3]);action=sys.argv[4]
if s.inspect({'pid':pid}).get('created')!=created:
 print(json.dumps({'exited':True}));sys.exit(0)
windows=[w for w in s.windows(pid,created) if w['class']=='Tauri Window' and w['title']=='DevSweep']
if action!='inspect':assert len(windows)==1,'fresh main-only process required'
if action in ('minimized','restored'):s.main_window_state(pid,created,action,windows[0]['handle'])
if action=='close':s.close_window(pid,created)
print(json.dumps({'exited':False,'windows':s.windows(pid,created)}))
`, join(repo, "tools/measure-readonly.py"), String(launch.pid), launch.created, action], { encoding: "utf8", windowsHide: true }));
}
const host = owner.address === "::1" ? "[::1]" : owner.address;
const targets = await fetch(`http://${host}:${port}/json/list`, { signal: AbortSignal.timeout(10000) }).then((r) => r.json());
write("debug-targets.json", targets);
const page = targets.find((t) => t.type === "page" && /^https?:\/\/tauri\.localhost\//.test(t.url) && !t.url.includes("hud.html"));
assert(page, "packaged main page required; no dev-server fallback");
const debuggerUrl = new URL(page.webSocketDebuggerUrl);
assert.equal(debuggerUrl.hostname, host);
assert.equal(debuggerUrl.port, String(port));
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept, reject) => {
  const timer = setTimeout(() => reject(new Error("CDP connect timeout")), 10000);
  socket.onopen = () => { clearTimeout(timer); accept(); };
  socket.onerror = (error) => { clearTimeout(timer); reject(error); };
});
verifyOwner();
let next = 0;
const pending = new Map();
let onEvent = () => {};
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.method) { onEvent(message); return; }
  const waiter = pending.get(message.id);
  if (!waiter) return;
  pending.delete(message.id); clearTimeout(waiter.timer);
  if (message.error) waiter.reject(new Error(message.error.message));
  else waiter.accept(message.result);
};
function send(method, params = {}) {
  return new Promise((accept, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 30000);
    pending.set(id, { accept, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const value = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw new Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
}
async function waitFor(expression) {
  const until = Date.now() + 30000;
  do {
    if (await evaluate(expression)) return;
    await new Promise((accept) => setTimeout(accept, 100));
  } while (Date.now() < until);
  throw new Error(`Native UI condition timed out: ${expression}`);
}
async function click(expression) {
  const point = await evaluate(`(() => { const e=${expression}; if(!e||e.disabled)throw Error('control missing or disabled'); e.scrollIntoView({block:'center'}); const r=e.getBoundingClientRect(); return {x:r.x+r.width/2,y:r.y+r.height/2}; })()`);
  for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, ...point, button: "left", clickCount: 1 });
}
const button = (label) => `[...document.querySelectorAll('button')].find(e=>e.textContent.trim()===${JSON.stringify(label)})`;
const scopeInput = (label) => `[...document.querySelectorAll('.scope-controls label')].find(e=>e.textContent.trim()===${JSON.stringify(label)})?.querySelector('input')`;
let preflightCount = 0;
async function startFixtureScan() {
  if (!await evaluate("!!document.querySelector('.scope-controls')")) {
    await click(button("Show details"));
  }
  for (const [label, checked] of [["Projects", true], ["Global caches", false]]) {
    const input = scopeInput(label);
    await waitFor(`!!(${input}) && !(${input}).disabled`);
    if (await evaluate(`(${input}).checked`) !== checked) await click(input);
    await waitFor(`(${input}).checked===${checked}`);
  }
  const scope = await evaluate(`({projects:(${scopeInput("Projects")}).checked,global:(${scopeInput("Global caches")}).checked,roots:[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)})`);
  assert.deepEqual(scope, { projects: true, global: false, roots: [root.replaceAll("\\", "/").toLowerCase()] }, "exact fixture scope required before clicking Scan");
  write(`scope-preflight-${++preflightCount}.json`, scope);
  await click(button("Scan"));
}
const progress = [], requests = [], channelFetches = [];
const abandonedRequests = new Set(), abandonedResponses = [];
const reloadRetries = [];
let scanRequest, result, failure, requestAt, settledAt, pausedAt, resumedAt, reloadedAt, closeRequested;
const nativeStates = [];
let pauseRequested = false, navigations = 0;
const pause = (ms) => new Promise((accept) => setTimeout(accept, ms));
onEvent = (event) => {
  const { method, params } = event;
  if (method === "Page.frameNavigated" && !params.frame.parentId) navigations++;
  if (method === "Runtime.bindingCalled" && params.name === "__scanObserved") {
    const observation = JSON.parse(params.payload);
    progress.push(observation);
    if (!pauseRequested && observation.targets > 0) {
      pauseRequested = true;
      if (mode === "minimize") {
        pausedAt = Date.now();
        nativeStates.push(native("minimized"));
        void pause(pauseMs).then(() => { nativeStates.push(native("restored")); resumedAt = Date.now(); });
      } else if (mode === "close") {
        closeRequested = Date.now();
        nativeStates.push(native("close"));
      } else {
        void send("Debugger.pause").catch((error) => { failure = String(error); });
      }
    }
  }
  if (method === "Debugger.paused") {
    pausedAt = Date.now();
    void pause(pauseMs).then(async () => {
      resumedAt = Date.now();
      if (mode === "reload") {
        reloadedAt = Date.now();
        abandonedRequests.add(scanRequest);
        await send("Page.reload");
      } else {
        await send("Debugger.resume");
        if (mode === "cancel") await click(button("Cancel scan"));
      }
    }).catch((error) => { failure = String(error); });
  }
  if (method === "Network.requestWillBeSent") {
    const request = params.request, url = new URL(request.url);
    if (url.hostname !== "ipc.localhost") return;
    const command = decodeURIComponent(url.pathname.slice(1));
    const observed = { command, method: request.method, request_id: params.requestId, at_ms: Date.now() };
    if (request.method === "POST" && command === "scan_progress_ack") {
      const args = JSON.parse(request.postData ?? "{}");
      observed.scan_id = args.scanId;
      observed.sequence = args.sequence;
    }
    requests.push(observed);
    if (command.includes("__TAURI_CHANNEL__")) channelFetches.push({ command, at_ms: Date.now(), request_id: params.requestId });
    if (request.method === "POST" && command === "scan_start") {
      scanRequest = params.requestId; requestAt = Date.now();
      const args = JSON.parse(request.postData);
      write(`request-${requests.filter((row) => row.command === "scan_start").length}.json`, args);
      if (args.options.include_projects !== true || args.options.include_global !== false
        || JSON.stringify(args.options.roots) !== JSON.stringify([root.replaceAll("\\", "/").toLowerCase()])) {
        failure = "scan request must preserve the exact task fixture and project-only scope";
      }
    }
  }
  if (method === "Network.loadingFailed" && abandonedRequests.has(params.requestId)) {
    abandonedResponses.push({ request_id: params.requestId, error: params.errorText, at_ms: Date.now() });
  }
  if (method === "Network.loadingFinished" && abandonedRequests.has(params.requestId)) {
    const record = { request_id: params.requestId, at_ms: Date.now() };
    abandonedResponses.push(record);
    void send("Network.getResponseBody", { requestId: params.requestId }).then((response) => {
      record.result = JSON.parse(response.base64Encoded ? Buffer.from(response.body, "base64").toString("utf8") : response.body);
    }).catch((error) => { record.unavailable_after_reload = String(error); });
    return;
  }
  if (method === "Network.loadingFinished" && params.requestId === scanRequest) {
    settledAt = Date.now();
    void send("Network.getResponseBody", { requestId: scanRequest }).then((response) => {
      result = JSON.parse(response.base64Encoded ? Buffer.from(response.body, "base64").toString("utf8") : response.body);
    }).catch((error) => { failure = String(error); });
  }
};
const installObserver = () => evaluate(`(() => {
  const callbacks=window.__TAURI_INTERNALS__.callbacks,original=callbacks.set;
  const wrap=callback=>function(value){
    const event=value?.message;
    if(event?.scan_id && Number.isInteger(event.sequence))window.__scanObserved(JSON.stringify({
      scan_id:event.scan_id,sequence:event.sequence,received_ms:Date.now(),phase:event.phase,
      targets:event.preview?.targets.length??0,bytes:new TextEncoder().encode(JSON.stringify(event)).length
    }));
    return callback.apply(this,arguments);
  };
  callbacks.set=function(id,callback){return original.call(this,id,wrap(callback));};
  for(const [id,callback]of callbacks)original.call(callbacks,id,wrap(callback));
})()`);
try {
  await send("Page.enable");
  await send("Network.enable");
  await send("Runtime.enable");
  await send("Debugger.enable");
  await send("Runtime.addBinding", { name: "__scanObserved" });
  await send("Page.reload");
  const navigationDeadline = Date.now() + 10000;
  while (navigations === 0 && Date.now() < navigationDeadline) await pause(50);
  assert(navigations > 0, "fresh document required for one observation wrapper");
  await waitFor("!!window.__TAURI_INTERNALS__ && !!document.querySelector('.scan-roots input') && !document.querySelector('.scan-roots').disabled");
  await installObserver();
  const roots = await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)");
  if (roots.length === 0) {
    await click("document.querySelector('.scan-roots input')");
    await send("Input.insertText", { text: root });
    await waitFor("!!document.querySelector('.scan-root-entry button') && !document.querySelector('.scan-root-entry button').disabled");
    await click("document.querySelector('.scan-root-entry button')");
  } else {
    assert.deepEqual(roots, [root.replaceAll("\\", "/").toLowerCase()], "only the same task fixture can be reused");
  }
  await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===1 && !document.querySelector('.scan-roots').disabled");
  await startFixtureScan();
  const deadline = Date.now() + 180000;
  if (mode === "close") {
    while (!closeRequested && !failure && Date.now() < deadline) await pause(100);
    assert(closeRequested, "close must happen during a real scan");
    while (!native("inspect").exited && Date.now() < deadline) await pause(100);
    assert(native("inspect").exited, "owned host must exit normally after scan close");
    write("result.json", { pass: true, mode, owner, close_requested_ms: closeRequested, progress, requests, nativeStates,
      teardown: "normal owned WM_CLOSE; outer sampler must confirm zero exit, no force and no leftovers" });
  } else {
  if (mode === "reload") {
    while ((!reloadedAt || navigations < 2) && !failure && Date.now() < deadline) await pause(100);
    assert(reloadedAt && navigations >= 2, "reload while receiver paused required");
    await waitFor(`!!window.__TAURI_INTERNALS__ && !!${button("Scan")} && !${button("Scan")}.disabled`);
    await installObserver();
    result = undefined; scanRequest = undefined;
    await startFixtureScan();
  }
  const reloadJoinDeadline = Date.now() + 30000;
  while ((!result || !resumedAt || (mode === "reload" && result.code === "scan_already_running")) && !failure && Date.now() < deadline) {
    if (mode === "reload" && result?.code === "scan_already_running") {
      reloadRetries.push({ at_ms: Date.now(), result });
      assert(Date.now() < reloadJoinDeadline, "canceled pre-reload scan must join before fresh scan");
      await pause(500);
      await waitFor(`!!${button("Scan")} && !${button("Scan")}.disabled`);
      result = undefined;
      await startFixtureScan();
    } else await pause(100);
  }
  assert.equal(failure, undefined);
  assert.equal(result?.type, mode === "cancel" ? "canceled" : "completed", "real scan must finish");
  assert(pausedAt && resumedAt, "non-vacuous paused receiver required");
  if (mode !== "cancel") await waitFor(`!!${button("Review targets")}`);
  await pause(500);
  assert(progress.length > 1);
  assert(progress.every((row, index) => index === 0 || row.scan_id !== progress[index - 1].scan_id || row.sequence > progress[index - 1].sequence));
  if (mode === "minimize") {
    assert(nativeStates[0].windows.some((window) => window.title === "DevSweep" && window.minimized), "actual native minimization required");
    assert(progress.some((row) => row.received_ms > pausedAt + 200 && row.received_ms < resumedAt), "explicit scan must continue while minimized");
  }
  const afterResume = progress.filter((row) => row.received_ms >= resumedAt);
  const burst = afterResume.filter((row) => row.received_ms <= resumedAt + 100);
  const acknowledgments = requests.filter((row) => row.command === "scan_progress_ack" && row.method === "POST");
  if (acknowledgments.length > 0 && result.type === "completed") {
    const completedId = progress.at(-1).scan_id;
    const completedProgress = progress.filter((row) => row.scan_id === completedId);
    assert(completedProgress.every((row) => acknowledgments.some((ack) => ack.scan_id === row.scan_id && ack.sequence === row.sequence)), "every completed scan packet must have its exact observed acknowledgment");
    assert(progress.at(-1).received_ms <= settledAt, "completed scan response must follow the final received progress");
  }
  if (result.report) write("report.json", result.report);
  write("result.json", {
    pass: true, mode, owner, native_sha256: digest(launch.executable), root, nativeStates, reloaded_at_ms: reloadedAt,
    request_at_ms: requestAt, backend_response_at_ms: settledAt,
    paused_at_ms: pausedAt, resumed_at_ms: resumedAt, requested_pause_ms: pauseMs,
    progress, requests, channel_fetches: channelFetches,
    abandoned_reload_responses: abandonedResponses,
    reload_join_retries: reloadRetries,
    received_count: progress.length, received_bytes: progress.reduce((n, row) => n + row.bytes, 0),
    acknowledgment_count: acknowledgments.length,
    acknowledgment_observations: acknowledgments.map((ack) => ({ ...ack,
      request_after_receive_ms: ack.at_ms - (progress.find((row) => row.scan_id === ack.scan_id && row.sequence === ack.sequence)?.received_ms ?? ack.at_ms),
    })),
    backend_settled_while_paused: settledAt >= pausedAt && settledAt < resumedAt,
    first_100ms_resume_burst_count: burst.length,
    first_100ms_resume_burst_bytes: burst.reduce((n, row) => n + row.bytes, 0),
    last_receive_after_backend_response_ms: progress.at(-1).received_ms - settledAt,
    completed_targets: result.report?.plan.targets.length ?? null, result_type: result.type,
    interpretation: "Observed receive timing/bytes and resume burst; no send timestamp is invented and no unobserved native queue bound is inferred. Closed decoder and application callbacks remain unchanged.",
  });
  console.log(JSON.stringify({ output, received: progress.length, resume_burst: burst.length }));
  }
} catch (error) {
  write("failure.json", { error: String(error), progress, requests, pausedAt, resumedAt, result, reloadRetries });
  throw error;
} finally {
  if (mode !== "close") try { await send("Debugger.resume"); } catch { /* Preserve earlier error. */ }
  socket.close();
}
