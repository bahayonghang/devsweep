param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class DevSweepNativeEvidence2 {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, IntPtr processId);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr destination, uint flags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr insertAfter, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
}
'@

$evidenceRoot = Join-Path $RepositoryRoot '.trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-lifecycle-native-20260830'
$sessionRoot = Join-Path $evidenceRoot 'release-attempt2'
$localAppData = Join-Path $sessionRoot 'localappdata'
$screenshots = Join-Path $sessionRoot 'screenshots'
if (Test-Path -LiteralPath $sessionRoot) {
    throw "release evidence root already exists: $sessionRoot"
}
New-Item -ItemType Directory -Path $localAppData | Out-Null
New-Item -ItemType Directory -Path $screenshots | Out-Null

$executable = Join-Path $RepositoryRoot 'target/release/devsweep-desktop.exe'
$expectedHash = 'E30AC97FB4EF8FA12BA41C415A6E66A7E36A8B9EEA58FD8920F9883E3F41C418'
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

function Focus-Window([IntPtr]$window) {
    $foreground = [DevSweepNativeEvidence2]::GetForegroundWindow()
    $currentThread = [DevSweepNativeEvidence2]::GetCurrentThreadId()
    $foregroundThread = [DevSweepNativeEvidence2]::GetWindowThreadProcessId($foreground, [IntPtr]::Zero)
    $attached = $false
    try {
        if ($currentThread -ne $foregroundThread) {
            $attached = [DevSweepNativeEvidence2]::AttachThreadInput($currentThread, $foregroundThread, $true)
        }
        [void][DevSweepNativeEvidence2]::BringWindowToTop($window)
        if (-not [DevSweepNativeEvidence2]::SetForegroundWindow($window)) { throw 'SetForegroundWindow failed' }
    } finally {
        if ($attached) {
            [void][DevSweepNativeEvidence2]::AttachThreadInput($currentThread, $foregroundThread, $false)
        }
    }
    Start-Sleep -Milliseconds 300
}

function Focused-Element([string]$label) {
    $element = [System.Windows.Automation.AutomationElement]::FocusedElement
    if ($null -eq $element) { return "${label}_FOCUS=<none>" }
    $name = $element.Current.Name
    $automationId = $element.Current.AutomationId
    $controlType = $element.Current.ControlType.ProgrammaticName
    return "${label}_FOCUS=NAME:$name|ID:$automationId|TYPE:$controlType"
}

function Capture-Window([IntPtr]$window, [string]$name) {
    [DevSweepNativeEvidence2+RECT]$rectangle = New-Object DevSweepNativeEvidence2+RECT
    if (-not [DevSweepNativeEvidence2]::GetWindowRect($window, [ref]$rectangle)) { throw 'GetWindowRect failed' }
    $width = [int]($rectangle.Right - $rectangle.Left)
    $height = [int]($rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try {
        if (-not [DevSweepNativeEvidence2]::PrintWindow($window, $device, 2)) { throw 'PrintWindow failed' }
    } finally {
        $graphics.ReleaseHdc($device)
        $graphics.Dispose()
    }
    $path = Join-Path $screenshots $name
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    "SCREENSHOT=$name|BYTES=$((Get-Item -LiteralPath $path).Length)|SHA256=$((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash)|WIDTH=$width|HEIGHT=$height"
}

function Resize-And-Capture([IntPtr]$window, [int]$width, [int]$height, [string]$name) {
    if (-not [DevSweepNativeEvidence2]::SetWindowPos($window, [IntPtr]::Zero, 0, 0, $width, $height, 0x0014)) {
        throw "SetWindowPos failed for $width x $height"
    }
    Start-Sleep -Milliseconds 600
    Capture-Window $window $name
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

function Close-DevSweep([System.Diagnostics.Process]$process, [IntPtr]$window, [string]$label) {
    $owned = @(Get-DescendantProcessIds $process.Id)
    "${label}_PID=$($process.Id)"
    "${label}_START=$($process.StartTime.ToString('o'))"
    "${label}_OWNED_DESCENDANTS_BEFORE=$($owned -join ',')"
    if (-not [DevSweepNativeEvidence2]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "$label WM_CLOSE post failed"
    }
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

$first = Start-DevSweep
$firstWindow = $first.MainWindowHandle
Focus-Window $firstWindow
Resize-And-Capture $firstWindow 390 720 '01-release-en-390-diagnostic.png'
Resize-And-Capture $firstWindow 800 720 '02-release-en-800-diagnostic.png'
Resize-And-Capture $firstWindow 1024 720 '03-release-en-1024-diagnostic.png'
Resize-And-Capture $firstWindow 1440 900 '04-release-en-1440-diagnostic.png'
[System.Windows.Forms.SendKeys]::SendWait('{TAB}{TAB}')
Start-Sleep -Milliseconds 300
Focused-Element 'LANGUAGE_OPENER'
Capture-Window $firstWindow '05-release-language-focus.png'
[System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
Start-Sleep -Milliseconds 700
Capture-Window $firstWindow '06-release-settings-en.png'
[System.Windows.Forms.SendKeys]::SendWait('{TAB}{TAB}{END}')

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
Capture-Window $firstWindow '07-release-settings-zh.png'
[System.Windows.Forms.SendKeys]::SendWait('%{LEFT}')
Start-Sleep -Milliseconds 800
Focused-Element 'AFTER_BACK'
Capture-Window $firstWindow '08-release-back-zh-focus.png'
Close-DevSweep $first $firstWindow 'SESSION1'

$second = Start-DevSweep
$secondWindow = $second.MainWindowHandle
Focus-Window $secondWindow
Resize-And-Capture $secondWindow 1024 720 '09-release-restart-zh.png'
$storeAfter = [System.IO.File]::ReadAllBytes($store)
"RESTART_STORE_SHA256=$((Get-FileHash -LiteralPath $store -Algorithm SHA256).Hash)"
if (-not [System.Linq.Enumerable]::SequenceEqual[byte]($storeBytes, $storeAfter)) { throw 'restart changed presentation bytes' }
Close-DevSweep $second $secondWindow 'SESSION2'

$ownedPids = @($first.Id, $second.Id)
$listeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object OwningProcess -in $ownedPids)
$residue = @(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue | Where-Object Name -Match '(?i)(\.tmp$|\.lock$)')
$processResidue = @(Get-Process -Name 'devsweep-desktop' -ErrorAction SilentlyContinue | Where-Object Path -eq $executable)
"FINAL_PROCESS_RESIDUE=$($processResidue.Count)"
"FINAL_LISTENER_RESIDUE=$($listeners.Count)"
"FINAL_LOCK_TEMP_RESIDUE=$($residue.Count)"
if ($processResidue.Count -ne 0 -or $listeners.Count -ne 0 -or $residue.Count -ne 0) { throw 'process, listener, or lock/temp residue remains' }
'RELEASE_NATIVE_EXIT=0'
