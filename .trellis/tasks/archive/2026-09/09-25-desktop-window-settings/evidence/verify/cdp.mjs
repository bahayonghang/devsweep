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
  ws.onmessage = ({ data }) => {
    const message = JSON.parse(data);
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
  const click = async (selector) => {
    const point = await evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el || el.disabled) throw new Error('Missing/enabled control: ' + ${JSON.stringify(selector)}); el.scrollIntoView({block:'center'}); const r = el.getBoundingClientRect(); return {x:r.x+r.width/2,y:r.y+r.height/2}; })()`);
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...point });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...point });
  };
  const select = async (selector, value) => {
    await evaluate(`(() => { const el=document.querySelector(${JSON.stringify(selector)}); if (!el || el.disabled) throw new Error('Select unavailable'); const set=Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set; set.call(el,${JSON.stringify(String(value))}); el.dispatchEvent(new Event('change',{bubbles:true})); })()`);
    await pause(250);
  };
  await send('Page.enable');
  await send('Runtime.enable');
  return { page, ws, send, evaluate, click, select, close: () => ws.close() };
}
export const pageFacts = `(() => {
  const root=document.documentElement, style=getComputedStyle(root);
  return {url:location.href, title:document.title, dpr:devicePixelRatio, width:innerWidth, height:innerHeight, theme:root.dataset.theme, motion:root.dataset.motion, font:style.fontFamily, fontSize:style.fontSize, horizontalOverflow:root.scrollWidth>root.clientWidth+1, controls:[...document.querySelectorAll('.window-controls button')].map(x=>({name:x.getAttribute('aria-label'),disabled:x.disabled})), selects:[...document.querySelectorAll('select')].map(x=>({id:x.id,value:x.value,disabled:x.disabled})), errors:[...document.querySelectorAll('[role=alert]')].map(x=>x.textContent.trim()), rawKeys:(document.body.innerText.match(/\b[a-z]+\.v1\.[a-z0-9_.]+/g)||[]).slice(0,10), badText:/\bundefined\b|\bNaN\b/.test(document.body.innerText)};
})()`;
