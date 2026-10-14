// Attach only to the benchmark's explicit loopback WebView2 port. No IPC,
// navigation, scan, mutation, process control, or production dependency.
import { writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const [portArg, outputArg] = process.argv.slice(2);
const port = Number(portArg);
if (!Number.isInteger(port) || port < 1024 || port > 65535 || !outputArg) {
  throw new Error("usage: node tools/inspect-native-readonly.mjs <port> <existing-output-directory>");
}
const output = resolve(outputArg);
const targets = await fetch(`http://127.0.0.1:${port}/json/list`).then((response) => response.json());
const page = targets.find((target) => target.type === "page"
  && /^https?:\/\/tauri\.localhost\//.test(target.url) && !target.url.includes("hud.html"));
if (!page) throw new Error("No packaged Tauri main page at the owned benchmark port");
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.onopen = resolve;
  socket.onerror = reject;
});
let nextId = 0;
const pending = new Map();
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  const entry = pending.get(message.id);
  if (!entry) return;
  pending.delete(message.id);
  clearTimeout(entry.timer);
  if (message.error) entry.reject(new Error(message.error.message));
  else entry.resolve(message.result);
};
function send(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Timeout: ${method}`));
    }, 10000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
try {
  const result = await send("Runtime.evaluate", {
    expression: `(() => ({ url: location.href, title: document.title,
      text: document.body.innerText, ready: document.readyState,
      dpr: devicePixelRatio, width: innerWidth, height: innerHeight,
      visibility: document.visibilityState, userAgent: navigator.userAgent,
      language: document.documentElement.lang, shell: !!document.querySelector('.app-shell'),
      canvases: [...document.querySelectorAll('canvas')].map(c => ({width:c.width,height:c.height})),
      scripts: [...document.scripts].map(s => s.src),
      resources: performance.getEntriesByType('resource').map(r => r.name)
    }))()`, returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  writeFileSync(join(output, "page.json"), JSON.stringify(result.result.value, null, 2), { flag: "wx" });
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(output, "page.png"), Buffer.from(shot.data, "base64"), { flag: "wx" });
  console.log(JSON.stringify(result.result.value, null, 2));
} finally {
  socket.close();
}
