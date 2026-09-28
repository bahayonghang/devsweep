import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { connect, pageFacts } from './cdp.mjs';
const [exeArg, outArg, portArg='9357', reuse='false', seed='defaults'] = process.argv.slice(2);
if (!exeArg || !outArg) throw new Error('Usage: native-launch.mjs exe out port [reuse] [defaults|v1]');
if (!['defaults', 'v1'].includes(seed)) throw new Error('Unknown isolated profile seed');
const exe=resolve(exeArg), out=resolve(outArg), profile=join(out,'localappdata'), port=Number(portArg);
if (existsSync(profile) && reuse !== 'true') throw new Error('Refusing to overwrite an existing profile');
try { await fetch(`http://127.0.0.1:${port}/json/list`); throw new Error('CDP port already responds'); } catch (error) { if (!error.cause) throw error; }
mkdirSync(join(profile,'DevSweep','settings'),{recursive:true});
if (reuse !== 'true') writeFileSync(join(profile,'DevSweep','settings','presentation-v1.json'),JSON.stringify({schema_version:1,language:'en'}));
if (reuse !== 'true' && seed === 'v1') {
  // V1 fields and valid values come from baseline fea6368.
  writeFileSync(join(profile,'DevSweep','settings','desktop-preferences-v1.json'), JSON.stringify({schema_version:1,theme:'light',font_family:'segoe_ui',text_scale_percent:110,motion:'system',planet_fps:30,status_interval_seconds:2,status_process_limit:15,hud_interval_seconds:2}));
}
const app=spawn(exe,[],{env:{...process.env,LOCALAPPDATA:profile,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port} --remote-allow-origins=*`},stdio:'ignore',detached:true,windowsHide:true});
app.unref();
const session={exe,sha256:createHash('sha256').update(readFileSync(exe)).digest('hex'),pid:app.pid,port,profile,reuse,seed,startedAt:new Date().toISOString()};
writeFileSync(join(out,'session.json'),JSON.stringify(session,null,2));
const page=await connect(port,url=>/tauri\.localhost/.test(url)&&!/hud\.html/.test(url));
session.target=page.page; session.initial=await page.evaluate(pageFacts);
writeFileSync(join(out,'session.json'),JSON.stringify(session,null,2));
page.close();
console.log(JSON.stringify(session));
