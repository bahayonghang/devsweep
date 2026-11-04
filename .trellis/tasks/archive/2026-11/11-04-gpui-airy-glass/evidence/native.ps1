# Run from the repository root on a Windows desktop. All messages and captures
# target the window from the newly launched PID. The scan uses one fixture root.
param([Parameter(Mandatory)][ValidateSet('before','after','debug')][string]$Round,
      [string]$BinaryPath='desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing,System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class AiryWindow {
    [DllImport("user32")] public static extern bool PrintWindow(IntPtr w, IntPtr dc, uint flags);
    [DllImport("user32")] public static extern bool PostMessage(IntPtr w, uint msg, IntPtr wp, IntPtr lp);
    [DllImport("user32")] public static extern bool GetWindowRect(IntPtr w, out Rect r);
    [DllImport("user32")] public static extern uint GetDpiForWindow(IntPtr w);
    [DllImport("user32")] public static extern bool SetWindowPos(IntPtr w, IntPtr z, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32")] public static extern bool IsZoomed(IntPtr w);
    [DllImport("user32")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
    [DllImport("user32")] public static extern IntPtr GetDC(IntPtr hwnd);
    [DllImport("user32")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);
    [DllImport("gdi32")] public static extern uint GetPixel(IntPtr dc, int x, int y);
    [DllImport("user32")] public static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32")] public static extern bool ScreenToClient(IntPtr hwnd, ref Point point);
    [DllImport("user32")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    public struct Point { public int x,y; }
    public struct Rect { public int left,top,right,bottom; }
}
'@
[void][AiryWindow]::SetProcessDpiAwarenessContext([IntPtr](-4))
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$utf8 = [Text.UTF8Encoding]::new($false)
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('airy-glass-' + [guid]::NewGuid().ToString('N').Substring(0,8))
$project = Join-Path $fixture 'sample-project'
foreach ($name in '.pytest_cache','.mypy_cache') {
    $cache = [IO.Directory]::CreateDirectory((Join-Path $project $name))
    [IO.File]::WriteAllText((Join-Path $cache.FullName 'sample.txt'), ('fixture' * 200))
}
[IO.File]::WriteAllText((Join-Path $project 'pyproject.toml'), "[project]`nname='sample'`nversion='0.1.0'`n")
$log = [Collections.Generic.List[string]]::new()
function Record([string]$line) { $log.Add($line); Write-Output $line }
function JsonFile($path,$value) { [IO.File]::WriteAllText($path, ($value | ConvertTo-Json -Compress), $utf8) }
function Rect { $r = New-Object AiryWindow+Rect; [void][AiryWindow]::GetWindowRect($hwnd,[ref]$r); $r }
function Capture([string]$view) {
    Start-Sleep -Milliseconds 300
    $r = Rect
    $bmp = [Drawing.Bitmap]::new($r.right-$r.left,$r.bottom-$r.top)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    try { $ok = [AiryWindow]::PrintWindow($hwnd,$dc,2) } finally { $g.ReleaseHdc($dc); $g.Dispose() }
    try {
        if (-not $ok) { throw 'PrintWindow failed' }
        $bmp.Save((Join-Path $PSScriptRoot "$Round-$theme-$view.png"),[Drawing.Imaging.ImageFormat]::Png)
    } finally { $bmp.Dispose() }
}
function Key([int]$vk) {
    [void][AiryWindow]::PostMessage($hwnd,0x100,[IntPtr]$vk,[IntPtr]1)
    [void][AiryWindow]::PostMessage($hwnd,0x101,[IntPtr]$vk,[IntPtr]1)
}
function PixelSample([int]$left,[int]$top) {
    $p=New-Object AiryWindow+Point
    $p.x=$r.left+[int]($left*$dpi/96); $p.y=$r.top+[int]($top*$dpi/96)
    if ([AiryWindow]::GetAncestor([AiryWindow]::WindowFromPoint($p),2) -ne $hwnd) { return 'occluded' }
    $dc=[AiryWindow]::GetDC([IntPtr]::Zero)
    try { '{0:X6}' -f [AiryWindow]::GetPixel($dc,$p.x,$p.y) }
    finally { [void][AiryWindow]::ReleaseDC([IntPtr]::Zero,$dc) }
}
function CompareBackdrop([string]$context) {
    $r=Rect
    $back=[Windows.Forms.Form]::new()
    try {
        $back.FormBorderStyle='None'; $back.ShowInTaskbar=$false
        $back.StartPosition='Manual'; $back.SetBounds($r.left,$r.top,$r.right-$r.left,$r.bottom-$r.top)
        $back.BackColor=[Drawing.Color]::DarkSlateBlue
        [void][AiryWindow]::SetWindowPos($hwnd,[IntPtr](-1),0,0,0,0,0x13)
        [void][AiryWindow]::SetWindowPos($back.Handle,$hwnd,$r.left,$r.top,$r.right-$r.left,$r.bottom-$r.top,0x50)
        $back.Refresh(); Start-Sleep -Milliseconds 400
        $blue=PixelSample 14 115; Record "$context over blue field_BGR=$blue interior_BGR=$(PixelSample 350 110)"
        $back.BackColor=[Drawing.Color]::Gold; $back.Refresh(); Start-Sleep -Milliseconds 400
        $gold=PixelSample 14 115; Record "$context over gold field_BGR=$gold interior_BGR=$(PixelSample 350 110)"
        $expectedChange=$Round -eq 'after' -and $context -like '* wide'
        if ($blue -eq 'occluded' -or $gold -eq 'occluded' -or (($blue -ne $gold) -ne $expectedChange)) {
            throw "Unexpected background response for $context ($blue, $gold)"
        }
    } finally { $back.Dispose(); [void][AiryWindow]::SetWindowPos($hwnd,[IntPtr](-2),0,0,0,0,0x13) }
}
function Nodes {
    $root = [Windows.Automation.AutomationElement]::FromHandle($hwnd)
    $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
}
function Names { (Nodes | ForEach-Object { $_.Current.Name }) -join ' | ' }
function Button([string]$name) {
    Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::Button -and $_.Current.Name -like $name } | Select-Object -First 1
}
Record "binary=$((Get-FileHash $binary).Hash) os=$([Environment]::OSVersion.Version) windowsBuild=$((Get-ItemProperty 'HKLM:/SOFTWARE/Microsoft/Windows NT/CurrentVersion').CurrentBuild) transparency=$((Get-ItemProperty 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Themes/Personalize' -ErrorAction SilentlyContinue).EnableTransparency)"
$cases=if ($Round -eq 'after') { @('dark','restored','light','solid') } elseif ($Round -eq 'debug') { 'dark' } else { @('dark','light') }
foreach ($theme in $cases) {
    $local = Join-Path $fixture $theme
    if ($theme -eq 'restored') { $local=Join-Path $fixture 'dark' }
    $settings = [IO.Directory]::CreateDirectory((Join-Path $local 'DevSweep/settings'))
    JsonFile (Join-Path $settings.FullName 'clean-scan-roots-v1.json') @{schema_version=1;roots=@($project)}
    $language = if ($theme -eq 'light') { 'zh-CN' } else { 'en' }
    JsonFile (Join-Path $settings.FullName 'presentation-v1.json') @{schema_version=1;language=$language}
    if ($theme -eq 'light') {
        JsonFile (Join-Path $settings.FullName 'desktop-preferences-v2.json') @{schema_version=2;theme='light';font=@{kind='system'};text_scale_percent=125;motion='reduced';planet_fps=30;status_interval_seconds=2;status_process_limit=15;hud_interval_seconds=2}
    }
    $info = [Diagnostics.ProcessStartInfo]::new($binary)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    foreach ($key in @('LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) { $info.EnvironmentVariables[$key]=$local }
    if ($theme -eq 'solid') { $info.EnvironmentVariables['GPUI_DISABLE_DIRECT_COMPOSITION']='1' }
    $info.EnvironmentVariables['DEVSWEEP_ANALYZE_ROOT']=$project
    $trace=Join-Path $local 'present-us.txt'
    $info.EnvironmentVariables['DEVSWEEP_FRAME_TRACE']=$trace
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
        $dpi=[AiryWindow]::GetDpiForWindow($hwnd)
        $r=Rect
        Record "$theme dpi=$dpi size=$($r.right-$r.left)x$($r.bottom-$r.top) locale=$language"
        [void](Nodes)
        if ($Round -eq 'debug') {
            foreach ($i in 1,2,3,4,0,0) {
                $tabs=@(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
                $bounds=$tabs[$i].Current.BoundingRectangle
                $point=New-Object AiryWindow+Point
                $point.x=[int]($bounds.X+$bounds.Width/2); $point.y=[int]($bounds.Y+$bounds.Height/2)
                [void][AiryWindow]::ScreenToClient($hwnd,[ref]$point)
                $hit=($point.y -shl 16) -bor ($point.x -band 0xffff)
                [void][AiryWindow]::PostMessage($hwnd,0x200,[IntPtr]0,[IntPtr]$hit)
                [void][AiryWindow]::PostMessage($hwnd,0x201,[IntPtr]1,[IntPtr]$hit)
                [void][AiryWindow]::PostMessage($hwnd,0x202,[IntPtr]0,[IntPtr]$hit)
                Start-Sleep -Milliseconds 250
                $tabs=@(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
                Record "debug click=$($tabs[$i].Current.Name) selected=$($tabs[$i].GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty))"
            }
            continue
        }
        if ($theme -eq 'restored') {
            Key 0x79; Start-Sleep -Milliseconds 200
            $item=Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::MenuItem -and $_.Current.Name -eq 'Settings' } | Select-Object -First 1
            $item.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            Start-Sleep -Milliseconds 200
            $light=Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem -and $_.Current.Name -eq 'Light' } | Select-Object -First 1
            Record "restart light_selected=$($light.GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty))"
            continue
        }
        Capture 'landing'
        if ($theme -eq 'solid') { CompareBackdrop 'no-composition'; continue }
        # Alternate two flat backgrounds immediately behind our HWND. PrintWindow
        # may omit compositor effects; report pixel evidence separately from UIA.
        CompareBackdrop "$theme wide"
        $proc.Refresh(); $cpu=$proc.TotalProcessorTime.TotalMilliseconds
        Start-Sleep -Seconds 3
        $proc.Refresh(); Record "$theme idle_cpu_ms_over_3s=$($proc.TotalProcessorTime.TotalMilliseconds-$cpu)"
        Key 0x78 # F9, read-only scan of the isolated configured project
        for ($n=0;$n -lt 30;$n++) {
            Start-Sleep -Milliseconds 200
            $names=Names
            if ($names -match 'pytest|mypy') { break }
        }
        $reached=$names -match 'pytest|mypy'
        Record "$theme fixture_review=$reached"
        if ($reached) { Capture 'review' }
        if ($reached -and (Test-Path -LiteralPath $trace)) {
            $skip=@(Get-Content -LiteralPath $trace).Count
            $point=(([int]($r.top+($r.bottom-$r.top)/2) -shl 16) -bor ([int]($r.left+($r.right-$r.left)/2) -band 0xffff))
            for ($j=0;$j -lt 48;$j++) {
                $amount=if (($j % 16) -lt 8) {-120} else {120}
                [void][AiryWindow]::PostMessage($hwnd,0x20a,[IntPtr]($amount -shl 16),[IntPtr]$point)
                Start-Sleep -Milliseconds 16
            }
            Start-Sleep -Milliseconds 200
            $stamps=@(Get-Content -LiteralPath $trace | Select-Object -Skip $skip | ForEach-Object {[long]$_})
            $intervals=@(for ($j=1;$j -lt $stamps.Count;$j++) {$stamps[$j]-$stamps[$j-1]})
            if ($intervals.Count) {
                $sorted=@($intervals | Sort-Object)
                Record "$theme scroll_present_n=$($sorted.Count) p50_us=$($sorted[[math]::Ceiling($sorted.Count*.5)-1]) p95_us=$($sorted[[math]::Ceiling($sorted.Count*.95)-1])"
            } else { Record "$theme scroll_present_n=0" }
        }
        if ($reached -and $theme -eq 'dark' -and $Round -eq 'after') {
            $confirm=Button 'Confirm move*'
            if ($null -ne $confirm) { Record "before_preview confirm_enabled=$($confirm.Current.IsEnabled)" }
            $row=Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::ListItem -and $_.Current.Name -match 'pytest' } | Select-Object -First 1
            if ($null -ne $row) {
                $row.SetFocus(); Start-Sleep -Milliseconds 150; Key 0x0d; Start-Sleep -Milliseconds 250
                Record "selection_after_enter=$((Button 'Confirm move*').Current.Name)"
                $preview=Button 'Preview digest'
                if ($null -ne $preview) {
                    $preview.SetFocus(); Start-Sleep -Milliseconds 100; Key 0x0d
                    for ($j=0;$j -lt 15;$j++) { Start-Sleep -Milliseconds 150; if ((Names) -match 'sha256:') { break } }
                    $confirm=Button 'Confirm move*'
                    Record "preview digest_visible=$((Names) -match 'sha256:') confirm_enabled=$($confirm.Current.IsEnabled)"
                    $clear=Button 'Clear selection'
                    $clear.SetFocus(); Start-Sleep -Milliseconds 100; Key 0x0d
                    Start-Sleep -Milliseconds 200
                    $confirm=Button 'Confirm move*'
                    Record "after_clear confirm_enabled=$($confirm.Current.IsEnabled) fixture_intact=$(Test-Path -LiteralPath (Join-Path $project '.pytest_cache/sample.txt'))"
                }
            }
        }
        $elements=@(Nodes)
        $tabs=@($elements | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
        Record "$theme uia_tabs=$(($tabs | ForEach-Object {$_.Current.Name}) -join ',')"
        if ($tabs.Count -eq 5) {
            $tabs[1].SetFocus()
            Start-Sleep -Milliseconds 200
            Record "$theme selected_first=$($tabs[0].GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty)) focus_second=$($tabs[1].Current.HasKeyboardFocus)"
            Key 0x0d
            Start-Sleep -Milliseconds 400
            $tabs=@(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
            Record "$theme enter_selected_second=$($tabs[1].GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty))"
            Key 9
            Start-Sleep -Milliseconds 200
            $focus=[Windows.Automation.AutomationElement]::FocusedElement
            if ($focus.Current.ProcessId -eq $proc.Id) { Record "$theme tab_focus=$($focus.Current.Name)" }
            if ([AiryWindow]::GetForegroundWindow() -eq $hwnd -and $focus.Current.ProcessId -eq $proc.Id) {
                [Windows.Forms.SendKeys]::SendWait('+{TAB}')
                Start-Sleep -Milliseconds 150
                Record "$theme reverse_tab_returns=$($tabs[1].Current.HasKeyboardFocus)"
                Key 9; Start-Sleep -Milliseconds 100
            }
        }
        $max=@(Nodes | Where-Object { $_.Current.Name -eq 'Maximize' -and $_.Current.ControlType -eq [Windows.Automation.ControlType]::Button })
        if ($max.Count) {
            $max[0].GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            Start-Sleep -Milliseconds 350
            Record "$theme zoom_after_invoke=$([AiryWindow]::IsZoomed($hwnd))"
            $restore=@(Nodes | Where-Object { $_.Current.Name -eq 'Restore' -and $_.Current.ControlType -eq [Windows.Automation.ControlType]::Button })
            if ($restore.Count) { $restore[0].GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke(); Start-Sleep -Milliseconds 350; Record "$theme zoom_after_restore=$([AiryWindow]::IsZoomed($hwnd))" }
        }
        [void][AiryWindow]::SetWindowPos($hwnd,[IntPtr]::Zero,0,0,[int](820*$dpi/96),[int](680*$dpi/96),0x16)
        Start-Sleep -Milliseconds 400
        Capture 'compact'
        CompareBackdrop "$theme compact"
        if ($Round -eq 'after' -and $theme -eq 'dark') {
            foreach ($i in 2,3,4,0) {
                $tabs=@(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
                $name=$tabs[$i].Current.Name
                $tabs[$i].SetFocus(); Key 0x0d
                Start-Sleep -Milliseconds 300
                $tabs=@(Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem })
                Record "compact mode=$name selected=$($tabs[$i].GetCurrentPropertyValue([Windows.Automation.SelectionItemPattern]::IsSelectedProperty))"
                if ($i -ne 0) { Capture $name.ToLowerInvariant() }
            }
            foreach ($surface in 'Settings','Protection','Rules','History') {
                Key 0x79; Start-Sleep -Milliseconds 150
                $item=Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::MenuItem -and $_.Current.Name -like "$surface*" } | Select-Object -First 1
                if ($null -ne $item) {
                    $item.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
                    Start-Sleep -Milliseconds 220
                    $names=Names
                    Record "support=$surface title_visible=$($names -match $surface)"
                    if ($surface -eq 'Settings') {
                        Capture 'settings'
                        $light=Nodes | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::TabItem -and $_.Current.Name -eq 'Light' } | Select-Object -First 1
                        $light.SetFocus(); Start-Sleep -Milliseconds 120; Key 0x0d
                        Start-Sleep -Milliseconds 200
                        $save=Button 'Save'; $save.SetFocus(); Start-Sleep -Milliseconds 120; Key 0x0d
                        Start-Sleep -Milliseconds 300
                    }
                } else { Record "support=$surface menu_item_missing" }
                Key 0x1b; Start-Sleep -Milliseconds 150
            }
        }
    } catch { Record "$theme ERROR $($_.Exception.Message)" } finally {
        if (-not $proc.HasExited) { [void]$proc.CloseMainWindow(); if (-not $proc.WaitForExit(5000)) { $proc.Kill(); $proc.WaitForExit() } }
        Record "$theme exit=$($proc.ExitCode)"
        $proc.Dispose()
    }
}
[IO.File]::WriteAllLines((Join-Path $PSScriptRoot "$Round.txt"),$log,$utf8)
if ($log -match ' ERROR |fixture_review=False|focus_second=False|enter_selected_second=False|selected=False|digest_visible=False|menu_item_missing|title_visible=False') { exit 1 }
if ($Round -eq 'after' -and (
    $log -notcontains 'preview digest_visible=True confirm_enabled=True' -or
    $log -notcontains 'restart light_selected=True' -or
    $log -notcontains 'after_clear confirm_enabled=False fixture_intact=True'
)) { exit 1 }
