$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C12R3 {
    public const uint KEYUP = 2;
    public const uint WM_CLOSE = 0x0010;
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint source, uint target, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);
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
    public static void Tap(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, KEYUP, UIntPtr.Zero);
    }
}
'@

$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-software-optimize-workbenches\evidence'
$analyzeEvidence = Join-Path $repo '.trellis\tasks\10-15-gpui-analyze-status-tray\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
New-Item -ItemType Directory -Path $evidence, $analyzeEvidence -Force | Out-Null
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }

function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][C12R3]::PostMessage($hwnd, [C12R3]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    }
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
        try {
            $name = [string]$item.Current.Name
            if (-not [string]::IsNullOrWhiteSpace($name)) { $names.Add($name) }
        } catch {}
    }
    return $names
}

function Save-Shot([IntPtr]$hwnd, [string]$path) {
    [C12R3+RECT]$rectangle = New-Object C12R3+RECT
    if (-not [C12R3]::GetWindowRect($hwnd, [ref]$rectangle)) { return $false }
    $width = [Math]::Max(1, $rectangle.Right - $rectangle.Left)
    $height = [Math]::Max(1, $rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try { [void][C12R3]::PrintWindow($hwnd, $device, 2) } finally { $graphics.ReleaseHdc($device); $graphics.Dispose() }
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    return $true
}

function New-Hive([string]$path) {
    $json = '{"version":1,"entries":[{"name":"ordinary","location":"current_user_run","class":"ordinary","run_value":[67,58,92,102,105,120,116,117,114,101,46,101,120,101]},{"name":"guard","location":"current_user_run","class":"security","run_value":[67]},{"name":"machine","location":"machine_run","class":"ordinary","run_value":[67]},{"name":"blank","location":"current_user_run","class":"ordinary","run_value":[67],"approval":[]}]}'
    [System.IO.File]::WriteAllText($path, $json, $utf8)
}

function New-AnalyzeRoot([string]$root) {
    New-Item -ItemType Directory -Path (Join-Path $root 'empty-dir'), (Join-Path $root 'denied-dir') -Force | Out-Null
    [System.IO.File]::WriteAllBytes((Join-Path $root 'keep.bin'), [byte[]](1, 2, 3, 4))
    $user = $env:USERNAME
    & icacls (Join-Path $root 'denied-dir') /inheritance:r /deny "${user}:(OI)(CI)R" | Out-Null
}

Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$length = (Get-Item -LiteralPath $binary).Length
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash length=$length"
Write-Log 'rustc_note=cargo 1.99.0 via mbx; RUSTFLAGS=-C link-arg=/Brepro; not an MSRV 1.88.0 pass'
Write-Log 'host_maintenance=not run; defrag dism RestoreHealth w32tm resync were not started'
Write-Log 'real_uninstall=not run; real startup keys were not written'

function Invoke-Review([string]$name, [string]$language, [bool]$stale) {
    $root = Join-Path $env:TEMP "devsweep-gpui-c12r4-$name"
    if (Test-Path $root) {
        Get-ChildItem $root -Recurse -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            & icacls $_.FullName /reset | Out-Null
        }
        Remove-Item $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    $local = Join-Path $root 'local'
    $roaming = Join-Path $root 'roaming'
    $rows = Join-Path $root 'rows'
    $analyze = Join-Path $root 'analyze'
    New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming, $rows, $analyze -Force | Out-Null
    if ($language) {
        $json = '{"schema_version":1,"language":"' + $language + '"}'
        [System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\presentation-v1.json'), $json, $utf8)
    }
    $hive = Join-Path $root 'hive.json'
    New-Hive $hive
    $hiveBefore = (Get-FileHash -LiteralPath $hive -Algorithm SHA256).Hash
    New-AnalyzeRoot $analyze
    for ($index = 0; $index -lt 1000; $index++) {
        $rowName = 'row-{0:d4}' -f $index
        [System.IO.File]::WriteAllText((Join-Path $rows $rowName), 'isolated', $utf8)
    }
    $missingProvider = Join-Path $root 'missing-update-provider.exe'
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $root
    $info.EnvironmentVariables['LOCALAPPDATA'] = $local
    $info.EnvironmentVariables['APPDATA'] = $roaming
    $info.EnvironmentVariables['DEVSWEEP_SOFTWARE_STARTUP_HIVE'] = $hive
    $info.EnvironmentVariables['DEVSWEEP_SOFTWARE_UPDATE_PROGRAM'] = $missingProvider
    $info.EnvironmentVariables['DEVSWEEP_WORKBENCH_ROW_DIR'] = $rows
    $info.EnvironmentVariables['DEVSWEEP_ANALYZE_ROOT'] = $analyze
    if ($stale) { $info.EnvironmentVariables['DEVSWEEP_SOFTWARE_REVIEW_STALE'] = '1' }
    $proc = [System.Diagnostics.Process]::Start($info)
    $procId = $proc.Id
    $deadline = (Get-Date).AddSeconds(20)
    do { Start-Sleep -Milliseconds 150; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
    $hwnd = $proc.MainWindowHandle
    if ($hwnd -eq [IntPtr]::Zero) {
        Write-Log "${name}_window=missing procId=$procId"
        Stop-Owned $procId $hwnd | Out-Null
        throw "$name window missing"
    }
    $current = [C12R3]::GetCurrentThreadId()
    [uint32]$ignore = 0
    $targetThread = [C12R3]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
    $foreground = [C12R3]::GetForegroundWindow()
    $fgThread = [C12R3]::GetWindowThreadProcessId($foreground, [ref]$ignore)
    [void][C12R3]::AttachThreadInput($current, $fgThread, $true)
    [void][C12R3]::AttachThreadInput($current, $targetThread, $true)
    [C12R3]::AltUp()
    [void][C12R3]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 400
    $ok = ([C12R3]::GetForegroundWindow() -eq $hwnd)
    Write-Log "${name}_foreground_ok=$ok procId=$procId hwnd=$($hwnd.ToInt64())"
    if (-not $ok) {
        Stop-Owned $procId $hwnd | Out-Null
        throw "$name foreground failed"
    }

    function Send-Focus([IntPtr]$window) {
        $currentId = [C12R3]::GetCurrentThreadId()
        [uint32]$ignored = 0
        $target = [C12R3]::GetWindowThreadProcessId($window, [ref]$ignored)
        $front = [C12R3]::GetForegroundWindow()
        $frontThread = [C12R3]::GetWindowThreadProcessId($front, [ref]$ignored)
        [void][C12R3]::AttachThreadInput($currentId, $frontThread, $true)
        [void][C12R3]::AttachThreadInput($currentId, $target, $true)
        [C12R3]::AltUp()
        [void][C12R3]::SetForegroundWindow($window)
        Start-Sleep -Milliseconds 250
        return ([C12R3]::GetForegroundWindow() -eq $window)
    }
    if (-not (Send-Focus $hwnd)) { Write-Log "${name}_refocus_software=false" }
    [C12R3]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 500
    [C12R3]::Chord(0x12, 0x28)
    Start-Sleep -Milliseconds 200
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 500
    [C12R3]::Chord(0x12, 0x28)
    Start-Sleep -Milliseconds 200
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 600
    [C12R3]::Chord(0x12, 0x28)
    Start-Sleep -Milliseconds 200
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 700
    $review = @(Get-Named $hwnd)
    [System.IO.File]::WriteAllLines((Join-Path $evidence "c12-r3-$name-startup-review-uia.txt"), $review, $utf8)
    foreach ($line in ($review | Where-Object { $_ -match 'sha256:|protected|probe_failed|confirmation_required|requires_administrator|startup_identity|not run' } | Select-Object -First 20)) {
        Write-Log "${name}_review_UIA $line"
    }
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 700
    $software = @(Get-Named $hwnd)
    $softwarePng = Join-Path $evidence "c12-r3-$name-software.png"
    $saved = Save-Shot $hwnd $softwarePng
    Write-Log "${name}_software_names=$($software.Count) screenshot=$saved path=$softwarePng"
    $far = @($software | Where-Object { $_ -match 'row-0999' })
    Write-Log "${name}_row0999_hits=$($far.Count)"
    [System.IO.File]::WriteAllLines((Join-Path $evidence "c12-r3-$name-software-uia.txt"), $software, $utf8)
    foreach ($line in ($software | Where-Object { $_ -match 'sha256:|protected|probe_failed|confirmation_required|stale_confirmation|requires_administrator|startup_identity|not run|Leftovers|Elevation|uac_declined|outcome=' } | Select-Object -First 40)) {
        Write-Log "${name}_software_UIA $line"
    }
    $hiveAfter = (Get-FileHash -LiteralPath $hive -Algorithm SHA256).Hash
    Write-Log "${name}_hive_before=$hiveBefore hive_after=$hiveAfter hive_changed=$($hiveBefore -ne $hiveAfter)"

    if (-not (Send-Focus $hwnd)) { Write-Log "${name}_refocus_optimize=false" }
    [C12R3]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 500
    1..5 | ForEach-Object { [C12R3]::Chord(0x12, 0x28); Start-Sleep -Milliseconds 120 }
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 400
    1..4 | ForEach-Object { [C12R3]::Chord(0x12, 0x28); Start-Sleep -Milliseconds 80 }
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 800
    $optimize = @(Get-Named $hwnd)
    $optimizePng = Join-Path $evidence "c12-r3-$name-optimize.png"
    $saved = Save-Shot $hwnd $optimizePng
    Write-Log "${name}_optimize_names=$($optimize.Count) screenshot=$saved path=$optimizePng"
    $success = @($optimize | Where-Object { $_ -match 'maintenance\.(retrim|image_scan|image_repair|clock_sync)|guidance\.drive_optimize' -and $_ -match 'succeed' })
    Write-Log "${name}_host_success_claims=$($success.Count)"
    [System.IO.File]::WriteAllLines((Join-Path $evidence "c12-r3-$name-optimize-uia.txt"), $optimize, $utf8)
    foreach ($line in ($optimize | Where-Object { $_ -match 'retrim|uac_declined|Elevation|never dispatched|not run|guidance' } | Select-Object -First 24)) {
        Write-Log "${name}_optimize_UIA $line"
    }

    if (-not (Send-Focus $hwnd)) { Write-Log "${name}_refocus_analyze=false" }
    [C12R3]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 400
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 1500
    $analyze = @(Get-Named $hwnd)
    $analyzePng = Join-Path $analyzeEvidence "c13-r3-$name-analyze.png"
    $saved = Save-Shot $hwnd $analyzePng
    Write-Log "${name}_analyze_names=$($analyze.Count) screenshot=$saved path=$analyzePng"
    [System.IO.File]::WriteAllLines((Join-Path $analyzeEvidence "c13-r3-$name-analyze-uia.txt"), $analyze, $utf8)
    foreach ($line in ($analyze | Where-Object { $_ -match 'empty|unreadable|present|treemap|sha256:|keep.bin|confirmation_required' } | Select-Object -First 30)) {
        Write-Log "${name}_analyze_UIA $line"
    }

    if (-not (Send-Focus $hwnd)) { Write-Log "${name}_refocus_status=false" }
    [C12R3]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 400
    [C12R3]::Tap(0x0D)
    Start-Sleep -Milliseconds 2500
    $status = @(Get-Named $hwnd)
    $statusPng = Join-Path $analyzeEvidence "c13-r3-$name-status.png"
    $saved = Save-Shot $hwnd $statusPng
    Write-Log "${name}_status_names=$($status.Count) screenshot=$saved path=$statusPng"
    [System.IO.File]::WriteAllLines((Join-Path $analyzeEvidence "c13-r3-$name-status-uia.txt"), $status, $utf8)
    foreach ($line in ($status | Where-Object { $_ -match 'unavailable|permission_denied|pid |termination authority' } | Select-Object -First 20)) {
        Write-Log "${name}_status_UIA $line"
    }
    Write-Log "${name}_displayed_pid_not_used_to_terminate=1"
    Write-Log "${name}_wm_close=$(Stop-Owned $procId $hwnd)"
    $left = @(Get-Process -Id $procId -ErrorAction SilentlyContinue).Count
    Write-Log "${name}_proc_remaining=$left"
    if ($left -gt 0) { Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue }
}

Invoke-Review 'en' $null $false
Invoke-Review 'zh' 'zh-CN' $false
Invoke-Review 'stale' $null $true
Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
Write-Log 'c06_ac4=NOT_RUN'
Write-Log 'c11_checkboxes=unchecked'
Write-Log 'os_crash_release=NOT_RUN'
Write-Log 'focus_pattern=NOT_PASS'
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r3-review.txt'), $log, $utf8)
