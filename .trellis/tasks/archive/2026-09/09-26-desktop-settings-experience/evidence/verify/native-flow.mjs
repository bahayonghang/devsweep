import {existsSync,readFileSync,writeFileSync,chmodSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,join,relative} from 'node:path';
import {connect,pause} from './cdp.mjs';
import {settings,openChoice,closePopup,options,typeQuery,chooseTheme,facts,assertFacts,quote} from './settings-ui.mjs';
const [portArg,outArg,phase='first']=process.argv.slice(2);
if(!portArg||!outArg)throw new Error('Usage: native-flow.mjs port out first|restart');
const out=resolve(outArg),session=JSON.parse(readFileSync(join(out,'session.json'),'utf8'));
if(!relative(out,session.profile)||relative(out,session.profile).startsWith('..'))throw new Error('Expected isolated child profile');
const prefs=join(session.profile,'DevSweep','settings'),v1=join(prefs,'desktop-preferences-v1.json'),v2=join(prefs,'desktop-preferences-v2.json'),lang=join(prefs,'presentation-v1.json');
const hash=p=>existsSync(p)?createHash('sha256').update(readFileSync(p)).digest('hex'):null;
const saved=()=>existsSync(v2)?JSON.parse(readFileSync(v2,'utf8')):null;
const report={phase,startedAt:new Date().toISOString(),session,checks:[]};
const record=(name,data={})=>{report.checks.push({name,...data});writeFileSync(join(out,`native-flow-${phase}.json`),JSON.stringify(report,null,2));console.log(name);};
const page=await connect(Number(portArg),url=>/tauri\.localhost/.test(url)&&!/hud\.html/.test(url));
try{
  await settings(page);
  await page.waitFor("document.getElementById('settings-font').getAttribute('aria-busy')!=='true'",'native font catalogue',300);
  const initial=await facts(page);assertFacts(initial,'initial');
  record('initial',initial);
  if(phase==='restart'){
    const expected=JSON.parse(readFileSync(join(out,'restart-expected.json'),'utf8'));
    if(JSON.stringify(saved())!==JSON.stringify(expected.preferences)||hash(v1)!==expected.v1||hash(lang)!==expected.language)throw new Error('Restart preferences or preserved bytes mismatch');
    if(initial.theme!==expected.preferences.theme||!initial.font.includes(expected.preferences.font.family))throw new Error('Restart renderer did not apply saved appearance');
    record('restart-applies-committed-theme-font',{preferences:saved(),facts:initial});
  }else{
    if(session.seed!=='v1'||existsSync(v2))throw new Error('Expected readonly V1 migration before first save');
    const original={v1:hash(v1),language:hash(lang)};
    if(initial.theme!=='light'||!initial.font.includes('Segoe UI'))throw new Error('V1 appearance not applied');
    await openChoice(page,'settings-font');
    const catalogue=await options(page);
    if(catalogue.length<=4)throw new Error('Native catalogue did not exceed old presets');
    record('native-catalogue',{count:catalogue.length-1,displayNames:catalogue.map(x=>x.text),userAgent:await page.evaluate('navigator.userAgent')});
    await typeQuery(page,'DevSweepNoSuchFamilyQZX');
    if((await options(page)).length!==0)throw new Error('Unknown font query had options');
    await closePopup(page);
    if(existsSync(v2)||hash(v1)!==original.v1||hash(lang)!==original.language)throw new Error('Read/query wrote preference files');
    record('read-query-escape-do-not-write',original);
    const selected=[];
    for(const index of [1,Math.floor(catalogue.length/2),catalogue.length-1]){
      const label=catalogue[index].text;
      await typeQuery(page,label);
      const matches=await options(page),match=matches.find(x=>x.text===label);
      if(!match)throw new Error(`Native font not searchable: ${label}`);
      await page.click('[id='+quote(match.id)+']');
      await page.waitFor("!document.querySelector('.settings-choice-popup[data-open]') && !document.getElementById('settings-font').disabled",'font saved');
      for(let n=0;n<50&&!saved();n++)await pause(50);
      const committed=saved(),view=await facts(page);
      if(committed?.font?.kind!=='installed'||!view.font.includes(committed.font.family))throw new Error('Chosen font not committed and rendered');
      selected.push({index,label,font:committed.font,css:view.font});
    }
    if(hash(v1)!==original.v1||hash(lang)!==original.language)throw new Error('Migration changed rollback or language bytes');
    record('first-middle-last-selection-v2-preserves-v1-language',{selected,preferences:saved()});
    await page.click('.settings-font-actions button');
    await page.waitFor("!document.querySelector('.settings-font-actions button').disabled",'font refresh');
    await openChoice(page,'settings-font');
    const refreshed=await options(page);
    if(refreshed.length!==catalogue.length)throw new Error('Unexpected catalogue count change on refresh');
    await closePopup(page);record('explicit-refresh',{count:refreshed.length-1});
    await chooseTheme(page,'catppuccin_mocha');
    const before=saved(),beforeHash=hash(v2);
    chmodSync(v2,0o444);
    try{
      await page.click('input[name=settings-theme][value=claude]');
      await page.waitFor("!!document.querySelector('.preferences-warning[role=alert]') && !document.getElementById('settings-font').disabled",'failed save notice');
      const failed=await facts(page);
      if(failed.theme!==before.theme||hash(v2)!==beforeHash)throw new Error('Failed save changed committed appearance');
      record('failed-save-preserves-committed-theme',{theme:failed.theme,errors:failed.errors,unchangedBytes:true});
    }finally{chmodSync(v2,0o666);}
    await chooseTheme(page,'codex');
    writeFileSync(join(out,'restart-expected.json'),JSON.stringify({preferences:saved(),v1:hash(v1),language:hash(lang)},null,2));
    record('ready-for-restart',{preferences:saved()});
  }
  report.result='PASS';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{report.events=page.events;report.finishedAt=new Date().toISOString();writeFileSync(join(out,`native-flow-${phase}.json`),JSON.stringify(report,null,2));page.close();}
