param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceDirectory,
    [Parameter(Mandatory = $true)]
    [string]$IsolatedLocalAppData
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DevSweepNativeEvidence {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    public static extern uint GetDpiForSystem();
    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr hwnd);
}
'@

function Wait-MainWindow([System.Diagnostics.Process]$Process) {
    foreach ($attempt in 1..100) {
        $Process.Refresh()
        if ($Process.HasExited) { throw "native app exited before opening a window: $($Process.ExitCode)" }
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) { return $Process.MainWindowHandle }
        Start-Sleep -Milliseconds 100
    }
    throw 'native app did not expose a main window within 10 seconds'
}

function Get-NativeWindowGeometry([IntPtr]$Handle) {
    $window = [DevSweepNativeEvidence+Rect]::new()
    $client = [DevSweepNativeEvidence+Rect]::new()
    if (-not [DevSweepNativeEvidence]::GetWindowRect($Handle, [ref]$window)) { throw 'GetWindowRect failed' }
    if (-not [DevSweepNativeEvidence]::GetClientRect($Handle, [ref]$client)) { throw 'GetClientRect failed' }
    [PSCustomObject]@{
        Left = $window.Left
        Top = $window.Top
        WindowWidth = $window.Right - $window.Left
        WindowHeight = $window.Bottom - $window.Top
        ClientWidth = $client.Right - $client.Left
        ClientHeight = $client.Bottom - $client.Top
    }
}

function Set-NativeClientSize([IntPtr]$Handle, [int]$Width, [int]$Height) {
    foreach ($attempt in 1..3) {
        $geometry = Get-NativeWindowGeometry $Handle
        $outerWidth = $geometry.WindowWidth + ($Width - $geometry.ClientWidth)
        $outerHeight = $geometry.WindowHeight + ($Height - $geometry.ClientHeight)
        if (-not [DevSweepNativeEvidence]::SetWindowPos($Handle, [IntPtr]::Zero, 36, 36, $outerWidth, $outerHeight, 0x0004 -bor 0x0040)) {
            throw "SetWindowPos failed for ${Width}x${Height}"
        }
        Start-Sleep -Milliseconds 250
        $geometry = Get-NativeWindowGeometry $Handle
        if ($geometry.ClientWidth -eq $Width -and $geometry.ClientHeight -eq $Height) { return $geometry }
    }
    throw "client size did not converge to ${Width}x${Height}: $($geometry | ConvertTo-Json -Compress)"
}

function Save-WindowScreenshot([IntPtr]$Handle, [string]$Path) {
    [void][DevSweepNativeEvidence]::SetForegroundWindow($Handle)
    Start-Sleep -Milliseconds 150
    $geometry = Get-NativeWindowGeometry $Handle
    $bitmap = [System.Drawing.Bitmap]::new($geometry.WindowWidth, $geometry.WindowHeight)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($geometry.Left, $geometry.Top, 0, 0, $bitmap.Size)
        } finally {
            $graphics.Dispose()
        }
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
}

function Save-DesktopScreenshot([string]$Path) {
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = [System.Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
        } finally {
            $graphics.Dispose()
        }
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
}

function Start-IsolatedApp {
    $process = Start-Process -FilePath $Executable -PassThru -Environment @{ LOCALAPPDATA = $IsolatedLocalAppData }
    $handle = Wait-MainWindow $process
    Start-Sleep -Milliseconds 750
    [PSCustomObject]@{ Process = $process; Handle = $handle }
}

function Close-AppGracefully([System.Diagnostics.Process]$Process) {
    if (-not $Process.CloseMainWindow()) { throw 'CloseMainWindow returned false' }
    if (-not $Process.WaitForExit(10000)) { throw 'native app did not exit within 10 seconds after CloseMainWindow' }
    "GRACEFUL_EXIT pid=$($Process.Id) exit=$($Process.ExitCode)"
}

$Executable = (Resolve-Path -LiteralPath $Executable).Path
$EvidenceDirectory = [System.IO.Path]::GetFullPath($EvidenceDirectory)
$IsolatedLocalAppData = [System.IO.Path]::GetFullPath($IsolatedLocalAppData)
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $IsolatedLocalAppData | Out-Null

$existing = @(Get-Process -Name devsweep-desktop -ErrorAction SilentlyContinue)
if ($existing.Count -ne 0) { throw "pre-existing devsweep-desktop process: $($existing.Id -join ',')" }

$systemDpi = [DevSweepNativeEvidence]::GetDpiForSystem()
"HOST_SYSTEM_DPI=$systemDpi"
"EXECUTABLE=$Executable"
"ISOLATED_LOCALAPPDATA=$IsolatedLocalAppData"

$first = Start-IsolatedApp
$windowDpi = [DevSweepNativeEvidence]::GetDpiForWindow($first.Handle)
$nativeScale = [int][math]::Round($windowDpi / 96 * 100)
$scaleFactor = $windowDpi / 96
"FIRST_PID=$($first.Process.Id) WINDOW_DPI=$windowDpi NATIVE_SCALE_PERCENT=$nativeScale"
foreach ($width in 390, 800, 1024, 1440) {
    $physicalWidth = [int][math]::Round($width * $scaleFactor)
    $physicalHeight = [int][math]::Round(900 * $scaleFactor)
    $geometry = Set-NativeClientSize $first.Handle $physicalWidth $physicalHeight
    Save-WindowScreenshot $first.Handle (Join-Path $EvidenceDirectory "en-${nativeScale}-css${width}.png")
    "CAPTURE locale=en scale=$nativeScale requested_css=${width}x900 target_physical=${physicalWidth}x${physicalHeight} actual_physical=$($geometry.ClientWidth)x$($geometry.ClientHeight)"
}

$focusWidth = [int][math]::Round(390 * $scaleFactor)
$focusHeight = [int][math]::Round(900 * $scaleFactor)
$focusGeometry = Set-NativeClientSize $first.Handle $focusWidth $focusHeight
[void][DevSweepNativeEvidence]::SetForegroundWindow($first.Handle)
[System.Windows.Forms.SendKeys]::SendWait('{TAB}')
Start-Sleep -Milliseconds 250
Save-WindowScreenshot $first.Handle (Join-Path $EvidenceDirectory "keyboard-focus-${nativeScale}-css390.png")
"CAPTURE keyboard_focus scale=$nativeScale requested_css=390x900 actual_physical=$($focusGeometry.ClientWidth)x$($focusGeometry.ClientHeight)"

[System.Windows.Forms.SendKeys]::SendWait('{TAB}')
[System.Windows.Forms.SendKeys]::SendWait('{END}')
[System.Windows.Forms.SendKeys]::SendWait('{TAB}')
Start-Sleep -Milliseconds 750

$settingsPath = Join-Path $IsolatedLocalAppData 'DevSweep\settings\presentation-v1.json'
if (-not (Test-Path -LiteralPath $settingsPath)) { throw "locale selection did not create $settingsPath" }
$settingsBytes = [System.IO.File]::ReadAllBytes($settingsPath)
$settingsText = [System.Text.Encoding]::UTF8.GetString($settingsBytes)
"SETTINGS_PATH=$settingsPath"
"SETTINGS_BYTES=$($settingsBytes.Length) SETTINGS_TEXT=$settingsText"

foreach ($width in 390, 800, 1024, 1440) {
    $physicalWidth = [int][math]::Round($width * $scaleFactor)
    $physicalHeight = [int][math]::Round(900 * $scaleFactor)
    $geometry = Set-NativeClientSize $first.Handle $physicalWidth $physicalHeight
    Save-WindowScreenshot $first.Handle (Join-Path $EvidenceDirectory "zh-CN-${nativeScale}-css${width}.png")
    "CAPTURE locale=zh-CN scale=$nativeScale requested_css=${width}x900 target_physical=${physicalWidth}x${physicalHeight} actual_physical=$($geometry.ClientWidth)x$($geometry.ClientHeight)"
}
Save-DesktopScreenshot (Join-Path $EvidenceDirectory "desktop-taskbar-${nativeScale}.png")
Close-AppGracefully $first.Process

$second = Start-IsolatedApp
$restartDpi = [DevSweepNativeEvidence]::GetDpiForWindow($second.Handle)
if ($restartDpi -ne $windowDpi) { throw "window DPI changed across restart: $windowDpi -> $restartDpi" }
"RESTART_PID=$($second.Process.Id) WINDOW_DPI=$restartDpi NATIVE_SCALE_PERCENT=$nativeScale"
$restartWidth = [int][math]::Round(800 * $scaleFactor)
$restartHeight = [int][math]::Round(900 * $scaleFactor)
$restartGeometry = Set-NativeClientSize $second.Handle $restartWidth $restartHeight
Save-WindowScreenshot $second.Handle (Join-Path $EvidenceDirectory "zh-CN-${nativeScale}-persisted-restart-css800.png")
"CAPTURE persisted_restart locale=zh-CN scale=$nativeScale requested_css=800x900 actual_physical=$($restartGeometry.ClientWidth)x$($restartGeometry.ClientHeight)"
Close-AppGracefully $second.Process

$residual = @(Get-Process -Name devsweep-desktop -ErrorAction SilentlyContinue)
if ($residual.Count -ne 0) { throw "residual devsweep-desktop process: $($residual.Id -join ',')" }
"ZERO_RESIDUAL=PASS"
