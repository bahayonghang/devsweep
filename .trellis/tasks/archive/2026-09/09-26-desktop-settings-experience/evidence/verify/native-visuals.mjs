import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {connect,pause} from './cdp.mjs';
import {settings,openChoice,closePopup,chooseIndex,chooseTheme,facts,options,typeQuery,quote,geometry} from './settings-ui.mjs';
const [portArg,outArg]=process.argv.slice(2);
if(!portArg||!outArg)throw new Error('Usage: native-visuals.mjs port out');
const out=resolve(outArg);mkdirSync(out,{recursive:true});
const main=await connect(Number(portArg),url=>url.includes('tauri.localhost')&&!url.includes('hud.html'));
const hud=await connect(Number(portArg),url=>url.includes('tauri.localhost/hud.html'));
const report={startedAt:new Date().toISOString(),method:'Actual isolated release app; HUD opened manually; CDP renderer captures record visibility separately',initial:{},themes:[],screenshots:[],dialogs:[],schemeChecks:[]};
const write=()=>writeFileSync(join(out,'native-visuals.json'),JSON.stringify(report,null,2));
const hudFacts=()=>hud.evaluate('(() => {const r=document.documentElement,s=getComputedStyle(r);return {theme:r.dataset.theme,scheme:s.colorScheme,font:getComputedStyle(document.body).fontFamily,width:innerWidth,height:innerHeight,dpr:devicePixelRatio,visibility:document.visibilityState,focused:document.hasFocus(),text:document.body.innerText,overflow:r.scrollWidth>r.clientWidth+1};})()');
async function capture(page,name){
  const shot=await page.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
  const bytes=Buffer.from(shot.data,'base64');writeFileSync(join(out,name+'.png'),bytes);
  report.screenshots.push({file:name+'.png',bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')});write();
}
async function matching(label){
  const m=await facts(main);
  await hud.waitFor('document.documentElement.dataset.theme==='+quote(m.theme),'HUD theme '+label);
  await hud.waitFor('getComputedStyle(document.body).fontFamily==='+quote(m.bodyFont),'HUD font '+label);
  const h=await hudFacts();
  if(h.overflow||!h.text.trim()||m.scheme!==h.scheme)throw new Error('HUD rendering mismatch '+label);
  return {label,main:m,hud:h};
}
try{
  await settings(main);report.initial=await matching('saved custom font and palette');write();
  await capture(hud,'hud-custom-font');
  await typeQuery(main,'Segoe UI');
  const choices=await options(main),match=choices.find(x=>x.text==='Segoe UI');
  if(!match)throw new Error('The observed Segoe UI family is missing');
  await main.click('[id='+quote(match.id)+']');
  await main.waitFor("!document.getElementById('settings-font').disabled && !document.querySelector('.settings-choice-popup[data-open]')",'Segoe UI committed');
  await chooseIndex(main,'settings-text-scale',1);
  await chooseIndex(main,'settings-language',1);
  await main.send('Emulation.setDeviceMetricsOverride',{width:1024,height:1050,deviceScaleFactor:1,mobile:false});
  for(const theme of ['catppuccin_latte','catppuccin_mocha','codex','claude']){
    await chooseTheme(main,theme);report.themes.push(await matching(theme));
    await openChoice(main,'settings-text-scale');
    await main.evaluate('window.scrollTo(0,0)');await pause(100);
    const popup=await geometry(main);if(!popup?.within)throw new Error('Visual popup out of bounds '+theme);
    await capture(main,'settings-'+theme);await capture(hud,'hud-'+theme);await closePopup(main);
    await main.evaluate("location.hash='#/protection'");
    await main.waitFor("!!document.querySelector('.support-page form input[name=path]')",'Protection form');
    await main.click('.support-page form input[name=path]');
    await main.send('Input.insertText',{text:'D:/DevSweep-native-verification/preview-only'});
    await main.click('.support-page form button[type=submit]');
    await main.waitFor("!!document.querySelector('.support-page dialog[open]')",'Protection confirmation');
    const dialog=await main.evaluate("(() => {const d=document.querySelector('.support-page dialog'),s=getComputedStyle(d),r=getComputedStyle(document.documentElement);const rgb=n=>{const v=r.getPropertyValue(n).trim().replace('#','');return 'rgb('+[0,2,4].map(i=>parseInt(v.slice(i,i+2),16)).join(', ')+')';};return {color:s.color,background:s.backgroundColor,border:s.borderTopColor,expectedText:rgb('--text'),expectedBackground:rgb('--raised'),expectedBorder:rgb('--control-border'),buttons:[...d.querySelectorAll('button')].map(x=>x.textContent)};})()");
    if(dialog.color!==dialog.expectedText||dialog.background!==dialog.expectedBackground||dialog.border!==dialog.expectedBorder)throw new Error('Dialog bypasses theme '+theme);
    report.dialogs.push({theme,...dialog});await capture(main,'dialog-'+theme);
    await main.click('.support-page dialog .secondary-button');
    await main.waitFor("!document.querySelector('.support-page dialog')",'Confirmation canceled');
    await settings(main);write();
  }
  for(const theme of ['dark','light','system','catppuccin_latte','catppuccin_mocha','codex','claude']){
    await chooseTheme(main,theme);
    for(const scheme of ['light','dark']){
      const params={features:[{name:'prefers-color-scheme',value:scheme}]};
      await main.send('Emulation.setEmulatedMedia',params);await hud.send('Emulation.setEmulatedMedia',params);
      await pause(100);const pair=await matching(theme+'/'+scheme);
      if(pair.main.theme!==(theme==='system'?scheme:theme))throw new Error('Incorrect shared System resolution');
      report.schemeChecks.push({chosen:theme,emulated:scheme,...pair});write();
    }
  }
  report.result='PASS';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{
  await closePopup(main).catch(()=>{});
  await main.send('Emulation.clearDeviceMetricsOverride').catch(()=>{});
  await main.send('Emulation.setEmulatedMedia',{features:[]}).catch(()=>{});
  await hud.send('Emulation.setEmulatedMedia',{features:[]}).catch(()=>{});
  report.events={main:main.events,hud:hud.events};report.finishedAt=new Date().toISOString();write();main.close();hud.close();
}
