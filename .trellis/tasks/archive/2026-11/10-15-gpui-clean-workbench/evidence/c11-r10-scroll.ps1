$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C11R10Scroll {
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
    public static void Chord(byte modifier, byte key) {
        keybd_event(modifier, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
        keybd_event(modifier, 0, KEYUP, UIntPtr.Zero);
    }
}
'@
$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$trace = Join-Path $evidence 'c11-r10-scroll-frames.txt'
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
    if ($hwnd -ne [IntPtr]::Zero) { [void][C11R10Scroll]::PostMessage($hwnd, [C11R10Scroll]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Write-Log "WM_CLOSE_TIMEOUT procId=$procId"
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash"
Write-Log 'note=select and scroll are measured while the Note still has focus, before Tab'

$local = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-scroll\local'
$roaming = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-scroll\roaming'
$appRoot = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-scroll'
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
$current = [C11R10Scroll]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C11R10Scroll]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
function Assert-Foreground([IntPtr]$window, [uint32]$threadId) {
    $foreground = [C11R10Scroll]::GetForegroundWindow()
    [uint32]$owner = 0
    $foregroundThread = [C11R10Scroll]::GetWindowThreadProcessId($foreground, [ref]$owner)
    [void][C11R10Scroll]::AttachThreadInput($script:current, $foregroundThread, $true)
    [void][C11R10Scroll]::AttachThreadInput($script:current, $threadId, $true)
    [C11R10Scroll]::AltUp()
    [void][C11R10Scroll]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 150
    return ([C11R10Scroll]::GetForegroundWindow() -eq $window)
}
function Get-Names([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($item in $items) {
        try { $names.Add([string]$item.Current.Name) } catch {}
    }
    return $names
}
function Row-Set([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $ids = New-Object System.Collections.Generic.List[string]
    foreach ($item in $items) {
        $id = ''
        try { $id = [string]$item.Current.AutomationId } catch {}
        if ($id.StartsWith('clean-row-')) { $ids.Add($id) }
    }
    return (($ids | Sort-Object) -join '|')
}
function Focus-Mark([IntPtr]$window) {
    foreach ($name in (Get-Names $window)) {
        if ($name.StartsWith('> ')) { return $name.Substring(0, [Math]::Min(60, $name.Length)) }
    }
    return ''
}
try {
    if (-not (Assert-Foreground $hwnd $targetThread)) { throw 'foreground failed' }
    Write-Log 'foreground_ok=True'
    $cancel = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt 8; $i++) {
        [void](Assert-Foreground $hwnd $targetThread)
        [C11R10Scroll]::Tap(0x78)
        $saw = $false
        $wait = (Get-Date).AddSeconds(2)
        do {
            Start-Sleep -Milliseconds 30
            foreach ($name in (Get-Names $hwnd)) {
                if ($name.StartsWith('Scanning')) { $saw = $true; break }
            }
        } while (-not $saw -and (Get-Date) -lt $wait)
        if (-not $saw) { Write-Log "cancel_scan_not_started i=$i"; break }
        $t0 = [DateTime]::UtcNow
        [C11R10Scroll]::Tap(0x1B)
        $acked = $false
        $ackDeadline = $t0.AddMilliseconds(700)
        do {
            Start-Sleep -Milliseconds 15
            foreach ($name in (Get-Names $hwnd)) {
                if ($name -like '*canceled*') { $acked = $true; break }
            }
        } while (-not $acked -and ([DateTime]::UtcNow) -lt $ackDeadline)
        if ($acked -and $i -ge 2) { $cancel.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds) }
        $settle = (Get-Date).AddSeconds(8)
        do { Start-Sleep -Milliseconds 80 } while (-not ((Get-Names $hwnd) -join "`n").Contains('canceled') -and -not ((Get-Names $hwnd) -join "`n").Contains('Scan finished') -and (Get-Date) -lt $settle)
        Start-Sleep -Milliseconds 200
    }
    if ($cancel.Count -eq 0) { Write-Log 'CANCEL_RESULT=NOT_RUN' } else {
        $cancelP95 = NearestRank @($cancel)
        Write-Log ("cancel_n=$($cancel.Count) nearest_rank_p95_ms=$([Math]::Round($cancelP95, 3))")
        Write-Log 'CANCEL_SAMPLE_COUNT=UNMET'
        if ($cancelP95 -le 100) { Write-Log 'CANCEL_ACK_BUDGET=WITHIN' } else { Write-Log 'CANCEL_ACK_BUDGET=NOT_PASS' }
    }

    [void](Assert-Foreground $hwnd $targetThread)
    [C11R10Scroll]::Tap(0x78)
    $scanStart = [DateTime]::UtcNow
    $finished = $false
    $deadline = (Get-Date).AddSeconds(40)
    do {
        Start-Sleep -Milliseconds 250
        if (((Get-Names $hwnd) -join "`n").Contains('Scan finished. 10000 targets found.')) { $finished = $true }
    } while (-not $finished -and (Get-Date) -lt $deadline)
    Write-Log ("scan_wall_ms=" + [Math]::Round(([DateTime]::UtcNow - $scanStart).TotalMilliseconds, 3))
    Write-Log "ten_k_present=$finished"
    if (-not $finished) { throw '10k rows were not presented' }
    [void](Assert-Foreground $hwnd $targetThread)

    function Measure-KeyActions([scriptblock]$send, [scriptblock]$signal, [int]$warmups, [int]$kept) {
        $samples = New-Object System.Collections.Generic.List[double]
        $timeouts = 0
        for ($i = 0; $i -lt ($warmups + $kept); $i++) {
            if (($i % 25) -eq 0) { [void](Assert-Foreground $hwnd $targetThread) }
            $prior = & $signal
            $t0 = [DateTime]::UtcNow
            & $send
            Start-Sleep -Milliseconds 20
            $limit = $t0.AddMilliseconds(350)
            $now = $prior
            do {
                $now = & $signal
                if ($now -ne $prior -and $now -ne '') { break }
                Start-Sleep -Milliseconds 15
            } while (([DateTime]::UtcNow) -lt $limit)
            if ($now -eq $prior -or $now -eq '') { $script:timeoutsSeen++; $timeouts++; continue }
            if ($i -ge $warmups) { $samples.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds) }
        }
        return [pscustomobject]@{ Samples = $samples; Timeouts = $timeouts }
    }

    $down = $true
    $select = Measure-KeyActions {
        if ($script:down) { [C11R10Scroll]::Chord(0x12, 0x28) } else { [C11R10Scroll]::Chord(0x12, 0x26) }
        $script:down = -not $script:down
    } { Focus-Mark $hwnd } 5 100
    $selectP95 = NearestRank @($select.Samples)
    Write-Log ("select_n=$($select.Samples.Count) timeouts=$($select.Timeouts) nearest_rank_p95_ms=$(if ($null -eq $selectP95) {'NA'} else {[Math]::Round($selectP95, 3)})")

    $traceBefore = 0
    if (Test-Path $trace) { $traceBefore = @(Get-Content $trace).Count }
    $scroll = Measure-KeyActions { [C11R10Scroll]::Tap(0x22) } { Row-Set $hwnd } 5 100
    Start-Sleep -Milliseconds 250
    $scrollP95 = NearestRank @($scroll.Samples)
    Write-Log ("scroll_n=$($scroll.Samples.Count) timeouts=$($scroll.Timeouts) nearest_rank_p95_ms=$(if ($null -eq $scrollP95) {'NA'} else {[Math]::Round($scrollP95, 3)})")
    if ($scroll.Samples.Count -eq 0) {
        Write-Log 'FRAME_RESULT=NOT_RUN'
        Write-Log 'frame_reason=PageDown did not change the visible row set, so Present intervals are not a scroll sample'
    } else {
        $intervals = @()
        if (Test-Path $trace) {
            $stamps = @(Get-Content $trace | Select-Object -Skip $traceBefore | Where-Object { $_ -match '^\d+$' } | ForEach-Object { [int64]$_ })
            for ($i = 1; $i -lt $stamps.Count; $i++) {
                $delta = $stamps[$i] - $stamps[$i - 1]
                if ($delta -gt 0) { $intervals += [double]$delta }
            }
        }
        if ($intervals.Count -eq 0) {
            Write-Log 'FRAME_RESULT=NOT_RUN'
        } else {
            $frameP95 = NearestRank $intervals
            $over = @($intervals | Where-Object { $_ -gt 16700 }).Count
            Write-Log ("frame_n=$($intervals.Count) nearest_rank_p95_us=$([Math]::Round($frameP95, 3)) over_16_7ms=$over")
            if ($frameP95 -le 16700) { Write-Log 'FRAME_RESULT=PASS' } else { Write-Log 'FRAME_RESULT=NOT_PASS' }
            [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-scroll-intervals-us.txt'), ($intervals -join "`n"), $utf8)
        }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-scroll-latency.txt'), ($scroll.Samples -join "`n"), $utf8)
    }
    if ($select.Samples.Count -gt 0) {
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-select-latency.txt'), ($select.Samples -join "`n"), $utf8)
    }
} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    if ($null -ne $proc) { Write-Log "wm_close_exit=$(Stop-Owned $proc.Id $hwnd)" }
    Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
    Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r10-scroll.txt'), $log, $utf8)
}
