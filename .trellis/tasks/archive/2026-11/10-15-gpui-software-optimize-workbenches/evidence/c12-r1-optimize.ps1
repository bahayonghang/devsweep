$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C12R1 {
    public const uint KEYUP = 2;
    public const uint WM_CLOSE = 0x0010;
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr destination, uint flags);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public static void AltUp() { keybd_event(0x12, 0, KEYUP, UIntPtr.Zero); }
    public static void Chord(byte modifier, byte key) {
        keybd_event(modifier, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
        keybd_event(modifier, 0, KEYUP, UIntPtr.Zero);
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
    if ($hwnd -ne [IntPtr]::Zero) { [void][C12R1]::PostMessage($hwnd, [C12R1]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$length = (Get-Item -LiteralPath $binary).Length
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash length=$length"
Write-Log 'rustc_note=cargo 1.99.0 via mbx; not an MSRV 1.88.0 pass'
$local = Join-Path $env:TEMP 'devsweep-gpui-c12-r1\local'
$roaming = Join-Path $env:TEMP 'devsweep-gpui-c12-r1\roaming'
$appRoot = Join-Path $env:TEMP 'devsweep-gpui-c12-r1'
if (Test-Path $appRoot) { Remove-Item $appRoot -Recurse -Force }
New-Item -ItemType Directory -Path $local, $roaming | Out-Null
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $repo
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$proc = [System.Diagnostics.Process]::Start($info)
$deadline = (Get-Date).AddSeconds(20)
do { Start-Sleep -Milliseconds 200; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
$hwnd = $proc.MainWindowHandle
$current = [C12R1]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C12R1]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
$foreground = [C12R1]::GetForegroundWindow()
$fgThread = [C12R1]::GetWindowThreadProcessId($foreground, [ref]$ignore)
[void][C12R1]::AttachThreadInput($current, $fgThread, $true)
[void][C12R1]::AttachThreadInput($current, $targetThread, $true)
[C12R1]::AltUp()
[void][C12R1]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 300
$ok = ([C12R1]::GetForegroundWindow() -eq $hwnd)
Write-Log "foreground_ok=$ok procId=$($proc.Id)"
if (-not $ok) { Stop-Owned $proc.Id $hwnd | Out-Null; throw 'foreground failed' }
[C12R1]::Chord(0x12, 0x27)
Start-Sleep -Milliseconds 250
[C12R1]::Chord(0x12, 0x27)
Start-Sleep -Milliseconds 400
$root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
$items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
$hits = 0
$bad = 0
foreach ($item in $items) {
    $name = ''
    try { $name = [string]$item.Current.Name } catch {}
    if ($name -like 'maintenance.*' -or $name -like 'guidance.*' -or $name -like 'dns.flush*') {
        $hits++
        Write-Log "ROW $name"
        if ($name -match 'succeed') { $bad++ }
    }
}
Write-Log "optimize_rows=$hits success_claims=$bad"
[C12R1+RECT]$rectangle = New-Object C12R1+RECT
if ([C12R1]::GetWindowRect($hwnd, [ref]$rectangle)) {
    $width = [Math]::Max(1, $rectangle.Right - $rectangle.Left)
    $height = [Math]::Max(1, $rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try { [void][C12R1]::PrintWindow($hwnd, $device, 2) } finally { $graphics.ReleaseHdc($device); $graphics.Dispose() }
    $png = Join-Path $evidence 'c12-r1-optimize.png'
    $bitmap.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    Write-Log "screenshot=$png"
}
$exitCode = Stop-Owned $proc.Id $hwnd
Write-Log "wm_close_exit=$exitCode spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r1-optimize.txt'), $log, $utf8)
