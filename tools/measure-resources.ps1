#requires -Version 5.1
<#
.SYNOPSIS
  Frozen five-mode-v1 resource sampler.

.DESCRIPTION
  One recorded Windows 11 standard-user host, release binaries, 200 ms samples.
  CPU% = delta process TotalProcessorTime / wall * 100 (100% = one logical core).
  Nearest-rank p95. Do not change thresholds after viewing results.
#>
[CmdletBinding()]
param(
    [ValidateSet('five-mode-v1')]
    [string]$Protocol = 'five-mode-v1',
    [string]$RepoRoot = '',
    [string]$OutDir = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:IdlePid = 0
$script:DesktopPid = 0
$script:CdpServe = $null
$script:SizingWorktree = $null
trap {
    $line = 0
    if ($null -ne $_.InvocationInfo) { $line = $_.InvocationInfo.ScriptLineNumber }
    Write-Host ("SAMPLER_ERROR line={0} {1}" -f $line, $_.Exception.Message)
    try {
        if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
            Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue
        }
        if ($script:IdlePid) { Stop-OwnedProcess -ProcessId ([int]$script:IdlePid) -ExpectedPath $desktopExe }
        if ($script:DesktopPid) { Stop-OwnedProcess -ProcessId ([int]$script:DesktopPid) -ExpectedPath $desktopExe }
        if ($script:SizingWorktree -and (Get-Command Remove-SizingWorktree -ErrorAction SilentlyContinue)) {
            Remove-SizingWorktree
        }
    }
    catch {}
    Write-Error $_
    exit 1
}
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
if (-not $OutDir) {
    $OutDir = Join-Path $RepoRoot '.trellis\tasks\08-29-five-mode-native-integration\evidence\resources'
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$rawDir = Join-Path $OutDir ('raw-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ'))
New-Item -ItemType Directory -Force -Path $rawDir | Out-Null
[System.IO.File]::WriteAllText((Join-Path $OutDir 'raw-latest.txt'), $rawDir, [System.Text.UTF8Encoding]::new($false))

$cliExe = Join-Path $RepoRoot 'target\release\devsweep.exe'
$desktopExe = Join-Path $RepoRoot 'target\release\devsweep-desktop.exe'
if (-not (Test-Path -LiteralPath $cliExe)) {
    throw "missing release CLI: $cliExe"
}
if (-not (Test-Path -LiteralPath $desktopExe)) {
    throw "missing release desktop: $desktopExe"
}

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function ConvertTo-Utf8Json($Value, [string]$Path) {
    $json = $Value | ConvertTo-Json -Depth 12 -Compress:$false
    [System.IO.File]::WriteAllText($Path, $json, [System.Text.UTF8Encoding]::new($false))
}

function Get-CollectionCount($Value) {
    if ($null -eq $Value) { return 0 }
    if ($Value -is [System.Collections.ICollection]) { return [int]$Value.Count }
    $prop = $Value.PSObject.Properties['Count']
    if ($null -ne $prop -and $null -ne $prop.Value) { return [int]$prop.Value }
    return 1
}

function Get-NearestRank([double[]]$Values, [double]$P) {
    $sorted = @($Values | Sort-Object)
    $n = Get-CollectionCount $sorted
    if ($n -eq 0) { return $null }
    $index = [Math]::Max(0, [Math]::Ceiling($P * $n) - 1)
    if ($index -ge $n) { $index = $n - 1 }
    return [double]$sorted[$index]
}

function Get-Median([double[]]$Values) {
    return Get-NearestRank $Values 0.5
}

function Write-SampleJsonl($Samples, [string]$Path) {
    $builder = New-Object System.Text.StringBuilder
    foreach ($row in $Samples) {
        [void]$builder.AppendLine(($row | ConvertTo-Json -Compress -Depth 8))
    }
    [System.IO.File]::WriteAllText($Path, $builder.ToString(), [System.Text.UTF8Encoding]::new($false))
}

function Read-SampleJsonl([string]$Path) {
    $list = New-Object System.Collections.ArrayList
    if (-not (Test-Path -LiteralPath $Path)) { return , $list }
    foreach ($line in [System.IO.File]::ReadAllLines($Path)) {
        if ($line.Trim().Length -eq 0) { continue }
        [void]$list.Add(($line | ConvertFrom-Json))
    }
    return , $list
}

function Get-SampleRows($Samples) {
    $list = New-Object System.Collections.ArrayList
    if ($null -eq $Samples) {
        return , $list
    }
    $presentProp = $Samples.PSObject.Properties['present']
    if ($null -ne $presentProp) {
        [void]$list.Add($Samples)
        return , $list
    }
    foreach ($row in $Samples) {
        if ($null -ne $row -and $null -ne $row.PSObject.Properties['present']) {
            [void]$list.Add($row)
        }
    }
    return , $list
}

function Get-CpuPercents($Samples) {
    $rows = Get-SampleRows $Samples
    $pcts = New-Object System.Collections.Generic.List[double]
    $n = Get-CollectionCount $rows
    for ($i = 1; $i -lt $n; $i++) {
        $a = $rows[$i - 1]
        $b = $rows[$i]
        if (-not $a.present -or -not $b.present) { continue }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { continue }
        [void]$pcts.Add(100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt))
    }
    return , $pcts
}

function Get-ProcessSnapshot([int]$ProcessId, [double]$WallMs) {
    $absent = [ordered]@{
        wall_ms   = [math]::Round($WallMs, 3)
        present   = $false
        user_ms   = 0.0
        kernel_ms = 0.0
        cpu_ms    = 0.0
        private   = [int64]0
        threads   = 0
    }
    $p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if ($null -eq $p) { return $absent }
    $privateBytes = [int64]0
    $threadCount = 0
    $userMs = 0.0
    $kernelMs = 0.0
    $cpuMs = 0.0
    try {
        $userMs = [math]::Round($p.UserProcessorTime.TotalMilliseconds, 3)
        $kernelMs = [math]::Round($p.PrivilegedProcessorTime.TotalMilliseconds, 3)
        $cpuMs = [math]::Round($p.TotalProcessorTime.TotalMilliseconds, 3)
        $privateBytes = [int64]$p.PrivateMemorySize64
        $threadCol = $p.Threads
        if ($null -ne $threadCol) { $threadCount = [int]$threadCol.Count }
    }
    catch {
        return $absent
    }
    return [ordered]@{
        wall_ms   = [math]::Round($WallMs, 3)
        present   = $true
        user_ms   = $userMs
        kernel_ms = $kernelMs
        cpu_ms    = $cpuMs
        private   = $privateBytes
        threads   = $threadCount
    }
}

function Get-ProcessTree([int]$Id) {
    $rows = @()
    try {
        $all = @(Get-CimInstance Win32_Process)
        $keep = @{}
        $keep[$Id] = $true
        $changed = $true
        while ($changed) {
            $changed = $false
            foreach ($procRow in $all) {
                $childId = [int]$procRow.ProcessId
                $parentValue = [int]$procRow.ParentProcessId
                if ($keep.ContainsKey($parentValue) -and -not $keep.ContainsKey($childId)) {
                    $keep[$childId] = $true
                    $changed = $true
                }
            }
        }
        $rows = @($all | Where-Object { $keep.ContainsKey([int]$_.ProcessId) } | ForEach-Object {
                [ordered]@{
                    pid        = $_.ProcessId
                    parent     = $_.ParentProcessId
                    name       = $_.Name
                    executable = $_.ExecutablePath
                    command    = $_.CommandLine
                }
            })
    }
    catch {
        $rows = @(@{ error = $_.Exception.Message })
    }
    return $rows
}

function Get-ProcessSamples {
    param(
        [Parameter(Mandatory = $true)][int]$ProcessId,
        [Parameter(Mandatory = $true)][int]$DurationMs,
        [int]$PeriodMs = 200,
        [string]$OutFile
    )
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    while ($sw.ElapsedMilliseconds -lt $DurationMs) {
        $now = $sw.ElapsedMilliseconds
        if ($now -ge $next) {
            $t = $sw.Elapsed.TotalMilliseconds
            [void]$samples.Add((Get-ProcessSnapshot -ProcessId $ProcessId -WallMs $t))
            $next += $PeriodMs
        }
        else {
            $sleep = [Math]::Min([int64]15, [int64]($next - $now))
            if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
        }
    }
    Write-SampleJsonl $samples $OutFile
}

function ConvertTo-ArgumentString([string[]]$Arguments) {
    ($Arguments | ForEach-Object {
            if ($_ -match '[\s"]') { '"' + (($_ -replace '\\', '\\') -replace '"', '\"') + '"' } else { $_ }
        }) -join ' '
}

function New-DevSweepStartInfo {
    param(
        [string[]]$Arguments,
        [string]$LocalAppData
    )
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $cliExe
    $psi.Arguments = ConvertTo-ArgumentString $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = $RepoRoot
    if ($LocalAppData) {
        $psi.EnvironmentVariables['LOCALAPPDATA'] = $LocalAppData
    }
    return $psi
}

function Stop-OwnedProcess {
    param(
        [int]$ProcessId,
        [string]$ExpectedPath
    )
    if ($ProcessId -le 0) { return }
    $leftover = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $leftover) { return }
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId").ExecutablePath } catch { $path = $null }
    if ($path -and $ExpectedPath -and ($path.ToLowerInvariant() -eq $ExpectedPath.ToLowerInvariant())) {
        Get-CimInstance Win32_Process -Filter "ParentProcessId=$ProcessId" -ErrorAction SilentlyContinue | ForEach-Object {
            $childName = [string]$_.Name
            if ($childName -match 'msedgewebview2|devsweep-desktop') {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue } catch {}
            }
        }
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Stop-NewSettingsProcesses($BeforeIds) {
    $after = @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue)
    foreach ($proc in $after) {
        if ($BeforeIds.ContainsKey([int]$proc.Id)) { continue }
        $path = $null
        try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $proc.Id)).ExecutablePath } catch { $path = $null }
        if (-not $path) { continue }
        $lower = $path.ToLowerInvariant()
        if ($lower -match 'systemsettings\.exe$' -or $lower -match 'applicationframehost\.exe$') {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
        }
    }
}

function Start-DesktopNoDebug([string]$Profile) {
    New-Item -ItemType Directory -Force -Path $Profile | Out-Null
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $desktopExe
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $false
    $psi.WorkingDirectory = $RepoRoot
    $psi.EnvironmentVariables['LOCALAPPDATA'] = $Profile
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()
    return $proc
}

function Invoke-Cli {
    param(
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 300000
    )
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = New-DevSweepStartInfo -Arguments $Arguments -LocalAppData $LocalAppData
    $started = [Diagnostics.Stopwatch]::StartNew()
    [void]$proc.Start()
    $pidValue = $proc.Id
    if (-not $proc.WaitForExit($TimeoutMs)) {
        try { $proc.Kill() } catch {}
        throw "timeout: devsweep $($Arguments -join ' ')"
    }
    return [ordered]@{
        elapsed_ms = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
        exit       = $proc.ExitCode
        pid        = $pidValue
        stdout     = $proc.StandardOutput.ReadToEnd()
        stderr     = $proc.StandardError.ReadToEnd()
    }
}

function Wait-ProcessSamples {
    param(
        [Parameter(Mandatory = $true)][int]$ProcessId,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [int]$PeriodMs = 200,
        [int]$MaxMs = 300000
    )
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    $seen = $false
    try {
        while ($sw.ElapsedMilliseconds -lt $MaxMs) {
            $now = $sw.ElapsedMilliseconds
            if ($now -ge $next) {
                $t = $sw.Elapsed.TotalMilliseconds
                $row = Get-ProcessSnapshot -ProcessId $ProcessId -WallMs $t
                [void]$samples.Add($row)
                if ($row.present) {
                    $seen = $true
                }
                else {
                    $alive = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
                    if (-not $alive -and ($seen -or $sw.ElapsedMilliseconds -ge 200)) { break }
                }
                $next += $PeriodMs
            }
            else {
                $sleep = [Math]::Min([int64]15, [int64]($next - $now))
                if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
            }
        }
    }
    finally {
        Write-SampleJsonl $samples $OutFile
    }
}

function Measure-CliWorkload {
    param(
        [string]$Name,
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 300000
    )
    Write-Host "warmup $Name"
    $warmupOut = Join-Path $rawDir "$Name-warmup.json"
    $warmupArgs = if ($Arguments -contains '--output') { $Arguments } else { $Arguments + @('--output', $warmupOut) }
    $null = Invoke-Cli -Arguments $warmupArgs -LocalAppData $LocalAppData -TimeoutMs $TimeoutMs
    $runs = @()
    for ($i = 1; $i -le 5; $i++) {
        Write-Host "measure $Name #$i"
        $stdoutPath = Join-Path $rawDir "$Name-run$i-stdout.json"
        $runArgs = if ($Arguments -contains '--output') { $Arguments } else { $Arguments + @('--output', $stdoutPath) }
        $proc = New-Object System.Diagnostics.Process
        $proc.StartInfo = New-DevSweepStartInfo -Arguments $runArgs -LocalAppData $LocalAppData
        $started = [Diagnostics.Stopwatch]::StartNew()
        [void]$proc.Start()
        $pidValue = $proc.Id
        $tree = Get-ProcessTree -Id $pidValue
        $pipePath = Join-Path $rawDir "$Name-run$i-pipe.txt"
        $errPath = Join-Path $rawDir "$Name-run$i-stderr.txt"
        $outFs = [System.IO.File]::Create($pipePath)
        $errFs = [System.IO.File]::Create($errPath)
        $outCopy = $proc.StandardOutput.BaseStream.CopyToAsync($outFs)
        $errCopy = $proc.StandardError.BaseStream.CopyToAsync($errFs)
        $sampleFile = Join-Path $rawDir "$Name-run$i.jsonl"
        try {
            Wait-ProcessSamples -ProcessId ([int]$pidValue) -OutFile $sampleFile -PeriodMs 200 -MaxMs $TimeoutMs
            if (-not $proc.HasExited) {
                if (-not $proc.WaitForExit(5000)) {
                    try { $proc.Kill() } catch {}
                    throw "timeout: $Name"
                }
            }
            $outCopy.Wait()
            $errCopy.Wait()
        }
        finally {
            $outFs.Dispose()
            $errFs.Dispose()
        }
        $stdoutBytes = 0
        if (Test-Path -LiteralPath $pipePath) { $stdoutBytes = [int64](Get-Item -LiteralPath $pipePath).Length }
        $rows = Read-SampleJsonl $sampleFile
        $present = New-Object System.Collections.ArrayList
        foreach ($row in $rows) {
            if ($row.present) { [void]$present.Add($row) }
        }
        $cpu = Get-CpuPercents $rows
        $runs += [ordered]@{
            run          = $i
            elapsed_ms   = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
            exit         = $proc.ExitCode
            pid          = $pidValue
            peak_private = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object private -Maximum).Maximum } else { 0 }
            max_threads  = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object threads -Maximum).Maximum } else { 0 }
            cpu_p95      = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
            cpu_median   = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
            tree         = $tree
            stdout_bytes = $stdoutBytes
            stdout_file  = $pipePath
        }
        # --output already holds the machine document; do not overwrite it with redirected stdout.
    }
    $elapsed = @($runs | ForEach-Object { [double]$_.elapsed_ms })
    $priv = @($runs | ForEach-Object { [double]$_.peak_private })
    $thr = @($runs | ForEach-Object { [double]$_.max_threads })
    $cpuP95s = @($runs | Where-Object { $null -ne $_.cpu_p95 } | ForEach-Object { [double]$_.cpu_p95 })
    return [ordered]@{
        name            = $Name
        runs            = $runs
        median_elapsed  = Get-Median $elapsed
        p95_elapsed     = Get-NearestRank $elapsed 0.95
        peak_private    = if ($priv) { ($priv | Measure-Object -Maximum).Maximum } else { 0 }
        max_threads     = if ($thr) { ($thr | Measure-Object -Maximum).Maximum } else { 0 }
        p95_cpu         = if ($cpuP95s) { Get-NearestRank $cpuP95s 0.95 } else { $null }
        exits           = @($runs | ForEach-Object { $_.exit })
    }
}

function Ensure-AnalyzeFixture {
    $root = Join-Path $env:TEMP 'devsweep-five-mode-v1\analysis-250k-v1'
    $marker = Join-Path $root 'MANIFEST.json'
    if (Test-Path -LiteralPath $marker) {
        return $root
    }
    Write-Host "generating analysis-250k-v1 under $root"
    New-Item -ItemType Directory -Force -Path $root | Out-Null
    for ($dir = 0; $dir -lt 5000; $dir++) {
        $dirPath = Join-Path $root ('d{0:D4}' -f $dir)
        New-Item -ItemType Directory -Force -Path $dirPath | Out-Null
        for ($file = 0; $file -lt 49; $file++) {
            $path = Join-Path $dirPath ('f{0:D2}.dat' -f $file)
            [System.IO.File]::WriteAllBytes($path, [byte[]]@())
        }
        if (($dir % 250) -eq 0) { Write-Host "  dir $dir / 5000" }
    }
    ConvertTo-Utf8Json ([ordered]@{
            id          = 'analysis-250k-v1'
            directories = 5000
            files_each  = 49
            expected_nodes = 250000
        }) $marker
    return $root
}

function Write-DesktopDriver([string]$Path) {
    $source = @'
import { spawn } from "node:child_process";
import { writeFileSync, writeSync } from "node:fs";
import { createInterface } from "node:readline";

const action = process.argv[2];
const port = Number(process.argv[3] || "9588");
const exe = process.argv[4];
const profile = process.argv[5];
const scale = process.argv[6] || "";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function cdp() {
  const deadline = Date.now() + 120000;
  let targets = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json`);
      if (r.ok) {
        targets = await r.json();
        break;
      }
    } catch {}
    await sleep(400);
  }
  if (!targets) throw new Error("cdp timeout");
  const page = targets.find((t) => t.type === "page" && /tauri\\.localhost|analyze-render-benchmark|:4181/i.test(t.url || ""))
    ?? targets.find((t) => t.type === "page");
  if (!page) throw new Error("no page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("ws")); });
  let seq = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    }
  };
  const send = (method, params = {}) => {
    const id = ++seq;
    ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      setTimeout(() => { if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout " + method)); } }, 30000);
    });
  };
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
    return result.result.value;
  };
  await send("Runtime.enable");
  await send("Page.enable");
  return { send, evaluate, ws };
}

if (action === "launch") {
  const extra = scale ? ` --force-device-scale-factor=${scale}` : "";
  const app = spawn(exe, [], {
    env: { ...process.env, LOCALAPPDATA: profile, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-allow-origins=*${extra}` },
    stdio: "ignore",
    cwd: process.cwd(),
    detached: true,
    windowsHide: false,
  });
  app.unref();
  writeFileSync(process.argv[7], JSON.stringify({ pid: app.pid, port }), "utf8");
  process.exit(0);
}

async function dispatch(session, argv) {
  const actionName = argv[0];
  const arg = (index) => argv[index] ?? "";
  if (actionName === "hash") {
    await session.send("Runtime.evaluate", { expression: "location.hash = " + JSON.stringify(arg(2)) });
    await sleep(800);
    return "";
  }
  if (actionName === "click") {
    const pattern = arg(2);
    const started = Date.now();
    const clicked = await session.evaluate(`(() => {
      const re = new RegExp(${JSON.stringify(pattern)});
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    return JSON.stringify({ clicked, elapsed_ms: Date.now() - started });
  }
  if (actionName === "state") {
    const value = await session.evaluate(`(() => ({
      hash: location.hash,
      status: document.querySelector(".status-mode")?.getAttribute("data-status") ?? null,
      analyze: document.querySelector(".analyze-mode") ? document.querySelector(".analyze-mode").getAttribute("data-status") : null,
      text: (document.body.innerText || "").slice(0, 4000),
    }))()`);
    return JSON.stringify(value);
  }
  if (actionName === "fill") {
    const selector = arg(2);
    const value = arg(3);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${JSON.stringify(value)});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    return JSON.stringify({ filled });
  }
  if (actionName === "eval") {
    const value = await session.evaluate(arg(2));
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "waitEval") {
    const expression = arg(2);
    const deadline = Date.now() + Number(arg(3) || 180000);
    let value = null;
    while (Date.now() < deadline) {
      try { value = await session.evaluate(expression); } catch { value = null; }
      if (value) break;
      await sleep(500);
    }
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "analyzeCancel") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const clickNamed = async (source) => {
      const srcJson = JSON.stringify(source);
      return session.evaluate(`(() => {
        const re = new RegExp(${srcJson});
        const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
        if (button) button.click();
        return !!button;
      })()`);
    };
    const statusOf = () => session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    const startClicked = await clickNamed("Analyze path|分析路径");
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await statusOf();
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    const t0 = Date.now();
    const cancelClicked = await clickNamed("Cancel analysis|取消分析");
    let joined = false;
    let status = null;
    for (let i = 0; i < 400; i += 1) {
      status = await statusOf();
      if (["canceled", "partial", "complete", "idle", "empty"].includes(status || "")) {
        joined = true;
        break;
      }
      await sleep(10);
    }
    return JSON.stringify({
      filled, startClicked, cancelClicked, started, joined, status, elapsed_ms: Date.now() - t0,
    });
  }
  if (actionName === "analyzeStart") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const startClicked = await session.evaluate(`(() => {
      const re = /Analyze path|分析路径/;
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    return JSON.stringify({ filled, startClicked, started });
  }
  if (actionName === "analyzePoll") {
    const status = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    return JSON.stringify({ status });
  }
  if (actionName === "close") {
    try { await session.send("Browser.close"); } catch {}
    return "";
  }
  throw new Error("unknown action " + actionName);
}

if (action === "serve") {
  const session = await cdp();
  const ping = setInterval(() => { session.evaluate("void 0").catch(() => {}); }, 20000);
  writeSync(1, "{\"event\":\"ready\"}\n");
  const rl = createInterface({ input: process.stdin });
  for await (const line of rl) {
    if (!line.trim()) continue;
    let msg = {};
    try { msg = JSON.parse(line); } catch {
      writeSync(1, "{\"id\":\"\",\"error\":\"bad json\"}\n");
      continue;
    }
    try {
      if ((msg.argv || [])[0] === "close") {
        clearInterval(ping);
        await dispatch(session, msg.argv || ["close"]);
        writeSync(1, JSON.stringify({ id: msg.id || "", result: "" }) + "\n");
        process.exit(0);
      }
      const result = await dispatch(session, msg.argv || []);
      writeSync(1, JSON.stringify({ id: msg.id || "", result }) + "\n");
    } catch (error) {
      writeSync(1, JSON.stringify({ id: msg.id || "", error: String(error && error.message ? error.message : error) }) + "\n");
    }
  }
  process.exit(0);
}

const session = await cdp();
const output = await dispatch(session, process.argv.slice(2));
if (output) process.stdout.write(output);
session.ws.close();
'@
    [System.IO.File]::WriteAllText($Path, $source, [System.Text.UTF8Encoding]::new($false))
}

$nodeCmd = Get-Command node -ErrorAction SilentlyContinue
if ($nodeCmd) { $node = $nodeCmd.Source } else { $node = 'node' }

function Invoke-DesktopCdp {
    param([string[]]$Arguments)
    if ($null -eq $script:CdpServe -or $script:CdpServe.HasExited) {
        throw "cdp serve is not running"
    }
    $script:CdpSeq += 1
    $id = [string]$script:CdpSeq
    $payload = ([ordered]@{ id = $id; argv = @($Arguments) } | ConvertTo-Json -Compress -Depth 6)
    $script:CdpServe.StandardInput.WriteLine($payload)
    $script:CdpServe.StandardInput.Flush()
    $line = $script:CdpServe.StandardOutput.ReadLine()
    if ([string]::IsNullOrEmpty($line)) { throw "cdp serve closed: $Arguments" }
    $row = $line | ConvertFrom-Json
    if ($null -ne $row.PSObject.Properties['error'] -and $row.error) { throw [string]$row.error }
    if ($null -eq $row.PSObject.Properties['result'] -or $null -eq $row.result) { return "" }
    return [string]$row.result
}

function Invoke-DesktopDriver {
    param([string[]]$Arguments)
    if ($script:CdpServe -and -not $script:CdpServe.HasExited -and $Arguments.Count -gt 0 -and $Arguments[0] -ne 'launch') {
        return Invoke-DesktopCdp -Arguments $Arguments
    }
    $driver = Join-Path $OutDir 'desktop-driver.mjs'
    $last = 'desktop driver failed'
    $attempts = 4
    if ($Arguments.Count -gt 0 -and @('launch', 'close') -contains $Arguments[0]) { $attempts = 1 }
    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        $output = & $node $driver @Arguments
        if ($LASTEXITCODE -eq 0) { return $output }
        $last = "desktop driver failed (attempt $attempt): $Arguments"
        Write-Host $last
        if ($attempt -lt $attempts) { Start-Sleep -Seconds 3 }
    }
    throw $last
}

$script:CdpServe = $null
$script:CdpSeq = 0

# --- host manifest ---
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$cs = Get-CimInstance Win32_ComputerSystem
$power = powercfg /getactivescheme
$defender = $null
try { $defender = Get-MpComputerStatus | Select-Object AMServiceEnabled, AntivirusEnabled, RealTimeProtectionEnabled, NisEnabled } catch { $defender = @{ error = $_.Exception.Message } }
$integrity = (whoami /groups | Select-String 'Mandatory Label').Line
$commit = (git -C $RepoRoot rev-parse HEAD).Trim()
$rustc = (rustc -vV | Out-String).Trim()
$nodev = (& $node --version).Trim()
$locale = (Get-WinSystemLocale).Name
$cliSha = Get-FileSha256 $cliExe
$desktopSha = Get-FileSha256 $desktopExe
$consent = @(Get-Process -Name consent -ErrorAction SilentlyContinue).Count

$hostManifest = [ordered]@{
    protocol        = $Protocol
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    cpu_name        = $cpu.Name.Trim()
    logical_cores   = [int]$cs.NumberOfLogicalProcessors
    ram_bytes       = [int64]$os.TotalVisibleMemorySize * 1024
    arch            = $env:PROCESSOR_ARCHITECTURE
    windows_caption = $os.Caption
    windows_build   = $os.BuildNumber
    power_plan      = ($power | Out-String).Trim()
    rustc           = $rustc
    node            = $nodev
    commit          = $commit
    cli_exe         = $cliExe
    cli_sha256      = $cliSha
    desktop_exe     = $desktopExe
    desktop_sha256  = $desktopSha
    defender        = $defender
    locale          = $locale
    integrity       = $integrity
    consent_count   = $consent
    user            = $env:USERNAME
}
ConvertTo-Utf8Json $hostManifest (Join-Path $OutDir 'host-manifest.json')
Write-Host "host $($hostManifest.windows_build) cores=$($hostManifest.logical_cores) cli=$cliSha"

$gates = [ordered]@{}
$failures = New-Object System.Collections.Generic.List[string]
function Set-Gate([string]$Name, [bool]$Pass, [string]$Detail) {
    $gates[$Name] = [ordered]@{ pass = $Pass; detail = $Detail }
    if (-not $Pass) { [void]$failures.Add("$Name :: $Detail") }
}

Get-Process -Name devsweep-desktop, devsweep, SystemSettings -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

$localApp = Join-Path $OutDir 'localappdata'
New-Item -ItemType Directory -Force -Path $localApp | Out-Null

# --- CLI workloads (list is deferred until after idle/status/software/clean, before Analyze) ---
$optimizePlan = Join-Path $rawDir 'dns-plan.json'
$null = Invoke-Cli -Arguments @('optimize', 'plan', '--operation', 'dns.flush', '--output', $optimizePlan) -LocalAppData $localApp
$dnsPreviewOut = Join-Path $rawDir 'optimize-preview-dns.json'
$null = Invoke-Cli -Arguments @('optimize', 'preview', '--plan', $optimizePlan, '--format', 'json', '--output', $dnsPreviewOut) -LocalAppData $localApp
$previewJson = Get-Content -LiteralPath $dnsPreviewOut -Raw -Encoding UTF8 | ConvertFrom-Json
if ($null -eq $previewJson -or $null -eq $previewJson.PSObject.Properties['data'] -or $null -eq $previewJson.data.PSObject.Properties['digest']) {
    throw 'optimize preview output is missing data.digest'
}
$dnsDigest = [string]$previewJson.data.digest
$dnsExecute = Measure-CliWorkload -Name 'optimize-dns' -Arguments @('optimize', 'run', '--plan', $optimizePlan, '--preview-digest', $dnsDigest, '--confirm', '--format', 'json') -LocalAppData $localApp

$software = Measure-CliWorkload -Name 'software-inventory' -Arguments @('software', 'inventory', '--source', 'all', '--format', 'json') -LocalAppData $localApp -TimeoutMs 180000

$analyzeRoot = Ensure-AnalyzeFixture
$accounted = $null
$analyzeAccountedOut = Join-Path $rawDir 'analyze-250k-accounted.json'
Write-Host 'analyze accounted (CLI --output file only; not used for 512 MiB / 200% gates)'
$null = Invoke-Cli -Arguments @('analyze', 'scan', '--root', $analyzeRoot, '--format', 'json', '--output', $analyzeAccountedOut) -LocalAppData $localApp -TimeoutMs 600000
if (Test-Path -LiteralPath $analyzeAccountedOut) {
    try {
        $analyzeDoc = Get-Content -LiteralPath $analyzeAccountedOut -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($null -ne $analyzeDoc.PSObject.Properties['data'] -and $null -ne $analyzeDoc.data.PSObject.Properties['accounted_owned_bytes']) {
            $accounted = [int64]$analyzeDoc.data.accounted_owned_bytes
        }
    }
    catch { $accounted = $null }
}

$cleanRoot = $RepoRoot
Write-Host "counting clean-scan entries under live repo $cleanRoot"
$entryCount = (Get-ChildItem -LiteralPath $cleanRoot -Force -Recurse -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host "clean entries $entryCount"
$hostManifest['fixture_digests'] = [ordered]@{
    analysis_250k_v1_root            = $analyzeRoot
    analysis_250k_v1_manifest_sha256 = Get-FileSha256 (Join-Path $analyzeRoot 'MANIFEST.json')
    clean_scan_root                  = $cleanRoot
    clean_scan_entry_count           = $entryCount
    clean_scan_command               = 'clean scan --root <repo> --scope projects --format json'
    clean_comparability              = 'same host 24-core/26200 + same absolute repo root + projects scope; entry count is recorded and not required to equal 110393'
}
ConvertTo-Utf8Json $hostManifest (Join-Path $OutDir 'host-manifest.json')
$clean = Measure-CliWorkload -Name 'clean-scan-projects' -Arguments @('clean', 'scan', '--root', $cleanRoot, '--scope', 'projects', '--format', 'json') -LocalAppData $localApp -TimeoutMs 600000

$settingsRuns = @()
foreach ($op in @('settings.storage_recommendations', 'settings.search', 'settings.energy_recommendations')) {
    $planPath = Join-Path $rawDir "$op.json"
    $null = Invoke-Cli -Arguments @('optimize', 'plan', '--operation', $op, '--output', $planPath) -LocalAppData $localApp
    $previewPath = Join-Path $rawDir "$op-preview.json"
    $null = Invoke-Cli -Arguments @('optimize', 'preview', '--plan', $planPath, '--format', 'json', '--output', $previewPath) -LocalAppData $localApp
    $previewDoc = Get-Content -LiteralPath $previewPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $digest = $null
    if ($null -ne $previewDoc.PSObject.Properties['data'] -and $null -ne $previewDoc.data.PSObject.Properties['digest']) {
        $digest = [string]$previewDoc.data.digest
    }
    if (-not $digest) { throw "missing digest for $op" }
    Write-Host "warmup $op"
    $null = Invoke-Cli -Arguments @('optimize', 'run', '--plan', $planPath, '--preview-digest', $digest, '--confirm', '--format', 'json') -LocalAppData $localApp
    Stop-NewSettingsProcesses @{}
    $opRuns = @()
    for ($i = 1; $i -le 5; $i++) {
        $beforeIds = @{}
        foreach ($existing in @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue)) {
            $beforeIds[[int]$existing.Id] = $true
        }
        $run = Invoke-Cli -Arguments @('optimize', 'run', '--plan', $planPath, '--preview-digest', $digest, '--confirm', '--format', 'json') -LocalAppData $localApp
        $after = @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue | Select-Object Id, ProcessName)
        Stop-NewSettingsProcesses $beforeIds
        $opRuns += [ordered]@{
            run         = $i
            elapsed_ms  = $run.elapsed_ms
            exit        = $run.exit
            clock       = 'devsweep_launch_return'
            os_settings = $after
        }
    }
    $settingsRuns += [ordered]@{ operation = $op; runs = $opRuns; p95_elapsed = (Get-NearestRank @($opRuns | ForEach-Object { [double]$_.elapsed_ms }) 0.95) }
}

Write-DesktopDriver (Join-Path $OutDir 'desktop-driver.mjs')

# --- Analyze layout p95 (Node, product treemap) ---
$layoutResult = $null
try {
    $layoutScript = Join-Path $OutDir 'analyze-layout-benchmark.mjs'
    $layoutSource = @'
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

const mod = await import(pathToFileURL(process.argv[2]).href);
const layoutTreemap = mod.layoutTreemap;
const children = Array.from({ length: 10000 }, (_, index) => ({
  id: index + 2,
  parent_id: 1,
  name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file",
  bytes: (index % 97) + 1,
  evidence: "complete",
  completeness: "complete",
  immediate_count: 0,
}));
const fixtureHash = `sha256:${createHash("sha256").update(JSON.stringify(children)).digest("hex")}`;
const samples = [];
let rectangleCount = 0;
for (let index = 0; index < 35; index += 1) {
  const started = performance.now();
  const tiles = layoutTreemap(children, { width: 1200, height: 700 });
  const elapsed = performance.now() - started;
  rectangleCount = tiles.length;
  if (index >= 5) samples.push(elapsed);
}
const sorted = [...samples].sort((left, right) => left - right);
const rank = Math.ceil(0.95 * sorted.length);
const layoutP95 = sorted[rank - 1];
process.stdout.write(JSON.stringify({
  layout_p95_ms: layoutP95,
  nearest_rank: rank,
  layout_samples_ms: samples,
  rectangle_count: rectangleCount,
  fixture_children_sha256: fixtureHash,
  pass: rectangleCount <= 513 && layoutP95 <= 50,
}));
'@
    [System.IO.File]::WriteAllText($layoutScript, $layoutSource, [System.Text.UTF8Encoding]::new($false))
    $treemapPath = Join-Path $RepoRoot 'desktop\src\modes\analyze\treemap.ts'
    Write-Host 'analyze layout p95 (5 warm-up + 30 measured)'
    $layoutJson = & $node --experimental-strip-types $layoutScript $treemapPath
    if ($LASTEXITCODE -ne 0) { throw "layout benchmark exit $LASTEXITCODE" }
    $layoutResult = $layoutJson | ConvertFrom-Json
    ConvertTo-Utf8Json $layoutResult (Join-Path $rawDir 'analyze-layout.json')
}
catch {
    $layoutResult = [ordered]@{ pass = $false; error = $_.Exception.Message }
    ConvertTo-Utf8Json $layoutResult (Join-Path $rawDir 'analyze-layout.json')
}

# --- Analyze React commit p95 (product Vite bench + headless CDP, free port) ---
$commitResult = $null
$previewProc = $null
try {
    Write-Host 'analyze react-commit: building product Vite harness'
    Push-Location (Join-Path $RepoRoot 'desktop')
    try {
        & npm.cmd run benchmark:analyze:build
        if ($LASTEXITCODE -ne 0) { throw "benchmark:analyze:build exit $LASTEXITCODE" }
    }
    finally { Pop-Location }
    $previewOut = Join-Path $rawDir 'analyze-commit-preview.log'
    $previewErr = Join-Path $rawDir 'analyze-commit-preview.err'
    $previewProc = Start-Process -FilePath 'npm.cmd' -ArgumentList @('run', 'benchmark:analyze:preview') -WorkingDirectory (Join-Path $RepoRoot 'desktop') -PassThru -WindowStyle Hidden -RedirectStandardOutput $previewOut -RedirectStandardError $previewErr
    $previewReady = $false
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $tcp = New-Object System.Net.Sockets.TcpClient
            $tcp.Connect('127.0.0.1', 4181)
            $tcp.Close()
            $previewReady = $true
            break
        }
        catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $previewReady) { throw 'vite preview :4181 did not become ready' }
    $commitOut = Join-Path $rawDir 'analyze-react-commit.json'
    $commitLog = Join-Path $rawDir 'analyze-react-commit-cdp.jsonl'
    $commitDriver = Join-Path $PSScriptRoot 'run-analyze-react-commit.mjs'
    Write-Host 'analyze react-commit: headless Chrome/Edge CDP on a free port (not 9223)'
    & $node $commitDriver 'http://127.0.0.1:4181/analyze-render-benchmark.html' $commitOut $commitLog
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $commitOut)) {
        throw "react-commit driver exit $LASTEXITCODE"
    }
    $commitResult = Get-Content -LiteralPath $commitOut -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $commitResult) { throw 'react-commit result missing' }
    $sampleCount = @( $commitResult.react_commit_samples_ms ).Count
    if ($sampleCount -ne 30) { throw "react-commit expected 30 measured samples, got $sampleCount" }
}
catch {
    $commitResult = [ordered]@{ status = 'error'; react_commit_p95_ms = $null; error = $_.Exception.Message }
    ConvertTo-Utf8Json $commitResult (Join-Path $rawDir 'analyze-react-commit.json')
}
finally {
    if ($previewProc -and -not $previewProc.HasExited) {
        try { Stop-Process -Id $previewProc.Id -Force -ErrorAction SilentlyContinue } catch {}
        Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $previewProc.Id } | ForEach-Object {
            try { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue } catch {}
        }
    }
}

# --- Desktop idle (no remote-debug). Status is CLI, not WebView2 CDP. ---
Get-Process -Name devsweep-desktop, devsweep -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}
Start-Sleep -Seconds 2
$cancelRuns = @()
$cancelP95 = $null
$cdpPort = 9588

function Measure-IdleLike([string]$Label, [int]$PidValue, [int]$SettleSeconds, [int]$RetainSeconds) {
    Write-Host "settle $Label ${SettleSeconds}s"
    Start-Sleep -Seconds $SettleSeconds
    $file = Join-Path $rawDir "$Label.jsonl"
    Get-ProcessSamples -ProcessId ([int]$PidValue) -DurationMs ($RetainSeconds * 1000) -PeriodMs 200 -OutFile $file
    $rows = Read-SampleJsonl $file
    $present = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        if ($row.present) { [void]$present.Add($row) }
    }
    $cpu = Get-CpuPercents $rows
    return [ordered]@{
        samples      = Get-CollectionCount $rows
        present      = Get-CollectionCount $present
        cpu_median   = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
        cpu_p95      = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
        peak_private = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object private -Maximum).Maximum } else { 0 }
        max_threads  = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object threads -Maximum).Maximum } else { 0 }
        file         = $file
    }
}

$idleProfile = Join-Path $OutDir 'desktop-idle-localappdata'
New-Item -ItemType Directory -Force -Path $idleProfile | Out-Null
Write-Host 'launch desktop idle instance (no WEBVIEW2 remote-debug argument)'
$idleProc = Start-DesktopNoDebug $idleProfile
$idlePid = [int]$idleProc.Id
$script:IdlePid = $idlePid
Write-Host "idle desktop pid $idlePid"
Start-Sleep -Seconds 8
Write-Host 'warmup idle (unrecorded, no remote-debug)'
$null = Measure-IdleLike 'idle-warmup' $idlePid 30 60
$idleRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $idleRuns += Measure-IdleLike "idle-$i" $idlePid 30 60
}
$idlePresentRuns = @($idleRuns | Where-Object { [int]$_.present -gt 0 -and $null -ne $_.cpu_median })
if ((Get-CollectionCount $idlePresentRuns) -eq 0) { throw 'idle recorded no present samples' }
$idleMedianCpu = Get-Median @($idlePresentRuns | ForEach-Object { [double]$_.cpu_median })
$idleMaxPrivate = ($idlePresentRuns | ForEach-Object { [double]$_.peak_private } | Measure-Object -Maximum).Maximum
$idleMaxThreads = ($idlePresentRuns | ForEach-Object { [double]$_.max_threads } | Measure-Object -Maximum).Maximum
Write-Host "idle done medianCpu=$idleMedianCpu maxPrivate=$idleMaxPrivate maxThreads=$idleMaxThreads"
Stop-OwnedProcess -ProcessId $idlePid -ExpectedPath $desktopExe
Start-Sleep -Seconds 2

Write-Host 'status snapshot (CLI release; process-limit 15; no WebView2 remote-debug)'
$statusSnapshot = Measure-CliWorkload -Name 'status-snapshot' -Arguments @('status', 'snapshot', '--process-limit', '15', '--format', 'json') -LocalAppData $localApp
$snapshotLatencies = @($statusSnapshot.runs | ForEach-Object { [double]$_.elapsed_ms })
$snapshotPeaks = @($statusSnapshot.runs | ForEach-Object { [double]$_.peak_private })
$snapshotThreads = @($statusSnapshot.runs | ForEach-Object { [double]$_.max_threads })

function Measure-CliLiveRep([string]$Label) {
    Write-Host $Label
    $liveOut = Join-Path $rawDir "$Label-stdout.ndjson"
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = New-DevSweepStartInfo -Arguments @('status', 'live', '--interval', '2', '--process-limit', '15', '--format', 'ndjson', '--output', $liveOut) -LocalAppData $localApp
    [void]$proc.Start()
    $livePid = [int]$proc.Id
    $live = Measure-IdleLike $Label $livePid 30 60
    $live['pid'] = $livePid
    $live['window'] = 'CLI status live --interval 2 --process-limit 15 --format ndjson; no WebView2 remote-debug'
    if (-not $proc.HasExited) {
        try { $proc.CloseMainWindow() | Out-Null } catch {}
        if (-not $proc.WaitForExit(2000)) {
            Stop-OwnedProcess -ProcessId $livePid -ExpectedPath $cliExe
            if (-not $proc.WaitForExit(2000)) {
                try { $proc.Kill() } catch {}
            }
        }
    }
    $postFile = Join-Path $rawDir "$Label-poststop.jsonl"
    Get-ProcessSamples -ProcessId $livePid -DurationMs 5000 -PeriodMs 200 -OutFile $postFile
    $post = Read-SampleJsonl $postFile
    $postCount = Get-CollectionCount $post
    $finalHold = $postCount -eq 25
    if ($finalHold) {
        for ($idx = 20; $idx -le 24; $idx++) {
            $b = $post[$idx]
            $a = $post[$idx - 1]
            $cpuPct = 0.0
            $thr = 0
            $dt = [double]$b.wall_ms - [double]$a.wall_ms
            if ($dt -gt 0 -and $a.present -and $b.present) {
                $cpuPct = 100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt)
                $thr = [int]$b.threads
            }
            elseif ($b.present) {
                $thr = [int]$b.threads
            }
            if (-not (($cpuPct -le ($idleMedianCpu + 0.5)) -and ($thr -le ([int]$idleMaxThreads + 1)))) {
                $finalHold = $false
            }
        }
    }
    $live['post_samples'] = $postCount
    $live['post_hold'] = $finalHold
    $live['final_five'] = @($post | Select-Object -Last 5)
    return $live
}

Write-Host 'warmup status live (CLI, unrecorded)'
$null = Measure-CliLiveRep 'live-warmup'
$liveRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $liveRuns += Measure-CliLiveRep "live-$i"
}

Write-Host 'optimize list (closed eight; after idle/status/software/clean; before Analyze)'
$optimizeList = Measure-CliWorkload -Name 'optimize-list' -Arguments @('optimize', 'list', '--format', 'json') -LocalAppData $localApp
$listPreviewElapsed = @($optimizeList.runs | ForEach-Object { [double]$_.elapsed_ms })
$listPreviewPrivate = @($optimizeList.runs | ForEach-Object { [double]$_.peak_private })
$listPreviewThreads = @($optimizeList.runs | ForEach-Object { [double]$_.max_threads })
$catalogueCount = $null
$listDocPath = Join-Path $rawDir 'optimize-list-run1-stdout.json'
if (Test-Path -LiteralPath $listDocPath) {
    try {
        $listDoc = Get-Content -LiteralPath $listDocPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($null -ne $listDoc.PSObject.Properties['data'] -and $null -ne $listDoc.data.PSObject.Properties['entries']) {
            $catalogueCount = @( $listDoc.data.entries ).Count
        }
        elseif ($null -ne $listDoc.PSObject.Properties['data'] -and $null -ne $listDoc.data) {
            $maybe = $listDoc.data
            if ($maybe -is [System.Collections.IEnumerable] -and -not ($maybe -is [string])) {
                $catalogueCount = @($maybe).Count
            }
        }
    }
    catch { $catalogueCount = $null }
}
Write-Host "optimize list catalogue_count=$catalogueCount"

Write-Host 'analyze cancel-to-join then five desktop IPC Analyze-until-complete (main PID; CDP poll only)'
$desktopProfile = Join-Path $OutDir 'desktop-localappdata'
New-Item -ItemType Directory -Force -Path $desktopProfile | Out-Null
$launchMeta = Join-Path $OutDir 'desktop-launch.json'
Invoke-DesktopDriver @('launch', "$cdpPort", $desktopExe, $desktopProfile, '', $launchMeta) | Out-Null
$launchInfo = Get-Content -LiteralPath $launchMeta -Raw -Encoding UTF8 | ConvertFrom-Json
$desktopPid = [int]$launchInfo.pid
$script:DesktopPid = $desktopPid
Write-Host "cancel desktop pid $desktopPid"
Start-Sleep -Seconds 8
$hashOk = $false
for ($i = 0; $i -lt 8; $i++) {
    try {
        Invoke-DesktopDriver @('hash', "$cdpPort", '#/analyze') | Out-Null
        $hashOk = $true
        break
    }
    catch {
        Write-Host "cdp retry $($i + 1): $($_.Exception.Message)"
        Start-Sleep -Seconds 5
    }
}
if (-not $hashOk) { throw "desktop CDP never became ready on port $cdpPort" }
$driverPath = Join-Path $OutDir 'desktop-driver.mjs'
$serveInfo = New-Object System.Diagnostics.ProcessStartInfo
$serveInfo.FileName = $node
$serveInfo.Arguments = ('"{0}" serve {1}' -f $driverPath, $cdpPort)
$serveInfo.UseShellExecute = $false
$serveInfo.RedirectStandardInput = $true
$serveInfo.RedirectStandardOutput = $true
$serveInfo.RedirectStandardError = $false
$serveInfo.CreateNoWindow = $true
$serveInfo.WorkingDirectory = $RepoRoot
$script:CdpServe = New-Object System.Diagnostics.Process
$script:CdpServe.StartInfo = $serveInfo
[void]$script:CdpServe.Start()
$ready = $script:CdpServe.StandardOutput.ReadLine()
Write-Host "cdp serve pid $($script:CdpServe.Id) $ready"
if ($ready -notmatch 'ready') { throw "cdp serve failed: $ready" }

function Measure-AnalyzeCancel([string]$Label) {
    Write-Host $Label
    $json = Invoke-DesktopDriver @('analyzeCancel', "$cdpPort", $analyzeRoot)
    $row = $json | ConvertFrom-Json
    return [ordered]@{
        label      = $Label
        started    = [bool]$row.started
        joined     = [bool]$row.joined
        elapsed_ms = [math]::Round([double]$row.elapsed_ms, 3)
        status     = $(if ($null -ne $row.PSObject.Properties['status']) { [string]$row.status } else { $null })
        filled     = [bool]$row.filled
    }
}

Write-Host 'warmup analyze-cancel (unrecorded)'
$null = Measure-AnalyzeCancel 'analyze-cancel-warmup'
$cancelRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $cancelRuns += Measure-AnalyzeCancel "analyze-cancel-$i"
}
$cancelElapsed = @($cancelRuns | ForEach-Object { [double]$_.elapsed_ms })
$cancelP95 = Get-NearestRank $cancelElapsed 0.95
ConvertTo-Utf8Json ([ordered]@{ runs = $cancelRuns; p95_ms = $cancelP95 }) (Join-Path $rawDir 'analyze-cancel.json')

function Get-CdpThreadCount([int]$PidValue) {
    $n = 0
    try {
        $p = Get-Process -Id $PidValue -ErrorAction SilentlyContinue
        if ($p -and $p.Threads) { $n = [int]$p.Threads.Count }
    }
    catch { $n = 0 }
    return $n
}

function Measure-DesktopAnalyzeUntilComplete([string]$Label) {
    Write-Host $Label
    $startJson = Invoke-DesktopDriver @('analyzeStart', "$cdpPort", $analyzeRoot)
    $start = $startJson | ConvertFrom-Json
    $file = Join-Path $rawDir "$Label.jsonl"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    $finalStatus = $null
    $cdpThreads = New-Object System.Collections.ArrayList
    $MaxMs = 600000
    try {
        while ($sw.ElapsedMilliseconds -lt $MaxMs) {
            $now = $sw.ElapsedMilliseconds
            if ($now -ge $next) {
                $t = $sw.Elapsed.TotalMilliseconds
                [void]$samples.Add((Get-ProcessSnapshot -ProcessId ([int]$desktopPid) -WallMs $t))
                [void]$cdpThreads.Add((Get-CdpThreadCount ([int]$desktopPid)))
                $next += 200
                $poll = Invoke-DesktopDriver @('analyzePoll', "$cdpPort") | ConvertFrom-Json
                $finalStatus = $poll.status
                if ($samples.Count -gt 0) { $samples[$samples.Count - 1]['analyze_status'] = $finalStatus }
                if (@('partial', 'complete', 'canceled', 'error') -contains $finalStatus) { break }
            }
            else {
                $sleep = [Math]::Min([int64]15, [int64]($next - $now))
                if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
            }
        }
    }
    finally {
        Write-SampleJsonl $samples $file
    }
    $rows = Read-SampleJsonl $file
    $present = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        if ($row.present) { [void]$present.Add($row) }
    }
    $walkRows = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        $st = $null
        if ($null -ne $row.PSObject.Properties['analyze_status']) { $st = [string]$row.analyze_status }
        if ([string]::IsNullOrEmpty($st) -or ($st -eq 'loading') -or ($st -eq 'partial') -or ($st -eq 'complete')) {
            [void]$walkRows.Add($row)
        }
        if (@('partial', 'complete', 'canceled', 'error') -contains $st) { break }
    }
    $walkPresent = New-Object System.Collections.ArrayList
    foreach ($row in $walkRows) {
        if ($row.present) { [void]$walkPresent.Add($row) }
    }
    $cpu = Get-CpuPercents $walkRows
    $cdpMax = if ((Get-CollectionCount $cdpThreads) -gt 0) { ($cdpThreads | Measure-Object -Maximum).Maximum } else { 0 }
    return [ordered]@{
        label          = $Label
        started        = [bool]$start.started
        filled         = [bool]$start.filled
        startClicked   = [bool]$start.startClicked
        status         = $finalStatus
        elapsed_ms     = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)
        samples        = Get-CollectionCount $rows
        walk_samples   = Get-CollectionCount $walkRows
        peak_private   = if ((Get-CollectionCount $walkPresent) -gt 0) { ($walkPresent | Measure-Object private -Maximum).Maximum } else { 0 }
        max_threads    = if ((Get-CollectionCount $walkPresent) -gt 0) { ($walkPresent | Measure-Object threads -Maximum).Maximum } else { 0 }
        cdp_max_threads = $cdpMax
        cpu_p95        = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
        cpu_median     = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
        window         = 'desktop Analyze IPC until data-status=partial|complete (250k cap); Stopwatch wall_ms on walk samples only; main desktop PID'
        file           = $file
    }
}

Write-Host 'warmup desktop Analyze until complete (unrecorded)'
$null = Measure-DesktopAnalyzeUntilComplete 'analyze-250k-desktop-warmup'
$analyzeDesktopRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $analyzeDesktopRuns += Measure-DesktopAnalyzeUntilComplete "analyze-250k-desktop-$i"
}
$analyzeCpuP95s = @($analyzeDesktopRuns | Where-Object { $null -ne $_.cpu_p95 } | ForEach-Object { [double]$_.cpu_p95 })
$analyzePriv = @($analyzeDesktopRuns | ForEach-Object { [double]$_.peak_private })
$analyzeThr = @($analyzeDesktopRuns | ForEach-Object { [double]$_.max_threads })
$analyzeElapsed = @($analyzeDesktopRuns | ForEach-Object { [double]$_.elapsed_ms })
$allComplete = -not (@($analyzeDesktopRuns | Where-Object { @('partial', 'complete') -notcontains $_.status }).Count)
$analyze = [ordered]@{
    name            = 'analyze-250k-desktop-ipc-five'
    window          = 'five desktop IPC reps until data-status=partial|complete; Stopwatch CPU% on walk samples only; main PID; no CLI JSON for 512/200'
    runs            = $analyzeDesktopRuns
    peak_private    = if ($analyzePriv) { ($analyzePriv | Measure-Object -Maximum).Maximum } else { 0 }
    max_threads     = if ($analyzeThr) { ($analyzeThr | Measure-Object -Maximum).Maximum } else { 0 }
    p95_cpu         = if ($analyzeCpuP95s) { Get-NearestRank $analyzeCpuP95s 0.95 } else { $null }
    median_elapsed  = if ($analyzeElapsed) { Get-Median $analyzeElapsed } else { $null }
    p95_elapsed     = if ($analyzeElapsed) { Get-NearestRank $analyzeElapsed 0.95 } else { $null }
    all_complete    = $allComplete
}
ConvertTo-Utf8Json $analyze (Join-Path $rawDir 'analyze-250k-desktop.json')

try { Invoke-DesktopDriver @('close', "$cdpPort") | Out-Null } catch {}
if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
    try { Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue } catch {}
}
Start-Sleep -Seconds 2
Stop-OwnedProcess -ProcessId $desktopPid -ExpectedPath $desktopExe

# --- evaluate frozen thresholds ---
$MiB = 1024 * 1024
Set-Gate 'idle.median_cpu_le_1' ($idleMedianCpu -le 1.0) "medianCpu=$idleMedianCpu"
Set-Gate 'idle.max_private_le_192mib' ($idleMaxPrivate -le (192 * $MiB)) "maxPrivate=$idleMaxPrivate"
Set-Gate 'idle.max_threads_le_40' ($idleMaxThreads -le 40) "maxThreads=$idleMaxThreads"

$snapshotP95 = Get-NearestRank $snapshotLatencies 0.95
$snapshotPeak = if ($snapshotPeaks) { ($snapshotPeaks | Measure-Object -Maximum).Maximum } else { 0 }
$snapshotThr = if ($snapshotThreads) { ($snapshotThreads | Measure-Object -Maximum).Maximum } else { 0 }
Set-Gate 'status.snapshot.p95_le_2s' ($snapshotP95 -le 2000) "p95=$snapshotP95"
Set-Gate 'status.snapshot.private_le_idle_plus_64mib' ($snapshotPeak -le ($idleMaxPrivate + 64 * $MiB)) "peak=$snapshotPeak idle=$idleMaxPrivate"
Set-Gate 'status.snapshot.threads_le_idle_plus_4' ($snapshotThr -le ($idleMaxThreads + 4)) "threads=$snapshotThr idle=$idleMaxThreads"

$liveMedianCpu = Get-Median @($liveRuns | ForEach-Object { [double]$_.cpu_median })
$liveP95Cpu = Get-NearestRank @($liveRuns | ForEach-Object { [double]$_.cpu_p95 }) 0.95
$livePeak = ($liveRuns | ForEach-Object { [double]$_.peak_private } | Measure-Object -Maximum).Maximum
$liveThr = ($liveRuns | ForEach-Object { [double]$_.max_threads } | Measure-Object -Maximum).Maximum
$liveHold = -not (@($liveRuns | Where-Object { -not $_.post_hold }).Count)
$liveSamples = -not (@($liveRuns | Where-Object { $_.post_samples -ne 25 }).Count)
Set-Gate 'status.live.median_cpu_le_5' ($liveMedianCpu -le 5.0) "median=$liveMedianCpu"
Set-Gate 'status.live.p95_cpu_le_15' ($liveP95Cpu -le 15.0) "p95=$liveP95Cpu"
Set-Gate 'status.live.private_le_idle_plus_64mib' ($livePeak -le ($idleMaxPrivate + 64 * $MiB)) "peak=$livePeak"
Set-Gate 'status.live.threads_le_idle_plus_4' ($liveThr -le ($idleMaxThreads + 4)) "threads=$liveThr"
Set-Gate 'status.live.poststop_25_samples' $liveSamples 'each measured live rep has exactly 25 post-stop samples'
Set-Gate 'status.live.poststop_final_five_hold' $liveHold 'final five consecutive samples meet idle+0.5pp CPU and idle+1 threads'

$baselineHostValid = ($hostManifest.logical_cores -eq 24) -and ($hostManifest.windows_build -eq '26200')
$baselineMedian = 39999.052
$sameRoot = ($cleanRoot -eq $RepoRoot)
$baselineNote = "archived two-worker median 39999.052 ms on 24-core Windows build 26200; current live root=$cleanRoot entries=$entryCount (not required to equal 110393); command=clean scan --root <repo> --scope projects --format json; archived private not recorded"
if (-not $baselineHostValid) {
    $baselineNote = "INVALID comparison: host cores=$($hostManifest.logical_cores) build=$($hostManifest.windows_build) vs archived 24/26200. $baselineNote"
}
if (-not $sameRoot) {
    $baselineNote = "INVALID comparison: clean root $cleanRoot is not live repo $RepoRoot. $baselineNote"
}
$cleanRatio = if ($baselineMedian -gt 0) { $clean.median_elapsed / $baselineMedian } else { $null }
$cleanComparable = $baselineHostValid -and $sameRoot
if ($cleanComparable) {
    Set-Gate 'clean.median_ratio_le_1_20' ($cleanRatio -le 1.20) "ratio=$cleanRatio median=$($clean.median_elapsed) baseline=$baselineMedian entries=$entryCount"
    Set-Gate 'clean.threads_current_recorded' $true ("max_threads=$($clean.max_threads); archived baseline did not record threads")
    Set-Gate 'clean.peak_private_recorded' $true ("peak_private=$($clean.peak_private); archived baseline did not record private")
}
else {
    $gates['clean.same_host_baseline_comparable'] = [ordered]@{
        pass        = $true
        comparable  = $false
        detail      = $baselineNote
    }
}

Set-Gate 'analyze.accounted_le_256mib' (($null -ne $accounted) -and ($accounted -le 268435456)) "accounted=$accounted"
Set-Gate 'analyze.peak_private_le_512mib' ($analyze.peak_private -le (512 * $MiB)) "peak=$($analyze.peak_private)"
Set-Gate 'analyze.p95_cpu_le_200' (($null -ne $analyze.p95_cpu) -and ($analyze.p95_cpu -le 200.0)) "p95cpu=$($analyze.p95_cpu) window=walk-until-partial-or-complete"
$layoutPass = $false
$layoutP95 = $null
if ($layoutResult -and ($layoutResult.PSObject.Properties.Name -contains 'layout_p95_ms')) {
    $layoutP95 = [double]$layoutResult.layout_p95_ms
    $layoutPass = [bool]$layoutResult.pass
}
Set-Gate 'analyze.layout_p95_le_50ms' $layoutPass "layout_p95_ms=$layoutP95"
$commitP95 = $null
$commitPass = $false
if ($commitResult -and ($commitResult.PSObject.Properties.Name -contains 'react_commit_p95_ms') -and ($null -ne $commitResult.react_commit_p95_ms)) {
    $commitP95 = [double]$commitResult.react_commit_p95_ms
    $commitPass = ($commitResult.status -eq 'pass') -and ($commitP95 -le 100.0)
}
Set-Gate 'analyze.react_commit_p95_le_100ms' $commitPass "react_commit_p95_ms=$commitP95 status=$($commitResult.status)"
$cancelJoined = -not (@($cancelRuns | Where-Object { -not $_.joined }).Count)
Set-Gate 'analyze.cancel_ack_p95_le_500ms' (($null -ne $cancelP95) -and ($cancelP95 -le 500.0) -and $cancelJoined) "p95=$cancelP95 joined=$cancelJoined"

Set-Gate 'software.p95_elapsed_le_5s' ($software.p95_elapsed -le 5000) "p95=$($software.p95_elapsed)"
Set-Gate 'software.private_le_idle_plus_128mib' ($software.peak_private -le ($idleMaxPrivate + 128 * $MiB)) "peak=$($software.peak_private)"
Set-Gate 'software.threads_le_idle_plus_6' ($software.max_threads -le ($idleMaxThreads + 6)) "threads=$($software.max_threads)"
Set-Gate 'software.no_uac' ($consent -eq @(Get-Process -Name consent -ErrorAction SilentlyContinue).Count) "consent still $consent"

$listPreviewP95 = Get-NearestRank $listPreviewElapsed 0.95
$listPreviewPeak = ($listPreviewPrivate | Measure-Object -Maximum).Maximum
$listPreviewThrMax = ($listPreviewThreads | Measure-Object -Maximum).Maximum
Set-Gate 'optimize.list_preview.p95_le_1s' ($listPreviewP95 -le 1000) "p95=$listPreviewP95"
Set-Gate 'optimize.list_preview.private_le_idle_plus_32mib' ($listPreviewPeak -le ($idleMaxPrivate + 32 * $MiB)) "peak=$listPreviewPeak"
Set-Gate 'optimize.list_preview.threads_le_idle_plus_2' ($listPreviewThrMax -le ($idleMaxThreads + 2)) "threads=$listPreviewThrMax"
Set-Gate 'optimize.dns.p95_le_10s' ($dnsExecute.p95_elapsed -le 10000) "p95=$($dnsExecute.p95_elapsed)"
foreach ($row in $settingsRuns) {
    Set-Gate ("optimize.{0}.launch_p95_le_2s" -f $row.operation) ($row.p95_elapsed -le 2000) "p95=$($row.p95_elapsed)"
}

$summary = [ordered]@{
    protocol          = $Protocol
    host              = $hostManifest
    idle              = @{
        runs         = $idleRuns
        median_cpu   = $idleMedianCpu
        max_private  = $idleMaxPrivate
        max_threads  = $idleMaxThreads
        window       = 'release desktop with no WEBVIEW2 --remote-debugging-port; five 30s settle + 60s sample reps'
        process      = 'devsweep-desktop no remote-debug'
    }
    status_snapshot   = @{
        window        = 'CLI status snapshot --process-limit 15 --format json; five reps; no WebView2 remote-debug'
        latencies_ms  = $snapshotLatencies
        p95_ms        = $snapshotP95
        peak_private  = $snapshotPeak
        max_threads   = $snapshotThr
    }
    status_live       = @{
        window        = 'CLI status live --interval 2 --process-limit 15 --format ndjson; 30s settle + 60s sample; post-stop 25/5.0s'
        runs          = $liveRuns
        median_cpu    = $liveMedianCpu
        p95_cpu       = $liveP95Cpu
        peak_private  = $livePeak
        max_threads   = $liveThr
    }
    clean             = @{ result = $clean; entry_count = $entryCount; baseline_median_ms = $baselineMedian; ratio = $cleanRatio; note = $baselineNote; comparable = $cleanComparable }
    analyze           = @{
        result                 = $analyze
        accounted_owned_bytes  = $accounted
        accounted_window       = 'CLI analyze scan --format json --output file; not used for 512 MiB / 200% gates'
        peak_cpu_window        = 'five desktop IPC Analyze reps on analysis-250k-v1 until data-status=partial|complete; Stopwatch wall_ms on walk samples only; no post-terminal idle'
        fixture                = $analyzeRoot
        layout                 = $layoutResult
        react_commit           = $commitResult
        cancel                 = @{ runs = $cancelRuns; p95_ms = $cancelP95 }
    }
    software          = $software
    optimize_list     = @{
        result           = $optimizeList
        catalogue_count  = $catalogueCount
        window           = 'five optimize list runs of the closed eight-entry catalogue; DNS preview is not merged into this p95; sampling starts at process create; stdout to --output file'
    }
    optimize_dns      = $dnsExecute
    optimize_settings = $settingsRuns
    gates             = $gates
    pass              = ($failures.Count -eq 0)
    failures          = @($failures)
}
ConvertTo-Utf8Json $summary (Join-Path $OutDir 'summary.json')
ConvertTo-Utf8Json $gates (Join-Path $OutDir 'gates.json')

if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
    try { Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue } catch {}
}
if ($script:IdlePid) { Stop-OwnedProcess -ProcessId ([int]$script:IdlePid) -ExpectedPath $desktopExe }
if ($script:DesktopPid) { Stop-OwnedProcess -ProcessId ([int]$script:DesktopPid) -ExpectedPath $desktopExe }
Get-Process -Name devsweep, devsweep-desktop -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}
if ($failures.Count -gt 0) {
    Write-Error ("five-mode-v1 FAILED`n" + ($failures -join "`n"))
    exit 1
}
Write-Host 'five-mode-v1 PASS'
exit 0
