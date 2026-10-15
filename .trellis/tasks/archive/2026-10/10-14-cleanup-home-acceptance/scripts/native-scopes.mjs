// Native packaged Tauri acceptance, using observed UI and actual IPC only.
// This driver stops at successful dry-run previews. It never confirms cleanup.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const [launchArg, outArg, fixtureArg, repoArg] = process.argv.slice(2);
const launch = JSON.parse(readFileSync(launchArg, 'utf8'));
const fixture = JSON.parse(readFileSync(fixtureArg, 'utf8'));
const output = resolve(outArg), repo = resolve(repoArg);
mkdirSync(output, { recursive: false });
const save = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: 'wx' });
const pause = ms => new Promise(accept => setTimeout(accept, ms));
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const windows = fixture.environment.SystemRoot;
assert.equal(typeof launch.created, 'string');
assert(Number.isInteger(launch.pid) && launch.pid > 0);
assert(Number.isInteger(launch.debug_port) && launch.debug_port >= 1024 && launch.debug_port <= 65535);
assert.equal(fixture.environment.HOME, fixture.environment.USERPROFILE);
assert(fixture.environment.HOME.startsWith(fixture.root));
const profile = join(dirname(launchArg), 'profile');
const actualEnv = { ...fixture.environment, LOCALAPPDATA: join(profile, 'Local'), APPDATA: join(profile, 'Roaming'), TEMP: join(profile, 'Temp'), TMP: join(profile, 'Temp'), WEBVIEW2_USER_DATA_FOLDER: join(profile, 'WebView'), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${launch.debug_port}` };

function native(width = 0, height = 0) {
  return JSON.parse(execFileSync(fixture.python, ['-I', '-c', `
import ctypes as ct,ctypes.wintypes as wt,importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1]);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
s=module.WindowsSampler();pid=int(sys.argv[2]);created=int(sys.argv[3]);width=int(sys.argv[4]);height=int(sys.argv[5])
assert s.inspect({'pid':pid}).get('created')==created,'owned process identity changed'
user=ct.WinDLL('user32');user.SetProcessDpiAwarenessContext.argtypes=[ct.c_void_p];user.SetProcessDpiAwarenessContext(ct.c_void_p(-4))
user.GetDpiForWindow.argtypes=[wt.HWND];user.GetDpiForWindow.restype=wt.UINT
user.GetWindowRect.argtypes=[wt.HWND,ct.POINTER(wt.RECT)];user.GetClientRect.argtypes=[wt.HWND,ct.POINTER(wt.RECT)]
user.SetWindowPos.argtypes=[wt.HWND,wt.HWND,ct.c_int,ct.c_int,ct.c_int,ct.c_int,wt.UINT];user.SetWindowPos.restype=wt.BOOL
before=s.windows(pid,created)
for w in before:
 w['dpi']=user.GetDpiForWindow(w['handle'])
 if w['class']=='Tauri Window' and w['title']=='DevSweep' and width:
  wr=wt.RECT();cr=wt.RECT();user.GetWindowRect(w['handle'],ct.byref(wr));user.GetClientRect(w['handle'],ct.byref(cr))
  assert user.SetWindowPos(w['handle'],None,0,0,round(width*w['dpi']/96)+(wr.right-wr.left)-(cr.right-cr.left),round(height*w['dpi']/96)+(wr.bottom-wr.top)-(cr.bottom-cr.top),0x16)
rows=s.tree(pid,{pid:created})
for row in rows:
 if 'created' in row:row['created']=str(row['created'])
print(json.dumps({'windows':s.windows(pid,created),'before':before,'processes':rows}))
`, join(repo, 'tools/measure-readonly.py'), String(launch.pid), launch.created, String(width), String(height)], { encoding: 'utf8', windowsHide: true, env: actualEnv }));
}
let listeners = [];
let listenerRows = [];
let tcpRowCount = 0, launchDiagnostic = false;
const diagnosticAt = Date.now() + 3000;
for (const deadline = Date.now() + 30000; Date.now() < deadline;) {
  const rows = execFileSync(join(windows, 'System32/netstat.exe'), ['-ano'], { encoding: 'utf8', windowsHide: true, env: actualEnv }).trim().split(/\r?\n/).map(line => line.trim().split(/\s+/)).filter(parts => parts[0] === 'TCP');
  tcpRowCount = rows.length;
  listenerRows = rows.filter(parts => parts[1].endsWith(`:${launch.debug_port}`));
  listeners = listenerRows.filter(parts => parts[3] === 'LISTENING').map(parts => Number(parts[4]));
  if (listeners.length) break;
  if (!launchDiagnostic && Date.now() >= diagnosticAt) {
    launchDiagnostic = true;
    const snapshot = native();
    try {
      const childCommandLines = execFileSync(join(windows, 'System32/WindowsPowerShell/v1.0/powershell.exe'), ['-NoProfile', '-Command', `Get-CimInstance Win32_Process -Filter "ParentProcessId = ${launch.pid}" | Select-Object ProcessId,ParentProcessId,CommandLine | ConvertTo-Json -Compress`], { encoding: 'utf8', windowsHide: true, env: actualEnv });
      save('browser-launch.json', { snapshot, childCommandLines });
    } catch (error) { save('browser-launch.json', { snapshot, error: String(error) }); }
  }
  await pause(100);
}
const owner = native();
save('listener-preflight.json', { owner, listenerRows, tcpRowCount });
if (!listeners.length) {
  try { save('debug-http-preflight.json', await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(2000) }).then(r => r.json())); }
  catch (error) { save('debug-http-preflight.json', { error: String(error), cause: String(error.cause) }); }
}
assert.equal(new Set(listeners).size, 1, 'exactly one owner for the debug listener');
const listener = listeners[0];
assert(owner.processes.some(row => row.pid === listener), 'debug port belongs to this exact launch');
const targets = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`, { signal: AbortSignal.timeout(10000) }).then(r => r.json());
save('debug-targets.json', targets);
const page = targets.find(target => target.type === 'page' && /^https?:\/\/tauri\.localhost\//.test(target.url) && !target.url.includes('hud.html'));
assert(page, 'packaged main page required, no dev-server fallback');
const ws = new URL(page.webSocketDebuggerUrl);
assert.equal(ws.hostname, '127.0.0.1'); assert.equal(ws.port, String(launch.debug_port));
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept, reject) => { socket.onopen = accept; socket.onerror = reject; });
let sequence = 0;
const pending = new Map(), ipc = [], requests = new Map();
let expectedOptions = null, scopeFailure = null;
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.method) { event(message); return; }
  const waiter = pending.get(message.id);
  if (!waiter) return;
  pending.delete(message.id); clearTimeout(waiter.timer);
  if (message.error) waiter.reject(Error(message.error.message)); else waiter.accept(message.result);
};
function send(method, params = {}) {
  return new Promise((accept, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { accept, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(expression, timeout = 30000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { if (await evaluate(expression)) return; await pause(100); }
  throw Error(`UI condition timed out: ${expression}`);
}
async function click(expression) {
  const point = await evaluate(`(() => {const e=${expression}; if(!e||e.disabled)throw Error('control missing or disabled');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, ...point, button: 'left', clickCount: 1 });
}
const button = label => `[...document.querySelectorAll('button')].find(e=>e.textContent.trim()===${JSON.stringify(label)})`;
const messages = Object.fromEntries(['en', 'zh-CN'].map(locale => [locale, JSON.parse(readFileSync(join(repo, `resources/i18n/${locale}.json`), 'utf8')).messages]));
const label = (locale, key) => messages[locale][key].forms.other;
const geometry = `(() => {const rect=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return{tag:e.tagName,text:e.textContent?.trim().slice(0,150),class:e.className,rect:{x:r.x,y:r.y,width:r.width,height:r.height},font:s.fontFamily,fontSize:s.fontSize,color:s.color,background:s.backgroundColor,border:s.borderColor}};return{width:innerWidth,height:innerHeight,dpr:devicePixelRatio,visibility:document.visibilityState,route:location.hash,language:document.documentElement.lang,body:document.body.innerText,overflow:document.documentElement.scrollWidth>innerWidth,rootStyle:rect(document.documentElement),regions:[...document.querySelectorAll('.app-shell,.app-titlebar,.capsule-nav,.stage,.stage-visual,.stage-content,.scope-controls,.scan-roots,.settings-page,fieldset,.clean-workbench-heading,.clean-scope-panel')].map(rect),inputs:[...document.querySelectorAll('input')].map(e=>({...rect(e),type:e.type,explicitType:e.getAttribute('type'),value:e.value,checked:e.checked,disabled:e.disabled})),buttons:[...document.querySelectorAll('button')].map(e=>({...rect(e),disabled:e.disabled})),scripts:[...document.scripts].map(s=>s.src),resources:performance.getEntriesByType('resource').map(r=>r.name)}})()`;
async function capture(name) {
  await pause(200);
  const pageGeometry = await evaluate(geometry);
  save(`${name}.json`, { native: native(), page: pageGeometry });
  const image = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(output, `${name}.png`), Buffer.from(image.data, 'base64'), { flag: 'wx' });
  assert.equal(pageGeometry.overflow, false, `${name}: page must not overflow horizontally`);
}
async function resize(width, height) {
  let w = width, h = height;
  for (let attempt = 0; attempt < 3; attempt++) {
    native(w, h); await pause(100);
    const current = await evaluate('({width:innerWidth,height:innerHeight})');
    if (current.width === width && current.height === height) return;
    w += width - current.width; h += height - current.height;
  }
  assert.deepEqual(await evaluate('({width:innerWidth,height:innerHeight})'), { width, height });
}
function event({ method, params }) {
  if (method === 'Network.requestWillBeSent' && params.request.method === 'POST' && new URL(params.request.url).hostname === 'ipc.localhost') {
    const command = new URL(params.request.url).pathname.slice(1);
    if (!['scan_start', 'scan_cancel', 'plan_dry_run', 'plan_execute'].includes(command)) return;
    const record = { command, request: JSON.parse(params.request.postData), requestId: params.requestId };
    ipc.push(record); requests.set(params.requestId, record);
    if (command === 'plan_execute') scopeFailure = 'Unexpected cleanup execute request';
    if (command === 'scan_start') {
      try { assert.deepEqual(record.request.options, expectedOptions); }
      catch (error) {
        scopeFailure = String(error); record.scope_failure = scopeFailure;
        // Cancel through the current real UI; never substitute a bridge result.
        void click(`[...document.querySelectorAll('.clean-scope-panel button')].find(e=>['Cancel','取消'].includes(e.textContent.trim())&&!e.disabled)`).catch(error => { record.cancel_error = String(error); });
      }
    }
  } else if (method === 'Network.loadingFinished' && requests.has(params.requestId)) {
    const record = requests.get(params.requestId);
    void send('Network.getResponseBody', { requestId: params.requestId }).then(response => {
      const body = response.base64Encoded ? Buffer.from(response.body, 'base64').toString('utf8') : response.body;
      record.response = JSON.parse(body);
    }).catch(error => { record.error = String(error); });
  } else if (method === 'Network.loadingFailed' && requests.has(params.requestId)) requests.get(params.requestId).error = params.errorText;
}
async function waitIpc(index, command) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    const record = ipc.slice(index).find(record => record.command === command);
    if (record?.response || record?.error) { assert.equal(record.error, undefined); assert.equal(scopeFailure, null); return record; }
    await pause(100);
  }
  throw Error(`No actual IPC result observed for ${command}`);
}
function cli(argv, name) {
  assert(!argv.includes('execute') && !argv.includes('--confirm'));
  const result = spawnSync(fixture.cli, argv, { cwd: fixture.project, env: actualEnv, windowsHide: true, timeout: 60000, encoding: 'utf8' });
  writeFileSync(join(output, `${name}.stdout.txt`), result.stdout ?? '', { flag: 'wx' });
  writeFileSync(join(output, `${name}.stderr.txt`), result.stderr ?? '', { flag: 'wx' });
  save(`${name}.command.json`, { argv, cwd: fixture.project, exit_code: result.status, error: result.error ? String(result.error) : null });
  assert.equal(result.status, 0, result.stderr);
  return argv.includes('--format') ? JSON.parse(result.stdout) : result.stdout;
}
const completed = [];
async function scanAndPreview(locale, scope) {
  const name = `${locale}-${scope}`, project = scope === 'project';
  expectedOptions = { include_projects: project, include_global: !project, roots: project ? [fixture.project.replaceAll('\\', '/').toLowerCase()] : [] };
  const start = ipc.length;
  await click(button(label(locale, `clean.v2.${scope}_scan`)));
  const scan = await waitIpc(start, 'scan_start');
  save(`${name}-scan-ipc.json`, scan);
  assert.equal(scan.response.type, 'completed');
  const report = scan.response.report;
  assert(report.plan.targets.every(target => target.scope.type === (project ? 'project' : 'global')));
  if (project) assert(report.plan.targets.every(target => !target.selected_by_default));
  const target = report.plan.targets.find(target => target.rule_id === (project ? 'node.next_cache' : 'npm.cache.clean'));
  assert(target, 'real supported target required');
  await waitFor(`!!${button(label(locale, `clean.v2.${scope}_review`))}`);
  await click(button(label(locale, `clean.v2.${scope}_review`)));
  const selector = `[...document.querySelectorAll('input[type="checkbox"]')].find(e=>e.getAttribute('aria-label')===${JSON.stringify(`Select ${target.path ?? target.id}`)})`;
  await waitFor(`!!${selector}`);
  assert.equal(await evaluate(`${selector}.disabled`), false);
  assert.equal(await evaluate(`${selector}.checked`), false, 'project artifacts and irreversible owner commands are opt-in');
  await click(selector);
  const previewStart = ipc.length;
  await click(button(label(locale, 'clean.v1.action.preview')));
  const preview = await waitIpc(previewStart, 'plan_dry_run');
  save(`${name}-preview-ipc.json`, preview);
  assert.deepEqual(preview.request.plan, report.plan);
  assert.deepEqual(preview.request.selectedIds, [target.id]);
  assert.match(preview.response.digest, /^sha256:[0-9a-f]{64}$/);
  assert.equal(preview.response.report.dry_run, true);
  assert.equal(preview.response.report.selected, 1);
  assert.equal(preview.response.report.attempted, 1);
  assert.equal(preview.response.report.succeeded, 0);
  assert.equal(preview.response.report.skipped, 1);
  assert.equal(preview.response.report.failed, 0);
  assert.deepEqual(preview.response.report.outcomes[0].status, { type: 'skipped', reason: 'dry run; no cleanup action executed' });
  assert.deepEqual(preview.response.report.outcomes.map(row => row.target_id), [target.id]);
  assert.equal(await evaluate('!!document.querySelector("dialog[open]")'), false, 'successful preview is not cleanup confirmation');
  await capture(`${name}-successful-preview`);
  const scanArgs = ['clean', 'scan', '--scope', project ? 'projects' : 'global', '--format', 'json'];
  if (project) scanArgs.push('--root', expectedOptions.roots[0]);
  const counterpart = cli(scanArgs, `${name}-cli-scan`);
  assert.deepEqual(counterpart.data, report, 'full CLI/native report parity under the same isolated effective environment');
  const planPath = join(output, `${name}-cli-plan.json`);
  cli(['clean', 'plan', '--observation', join(output, `${name}-cli-scan.stdout.txt`), '--select', target.id, '--output', planPath], `${name}-cli-plan`);
  const cliPreview = cli(['clean', 'preview', '--plan', planPath, '--format', 'json'], `${name}-cli-preview`);
  cli(['--language', locale, 'clean', 'preview', '--plan', planPath], `${name}-cli-preview-human`);
  assert.equal(cliPreview.data.digest, preview.response.digest, 'same live provider/project authority binds the same preview digest');
  assert.deepEqual(cliPreview.data.report, preview.response.report);
  completed.push({ locale, scope, target_id: target.id, digest: preview.response.digest, full_report_parity: true, preview_parity: true });
  await click(button(label(locale, 'clean.v1.action.back_review')));
  await click(button(label(locale, 'clean.v2.back')));
}
try {
  await send('Page.enable'); await send('Network.enable');
  await waitFor("!!document.querySelector('.clean-workbench') && !!document.querySelector('.scan-roots input')");
  save('identity.json', { launch, owner, listener_pid: listener, browser: await send('Browser.getVersion'), fixture_manifest_sha256: hash(fixtureArg), binary_sha256: hash(launch.executable), actual_environment: actualEnv, method: 'Native packaged WebView at existing OS DPI; exact owned PID/creation/debug port; actual UI and IPC, no replacement bridge; scan/plan/preview only.' });
  save('initial-dom.json', await evaluate(geometry));
  assert.equal(await evaluate("document.querySelectorAll('.clean-workbench .planet').length"), 0);
  assert.equal(await evaluate("document.querySelectorAll('.scan-roots > ul > li').length"), 0);
  assert.equal(await evaluate(`${button(label('en', 'clean.v2.global_scan'))}.disabled`), false);
  assert.equal(await evaluate(`${button(label('en', 'clean.v2.project_scan'))}.disabled`), true);
  for (const [width, height] of [[900, 600], [1080, 720], [1350, 900]]) { await resize(width, height); await capture(`en-clean-${width}x${height}`); }
  await resize(1080, 720);
  for (const route of ['settings', 'optimize']) {
    await click(`document.getElementById('primary-tab-${route}')`);
    await waitFor(`location.hash==='#/${route}'`); await capture(`en-${route}-1080x720`);
  }
  await click("document.getElementById('primary-tab-clean')");
  await scanAndPreview('en', 'global');
  await click("document.querySelector('.scan-roots input')");
  await send('Input.insertText', { text: fixture.project });
  await click("document.querySelector('.scan-root-entry button')");
  await waitFor("document.querySelectorAll('.scan-roots > ul > li').length===1 && document.querySelector('.scan-roots input').value==='' && !document.querySelector('.scan-roots').disabled");
  assert.deepEqual(await evaluate("[...document.querySelectorAll('.scan-roots > ul > li > span')].map(e=>e.textContent)"), [fixture.project.replaceAll('\\', '/').toLowerCase()]);
  await scanAndPreview('en', 'project');
  await capture('en-both-scopes-retained');
  await click("document.getElementById('primary-tab-settings')");
  await waitFor("!!document.querySelector('.settings-page')");
  save('settings-before-locale-dom.json', await evaluate(geometry));
  await click("[...document.querySelectorAll('[role=combobox]')].find(e=>e.textContent.includes('English'))");
  await waitFor("[...document.querySelectorAll('[role=option]')].some(e=>e.textContent.includes('Simplified Chinese'))");
  await click("[...document.querySelectorAll('[role=option]')].find(e=>e.textContent.includes('Simplified Chinese'))");
  await waitFor("document.getElementById('primary-tab-clean').textContent.trim()==='清理'");
  await capture('zh-CN-settings-1080x720');
  await click("document.getElementById('primary-tab-clean')");
  for (const scope of ['global', 'project']) assert.equal(await evaluate(`!!${button(label('zh-CN', `clean.v2.${scope}_review`))}`), true, 'both reports survive route and locale change');
  await capture('zh-CN-retained-after-settings');
  await scanAndPreview('zh-CN', 'global'); await scanAndPreview('zh-CN', 'project');
  for (const [width, height] of [[900, 600], [1080, 720], [1350, 900]]) { await resize(width, height); await capture(`zh-CN-clean-${width}x${height}`); }
  save('accessibility.json', await send('Accessibility.getFullAXTree'));
  assert.equal(ipc.some(record => record.command === 'plan_execute'), false);
  for (const sentinel of fixture.sentinels) assert.equal(hash(sentinel.path), sentinel.sha256);
  save('summary.json', { pass: true, completed, scope_and_root_submission_verified: true, no_execute_request: true, sentinels_unchanged: true, locale_route_retention: true, actual_windows_dpi: true, taskbar_tray: 'NOT_RUN', installer: 'NOT_RUN', idle_cpu: 'NOT_RUN: interaction sampling is diagnostic only' });
  console.log(JSON.stringify({ pass: true, previews: completed.length, output }));
} catch (error) {
  save('failure.json', { message: String(error), stack: error.stack, scopeFailure, completed });
  try { await capture('failure'); } catch { /* keep primary failure */ }
  throw error;
} finally {
  save('ipc.json', ipc); socket.close();
}
