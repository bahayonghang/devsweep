param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('history', 'ai', 'perf-scan')]
    [string]$Mode
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class C11R10Probe {
    public const uint KEYEVENTF_KEYUP = 2;
    public const uint WM_CLOSE = 0x0010;

    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr destination, uint flags);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    public static void AltUp() {
        keybd_event(0x12, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }

    public static void Tap(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }
}
'@
Add-Type -AssemblyName System.Drawing

$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$length = (Get-Item -LiteralPath $binary).Length
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) {
    $script:log.Add($line)
    Write-Host $line
}

function Stop-OwnedSpike([int]$procId, [IntPtr]$hwnd) {
    Write-Log "wm_close_begin procId=$procId hwnd=$($hwnd.ToInt64())"
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][C11R10Probe]::PostMessage($hwnd, [C11R10Probe]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        $alive = Get-Process -Id $procId -ErrorAction SilentlyContinue
        if (-not $alive) {
            Write-Log 'wm_close_exited=0'
            return 0
        }
        Start-Sleep -Milliseconds 200
    }
    Write-Log "WM_CLOSE_TIMEOUT procId=$procId"
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
    $alive = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if ($alive) { return 1 }
    return 1
}

Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Log "preexisting_spike=$($_.Id)"
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}

function New-IsolatedRoots([string]$name) {
    $root = Join-Path $env:TEMP $name
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
    $local = Join-Path $root 'local'
    $roaming = Join-Path $root 'roaming'
    New-Item -ItemType Directory -Path $local, $roaming | Out-Null
    [pscustomobject]@{ Root = $root; Local = $local; Roaming = $roaming }
}

function Start-Isolated([hashtable]$envMap) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $repo
    foreach ($key in @($envMap.Keys)) {
        if ($info.EnvironmentVariables.ContainsKey($key)) {
            $info.EnvironmentVariables[$key] = [string]$envMap[$key]
        } else {
            [void]$info.EnvironmentVariables.Add($key, [string]$envMap[$key])
        }
    }
    $proc = [System.Diagnostics.Process]::Start($info)
    $deadline = (Get-Date).AddSeconds(25)
    do {
        Start-Sleep -Milliseconds 200
        $proc.Refresh()
    } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
    if ($proc.HasExited -or $proc.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "window did not appear exit=$($proc.ExitCode)"
    }
    return $proc
}

function Set-ProbeForeground([IntPtr]$hwnd) {
    $current = [C11R10Probe]::GetCurrentThreadId()
    $foreground = [C11R10Probe]::GetForegroundWindow()
    [uint32]$foregroundOwner = 0
    $foregroundThread = [C11R10Probe]::GetWindowThreadProcessId($foreground, [ref]$foregroundOwner)
    [uint32]$targetOwner = 0
    $targetThread = [C11R10Probe]::GetWindowThreadProcessId($hwnd, [ref]$targetOwner)
    $attachForeground = [C11R10Probe]::AttachThreadInput($current, $foregroundThread, $true)
    $attachTarget = [C11R10Probe]::AttachThreadInput($current, $targetThread, $true)
    [C11R10Probe]::AltUp()
    $set = [C11R10Probe]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 300
    $now = [C11R10Probe]::GetForegroundWindow()
    $ok = ($now -eq $hwnd)
    [pscustomobject]@{
        Ok = $ok
        AttachForeground = $attachForeground
        AttachTarget = $attachTarget
        SetForeground = $set
        ForegroundHwnd = $now.ToInt64()
        TargetHwnd = $hwnd.ToInt64()
        CurrentThread = $current
        ForegroundThread = $foregroundThread
        TargetThread = $targetThread
        DetachForeground = $null
        DetachTarget = $null
        Current = $current
        ForegroundThreadId = $foregroundThread
        TargetThreadId = $targetThread
    }
}

function Clear-ProbeForeground($state) {
    [void][C11R10Probe]::AttachThreadInput($state.Current, $state.ForegroundThreadId, $false)
    [void][C11R10Probe]::AttachThreadInput($state.Current, $state.TargetThreadId, $false)
}

function Send-Tap([byte]$key) {
    [C11R10Probe]::Tap($key)
}

function Get-UiaText([IntPtr]$hwnd) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    $lines = New-Object System.Collections.Generic.List[string]
    $focusExposed = 0
    $nodes = 0
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $stack = New-Object System.Collections.Generic.Stack[object]
    $stack.Push(@($root, 0))
    while ($stack.Count -gt 0) {
        $item = $stack.Pop()
        $element = $item[0]
        $depth = [int]$item[1]
        if ($null -eq $element -or $depth -gt 8) { continue }
        $nodes++
        $name = ''
        $id = ''
        $type = ''
        try { $name = [string]$element.Current.Name } catch {}
        try { $id = [string]$element.Current.AutomationId } catch {}
        try { $type = [string]$element.Current.ControlType.ProgrammaticName } catch {}
        $pattern = $null
        $hasFocus = $false
        try {
            $hasFocus = $element.TryGetCurrentPattern([System.Windows.Automation.FocusPattern]::Pattern, [ref]$pattern)
        } catch {
            $hasFocus = $false
        }
        if ($hasFocus) { $focusExposed++ }
        $pad = '  ' * $depth
        $lines.Add("$pad$type | id=$id | name=$name | FocusPattern=$hasFocus")
        $child = $walker.GetLastChild($element)
        while ($null -ne $child) {
            $stack.Push(@($child, $depth + 1))
            $child = $walker.GetPreviousSibling($child)
        }
    }
    [pscustomobject]@{ Text = ($lines -join "`n"); FocusExposed = $focusExposed; Nodes = $nodes }
}

function Save-Window([IntPtr]$hwnd, [string]$path) {
    [C11R10Probe+RECT]$rectangle = New-Object C11R10Probe+RECT
    if (-not [C11R10Probe]::GetWindowRect($hwnd, [ref]$rectangle)) { return }
    $width = [Math]::Max(1, $rectangle.Right - $rectangle.Left)
    $height = [Math]::Max(1, $rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try { [void][C11R10Probe]::PrintWindow($hwnd, $device, 2) } finally {
        $graphics.ReleaseHdc($device)
        $graphics.Dispose()
    }
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "mode=$Mode"
Write-Log "binary=$binary"
Write-Log "binary_sha256=$hash length=$length"
Write-Log "rustc_note=PATH cargo is mbx; this binary was built with cargo 1.99.0, not an MSRV 1.88.0 pass"

$proc = $null
$state = $null
$exitCode = $null
try {
    if ($Mode -eq 'history') {
        $dirs = New-IsolatedRoots 'devsweep-gpui-c11-r10-history'
        $auditDir = Join-Path $dirs.Local 'DevSweep\audit\v1'
        New-Item -ItemType Directory -Path $auditDir | Out-Null
        $record = '{"record_kind":"execution_transition","schema_version":1,"domain":"clean","operation_id":"op-c11-r10-history","timestamp_epoch_ms":1,"transition":2,"outcome_code":"succeeded","error_code":null,"evidence":{"target_identity_sha256":"ab","action_kind":"move_to_trash","irreversible":false,"capacity_class":"verified","duration_ms":3,"estimated_bytes":1}}' + "`n"
        [System.IO.File]::WriteAllText((Join-Path $auditDir 'clean.jsonl'), $record, $utf8)
        Write-Log "audit=$auditDir\clean.jsonl"
        $proc = Start-Isolated @{
            LOCALAPPDATA = $dirs.Local
            APPDATA = $dirs.Roaming
        }
        Write-Log "procId=$($proc.Id) hwnd=$($proc.MainWindowHandle.ToInt64())"
        $state = Set-ProbeForeground $proc.MainWindowHandle
        Write-Log "foreground_ok=$($state.Ok) attach_foreground=$($state.AttachForeground) attach_target=$($state.AttachTarget) set=$($state.SetForeground) foreground_hwnd=$($state.ForegroundHwnd)"
        if (-not $state.Ok) { throw 'foreground_ok is false; keys were not sent' }
        Send-Tap 0x79
        Start-Sleep -Milliseconds 400
        Send-Tap 0x28
        Start-Sleep -Milliseconds 250
        Send-Tap 0x28
        Start-Sleep -Milliseconds 250
        Send-Tap 0x28
        Start-Sleep -Milliseconds 250
        Send-Tap 0x0D
        Start-Sleep -Milliseconds 700
        $dump = Get-UiaText $proc.MainWindowHandle
        $png = Join-Path $evidence 'c11-r10-history.png'
        Save-Window $proc.MainWindowHandle $png
        Write-Log "focus_pattern_exposed=$($dump.FocusExposed) nodes=$($dump.Nodes)"
        Write-Log "FOCUS_PATTERN=NOT_EXPOSED_BY_PINNED_ACCESSKIT"
        Write-Log "FOCUS_PATTERN_RESULT=NOT_PASS"
        Write-Log 'DUMP'
        Write-Log $dump.Text
        $joined = $dump.Text
        Write-Log ("has_history_title=" + ($joined -match 'name=History'))
        Write-Log ("has_operation=" + ($joined.Contains('clean | op-c11-r10-history | succeeded')))
        Write-Log ("has_no_replay=" + ($joined.Contains('cannot execute or replay')))
        Write-Log ("has_unavailable=" + ($joined.Contains('could not be read')))
        Write-Log ("has_empty=" + ($joined.Contains('No audit operations')))
    } elseif ($Mode -eq 'ai') {
        $aiHome = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-ai\profile\.codex'
        if (-not (Test-Path -LiteralPath $aiHome)) { throw "isolated AI home missing: $aiHome" }
        $profileRoot = Split-Path -Parent (Split-Path -Parent $aiHome)
        $dirs = New-IsolatedRoots 'devsweep-gpui-c11-r10-ai-app'
        Write-Log "explicit_home=$aiHome"
        Write-Log "overridden_userprofile=$profileRoot"
        $proc = Start-Isolated @{
            LOCALAPPDATA = $dirs.Local
            APPDATA = $dirs.Roaming
            USERPROFILE = $profileRoot
            'HOME' = $profileRoot
            DEVSWEEP_AI_REVIEW_HOME = $aiHome
        }
        Write-Log "procId=$($proc.Id) hwnd=$($proc.MainWindowHandle.ToInt64())"
        $state = Set-ProbeForeground $proc.MainWindowHandle
        Write-Log "foreground_ok=$($state.Ok) attach_foreground=$($state.AttachForeground) attach_target=$($state.AttachTarget) set=$($state.SetForeground) foreground_hwnd=$($state.ForegroundHwnd)"
        if (-not $state.Ok) { throw 'foreground_ok is false; keys were not sent' }
        Send-Tap 0x77
        Start-Sleep -Milliseconds 500
        $opened = Get-UiaText $proc.MainWindowHandle
        Write-Log 'DUMP opened'
        Write-Log $opened.Text
        Write-Log ("opened_without_home_line=" + (-not $opened.Text.Contains('AI home:')))
        Send-Tap 0x0D
        $review = $null
        $deadline = (Get-Date).AddSeconds(20)
        do {
            Start-Sleep -Milliseconds 300
            $review = Get-UiaText $proc.MainWindowHandle
        } while (-not $review.Text.Contains('AI home:') -and (Get-Date) -lt $deadline)
        Start-Sleep -Milliseconds 800
        $review = Get-UiaText $proc.MainWindowHandle
        $png = Join-Path $evidence 'c11-r10-ai.png'
        Save-Window $proc.MainWindowHandle $png
        Write-Log "focus_pattern_exposed=$($review.FocusExposed) nodes=$($review.Nodes)"
        Write-Log "FOCUS_PATTERN_RESULT=NOT_PASS"
        Write-Log 'DUMP review'
        Write-Log $review.Text
        $realNeedles = @('C:\Users\lyh\.codex', 'C:\Users\lyh\.claude', 'C:\Users\lyh\AppData\Roaming\Cursor')
        foreach ($needle in $realNeedles) {
            Write-Log ("contains_real[$needle]=" + $review.Text.Contains($needle))
        }
        Write-Log ("contains_explicit_home=" + $review.Text.Contains($aiHome))
        Write-Log ("contains_review_count=" + ($review.Text -match 'AI review:'))
    } else {
        throw "mode $Mode is not implemented in this runner yet"
    }
} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    if ($null -ne $state) { Clear-ProbeForeground $state }
    if ($null -ne $proc) {
        $hwnd = [IntPtr]::Zero
        try { $hwnd = $proc.MainWindowHandle } catch {}
        $exitCode = Stop-OwnedSpike $proc.Id $hwnd
        Write-Log "wm_close_exit=$exitCode"
    }
    $left = @(Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue)
    Write-Log "spike_remaining=$($left.Count)"
    Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
    $outName = if ($Mode -eq 'history') { 'c11-r10-history.txt' } elseif ($Mode -eq 'ai') { 'c11-r10-ai.txt' } else { "c11-r10-$Mode.txt" }
    [System.IO.File]::WriteAllLines((Join-Path $evidence $outName), $log, $utf8)
}
if ($left.Count -gt 0) { exit 2 }
