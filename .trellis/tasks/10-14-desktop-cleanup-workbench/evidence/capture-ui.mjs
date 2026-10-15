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
const {createServer: createViteServer} = await import('../../../../desktop/node_modules/vite/dist/node/index.js');
const server = await createViteServer({root:resolve('desktop'),mode:'fixture',server:{host:'127.0.0.1',port:0,strictPort:false}});
await server.listen();
const port=server.httpServer.address().port;
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
  await command("Page.navigate", {url:`http://127.0.0.1:${port}/`});
  for(let i=0;i<100 && !await evaluate("Boolean(document.querySelector('.clean-workbench'))");i++) await sleep(100);
  if (!await evaluate("Boolean(document.querySelector('.clean-workbench'))")) throw new Error("Fixture workbench failed to load");
  async function capture(name,width,height=900){
    await command("Emulation.setDeviceMetricsOverride",{width,height,deviceScaleFactor:1,mobile:false});
    await sleep(150);
    await evaluate("window.scrollTo(0,0)");
    const geometry=await evaluate(`({overflow:document.documentElement.scrollWidth>innerWidth,width:innerWidth,height:innerHeight,theme:document.documentElement.dataset.theme,rootFont:getComputedStyle(document.documentElement).fontSize,headingFont:document.querySelector('.clean-workbench h1')?getComputedStyle(document.querySelector('.clean-workbench h1')).fontSize:null,scale:document.documentElement.style.getPropertyValue('--text-scale'),buttons:[...document.querySelectorAll('.clean-scope-actions>.primary-button')].map(e=>({text:e.textContent,y:e.getBoundingClientRect().y,bottom:e.getBoundingClientRect().bottom,disabled:e.disabled})),panels:[...document.querySelectorAll('.clean-scope-panel')].map(e=>({label:e.getAttribute('aria-labelledby'),x:e.getBoundingClientRect().x,y:e.getBoundingClientRect().y,width:e.getBoundingClientRect().width,height:e.getBoundingClientRect().height})),inputs:[...document.querySelectorAll('.clean-workbench input')].map(e=>({type:e.type,height:e.getBoundingClientRect().height})),text:document.querySelector('main')?.innerText??document.body.innerText})`);
    const png=await command("Page.captureScreenshot",{format:"png"});
    await writeFile(join(out,name+".png"),Buffer.from(png.data,"base64")); results.push({name,...geometry});
  }
  const until = async expression => { for(let i=0;i<100;i++){if(await evaluate(expression)) return;await sleep(50);} throw Error("Condition timeout: "+expression); };
  const click = async selector => { await until(`Boolean(document.querySelector(${JSON.stringify(selector)}))`); await evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`); await sleep(100); };
  const settings = async () => { await evaluate(`document.querySelectorAll('[role="tab"]')[5].click()`); await until("Boolean(document.getElementById('settings-text-scale'))"); };
  const clean = async () => { await evaluate(`document.querySelectorAll('[role="tab"]')[0].click()`); await until("Boolean(document.querySelector('.clean-workbench'))"); };
  await capture("home-1350",1350);
  await capture("home-1080",1080,720);
  await capture("home-900",900,600);
  for(const width of [320,390,800,1024,1440]) await capture("home-"+width,width);
  await command("Emulation.setDeviceMetricsOverride",{width:1350,height:900,deviceScaleFactor:1,mobile:false});
  await click('.clean-scope-global .primary-button');
  await capture("global-scanning",1350);
  await click('.clean-scope-global .secondary-button');
  await until("!document.querySelector('.clean-scope-global .primary-button').disabled");
  await click('.clean-scope-global .primary-button');
  await until("Boolean(document.querySelector('.clean-scope-global .clean-report-summary'))");
  await capture("global-complete",1350);
  await click('.clean-scope-project .clean-scope-actions .primary-button');
  await until("Boolean(document.querySelector('.clean-scope-project .clean-report-summary'))");
  await until("Boolean(document.querySelector('.clean-owner'))");
  await capture("both-complete",1350);
  await click('.clean-scope-project .clean-report-summary .secondary-button');
  await capture("project-review",1350);
  await click('.clean-workbench .clean-detail-heading button');
  await settings(); await capture("settings-dark",1350);
  await click('#settings-text-scale');
  await evaluate(`[...document.querySelectorAll('[role="option"]')].find(e=>e.textContent.includes('125')).click()`);
  await until("document.documentElement.style.getPropertyValue('--text-scale')==='1.25'");
  await clean(); await capture("scaled-125",1350);
  await settings(); await click('#settings-text-scale');
  await evaluate(`[...document.querySelectorAll('[role="option"]')].find(e=>e.textContent.includes('100')).click()`);
  await until("document.documentElement.style.getPropertyValue('--text-scale')==='1'");
  await clean();
  for (const theme of ['light','catppuccin_latte','catppuccin_mocha','codex','claude']) {
    await settings(); await click('input[name="settings-theme"][value="'+theme+'"]'); await until(`document.documentElement.dataset.theme===${JSON.stringify(theme)}`); await clean(); await capture('theme-'+theme,1350);
  }
  await settings(); await click('input[name="settings-theme"][value="dark"]'); await click('#settings-language');
  await evaluate(`[...document.querySelectorAll('[role="option"]')].find(e=>e.textContent.includes('Simplified Chinese')).click()`);
  await clean(); await until("document.querySelector('.clean-workbench h1').textContent==='开发缓存清理'");
  await capture('zh-1350',1350);await capture('zh-390',390);
  await command('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});await capture('zh-reduced-motion',1350);
  await command('Emulation.setEmulatedMedia',{features:[{name:'forced-colors',value:'active'}]});await capture('zh-forced-colors',1350);
  await writeFile(join(out,'retention.json'),JSON.stringify({bothReportsRetained:await evaluate("document.querySelectorAll('.clean-report-summary').length===2"),selected:await evaluate("document.querySelectorAll('.clean-owner').length")},null,2));
  await writeFile(join(out,"geometry.json"),JSON.stringify(results,null,2));
  console.log(JSON.stringify({captures:results.length,overflows:results.filter(x=>x.overflow).map(x=>x.name)}));
} finally {
  if (socket?.readyState === WebSocket.OPEN && alive()) { await Promise.race([command("Browser.close").catch(() => {}), sleep(1000)]); socket.close(); }
  await server.close();
  if (alive()) await Promise.race([exit, sleep(5000)]);
  const forced = Boolean(alive());
  if (forced) { browser.kill(); await Promise.race([exit, sleep(5000)]); }
  const teardown = { pid: browser.pid, forced, exit_code: browser.exitCode, signal: browser.signalCode, error: browserError?.message, lifecycle_pass: !forced && !browserError && browser.exitCode === 0 };
  await writeFile(join(out, "teardown.json"), JSON.stringify(teardown, null, 2));
  if (!teardown.lifecycle_pass) process.exitCode = 1;
}
