# Native capture for the airy-glass shell task. Run from the repository root on
# a Windows desktop session. Method follows the archived
# 11-04-gpui-airy-glass/evidence/native.ps1: every run uses an isolated
# LOCALAPPDATA, navigates through UI Automation ids, and captures only the
# window of the PID it launched. Read-only: no scan, preview or cleanup runs.
#
# Optional switches for the fallback cases (AC5). They change only this
# process tree and this window, never a Windows setting:
#   -Suffix name       appended to every file name and to capture<name>.txt
#   -LogicalWidth n    resize the launched window to n logical px wide
#   -LogicalHeight n   with -LogicalWidth, also set the height (AC3 sizes:
#                      1080x720 and a wide window of 1350 or more)
#   -Treemap           also scan a generated fixture folder (never user data)
#                      in Clean and in Analyze and capture both treemaps. The
#                      fixture is a saved scan root of the isolated profile.
#                      Scans are read-only; no preview or cleanup is invoked.
#   -NoComposition     launch with GPUI_DISABLE_DIRECT_COMPOSITION=1
#   -Only a,b          run only the named cases
param([Parameter(Mandatory)][ValidateSet('before','after','final')][string]$Round,
      [string]$BinaryPath='desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe',
      [string]$Suffix='',
      [int]$LogicalWidth=0,
      [int]$LogicalHeight=0,
      [switch]$Treemap,
      [switch]$NoComposition,
      [string[]]$Only=@())
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing,System.Windows.Forms
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
    [DllImport("user32")] public static extern IntPtr GetDC(IntPtr hwnd);
    [DllImport("user32")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);
    [DllImport("gdi32")] public static extern uint GetPixel(IntPtr dc, int x, int y);
    [DllImport("user32")] public static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    public struct Point { public int x,y; }
    public struct Rect { public int left,top,right,bottom; }
}
'@
[void][ShellWindow]::SetProcessDpiAwarenessContext([IntPtr](-4))
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$out = Join-Path $PSScriptRoot $Round
[void][IO.Directory]::CreateDirectory($out)
$utf8 = [Text.UTF8Encoding]::new($false)
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('airy-shell-' + [guid]::NewGuid().ToString('N').Substring(0,8))
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
        $bmp.Save((Join-Path $out "$name$Suffix.png"),[Drawing.Imaging.ImageFormat]::Png)
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
    # The accessibility tree is built on the next painted frame after the first
    # UIA query, and an idle window does not repaint. A pointer move over the
    # blank field (client 6,300 then 7,300) requests a frame without a click.
    for ($try=0; $try -lt 15; $try++) {
        $found = Nodes | Where-Object { $_.Current.AutomationId -eq $id } | Select-Object -First 1
        if ($null -ne $found) { return $found }
        $spot=((300 -shl 16) -bor (6 + ($try % 2)))
        [void][ShellWindow]::PostMessage($hwnd,0x200,[IntPtr]0,[IntPtr]$spot)
        Start-Sleep -Milliseconds 200
    }
    $null
}
function ByName([string]$part,$type) {
    for ($try=0; $try -lt 15; $try++) {
        $found = Nodes | Where-Object { $_.Current.ControlType -eq $type -and $_.Current.Name -like "*$part*" } | Select-Object -First 1
        if ($null -ne $found) { return $found }
        $spot=((300 -shl 16) -bor (6 + ($try % 2)))
        [void][ShellWindow]::PostMessage($hwnd,0x200,[IntPtr]0,[IntPtr]$spot)
        Start-Sleep -Milliseconds 200
    }
    $null
}
function Press($node) {
    $pattern=$null
    if ($node.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern,[ref]$pattern)) { $pattern.Invoke() }
    else { $node.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d }
}
# A small project tree with known sizes. Names are long on purpose, so the
# treemap labels have to truncate.
function Fill([string]$path,[int]$bytes) {
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path))
    [IO.File]::WriteAllBytes($path,[byte[]]::new($bytes))
}
function NewTreeFixture([string]$root) {
    Fill (Join-Path $root 'service-rust/src/main.rs') 2000
    [IO.File]::WriteAllText((Join-Path $root 'service-rust/Cargo.toml'),"[package]`nname='service-rust'`nversion='0.1.0'`n",$utf8)
    Fill (Join-Path $root 'service-rust/target/debug/service-rust.pdb') 6000000
    Fill (Join-Path $root 'service-rust/target/release/service-rust.exe') 2500000
    Fill (Join-Path $root 'web-frontend/src/index.js') 3000
    [IO.File]::WriteAllText((Join-Path $root 'web-frontend/package.json'),'{"name":"web-frontend","version":"0.1.0"}',$utf8)
    Fill (Join-Path $root 'web-frontend/node_modules/left-pad/index.js') 3500000
    Fill (Join-Path $root 'web-frontend/dist/bundle.js') 900000
    Fill (Join-Path $root 'data-pipeline/pipeline.py') 1500
    [IO.File]::WriteAllText((Join-Path $root 'data-pipeline/pyproject.toml'),"[project]`nname='data-pipeline'`nversion='0.1.0'`n",$utf8)
    Fill (Join-Path $root 'data-pipeline/__pycache__/pipeline.cpython-312.pyc') 700000
    Fill (Join-Path $root 'data-pipeline/.pytest_cache/v/cache/lastfailed') 150000
    Fill (Join-Path $root 'documentation-and-design-notes/handbook.pdf') 1800000
    Fill (Join-Path $root 'recordings/standup.bin') 400000
    Fill (Join-Path $root 'notes.txt') 60000
    Fill (Join-Path $root 'a/b.bin') 20000
}
function Luminance([string]$bgr) {
    $v=[Convert]::ToInt32($bgr,16)
    $c=@(($v -band 0xff),(($v -shr 8) -band 0xff),(($v -shr 16) -band 0xff)) | ForEach-Object {
        $s=$_/255.0; if ($s -le 0.04045) { $s/12.92 } else { [math]::Pow(($s+0.055)/1.055,2.4) }
    }
    [math]::Round(0.2126*$c[0]+0.7152*$c[1]+0.0722*$c[2],4)
}
function PixelSample([int]$left,[int]$top) {
    $r=Rect
    $p=New-Object ShellWindow+Point
    $p.x=$r.left+[int]($left*$dpi/96); $p.y=$r.top+[int]($top*$dpi/96)
    if ([ShellWindow]::GetAncestor([ShellWindow]::WindowFromPoint($p),2) -ne $hwnd) { return 'occluded' }
    $dc=[ShellWindow]::GetDC([IntPtr]::Zero)
    try { '{0:X6}' -f [ShellWindow]::GetPixel($dc,$p.x,$p.y) }
    finally { [void][ShellWindow]::ReleaseDC([IntPtr]::Zero,$dc) }
}
# One field pixel (left margin, outside the paper) over black, then white.
function FieldOverBackdrops([string]$case) {
    $r=Rect
    $back=[Windows.Forms.Form]::new()
    try {
        $back.FormBorderStyle='None'; $back.ShowInTaskbar=$false
        $back.StartPosition='Manual'; $back.SetBounds($r.left,$r.top,$r.right-$r.left,$r.bottom-$r.top)
        [void][ShellWindow]::SetWindowPos($hwnd,[IntPtr](-1),0,0,0,0,0x13)
        foreach ($pair in @(@('black',[Drawing.Color]::Black),@('white',[Drawing.Color]::White))) {
            $back.BackColor=$pair[1]
            [void][ShellWindow]::SetWindowPos($back.Handle,$hwnd,$r.left,$r.top,$r.right-$r.left,$r.bottom-$r.top,0x50)
            $back.Refresh(); Start-Sleep -Milliseconds 450
            $field=PixelSample 10 400
            $lum=if ($field -eq 'occluded') { 'n/a' } else { Luminance $field }
            Record "$case field over $($pair[0]) BGR=$field luminance=$lum"
            # The lavender end of the gradient: bottom margin, right side.
            $wide=[int](($r.right-$r.left)*96/$dpi); $tall=[int](($r.bottom-$r.top)*96/$dpi)
            $end=PixelSample ($wide-60) ($tall-16)
            $elum=if ($end -eq 'occluded') { 'n/a' } else { Luminance $end }
            Record "$case field-end over $($pair[0]) BGR=$end luminance=$elum"
            # Paper padding, inside the sheet and left of every well. Only
            # meaningful at the default width, where the outer and inner
            # padding are 24 logical px each (30 at 125% text scale).
            if ($LogicalWidth -eq 0) {
                $paper=PixelSample 40 400
                $plum=if ($paper -eq 'occluded') { 'n/a' } else { Luminance $paper }
                Record "$case paper over $($pair[0]) BGR=$paper luminance=$plum"
            }
        }
    } finally { $back.Dispose(); [void][ShellWindow]::SetWindowPos($hwnd,[IntPtr](-2),0,0,0,0,0x13) }
}
$sha=[Security.Cryptography.SHA256]::Create()
try { $binaryHash=[BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($binary))).Replace('-','') } finally { $sha.Dispose() }
$transparency=[Microsoft.Win32.Registry]::GetValue('HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize','EnableTransparency',$null)
Record "round=$Round$Suffix binary_sha256=$binaryHash os=$([Environment]::OSVersion.Version) transparency=$transparency logical_width=$LogicalWidth no_composition=$($NoComposition.IsPresent)"
$cases=@(
    @{name='en-dark-100'; language='en';    theme='dark';  scale=100},
    @{name='zh-dark-100'; language='zh-CN'; theme='dark';  scale=100},
    @{name='en-light-100';language='en';    theme='light'; scale=100},
    @{name='zh-light-125';language='zh-CN'; theme='light'; scale=125}
)
# `-File` passes "a,b" as one string, so split it here.
$Only=@($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
if ($Only.Count -gt 0) { $cases=@($cases | Where-Object { $Only -contains $_.name }) }
foreach ($case in $cases) {
    $name=$case.name
    $local = Join-Path $fixture $name
    $settings = [IO.Directory]::CreateDirectory((Join-Path $local 'DevSweep/settings'))
    JsonFile (Join-Path $settings.FullName 'presentation-v1.json') @{schema_version=1;language=$case.language}
    if ($Treemap) {
        $tree=[IO.Directory]::CreateDirectory((Join-Path $fixture "$name-tree")).FullName
        NewTreeFixture $tree
        JsonFile (Join-Path $settings.FullName 'clean-scan-roots-v1.json') @{schema_version=1;roots=@($tree)}
    }
    JsonFile (Join-Path $settings.FullName 'desktop-preferences-v2.json') @{schema_version=2;theme=$case.theme;font=@{kind='system'};text_scale_percent=$case.scale;motion='reduced';planet_fps=30;status_interval_seconds=2;status_process_limit=15;hud_interval_seconds=2}
    $info = [Diagnostics.ProcessStartInfo]::new($binary)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    foreach ($key in @('LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) { $info.EnvironmentVariables[$key]=$local }
    if ($NoComposition) { $info.EnvironmentVariables['GPUI_DISABLE_DIRECT_COMPOSITION']='1' }
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
        if ($LogicalWidth -gt 0) {
            $r=Rect
            $tall=if ($LogicalHeight -gt 0) { [int]($LogicalHeight*$dpi/96) } else { $r.bottom-$r.top }
            # SWP_NOZORDER | SWP_NOACTIVATE; with a height the window also
            # moves to the top-left so a wide window stays on screen.
            if ($LogicalHeight -gt 0) { [void][ShellWindow]::SetWindowPos($hwnd,[IntPtr]::Zero,40,40,[int]($LogicalWidth*$dpi/96),$tall,0x14) }
            else { [void][ShellWindow]::SetWindowPos($hwnd,[IntPtr]::Zero,0,0,[int]($LogicalWidth*$dpi/96),$tall,0x16) }
            Start-Sleep -Milliseconds 400
        }
        $r=Rect
        Record "$name dpi=$dpi size=$($r.right-$r.left)x$($r.bottom-$r.top)"
        Start-Sleep -Milliseconds 600
        [void](Nodes)
        foreach ($mode in 'clean','software','optimize','analyze','status') {
            $tab=ById "mode-$mode"
            if ($null -eq $tab) { Record "$name ERROR tab mode-$mode missing"; continue }
            $tab.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d
            Start-Sleep -Milliseconds 700
            $tab=ById "mode-$mode"
            Record "$name mode=$mode selected=$($tab.GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty))"
            Capture "$name-$mode"
        }
        if ($Treemap) {
            $button=[Windows.Automation.ControlType]::Button
            $leaf=[IO.Path]::GetFileName($tree)
            # Analyze: open the fixture's location card, wait for the walk.
            $tab=ById 'mode-analyze'; $tab.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d; Start-Sleep -Milliseconds 700
            $card=ByName $leaf $button
            if ($null -eq $card) { Record "$name ERROR analyze fixture card missing" } else {
                Press $card
                $map=$null
                for ($n=0; $n -lt 40 -and $null -eq $map; $n++) { Start-Sleep -Milliseconds 300; $map=Nodes | Where-Object { $_.Current.Name -like 'service-rust *' } | Select-Object -First 1 }
                Record "$name analyze fixture opened=$($null -ne $map)"
                Capture "$name-analyze-treemap"
            }
            # Clean: scan the saved fixture root (project scope only).
            # Windows PowerShell reads this file as ANSI, so the zh-CN label is built from code points.
            $scanZh=-join [char[]](0x626B,0x63CF,0x9879,0x76EE)
            $tab=ById 'mode-clean'; $tab.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d; Start-Sleep -Milliseconds 700
            $scan=Nodes | Where-Object { $_.Current.ControlType -eq $button -and ($_.Current.Name -eq 'Scan projects' -or $_.Current.Name -eq $scanZh) } | Select-Object -First 1
            if ($null -eq $scan) { Record "$name ERROR clean scan button missing"; Capture "$name-clean-treemap" } else {
                Press $scan
                Start-Sleep -Seconds 6
                Record "$name clean fixture scan requested"
                Capture "$name-clean-review"
                # The disk map sits below the rows. Wheel the page from a
                # point on the card header (outside the row list's own scroll).
                $r=Rect
                $at=((($r.top+[int](290*$dpi/96)) -band 0xffff) -shl 16) -bor (($r.left+[int](500*$dpi/96)) -band 0xffff)
                for ($n=0; $n -lt 30; $n++) { [void][ShellWindow]::PostMessage($hwnd,0x20A,[IntPtr](-120 -shl 16),[IntPtr]$at); Start-Sleep -Milliseconds 30 }
                Capture "$name-clean-treemap"
            }
        }
        Key 0x79; Start-Sleep -Milliseconds 300
        $item=ById 'menu-settings'
        if ($null -ne $item) {
            $item.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            Start-Sleep -Milliseconds 500
            Capture "$name-settings"
            Key 0x1b; Start-Sleep -Milliseconds 200
        } else { Record "$name ERROR menu-settings missing" }
        $names=(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem -and $_.Current.AutomationId -like 'mode-*' } | ForEach-Object { $_.Current.Name }) -join ','
        Record "$name uia_mode_tabs=$names"
        FieldOverBackdrops $name
    } catch { Record "$name ERROR $($_.Exception.Message)" } finally {
        if (-not $proc.HasExited) { [void]$proc.CloseMainWindow(); if (-not $proc.WaitForExit(5000)) { $proc.Kill(); $proc.WaitForExit() } }
        Record "$name exit=$($proc.ExitCode)"
        $proc.Dispose()
    }
}
[IO.File]::WriteAllLines((Join-Path $out "capture$Suffix.txt"),$log,$utf8)
Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
if ($log -match ' ERROR ') { exit 1 }
