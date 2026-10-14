// One read-only UI operation sample on an explicitly owned packaged app.
// Run separate invocations to alternate baseline/candidate without hidden warmups.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const [launchArg, outputArg, operation, fixtureArg] = process.argv.slice(2);
assert(launchArg && outputArg && ["software", "optimize", "analyze-cancel"].includes(operation),
  "usage: node tools/native-operation-smoke.mjs <launch.json> <fresh-output> <software|optimize|analyze-cancel> [task-fixture]");
const repo = resolve(dirname(fileURLToPath(import.meta.url)), ".."), output = resolve(outputArg);
const launch = JSON.parse(readFileSync(resolve(launchArg), "utf8"));
assert.equal(typeof launch.created, "string", "exact FILETIME required");
assert(Number.isInteger(launch.debug_port) && launch.debug_port >= 1024 && launch.debug_port <= 65535);
let fixture;
if (operation === "analyze-cancel") {
  assert(fixtureArg, "Analyze requires a task-owned fixture");
  fixture = realpathSync(resolve(fixtureArg));
  const allowed = realpathSync(join(repo, "dist/performance/cleanup-earth/fixtures"));
  assert(fixture.toLowerCase().startsWith((allowed + sep).toLowerCase()), "Analyze root must be inside task fixtures");
}
mkdirSync(output, { recursive: false });
const save = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: "wx" });
const pause = (ms) => new Promise((accept) => setTimeout(accept, ms));
function verifyOwner() {
  const owner = execFileSync("powershell.exe", ["-NoProfile", "-Command",
    `Get-NetTCPConnection -State Listen -LocalPort ${launch.debug_port} -ErrorAction Stop | Select-Object -ExpandProperty OwningProcess -Unique`], { encoding: "utf8", windowsHide: true }).trim();
  assert.match(owner, /^\d+$/);
  return JSON.parse(execFileSync("python", ["-c", `
import importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
s=module.WindowsSampler();pid=int(sys.argv[2]);created=int(sys.argv[3])
assert s.inspect({'pid':pid}).get('created')==created,'root process identity changed'
rows=s.tree(pid,{pid:created});owner=int(sys.argv[4])
assert any(row['pid']==owner for row in rows),'debug listener is not owned by this launch'
print(json.dumps({'pid':pid,'created':str(created),'listener_pid':owner}))
`, join(repo, "tools/measure-readonly.py"), String(launch.pid), launch.created, owner], { encoding: "utf8", windowsHide: true }));
}
const owner = verifyOwner();
const targets = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(10000) }).then((r) => r.json());
const page = targets.find((target) => target.type === "page" && /^https?:\/\/tauri\.localhost\//.test(target.url) && !target.url.includes("hud.html"));
assert(page, "packaged main page required");
const endpoint = new URL(page.webSocketDebuggerUrl);
assert.equal(endpoint.hostname, "127.0.0.1");
assert.equal(endpoint.port, String(launch.debug_port));
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept, reject) => {
  const timer = setTimeout(() => reject(new Error("CDP connection timeout")), 10000);
  socket.onopen = () => { clearTimeout(timer); accept(); };
  socket.onerror = () => { clearTimeout(timer); reject(new Error("CDP connection failed")); };
});
let sequence = 0;
const pending = new Map(), requests = new Map(), observations = [];
const commands = operation === "software" ? ["software_inventory_start"] : operation === "optimize" ? ["optimize_list"] : ["analyze_start", "analyze_cancel"];
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data), waiter = pending.get(message.id);
  if (waiter) {
    pending.delete(message.id); clearTimeout(waiter.timer);
    if (message.error) waiter.reject(new Error(message.error.message)); else waiter.accept(message.result);
    return;
  }
  const event = message.params;
  if (message.method === "Network.requestWillBeSent" && event.request.method === "POST") {
    const url = new URL(event.request.url), command = url.pathname.slice(1);
    if (url.hostname !== "ipc.localhost" || !commands.includes(command)) return;
    const record = { command, request_id: event.requestId, started_seconds: event.timestamp, request: JSON.parse(event.request.postData) };
    requests.set(event.requestId, record); observations.push(record);
  }
  const record = requests.get(event?.requestId);
  if (!record) return;
  if (message.method === "Network.responseReceived") record.http_status = event.response.status;
  if (message.method === "Network.loadingFailed") record.error = event.errorText;
  if (message.method === "Network.loadingFinished") {
    record.finished_seconds = event.timestamp;
    record.request_to_response_ms = (event.timestamp - record.started_seconds) * 1000;
    void send("Network.getResponseBody", { requestId: event.requestId }).then((response) => {
      record.response = JSON.parse(response.base64Encoded ? Buffer.from(response.body, "base64").toString("utf8") : response.body);
      record.done = true;
    }).catch((error) => { record.error = String(error); });
  }
};
socket.onclose = () => {
  for (const waiter of pending.values()) { clearTimeout(waiter.timer); waiter.reject(new Error("CDP closed")); }
  pending.clear();
};
function send(method, params = {}) {
  return new Promise((accept, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 30000);
    pending.set(id, { accept, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function until(condition, description, timeout = 60000) {
  const deadline = Date.now() + timeout;
  do { if (await condition()) return; await pause(20); } while (Date.now() < deadline);
  throw new Error(`Timed out: ${description}`);
}
async function click(expression) {
  const point = await evaluate(`(() => {const e=${expression};if(!e||e.disabled)throw Error('control unavailable');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, ...point, button: "left", clickCount: 1 });
}
const button = (label) => `[...document.querySelectorAll('button')].find(e=>e.textContent.trim()===${JSON.stringify(label)})`;
async function settled(command) {
  await until(() => observations.some((row) => row.command === command && (row.done || row.error)), command);
  const record = observations.find((row) => row.command === command);
  assert.equal(record.error, undefined); assert.equal(record.http_status, 200);
  return record;
}
try {
  await send("Page.enable"); await send("Network.enable");
  assert.equal(await evaluate("document.visibilityState"), "visible", "sampled native window must be restored");
  const mode = operation === "analyze-cancel" ? "analyze" : operation;
  await click(`document.getElementById('primary-tab-${mode}')`);
  await until(() => evaluate(`!!document.querySelector('.${mode}-mode')`), "mode mounted");
  if (fixture) {
    await until(() => evaluate("!!document.querySelector('.analyze-root-input input')?.value"), "default root loaded");
    await click("document.querySelector('.analyze-root-input input')");
    await send("Input.dispatchKeyEvent", { type: "keyDown", key: "a", code: "KeyA", modifiers: 2, windowsVirtualKeyCode: 65 });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: "a", code: "KeyA", modifiers: 2, windowsVirtualKeyCode: 65 });
    await send("Input.insertText", { text: fixture });
    assert.equal(await evaluate("document.querySelector('.analyze-root-input input').value"), fixture);
  }
  const label = operation === "software" ? "Refresh inventory" : operation === "optimize" ? "Reload catalogue" : "Analyze path";
  // Completed Optimize/Analyze stages expose the repeated action in Details.
  if (!await evaluate(`!!${button(label)}`)) await click("document.querySelector('.stage-action')");
  const clicked = performance.now();
  await click(button(label));
  let cancel;
  if (fixture) {
    await until(() => observations.some((row) => row.command === "analyze_start"), "actual Analyze start request");
    assert(!observations.find((row) => row.command === "analyze_start").done, "unavailable: analysis finished before cancellation input");
    await click(button("Cancel analysis"));
    cancel = await settled("analyze_cancel");
  }
  const start = await settled(commands[0]);
  for (const command of commands) assert.equal(observations.filter((row) => row.command === command).length, 1, "one actual IPC request per action");
  assert.equal(start.response.operation_id, start.request.operationId);
  assert.equal(start.response.type, fixture ? "canceled" : "completed");
  if (cancel) {
    assert.equal(cancel.request.operationId, start.request.operationId);
    assert.equal(cancel.response, null);
    assert.equal(start.request.root, fixture);
  }
  await until(() => evaluate(`document.querySelector('.${mode}-mode')?.dataset.status===${JSON.stringify(fixture ? "canceled" : "ready")}`), "terminal UI state");
  const clickToTerminalUi = performance.now() - clicked;
  const ui = await evaluate(`({url:location.href,status:document.querySelector('.${mode}-mode').dataset.status,user_agent:navigator.userAgent,dpr:devicePixelRatio,width:innerWidth,height:innerHeight,visibility:document.visibilityState,text_excerpt:document.querySelector('.${mode}-mode').innerText.slice(0,2000)})`);
  const screenshot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(output, "terminal.png"), Buffer.from(screenshot.data, "base64"), { flag: "wx" });
  save("result.json", {
    status: "pass", operation, launch, owner, executable_sha256: createHash("sha256").update(readFileSync(launch.executable)).digest("hex"), fixture,
    fixture_manifest_sha256: fixture ? createHash("sha256").update(readFileSync(join(repo, "dist/performance/cleanup-earth/fixtures/fixture-manifest.json"))).digest("hex") : null,
    inventory: start.response.inventory ? { entry_count: start.response.inventory.entries.length, sources: start.response.inventory.sources } : null,
    ui, click_to_terminal_ui_ms: clickToTerminalUi,
    request_to_response_ms: start.request_to_response_ms,
    cancel_ack_ms: cancel?.request_to_response_ms,
    cancel_request_to_joined_result_ms: cancel ? (start.finished_seconds - cancel.started_seconds) * 1000 : null,
    method: "Actual UI input and native Network IPC response. CDP monotonic request/loadingFinished timestamps; UI completion includes polling. Analyze terminal response follows the awaited worker join. One sample only, no percentile/pass-budget inference.",
  });
} catch (error) {
  save("failure.json", { operation, error: String(error), launch });
  throw error;
} finally {
  save("ipc.json", observations);
  socket.close();
}
