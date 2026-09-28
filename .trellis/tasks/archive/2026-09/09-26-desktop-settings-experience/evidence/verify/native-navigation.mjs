import {writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {connect,pause} from './cdp.mjs';
import {settings,chooseIndex} from './settings-ui.mjs';
const [portArg,outArg]=process.argv.slice(2);
if(!portArg||!outArg)throw new Error('Usage: native-navigation.mjs port out');
const out=resolve(outArg),page=await connect(Number(portArg),url=>/tauri\.localhost/.test(url)&&!/hud\.html/.test(url));
const report={method:'Owned WebView2 native renderer and CDP keyboard input; no operation started',cases:[]};
const selected=route=>`document.getElementById('primary-tab-${route}')?.getAttribute('aria-selected')==='true'`;
const focused=route=>`document.activeElement?.id==='primary-tab-${route}'`;
const snapshot=()=>page.evaluate(`(() => {const tabs=[...document.querySelectorAll('[role=tab]')],panel=document.getElementById('mode-panel'),focus=document.activeElement,r=focus.getBoundingClientRect();return {hash:location.hash,ids:tabs.map(x=>x.id),labels:tabs.map(x=>x.textContent.trim()),selected:tabs.filter(x=>x.getAttribute('aria-selected')==='true').map(x=>x.id),focused:focus.id,focusVisible:r.x>=0&&r.right<=innerWidth,panelRole:panel.getAttribute('role'),panelLabel:panel.getAttribute('aria-labelledby')};})()`);
try{
  for(const [locale,index] of [['en',0],['zh-CN',1]]){
    await settings(page);await chooseIndex(page,'settings-language',index);
    await page.send('Emulation.setDeviceMetricsOverride',{width:390,height:900,deviceScaleFactor:1,mobile:false});
    await page.click('#primary-tab-clean');await page.waitFor(selected('clean'),'Clean selected');
    const sequence=[];
    for(const route of ['software','optimize','analyze','status','settings','clean']){
      await page.key('ArrowRight',39);await page.waitFor(focused(route),'focus '+route);
      if(!await page.evaluate(selected('clean')))throw new Error('Arrow focus committed a route');
      sequence.push(await snapshot());
    }
    await page.key('End',35);await page.waitFor(focused('settings'),'End Settings');
    await page.key('Enter',13);await page.waitFor(selected('settings'),'Settings activation');
    await pause(80);const state=await snapshot();
    if(state.ids.length!==6||state.ids.at(-1)!=='primary-tab-settings'||state.selected.length!==1||state.panelRole!=='tabpanel'||state.panelLabel!=='primary-tab-settings'||!state.focusVisible)throw new Error('Settings native tab semantics or visibility failed');
    await page.evaluate('history.back()');await page.waitFor(selected('clean'),'history back');
    await page.evaluate('history.forward()');await page.waitFor(selected('settings'),'history forward');
    report.cases.push({locale,width:390,sequence,state,history:await snapshot()});
  }
  report.result='PASS';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{await page.send('Emulation.clearDeviceMetricsOverride').catch(()=>{});report.events=page.events;writeFileSync(join(out,'native-navigation.json'),JSON.stringify(report,null,2));page.close();}
