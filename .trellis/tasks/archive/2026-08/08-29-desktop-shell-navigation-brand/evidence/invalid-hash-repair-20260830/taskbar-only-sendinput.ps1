param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot,
    [int]$CdpPort = 9365,
    [string]$ExpectedHash = "DEC46FB5FC07338C84A16AAFF5A6412EFFC4EE9F5FCA6381C3ADDAEAAD54D604"
)

$ErrorActionPreference = "Stop"
$releaseExe = (Resolve-Path (Join-Path $RepositoryRoot "target/release/devsweep-desktop.exe")).Path
$localAppData = Join-Path $EvidenceRoot "localappdata"
$settingsRoot = Join-Path $localAppData "DevSweep/settings"
$screenshots = Join-Path $EvidenceRoot "screenshots"
$verificationPath = Join-Path $EvidenceRoot "verification.json"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
if (-not ("TaskbarOnlyNative" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TaskbarOnlyNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public UIntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; }
    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT { public uint type; public INPUTUNION union; }
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern IntPtr FindWindow(string className, string windowName);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int command);
    [DllImport("user32.dll")]
    public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    public static bool ClickOnce(int x, int y) {
        if (!SetCursorPos(x, y)) return false;
        INPUT down = new INPUT { type = 0, union = new INPUTUNION { mi = new MOUSEINPUT { dwFlags = 0x0002 } } };
        INPUT up = new INPUT { type = 0, union = new INPUTUNION { mi = new MOUSEINPUT { dwFlags = 0x0004 } } };
        INPUT[] inputs = new INPUT[] { down, up };
        return SendInput(2, inputs, Marshal.SizeOf(typeof(INPUT))) == 2;
    }
}
'@
}

function Get-ForegroundAudit {
    $hwnd = [TaskbarOnlyNative]::GetForegroundWindow()
    [uint32]$foregroundProcessId = 0
    if ($hwnd -ne [IntPtr]::Zero) {
        [TaskbarOnlyNative]::GetWindowThreadProcessId($hwnd, [ref]$foregroundProcessId) | Out-Null
    }
    return [ordered]@{ hwnd = "0x{0:X}" -f $hwnd; pid = $foregroundProcessId }
}

function Get-CursorAudit {
    $point = New-Object TaskbarOnlyNative+POINT
    if (-not [TaskbarOnlyNative]::GetCursorPos([ref]$point)) { throw "GetCursorPos failed" }
    return [ordered]@{ x = $point.X; y = $point.Y }
}

function Save-DesktopCapture {
    param([string]$Path)
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [System.Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try { $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size) }
    finally { $graphics.Dispose() }
    try { $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $bitmap.Dispose() }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{ path = $item.FullName; bytes = $item.Length; width = $bounds.Width; height = $bounds.Height; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }
}

function Save-WindowCapture {
    param([IntPtr]$WindowHandle, [string]$Path)
    $rect = New-Object TaskbarOnlyNative+RECT
    if (-not [TaskbarOnlyNative]::GetWindowRect($WindowHandle, [ref]$rect)) { throw "GetWindowRect failed for 0x$('{0:X}' -f $WindowHandle)" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "invalid capture bounds ${width}x${height}" }
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    try { $captured = [TaskbarOnlyNative]::PrintWindow($WindowHandle, $hdc, 2) }
    finally { $graphics.ReleaseHdc($hdc); $graphics.Dispose() }
    if (-not $captured) { $bitmap.Dispose(); throw "PrintWindow failed" }
    try { $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $bitmap.Dispose() }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{ path = $item.FullName; bytes = $item.Length; width = $width; height = $height; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }
}

function Get-DescendantProcesses {
    param([int]$RootPid)
    $all = @(Get-CimInstance Win32_Process)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootPid)
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($child in @($all | Where-Object ParentProcessId -eq $parent)) {
            if ($ids.Add([int]$child.ProcessId)) { $queue.Enqueue([int]$child.ProcessId) }
        }
    }
    return [ordered]@{
        ids = @($ids)
        rows = @($all | Where-Object { $ids.Contains([int]$_.ProcessId) } |
            Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
    }
}

function Wait-ForResidueZero {
    param([int]$RootPid, [int[]]$DescendantIds, [int]$Port)
    $samples = @()
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 250
        $all = @(Get-CimInstance Win32_Process)
        $main = @($all | Where-Object ProcessId -eq $RootPid)
        $descendants = @($all | Where-Object { $DescendantIds -contains [int]$_.ProcessId })
        $webviews = @($all | Where-Object {
            $_.Name -eq "msedgewebview2.exe" -and
            (($DescendantIds -contains [int]$_.ProcessId) -or
             ($DescendantIds -contains [int]$_.ParentProcessId) -or
             $_.CommandLine -match "mojo-named-platform-channel-pipe=$RootPid\.")
        })
        $listeners = @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
        $lockTemp = @(Get-ChildItem -LiteralPath $settingsRoot -File -Force -ErrorAction SilentlyContinue |
            Where-Object Name -ne "presentation-v1.json")
        $samples += [pscustomobject]@{
            time = (Get-Date).ToString("o")
            main = $main.Count
            descendants = $descendants.Count
            owned_webviews = $webviews.Count
            listeners = $listeners.Count
            lock_temp = $lockTemp.Count
        }
    } while (($main.Count -ne 0 -or $descendants.Count -ne 0 -or $webviews.Count -ne 0 -or
              $listeners.Count -ne 0 -or $lockTemp.Count -ne 0) -and (Get-Date) -lt $deadline)
    return [ordered]@{
        samples = $samples
        main = @($main | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        descendants = @($descendants | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        owned_webviews = @($webviews | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        listeners = @($listeners | Select-Object LocalAddress, LocalPort, OwningProcess)
        lock_temp = @($lockTemp | Select-Object Name, Length, FullName)
    }
}

New-Item -ItemType Directory -Force -Path $EvidenceRoot, $localAppData, $screenshots | Out-Null
$run = [ordered]@{
    started_at = (Get-Date).ToString("o")
    expected_hash = $ExpectedHash
    release = $releaseExe
    localappdata = $localAppData
    cdp_port = $CdpPort
    state = "started"
}
$process = $null
$closeAttempted = $false
$descendantAudit = $null

try {
    $hashBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    if ($hashBefore -ne $ExpectedHash) { throw "release hash drift before launch: $hashBefore" }
    if (@(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue).Count -ne 0) { throw "CDP port $CdpPort occupied before launch" }
    if (@(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue).Count -ne 0) { throw "isolated LOCALAPPDATA was not empty" }

    $oldLocal = $env:LOCALAPPDATA
    $oldArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    try {
        $env:LOCALAPPDATA = $localAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$CdpPort"
        $process = Start-Process -FilePath $releaseExe -PassThru
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldArgs
    }

    $deadline = (Get-Date).AddSeconds(40)
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
    } while (($process.MainWindowHandle -eq 0 -or $process.MainWindowTitle -ne "devsweep") -and -not $process.HasExited -and (Get-Date) -lt $deadline)
    if ($process.HasExited) { throw "release exited before taskbar audit" }
    if ($process.MainWindowHandle -eq 0) { throw "release HWND unavailable" }
    $appHwnd = [IntPtr]$process.MainWindowHandle
    $processPath = $process.Path
    $launchHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $processPath).Hash
    if ($processPath -ne $releaseExe -or $launchHash -ne $ExpectedHash) { throw "launch identity mismatch" }
    $run.launch = [ordered]@{
        pid = $process.Id
        start_time = $process.StartTime.ToString("o")
        path = $processPath
        hash = $launchHash
        hwnd = "0x{0:X}" -f $appHwnd
        title = $process.MainWindowTitle
    }

    $taskbarHwnd = [TaskbarOnlyNative]::FindWindow("Shell_TrayWnd", $null)
    if ($taskbarHwnd -eq [IntPtr]::Zero) { throw "Shell_TrayWnd unavailable" }
    $taskbarRoot = [System.Windows.Automation.AutomationElement]::FromHandle($taskbarHwnd)
    $elements = $taskbarRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $rows = @()
    $matchingElements = @()
    for ($index = 0; $index -lt $elements.Count; $index += 1) {
        $element = $elements.Item($index)
        $patterns = @($element.GetSupportedPatterns() | ForEach-Object ProgrammaticName)
        $row = [pscustomobject][ordered]@{
            index = $index
            name = $element.Current.Name
            automation_id = $element.Current.AutomationId
            control_type = $element.Current.ControlType.ProgrammaticName
            bounding_rectangle = $element.Current.BoundingRectangle.ToString()
            supported_patterns = $patterns
        }
        $rows += $row
        if ($row.name -eq "devsweep - 1 running window" -and $row.control_type -eq "ControlType.Button") {
            $matchingElements += $element
        }
    }
    $run.taskbar_enumeration = [ordered]@{
        shell_tray_wnd = "0x{0:X}" -f $taskbarHwnd
        candidate_count = $rows.Count
        candidates = $rows
        exact_match_count = $matchingElements.Count
    }
    if ($matchingElements.Count -ne 1) { throw "expected exactly one DevSweep taskbar Button, found $($matchingElements.Count)" }
    $button = $matchingElements[0]

    $minimizeRequestedAt = (Get-Date).ToString("o")
    [TaskbarOnlyNative]::ShowWindow($appHwnd, 6) | Out-Null
    $minimizeSamples = @()
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $foreground = Get-ForegroundAudit
        $iconic = [TaskbarOnlyNative]::IsIconic($appHwnd)
        $minimizeSamples += [pscustomobject]@{ time = (Get-Date).ToString("o"); foreground = $foreground; is_iconic = $iconic }
    } while ((-not $iconic -or $foreground.hwnd -eq ("0x{0:X}" -f $appHwnd)) -and (Get-Date) -lt $deadline)
    if (-not $iconic -or $foreground.hwnd -eq ("0x{0:X}" -f $appHwnd)) { throw "app did not become minimized and non-foreground" }

    $rect = $button.Current.BoundingRectangle
    $clickPoint = New-Object System.Windows.Point
    $clickablePointAvailable = $button.TryGetClickablePoint([ref]$clickPoint)
    if (-not $clickablePointAvailable) {
        $clickPoint = [System.Windows.Point]::new($rect.Left + ($rect.Width / 2), $rect.Top + ($rect.Height / 2))
    }
    $beforeClick = [ordered]@{
        time = (Get-Date).ToString("o")
        cursor = Get-CursorAudit
        foreground = Get-ForegroundAudit
        is_iconic = [TaskbarOnlyNative]::IsIconic($appHwnd)
    }
    $clickedAt = (Get-Date).ToString("o")
    $sent = [TaskbarOnlyNative]::ClickOnce([Math]::Round($clickPoint.X), [Math]::Round($clickPoint.Y))
    if (-not $sent) { throw "SetCursorPos + SendInput failed" }
    $activationSamples = @()
    $deadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $foreground = Get-ForegroundAudit
        $iconic = [TaskbarOnlyNative]::IsIconic($appHwnd)
        $activationSamples += [pscustomobject]@{ time = (Get-Date).ToString("o"); foreground = $foreground; is_iconic = $iconic }
    } while (($foreground.hwnd -ne ("0x{0:X}" -f $appHwnd) -or $iconic) -and (Get-Date) -lt $deadline)
    if ($foreground.hwnd -ne ("0x{0:X}" -f $appHwnd) -or $iconic) { throw "taskbar SendInput did not restore the exact app HWND to foreground" }
    [uint32]$foregroundPid = 0
    [TaskbarOnlyNative]::GetWindowThreadProcessId($appHwnd, [ref]$foregroundPid) | Out-Null
    if ($foregroundPid -ne $process.Id) { throw "foreground HWND PID mismatch: $foregroundPid != $($process.Id)" }
    $foregroundProcess = Get-Process -Id $foregroundPid -ErrorAction Stop
    $foregroundPath = $foregroundProcess.Path
    $foregroundHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $foregroundPath).Hash
    if ($foregroundPath -ne $releaseExe -or $foregroundHash -ne $ExpectedHash) { throw "foreground executable identity mismatch" }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
    $captureStarted = (Get-Date).ToString("o")
    $desktopCapture = Save-DesktopCapture -Path (Join-Path $screenshots "taskbar-bound-desktop-$timestamp.png")
    $taskbarCapture = Save-WindowCapture -WindowHandle $taskbarHwnd -Path (Join-Path $screenshots "taskbar-bound-crop-$timestamp.png")
    $appCapture = Save-WindowCapture -WindowHandle $appHwnd -Path (Join-Path $screenshots "taskbar-bound-app-reference-$timestamp.png")
    $captureFinished = (Get-Date).ToString("o")
    $run.binding = [ordered]@{
        minimize_requested_at = $minimizeRequestedAt
        minimize_samples = $minimizeSamples
        clickable_point_available = $clickablePointAvailable
        click_point = [ordered]@{ x = $clickPoint.X; y = $clickPoint.Y }
        before_click = $beforeClick
        clicked_at = $clickedAt
        after_click_time = (Get-Date).ToString("o")
        cursor_after = Get-CursorAudit
        activation_samples = $activationSamples
        foreground = [ordered]@{
            hwnd = "0x{0:X}" -f $appHwnd
            pid = $foregroundPid
            path = $foregroundPath
            hash = $foregroundHash
            is_iconic = [TaskbarOnlyNative]::IsIconic($appHwnd)
        }
        capture_timestamp = $timestamp
        capture_started = $captureStarted
        capture_finished = $captureFinished
        desktop_capture = $desktopCapture
        taskbar_capture = $taskbarCapture
        app_reference_capture = $appCapture
    }

    $descendantAudit = Get-DescendantProcesses -RootPid $process.Id
    $closeStarted = Get-Date
    $closeResult = $process.CloseMainWindow()
    $closeAttempted = $true
    $residue = Wait-ForResidueZero -RootPid $process.Id -DescendantIds $descendantAudit.ids -Port $CdpPort
    $run.close = [ordered]@{
        close_main_window = $closeResult
        elapsed_ms = [Math]::Round(((Get-Date) - $closeStarted).TotalMilliseconds)
        descendants_before = $descendantAudit.rows
        residue = $residue
    }
    if (-not $closeResult -or $residue.main.Count -ne 0 -or $residue.descendants.Count -ne 0 -or
        $residue.owned_webviews.Count -ne 0 -or $residue.listeners.Count -ne 0 -or $residue.lock_temp.Count -ne 0) {
        throw "natural close left task-owned residue"
    }
    $hashAfter = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    if ($hashAfter -ne $ExpectedHash) { throw "release hash drift after close: $hashAfter" }
    $run.final_hash = $hashAfter
    $run.state = "pass"
}
catch {
    $run.state = "failed"
    $run.failure = [ordered]@{
        message = $_.Exception.Message
        position = $_.InvocationInfo.PositionMessage
        stack = $_.ScriptStackTrace
    }
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        try {
            if (-not $closeAttempted) {
                $descendantAudit = Get-DescendantProcesses -RootPid $process.Id
                $run.failure_close_main_window = $process.CloseMainWindow()
                $closeAttempted = $true
            }
            $failureResidue = Wait-ForResidueZero -RootPid $process.Id -DescendantIds $descendantAudit.ids -Port $CdpPort
            $run.failure_residue = $failureResidue
            $process.Refresh()
            if (-not $process.HasExited) {
                $live = Get-Process -Id $process.Id -ErrorAction Stop
                $livePath = $live.Path
                $liveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $livePath).Hash
                if ($livePath -eq $releaseExe -and $liveHash -eq $ExpectedHash) {
                    Stop-Process -Id $process.Id -ErrorAction Stop
                    $process.WaitForExit(5000) | Out-Null
                    $run.identity_gated_residual_termination = [ordered]@{
                        command = "Stop-Process -Id $($process.Id)"
                        path = $livePath
                        hash = $liveHash
                        alive_after = -not $process.HasExited
                    }
                }
            }
        }
        catch { $run.failure_cleanup_error = $_.Exception.Message }
    }
    $run.finished_at = (Get-Date).ToString("o")
    $run | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $verificationPath -Encoding utf8
}

if ($run.state -ne "pass") { exit 1 }
exit 0
