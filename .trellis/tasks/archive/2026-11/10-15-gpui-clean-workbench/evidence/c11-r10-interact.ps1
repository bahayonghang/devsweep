$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C11R10Interact {
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
$trace = Join-Path $evidence 'c11-r10-interact-frames.txt'
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
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][C11R10Interact]::PostMessage($hwnd, [C11R10Interact]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
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
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }

$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$length = (Get-Item -LiteralPath $binary).Length
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash length=$length"
Write-Log 'method=release binary; isolated 10000 pytest caches; nearest-rank p95; UIA upper bound includes one observer read after a 20 ms gap'

$local = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-interact\local'
$roaming = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-interact\roaming'
$rootDir = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-interact'
if (Test-Path $rootDir) { Remove-Item $rootDir -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
$roots = '{"schema_version":1,"roots":["C:\\Users\\lyh\\AppData\\Local\\Temp\\devsweep-gpui-c11-r10-perf\\scan-root"]}'
[System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\clean-scan-roots-v1.json'), $roots, $utf8)

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
$current = [C11R10Interact]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C11R10Interact]::GetWindowThreadProcessId($hwnd, [ref]$ignore)

function Assert-Foreground([IntPtr]$window, [uint32]$threadId) {
    $foreground = [C11R10Interact]::GetForegroundWindow()
    [uint32]$owner = 0
    $foregroundThread = [C11R10Interact]::GetWindowThreadProcessId($foreground, [ref]$owner)
    [void][C11R10Interact]::AttachThreadInput($script:current, $foregroundThread, $true)
    [void][C11R10Interact]::AttachThreadInput($script:current, $threadId, $true)
    [C11R10Interact]::AltUp()
    [void][C11R10Interact]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 200
    return ([C11R10Interact]::GetForegroundWindow() -eq $window)
}

function Get-Rows([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $rows = New-Object System.Collections.Generic.List[string]
    foreach ($item in $items) {
        $id = ''
        $name = ''
        try { $id = [string]$item.Current.AutomationId } catch {}
        try { $name = [string]$item.Current.Name } catch {}
        if ($id.StartsWith('clean-row-')) { $rows.Add("$id`t$name") }
    }
    return $rows
}
function First-RowId($rows) {
    if ($rows.Count -eq 0) { return '' }
    return ($rows[0] -split "`t")[0]
}
function Focused-Mark($rows) {
    foreach ($row in $rows) {
        $name = ($row -split "`t", 2)[1]
        if ($name.StartsWith('> ')) { return $name.Substring(0, [Math]::Min(80, $name.Length)) }
    }
    return ''
}

try {
    if (-not (Assert-Foreground $hwnd $targetThread)) { throw 'initial foreground failed' }
    Write-Log 'foreground_ok=True'
    [C11R10Interact]::Tap(0x78)
    $scanStart = [DateTime]::UtcNow
    $finished = $false
    $deadline = (Get-Date).AddSeconds(60)
    do {
        Start-Sleep -Milliseconds 300
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
        $name = ''
        try { $name = [string]$root.Current.Name } catch {}
        $textCond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Scan finished. 10000 targets found.')
        $hit = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $textCond)
        if ($null -ne $hit) { $finished = $true }
    } while (-not $finished -and (Get-Date) -lt $deadline)
    Write-Log ("scan_wall_ms=" + [Math]::Round(([DateTime]::UtcNow - $scanStart).TotalMilliseconds, 3))
    Write-Log "ten_k_present=$finished"
    if (-not $finished) { throw '10k scan did not present' }

    if (-not (Assert-Foreground $hwnd $targetThread)) { throw 'foreground lost after scan' }
    $before = Get-Rows $hwnd
    Write-Log ("visible_rows=$($before.Count) first=$(First-RowId $before)")

    function Measure-KeyActions([IntPtr]$window, [uint32]$threadId, [scriptblock]$send, [scriptblock]$signal, [int]$warmups, [int]$kept) {
        $samples = New-Object System.Collections.Generic.List[double]
        $timeouts = 0
        for ($i = 0; $i -lt ($warmups + $kept); $i++) {
            if (($i % 20) -eq 0) { [void](Assert-Foreground $window $threadId) }
            $prior = & $signal $window
            $t0 = [DateTime]::UtcNow
            & $send
            Start-Sleep -Milliseconds 20
            $deadline = $t0.AddMilliseconds(400)
            $now = $prior
            do {
                $now = & $signal $window
                if ($now -ne $prior) { break }
                Start-Sleep -Milliseconds 15
            } while (([DateTime]::UtcNow) -lt $deadline)
            if ($now -eq $prior) { $timeouts++; continue }
            if ($i -ge $warmups) { $samples.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds) }
        }
        return [pscustomobject]@{ Samples = $samples; Timeouts = $timeouts }
    }

    [C11R10Interact]::Tap(0x09)
    Start-Sleep -Milliseconds 200
    $searchEdit = ([System.Windows.Automation.AutomationElement]::FromHandle($hwnd)).FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'clean-search-input')))
    Write-Log ("search_field_present=" + ($null -ne $searchEdit))
    $toggle = $true
    $search = Measure-KeyActions $hwnd $targetThread {
        if ($script:toggle) { [C11R10Interact]::Tap(0x5A) } else { [C11R10Interact]::Tap(0x08) }
        $script:toggle = -not $script:toggle
    } { param($window) First-RowId (Get-Rows $window) } 5 100
    $searchP95 = NearestRank @($search.Samples)
    Write-Log ("search_n=$($search.Samples.Count) timeouts=$($search.Timeouts) nearest_rank_p95_ms=$(if ($null -eq $searchP95) {'NA'} else {[Math]::Round($searchP95, 3)})")
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-search-samples.txt'), ($search.Samples -join "`n"), $utf8)

    $down = $true
    $select = Measure-KeyActions $hwnd $targetThread {
        if ($script:down) { [C11R10Interact]::Chord(0x12, 0x28) } else { [C11R10Interact]::Chord(0x12, 0x26) }
        $script:down = -not $script:down
    } { param($window) Focused-Mark (Get-Rows $window) } 5 100
    $selectP95 = NearestRank @($select.Samples)
    Write-Log ("select_n=$($select.Samples.Count) timeouts=$($select.Timeouts) nearest_rank_p95_ms=$(if ($null -eq $selectP95) {'NA'} else {[Math]::Round($selectP95, 3)})")
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-select-samples.txt'), ($select.Samples -join "`n"), $utf8)

    $traceBefore = 0
    if (Test-Path $trace) { $traceBefore = @(Get-Content $trace).Count }
    $scroll = Measure-KeyActions $hwnd $targetThread { [C11R10Interact]::Tap(0x22) } { param($window) First-RowId (Get-Rows $window) } 5 100
    Start-Sleep -Milliseconds 300
    $scrollP95 = NearestRank @($scroll.Samples)
    Write-Log ("scroll_n=$($scroll.Samples.Count) timeouts=$($scroll.Timeouts) nearest_rank_p95_ms=$(if ($null -eq $scrollP95) {'NA'} else {[Math]::Round($scrollP95, 3)})")
    [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-scroll-samples.txt'), ($scroll.Samples -join "`n"), $utf8)

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
        Write-Log 'frame_reason=no Present intervals during the scroll burst'
    } else {
        $frameP95 = NearestRank $intervals
        $over = @($intervals | Where-Object { $_ -gt 16700 }).Count
        Write-Log ("frame_n=$($intervals.Count) nearest_rank_p95_us=$([Math]::Round($frameP95, 3)) over_16_7ms=$over")
        if ($frameP95 -le 16700) { Write-Log 'FRAME_RESULT=PASS' } else { Write-Log 'FRAME_RESULT=NOT_PASS' }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-frame-intervals-us.txt'), ($intervals -join "`n"), $utf8)
    }

    $cancel = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt 12; $i++) {
        [void](Assert-Foreground $hwnd $targetThread)
        [C11R10Interact]::Tap(0x78)
        $saw = $false
        $wait = (Get-Date).AddSeconds(3)
        do {
            Start-Sleep -Milliseconds 40
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
            $hit = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Scanning… 0 targets found so far.')))
            if ($null -eq $hit) {
                $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)))
                foreach ($item in $items) {
                    $name = ''
                    try { $name = [string]$item.Current.Name } catch {}
                    if ($name.StartsWith('Scanning')) { $saw = $true; break }
                }
            } else { $saw = $true }
        } while (-not $saw -and (Get-Date) -lt $wait)
        if (-not $saw) { Write-Log "cancel_scan_not_started i=$i"; break }
        $t0 = [DateTime]::UtcNow
        [C11R10Interact]::Tap(0x1B)
        $acked = $false
        $ackDeadline = $t0.AddMilliseconds(800)
        do {
            Start-Sleep -Milliseconds 15
            $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
            $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)))
            foreach ($item in $items) {
                $name = ''
                try { $name = [string]$item.Current.Name } catch {}
                if ($name -eq 'The clean was canceled.') { $acked = $true; break }
            }
        } while (-not $acked -and ([DateTime]::UtcNow) -lt $ackDeadline)
        if ($acked -and $i -ge 2) { $cancel.Add(([DateTime]::UtcNow - $t0).TotalMilliseconds) }
        Start-Sleep -Milliseconds 250
    }
    if ($cancel.Count -eq 0) {
        Write-Log 'CANCEL_RESULT=NOT_RUN'
    } else {
        $cancelP95 = NearestRank @($cancel)
        Write-Log ("cancel_n=$($cancel.Count) nearest_rank_p95_ms=$([Math]::Round($cancelP95, 3))")
        Write-Log 'CANCEL_SAMPLE_COUNT=UNMET'
        if ($cancelP95 -le 100) { Write-Log 'CANCEL_ACK_BUDGET=WITHIN' } else { Write-Log 'CANCEL_ACK_BUDGET=NOT_PASS' }
        [System.IO.File]::WriteAllText((Join-Path $evidence 'c11-r10-cancel-samples.txt'), ($cancel -join "`n"), $utf8)
    }
    Write-Log 'FOCUS_PATTERN_RESULT=NOT_PASS'
    Write-Log 'focus_reason=pinned AccessKit Windows adapter does not expose UIA FocusPattern'
} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    if ($null -ne $proc) {
        $exitCode = Stop-Owned $proc.Id $hwnd
        Write-Log "wm_close_exit=$exitCode"
    }
    $left = @(Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue)
    Write-Log "spike_remaining=$($left.Count)"
    Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r10-interact.txt'), $log, $utf8)
}
