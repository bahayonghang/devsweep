param(
    [Parameter(Mandatory=$true)][int]$AppPid,
    [Parameter(Mandatory=$true)][string]$Executable,
    [ValidateSet('State','Focus','Restore','Position','Drag','DoubleClick','Resize','WinLeft','WinUp','WinDown','AltF4','TaskbarRestore','TrayToggle')][string]$Action='State',
    [long]$Handle=0
)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class TaskWindow {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx,dy; public uint mouseData,dwFlags,time; public UIntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk,wScan; public uint dwFlags,time; public UIntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public UNION value; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out RECT rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd,out RECT rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd,ref POINT point);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd,int command);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int width,int height,uint flags);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
    [DllImport("user32.dll",SetLastError=true)] public static extern uint SendInput(uint count,INPUT[] inputs,int size);
    public static void Mouse(uint flags) { INPUT v=new INPUT{type=0,value=new UNION{mi=new MOUSEINPUT{dwFlags=flags}}}; if(SendInput(1,new[]{v},Marshal.SizeOf(typeof(INPUT)))!=1)throw new Exception("Mouse input rejected"); }
    public static void Key(ushort key,bool up) { INPUT v=new INPUT{type=1,value=new UNION{ki=new KEYBDINPUT{wVk=key,dwFlags=up?2u:0u}}}; if(SendInput(1,new[]{v},Marshal.SizeOf(typeof(INPUT)))!=1)throw new Exception("Key input rejected"); }
}
'@
$app=Get-Process -Id $AppPid
if ($app.Path -ne (Resolve-Path -LiteralPath $Executable).Path) { throw 'Executable identity mismatch' }
$app.Refresh()
if ($Handle -eq 0) { $Handle=$app.MainWindowHandle.ToInt64() }
$hwnd=[IntPtr]$Handle
[uint32]$owner=0
[void][TaskWindow]::GetWindowThreadProcessId($hwnd,[ref]$owner)
if ($owner -ne $AppPid) { throw 'Window owner mismatch' }
function State {
    $r=[TaskWindow+RECT]::new(); $c=[TaskWindow+RECT]::new()
    [void][TaskWindow]::GetWindowRect($hwnd,[ref]$r)
    [void][TaskWindow]::GetClientRect($hwnd,[ref]$c)
    [ordered]@{pid=$AppPid;hwnd=$Handle;dpi=[TaskWindow]::GetDpiForWindow($hwnd);minimized=[TaskWindow]::IsIconic($hwnd);maximized=[TaskWindow]::IsZoomed($hwnd);foreground=([TaskWindow]::GetForegroundWindow() -eq $hwnd);rect=@($r.Left,$r.Top,$r.Right,$r.Bottom);client=@($c.Right,$c.Bottom)}
}
$before=State
if ($Action -notin @('State','Restore','TaskbarRestore','TrayToggle')) {
    [void][TaskWindow]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 200
    if ([TaskWindow]::GetForegroundWindow() -ne $hwnd) { throw 'Owned window is not foreground; no input sent' }
}
$cursor=[TaskWindow+POINT]::new()
[void][TaskWindow]::GetCursorPos([ref]$cursor)
try {
    switch ($Action) {
        'Restore' { [void][TaskWindow]::ShowWindow($hwnd,9) }
        'Position' { [void][TaskWindow]::SetWindowPos($hwnd,[IntPtr]::Zero,100,100,0,0,0x0001 -bor 0x0004) }
        { $_ -in 'Drag','DoubleClick','Resize' } {
            $p=[TaskWindow+POINT]::new(); [void][TaskWindow]::ClientToScreen($hwnd,[ref]$p)
            $scale=$before.dpi/96.0
            $x=$p.X+[int](200*$scale); $y=$p.Y+[int](20*$scale)
            if ($Action -eq 'Resize') { $x=$before.rect[2]-2; $y=[int](($before.rect[1]+$before.rect[3])/2) }
            [void][TaskWindow]::SetCursorPos($x,$y)
            if ($Action -eq 'DoubleClick') {
                1..2 | ForEach-Object { [TaskWindow]::Mouse(2); Start-Sleep -Milliseconds 35; [TaskWindow]::Mouse(4); Start-Sleep -Milliseconds 70 }
            } else {
                [TaskWindow]::Mouse(2)
                try {
                    Start-Sleep -Milliseconds 120
                    foreach ($step in 1..10) {
                        $dy=if ($Action -eq 'Drag') { $step*4 } else { 0 }
                        [void][TaskWindow]::SetCursorPos($x+$step*8,$y+$dy)
                        Start-Sleep -Milliseconds 30
                    }
                } finally { [TaskWindow]::Mouse(4) }
            }
        }
        { $_ -in 'WinLeft','WinUp','WinDown','AltF4' } {
            $modifier=if ($Action -eq 'AltF4') { 0x12 } else { 0x5B }
            $key=switch ($Action) { WinLeft {0x25}; WinUp {0x26}; WinDown {0x28}; AltF4 {0x73} }
            [TaskWindow]::Key($modifier,$false)
            try { [TaskWindow]::Key($key,$false); [TaskWindow]::Key($key,$true) } finally { [TaskWindow]::Key($modifier,$true) }
        }
        { $_ -in 'TaskbarRestore','TrayToggle' } {
            if (@(Get-Process -Name $app.ProcessName).Count -ne 1) { throw 'More than one DevSweep process; taskbar target is ambiguous' }
            Add-Type -AssemblyName UIAutomationClient
            Add-Type -AssemblyName UIAutomationTypes
            $bar=[TaskWindow]::FindWindow('Shell_TrayWnd',$null)
            $root=[System.Windows.Automation.AutomationElement]::FromHandle($bar)
            $items=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
            $candidates=@()
            foreach ($item in $items) {
                $name=$item.Current.Name
                if ($name -notmatch '(?i)devsweep' -or -not $item.Current.IsEnabled -or $item.Current.IsOffscreen) { continue }
                $isTray=$item.Current.ClassName -match 'SystemTray' -or $item.Current.AutomationId -match 'SystemTray'
                if (($Action -eq 'TrayToggle' -and $isTray) -or ($Action -eq 'TaskbarRestore' -and $name -match '(?i)running window|运行窗口|个窗口')) { $candidates += $item }
            }
            if ($candidates.Count -ne 1) { throw "Expected one taskbar target; found $($candidates.Count)" }
            $rect=$candidates[0].Current.BoundingRectangle
            [void][TaskWindow]::SetCursorPos([int]($rect.Left+$rect.Width/2),[int]($rect.Top+$rect.Height/2))
            [TaskWindow]::Mouse(2); [TaskWindow]::Mouse(4)
        }
    }
    Start-Sleep -Milliseconds 600
} finally { [void][TaskWindow]::SetCursorPos($cursor.X,$cursor.Y) }
$exited=$null -eq (Get-Process -Id $AppPid -ErrorAction SilentlyContinue)
[ordered]@{action=$Action;before=$before;after=$(if ($exited) {$null} else {State});exited=$exited} | ConvertTo-Json -Depth 5 -Compress
