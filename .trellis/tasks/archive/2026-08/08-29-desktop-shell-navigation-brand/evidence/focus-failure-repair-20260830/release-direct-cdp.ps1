param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot,
    [int]$FirstPort = 9351,
    [int]$RestartPort = 9352,
    [string]$ExpectedHash = "6A54C2C752969045B247251A87FB6053271985511DEF1DC2D663088F6B530DC6"
)

$ErrorActionPreference = "Stop"
$expectedHash = $ExpectedHash
$expectedBytes = [System.Text.Encoding]::UTF8.GetBytes('{"schema_version":1,"language":"zh-CN"}')
$releaseExe = (Resolve-Path (Join-Path $RepositoryRoot "target/release/devsweep-desktop.exe")).Path
$localAppData = Join-Path $EvidenceRoot "localappdata"
$settingsRoot = Join-Path $localAppData "DevSweep/settings"
$storePath = Join-Path $settingsRoot "presentation-v1.json"
$script:CdpRequestId = 0
$script:CdpNotifications = [System.Collections.Generic.List[object]]::new()
$script:ClosedPids = [System.Collections.Generic.HashSet[int]]::new()

function Receive-CdpMessage {
    param([System.Net.WebSockets.ClientWebSocket]$Socket)
    $memory = [System.IO.MemoryStream]::new()
    try {
        do {
            $buffer = [byte[]]::new(65536)
            $segment = [System.ArraySegment[byte]]::new($buffer)
            $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
            try { $received = $Socket.ReceiveAsync($segment, $cts.Token).GetAwaiter().GetResult() }
            finally { $cts.Dispose() }
            if ($received.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
                throw "CDP websocket closed before the expected response"
            }
            $memory.Write($buffer, 0, $received.Count)
        } while (-not $received.EndOfMessage)
        return [System.Text.Encoding]::UTF8.GetString($memory.ToArray()) | ConvertFrom-Json -Depth 100
    }
    finally { $memory.Dispose() }
}

function Invoke-Cdp {
    param(
        [System.Net.WebSockets.ClientWebSocket]$Socket,
        [string]$Method,
        [hashtable]$Params = @{}
    )
    $script:CdpRequestId += 1
    $requestId = $script:CdpRequestId
    $request = [ordered]@{ id = $requestId; method = $Method; params = $Params } |
        ConvertTo-Json -Depth 30 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
    $segment = [System.ArraySegment[byte]]::new($bytes)
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try {
        $Socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()
    }
    finally { $cts.Dispose() }
    while ($true) {
        $message = Receive-CdpMessage -Socket $Socket
        if ($null -ne $message.id -and [int]$message.id -eq $requestId) {
            if ($null -ne $message.error) { throw "CDP $Method failed: $($message.error | ConvertTo-Json -Compress)" }
            return $message.result
        }
        $script:CdpNotifications.Add($message)
    }
}

function Invoke-CdpEvaluation {
    param([System.Net.WebSockets.ClientWebSocket]$Socket, [string]$Expression)
    $response = Invoke-Cdp -Socket $Socket -Method "Runtime.evaluate" -Params @{
        expression = $Expression
        awaitPromise = $true
        returnByValue = $true
        userGesture = $true
    }
    if ($null -ne $response.exceptionDetails) {
        throw "Runtime.evaluate exception: $($response.exceptionDetails | ConvertTo-Json -Depth 20 -Compress)"
    }
    return $response.result.value
}

function Get-DescendantProcesses {
    param([int]$RootPid)
    $all = Get-CimInstance Win32_Process
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootPid)
    while ($queue.Count -gt 0) {
        $parentId = $queue.Dequeue()
        foreach ($child in ($all | Where-Object ParentProcessId -eq $parentId)) {
            if ($ids.Add([int]$child.ProcessId)) { $queue.Enqueue([int]$child.ProcessId) }
        }
    }
    return [ordered]@{
        ids = @($ids)
        rows = @($all | Where-Object { $ids.Contains([int]$_.ProcessId) } |
            Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
    }
}

function Save-WindowCapture {
    param([IntPtr]$WindowHandle, [string]$Path)
    Add-Type -AssemblyName System.Drawing
    if (-not ("ReleaseDirectCdpNative" -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ReleaseDirectCdpNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
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
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public UIntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; }
    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT { public uint type; public INPUTUNION union; }
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    public static bool ClickAt(int x, int y) {
        if (!SetCursorPos(x, y)) return false;
        INPUT down = new INPUT { type = 0, union = new INPUTUNION { mi = new MOUSEINPUT { dwFlags = 0x0002 } } };
        INPUT up = new INPUT { type = 0, union = new INPUTUNION { mi = new MOUSEINPUT { dwFlags = 0x0004 } } };
        INPUT[] inputs = new INPUT[] { down, up };
        return SendInput(2, inputs, Marshal.SizeOf(typeof(INPUT))) == 2;
    }
}
'@
    }
    $rect = New-Object ReleaseDirectCdpNative+RECT
    if (-not [ReleaseDirectCdpNative]::GetWindowRect($WindowHandle, [ref]$rect)) { throw "GetWindowRect failed" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    try { $captured = [ReleaseDirectCdpNative]::PrintWindow($WindowHandle, $hdc, 2) }
    finally { $graphics.ReleaseHdc($hdc); $graphics.Dispose() }
    if (-not $captured) { $bitmap.Dispose(); throw "PrintWindow failed" }
    try { $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $bitmap.Dispose() }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{ path = $Path; width = $width; height = $height; bytes = $item.Length; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }
}

function Save-DesktopCapture {
    param([string]$Path)
    Add-Type -AssemblyName System.Drawing
    Add-Type -AssemblyName System.Windows.Forms
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = [System.Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try { $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size) }
    finally { $graphics.Dispose() }
    try { $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $bitmap.Dispose() }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{ path = $Path; width = $bounds.Width; height = $bounds.Height; bytes = $item.Length; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }
}

function Start-ReleaseRun {
    param([int]$Port, [string]$Name)
    if (@(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "CDP port $Port was occupied before $Name"
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash -ne $expectedHash) { throw "release hash drift before $Name" }
    $oldLocal = $env:LOCALAPPDATA
    $oldBrowserArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    try {
        $env:LOCALAPPDATA = $localAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
        $process = Start-Process -FilePath $releaseExe -PassThru
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArgs
    }
    $deadline = (Get-Date).AddSeconds(40)
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        try { $targets = @(Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 2) }
        catch { $targets = @() }
        $matching = @($targets | Where-Object { $_.type -eq "page" -and $_.title -eq "devsweep" -and $_.url -notin @("about:blank", "") })
    } while (($process.MainWindowHandle -eq 0 -or $matching.Count -ne 1) -and (Get-Date) -lt $deadline)
    if ($process.HasExited -or $process.MainWindowHandle -eq 0 -or $matching.Count -ne 1) { throw "$Name did not expose one native DevSweep target" }
    $listeners = @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    if ($listeners.Count -ne 1 -or $listeners[0].LocalAddress -notin @("127.0.0.1", "::1")) { throw "$Name CDP listener was not unique loopback" }
    $socket = [System.Net.WebSockets.ClientWebSocket]::new()
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try { $socket.ConnectAsync([Uri]$matching[0].webSocketDebuggerUrl, $cts.Token).GetAwaiter().GetResult() }
    finally { $cts.Dispose() }
    $script:CdpNotifications = [System.Collections.Generic.List[object]]::new()
    Invoke-Cdp -Socket $socket -Method "Runtime.enable" | Out-Null
    Invoke-Cdp -Socket $socket -Method "Page.enable" | Out-Null
    Invoke-Cdp -Socket $socket -Method "Log.enable" | Out-Null
    Invoke-CdpEvaluation -Socket $socket -Expression @'
(() => {
  window.__releaseDirectCdp = { unhandled: [], errors: [] };
  window.addEventListener('unhandledrejection', event => window.__releaseDirectCdp.unhandled.push(String(event.reason)));
  window.addEventListener('error', event => window.__releaseDirectCdp.errors.push(String(event.error ?? event.message)));
  return true;
})()
'@ | Out-Null
    return [pscustomobject]@{
        Name = $Name
        Port = $Port
        Process = $process
        Socket = $socket
        Target = $matching[0]
        Listener = $listeners[0]
        Pid = $process.Id
        Hwnd = $process.MainWindowHandle
        StartTime = $process.StartTime.ToString("o")
        Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    }
}

function Get-ReleaseState {
    param([System.Net.WebSockets.ClientWebSocket]$Socket)
    return Invoke-CdpEvaluation -Socket $Socket -Expression @'
(() => {
  const active = document.activeElement;
  const select = document.querySelector('.settings-panel select');
  return {
    hash: location.hash,
    locale: document.querySelector('[data-locale]')?.getAttribute('data-locale') ?? null,
    activeTag: active?.tagName ?? null,
    activeText: active?.textContent?.trim() ?? null,
    cleanLabel: Array.from(document.querySelectorAll('[role="tab"]')).find(tab => ['Clean','清理'].includes(tab.textContent?.trim() ?? ''))?.textContent?.trim() ?? null,
    languageButtons: Array.from(document.querySelectorAll('button')).filter(button => ['Language','语言'].includes(button.textContent?.trim() ?? '')).map(button => button.textContent?.trim()),
    selectPresent: Boolean(select),
    selectValue: select?.value ?? null,
    unhandled: window.__releaseDirectCdp?.unhandled ?? [],
    windowErrors: window.__releaseDirectCdp?.errors ?? []
  };
})()
'@
}

function Close-ReleaseRun {
    param([pscustomobject]$Run)
    if ($Run.Socket.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
        $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(3))
        try { $Run.Socket.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "evidence complete", $cts.Token).GetAwaiter().GetResult() | Out-Null }
        finally { $cts.Dispose() }
    }
    $Run.Socket.Dispose()
    $descendants = Get-DescendantProcesses -RootPid $Run.Pid
    $started = Get-Date
    $closeResult = $Run.Process.CloseMainWindow()
    $script:ClosedPids.Add([int]$Run.Pid) | Out-Null
    $Run.Process.WaitForExit(25000) | Out-Null
    $elapsed = [Math]::Round(((Get-Date) - $started).TotalMilliseconds)
    $samples = @()
    $residueDeadline = (Get-Date).AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 250
        $allProcesses = @(Get-CimInstance Win32_Process)
        $mainResidue = @($allProcesses | Where-Object ProcessId -eq $Run.Pid |
            Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        $remaining = @($allProcesses | Where-Object { $descendants.ids -contains [int]$_.ProcessId } |
            Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        $ownedWebViews = @($allProcesses | Where-Object {
            $_.Name -eq "msedgewebview2.exe" -and
            (($descendants.ids -contains [int]$_.ProcessId) -or
             ($descendants.ids -contains [int]$_.ParentProcessId) -or
             $_.CommandLine -match "mojo-named-platform-channel-pipe=$($Run.Pid)\.")
        } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        $listeners = @(Get-NetTCPConnection -LocalPort $Run.Port -State Listen -ErrorAction SilentlyContinue)
        $lockTemp = @(Get-ChildItem -LiteralPath $settingsRoot -File -Force -ErrorAction SilentlyContinue |
            Where-Object Name -ne "presentation-v1.json" | Select-Object Name, Length, FullName)
        $vite = @($allProcesses | Where-Object {
            $_.Name -in @("node.exe", "cmd.exe") -and
            $_.CommandLine -match "vite" -and
            $_.CommandLine -match "devsweep|4180"
        } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        $samples += [pscustomobject]@{
            time = (Get-Date).ToString("o")
            main_count = $mainResidue.Count
            descendant_count = $remaining.Count
            owned_webview_count = $ownedWebViews.Count
            listener_count = $listeners.Count
            lock_temp_count = $lockTemp.Count
            vite_count = $vite.Count
        }
    } while (($mainResidue.Count -ne 0 -or $remaining.Count -ne 0 -or $ownedWebViews.Count -ne 0 -or
              $listeners.Count -ne 0 -or $lockTemp.Count -ne 0 -or $vite.Count -ne 0) -and
             (Get-Date) -lt $residueDeadline)
    return [ordered]@{
        close_main_window = $closeResult
        elapsed_ms = $elapsed
        main_alive = -not $Run.Process.HasExited
        descendants_before = $descendants.rows
        residue_samples = $samples
        main_residue = $mainResidue
        descendant_residue = $remaining
        owned_webview_residue = $ownedWebViews
        listener_residue = $listeners | Select-Object LocalAddress, LocalPort, OwningProcess
        lock_temp_residue = $lockTemp
        vite_residue = $vite
    }
}

function Get-StoreAudit {
    if (-not (Test-Path -LiteralPath $storePath)) { return [ordered]@{ exists = $false } }
    $bytes = [System.IO.File]::ReadAllBytes($storePath)
    $files = @(Get-ChildItem -LiteralPath $settingsRoot -File -Force -ErrorAction SilentlyContinue)
    return [ordered]@{
        exists = $true
        path = $storePath
        length = $bytes.Length
        utf8 = [System.Text.Encoding]::UTF8.GetString($bytes)
        base64 = [Convert]::ToBase64String($bytes)
        expected_base64 = [Convert]::ToBase64String($expectedBytes)
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $storePath).Hash
        files = $files | Select-Object Name, Length, FullName
        lock_temp_count = @($files | Where-Object Name -ne "presentation-v1.json").Count
    }
}

function Get-TaskbarBinding {
    param([int]$ExpectedPid, [IntPtr]$ExpectedHwnd)
    $result = [ordered]@{ state = "UNVERIFIED"; candidates = @(); expected_pid = $ExpectedPid; expected_hwnd = "0x{0:X}" -f $ExpectedHwnd }
    try {
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        $taskbarHwnd = [ReleaseDirectCdpNative]::FindWindow("Shell_TrayWnd", $null)
        if ($taskbarHwnd -eq [IntPtr]::Zero) { throw "Shell_TrayWnd was unavailable" }
        $result.shell_tray_wnd = "0x{0:X}" -f $taskbarHwnd
        $taskbarRoot = [System.Windows.Automation.AutomationElement]::FromHandle($taskbarHwnd)
        $found = $taskbarRoot.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        $rows = @()
        $matchingElements = @()
        for ($index = 0; $index -lt $found.Count; $index += 1) {
            $element = $found.Item($index)
            $patterns = @($element.GetSupportedPatterns() | ForEach-Object ProgrammaticName)
            $row = [pscustomobject][ordered]@{
                index = $index
                name = $element.Current.Name
                control_type = $element.Current.ControlType.ProgrammaticName
                automation_id = $element.Current.AutomationId
                class_name = $element.Current.ClassName
                is_enabled = $element.Current.IsEnabled
                bounding_rectangle = $element.Current.BoundingRectangle.ToString()
                supported_patterns = $patterns
            }
            $rows += $row
            if ($row.name -eq "devsweep - 1 running window" -and $row.control_type -eq "ControlType.Button") {
                $matchingElements += $element
            }
        }
        $result.candidates = $rows
        $result.exact_match_count = $matchingElements.Count
        if ($matchingElements.Count -ne 1) { throw "expected exactly one DevSweep taskbar Button, found $($matchingElements.Count)" }
        $button = $matchingElements[0]

        $result.minimize_requested_at = (Get-Date).ToString("o")
        [ReleaseDirectCdpNative]::ShowWindow($ExpectedHwnd, 6) | Out-Null
        $result.minimize_samples = @()
        $deadline = (Get-Date).AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $foreground = [ReleaseDirectCdpNative]::GetForegroundWindow()
            $iconic = [ReleaseDirectCdpNative]::IsIconic($ExpectedHwnd)
            $result.minimize_samples += [pscustomobject]@{
                time = (Get-Date).ToString("o")
                foreground_hwnd = "0x{0:X}" -f $foreground
                is_iconic = $iconic
            }
        } while ((-not $iconic -or $foreground -eq $ExpectedHwnd) -and (Get-Date) -lt $deadline)
        if (-not $iconic -or $foreground -eq $ExpectedHwnd) { throw "app did not become minimized and non-foreground" }

        $rect = $button.Current.BoundingRectangle
        $point = New-Object System.Windows.Point
        $result.clickable_point_available = $button.TryGetClickablePoint([ref]$point)
        if (-not $result.clickable_point_available) {
            $point = [System.Windows.Point]::new($rect.Left + ($rect.Width / 2), $rect.Top + ($rect.Height / 2))
        }
        $cursorBefore = New-Object ReleaseDirectCdpNative+POINT
        [ReleaseDirectCdpNative]::GetCursorPos([ref]$cursorBefore) | Out-Null
        [uint32]$beforePid = 0
        [ReleaseDirectCdpNative]::GetWindowThreadProcessId($foreground, [ref]$beforePid) | Out-Null
        $result.before_click = [ordered]@{
            time = (Get-Date).ToString("o")
            cursor_x = $cursorBefore.X
            cursor_y = $cursorBefore.Y
            foreground_hwnd = "0x{0:X}" -f $foreground
            foreground_pid = $beforePid
            is_iconic = [ReleaseDirectCdpNative]::IsIconic($ExpectedHwnd)
        }
        $result.click_point = [ordered]@{ x = $point.X; y = $point.Y }
        $result.clicked_at = (Get-Date).ToString("o")
        if (-not [ReleaseDirectCdpNative]::ClickAt([Math]::Round($point.X), [Math]::Round($point.Y))) { throw "SetCursorPos + SendInput failed" }
        $result.activation = "SetCursorPos+SendInput"
        $result.activation_samples = @()
        $deadline = (Get-Date).AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $foreground = [ReleaseDirectCdpNative]::GetForegroundWindow()
            $iconic = [ReleaseDirectCdpNative]::IsIconic($ExpectedHwnd)
            [uint32]$samplePid = 0
            [ReleaseDirectCdpNative]::GetWindowThreadProcessId($foreground, [ref]$samplePid) | Out-Null
            $result.activation_samples += [pscustomobject]@{
                time = (Get-Date).ToString("o")
                foreground_hwnd = "0x{0:X}" -f $foreground
                foreground_pid = $samplePid
                is_iconic = $iconic
            }
        } while (($foreground -ne $ExpectedHwnd -or $iconic) -and (Get-Date) -lt $deadline)
        if ($foreground -ne $ExpectedHwnd -or $iconic) { throw "taskbar SendInput did not restore exact app HWND" }
        [uint32]$foregroundPid = 0
        [ReleaseDirectCdpNative]::GetWindowThreadProcessId($foreground, [ref]$foregroundPid) | Out-Null
        if ($foregroundPid -ne $ExpectedPid) { throw "foreground PID mismatch" }
        $path = (Get-Process -Id $foregroundPid -ErrorAction Stop).Path
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
        if ($path -ne $releaseExe -or $hash -ne $expectedHash) { throw "foreground executable identity mismatch" }
        $result.bound = [ordered]@{
            foreground_hwnd = "0x{0:X}" -f $foreground
            foreground_pid = $foregroundPid
            foreground_path = $path
            foreground_hash = $hash
            is_iconic = [ReleaseDirectCdpNative]::IsIconic($ExpectedHwnd)
        }
        $timestamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
        $result.capture_id = $timestamp
        $result.capture_started = (Get-Date).ToString("o")
        $result.desktop_capture = Save-DesktopCapture -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-desktop-$timestamp.png")
        $result.taskbar_capture = Save-WindowCapture -WindowHandle $taskbarHwnd -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-crop-$timestamp.png")
        $result.app_reference_capture = Save-WindowCapture -WindowHandle $ExpectedHwnd -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-app-reference-$timestamp.png")
        $result.capture_finished = (Get-Date).ToString("o")
        $result.state = "PASS"
    }
    catch { $result.error = $_.Exception.Message }
    return $result
}

New-Item -ItemType Directory -Force -Path $EvidenceRoot, $localAppData, (Join-Path $EvidenceRoot "screenshots") | Out-Null
$run = [ordered]@{ started_at = (Get-Date).ToString("o"); release = $releaseExe; expected_hash = $expectedHash; localappdata = $localAppData; state = "started" }
$first = $null
$restart = $null
try {
    if (@(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue).Count -ne 0) { throw "isolated LOCALAPPDATA was not empty" }
    $first = Start-ReleaseRun -Port $FirstPort -Name "first"
    Start-Sleep -Milliseconds 750
    $initial = Get-ReleaseState -Socket $first.Socket
    if ($initial.locale -ne "en" -or $initial.hash -ne "#/clean" -or $initial.languageButtons -notcontains "Language") { throw "unexpected initial English state" }
    $openSettings = Invoke-CdpEvaluation -Socket $first.Socket -Expression @'
(() => { const button = Array.from(document.querySelectorAll('button')).find(item => item.textContent?.trim() === 'Language'); if (!button) return false; button.focus(); button.click(); return true; })()
'@
    if (-not $openSettings) { throw "English Language button was unavailable" }
    Start-Sleep -Milliseconds 500
    $settingsEn = Get-ReleaseState -Socket $first.Socket
    if ($settingsEn.hash -ne "#/settings" -or -not $settingsEn.selectPresent -or $settingsEn.selectValue -ne "en") { throw "English Settings did not open" }
    $selectZh = Invoke-CdpEvaluation -Socket $first.Socket -Expression @'
(() => { const select = document.querySelector('.settings-panel select'); if (!select) return false; select.value = 'zh-CN'; select.dispatchEvent(new Event('change', { bubbles: true })); return true; })()
'@
    if (-not $selectZh) { throw "language select was unavailable" }
    $deadline = (Get-Date).AddSeconds(10)
    do { Start-Sleep -Milliseconds 250; $zhState = Get-ReleaseState -Socket $first.Socket } while (($zhState.locale -ne "zh-CN" -or -not (Test-Path -LiteralPath $storePath)) -and (Get-Date) -lt $deadline)
    $storeAfterZh = Get-StoreAudit
    if ($zhState.locale -ne "zh-CN" -or $zhState.cleanLabel -ne "清理" -or $zhState.languageButtons -notcontains "语言") { throw "canonical Chinese shell did not commit" }
    if (-not $storeAfterZh.exists -or $storeAfterZh.base64 -ne $storeAfterZh.expected_base64 -or $storeAfterZh.lock_temp_count -ne 0) { throw "exact V1 store audit failed" }
    if ($zhState.unhandled.Count -ne 0 -or $zhState.windowErrors.Count -ne 0) { throw "first window observed unhandled/error" }
    $firstCapture = Save-WindowCapture -WindowHandle ([IntPtr]$first.Hwnd) -Path (Join-Path $EvidenceRoot "screenshots/release-first-zh-CN.png")
    $firstNotifications = @($script:CdpNotifications)
    $firstClose = Close-ReleaseRun -Run $first
    $run.first = [ordered]@{ launch = $first | Select-Object Name,Port,Pid,Hwnd,StartTime,Hash,Target,Listener; initial = $initial; settings_en = $settingsEn; chinese = $zhState; store = $storeAfterZh; capture = $firstCapture; close = $firstClose }
    if (-not $firstClose.close_main_window -or $firstClose.main_alive -or $firstClose.main_residue.Count -ne 0 -or
        $firstClose.descendant_residue.Count -ne 0 -or $firstClose.owned_webview_residue.Count -ne 0 -or
        $firstClose.listener_residue.Count -ne 0 -or $firstClose.lock_temp_residue.Count -ne 0 -or
        $firstClose.vite_residue.Count -ne 0) { throw "first release close left residue" }

    $restart = Start-ReleaseRun -Port $RestartPort -Name "restart"
    Start-Sleep -Milliseconds 750
    $restartInitial = Get-ReleaseState -Socket $restart.Socket
    if ($restartInitial.locale -ne "zh-CN" -or $restartInitial.hash -ne "#/clean" -or $restartInitial.languageButtons -notcontains "语言") { throw "restart did not consume persisted Chinese" }
    $openSettingsZh = Invoke-CdpEvaluation -Socket $restart.Socket -Expression @'
(() => { const button = Array.from(document.querySelectorAll('button')).find(item => item.textContent?.trim() === '语言'); if (!button) return false; button.focus(); button.click(); return true; })()
'@
    if (-not $openSettingsZh) { throw "Chinese Language button was unavailable" }
    Start-Sleep -Milliseconds 500
    $restartSettings = Get-ReleaseState -Socket $restart.Socket
    if ($restartSettings.hash -ne "#/settings" -or -not $restartSettings.selectPresent) { throw "restart Settings did not open" }
    $backSent = Invoke-CdpEvaluation -Socket $restart.Socket -Expression "(() => { history.back(); return true; })()"
    if (-not $backSent) { throw "history Back was not sent" }
    $deadline = (Get-Date).AddSeconds(10)
    do { Start-Sleep -Milliseconds 250; $afterBack = Get-ReleaseState -Socket $restart.Socket } while (($afterBack.hash -ne "#/clean" -or $afterBack.activeText -ne "语言") -and (Get-Date) -lt $deadline)
    if ($afterBack.hash -ne "#/clean" -or $afterBack.activeTag -ne "BUTTON" -or $afterBack.activeText -ne "语言") { throw "Back did not restore Chinese Language opener" }
    if ($afterBack.unhandled.Count -ne 0 -or $afterBack.windowErrors.Count -ne 0) { throw "restart window observed unhandled/error" }
    $restartCapture = Save-WindowCapture -WindowHandle ([IntPtr]$restart.Hwnd) -Path (Join-Path $EvidenceRoot "screenshots/release-restart-after-back.png")
    $taskbar = Get-TaskbarBinding -ExpectedPid $restart.Pid -ExpectedHwnd ([IntPtr]$restart.Hwnd)
    if ($taskbar.state -ne "PASS") { throw "taskbar direct binding failed: $($taskbar.error)" }
    $storeAfterBack = Get-StoreAudit
    if ($storeAfterBack.base64 -ne $storeAfterZh.base64 -or $storeAfterBack.lock_temp_count -ne 0) { throw "store changed across restart/Back" }
    $restartNotifications = @($script:CdpNotifications)
    $restartClose = Close-ReleaseRun -Run $restart
    if (-not $restartClose.close_main_window -or $restartClose.main_alive -or $restartClose.main_residue.Count -ne 0 -or
        $restartClose.descendant_residue.Count -ne 0 -or $restartClose.owned_webview_residue.Count -ne 0 -or
        $restartClose.listener_residue.Count -ne 0 -or $restartClose.lock_temp_residue.Count -ne 0 -or
        $restartClose.vite_residue.Count -ne 0) { throw "restart release close left residue" }

    $runtimeExceptions = @($firstNotifications + $restartNotifications | Where-Object method -eq "Runtime.exceptionThrown")
    $consoleErrors = @($firstNotifications + $restartNotifications | Where-Object { $_.method -eq "Runtime.consoleAPICalled" -and $_.params.type -eq "error" })
    $logErrors = @($firstNotifications + $restartNotifications | Where-Object { $_.method -eq "Log.entryAdded" -and $_.params.entry.level -eq "error" })
    if ($runtimeExceptions.Count -ne 0 -or $consoleErrors.Count -ne 0 -or $logErrors.Count -ne 0) { throw "release CDP error audit failed" }
    $run.restart = [ordered]@{ launch = $restart | Select-Object Name,Port,Pid,Hwnd,StartTime,Hash,Target,Listener; initial = $restartInitial; settings = $restartSettings; after_back = $afterBack; store = $storeAfterBack; capture = $restartCapture; taskbar = $taskbar; close = $restartClose }
    $run.error_audit = [ordered]@{ runtime_exceptions = $runtimeExceptions; console_errors = $consoleErrors; log_errors = $logErrors }
    $run.state = "pass"
}
catch {
    $run.state = "failed"
    $run.failure = [ordered]@{ message = $_.Exception.Message; position = $_.InvocationInfo.PositionMessage; stack = $_.ScriptStackTrace }
    foreach ($openRun in @($first, $restart)) {
        if ($null -ne $openRun -and -not $openRun.Process.HasExited) {
            try {
                if (-not $script:ClosedPids.Contains([int]$openRun.Pid)) {
                    $run["emergency_close_$($openRun.Name)"] = Close-ReleaseRun -Run $openRun
                }
                $openRun.Process.Refresh()
                if (-not $openRun.Process.HasExited) {
                    $live = Get-Process -Id $openRun.Pid -ErrorAction Stop
                    $livePath = $live.Path
                    $liveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $livePath).Hash
                    if ($livePath -ne $releaseExe -or $liveHash -ne $expectedHash) {
                        throw "refused residual termination because release identity changed"
                    }
                    Stop-Process -Id $openRun.Pid -ErrorAction Stop
                    $openRun.Process.WaitForExit(5000) | Out-Null
                    $run["identity_gated_termination_$($openRun.Name)"] = [ordered]@{
                        command = "Stop-Process -Id $($openRun.Pid)"
                        path = $livePath
                        hash = $liveHash
                        alive_after = -not $openRun.Process.HasExited
                    }
                }
            }
            catch { $run["emergency_close_error_$($openRun.Name)"] = $_.Exception.Message }
        }
    }
}
finally {
    Start-Sleep -Milliseconds 1000
    $run.final = [ordered]@{
        release_hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
        main_processes = @(Get-CimInstance Win32_Process | Where-Object Name -eq "devsweep-desktop.exe" | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath,CreationDate,CommandLine)
        webview_processes = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" } | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath,CreationDate,CommandLine)
        first_port_listeners = @(Get-NetTCPConnection -LocalPort $FirstPort -State Listen -ErrorAction SilentlyContinue)
        restart_port_listeners = @(Get-NetTCPConnection -LocalPort $RestartPort -State Listen -ErrorAction SilentlyContinue)
        vite_processes = @(Get-CimInstance Win32_Process | Where-Object {
            $_.Name -in @("node.exe", "cmd.exe") -and $_.CommandLine -match "vite" -and $_.CommandLine -match "devsweep|4180"
        } | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath,CreationDate,CommandLine)
        store = Get-StoreAudit
    }
    $run.finished_at = (Get-Date).ToString("o")
    $run | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath (Join-Path $EvidenceRoot "verification.json") -Encoding utf8
}

if ($run.state -ne "pass") { exit 1 }
if ($run.final.release_hash -ne $expectedHash -or $run.final.main_processes.Count -ne 0 -or $run.final.webview_processes.Count -ne 0) { exit 2 }
if ($run.final.first_port_listeners.Count -ne 0 -or $run.final.restart_port_listeners.Count -ne 0 -or $run.final.vite_processes.Count -ne 0) { exit 3 }
if ($run.final.store.base64 -ne [Convert]::ToBase64String($expectedBytes) -or $run.final.store.lock_temp_count -ne 0) { exit 4 }
exit 0
