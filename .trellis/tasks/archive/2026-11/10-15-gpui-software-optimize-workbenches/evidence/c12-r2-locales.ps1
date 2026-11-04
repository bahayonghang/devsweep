$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C12R2 {
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
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-software-optimize-workbenches\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }
function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) { [void][C12R2]::PostMessage($hwnd, [C12R2]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}
function Get-Names([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $items = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($item in $items) { try { $names.Add([string]$item.Current.Name) } catch {} }
    $tabs = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)))
    foreach ($tab in $tabs) { try { $names.Add('TAB ' + [string]$tab.Current.Name) } catch {} }
    return $names
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
Write-Log "utc_start=$([DateTime]::UtcNow.ToString('o'))"
Write-Log "binary_sha256=$hash length=$((Get-Item $binary).Length)"
Write-Log 'method=isolated LOCALAPPDATA; Alt+Right keyboard; no uninstall'

function Invoke-Locale([string]$name, [string]$language) {
    $root = Join-Path $env:TEMP "devsweep-gpui-c12-$name"
    if (Test-Path $root) { Remove-Item $root -Recurse -Force }
    $local = Join-Path $root 'local'
    $roaming = Join-Path $root 'roaming'
    New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
    if ($language) {
        [System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\presentation-v1.json'), "{`"schema_version`":1,`"language`":`"$language`"}", $utf8)
    }
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $repo
    $info.EnvironmentVariables['LOCALAPPDATA'] = $local
    $info.EnvironmentVariables['APPDATA'] = $roaming
    $proc = [System.Diagnostics.Process]::Start($info)
    $deadline = (Get-Date).AddSeconds(20)
    do { Start-Sleep -Milliseconds 150; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
    $hwnd = $proc.MainWindowHandle
    $current = [C12R2]::GetCurrentThreadId()
    [uint32]$ignore = 0
    $targetThread = [C12R2]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
    $foreground = [C12R2]::GetForegroundWindow()
    $fgThread = [C12R2]::GetWindowThreadProcessId($foreground, [ref]$ignore)
    [void][C12R2]::AttachThreadInput($current, $fgThread, $true)
    [void][C12R2]::AttachThreadInput($current, $targetThread, $true)
    [C12R2]::AltUp()
    [void][C12R2]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 300
    $ok = ([C12R2]::GetForegroundWindow() -eq $hwnd)
    Write-Log "${name}_foreground_ok=$ok procId=$($proc.Id)"
    if (-not $ok) { Stop-Owned $proc.Id $hwnd | Out-Null; throw "$name foreground failed" }
    [C12R2]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 300
    $software = @(Get-Names $hwnd)
    $softwareHits = @($software | Where-Object { $_ -like 'uninstall*' -or $_ -like 'updates*' -or $_ -like 'startup*' -or $_ -like 'leftovers*' })
    Write-Log "${name}_software_rows=$($softwareHits.Count)"
    $softwareHits | ForEach-Object { Write-Log "${name}_SOFTWARE $_" }
    [C12R2]::Chord(0x12, 0x27)
    Start-Sleep -Milliseconds 350
    $optimize = @(Get-Names $hwnd)
    $optimizeHits = @($optimize | Where-Object { $_ -like 'maintenance.*' -or $_ -like 'guidance.*' -or $_ -like 'dns.flush*' -or $_ -like 'settings.*' })
    $success = @($optimizeHits | Where-Object { $_ -match 'succeed|成功' })
    Write-Log "${name}_optimize_rows=$($optimizeHits.Count) success_claims=$($success.Count)"
    $optimizeHits | Select-Object -First 6 | ForEach-Object { Write-Log "${name}_OPTIMIZE $_" }
    $tabs = @($optimize | Where-Object { $_ -like 'TAB *' })
    Write-Log "${name}_tabs=$($tabs -join ' | ')"
    [C12R2+RECT]$rectangle = New-Object C12R2+RECT
    if ([C12R2]::GetWindowRect($hwnd, [ref]$rectangle)) {
        $bitmap = [System.Drawing.Bitmap]::new([Math]::Max(1, $rectangle.Right - $rectangle.Left), [Math]::Max(1, $rectangle.Bottom - $rectangle.Top))
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $device = $graphics.GetHdc()
        try { [void][C12R2]::PrintWindow($hwnd, $device, 2) } finally { $graphics.ReleaseHdc($device); $graphics.Dispose() }
        $png = Join-Path $evidence "c12-r2-$name.png"
        $bitmap.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose()
        Write-Log "${name}_screenshot=$png"
    }
    Write-Log "${name}_wm_close=$(Stop-Owned $proc.Id $hwnd)"
}

Invoke-Locale 'en' $null
Invoke-Locale 'zh' 'zh-CN'
Write-Log "spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
Write-Log "utc_end=$([DateTime]::UtcNow.ToString('o'))"
Write-Log 'confirmation_error_partial_native=NOT_RUN'
Write-Log 'native_list_virtualization=NOT_RUN'
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c12-r2-locales.txt'), $log, $utf8)
