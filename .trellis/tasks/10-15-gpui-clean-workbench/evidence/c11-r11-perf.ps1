$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class C11R11Perf {
    public const uint KEYEVENTF_KEYUP = 2;
    public const uint WM_CLOSE = 0x0010;

    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);

    public static void AltUp() { keybd_event(0x12, 0, KEYEVENTF_KEYUP, UIntPtr.Zero); }
    public static void Tap(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }
    public static void Chord(byte modifier, byte key) {
        keybd_event(modifier, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
        keybd_event(modifier, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }
}
'@

$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }

function NearestRankP95([double[]]$samples) {
    if ($samples.Count -eq 0) { return $null }
    $sorted = @($samples | Sort-Object)
    $rank = [Math]::Ceiling(0.95 * $sorted.Count)
    if ($rank -lt 1) { $rank = 1 }
    return [double]$sorted[$rank - 1]
}

function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][C11R11Perf]::PostMessage($hwnd, [C11R11Perf]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Write-Log "WM_CLOSE_TIMEOUT procId=$procId"
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}

Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object {
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}

$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$length = (Get-Item -LiteralPath $binary).Length
$fixture = 'C:\Users\lyh\AppData\Local\Temp\devsweep-gpui-c11-r11-perf\scan-root'
$trace = Join-Path $evidence 'c11-r11-frames.txt'
if (Test-Path -LiteralPath $trace) { Remove-Item -LiteralPath $trace -Force }
$dirsRoot = Join-Path $env:TEMP 'devsweep-gpui-c11-r11-perf-app'
if (Test-Path -LiteralPath $dirsRoot) { Remove-Item -LiteralPath $dirsRoot -Recurse -Force }
$local = Join-Path $dirsRoot 'local'
$roaming = Join-Path $dirsRoot 'roaming'
New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
$rootsJson = '{"schema_version":1,"roots":["C:\\Users\\lyh\\AppData\\Local\\Temp\\devsweep-gpui-c11-r11-perf\\scan-root"]}'
[System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\clean-scan-roots-v1.json'), $rootsJson, $utf8)

Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary=$binary"
Write-Log "binary_sha256=$hash length=$length"
Write-Log 'rustc_note=cargo 1.99.0 via mbx; not an MSRV 1.88.0 pass'
Write-Log "fixture=$fixture"
Write-Log "frame_trace=$trace"

$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $repo
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$info.EnvironmentVariables['DEVSWEEP_FRAME_TRACE'] = $trace
$proc = [System.Diagnostics.Process]::Start($info)
$deadline = (Get-Date).AddSeconds(25)
do {
    Start-Sleep -Milliseconds 200
    $proc.Refresh()
} while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
if ($proc.HasExited -or $proc.MainWindowHandle -eq [IntPtr]::Zero) { throw 'window did not appear' }
$hwnd = $proc.MainWindowHandle
Write-Log "procId=$($proc.Id) hwnd=$($hwnd.ToInt64())"

$current = [C11R11Perf]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C11R11Perf]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
$foregroundOk = $false
$attachForeground = $false
$attachTarget = $false
$set = $false
$foregroundThread = 0
for ($attempt = 1; $attempt -le 5 -and -not $foregroundOk; $attempt++) {
    $foreground = [C11R11Perf]::GetForegroundWindow()
    $foregroundThread = [C11R11Perf]::GetWindowThreadProcessId($foreground, [ref]$ignore)
    $attachForeground = [C11R11Perf]::AttachThreadInput($current, $foregroundThread, $true)
    $attachTarget = [C11R11Perf]::AttachThreadInput($current, $targetThread, $true)
    [C11R11Perf]::AltUp()
    $set = [C11R11Perf]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 400
    $now = [C11R11Perf]::GetForegroundWindow()
    $foregroundOk = ($now -eq $hwnd)
    Write-Log "foreground_attempt=$attempt ok=$foregroundOk attach_foreground=$attachForeground attach_target=$attachTarget set=$set now=$($now.ToInt64())"
}
Write-Log "foreground_ok=$foregroundOk"
if (-not $foregroundOk) {
    Write-Log 'keys_sent=false'
    $exitCode = Stop-Owned $proc.Id $hwnd
    Write-Log "wm_close_exit=$exitCode"
    $left = @(Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue)
    Write-Log "spike_remaining=$($left.Count)"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r11-perf.txt'), $log, $utf8)
    exit 3
}

function Get-TreeText([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $lines = New-Object System.Collections.Generic.List[string]
    $stack = New-Object System.Collections.Generic.Stack[object]
    $stack.Push(@($root, 0))
    $focusExposed = 0
    while ($stack.Count -gt 0) {
        $item = $stack.Pop()
        $element = $item[0]
        $depth = [int]$item[1]
        if ($null -eq $element -or $depth -gt 7) { continue }
        $name = ''; $id = ''
        try { $name = [string]$element.Current.Name } catch {}
        try { $id = [string]$element.Current.AutomationId } catch {}
        $pattern = $null
        $has = $false
        try { $has = $element.TryGetCurrentPattern([System.Windows.Automation.FocusPattern]::Pattern, [ref]$pattern) } catch {}
        if ($has) { $focusExposed++ }
        $lines.Add(('  ' * $depth) + "$id | $name")
        try { $child = $walker.GetLastChild($element) } catch { $child = $null }
        while ($null -ne $child) {
            $stack.Push(@($child, $depth + 1))
            try { $child = $walker.GetPreviousSibling($child) } catch { $child = $null }
        }
    }
    [pscustomobject]@{ Text = ($lines -join "`n"); FocusExposed = $focusExposed }
}

function Wait-Text([IntPtr]$window, [string]$needle, [int]$seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    do {
        $dump = Get-TreeText $window
        if ($dump.Text.Contains($needle)) { return $dump }
        Start-Sleep -Milliseconds 200
    } while ((Get-Date) -lt $deadline)
    return (Get-TreeText $window)
}

try {
[C11R11Perf]::Tap(0x78) # F9
$scanStarted = [DateTime]::UtcNow
$found = Wait-Text $hwnd 'Scan finished.' 180
$scanMs = ([DateTime]::UtcNow - $scanStarted).TotalMilliseconds
Write-Log ("scan_wall_ms=" + [Math]::Round($scanMs, 3))
$finished = [regex]::Match($found.Text, 'Scan finished\. (\d+) targets found\.')
$rowCount = if ($finished.Success) { [int]$finished.Groups[1].Value } else { -1 }
Write-Log "row_count=$rowCount"
Write-Log "focus_pattern_exposed=$($found.FocusExposed)"
Write-Log 'FOCUS_PATTERN_RESULT=NOT_PASS'
if ($rowCount -ne 10000) {
    Write-Log 'TEN_K_ROWS=NOT_RUN'
    Write-Log 'reason=release scan did not present 10000 targets; interaction samples were not taken'
    Write-Log 'DUMP'
    Write-Log $found.Text
} else {
    Write-Log 'TEN_K_ROWS=PRESENT'
    function Poll-Baseline([IntPtr]$window) {
        $samples = New-Object System.Collections.Generic.List[double]
        for ($i = 0; $i -lt 20; $i++) {
            $t0 = [DateTime]::UtcNow
            [void](Get-TreeText $window)
            $samples.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds)
        }
        return $samples
    }
    $baseline = Poll-Baseline $hwnd
    $baselineP95 = NearestRankP95 @($baseline)
    Write-Log ("uia_poll_n=20 nearest_rank_p95_ms=" + [Math]::Round($baselineP95, 3))
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-uia-baseline.txt'), ($baseline -join "`n"), $utf8)

    function Measure-Actions([IntPtr]$window, [scriptblock]$send, [scriptblock]$changed, [int]$warmups, [int]$kept) {
        $samples = New-Object System.Collections.Generic.List[double]
        $total = $warmups + $kept
        for ($i = 0; $i -lt $total; $i++) {
            $before = & $changed $window
            $t0 = [DateTime]::UtcNow
            & $send
            $deadline = $t0.AddMilliseconds(500)
            do {
                $after = & $changed $window
                if ($after -ne $before) { break }
                Start-Sleep -Milliseconds 5
            } while (([DateTime]::UtcNow) -lt $deadline)
            $elapsed = ([DateTime]::UtcNow - $t0).TotalMilliseconds
            $changedNow = $after -ne $before
            if ($i -ge $warmups -and $changedNow) { $samples.Add($elapsed) }
            if (-not $changedNow) { Write-Log "action_timeout i=$i" }
        }
        return $samples
    }

    # Tab moves from the Note into the search field. Then alternate a digit and backspace.
    [C11R11Perf]::Tap(0x09)
    Start-Sleep -Milliseconds 250
    $searchKey = 0x39
    $search = Measure-Actions $hwnd {
        [C11R11Perf]::Tap($searchKey)
        if ($searchKey -eq 0x39) { $script:searchKey = 0x08 } else { $script:searchKey = 0x39 }
    } {
        param($window)
        return (Get-TreeText $window).Text
    } 5 100
    $searchAll = @($search)
    $searchP95 = NearestRankP95 $searchAll
    Write-Log ("search_n=$($searchAll.Count) nearest_rank_p95_ms=" + [Math]::Round($searchP95, 3))
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-search-samples.txt'), ($searchAll -join "`n"), $utf8)

    $selectDown = $true
    $select = Measure-Actions $hwnd {
        if ($selectDown) { [C11R11Perf]::Chord(0x12, 0x28) } else { [C11R11Perf]::Chord(0x12, 0x26) }
        $script:selectDown = -not $script:selectDown
    } {
        param($window)
        return (Get-TreeText $window).Text
    } 5 100
    $selectP95 = NearestRankP95 @($select)
    Write-Log ("select_n=$($select.Count) nearest_rank_p95_ms=" + [Math]::Round($selectP95, 3))
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-select-samples.txt'), ($select -join "`n"), $utf8)

    $traceBefore = 0
    if (Test-Path -LiteralPath $trace) { $traceBefore = @(Get-Content -LiteralPath $trace).Count }
    $scroll = Measure-Actions $hwnd { [C11R11Perf]::Tap(0x22) } {
        param($window)
        return (Get-TreeText $window).Text
    } 5 100
    Start-Sleep -Milliseconds 400
    $scrollP95 = NearestRankP95 @($scroll)
    Write-Log ("scroll_n=$($scroll.Count) nearest_rank_p95_ms=" + [Math]::Round($scrollP95, 3))
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-scroll-samples.txt'), ($scroll -join "`n"), $utf8)

    $intervals = @()
    if (Test-Path -LiteralPath $trace) {
        $lines = @(Get-Content -LiteralPath $trace | Select-Object -Skip $traceBefore)
        $stamps = @($lines | Where-Object { $_ -match '^\d+$' } | ForEach-Object { [int64]$_ })
        for ($i = 1; $i -lt $stamps.Count; $i++) {
            $delta = $stamps[$i] - $stamps[$i - 1]
            if ($delta -gt 0) { $intervals += [double]$delta }
        }
    }
    if ($intervals.Count -eq 0) {
        Write-Log 'FRAME_RESULT=NOT_RUN'
        Write-Log 'frame_reason=DEVSWEEP_FRAME_TRACE produced no scroll intervals'
    } else {
        $frameP95 = NearestRankP95 $intervals
        $over = @($intervals | Where-Object { $_ -gt 16700 }).Count
        Write-Log ("frame_n=$($intervals.Count) nearest_rank_p95_us=" + [Math]::Round($frameP95, 3) + " over_16_7ms=$over")
        if ($frameP95 -le 16700) { Write-Log 'FRAME_RESULT=PASS' } else { Write-Log 'FRAME_RESULT=NOT_PASS' }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-frame-intervals-us.txt'), ($intervals -join "`n"), $utf8)
    }

    $cancel = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt 105; $i++) {
        [C11R11Perf]::Tap(0x78)
        $sawScan = Wait-Text $hwnd 'Scanning' 8
        if (-not $sawScan.Text.Contains('Scanning')) { Write-Log "cancel_scan_not_started i=$i"; break }
        $t0 = [DateTime]::UtcNow
        [C11R11Perf]::Tap(0x1B)
        $ack = Wait-Text $hwnd 'The clean was canceled.' 5
        $elapsed = ([DateTime]::UtcNow - $t0).TotalMilliseconds
        if ($i -ge 5 -and $ack.Text.Contains('The clean was canceled.')) { $cancel.Add($elapsed) }
        Start-Sleep -Milliseconds 200
    }
    if ($cancel.Count -eq 0) {
        Write-Log 'CANCEL_RESULT=NOT_RUN'
    } else {
        $cancelP95 = NearestRankP95 @($cancel)
        Write-Log ("cancel_n=$($cancel.Count) nearest_rank_p95_ms=" + [Math]::Round($cancelP95, 3))
        if ($cancel.Count -lt 100) { Write-Log 'CANCEL_SAMPLE_COUNT=UNMET' }
        if ($cancelP95 -le 100) { Write-Log 'CANCEL_ACK_BUDGET=WITHIN' } else { Write-Log 'CANCEL_ACK_BUDGET=NOT_PASS' }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r11-cancel-samples.txt'), ($cancel -join "`n"), $utf8)
    }

    $interaction = @($searchAll + $select + $scroll)
    $interactionP95 = NearestRankP95 $interaction
    Write-Log ("interaction_n=$($interaction.Count) nearest_rank_p95_ms=" + [Math]::Round($interactionP95, 3))
    if ($baselineP95 -gt 80) {
        Write-Log 'INTERACTION_RESULT=UNVERIFIED'
        Write-Log 'interaction_reason=UIA observer poll p95 is too coarse for the 100 ms budget'
    } elseif ($interactionP95 -le 100) {
        Write-Log 'INTERACTION_RESULT=PASS'
    } else {
        Write-Log 'INTERACTION_RESULT=NOT_PASS'
    }
}

} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    try { [void][C11R11Perf]::AttachThreadInput($current, $foregroundThread, $false) } catch {}
    try { [void][C11R11Perf]::AttachThreadInput($current, $targetThread, $false) } catch {}
    if ($null -ne $proc) {
        $exitCode = Stop-Owned $proc.Id $hwnd
        Write-Log "wm_close_exit=$exitCode"
    }
    $left = @(Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue)
    Write-Log "spike_remaining=$($left.Count)"
    Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r11-perf.txt'), $log, $utf8)
    if ($left.Count -gt 0) { exit 2 }
}
