$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C11R10Diag {
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

$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-clean-workbench\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$log = New-Object System.Collections.Generic.List[string]
function Write-Log([string]$line) { $script:log.Add($line); Write-Host $line }
function Stop-Owned([int]$procId, [IntPtr]$hwnd) {
    if ($hwnd -ne [IntPtr]::Zero) {
        [void][C11R10Diag]::PostMessage($hwnd, [C11R10Diag]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    }
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { return 0 }
        Start-Sleep -Milliseconds 200
    }
    Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    return 1
}
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }

$local = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-diag\local'
$roaming = Join-Path $env:TEMP 'devsweep-gpui-c11-r10-diag\roaming'
if (Test-Path (Join-Path $env:TEMP 'devsweep-gpui-c11-r10-diag')) { Remove-Item (Join-Path $env:TEMP 'devsweep-gpui-c11-r10-diag') -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $local 'DevSweep\settings'), $roaming | Out-Null
$roots = '{"schema_version":1,"roots":["C:\\Users\\lyh\\AppData\\Local\\Temp\\devsweep-gpui-c11-r10-perf\\scan-root"]}'
[System.IO.File]::WriteAllText((Join-Path $local 'DevSweep\settings\clean-scan-roots-v1.json'), $roots, $utf8)

$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $binary
$info.UseShellExecute = $false
$info.WorkingDirectory = $repo
$info.EnvironmentVariables['LOCALAPPDATA'] = $local
$info.EnvironmentVariables['APPDATA'] = $roaming
$proc = [System.Diagnostics.Process]::Start($info)
$deadline = (Get-Date).AddSeconds(25)
do { Start-Sleep -Milliseconds 200; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
$hwnd = $proc.MainWindowHandle
$current = [C11R10Diag]::GetCurrentThreadId()
[uint32]$ignore = 0
$targetThread = [C11R10Diag]::GetWindowThreadProcessId($hwnd, [ref]$ignore)
$foregroundOk = $false
for ($attempt = 1; $attempt -le 5 -and -not $foregroundOk; $attempt++) {
    $foreground = [C11R10Diag]::GetForegroundWindow()
    $foregroundThread = [C11R10Diag]::GetWindowThreadProcessId($foreground, [ref]$ignore)
    [void][C11R10Diag]::AttachThreadInput($current, $foregroundThread, $true)
    [void][C11R10Diag]::AttachThreadInput($current, $targetThread, $true)
    [C11R10Diag]::AltUp()
    [void][C11R10Diag]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 400
    $foregroundOk = ([C11R10Diag]::GetForegroundWindow() -eq $hwnd)
}
Write-Log "foreground_ok=$foregroundOk procId=$($proc.Id)"
if (-not $foregroundOk) { Stop-Owned $proc.Id $hwnd | Out-Null; throw 'foreground failed' }

function Get-Sig([IntPtr]$window) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($window)
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $names = New-Object System.Collections.Generic.List[string]
    $stack = New-Object System.Collections.Generic.Stack[System.Windows.Automation.AutomationElement]
    $stack.Push($root)
    $guard = 0
    while ($stack.Count -gt 0 -and $guard -lt 400) {
        $guard++
        $element = $stack.Pop()
        $name = ''; $id = ''; $value = ''
        try { $name = [string]$element.Current.Name } catch {}
        try { $id = [string]$element.Current.AutomationId } catch {}
        $pattern = $null
        try {
            if ($element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
                $value = [string]$pattern.Current.Value
            }
        } catch {}
        if ($id -eq 'clean-search-input' -or $id -eq 'note-input' -or $name -like 'Scan finished*' -or $name -like '*canceled*' -or $id -like 'clean-row-*' -or $name -like '>*' -or $name -like 'Search*') {
            $names.Add("$id | $name | value=$value")
        }
        try { $child = $walker.GetFirstChild($element) } catch { $child = $null }
        while ($null -ne $child -and $names.Count -lt 12) {
            $stack.Push($child)
            try { $child = $walker.GetNextSibling($child) } catch { break }
        }
    }
    return ($names -join "`n")
}

try {
    [C11R10Diag]::Tap(0x78)
    $deadline = (Get-Date).AddSeconds(60)
    $sig = ''
    do {
        Start-Sleep -Milliseconds 400
        $sig = Get-Sig $hwnd
    } while (-not $sig.Contains('Scan finished') -and (Get-Date) -lt $deadline)
    Write-Log 'DUMP after-scan'
    Write-Log $sig
    Start-Sleep -Milliseconds 200
    [C11R10Diag]::Tap(0x09)
    Start-Sleep -Milliseconds 350
    Write-Log 'DUMP after-tab'
    Write-Log (Get-Sig $hwnd)
    [C11R10Diag]::Tap(0x39)
    Start-Sleep -Milliseconds 350
    Write-Log 'DUMP after-nine'
    Write-Log (Get-Sig $hwnd)
    [C11R10Diag]::Chord(0x12, 0x28)
    Start-Sleep -Milliseconds 350
    Write-Log 'DUMP after-altdown'
    Write-Log (Get-Sig $hwnd)
    [C11R10Diag]::Tap(0x22)
    Start-Sleep -Milliseconds 350
    Write-Log 'DUMP after-pagedown'
    Write-Log (Get-Sig $hwnd)
    [C11R10Diag]::Tap(0x78)
    Start-Sleep -Milliseconds 400
    [C11R10Diag]::Tap(0x1B)
    Start-Sleep -Milliseconds 400
    Write-Log 'DUMP after-cancel'
    Write-Log (Get-Sig $hwnd)
} finally {
    $exitCode = Stop-Owned $proc.Id $hwnd
    Write-Log "wm_close_exit=$exitCode spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)"
    [System.IO.File]::WriteAllLines((Join-Path $evidence 'c11-r10-diag.txt'), $log, $utf8)
}
