param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot
)

$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class DevSweepReleaseEvidence3 {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr destination, uint flags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr insertAfter, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
}
'@
[void][DevSweepReleaseEvidence3]::SetProcessDpiAwarenessContext([IntPtr](-4))
Add-Type -AssemblyName System.Drawing

$evidenceRoot = Join-Path $RepositoryRoot '.trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-lifecycle-native-20260830'
$sessionRoot = Join-Path $evidenceRoot 'release-attempt3'
$localAppData = Join-Path $sessionRoot 'localappdata'
$screenshots = Join-Path $sessionRoot 'screenshots'
if (Test-Path -LiteralPath $sessionRoot) { throw "release evidence root already exists: $sessionRoot" }
New-Item -ItemType Directory -Path $localAppData | Out-Null
New-Item -ItemType Directory -Path $screenshots | Out-Null

$executable = Join-Path $RepositoryRoot 'target/release/devsweep-desktop.exe'
$expectedHash = 'C75D08506B97984635EB3799E8BD3D9D5732A5D40A328BBA4B3FEA2D6FD66742'
$actualHash = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
if ($actualHash -ne $expectedHash) { throw "release hash changed: $actualHash" }

function Start-DevSweep {
    $previous = $env:LOCALAPPDATA
    try {
        $env:LOCALAPPDATA = $localAppData
        $process = Start-Process -FilePath $executable -PassThru
    } finally {
        $env:LOCALAPPDATA = $previous
    }
    $deadline = (Get-Date).AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
    } while ($process.MainWindowHandle -eq 0 -and -not $process.HasExited -and (Get-Date) -lt $deadline)
    if ($process.HasExited -or $process.MainWindowHandle -eq 0) { throw 'release window did not appear' }
    return $process
}

function Force-Foreground([System.Diagnostics.Process]$process) {
    [DevSweepReleaseEvidence3]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [DevSweepReleaseEvidence3]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    if (-not [DevSweepReleaseEvidence3]::SetForegroundWindow($process.MainWindowHandle)) {
        throw 'SetForegroundWindow failed'
    }
    Start-Sleep -Milliseconds 350
    $foreground = [DevSweepReleaseEvidence3]::GetForegroundWindow()
    [uint32]$owner = 0
    [void][DevSweepReleaseEvidence3]::GetWindowThreadProcessId($foreground, [ref]$owner)
    if ($owner -ne $process.Id) { throw "foreground owner mismatch: $owner" }
    "FOREGROUND_PID=$owner|HWND=$foreground"
}

function Capture-Window([IntPtr]$window, [string]$name) {
    [DevSweepReleaseEvidence3+RECT]$rectangle = New-Object DevSweepReleaseEvidence3+RECT
    if (-not [DevSweepReleaseEvidence3]::GetWindowRect($window, [ref]$rectangle)) { throw 'GetWindowRect failed' }
    $width = [int]($rectangle.Right - $rectangle.Left)
    $height = [int]($rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try {
        if (-not [DevSweepReleaseEvidence3]::PrintWindow($window, $device, 2)) { throw 'PrintWindow failed' }
    } finally {
        $graphics.ReleaseHdc($device)
        $graphics.Dispose()
    }
    $path = Join-Path $screenshots $name
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    "SCREENSHOT=$name|BYTES=$((Get-Item -LiteralPath $path).Length)|SHA256=$((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash)|PHYSICAL_WIDTH=$width|PHYSICAL_HEIGHT=$height"
}

function Resize-Diagnostic([System.Diagnostics.Process]$process, [int]$cssWidth, [int]$cssHeight, [string]$name) {
    $dpi = [DevSweepReleaseEvidence3]::GetDpiForWindow($process.MainWindowHandle)
    $physicalWidth = [int][Math]::Round($cssWidth * $dpi / 96.0)
    $physicalHeight = [int][Math]::Round($cssHeight * $dpi / 96.0)
    if (-not [DevSweepReleaseEvidence3]::SetWindowPos($process.MainWindowHandle, [IntPtr]::Zero, 0, 0, $physicalWidth, $physicalHeight, 0x0014)) {
        throw "SetWindowPos failed for diagnostic CSS $cssWidth x $cssHeight"
    }
    Start-Sleep -Milliseconds 550
    "DIAGNOSTIC_REQUEST=CSS_WIDTH:$cssWidth|CSS_HEIGHT:$cssHeight|DPI:$dpi|PHYSICAL_WIDTH:$physicalWidth|PHYSICAL_HEIGHT:$physicalHeight"
    Capture-Window $process.MainWindowHandle $name
}

function Get-DescendantProcessIds([int]$rootProcessId) {
    $all = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($rootProcessId)
    $descendants = [System.Collections.Generic.List[int]]::new()
    while ($pending.Count -gt 0) {
        $parent = $pending.Dequeue()
        foreach ($child in $all | Where-Object ParentProcessId -eq $parent) {
            if (-not $descendants.Contains([int]$child.ProcessId)) {
                $descendants.Add([int]$child.ProcessId)
                $pending.Enqueue([int]$child.ProcessId)
            }
        }
    }
    return @($descendants)
}

function Close-DevSweep([System.Diagnostics.Process]$process, [string]$label) {
    $owned = @(Get-DescendantProcessIds $process.Id)
    "${label}_PID=$($process.Id)"
    "${label}_START=$($process.StartTime.ToString('o'))"
    "${label}_OWNED_DESCENDANTS_BEFORE=$($owned -join ',')"
    $requested = $process.CloseMainWindow()
    "${label}_CLOSE_MAIN_WINDOW=$requested"
    if (-not $requested) { throw "$label CloseMainWindow refused" }
    $exited = $process.WaitForExit(20000)
    "${label}_MAIN_EXITED=$exited"
    if (-not $exited) { throw "$label main process did not exit after native close" }
    Start-Sleep -Seconds 2
    $remaining = @($owned | Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue })
    "${label}_DESCENDANT_RESIDUE=$($remaining -join ',')"
    if ($remaining.Count -ne 0) { throw "$label descendant process residue: $($remaining -join ',')" }
}

"CAPTURED_AT=$((Get-Date).ToString('o'))"
"EXECUTABLE=$executable"
"EXECUTABLE_BYTES=$((Get-Item -LiteralPath $executable).Length)"
"EXECUTABLE_SHA256=$actualHash"
"LOCALAPPDATA=$localAppData"
"INITIAL_FILES=$(@(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue).Count)"

$shell = New-Object -ComObject WScript.Shell
$first = Start-DevSweep
Resize-Diagnostic $first 390 720 '01-release-en-390-diagnostic.png'
Resize-Diagnostic $first 800 720 '02-release-en-800-diagnostic.png'
Resize-Diagnostic $first 1024 720 '03-release-en-1024-diagnostic.png'
Resize-Diagnostic $first 1440 900 '04-release-en-1440-diagnostic.png'
Force-Foreground $first
$shell.SendKeys('{TAB}')
Start-Sleep -Milliseconds 350
Capture-Window $first.MainWindowHandle '05-release-first-tab-focus.png'
$shell.SendKeys('{TAB}')
Start-Sleep -Milliseconds 350
Capture-Window $first.MainWindowHandle '06-release-language-focus.png'
$shell.SendKeys('{ENTER}')
Start-Sleep -Milliseconds 700
Capture-Window $first.MainWindowHandle '07-release-settings-en.png'
$shell.SendKeys('{TAB}')
Start-Sleep -Milliseconds 200
$shell.SendKeys('{TAB}')
Start-Sleep -Milliseconds 200
$shell.SendKeys('{END}')

$store = Join-Path $localAppData 'DevSweep/settings/presentation-v1.json'
$storeDeadline = (Get-Date).AddSeconds(8)
do { Start-Sleep -Milliseconds 200 } while (-not (Test-Path -LiteralPath $store) -and (Get-Date) -lt $storeDeadline)
if (-not (Test-Path -LiteralPath $store)) { throw 'Chinese presentation store was not created' }
$storeBytes = [System.IO.File]::ReadAllBytes($store)
$storeText = [System.Text.Encoding]::UTF8.GetString($storeBytes)
if ($storeText -ne '{"schema_version":1,"language":"zh-CN"}') { throw "unexpected store bytes: $storeText" }
"STORE_BYTES=$($storeBytes.Length)"
"STORE_SHA256=$((Get-FileHash -LiteralPath $store -Algorithm SHA256).Hash)"
"STORE_TEXT=$storeText"
Start-Sleep -Milliseconds 800
Capture-Window $first.MainWindowHandle '08-release-settings-zh.png'
$shell.SendKeys('%{LEFT}')
Start-Sleep -Milliseconds 800
Capture-Window $first.MainWindowHandle '09-release-back-zh-focus.png'
Close-DevSweep $first 'SESSION1'

$second = Start-DevSweep
Resize-Diagnostic $second 1024 720 '10-release-restart-zh.png'
$storeAfter = [System.IO.File]::ReadAllBytes($store)
"RESTART_STORE_SHA256=$((Get-FileHash -LiteralPath $store -Algorithm SHA256).Hash)"
if (-not [System.Linq.Enumerable]::SequenceEqual[byte]($storeBytes, $storeAfter)) { throw 'restart changed presentation bytes' }
Close-DevSweep $second 'SESSION2'

$ownedPids = @($first.Id, $second.Id)
$listeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object OwningProcess -in $ownedPids)
$residue = @(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue | Where-Object Name -Match '(?i)(\.tmp$|\.lock$)')
$processResidue = @(Get-Process -Name 'devsweep-desktop' -ErrorAction SilentlyContinue | Where-Object Path -eq $executable)
"FINAL_PROCESS_RESIDUE=$($processResidue.Count)"
"FINAL_LISTENER_RESIDUE=$($listeners.Count)"
"FINAL_LOCK_TEMP_RESIDUE=$($residue.Count)"
if ($processResidue.Count -ne 0 -or $listeners.Count -ne 0 -or $residue.Count -ne 0) { throw 'process, listener, or lock/temp residue remains' }
'RELEASE_NATIVE_EXIT=0'
