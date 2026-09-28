import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,join} from 'node:path';
import {connect} from './cdp.mjs';
import {settings,closePopup,options,typeQuery,quote} from './settings-ui.mjs';
const [portArg,outArg]=process.argv.slice(2),out=resolve(outArg);
const session=JSON.parse(readFileSync(join(out,'session.json'),'utf8'));
const v2=join(session.profile,'DevSweep','settings','desktop-preferences-v2.json');
const hash=()=>createHash('sha256').update(readFileSync(v2)).digest('hex');
const report={method:'CDP native WebView2 renderer composition; Windows IME candidate-window interaction is not exercised',startedAt:new Date().toISOString(),binarySha256:session.sha256,pid:session.pid};
const page=await connect(Number(portArg),url=>url.includes('tauri.localhost')&&!url.includes('hud.html'));
try{
  await page.evaluate("location.hash='#/clean'");
  await page.waitFor("!document.getElementById('settings-font')",'old Settings unmounted');
  await settings(page);await typeQuery(page,'Segoe UI');
  const selected=(await options(page)).find(x=>x.text==='Segoe UI');
  if(!selected)throw new Error('Expected observed Segoe UI font');
  await page.click('[id='+quote(selected.id)+']');
  await page.waitFor("!document.getElementById('settings-font').disabled && !document.querySelector('.settings-choice-popup[data-open]')",'baseline font committed');
  report.before=hash();report.preferencesBefore=JSON.parse(readFileSync(v2,'utf8'));
  await page.click('#settings-font');
  await page.key('a',65,2);await page.key('Backspace',8);
  await page.evaluate("(() => {const el=document.getElementById('settings-font'),types=['compositionstart','compositionupdate','compositionend','keydown','input'];window.__settingsCompositionProbe={el,types,events:[]};const p=window.__settingsCompositionProbe;p.listener=e=>{if(e.target===el)p.events.push({type:e.type,key:e.key,isComposing:e.isComposing,data:e.data,value:el.value});};for(const t of types)document.addEventListener(t,p.listener,true);})()");
  await page.send('Input.imeSetComposition',{text:'宋',selectionStart:1,selectionEnd:1});
  await page.waitFor("document.getElementById('settings-font').value.includes('宋')",'Chinese composition query');
  const candidates=await options(page);report.candidateCountDuringComposition=candidates.length;
  report.firstCandidateDuringComposition=candidates[0]?.text;
  await page.key('ArrowDown',40);await page.key('Enter',13);
  report.during=hash();
  report.events=await page.evaluate('window.__settingsCompositionProbe.events');
  report.nativeComposingEnter=report.events.some(e=>e.type==='keydown'&&e.key==='Enter'&&e.isComposing===true);
  report.preferencesDuring=JSON.parse(readFileSync(v2,'utf8'));
  if(report.before!==report.during)throw new Error('Composing Enter persisted a font choice');
  // Renderer composition started by CDP needs an explicit IME text commit.
  // A key event alone does not drive the Windows IME candidate window.
  report.commitMethod='CDP Input.insertText after the composing-Enter assertion';
  await page.send('Input.insertText',{text:'宋'});
  await page.waitFor("window.__settingsCompositionProbe.events.some(e=>e.type==='compositionend')",'native composition completed');
  report.events=await page.evaluate('window.__settingsCompositionProbe.events');
  // Localized aliases can match while the current locale shows English names.
  await page.waitFor(`(() => {const count=document.querySelectorAll('.settings-choice-popup[data-open] [role=option]').length;return document.getElementById('settings-font').value==='宋' && count>0 && count<${candidates.length};})()`,'committed Chinese query filtered');
  report.matchesAfterComposition=(await options(page)).map(x=>x.text);
  await page.click('#settings-font');
  await closePopup(page);report.after=hash();
  if(report.before!==report.during||report.before!==report.after)throw new Error('Composition or dismissal persisted a font choice');
  report.result=report.nativeComposingEnter?'PASS':'INCONCLUSIVE';
}catch(error){report.result='FAIL';report.error=String(error.stack??error);throw error;}
finally{
  if(report.result!=='PASS'){
    await page.send('Input.imeSetComposition',{text:'',selectionStart:0,selectionEnd:0}).catch(()=>{});
    await page.click('#settings-font').catch(()=>{});
    await closePopup(page).catch(error=>{report.cleanupError=String(error);});
  }
  report.after=hash();
  await page.evaluate("(() => {const p=window.__settingsCompositionProbe;if(p){for(const t of p.types)document.removeEventListener(t,p.listener,true);delete window.__settingsCompositionProbe;}})()").catch(()=>{});
  report.finishedAt=new Date().toISOString();writeFileSync(join(out,'native-composition.json'),JSON.stringify(report,null,2));page.close();
}
