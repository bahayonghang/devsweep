# Keyboard focus probe for the GPUI release binary.
#
# UI Automation has no "Focus" control pattern. Keyboard focus is read from
# AutomationElement.FocusedElement and the HasKeyboardFocus property. This
# probe presses Tab and Shift+Tab and records what UIA reports as focused.
# It reads only. It starts no scan, cleanup, uninstall or optimization.
#
# Usage, from the repository root:
#   powershell -ExecutionPolicy Bypass -File .trellis\tasks\10-15-gpui-desktop-rewrite\evidence\windows-focus-probe.ps1 -Round r11
param(
    [string]$Round = 'r11',
    [int]$Tabs = 12
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class FocusProbe {
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

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
$evidence = $PSScriptRoot
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$out = Join-Path $evidence "$Round-focus-probe.txt"
$utf8 = New-Object System.Text.UTF8Encoding $false
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }

if (-not (Test-Path -LiteralPath $binary)) { throw "release binary missing: $binary" }
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }

Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash) length=$((Get-Item -LiteralPath $binary).Length)"
Write-Log 'method=AutomationElement.FocusedElement + HasKeyboardFocus after each key; no control pattern is queried'

$appRoot = Join-Path $env:TEMP "devsweep-gpui-$Round-focus"
if (Test-Path $appRoot) { Remove-Item $appRoot -Recurse -Force }
$local = Join-Path $appRoot 'local'
$roaming = Join-Path $appRoot 'roaming'
New-Item -ItemType Directory -Path $local, $roaming | Out-Null

$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $repo
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$proc = [System.Diagnostics.Process]::Start($info)
$procId = $proc.Id
$deadline = (Get-Date).AddSeconds(25)
do { Start-Sleep -Milliseconds 200; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
$hwnd = $proc.MainWindowHandle
if ($hwnd -eq [IntPtr]::Zero) { Write-Log 'window=missing'; [System.IO.File]::WriteAllLines($out, $log, $utf8); exit 1 }

$current = [FocusProbe]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [FocusProbe]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
function Assert-Foreground {
    $foreground = [FocusProbe]::GetForegroundWindow()
    [uint32]$owner = 0
    $foregroundThread = [FocusProbe]::GetWindowThreadProcessId($foreground, [ref]$owner)
    [void][FocusProbe]::AttachThreadInput($script:current, $foregroundThread, $true)
    [void][FocusProbe]::AttachThreadInput($script:current, $script:targetThread, $true)
    [FocusProbe]::AltUp()
    [void][FocusProbe]::SetForegroundWindow($script:hwnd)
    Start-Sleep -Milliseconds 150
    return ([FocusProbe]::GetForegroundWindow() -eq $script:hwnd)
}
function Read-Focus([string]$step) {
    $name = ''; $id = ''; $type = ''; $has = ''; $owner = ''
    try {
        $element = [System.Windows.Automation.AutomationElement]::FocusedElement
        try { $name = [string]$element.Current.Name } catch {}
        try { $id = [string]$element.Current.AutomationId } catch {}
        try { $type = [string]$element.Current.ControlType.ProgrammaticName } catch {}
        try { $has = [string]$element.Current.HasKeyboardFocus } catch {}
        try { $owner = [string]$element.Current.ProcessId } catch {}
    } catch { $name = "ERROR $($_.Exception.Message)" }
    $ours = ($owner -eq [string]$script:procId)
    if ($name.Length -gt 80) { $name = $name.Substring(0, 80) }
    Write-Log "step=$step ours=$ours has_keyboard_focus=$has type=$type id=$id name=$name"
    return [pscustomobject]@{ Ours = $ours; Id = $id; Has = $has }
}

$seen = New-Object System.Collections.Generic.HashSet[string]
$oursCount = 0
try {
    $fg = Assert-Foreground
    Write-Log "foreground_ok=$fg"
    if (-not $fg) { throw 'foreground failed' }
    [void](Read-Focus 'start')
    for ($i = 1; $i -le $Tabs; $i++) {
        [FocusProbe]::Tap(0x09)
        Start-Sleep -Milliseconds 200
        $focus = Read-Focus "tab-$i"
        if ($focus.Ours -and $focus.Has -eq 'True') { $oursCount++; [void]$seen.Add($focus.Id) }
    }
    [FocusProbe]::Chord(0x10, 0x09)
    Start-Sleep -Milliseconds 200
    [void](Read-Focus 'shift-tab')
    Write-Log "tab_steps=$Tabs focused_in_app_with_keyboard_focus=$oursCount distinct_ids=$($seen.Count)"
    if ($oursCount -gt 0 -and $seen.Count -gt 1) {
        Write-Log 'FOCUS_TRAVERSAL=OBSERVED'
    } else {
        Write-Log 'FOCUS_TRAVERSAL=NOT_OBSERVED'
    }
} catch {
    Write-Log "ERROR=$($_.Exception.Message)"
} finally {
    try { [void][FocusProbe]::AttachThreadInput($current, $targetThread, $false) } catch {}
    [void][FocusProbe]::PostMessage($hwnd, [FocusProbe]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    $closed = $false
    $wait = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $wait) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { $closed = $true; break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $closed) { Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue }
    Write-Log "wm_close_clean=$closed"
    Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
    [System.IO.File]::WriteAllLines($out, $log, $utf8)
}
