// Task-local DOM and accessibility evidence. No image capture.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { connect, pause, pageFacts } from './cdp.mjs';
const [portArg, outArg, kind = 'fixture'] = process.argv.slice(2);
if (!portArg || !outArg) throw new Error('Usage: ui-matrix.mjs port out fixture|native');
const out = resolve(outArg);
mkdirSync(out, { recursive: true });
const page = await connect(Number(portArg), (url) => kind === 'native' ? /tauri\.localhost/.test(url) && !/hud\.html/.test(url) : /127\.0\.0\.1:4180/.test(url) && !/hud\.html/.test(url));
const rows = [];
const visit = async (route) => { await page.evaluate(`location.hash=${JSON.stringify('#/' + route)}`); await pause(350); };
const change = async (id, value) => {
  await page.select('#' + id, value);
  for (let i = 0; i < 40; i++) {
    if (await page.evaluate(`(() => {const el=document.getElementById(${JSON.stringify(id)});return el && !el.disabled && el.value===${JSON.stringify(String(value))};})()`)) return;
    await pause(100);
  }
  throw new Error(`Preference did not commit: ${id}=${value}`);
};
const capture = async (label) => {
  const facts = await page.evaluate(pageFacts);
  rows.push({ label, kind, ...facts });
  if (facts.horizontalOverflow || facts.rawKeys.length || facts.badText) throw new Error(`Invalid visible layout: ${label}`);
};
try {
  for (const locale of ['en', 'zh-CN']) {
    await visit('settings');
    await change('settings-language', locale);
    for (const theme of ['dark', 'light', 'system']) {
      await change('settings-theme', theme);
      for (const width of [390, 800, 1024, 1440]) {
        await page.send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
        await pause(80);
        await capture(`${locale}-${theme}-${width}`);
      }
    }
    await page.send('Emulation.setDeviceMetricsOverride', { width: 1024, height: 900, deviceScaleFactor: 1, mobile: false });
    for (const font of ['system', 'segoe_ui', 'microsoft_yahei_ui']) {
      await change('settings-font', font);
      for (const scale of [100, 110, 125]) {
        await change('settings-text-scale', scale);
        await capture(`${locale}-${font}-${scale}`);
      }
    }
    for (const theme of ['dark', 'light']) {
      await visit('settings');
      await change('settings-theme', theme);
      for (const route of ['clean','software','optimize','analyze','status','history','protection','rules','settings']) {
        await visit(route);
        await capture(`${locale}-${theme}-${route}-125`);
      }
    }
  }
  await visit('settings');
  await change('settings-theme', 'system');
  for (const scheme of ['dark','light']) {
    await page.send('Emulation.setEmulatedMedia', { features: [{name:'prefers-color-scheme',value:scheme}] });
    await pause(100);
    await capture(`system-media-${scheme}`);
    if (rows.at(-1).theme !== scheme) throw new Error('System theme did not update');
  }
  await page.send('Emulation.setEmulatedMedia', { features: [{name:'prefers-reduced-motion',value:'reduce'}] });
  await pause(100);
  await capture('reduced-motion');
  if (!await page.evaluate(`document.getElementById('settings-planet-fps').disabled`)) throw new Error('Frame control enabled under reduced motion');
  await page.send('Emulation.setEmulatedMedia', { features: [{name:'forced-colors',value:'active'}] });
  await pause(100);
  await capture('forced-colors');
  await page.send('Accessibility.enable');
  writeFileSync(join(out, 'settings-accessibility.json'), JSON.stringify(await page.send('Accessibility.getFullAXTree'), null, 2));
  await page.send('Emulation.setEmulatedMedia', { features: [] });
  await page.send('Emulation.clearDeviceMetricsOverride');
  writeFileSync(join(out, 'matrix.json'), JSON.stringify(rows, null, 2));
  console.log(JSON.stringify({kind, rows:rows.length, horizontalOverflow:rows.filter(row=>row.horizontalOverflow).length, out}));
} finally {
  writeFileSync(join(out, 'matrix.json'), JSON.stringify(rows, null, 2));
  page.close();
}
