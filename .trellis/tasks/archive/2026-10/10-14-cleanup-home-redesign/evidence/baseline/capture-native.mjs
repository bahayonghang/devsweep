// Read-only pre-edit geometry capture. Never click a scan or mutate preferences.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
const [launchPath, outputPath] = process.argv.slice(2);
const launch = JSON.parse(readFileSync(launchPath, 'utf8'));
const output = resolve(outputPath), repo = process.cwd();
mkdirSync(output, { recursive: false });
const save = (name, value) => writeFileSync(join(output, name), JSON.stringify(value, null, 2), { flag: 'wx' });
const pause = (ms) => new Promise(r => setTimeout(r, ms));
function native(width = 0, height = 0) {
  return JSON.parse(execFileSync('python', ['-c', `
import ctypes as ct,ctypes.wintypes as wt,importlib.util,json,sys
spec=importlib.util.spec_from_file_location('owned',sys.argv[1]);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
s=module.WindowsSampler();pid=int(sys.argv[2]);created=int(sys.argv[3]);width=int(sys.argv[4]);height=int(sys.argv[5])
assert s.inspect({'pid':pid}).get('created')==created,'owned process identity changed'
windows=s.windows(pid,created)
user=ct.WinDLL('user32');user.SetProcessDpiAwarenessContext.argtypes=[ct.c_void_p];user.SetProcessDpiAwarenessContext(ct.c_void_p(-4))
user.GetDpiForWindow.argtypes=[wt.HWND];user.GetDpiForWindow.restype=wt.UINT
user.GetWindowRect.argtypes=[wt.HWND,ct.POINTER(wt.RECT)];user.GetClientRect.argtypes=[wt.HWND,ct.POINTER(wt.RECT)]
user.SetWindowPos.argtypes=[wt.HWND,wt.HWND,ct.c_int,ct.c_int,ct.c_int,ct.c_int,wt.UINT];user.SetWindowPos.restype=wt.BOOL
for w in windows:
 w['dpi']=user.GetDpiForWindow(w['handle'])
 if w['class']=='Tauri Window' and w['title']=='DevSweep' and width:
  wr=wt.RECT();cr=wt.RECT();user.GetWindowRect(w['handle'],ct.byref(wr));user.GetClientRect(w['handle'],ct.byref(cr))
  assert user.SetWindowPos(w['handle'],None,0,0,int(width*w['dpi']/96)+(wr.right-wr.left)-(cr.right-cr.left),int(height*w['dpi']/96)+(wr.bottom-wr.top)-(cr.bottom-cr.top),0x16)
rows=s.tree(pid,{pid:created})
for row in rows:
 if 'created' in row:row['created']=str(row['created'])
print(json.dumps({'windows':s.windows(pid,created),'before':windows,'processes':rows}))
`, join(repo,'tools/measure-readonly.py'), String(launch.pid), launch.created, String(width), String(height)], { encoding: 'utf8', windowsHide: true }));
}
const owner = native();
const listener = Number(execFileSync('powershell.exe',['-NoProfile','-Command',`Get-NetTCPConnection -State Listen -LocalPort ${launch.debug_port} -ErrorAction Stop | Select-Object -ExpandProperty OwningProcess -Unique`],{encoding:'utf8',windowsHide:true}).trim());
assert(owner.processes.some(row => row.pid === listener), 'debug port must belong to this launch');
const targets = await fetch(`http://127.0.0.1:${launch.debug_port}/json/list`).then(r=>r.json());
const page = targets.find(target => target.type==='page' && /^https?:\/\/tauri\.localhost\//.test(target.url) && !target.url.includes('hud.html'));
assert(page, 'packaged Tauri page required');
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((accept,reject)=>{socket.onopen=accept;socket.onerror=reject;});
let id=0;const pending=new Map();
socket.onmessage=({data})=>{const message=JSON.parse(data),waiter=pending.get(message.id);if(!waiter)return;pending.delete(message.id);clearTimeout(waiter.timer);if(message.error)waiter.reject(Error(message.error.message));else waiter.accept(message.result);};
function send(method,params={}) {return new Promise((accept,reject)=>{const key=++id;const timer=setTimeout(()=>{pending.delete(key);reject(Error(`CDP timeout ${method}`));},15000);pending.set(key,{accept,reject,timer});socket.send(JSON.stringify({id:key,method,params}));});}
async function evaluate(expression){const result=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(result.exceptionDetails)throw Error(JSON.stringify(result.exceptionDetails));return result.result.value;}
const geometry = `(() => {const rect=e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return{tag:e.tagName,text:e.textContent?.trim().slice(0,150),class:e.className,rect:{x:r.x,y:r.y,width:r.width,height:r.height},font:s.fontFamily,fontSize:s.fontSize,color:s.color,background:s.backgroundColor,border:s.borderColor}};return{width:innerWidth,height:innerHeight,dpr:devicePixelRatio,visibility:document.visibilityState,route:location.hash,language:document.documentElement.lang,body:document.body.innerText,overflow:document.documentElement.scrollWidth>innerWidth,rootStyle:rect(document.documentElement),regions:[...document.querySelectorAll('.app-shell,.app-titlebar,.capsule-nav,.stage,.stage-visual,.stage-content,.scope-controls,.scan-roots,.settings-page,fieldset')].map(rect),inputs:[...document.querySelectorAll('input')].map(e=>({...rect(e),type:e.type,explicitType:e.getAttribute('type'),value:e.value,checked:e.checked,disabled:e.disabled})),buttons:[...document.querySelectorAll('button')].map(e=>({...rect(e),disabled:e.disabled})),scripts:[...document.scripts].map(s=>s.src),resources:performance.getEntriesByType('resource').map(r=>r.name)}})()`;
async function capture(name){await pause(250);save(`${name}.json`,{native:native(),page:await evaluate(geometry)});const shot=await send('Page.captureScreenshot',{format:'png'});writeFileSync(join(output,`${name}.png`),Buffer.from(shot.data,'base64'),{flag:'wx'});}
try {
  await send('Page.enable');
  for(let attempt=0;attempt<100;attempt++){if(await evaluate("!!document.querySelector('#primary-tab-clean')"))break;await pause(100);}
  save('identity.json',{launch,owner,browser:await send('Browser.getVersion'),method:'Owned native production WebView; actual window resize at existing Windows DPI; isolated settings; no scan, cleanup, or preference writes.'});
  await capture('clean-initial');
  for(const [width,height] of [[900,600],[1080,720],[1350,900]]){native(width,height);await capture(`clean-${width}x${height}`);}
  native(1080,720);
  for(const route of ['settings','optimize']) {
    await evaluate(`document.getElementById('primary-tab-${route}').click()`);
    await capture(`${route}-1080x720`);
  }
  await evaluate("document.getElementById('primary-tab-clean').click()");
  await capture('clean-after-route-return');
  save('no-scan-assertion.json',await evaluate("({scope:[...document.querySelectorAll('.scope-controls input')].map(e=>({label:e.parentElement.textContent.trim(),checked:e.checked})),roots:[...document.querySelectorAll('.scan-roots input')].map(e=>e.value),scanDisabled:[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Scan')?.disabled})"));
  save('accessibility.json',await send('Accessibility.getFullAXTree'));
  console.log(JSON.stringify({captured:7,output,native_geometry:true}));
} finally {socket.close();}
