$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C11R10SelectFrames {
    public const uint KEYUP = 2;
    public const uint WM_CLOSE = 0x0010;
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);
    public static void AltUp() { keybd_event(0x12, 0, KEYUP, UIntPtr.Zero); }
    public static void Tap(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
    }
}
'@
$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$trace = Join-Path $evidence 'c11-r10-select-frames.txt'
if (Test-Path $trace) { Remove-Item $trace -Force }
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }
function NearestRank([double[]]$samples) {
    if ($samples.Count -eq 0) { return $null }
    $sorted = @($samples | Sort-Object)
    $rank = [Math]::Ceiling(0.95 * $sorted.Count)
    if ($rank -lt 1) { $rank = 1 }
    return [double]$sorted[$rank - 1]
}
function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) { [void][C11R10SelectFrames]::PostMessage($hwnd, [C11R10SelectFrames]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$local = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-select\local'
$roaming = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-select\roaming'
$appRoot = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-select'
if (Test-Path $appRoot) { Remove-Item $appRoot -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
[System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\clean-scan-roots-v1.json'), '{"schema_version":1,"roots":["C:\\Users\\lyh\\AppData\\Local\\Temp\\devsweep-gpui-c11-r10-perf\\scan-root"]}', $utf8)
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $repo
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$info.EnvironmentVariables['DEVSWEEP_FRAME_TRACE'] = $trace
$proc = [System.Diagnostics.Process]::Start($info)
$deadline = (Get-Date).AddSeconds(25)
do { Start-Sleep -Milliseconds 200; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
$hwnd = $proc.MainWindowHandle
$current = [C11R10SelectFrames]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C11R10SelectFrames]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
function Assert-Foreground([IntPtr]$window, [uint32]$threadId) {
    $foreground = [C11R10SelectFrames]::GetForegroundWindow()
    [uint32]$owner = 0
    $fgThread = [C11R10SelectFrames]::GetWindowThreadProcessId($foreground, [ref]$owner)
    [void][C11R10SelectFrames]::AttachThreadInput($script:current, $fgThread, $true)
    [void][C11R10SelectFrames]::AttachThreadInput($script:current, $threadId, $true)
    [C11R10SelectFrames]::AltUp()
    [void][C11R10SelectFrames]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 150
    return ([C11R10SelectFrames]::GetForegroundWindow() -eq $window)
}
function Selected-Count([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $count = 0
    foreach ($item in $items) {
        $name = ''
        try { $name = [string]$item.Current.Name } catch {}
        if ($name.Contains('[x]')) { $count++ }
    }
    return $count
}
try {
    if (-not (Assert-Foreground $hwnd $targetThread)) { throw 'foreground failed' }
    Write-Log 'foreground_ok=True'
    Write-Log "binary_sha256=$((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash)"
    [C11R10SelectFrames]::Tap(0x78)
    $deadline = (Get-Date).AddSeconds(40)
    $finished = $false
    do {
        Start-Sleep -Milliseconds 250
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
        $hit = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Scan finished. 10000 targets found.')))
        if ($null -ne $hit) { $finished = $true }
    } while (-not $finished -and (Get-Date) -lt $deadline)
    Write-Log "ten_k_present=$finished"
    if (-not $finished) { throw '10k rows were not presented' }
    [void](Assert-Foreground $hwnd $targetThread)
    $samples = New-Object System.Collections.Generic.List[double]
    $timeouts = 0
    for ($i = 0; $i -lt 105; $i++) {
        if (($i % 25) -eq 0) { [void](Assert-Foreground $hwnd $targetThread) }
        $prior = Selected-Count $hwnd
        $t0 = [DateTime]::UtcNow
        [C11R10SelectFrames]::Tap(0x20)
        Start-Sleep -Milliseconds 20
        $limit = $t0.AddMilliseconds(350)
        $now = $prior
        do {
            $now = Selected-Count $hwnd
            if ($now -ne $prior) { break }
            Start-Sleep -Milliseconds 15
        } while (([DateTime]::UtcNow) -lt $limit)
        if ($now -eq $prior) { $timeouts++; continue }
        if ($i -ge 5) { $samples.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds) }
    }
    $p95 = NearestRank @($samples)
    Write-Log ("select_space_n=$($samples.Count) timeouts=$timeouts nearest_rank_p95_ms=$(if ($null -eq $p95) {'NA'} else {[Math]::Round($p95, 3)})")
    if ($samples.Count -gt 0) { [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-select-space.txt'), ($samples -join "`n"), $utf8) }

    [void](Assert-Foreground $hwnd $targetThread)
    $before = 0
    if (Test-Path $trace) { $before = @(Get-Content $trace).Count }
    for ($i = 0; $i -lt 120; $i++) {
        [C11R10SelectFrames]::Tap(0x22)
        Start-Sleep -Milliseconds 16
    }
    Start-Sleep -Milliseconds 400
    $intervals = @()
    if (Test-Path $trace) {
        $stamps = @(Get-Content $trace | Select-Object -Skip $before | Where-Object { $_ -match '^\d+$' } | ForEach-Object { [int64]$_ })
        for ($i = 1; $i -lt $stamps.Count; $i++) {
            $delta = $stamps[$i] - $stamps[$i - 1]
            if ($delta -gt 0) { $intervals += [double]$delta }
        }
    }
    if ($intervals.Count -eq 0) {
        Write-Log 'FRAME_BURST_RESULT=NOT_RUN'
        Write-Log 'frame_reason=no Present intervals during PageDown burst without UIA polling'
    } else {
        $frameP95 = NearestRank $intervals
        $over = @($intervals | Where-Object { $_ -gt 16700 }).Count
        Write-Log ("frame_burst_n=$($intervals.Count) nearest_rank_p95_us=$([Math]::Round($frameP95, 3)) over_16_7ms=$over")
        if ($frameP95 -le 16700) { Write-Log 'FRAME_BURST_RESULT=PASS' } else { Write-Log 'FRAME_BURST_RESULT=NOT_PASS' }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-frame-burst-us.txt'), ($intervals -join "`n"), $utf8)
    }
} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    if ($null -ne $proc) { Write-Log "wm_close_exit=$(Stop-Owned $proc.Id $hwnd)" }
    Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r10-select-frames-log.txt'), $log, $utf8)
}
