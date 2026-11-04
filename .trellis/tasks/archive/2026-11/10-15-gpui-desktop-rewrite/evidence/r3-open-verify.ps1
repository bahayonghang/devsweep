$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class R3Probe {
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
}
'@

$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-desktop-rewrite\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }

function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][R3Probe]::PostMessage($hwnd, [R3Probe]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
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
    [R3Probe+RECT]$rectangle = New-Object R3Probe+RECT
    if (-not [R3Probe]::GetWindowRect($hwnd, [ref]$rectangle)) { return $false }
    $width = [Math]::Max(1, $rectangle.Right - $rectangle.Left)
    $height = [Math]::Max(1, $rectangle.Bottom - $rectangle.Top)
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $device = $graphics.GetHdc()
    try { [void][R3Probe]::PrintWindow($hwnd, $device, 2) } finally { $graphics.ReleaseHdc($device); $graphics.Dispose() }
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    return $true
}

Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash length=$((Get-Item -LiteralPath $binary).Length)"
Write-Log 'rustc_note=cargo 1.99.0 via mbx; RUSTFLAGS=-C link-arg=/Brepro; not an MSRV 1.88.0 pass'
Write-Log 'method=isolated LOCALAPPDATA; foreground then Alt+Right; no uninstall, no defrag, no dism, no resync'

function Invoke-Walk([string]$name, [string]$language) {
    $root = Join-Path $env:TEMP "devsweep-gpui-r3-$name"
    if (Test-Path $root) { Remove-Item $root -Recurse -Force }
    $local = Join-Path $root 'local'
    $roaming = Join-Path $root 'roaming'
    New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
    if ($language) {
        $json = '{"schema_version":1,"language":"' + $language + '"}'
        [System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\presentation-v1.json'), $json, $utf8)
    }
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $root
    $info.EnvironmentVariables['LOCALAPPDATA'] = $local
    $info.EnvironmentVariables['APPDATA'] = $roaming
    $proc = [System.Diagnostics.Process]::Start($info)
    $deadline = (Get-Date).AddSeconds(20)
    do { Start-Sleep -Milliseconds 150; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
    $hwnd = $proc.MainWindowHandle
    if ($hwnd -eq [IntPtr]::Zero) {
        Write-Log "${name}_window=missing procId=$($proc.Id)"
        Stop-Owned $proc.Id $hwnd | Out-Null
        throw "$name window missing"
    }
    $current = [R3Probe]::GetCurrentThreadId()
    [uint32]$ignore = 0
    $targetThread = [R3Probe]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
    $foreground = [R3Probe]::GetForegroundWindow()
    $fgThread = [R3Probe]::GetWindowThreadProcessId($foreground, [ref]$ignore)
    [void][R3Probe]::AttachThreadInput($current, $fgThread, $true)
    [void][R3Probe]::AttachThreadInput($current, $targetThread, $true)
    [R3Probe]::AltUp()
    [void][R3Probe]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 400
    $ok = ([R3Probe]::GetForegroundWindow() -eq $hwnd)
    Write-Log "${name}_foreground_ok=$ok procId=$($proc.Id) hwnd=$($hwnd.ToInt64())"
    if (-not $ok) {
        Stop-Owned $proc.Id $hwnd | Out-Null
        throw "$name foreground failed"
    }
    $modes = @('clean', 'software', 'optimize', 'analyze', 'status')
    for ($index = 0; $index -lt $modes.Count; $index++) {
        if ($index -gt 0) {
            [R3Probe]::Chord(0x12, 0x27)
            Start-Sleep -Milliseconds 450
        } else {
            Start-Sleep -Milliseconds 250
        }
        $mode = $modes[$index]
        $names = @(Get-Named $hwnd)
        $png = Join-Path $evidence "r3-$name-$mode.png"
        $saved = Save-Shot $hwnd $png
        Write-Log "${name}_${mode}_names=$($names.Count) screenshot=$saved path=$png"
        $interesting = @($names | Where-Object {
            $_ -match 'Clean|Software|Optimize|Analyze|Status|清理|软件|优化|分析|状态|uninstall|updates|startup|leftovers|dns\.flush|maintenance\.|guidance\.|unavailable|not run|未运行|失败|确认|reboot|pending|Note|笔记'
        } | Select-Object -First 24)
        foreach ($line in $interesting) { Write-Log "${name}_${mode}_UIA $line" }
        $stateHits = @($names | Where-Object { $_ -match 'uninstall failed|Elevation was declined|reboot is required|provider UI is still pending|卸载失败|已拒绝提升|需要重新启动|仍在等待' })
        Write-Log "${name}_${mode}_confirmation_error_partial_hits=$($stateHits.Count)"
    }
    Write-Log "${name}_wm_close=$(Stop-Owned $proc.Id $hwnd)"
}

Invoke-Walk 'en' $null
Invoke-Walk 'zh' 'zh-CN'
Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
Write-Log 'confirmation_error_partial_native=NOT_RUN'
Write-Log 'native_list_virtualization=NOT_RUN'
Write-Log 'analyze_uia_selection=NOT_RUN'
Write-Log 'c06_ac4=NOT_RUN'
[System.IO.File]::WriteAllLines((Join-Path $evidence 'r3-open-verify.txt'), $log, $utf8)
