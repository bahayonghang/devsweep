// Test-only helpers for the owned WebView2 renderer. All setting writes use UI.
import { pause, pageFacts } from './cdp.mjs';
export const quote = JSON.stringify;
// Base UI Select can retain a hidden, closed portal after dismissal.
export const popup = '.settings-choice-popup[data-open]';
export const option = `${popup} [role=option]`;
export async function settings(page) {
  await page.evaluate("location.hash='#/settings'");
  await page.waitFor("!!document.getElementById('settings-font') && !document.getElementById('settings-font').disabled", 'Settings ready');
}
export async function closePopup(page) {
  if (await page.evaluate(`!!document.querySelector(${quote(popup)})`)) {
    await page.key('Escape', 27);
    await page.waitFor(`!document.querySelector(${quote(popup)})`, 'popup closed');
  }
}
export async function openChoice(page, id) {
  await closePopup(page);
  const selector = id === 'settings-font' ? '.settings-combobox-trigger' : '#' + id;
  await page.click(selector);
  await page.waitFor(`!!document.querySelector(${quote(popup)})`, `${id} popup`);
  await pause(80);
}
export async function options(page) {
  return page.evaluate(`Array.from(document.querySelectorAll(${quote(option)}), x => { const label=x.cloneNode(true); label.querySelectorAll('[aria-hidden=true]').forEach(el=>el.remove()); return {id:x.id,text:label.textContent.trim(),selected:x.getAttribute('aria-selected'),disabled:x.getAttribute('aria-disabled')}; })`);
}
export async function chooseIndex(page, id, index) {
  await openChoice(page, id);
  const choices = await options(page);
  if (!choices[index]) throw new Error(`Missing option ${index} for ${id}`);
  await page.click(option, index);
  await page.waitFor(`!document.querySelector(${quote(popup)}) && !document.getElementById(${quote(id)}).disabled`, `${id} committed`);
  await pause(60);
  return choices[index].text;
}
export async function chooseTheme(page, theme) {
  await closePopup(page);
  await page.click('input[name=settings-theme][value=' + quote(theme) + ']');
  await page.waitFor(`(() => {const x=document.querySelector('input[name=settings-theme][value='+${quote(quote(theme))}+']');return x?.checked && !x.disabled;})()`, `${theme} committed`);
}
export async function typeQuery(page, query) {
  await page.click('#settings-font');
  await page.key('a', 65, 2);
  await page.send('Input.insertText', {text: query});
  await page.waitFor(`document.getElementById('settings-font').value===${quote(query)}`, 'font query');
  await pause(80);
}
export async function facts(page) {
  return page.evaluate(pageFacts);
}
export async function geometry(page) {
  return page.evaluate(`(() => {
    const p=document.querySelector(${quote(popup)});if(!p)return null;
    const r=p.getBoundingClientRect(),s=getComputedStyle(p);
    const list=p.querySelector('[role=listbox]')??p;
    const within=r.x>=-1&&r.y>=-1&&r.right<=innerWidth+1&&r.bottom<=innerHeight+1;
    return {width:innerWidth,height:innerHeight,rect:{x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom},within,options:p.querySelectorAll('[role=option]').length,scrollHeight:list.scrollHeight,clientHeight:list.clientHeight,background:s.backgroundColor,color:s.color,border:s.borderColor,font:s.fontFamily,bodyFont:getComputedStyle(document.body).fontFamily,portal:!p.closest('.settings-page')};
  })()`);
}
export function assertFacts(value, label) {
  if(value.horizontalOverflow||value.rawKeys.length||value.badText)throw new Error(`Invalid visible page ${label}: ${JSON.stringify(value)}`);
  if(value.themes.length!==7||value.themes.filter(x=>x.checked).length!==1)throw new Error(`Invalid radio selection ${label}`);
}
