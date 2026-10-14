// Packaged Clean smoke on an explicitly owned measure-readonly.py launch.
// Exercises root selection and read-only scanning. Never previews or executes cleanup.
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [launchArg, outputArg, rootA, rootB, cliArg] = process.argv.slice(2);
if (!launchArg || !outputArg || !rootA || !rootB || !cliArg) {
  throw new Error("usage: node tools/native-clean-smoke.mjs <launch.json> <fresh-output> <root-A> <root-B> <CLI.exe>");
}
const launchPath = resolve(launchArg), output = resolve(outputArg);
// Expected lexical Windows display form for these fixture assertions only.
const expectedRoots = [rootA, rootB].map((root) => root.replaceAll("\\", "/").toLowerCase());
const launch = JSON.parse(readFileSync(launchPath, "utf8"));
const port = launch.debug_port;
assert(Number.isInteger(port) && port >= 1024 && port <= 65535);
assert.equal(typeof launch.created, "string", "FILETIME must not lose precision in JSON");
assert.notEqual(rootA.slice(0, 2).toLowerCase(), rootB.slice(0, 2).toLowerCase(), "two real fixture drives required");
mkdirSync(output, { recursive: false });
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const write = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: "wx" });
const digest = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
function verifyOwner() {
  const owner = execFileSync("powershell.exe", ["-NoProfile", "-Command",
    `Get-NetTCPConnection -State Listen -LocalPort ${port} -ErrorAction Stop | Select-Object -ExpandProperty OwningProcess -Unique`], { encoding: "utf8", windowsHide: true }).trim();
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
  return proof;
}
const owner = verifyOwner();
const targets = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(10000) }).then((r) => r.json());
write("debug-targets.json", targets);
const page = targets.find((t) => t.type === "page" && /^https?:\/\/tauri\.localhost\//.test(t.url) && !t.url.includes("hud.html"));
assert(page, "packaged main page required; no dev-server fallback");
const debuggerUrl = new URL(page.webSocketDebuggerUrl);
assert.equal(debuggerUrl.hostname, "127.0.0.1");
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
async function shot(name) {
  const image = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(output, name), Buffer.from(image.data, "base64"), { flag: "wx" });
}
const labels = JSON.parse(readFileSync(join(repo, "resources/i18n/en.json"), "utf8")).messages;
const projectLabel = labels["clean.v1.scope.projects"].forms.other;
const globalLabel = labels["clean.v1.scope.global"].forms.other;
const scopeInput = (label) => `[...document.querySelectorAll('.scope-controls label')].find(e=>e.textContent.trim()===${JSON.stringify(label)}).querySelector('input[type="checkbox"]')`;
const readScope = () => evaluate("[...document.querySelectorAll('.scope-controls label')].map(e=>({label:e.textContent.trim(),checked:e.querySelector('input').checked,disabled:e.querySelector('input').disabled}))");
let expectedOptions;
async function assertProjectScope(roots) {
  const scope = await readScope();
  assert.deepEqual(scope.map((row) => row.label).sort(), [projectLabel, globalLabel].sort());
  assert.deepEqual(scope.find((row) => row.label === projectLabel), { label: projectLabel, checked: true, disabled: false });
  assert.deepEqual(scope.find((row) => row.label === globalLabel), { label: globalLabel, checked: false, disabled: false });
  assert.deepEqual(await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)"), roots);
  return scope;
}
async function projectScope(roots, name) {
  const initial = await readScope();
  assert.deepEqual(initial.map((row) => row.label).sort(), [projectLabel, globalLabel].sort());
  assert(initial.every((row) => !row.disabled));
  if (initial.find((row) => row.label === globalLabel).checked) await click(scopeInput(globalLabel));
  if (!initial.find((row) => row.label === projectLabel).checked) await click(scopeInput(projectLabel));
  expectedOptions = { roots, include_projects: true, include_global: false };
  write(`${name}-scope-preflight.json`, { initial, verified: await assertProjectScope(roots), expected_options: expectedOptions });
}
try {
  await send("Page.enable");
  await send("Network.enable");
  await send("Network.emulateNetworkConditions", { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
  await send("Page.reload", { ignoreCache: true });
  await waitFor(`document.querySelector('[data-planet="clean"]')?.dataset.renderState==='ready' && !!document.querySelector('.scan-roots input')`);
  const offline = await evaluate(`({online:navigator.onLine,url:location.href,dpr:devicePixelRatio,width:innerWidth,height:innerHeight,render:document.querySelector('[data-planet="clean"]').dataset.renderState,resources:performance.getEntriesByType('resource').map(r=>r.name),scripts:[...document.scripts].map(s=>s.src)})`);
  assert.equal(offline.online, false);
  assert(offline.resources.some((url) => url.endsWith("/earth/natural-earth.png")));
  assert(offline.resources.every((url) => ["tauri.localhost", "ipc.localhost"].includes(new URL(url).hostname)), "only packaged assets and native IPC may load offline");
  write("offline.json", offline); await shot("offline-empty.png");
  await send("Network.setBlockedURLs", { urls: ["*natural-earth.png*"] });
  await send("Page.reload", { ignoreCache: true });
  await waitFor(`document.querySelector('[data-planet="clean"]')?.dataset.renderState==='fallback' && document.querySelector('.planet-fallback')?.naturalWidth===512 && !!document.querySelector('.scan-roots input')`);
  await shot("offline-static-fallback.png");
  await send("Network.setBlockedURLs", { urls: [] });
  await send("Page.reload", { ignoreCache: true });
  await waitFor(`document.querySelector('[data-planet="clean"]')?.dataset.renderState==='ready' && !!document.querySelector('.scan-roots input')`);
  // Offline Earth is proved above. Restore the webview network state for
  // native IPC observation: offline Tauri can use its non-Network fallback.
  await send("Network.emulateNetworkConditions", { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  assert.equal(await evaluate("document.querySelectorAll('.scan-roots > ul > li').length"), 0);
  assert.equal(await evaluate(`${button("Scan")}.disabled`), true);
  // Tauri's invoke property is immutable. Observe actual IPC request/response
  // bytes through CDP; do not replace the bridge or call a synthetic scanner.
  let captured = { network_requests: [] };
  let scanRequestId;
  let rejectedScopeCancel;
  onEvent = (event) => {
    const params = event.params;
    if (event.method === "Network.requestWillBeSent") captured.network_requests.push({ url: params.request.url, method: params.request.method });
    if (event.method === "Network.requestWillBeSent" && params.request.method === "POST"
      && new URL(params.request.url).hostname === "ipc.localhost" && new URL(params.request.url).pathname === "/scan_start") {
      scanRequestId = params.requestId;
      try {
        captured.request = JSON.parse(params.request.postData); captured.options = captured.request.options;
        try { assert.deepEqual(captured.options, expectedOptions); }
        catch (error) {
          captured.scope_guard_failure = String(error);
          write("scope-request-rejected.json", { request: captured.request, error: String(error) });
          rejectedScopeCancel = (async () => {
            await waitFor(`${button("Cancel scan")} && !${button("Cancel scan")}.disabled`);
            await click(button("Cancel scan"));
            captured.scope_guard_cancel_clicked = true;
          })().catch((error) => { captured.scope_guard_cancel_error = String(error); });
        }
      }
      catch (error) { captured.error = String(error); }
    } else if (params?.requestId === scanRequestId && event.method === "Network.loadingFinished") {
      void send("Network.getResponseBody", { requestId: scanRequestId }).then((response) => {
        const body = response.base64Encoded ? Buffer.from(response.body, "base64").toString("utf8") : response.body;
        captured.result = JSON.parse(body);
      }).catch((error) => { captured.error = String(error); });
    } else if (params?.requestId === scanRequestId && event.method === "Network.loadingFailed") {
      captured.error = params.errorText;
    }
  };
  async function addRoot(root, count) {
    await click("document.querySelector('.scan-roots input')");
    await send("Input.dispatchKeyEvent", { type: "keyDown", key: "a", code: "KeyA", modifiers: 2, windowsVirtualKeyCode: 65 });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: "a", code: "KeyA", modifiers: 2, windowsVirtualKeyCode: 65 });
    await send("Input.insertText", { text: root });
    await click("document.querySelector('.scan-root-entry button')");
    await waitFor(`document.querySelectorAll('.scan-roots > ul > li').length===${count} && document.querySelector('.scan-roots input').value==='' && !document.querySelector('.scan-roots').disabled`);
  }
  await addRoot(rootA, 1); await addRoot(rootA, 1); await addRoot(rootB, 2);
  const missingRoot = join(output, "missing-root-never-created").replaceAll("\\", "/").toLowerCase();
  assert.equal(existsSync(missingRoot), false);
  await click("document.querySelector('.scan-roots input')");
  await send("Input.insertText", { text: missingRoot });
  await click("document.querySelector('.scan-root-entry button')");
  await waitFor("!!document.querySelector('.clean-mode .error-banner') && !document.querySelector('.scan-roots').disabled");
  assert.deepEqual(await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)"), expectedRoots);
  write("missing-root-validation.json", await evaluate("({error:document.querySelector('.clean-mode .error-banner').innerText,roots:[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)})"));
  await shot("missing-root-validation.png");
  await click(button("Clear folder selections"));
  await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===0 && !document.querySelector('.scan-roots').disabled");
  assert.equal(await evaluate(`${button("Scan")}.disabled`), true);
  await shot("cleared-scope.png");
  await addRoot(rootA, 1); await addRoot(rootB, 2);
  const nestedRoot = join(rootA, "node_modules");
  await addRoot(nestedRoot, 3);
  await click(`[...document.querySelectorAll('.scan-roots > ul > li')].find(e=>e.querySelector('span').textContent===${JSON.stringify(expectedRoots[0] + "/node_modules")}).querySelector('button')`);
  await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===2 && !document.querySelector('.scan-roots').disabled");
  // The roots must survive the real persisted-preferences startup path.
  await send("Page.reload", { ignoreCache: true });
  await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===2 && !document.querySelector('.scan-roots').disabled");
  assert.deepEqual(await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)"), expectedRoots);
  await projectScope(expectedRoots, "healthy");
  await shot("explicit-two-roots.png");
  // Reach Scan through actual Tab input, rather than programmatic element.focus().
  let reached = false;
  for (let i = 0; i < 45; i++) {
    await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
    if (await evaluate("document.activeElement?.tagName==='BUTTON' && document.activeElement.textContent.trim()==='Scan'")) { reached = true; break; }
  }
  assert(reached, "Scan must be keyboard reachable");
  await assertProjectScope(expectedRoots);
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
  write("after-keyboard-scan.json", await evaluate("({url:location.href,online:navigator.onLine,text:document.body.innerText,focused:document.activeElement?.outerHTML,tauri_keys:Object.keys(window.__TAURI_INTERNALS__),callback_map:window.__TAURI_INTERNALS__.callbacks instanceof Map})"));
  const scanDeadline = Date.now() + 30000;
  while (!captured.result && !captured.error && Date.now() < scanDeadline) {
    await new Promise((accept) => setTimeout(accept, 100));
  }
  if (rejectedScopeCancel) await rejectedScopeCancel;
  write("tauri-observation.json", captured);
  assert.equal(captured.scope_guard_failure, undefined);
  assert(captured.result || captured.error, "real IPC scan response was not observed before deadline");
  assert.equal(captured.error, undefined); assert.equal(captured.result.type, "completed");
  assert.deepEqual(captured.options, { roots: expectedRoots, include_projects: true, include_global: false });
  assert.equal(captured.result.report.plan.targets.length, 2);
  write("tauri-report.json", captured);
  await waitFor(`!!${button("Review targets")}`);
  await click(button("Review targets"));
  await waitFor("!!document.querySelector('.preview-groups')");
  if (await evaluate("!!document.querySelector('.skipped-group:not([open])')")) await click("document.querySelector('.skipped-group > summary')");
  await waitFor("document.querySelectorAll('.preview-groups tbody tr').length===2");
  await evaluate("document.querySelector('.preview-groups').scrollIntoView({block:'start'})");
  await shot("native-review.png");
  write("accessibility.json", await send("Accessibility.getFullAXTree"));
  const profile = join(dirname(launchPath), "profile");
  const env = { ...process.env, LOCALAPPDATA: join(profile, "Local"), APPDATA: join(profile, "Roaming"), TEMP: join(profile, "Temp"), TMP: join(profile, "Temp") };
  const argv = ["clean", "scan", "--root", rootA, "--root", rootB, "--scope", "projects", "--format", "json"];
  const cli = JSON.parse(execFileSync(resolve(cliArg), argv, { encoding: "utf8", cwd: output, env, timeout: 60000, windowsHide: true }));
  write("cli-envelope.json", cli);
  assert.deepEqual(cli.data, captured.result.report, "CLI and Tauri complete report parity, without dropping fields");

  // Seed only this launch's saved preferences to model a selected folder that
  // disappeared between sessions. The real preferences loader and scanner run;
  // no JS state, validation outcome or native response is replaced.
  const preferencesPath = join(profile, "Local/DevSweep/settings/clean-scan-roots-v1.json");
  const savedPreferences = readFileSync(preferencesPath);
  writeFileSync(join(output, "healthy-saved-preferences.json"), savedPreferences, { flag: "wx" });
  const emptyRoot = join(output, "empty-root"); mkdirSync(emptyRoot);
  const cases = [
    { name: "mixed-missing", roots: [...expectedRoots, missingRoot], targets: 2, completeness: "partial", statuses: ["complete", "complete", "failed"], exit: 5 },
    { name: "zero-failed", roots: [missingRoot], targets: 0, completeness: "partial", statuses: ["failed"], exit: 5, headline: "No targets found; scan coverage is incomplete." },
    { name: "empty", roots: [emptyRoot.replaceAll("\\", "/").toLowerCase()], targets: 0, completeness: "complete", statuses: ["empty"], exit: 0, headline: "Scan complete; no cleanup targets found." },
  ];
  const scopeResults = [];
  try {
    for (const test of cases) {
      writeFileSync(preferencesPath, JSON.stringify({ schema_version: 1, roots: test.roots }));
      captured = { network_requests: [] }; scanRequestId = undefined; rejectedScopeCancel = undefined;
      await send("Page.reload", { ignoreCache: true });
      await waitFor(`document.querySelectorAll('.scan-roots > ul > li').length===${test.roots.length} && !document.querySelector('.scan-roots').disabled`);
      assert.deepEqual(await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)"), test.roots);
      await projectScope(test.roots, test.name);
      await assertProjectScope(test.roots);
      await click(button("Scan"));
      const deadline = Date.now() + 30000;
      while (!captured.result && !captured.error && Date.now() < deadline) await new Promise((accept) => setTimeout(accept, 50));
      if (rejectedScopeCancel) await rejectedScopeCancel;
      write(`${test.name}-tauri.json`, captured);
      assert.equal(captured.scope_guard_failure, undefined);
      assert.equal(captured.error, undefined); assert.equal(captured.result?.type, "completed");
      assert.deepEqual(captured.options, { roots: test.roots, include_projects: true, include_global: false });
      const report = captured.result.report;
      assert.equal(report.plan.targets.length, test.targets);
      assert.equal(report.health.completeness, test.completeness);
      assert.deepEqual(report.health.coverage.map((unit) => unit.status).sort(), [...test.statuses].sort());
      await waitFor("!!document.querySelector('.scan-coverage')");
      if (test.headline) await waitFor(`document.querySelector('.stage-title')?.textContent===${JSON.stringify(test.headline)}`);
      write(`${test.name}-ui.json`, await evaluate("({headline:document.querySelector('.stage-title')?.textContent,text:document.querySelector('.clean-mode').innerText})"));
      await shot(`${test.name}.png`);
      const argumentsForCase = ["clean", "scan", ...test.roots.flatMap((root) => ["--root", root]), "--scope", "projects", "--format", "json"];
      const processResult = spawnSync(resolve(cliArg), argumentsForCase, { encoding: "utf8", cwd: output, env, timeout: 60000, windowsHide: true });
      const envelope = JSON.parse(processResult.stdout);
      write(`${test.name}-cli.json`, { status: processResult.status, error: processResult.error?.message, stderr: processResult.stderr, argv: argumentsForCase, envelope });
      assert.equal(processResult.error, undefined); assert.equal(processResult.status, test.exit);
      assert.deepEqual(envelope.data, report, `${test.name}: every-field CLI/Tauri parity with identical input roots`);
      scopeResults.push({ case: test.name, targets: test.targets, completeness: test.completeness, coverage_statuses: test.statuses, cli_exit: test.exit, full_report_parity: true });
    }
  } finally {
    writeFileSync(preferencesPath, savedPreferences);
    await send("Page.reload", { ignoreCache: true });
    await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===2 && !document.querySelector('.scan-roots').disabled");
  }
  write("result.json", { pass: true, owner, native_sha256: digest(launch.executable), cli_sha256: digest(resolve(cliArg)), argv, roots: [rootA, rootB], offline: true, native_asset_failure_fallback: true, duplicate_root_deduped: true, missing_root_add_rejected: true, cleared_scope_disabled_then_recovered: true, nested_selection_removed_without_excluding_parent: true, roots_persist_after_reload: true, keyboard_scan: true, targets: 2, full_report_parity: true, scope_cases: scopeResults, acl_denied_native_test: "not exercised; missing path is not permission denial", cleanup_executed: false });
  console.log(JSON.stringify({ pass: true, output, targets: 2, full_report_parity: true }));
} catch (error) {
  write("failure.json", { message: String(error), stack: error.stack });
  try { await shot("failure.png"); } catch { /* Preserve the original failure. */ }
  throw error;
} finally {
  socket.close();
}
