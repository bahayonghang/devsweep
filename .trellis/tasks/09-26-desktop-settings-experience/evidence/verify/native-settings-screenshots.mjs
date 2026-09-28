import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,join} from 'node:path';
import {connect,pause} from './cdp.mjs';
import {settings,openChoice,closePopup,chooseIndex,chooseTheme,facts,assertFacts,geometry} from './settings-ui.mjs';
const [portArg,outArg]=process.argv.slice(2),out=resolve(outArg);
const session=JSON.parse(readFileSync(join(out,'session.json'),'utf8'));
const report={method:'Final isolated release WebView2 renderer captures; emulated CSS viewport',binarySha256:session.sha256,pid:session.pid,startedAt:new Date().toISOString(),screenshots:[]};
const page=await connect(Number(portArg),url=>url.includes('tauri.localhost')&&!url.includes('hud.html'));
try{
  await settings(page);
  await chooseIndex(page,'settings-language',1);
  await chooseIndex(page,'settings-text-scale',1);
  await page.send('Emulation.setDeviceMetricsOverride',{width:1024,height:1050,deviceScaleFactor:1,mobile:false});
  for(const theme of ['catppuccin_latte','catppuccin_mocha','codex','claude']){
    await chooseTheme(page,theme);
    await openChoice(page,'settings-text-scale');
    await page.evaluate('window.scrollTo(0,0)');
    await pause(100);
    const current=await facts(page),popup=await geometry(page);
    assertFacts(current,theme);
    if(!popup?.within)throw new Error('Popup clipped: '+theme);
    const shot=await page.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
    const bytes=Buffer.from(shot.data,'base64'),file='settings-'+theme+'.png';
    writeFileSync(join(out,file),bytes);
    report.screenshots.push({theme,file,sha256:createHash('sha256').update(bytes).digest('hex'),facts:current,popup});
    await closePopup(page);
  }
  report.result='PASS';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{
  await closePopup(page).catch(()=>{});
  await page.send('Emulation.clearDeviceMetricsOverride').catch(()=>{});
  report.events=page.events;report.finishedAt=new Date().toISOString();
  writeFileSync(join(out,'native-settings-screenshots.json'),JSON.stringify(report,null,2));
  page.close();
}
