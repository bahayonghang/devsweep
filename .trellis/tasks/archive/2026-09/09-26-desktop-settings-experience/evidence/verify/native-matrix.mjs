// CSS viewport and media emulation run inside the real owned WebView2 page.
// Emulation does not change Windows display, locale, or accessibility settings.
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {connect,pause} from './cdp.mjs';
import {settings,openChoice,closePopup,chooseIndex,chooseTheme,geometry,facts,assertFacts} from './settings-ui.mjs';
const [portArg,outArg]=process.argv.slice(2);
if(!portArg||!outArg)throw new Error('Usage: native-matrix.mjs port out');
const out=resolve(outArg);mkdirSync(out,{recursive:true});
const page=await connect(Number(portArg),url=>/tauri\.localhost/.test(url)&&!/hud\.html/.test(url));
const report={method:'Owned native WebView2; CDP input, DOM/AX capture, CSS viewport/media emulation',startedAt:new Date().toISOString(),pages:[],popups:[],media:[]};
const write=()=>writeFileSync(join(out,'native-matrix.json'),JSON.stringify(report,null,2));
const capture=async label=>{const value=await facts(page);assertFacts(value,label);report.pages.push({label,...value});write();};
const controls=['settings-font','settings-text-scale','settings-language','settings-motion','settings-planet-fps','settings-status-interval','settings-status-rows','settings-hud-interval'];
const themes=['dark','light','system','catppuccin_latte','catppuccin_mocha','codex','claude'];
const schemes={dark:'dark',light:'light',catppuccin_latte:'light',catppuccin_mocha:'dark',codex:'dark',claude:'light'};
try{
  await settings(page);
  await page.send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'no-preference'}]});
  for(const [locale,localeIndex] of [['en',0],['zh-CN',1]]){
    await chooseIndex(page,'settings-language',localeIndex);
    await page.waitFor(`document.querySelector('.app-shell').dataset.locale===${JSON.stringify(locale)}`,'locale commit');
    await chooseTheme(page,locale==='en'?'catppuccin_latte':'catppuccin_mocha');
    for(const [scale,scaleIndex] of [[100,0],[125,2]]){
      await chooseIndex(page,'settings-text-scale',scaleIndex);
      for(const width of [390,800,1024,1440]){
        await page.send('Emulation.setDeviceMetricsOverride',{width,height:900,deviceScaleFactor:1,mobile:false});
        await pause(80);
        await capture(`${locale}-${scale}-${width}`);
        for(const control of controls){
          await openChoice(page,control);
          const g=await geometry(page);
          report.popups.push({locale,scale,width,control,...g});write();
          if(!g?.within||!g.portal||g.options<1)throw new Error(`Popup geometry failed: ${locale}-${scale}-${width}-${control}`);
          if(g.font!==g.bodyFont)throw new Error(`Popup font differs from UI: ${control}`);
          await closePopup(page);
        }
        console.log(`matrix ${locale} ${scale}% ${width}px complete`);
      }
    }
    await page.send('Emulation.setDeviceMetricsOverride',{width:1024,height:900,deviceScaleFactor:1,mobile:false});
    for(const [scale,scaleIndex] of [[110,1],[125,2]]){
      await chooseIndex(page,'settings-text-scale',scaleIndex);
      for(const theme of themes){
        await chooseTheme(page,theme);await capture(`${locale}-${theme}-${scale}`);
        await openChoice(page,'settings-text-scale');
        const g=await geometry(page);report.popups.push({locale,scale,width:1024,theme,control:'settings-text-scale',...g});
        if(!g?.within||!g.portal)throw new Error(`Theme popup geometry failed: ${theme}`);
        await closePopup(page);
      }
    }
    await page.send('Accessibility.enable');
    const tree=await page.send('Accessibility.getFullAXTree');
    writeFileSync(join(out,`settings-accessibility-${locale}.json`),JSON.stringify(tree,null,2));
  }
  for(const theme of themes){
    await chooseTheme(page,theme);
    for(const scheme of ['light','dark']){
      await page.send('Emulation.setEmulatedMedia',{features:[{name:'prefers-color-scheme',value:scheme}]});await pause(100);
      const value=await facts(page);report.media.push({kind:'scheme',chosen:theme,emulated:scheme,...value});
      if(value.theme!==(theme==='system'?scheme:theme)||value.scheme!==(theme==='system'?scheme:schemes[theme]))throw new Error(`Incorrect system palette resolution: ${theme}/${scheme}`);
    }
    await page.send('Emulation.setEmulatedMedia',{features:[{name:'forced-colors',value:'active'}]});await pause(100);
    await openChoice(page,'settings-text-scale');
    const forced=await geometry(page);
    const previousForced=report.media.find(row=>row.kind==='forced-colors');
    if(!await page.evaluate("matchMedia('(forced-colors: active)').matches"))throw new Error('Forced colors media not active');
    if(previousForced&&(forced.background!==previousForced.geometry.background||forced.color!==previousForced.geometry.color))throw new Error(`Palette overrides forced colors: ${theme}`);
    report.media.push({kind:'forced-colors',theme,geometry:forced,facts:await facts(page)});
    await closePopup(page);
    await page.send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});await pause(100);
    if(!await page.evaluate("document.getElementById('settings-planet-fps').disabled"))throw new Error(`Reduced-motion frame control still enabled: ${theme}`);
    const reduced=await facts(page);
    if(reduced.motion!=='reduced')throw new Error(`Reduced motion root not applied: ${theme}`);
    report.media.push({kind:'reduced-motion',theme,facts:reduced});
    await page.send('Emulation.setEmulatedMedia',{features:[]});
  }
  report.result='PASS';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{
  await closePopup(page).catch(()=>{});
  await page.send('Emulation.setEmulatedMedia',{features:[]}).catch(()=>{});
  await page.send('Emulation.clearDeviceMetricsOverride').catch(()=>{});
  report.events=page.events;report.finishedAt=new Date().toISOString();write();page.close();
}
