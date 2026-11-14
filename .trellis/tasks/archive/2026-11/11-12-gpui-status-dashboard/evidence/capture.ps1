# Native Status capture for the status-dashboard task. Adapted from the
# archived 11-12-gpui-airy-glass-shell/evidence/capture.ps1: every run uses an
# isolated LOCALAPPDATA, navigates through UI Automation, resizes the launched
# window to 1080x720 logical px and captures only the window of the PID it
# launched. Read-only: Status sampling only; no scan, preview or cleanup.
#
# Per case it records, from the process's own TotalProcessorTime (method of
# 11-04-gpui-airy-glass/evidence/native.ps1, 3 s windows):
#   clean_idle      Clean landing, before Status was ever shown
#   live            Status after about 12 s of live sampling
#   paused          Status after Pause (not live)
#   after_leave     Clean again after leaving Status
# and the thread count at each point, a capture about two seconds after
# entering Status (en-dark), and screenshots after 12 s of sampling.
#
#   -Baseline   run the binary as the pre-change build: Clean idle, then the
#               Status page (one snapshot on Enter, never live) idle. No
#               screenshots. Writes capture-baseline.txt.
#   -Only a,b   run only the named cases
param([string]$BinaryPath='desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe',
      [switch]$Baseline,
      [string[]]$Only=@())
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ShellWindow {
    [DllImport("user32")] public static extern bool PrintWindow(IntPtr w, IntPtr dc, uint flags);
    [DllImport("user32")] public static extern bool PostMessage(IntPtr w, uint msg, IntPtr wp, IntPtr lp);
    [DllImport("user32")] public static extern bool GetWindowRect(IntPtr w, out Rect r);
    [DllImport("user32")] public static extern uint GetDpiForWindow(IntPtr w);
    [DllImport("user32")] public static extern bool SetWindowPos(IntPtr w, IntPtr z, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
    public struct Rect { public int left,top,right,bottom; }
}
'@
[void][ShellWindow]::SetProcessDpiAwarenessContext([IntPtr](-4))
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$out = $PSScriptRoot
$utf8 = [Text.UTF8Encoding]::new($false)
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('status-dash-' + [guid]::NewGuid().ToString('N').Substring(0,8))
$log = [Collections.Generic.List[string]]::new()
function Record([string]$line) { $log.Add($line); Write-Output $line }
function JsonFile($path,$value) { [IO.File]::WriteAllText($path, ($value | ConvertTo-Json -Compress), $utf8) }
function Rect { $r = New-Object ShellWindow+Rect; [void][ShellWindow]::GetWindowRect($hwnd,[ref]$r); $r }
function Capture([string]$name) {
    Start-Sleep -Milliseconds 350
    $r = Rect
    $bmp = [Drawing.Bitmap]::new($r.right-$r.left,$r.bottom-$r.top)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    try { $ok = [ShellWindow]::PrintWindow($hwnd,$dc,2) } finally { $g.ReleaseHdc($dc); $g.Dispose() }
    try {
        if (-not $ok) { throw 'PrintWindow failed' }
        $bmp.Save((Join-Path $out "$name.png"),[Drawing.Imaging.ImageFormat]::Png)
    } finally { $bmp.Dispose() }
}
function Key([int]$vk) {
    [void][ShellWindow]::PostMessage($hwnd,0x100,[IntPtr]$vk,[IntPtr]1)
    [void][ShellWindow]::PostMessage($hwnd,0x101,[IntPtr]$vk,[IntPtr]1)
}
function Nodes {
    $root = [Windows.Automation.AutomationElement]::FromHandle($hwnd)
    $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
}
function ById([string]$id) {
    # An idle window does not repaint, and the UIA tree is built on the next
    # painted frame. A pointer move over the blank field requests a frame.
    for ($try=0; $try -lt 15; $try++) {
        $found = Nodes | Where-Object { $_.Current.AutomationId -eq $id } | Select-Object -First 1
        if ($null -ne $found) { return $found }
        $spot=((300 -shl 16) -bor (6 + ($try % 2)))
        [void][ShellWindow]::PostMessage($hwnd,0x200,[IntPtr]0,[IntPtr]$spot)
        Start-Sleep -Milliseconds 200
    }
    $null
}
function ByName([string]$exact,$type) {
    for ($try=0; $try -lt 15; $try++) {
        $found = Nodes | Where-Object { $_.Current.ControlType -eq $type -and $_.Current.Name -eq $exact } | Select-Object -First 1
        if ($null -ne $found) { return $found }
        $spot=((300 -shl 16) -bor (6 + ($try % 2)))
        [void][ShellWindow]::PostMessage($hwnd,0x200,[IntPtr]0,[IntPtr]$spot)
        Start-Sleep -Milliseconds 200
    }
    $null
}
function Invoke($node) {
    $pattern=$null
    if ($node.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern,[ref]$pattern)) { $pattern.Invoke() }
    else { $node.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d }
}
function Tab([string]$mode) {
    $tab=ById "mode-$mode"
    if ($null -eq $tab) { throw "tab mode-$mode missing" }
    $tab.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d
}
# Process CPU time over a 3 s window, plus the thread count at its end.
function IdleCpu([string]$label) {
    $proc.Refresh(); $cpu=$proc.TotalProcessorTime.TotalMilliseconds
    Start-Sleep -Seconds 3
    $proc.Refresh()
    Record "$name ${label}_cpu_ms_over_3s=$($proc.TotalProcessorTime.TotalMilliseconds-$cpu) threads=$($proc.Threads.Count)"
}
$sha=[Security.Cryptography.SHA256]::Create()
try { $binaryHash=[BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($binary))).Replace('-','') } finally { $sha.Dispose() }
Record "baseline=$($Baseline.IsPresent) binary_sha256=$binaryHash os=$([Environment]::OSVersion.Version) cores=$([Environment]::ProcessorCount)"
$cases=@(
    @{name='en-dark';  language='en';    theme='dark'},
    @{name='zh-dark';  language='zh-CN'; theme='dark'},
    @{name='en-light'; language='en';    theme='light'},
    @{name='zh-light'; language='zh-CN'; theme='light'}
)
if ($Baseline) { $cases=@($cases[0]) }
$Only=@($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
if ($Only.Count -gt 0) { $cases=@($cases | Where-Object { $Only -contains $_.name }) }
# Windows PowerShell reads this file as ANSI, so zh-CN labels are built from code points.
$pauseZh=-join [char[]](0x6682,0x505C)
foreach ($case in $cases) {
    $name=$case.name
    $local = Join-Path $fixture $name
    $settings = [IO.Directory]::CreateDirectory((Join-Path $local 'DevSweep/settings'))
    JsonFile (Join-Path $settings.FullName 'presentation-v1.json') @{schema_version=1;language=$case.language}
    JsonFile (Join-Path $settings.FullName 'desktop-preferences-v2.json') @{schema_version=2;theme=$case.theme;font=@{kind='system'};text_scale_percent=100;motion='reduced';planet_fps=30;status_interval_seconds=2;status_process_limit=15;hud_interval_seconds=2}
    $info = [Diagnostics.ProcessStartInfo]::new($binary)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    foreach ($key in @('LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) { $info.EnvironmentVariables[$key]=$local }
    $proc = [Diagnostics.Process]::Start($info)
    try {
        for ($n=0; $n -lt 100; $n++) {
            $proc.Refresh()
            if ($proc.HasExited) { throw "startup exit $($proc.ExitCode)" }
            if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
            Start-Sleep -Milliseconds 200
        }
        $hwnd=$proc.MainWindowHandle
        if ($hwnd -eq [IntPtr]::Zero) { throw 'window missing' }
        $dpi=[ShellWindow]::GetDpiForWindow($hwnd)
        # SWP_NOZORDER | SWP_NOACTIVATE, top-left so the window stays on screen.
        [void][ShellWindow]::SetWindowPos($hwnd,[IntPtr]::Zero,40,40,[int](1080*$dpi/96),[int](720*$dpi/96),0x14)
        Start-Sleep -Milliseconds 600
        $r=Rect
        Record "$name dpi=$dpi size=$($r.right-$r.left)x$($r.bottom-$r.top)"
        [void](Nodes)
        Tab 'clean'; Start-Sleep -Seconds 2
        IdleCpu 'clean_idle'
        $entered=[Diagnostics.Stopwatch]::StartNew()
        Tab 'status'
        if ($Baseline) {
            # The pre-change page samples once on Enter and is never live.
            Start-Sleep -Seconds 3
            IdleCpu 'status_not_live'
            Tab 'clean'; Start-Sleep -Seconds 1
            IdleCpu 'after_leave'
            continue
        }
        # The tiles are not exposed by name in UIA, so first-reading latency
        # is shown by a capture two seconds after entry, with no click.
        if ($name -eq 'en-dark') {
            while ($entered.ElapsedMilliseconds -lt 1650) { Start-Sleep -Milliseconds 50 }
            Capture "status-$name-2s"
            Record "$name captured_after_entry_ms=$($entered.ElapsedMilliseconds)"
        }
        while ($entered.ElapsedMilliseconds -lt 12000) { Start-Sleep -Milliseconds 100 }
        Capture "status-$name"
        IdleCpu 'live'
        $label=if ($case.language -eq 'en') { 'Pause' } else { $pauseZh }
        $pause=ByName $label ([Windows.Automation.ControlType]::Button)
        if ($null -eq $pause) { Record "$name ERROR pause button missing" } else {
            Invoke $pause; Start-Sleep -Seconds 1
            IdleCpu 'paused'
        }
        Tab 'clean'; Start-Sleep -Seconds 1
        IdleCpu 'after_leave'
    } catch { Record "$name ERROR $($_.Exception.Message)" } finally {
        if (-not $proc.HasExited) { [void]$proc.CloseMainWindow(); if (-not $proc.WaitForExit(5000)) { $proc.Kill(); $proc.WaitForExit() } }
        Record "$name exit=$($proc.ExitCode)"
        $proc.Dispose()
    }
}
$file=if ($Baseline) { 'capture-baseline.txt' } else { 'capture.txt' }
[IO.File]::WriteAllLines((Join-Path $out $file),$log,$utf8)
Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
if ($log -match ' ERROR ') { exit 1 }
