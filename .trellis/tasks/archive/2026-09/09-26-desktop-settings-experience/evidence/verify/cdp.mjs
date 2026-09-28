// Task-local CDP helpers. These helpers never invoke product commands directly.
export const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
export async function connect(port, match) {
  let page;
  for (let i = 0; i < 150; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      page = targets.find((target) => target.type === 'page' && match(target.url));
    } catch { /* The owned browser may still be starting. */ }
    if (page) break;
    await pause(200);
  }
  if (!page) throw new Error('Owned page did not expose CDP');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  let sequence = 0;
  const pending = new Map();
  const events = [];
  ws.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    if (message.method === 'Runtime.exceptionThrown' || message.method === 'Log.entryAdded') events.push(message);
    const request = pending.get(message.id);
    if (!request) return;
    clearTimeout(request.timer);
    pending.delete(message.id);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { resolve, reject, timer });
    ws.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true, userGesture: true });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const click = async (selector, index = 0) => {
    const point = await evaluate(`(() => { const el = document.querySelectorAll(${JSON.stringify(selector)})[${index}]; if (!el || el.disabled) throw new Error('Missing/enabled control: ' + ${JSON.stringify(selector)}); el.scrollIntoView({block:'center'}); const r = el.getBoundingClientRect(); return {x:r.x+r.width/2,y:r.y+r.height/2}; })()`);
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...point });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...point });
  };
  const waitFor = async (expression, label, attempts = 100) => {
    for (let i = 0; i < attempts; i++) {
      if (await evaluate(expression)) return;
      await pause(50);
    }
    throw new Error(`UI condition timed out: ${label}`);
  };
  const key = async (name, code, modifiers = 0) => {
    const text = name === 'Enter' ? String.fromCharCode(13) : undefined;
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: name, windowsVirtualKeyCode: code, modifiers, ...(text ? { code: name, text, unmodifiedText: text } : {}) });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: name, windowsVirtualKeyCode: code, modifiers });
  };
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Log.enable');
  return { page, ws, send, evaluate, click, waitFor, key, events, close: () => ws.close() };
}
export const pageFacts = `(() => {
  const root=document.documentElement, style=getComputedStyle(root);
  return {url:location.href, title:document.title, dpr:devicePixelRatio, width:innerWidth, height:innerHeight, theme:root.dataset.theme, scheme:style.colorScheme, motion:root.dataset.motion, font:style.fontFamily, bodyFont:getComputedStyle(document.body).fontFamily, fontSize:style.fontSize, horizontalOverflow:root.scrollWidth>root.clientWidth+1, controls:[...document.querySelectorAll('[role=combobox]')].map(x=>({id:x.id,value:x.value??x.textContent,disabled:x.disabled,expanded:x.getAttribute('aria-expanded')})), themes:[...document.querySelectorAll('input[name=settings-theme]')].map(x=>({value:x.value,checked:x.checked})), active:document.activeElement?.id, errors:[...document.querySelectorAll('[role=alert]')].map(x=>x.textContent.trim()), rawKeys:(document.body.innerText.match(/\b[a-z]+\.v[12]\.[a-z0-9_.]+/g)||[]).slice(0,10), badText:/\bundefined\b|\bNaN\b/.test(document.body.innerText)};
})()`;
