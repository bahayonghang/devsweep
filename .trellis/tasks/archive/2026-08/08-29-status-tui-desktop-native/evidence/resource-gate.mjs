// Resource gate for Status presentation on the release desktop binary.
// Idle (snapshot-only), snapshot p95, 60 s explicit live, 25-sample post-exit.
import { spawn, execFileSync, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repo = resolve(import.meta.dirname, "../../../..");
const exe = join(repo, "target/release/devsweep-desktop.exe");
const evidence = join(repo, ".trellis/tasks/08-29-status-tui-desktop-native/evidence/native-desktop");
const resourceDir = join(evidence, "resource");
const sampler = join(resourceDir, "sample-pid.ps1");
mkdirSync(resourceDir, { recursive: true });

if (!existsSync(exe)) throw new Error("missing release desktop binary: " + exe);

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const MiB = 1024 * 1024;

function pct(values, p) {
  const sorted = values.filter((v) => Number.isFinite(v)).slice().sort((a, b) => a - b);
  if (sorted.length === 0) return null;
  const idx = Math.max(0, Math.min(sorted.length - 1, Math.ceil(p * sorted.length) - 1));
  return sorted[idx];
}

function cpuSeries(samples) {
  const pcts = [];
  for (let i = 1; i < samples.length; i += 1) {
    const a = samples[i - 1];
    const b = samples[i];
    if (!a.present || !b.present) continue;
    const dt = b.wall_ms - a.wall_ms;
    if (dt <= 0) continue;
    pcts.push(100 * (b.cpu_ms - a.cpu_ms) / dt);
  }
  return pcts;
}

function samplePid(id, durationMs, periodMs, outFile) {
  execFileSync("powershell.exe", [
    "-NoLogo", "-NoProfile", "-File", sampler,
    "-Id", String(id),
    "-DurationMs", String(durationMs),
    "-PeriodMs", String(periodMs),
    "-OutFile", outFile,
  ], { encoding: "utf8" });
  const raw = readFileSync(outFile, "utf8").replace(/^\uFEFF/, "");
  const parsed = JSON.parse(raw);
  return Array.isArray(parsed) ? parsed : [parsed];
}

function oneSample(id) {
  const outFile = join(resourceDir, `oneshot-${id}-${Date.now()}.json`);
  return samplePid(id, 1, 1, outFile)[0];
}

async function launch(cdpPort) {
  const localAppData = join(evidence, "resource-localappdata");
  mkdirSync(localAppData, { recursive: true });
  const env = {
    ...process.env,
    LOCALAPPDATA: localAppData,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
  };
  const app = spawn(exe, [], { env, cwd: repo, stdio: "ignore" });
  const deadline = Date.now() + 45000;
  let targets = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${cdpPort}/json`);
      if (r.ok) {
        targets = await r.json();
        break;
      }
    } catch {}
    await sleep(500);
  }
  if (!targets) throw new Error("cdp timeout port " + cdpPort);
  const page = targets.find((t) => t.type === "page" && /tauri\.localhost/i.test(t.url || ""));
  const chosen = page ?? targets.find((t) => t.type === "page");
  if (!chosen) throw new Error("no page target");
  const ws = new WebSocket(chosen.webSocketDebuggerUrl);
  await new Promise((resolveWs, reject) => {
    ws.onopen = resolveWs;
    ws.onerror = () => reject(new Error("ws error"));
  });
  let seq = 0;
  const pending = new Map();
  const send = (method, params = {}) => {
    const id = ++seq;
    ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve2, reject) => {
      pending.set(id, { resolve: resolve2, reject });
      setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id);
          reject(new Error("cdp timeout: " + method));
        }
      }, 30000);
    });
  };
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve: resolve2, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve2(message.result);
    }
  };
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (result.exceptionDetails) {
      throw new Error("page exception: " + JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
    }
    return result.result.value;
  };
  return { app, send, evaluate, pid: app.pid };
}

async function waitReady(evaluate) {
  let last = null;
  for (let i = 0; i < 50; i += 1) {
    await sleep(400);
    last = await evaluate(`(() => ({
      status: document.querySelector('.status-mode')?.getAttribute('data-status') ?? null,
      snapshot: /Status snapshot|状态快照/.test(document.body.innerText || ''),
    }))()`);
    if (last.status === "ready" && last.snapshot) return last;
  }
  throw new Error("status snapshot did not become ready: " + JSON.stringify(last));
}

const os = execSync('powershell.exe -NoLogo -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).Caption; (Get-CimInstance Win32_OperatingSystem).BuildNumber"', { encoding: "utf8" }).trim().split(/\r?\n/);
const integrity = execSync("whoami /groups", { encoding: "utf8" }).split(/\r?\n/).find((line) => /Mandatory Label/i.test(line)) ?? "";
const consent = Number(execSync('powershell.exe -NoLogo -NoProfile -Command "@(Get-Process -Name consent -ErrorAction SilentlyContinue).Count"', { encoding: "utf8" }).trim());

const session = await launch(9571);
await session.send("Runtime.enable");
await session.send("Page.enable");
await sleep(2200);
await session.send("Runtime.evaluate", { expression: "location.hash = '#/status'" });
await sleep(800);
const warmup = await waitReady(session.evaluate);

const idleSamples = samplePid(session.pid, 2000, 200, join(resourceDir, "idle.json"));
writeFileSync(join(resourceDir, "idle.json"), JSON.stringify(idleSamples, null, 2));
const idlePrivate = idleSamples.filter((s) => s.present).map((s) => s.private);
const idleThreads = idleSamples.filter((s) => s.present).map((s) => s.threads);
const idlePrivateMedian = pct(idlePrivate, 0.5) ?? 0;
const idleThreadsMedian = pct(idleThreads, 0.5) ?? 0;

const snapshotLatencies = [];
for (let i = 0; i < 5; i += 1) {
  const started = Date.now();
  await session.evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Refresh snapshot|刷新快照/.test(b.textContent || ''))?.click(); })()`);
  await waitReady(session.evaluate);
  snapshotLatencies.push(Date.now() - started);
}
const snapshotP95 = pct(snapshotLatencies, 0.95);

await session.evaluate(`(() => { Array.from(document.querySelectorAll('button')).find((b) => /Start live|开始实时/.test(b.textContent || ''))?.click(); })()`);
let liveReady = null;
for (let i = 0; i < 40; i += 1) {
  await sleep(400);
  liveReady = await session.evaluate(`(() => ({
    status: document.querySelector('.status-mode')?.getAttribute('data-status'),
    stop: !!Array.from(document.querySelectorAll('button')).find((b) => /Stop live|停止实时/.test(b.textContent || '')),
  }))()`);
  if (liveReady.status === "live" && liveReady.stop) break;
}
if (liveReady?.status !== "live") throw new Error("live did not start: " + JSON.stringify(liveReady));

const liveSamples = samplePid(session.pid, 60000, 200, join(resourceDir, "live-60s.json"));
writeFileSync(join(resourceDir, "live-60s.json"), JSON.stringify(liveSamples, null, 2));
const liveCpu = cpuSeries(liveSamples);
const livePrivate = liveSamples.filter((s) => s.present).map((s) => s.private);
const liveThreads = liveSamples.filter((s) => s.present).map((s) => s.threads);
const liveCpuMedian = pct(liveCpu, 0.5);
const liveCpuP95 = pct(liveCpu, 0.95);
const livePrivatePeak = livePrivate.length ? Math.max(...livePrivate) : 0;
const liveThreadsPeak = liveThreads.length ? Math.max(...liveThreads) : 0;

try { await session.send("Browser.close"); } catch {}
const gracefulDeadline = Date.now() + 2000;
while (Date.now() < gracefulDeadline) {
  try {
    execSync(`powershell.exe -NoLogo -NoProfile -Command "Get-Process -Id ${session.pid} | Out-Null"`, { stdio: "pipe" });
    await sleep(50);
  } catch {
    break;
  }
}
let remaining = "";
try {
  remaining = execSync(
    `powershell.exe -NoLogo -NoProfile -Command "Get-CimInstance Win32_Process -Filter \\"ProcessId=${session.pid}\\" | Select-Object -ExpandProperty ExecutablePath"`,
    { encoding: "utf8" },
  ).trim();
} catch {
  remaining = "";
}
if (remaining && remaining.toLowerCase() === exe.toLowerCase()) {
  try { execSync(`taskkill /PID ${session.pid} /F`, { stdio: "pipe" }); } catch {}
}
const exitDeadline = Date.now() + 3000;
while (Date.now() < exitDeadline) {
  try {
    execSync(`powershell.exe -NoLogo -NoProfile -Command "Get-Process -Id ${session.pid} | Out-Null"`, { stdio: "pipe" });
    await sleep(50);
  } catch {
    break;
  }
}

const postSamplesRaw = samplePid(session.pid, 5000, 200, join(resourceDir, "post-exit.json"));
let postSamples = postSamplesRaw.slice();
while (postSamples.length < 25) postSamples.push(oneSample(session.pid) ?? { wall_ms: 0, present: false, user_ms: 0, kernel_ms: 0, cpu_ms: 0, private: 0, threads: 0 });
postSamples = postSamples.slice(0, 25);
writeFileSync(join(resourceDir, "post-exit.json"), JSON.stringify(postSamples, null, 2));

const idlePlus64 = idlePrivateMedian + 64 * MiB;
const idlePlus4 = idleThreadsMedian + 4;
const finalFive = postSamples.slice(-5);
const missingSample = postSamples.length !== 25;
const finalFiveHold = !missingSample && finalFive.every((s) => s.private <= idlePlus64 && s.threads <= idlePlus4);

const gates = {
  snapshot_p95_le_2000ms: snapshotP95 !== null && snapshotP95 <= 2000,
  live_private_le_idle_plus_64mib: livePrivatePeak <= idlePlus64,
  live_threads_le_idle_plus_4: liveThreadsPeak <= idlePlus4,
  live_cpu_median_le_5: liveCpuMedian !== null && liveCpuMedian <= 5,
  live_cpu_p95_le_15: liveCpuP95 !== null && liveCpuP95 <= 15,
  post_exit_25_samples: postSamples.length === 25,
  post_exit_final_five_hold: finalFiveHold,
  post_exit_no_missing_sample: !missingSample,
};

const summary = {
  host: {
    os: os[0] ?? null,
    build: os[1] ?? null,
    integrity,
    consent,
    sha256: sha256(exe),
    exe,
    pid: session.pid,
  },
  warmup,
  snapshot_latency_ms: snapshotLatencies,
  snapshot_p95_ms: snapshotP95,
  idle: {
    private_median: idlePrivateMedian,
    threads_median: idleThreadsMedian,
    sample_count: idleSamples.length,
  },
  live60: {
    sample_count: liveSamples.length,
    present_count: liveSamples.filter((s) => s.present).length,
    cpu_median_pct: liveCpuMedian,
    cpu_p95_pct: liveCpuP95,
    private_peak: livePrivatePeak,
    threads_peak: liveThreadsPeak,
    live_ready: liveReady,
  },
  post_exit: {
    sample_count: postSamples.length,
    final_five: finalFive,
    hold: finalFiveHold,
  },
  thresholds: { idlePlus64, idlePlus4 },
  gates,
  pass: Object.values(gates).every(Boolean),
};

writeFileSync(join(resourceDir, "summary.json"), JSON.stringify(summary, null, 2));
writeFileSync(join(resourceDir, "harness.log"), JSON.stringify(summary, null, 2) + "\n");
if (!summary.pass) {
  console.error(JSON.stringify(gates, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(gates, null, 2));
process.exit(0);
