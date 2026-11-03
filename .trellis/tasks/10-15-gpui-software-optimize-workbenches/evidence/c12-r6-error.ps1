$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C12R6 {
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
    public static void Chord(byte modifier, byte key) {
        keybd_event(modifier, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
        keybd_event(modifier, 0, KEYUP, UIntPtr.Zero);
    }
    public static void Tap(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
    }
}
'@
$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-software-optimize-workbenches\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }
function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) { [void][C12R6]::PostMessage($hwnd, [C12R6]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    return 1
}
function Get-Named([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($item in $all) {
        try { $name = [string]$item.Current.Name; if ($name) { $names.Add($name) } } catch {}
    }
    return $names
}
function Focus-Window([IntPtr]$window) {
    $current = [C12R6]::GetCurrentThreadId()
    [uint32]$ignore = 0
    $target = [C12R6]::GetWindowThreadProcessId($window, [ref]$ignore)
    $front = [C12R6]::GetForegroundWindow()
    $frontThread = [C12R6]::GetWindowThreadProcessId($front, [ref]$ignore)
    [void][C12R6]::AttachThreadInput($current, $frontThread, $true)
    [void][C12R6]::AttachThreadInput($current, $target, $true)
    [C12R6]::AltUp()
    [void][C12R6]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 300
    return ([C12R6]::GetForegroundWindow() -eq $window)
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
Write-Log "binary_sha256=$hash length=$((Get-Item $binary).Length)"
$root = Join-Path $env:TEMP 'devsweep-gpui-c12r6-error'
if (Test-Path $root) { Remove-Item $root -Recurse -Force -ErrorAction SilentlyContinue }
$local = Join-Path $root 'local'
$roaming = Join-Path $root 'roaming'
New-Item -ItemType Directory -Path $local, $roaming -Force | Out-Null
$hive = Join-Path $root 'hive.json'
$json = '{"version":1,"entries":[{"name":"ordinary","location":"current_user_run","class":"ordinary","run_value":[67]},{"name":"guard","location":"current_user_run","class":"security","run_value":[67]}]}'
[System.IO.File]::WriteAllText($hive, $json, $utf8)
$before = (Get-FileHash -LiteralPath $hive -Algorithm SHA256).Hash
$missing = Join-Path $root 'missing-update-provider.exe'
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $root
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$info.EnvironmentVariables['DEVSWEEP_SOFTWARE_STARTUP_HIVE'] = $hive
$info.EnvironmentVariables['DEVSWEEP_SOFTWARE_UPDATE_PROGRAM'] = $missing
$info.EnvironmentVariables['DEVSWEEP_SOFTWARE_REVIEW_STALE'] = '1'
$proc = [System.Diagnostics.Process]::Start($info)
$procId = $proc.Id
$deadline = (Get-Date).AddSeconds(20)
do { Start-Sleep -Milliseconds 150; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
$hwnd = $proc.MainWindowHandle
$ok = Focus-Window $hwnd
Write-Log "foreground_ok=$ok procId=$procId"
if (-not $ok) { Stop-Owned $procId $hwnd | Out-Null; throw 'foreground failed' }
[C12R6]::Chord(0x12, 0x27)
Start-Sleep -Milliseconds 600
[C12R6]::Chord(0x12, 0x28)
Start-Sleep -Milliseconds 250
[C12R6]::Chord(0x12, 0x28)
Start-Sleep -Milliseconds 250
[C12R6]::Tap(0x0D)
Start-Sleep -Milliseconds 800
$updates = @(Get-Named $hwnd)
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r6-updates-uia.txt'), $updates, $utf8)
foreach ($line in ($updates | Where-Object { $_ -match 'probe_failed|updates' })) { Write-Log "updates_UIA $line" }
if (-not (Focus-Window $hwnd)) { Write-Log 'refocus_startup=false' }
[C12R6]::Chord(0x12, 0x28)
Start-Sleep -Milliseconds 250
[C12R6]::Tap(0x0D)
Start-Sleep -Milliseconds 800
[C12R6]::Tap(0x0D)
Start-Sleep -Milliseconds 800
$stale = @(Get-Named $hwnd)
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r6-stale-uia.txt'), $stale, $utf8)
foreach ($line in ($stale | Where-Object { $_ -match 'stale_confirmation|sha256:|protected' })) { Write-Log "stale_UIA $line" }
$after = (Get-FileHash -LiteralPath $hive -Algorithm SHA256).Hash
Write-Log "hive_changed=$($before -ne $after)"
Write-Log "wm_close=$(Stop-Owned $procId $hwnd)"
Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r6-error.txt'), $log, $utf8)
