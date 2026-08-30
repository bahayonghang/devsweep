param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot,
    [int]$FirstPort = 9401,
    [int]$RestartPort = 9402,
    [string]$ExpectedHash = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20",
    [Parameter(Mandatory = $true)]
    [string]$PwshPath,
    [Parameter(Mandatory = $true)]
    [string]$WrapperPath
)

$ErrorActionPreference = "Stop"
$expectedHash = $ExpectedHash
$expectedDebugHash = "F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5"
$expectedNsisHash = "0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328"
$expectedBytes = [System.Text.Encoding]::UTF8.GetBytes('{"schema_version":1,"language":"zh-CN"}')
$releaseExe = (Resolve-Path (Join-Path $RepositoryRoot "target/release/devsweep-desktop.exe")).Path
$localAppData = Join-Path $EvidenceRoot "localappdata"
$settingsRoot = Join-Path $localAppData "DevSweep/settings"
$storePath = Join-Path $settingsRoot "presentation-v1.json"
$script:CdpRequestId = 0
$script:CdpNotifications = [System.Collections.Generic.List[object]]::new()
$script:ClosedPids = [System.Collections.Generic.HashSet[int]]::new()
$script:CdpRunName = "prelaunch"
$script:CdpLogPath = Join-Path $EvidenceRoot "cdp-messages.jsonl"
$script:CdpLogEncoding = [System.Text.UTF8Encoding]::new($false)
$script:PendingNativeRun = $null
$script:TranscriptStarted = $false
$transcriptPath = Join-Path $EvidenceRoot "02-native-release-rebind.log"

function Write-EvidenceCheckpoint {
    param([string]$Message)
    Write-Host ("[{0}] {1}" -f (Get-Date).ToString("o"), $Message)
}

function Write-CdpRecord {
    param(
        [string]$Direction,
        [object]$Message
    )
    $record = [ordered]@{
        timestamp = (Get-Date).ToString("o")
        run = $script:CdpRunName
        direction = $Direction
        id = $Message.id
        method = $Message.method
        params = $Message.params
        result = $Message.result
        error = $Message.error
    }
    $line = ($record | ConvertTo-Json -Depth 100 -Compress) + [Environment]::NewLine
    [System.IO.File]::AppendAllText($script:CdpLogPath, $line, $script:CdpLogEncoding)
}

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
                Write-CdpRecord -Direction "event" -Message ([pscustomobject]@{
                    id = $null
                    method = "WebSocket.close"
                    params = @{
                        closeStatus = [string]$Socket.CloseStatus
                        description = [string]$Socket.CloseStatusDescription
                    }
                    result = $null
                    error = $null
                })
                throw "CDP websocket closed before the expected response"
            }
            $memory.Write($buffer, 0, $received.Count)
        } while (-not $received.EndOfMessage)
        $message = [System.Text.Encoding]::UTF8.GetString($memory.ToArray()) | ConvertFrom-Json -Depth 100
        Write-CdpRecord -Direction "receive" -Message $message
        return $message
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
    Write-CdpRecord -Direction "send" -Message ([pscustomobject]@{
        id = $requestId
        method = $Method
        params = $Params
        result = $null
        error = $null
    })
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
    $segment = [System.ArraySegment[byte]]::new($bytes)
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try {
        $Socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult() | Out-Null
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

function Send-CdpAndWaitSocketClose {
    param(
        [System.Net.WebSockets.ClientWebSocket]$Socket,
        [string]$Method,
        [hashtable]$Params = @{},
        [int]$TimeoutSeconds = 5
    )
    $started = Get-Date
    if ($Socket.State -ne [System.Net.WebSockets.WebSocketState]::Open) {
        return [ordered]@{
            sent = $false
            method = $Method
            reason = "socket_not_open"
            state = [string]$Socket.State
            elapsed_ms = [Math]::Round(((Get-Date) - $started).TotalMilliseconds)
            socket_closed = $true
        }
    }
    $script:CdpRequestId += 1
    $requestId = $script:CdpRequestId
    $request = [ordered]@{ id = $requestId; method = $Method; params = $Params } |
        ConvertTo-Json -Depth 30 -Compress
    Write-CdpRecord -Direction "send" -Message ([pscustomobject]@{
        id = $requestId
        method = $Method
        params = $Params
        result = $null
        error = $null
    })
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
    $segment = [System.ArraySegment[byte]]::new($bytes)
    $sendCts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(5))
    try {
        $Socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $sendCts.Token).GetAwaiter().GetResult() | Out-Null
    }
    finally { $sendCts.Dispose() }

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $closeSeen = $false
    $jsonResult = $null
    while ((Get-Date) -lt $deadline) {
        if ($Socket.State -ne [System.Net.WebSockets.WebSocketState]::Open) {
            $closeSeen = $true
            break
        }
        $buffer = [byte[]]::new(65536)
        $recvSegment = [System.ArraySegment[byte]]::new($buffer)
        $recvCts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromMilliseconds(400))
        try {
            $received = $Socket.ReceiveAsync($recvSegment, $recvCts.Token).GetAwaiter().GetResult()
            if ($received.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
                Write-CdpRecord -Direction "event" -Message ([pscustomobject]@{
                    id = $null
                    method = "WebSocket.close"
                    params = @{
                        closeStatus = [string]$Socket.CloseStatus
                        description = [string]$Socket.CloseStatusDescription
                    }
                    result = $null
                    error = $null
                })
                $closeSeen = $true
                break
            }
            $text = [System.Text.Encoding]::UTF8.GetString($buffer, 0, $received.Count)
            if ($received.EndOfMessage -and $text.Length -gt 0) {
                try {
                    $message = $text | ConvertFrom-Json -Depth 100
                    Write-CdpRecord -Direction "receive" -Message $message
                    if ($null -ne $message.id -and [int]$message.id -eq $requestId) {
                        $jsonResult = $message
                    }
                    elseif ($null -eq $message.id) {
                        $script:CdpNotifications.Add($message)
                    }
                }
                catch {
                    Write-CdpRecord -Direction "receive" -Message ([pscustomobject]@{
                        id = $null
                        method = "unparsed"
                        params = @{ raw = $text }
                        result = $null
                        error = $null
                    })
                }
            }
        }
        catch [System.OperationCanceledException] { }
        catch {
            if ($Socket.State -ne [System.Net.WebSockets.WebSocketState]::Open) {
                $closeSeen = $true
                break
            }
        }
        finally { $recvCts.Dispose() }
    }
    $closed = $closeSeen -or ($Socket.State -ne [System.Net.WebSockets.WebSocketState]::Open)
    return [ordered]@{
        sent = $true
        method = $Method
        request_id = $requestId
        json_result = $jsonResult
        socket_closed = $closed
        state = [string]$Socket.State
        elapsed_ms = [Math]::Round(((Get-Date) - $started).TotalMilliseconds)
    }
}

function Get-OwnedWebViews {
    param([int]$RootPid)
    $started = Get-Date
    $cheap = @(Get-Process -Name "msedgewebview2" -ErrorAction SilentlyContinue)
    $cheapMs = [Math]::Round(((Get-Date) - $started).TotalMilliseconds, 1)
    if ($cheap.Count -eq 0) {
        return [ordered]@{
            query_ms = $cheapMs
            cheap_ms = $cheapMs
            identity_ms = 0
            candidate_count = 0
            owned = @()
        }
    }
    $identityStart = Get-Date
    $candidates = @(Get-CimInstance -ClassName Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue)
    $owned = @($candidates | Where-Object {
        $cmd = [string]$_.CommandLine
        ([int]$_.ParentProcessId -eq $RootPid) -or
        ($cmd -match "webview-exe-name=devsweep-desktop\.exe")
    } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
    $identityMs = [Math]::Round(((Get-Date) - $identityStart).TotalMilliseconds, 1)
    return [ordered]@{
        query_ms = [Math]::Round(((Get-Date) - $started).TotalMilliseconds, 1)
        cheap_ms = $cheapMs
        identity_ms = $identityMs
        candidate_count = $candidates.Count
        owned = $owned
    }
}

function Get-PortListeners {
    param([int]$Port)
    $props = [System.Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties()
    @($props.GetActiveTcpListeners() | Where-Object { $_.Port -eq $Port } | ForEach-Object {
        [pscustomobject]@{
            LocalAddress = $_.Address.ToString()
            LocalPort = $_.Port
            AddressFamily = [string]$_.AddressFamily
        }
    })
}

function Get-FrozenArtifactGate {
    $debugPath = (Resolve-Path (Join-Path $RepositoryRoot "target/debug/devsweep-desktop.exe")).Path
    $nsisPath = (Resolve-Path (Join-Path $RepositoryRoot "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe")).Path
    return @(
        [pscustomobject]@{
            name = "release"
            path = $releaseExe
            bytes = (Get-Item -LiteralPath $releaseExe).Length
            hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
            expected_bytes = 9827328
            expected_hash = $expectedHash
        },
        [pscustomobject]@{
            name = "debug"
            path = $debugPath
            bytes = (Get-Item -LiteralPath $debugPath).Length
            hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $debugPath).Hash
            expected_bytes = 14758912
            expected_hash = $expectedDebugHash
        },
        [pscustomobject]@{
            name = "nsis"
            path = $nsisPath
            bytes = (Get-Item -LiteralPath $nsisPath).Length
            hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $nsisPath).Hash
            expected_bytes = 3152372
            expected_hash = $expectedNsisHash
        }
    )
}

function Assert-FrozenArtifactGate {
    param($Gate, [string]$When)
    foreach ($artifact in $Gate) {
        if ($artifact.bytes -ne $artifact.expected_bytes -or $artifact.hash -ne $artifact.expected_hash) {
            throw ("frozen hash drift at {0}: {1} hash={2} bytes={3}" -f $When, $artifact.name, $artifact.hash, $artifact.bytes)
        }
        Write-EvidenceCheckpoint ("ARTIFACT_GATE when={0} name={1} bytes={2} sha256={3}" -f $When, $artifact.name, $artifact.bytes, $artifact.hash)
    }
}

function Save-WindowCapture {
    param([IntPtr]$WindowHandle, [string]$Path)
    Add-Type -AssemblyName System.Drawing
    if (-not ("RecaptureCdpNative" -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RecaptureCdpNative {
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
    $rect = New-Object RecaptureCdpNative+RECT
    if (-not [RecaptureCdpNative]::GetWindowRect($WindowHandle, [ref]$rect)) { throw "GetWindowRect failed" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    try { $captured = [RecaptureCdpNative]::PrintWindow($WindowHandle, $hdc, 2) }
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
    $script:CdpRunName = $Name
    Write-EvidenceCheckpoint "START_RELEASE name=$Name port=$Port"
    if (@(Get-PortListeners -Port $Port).Count -ne 0) {
        throw "CDP port $Port was occupied before $Name"
    }
    Assert-FrozenArtifactGate -Gate (Get-FrozenArtifactGate) -When ("before_$Name")
    $oldLocal = $env:LOCALAPPDATA
    $oldBrowserArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    try {
        $env:LOCALAPPDATA = $localAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
        $process = Start-Process -FilePath $releaseExe -PassThru
        $script:PendingNativeRun = [pscustomobject]@{ Name = $Name; Port = $Port; Process = $process }
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        if ($null -eq $oldBrowserArgs) { Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue }
        else { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArgs }
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
    $listeners = @(Get-PortListeners -Port $Port)
    $loopbackListeners = @($listeners | Where-Object { $_.LocalAddress -in @("127.0.0.1", "::1") })
    $nonLoopbackListeners = @($listeners | Where-Object { $_.LocalAddress -notin @("127.0.0.1", "::1") })
    if ($loopbackListeners.Count -lt 1 -or $nonLoopbackListeners.Count -ne 0) { throw "$Name CDP listener was not unique loopback" }
    $socket = [System.Net.WebSockets.ClientWebSocket]::new()
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try { $socket.ConnectAsync([Uri]$matching[0].webSocketDebuggerUrl, $cts.Token).GetAwaiter().GetResult() | Out-Null }
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
    Write-EvidenceCheckpoint ("CDP_READY name={0} pid={1} hwnd=0x{2:X} target={3}" -f $Name, $process.Id, $process.MainWindowHandle, $matching[0].id)
    $completedRun = [pscustomobject]@{
        Name = $Name
        Port = $Port
        Process = $process
        Socket = $socket
        Target = $matching[0]
        Listener = $listeners[0]
        Pid = $process.Id
        Hwnd = $process.MainWindowHandle
        StartTime = $process.StartTime.ToString("o")
        Path = $process.Path
        Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    }
    $script:PendingNativeRun = $null
    return $completedRun
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

function Get-StoreAudit {
    if (-not (Test-Path -LiteralPath $storePath)) { return [ordered]@{ exists = $false; lock_temp_count = 0 } }
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

function Get-LockTempResidue {
    @(Get-ChildItem -LiteralPath $settingsRoot -File -Force -ErrorAction SilentlyContinue |
        Where-Object Name -ne "presentation-v1.json" | Select-Object Name, Length, FullName)
}

function Invoke-IdentityGatedStop {
    param([pscustomobject]$Run)
    $actions = [System.Collections.Generic.List[object]]::new()
    $Run.Process.Refresh()
    if (-not $Run.Process.HasExited) {
        $fresh = Get-Process -Id $Run.Pid -ErrorAction SilentlyContinue
        if ($null -ne $fresh) {
            $freshPath = $fresh.Path
            $freshStart = $fresh.StartTime.ToString("o")
            $freshHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $freshPath).Hash
            $matched = ($freshPath -eq $releaseExe) -and ($freshStart -eq $Run.StartTime) -and ($freshHash -eq $expectedHash) -and ($fresh.ProcessName -eq "devsweep-desktop")
            $command = "Stop-Process -Id $($Run.Pid) -Force"
            $result = "skipped_identity_mismatch"
            if ($matched) {
                Stop-Process -Id $Run.Pid -Force
                Start-Sleep -Milliseconds 200
                $still = Get-Process -Id $Run.Pid -ErrorAction SilentlyContinue
                $result = if ($null -eq $still) { "exited" } else { "still_alive" }
            }
            $actions.Add([ordered]@{
                target = "main"
                pid = $Run.Pid
                command = $command
                matched = $matched
                path = $freshPath
                start_time = $freshStart
                hash = $freshHash
                ownership = "task-owned-release-main"
                result = $result
            }) | Out-Null
        }
    }
    $ownedQuery = Get-OwnedWebViews -RootPid $Run.Pid
    foreach ($row in @($ownedQuery.owned)) {
        $cmdLine = [string]$row.CommandLine
        $okName = $row.Name -eq "msedgewebview2.exe"
        $okOwner = ([int]$row.ParentProcessId -eq $Run.Pid) -or ($cmdLine -match "webview-exe-name=devsweep-desktop\.exe")
        $matched = $okName -and $okOwner
        $command = "Stop-Process -Id $($row.ProcessId) -Force"
        $result = "skipped_identity_mismatch"
        if ($matched) {
            Stop-Process -Id $row.ProcessId -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 50
            $still = Get-Process -Id $row.ProcessId -ErrorAction SilentlyContinue
            $result = if ($null -eq $still) { "exited" } else { "still_alive" }
        }
        $actions.Add([ordered]@{
            target = "owned_webview"
            pid = $row.ProcessId
            parent = $row.ParentProcessId
            command = $command
            matched = $matched
            path = $row.ExecutablePath
            start_time = [string]$row.CreationDate
            ownership = "webview-exe-name=devsweep-desktop.exe"
            result = $result
        }) | Out-Null
    }
    return @($actions)
}

function Close-ReleaseRun {
    param([pscustomobject]$Run)
    Write-EvidenceCheckpoint ("CLOSE_BEGIN name={0} pid={1} target={2} socket={3}" -f $Run.Name, $Run.Pid, $Run.Target.id, [string]$Run.Socket.State)
    $debugger = [ordered]@{
        target_id = [string]$Run.Target.id
        methods = @()
        socket_state_before = [string]$Run.Socket.State
        browser_close_used = $false
        page_close_used = $false
        close_order = "CloseMainWindow-WaitForExit-Target.detachFromTarget-dispose"
    }

    $closeRequestedAt = Get-Date
    $closeResult = $Run.Process.CloseMainWindow()
    $script:ClosedPids.Add([int]$Run.Pid) | Out-Null
    Write-EvidenceCheckpoint ("CLOSE_MAIN_WINDOW name={0} pid={1} result={2} socket={3}" -f $Run.Name, $Run.Pid, $closeResult, [string]$Run.Socket.State)
    $waited = $Run.Process.WaitForExit(5000)
    $mainElapsedMs = [Math]::Round(((Get-Date) - $closeRequestedAt).TotalMilliseconds)
    Write-EvidenceCheckpoint ("WAIT_FOR_EXIT name={0} waited={1} elapsed_ms={2} has_exited={3}" -f $Run.Name, $waited, $mainElapsedMs, $Run.Process.HasExited)

    if ($Run.Socket.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
        try {
            $detach = Send-CdpAndWaitSocketClose -Socket $Run.Socket -Method "Target.detachFromTarget" -Params @{
                targetId = [string]$Run.Target.id
            } -TimeoutSeconds 2
            $cdpError = $null
            if ($null -ne $detach.json_result -and $null -ne $detach.json_result.error) {
                $cdpError = $detach.json_result.error
            }
            $detach.cdp_error = $cdpError
            $debugger.methods += $detach
            Write-EvidenceCheckpoint ("CDP_Target.detachFromTarget sent={0} socket_closed={1} state={2} elapsed_ms={3} error={4}" -f $detach.sent, $detach.socket_closed, $detach.state, $detach.elapsed_ms, $(if ($null -eq $cdpError) { "none" } else { ($cdpError | ConvertTo-Json -Compress) }))
        }
        catch {
            $debugger.methods += [ordered]@{
                sent = $false
                method = "Target.detachFromTarget"
                reason = "exception"
                error = $_.Exception.Message
                state = [string]$Run.Socket.State
            }
            Write-EvidenceCheckpoint ("CDP_Target.detachFromTarget_ERROR {0}" -f $_.Exception.Message)
        }
    }
    else {
        $debugger.methods += [ordered]@{
            sent = $false
            method = "Target.detachFromTarget"
            reason = "socket_not_open"
            state = [string]$Run.Socket.State
            socket_closed = $true
        }
    }
    $debugger.socket_state_before_dispose = [string]$Run.Socket.State
    try { $Run.Socket.Dispose() } catch { $debugger.dispose_error = $_.Exception.Message }
    $debugger.socket_state_after_dispose = [string]$Run.Socket.State
    Write-EvidenceCheckpoint ("DEBUGGER_DISPOSED name={0} state={1}" -f $Run.Name, $debugger.socket_state_after_dispose)

    $samples = @()
    $residueDeadline = $closeRequestedAt.AddSeconds(5)
    $mainAlive = @(Get-Process -Id $Run.Pid -ErrorAction SilentlyContinue)
    $ownedQuery = Get-OwnedWebViews -RootPid $Run.Pid
    $listeners = @(Get-PortListeners -Port $Run.Port)
    $lockTemp = @(Get-LockTempResidue)
    $sampleIndex = 0
    do {
        $sampleIndex += 1
        $now = Get-Date
        $sample = [ordered]@{
            i = $sampleIndex
            time = $now.ToString("o")
            ms_after_close = [Math]::Round(($now - $closeRequestedAt).TotalMilliseconds)
            query_ms = $ownedQuery.query_ms
            cheap_ms = $ownedQuery.cheap_ms
            identity_ms = $ownedQuery.identity_ms
            candidate_webview_count = $ownedQuery.candidate_count
            main_count = $mainAlive.Count
            owned_webview_count = @($ownedQuery.owned).Count
            listener_count = $listeners.Count
            lock_temp_count = $lockTemp.Count
            owned = @($ownedQuery.owned | ForEach-Object {
                [ordered]@{
                    pid = $_.ProcessId
                    parent = $_.ParentProcessId
                    command = $_.CommandLine
                }
            })
        }
        $samples += $sample
        Write-EvidenceCheckpoint ("RESIDUE i={0} after_ms={1} main={2} owned={3} listener={4} lock_temp={5} query_ms={6}" -f $sample.i, $sample.ms_after_close, $sample.main_count, $sample.owned_webview_count, $sample.listener_count, $sample.lock_temp_count, $sample.query_ms)
        $clear = ($sample.main_count -eq 0 -and $sample.owned_webview_count -eq 0 -and $sample.listener_count -eq 0 -and $sample.lock_temp_count -eq 0)
        if ($clear -or ((Get-Date) -ge $residueDeadline)) { break }
        $spent = ((Get-Date) - $now).TotalMilliseconds
        if ($spent -lt 200) { Start-Sleep -Milliseconds ([Math]::Max(1, 200 - $spent)) }
        $mainAlive = @(Get-Process -Id $Run.Pid -ErrorAction SilentlyContinue)
        $ownedQuery = Get-OwnedWebViews -RootPid $Run.Pid
        $listeners = @(Get-PortListeners -Port $Run.Port)
        $lockTemp = @(Get-LockTempResidue)
    } while ((Get-Date) -lt $residueDeadline)

    $Run.Process.Refresh()
    $finalMain = @(Get-Process -Id $Run.Pid -ErrorAction SilentlyContinue)
    $finalOwned = Get-OwnedWebViews -RootPid $Run.Pid
    $finalListeners = @(Get-PortListeners -Port $Run.Port)
    $finalLockTemp = @(Get-LockTempResidue)
    $gracefulClear = ($finalMain.Count -eq 0 -and @($finalOwned.owned).Count -eq 0 -and $finalListeners.Count -eq 0 -and $finalLockTemp.Count -eq 0)
    $identityStops = @()
    if (-not $gracefulClear) {
        Write-EvidenceCheckpoint ("GRACEFUL_CLOSE_FAILED name={0} main={1} owned={2} listener={3} lock_temp={4}" -f $Run.Name, $finalMain.Count, @($finalOwned.owned).Count, $finalListeners.Count, $finalLockTemp.Count)
        $identityStops = @(Invoke-IdentityGatedStop -Run $Run)
        Write-EvidenceCheckpoint ("IDENTITY_GATED_STOP count={0}" -f $identityStops.Count)
    }

    $closeAudit = [ordered]@{
        debugger = $debugger
        close_main_window = $closeResult
        wait_for_exit = $waited
        close_requested_at = $closeRequestedAt.ToString("o")
        elapsed_ms = $mainElapsedMs
        main_alive = -not $Run.Process.HasExited
        residue_samples = $samples
        main_residue = @($finalMain | ForEach-Object { [ordered]@{ pid = $_.Id; path = $_.Path; start_time = $_.StartTime.ToString("o") } })
        owned_webview_residue = @($finalOwned.owned | ForEach-Object {
            [ordered]@{ pid = $_.ProcessId; parent = $_.ParentProcessId; command = $_.CommandLine }
        })
        listener_residue = @($finalListeners | Select-Object LocalAddress, LocalPort)
        lock_temp_residue = $finalLockTemp
        final_query_ms = $finalOwned.query_ms
        graceful_clear = $gracefulClear
        identity_gated_termination = $identityStops
    }
    Write-EvidenceCheckpoint ("CLOSE_RESULT name={0} elapsed_ms={1} main={2} webview={3} listener={4} lock_temp={5} graceful_clear={6}" -f $Run.Name, $mainElapsedMs, $finalMain.Count, @($finalOwned.owned).Count, $finalListeners.Count, $finalLockTemp.Count, $gracefulClear)
    return $closeAudit
}

function Get-TaskbarBinding {
    param([int]$ExpectedPid, [IntPtr]$ExpectedHwnd)
    $result = [ordered]@{ state = "UNVERIFIED"; candidates = @(); expected_pid = $ExpectedPid; expected_hwnd = "0x{0:X}" -f $ExpectedHwnd }
    try {
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        if (-not ("RecaptureCdpNative" -as [type])) { Save-WindowCapture -WindowHandle $ExpectedHwnd -Path (Join-Path $EvidenceRoot "screenshots/_native-type-init.png") | Out-Null }
        $taskbarHwnd = [RecaptureCdpNative]::FindWindow("Shell_TrayWnd", $null)
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
        [ordered]@{
            captured_at = (Get-Date).ToString("o")
            shell_tray_wnd = $result.shell_tray_wnd
            expected_pid = $ExpectedPid
            expected_hwnd = $result.expected_hwnd
            exact_match_count = $matchingElements.Count
            descendants = $rows
        } | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath (Join-Path $EvidenceRoot "shell-tray-wnd-uia-raw.json") -Encoding utf8
        if ($matchingElements.Count -ne 1) { throw "expected exactly one DevSweep taskbar Button, found $($matchingElements.Count)" }
        $button = $matchingElements[0]

        $result.minimize_requested_at = (Get-Date).ToString("o")
        [RecaptureCdpNative]::ShowWindow($ExpectedHwnd, 6) | Out-Null
        $result.minimize_samples = @()
        $deadline = (Get-Date).AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $foreground = [RecaptureCdpNative]::GetForegroundWindow()
            $iconic = [RecaptureCdpNative]::IsIconic($ExpectedHwnd)
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
        $cursorBefore = New-Object RecaptureCdpNative+POINT
        [RecaptureCdpNative]::GetCursorPos([ref]$cursorBefore) | Out-Null
        [uint32]$beforePid = 0
        [RecaptureCdpNative]::GetWindowThreadProcessId($foreground, [ref]$beforePid) | Out-Null
        $result.before_click = [ordered]@{
            time = (Get-Date).ToString("o")
            cursor_x = $cursorBefore.X
            cursor_y = $cursorBefore.Y
            foreground_hwnd = "0x{0:X}" -f $foreground
            foreground_pid = $beforePid
            is_iconic = [RecaptureCdpNative]::IsIconic($ExpectedHwnd)
        }
        $result.click_point = [ordered]@{ x = $point.X; y = $point.Y }
        $result.clicked_at = (Get-Date).ToString("o")
        if (-not [RecaptureCdpNative]::ClickAt([Math]::Round($point.X), [Math]::Round($point.Y))) { throw "SetCursorPos + SendInput failed" }
        $result.activation = "SetCursorPos+SendInput"
        $result.activation_samples = @()
        $deadline = (Get-Date).AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $foreground = [RecaptureCdpNative]::GetForegroundWindow()
            $iconic = [RecaptureCdpNative]::IsIconic($ExpectedHwnd)
            [uint32]$samplePid = 0
            [RecaptureCdpNative]::GetWindowThreadProcessId($foreground, [ref]$samplePid) | Out-Null
            $result.activation_samples += [pscustomobject]@{
                time = (Get-Date).ToString("o")
                foreground_hwnd = "0x{0:X}" -f $foreground
                foreground_pid = $samplePid
                is_iconic = $iconic
            }
        } while (($foreground -ne $ExpectedHwnd -or $iconic) -and (Get-Date) -lt $deadline)
        if ($foreground -ne $ExpectedHwnd -or $iconic) { throw "taskbar SendInput did not restore exact app HWND" }
        [uint32]$foregroundPid = 0
        [RecaptureCdpNative]::GetWindowThreadProcessId($foreground, [ref]$foregroundPid) | Out-Null
        if ($foregroundPid -ne $ExpectedPid) { throw "foreground PID mismatch" }
        $path = (Get-Process -Id $foregroundPid -ErrorAction Stop).Path
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
        if ($path -ne $releaseExe -or $hash -ne $expectedHash) { throw "foreground executable identity mismatch" }
        $result.bound = [ordered]@{
            foreground_hwnd = "0x{0:X}" -f $foreground
            foreground_pid = $foregroundPid
            foreground_path = $path
            foreground_hash = $hash
            is_iconic = [RecaptureCdpNative]::IsIconic($ExpectedHwnd)
        }
        $timestamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
        $result.capture_id = $timestamp
        $result.capture_started = (Get-Date).ToString("o")
        $result.desktop_capture = Save-DesktopCapture -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-desktop-$timestamp.png")
        $result.taskbar_capture = Save-WindowCapture -WindowHandle $taskbarHwnd -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-crop-$timestamp.png")
        $result.app_reference_capture = Save-WindowCapture -WindowHandle $ExpectedHwnd -Path (Join-Path $EvidenceRoot "screenshots/release-taskbar-bound-app-reference-$timestamp.png")
        $result.capture_finished = (Get-Date).ToString("o")
        $result.state = "PASS"
        Write-EvidenceCheckpoint ("TASKBAR_PASS pid={0} hwnd={1} capture_id={2}" -f $ExpectedPid, $result.expected_hwnd, $timestamp)
    }
    catch { $result.error = $_.Exception.Message }
    return $result
}

New-Item -ItemType Directory -Force -Path $EvidenceRoot, $localAppData, (Join-Path $EvidenceRoot "screenshots") | Out-Null
if (Test-Path -LiteralPath $transcriptPath) { throw "native-close transcript already exists; refusing to overwrite" }
Start-Transcript -LiteralPath $transcriptPath | Out-Null
$script:TranscriptStarted = $true
Write-EvidenceCheckpoint "PRELAUNCH transcript_started=true devsweep_started=false"
if (Test-Path -LiteralPath $script:CdpLogPath) { throw "CDP log already exists; refusing to overwrite prior evidence" }
[System.IO.File]::WriteAllText($script:CdpLogPath, "", $script:CdpLogEncoding)
Write-EvidenceCheckpoint ("HELPER_START release={0} expected_hash={1} localappdata={2} cdp_log={3}" -f $releaseExe, $expectedHash, $localAppData, $script:CdpLogPath)
$run = [ordered]@{ started_at = (Get-Date).ToString("o"); release = $releaseExe; expected_hash = $expectedHash; localappdata = $localAppData; state = "started" }
$first = $null
$restart = $null
try {
    $resolvedPwsh = (Resolve-Path -LiteralPath $PwshPath).Path
    $actualPwsh = (Get-Process -Id $PID -ErrorAction Stop).Path
    $selfCim = Get-CimInstance Win32_Process -Filter "ProcessId = $PID"
    if ($actualPwsh -ne $resolvedPwsh -or $PSVersionTable.PSVersion.Major -lt 7) {
        throw "native-close helper is not executing under the required PowerShell 7 binary"
    }
    Write-EvidenceCheckpoint ("TOOLCHAIN_PWSH path={0} version={1} pid={2} commandline={3}" -f $actualPwsh, $PSVersionTable.PSVersion.ToString(), $PID, $selfCim.CommandLine)

    $nestedFixture = '{"id":7,"method":"Runtime.evaluate","params":{"expression":"1+1","nested":{"levels":[{"value":2}]}},"result":{"result":{"type":"number","value":2}},"error":null}'
    $nestedDecoded = $nestedFixture | ConvertFrom-Json -Depth 100
    $nestedRoundTrip = $nestedDecoded | ConvertTo-Json -Depth 100 -Compress | ConvertFrom-Json -Depth 100
    if ($nestedRoundTrip.id -ne 7 -or $nestedRoundTrip.params.nested.levels[0].value -ne 2 -or $nestedRoundTrip.result.result.value -ne 2) {
        throw "nested CDP JSON round-trip gate failed"
    }
    $probeSocket = [System.Net.WebSockets.ClientWebSocket]::new()
    $probeSocket.Dispose()
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    if ($null -eq [System.Windows.Automation.AutomationElement]) { throw "UIAutomation type gate failed" }
    foreach ($scriptPath in @($PSCommandPath, $WrapperPath)) {
        $tokens = $null
        $parseErrors = $null
        [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path -LiteralPath $scriptPath).Path, [ref]$tokens, [ref]$parseErrors) | Out-Null
        if ($parseErrors.Count -ne 0) { throw "static parse failed for $scriptPath : $($parseErrors[0].ToString())" }
    }
    $hashesBefore = Get-FrozenArtifactGate
    Assert-FrozenArtifactGate -Gate $hashesBefore -When "before_launch"
    $run.hashes_before = $hashesBefore
    $preMain = @(Get-CimInstance Win32_Process -Filter "Name='devsweep-desktop.exe'")
    $preWebView = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" })
    $prePort1 = @(Get-PortListeners -Port $FirstPort)
    $prePort2 = @(Get-PortListeners -Port $RestartPort)
    if ($preMain.Count -ne 0 -or $preWebView.Count -ne 0 -or $prePort1.Count -ne 0 -or $prePort2.Count -ne 0) { throw "prelaunch process/listener gate failed" }
    $run.toolchain = [ordered]@{
        pwsh_path = $actualPwsh
        pwsh_version = $PSVersionTable.PSVersion.ToString()
        process_id = $PID
        command_line = $selfCim.CommandLine
        nested_json_roundtrip = "pass"
        client_websocket = "pass"
        start_transcript = "pass"
        ui_automation = "pass"
        static_parse = "pass"
        artifacts = $hashesBefore
        prelaunch = [ordered]@{ main = 0; webview = 0; first_port = 0; restart_port = 0 }
    }
    Write-EvidenceCheckpoint "TOOLCHAIN_GATE=PASS all_prelaunch_checks=true"
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
    if (-not $storeAfterZh.exists -or $storeAfterZh.length -ne 39 -or $storeAfterZh.base64 -ne $storeAfterZh.expected_base64 -or $storeAfterZh.lock_temp_count -ne 0) { throw "exact V1 store audit failed" }
    if ($zhState.unhandled.Count -ne 0 -or $zhState.windowErrors.Count -ne 0) { throw "first window observed unhandled/error" }
    Write-EvidenceCheckpoint ("FIRST_LOCALE_PASS locale={0} store_bytes={1} store_hash={2} lock_temp={3}" -f $zhState.locale, $storeAfterZh.length, $storeAfterZh.sha256, $storeAfterZh.lock_temp_count)
    $firstCapture = Save-WindowCapture -WindowHandle ([IntPtr]$first.Hwnd) -Path (Join-Path $EvidenceRoot "screenshots/release-first-zh-CN.png")
    $firstNotifications = @($script:CdpNotifications)
    $firstClose = Close-ReleaseRun -Run $first
    $run.first = [ordered]@{
        launch = $first | Select-Object Name, Port, Pid, Hwnd, StartTime, Path, Hash, Target, Listener
        initial = $initial
        settings_en = $settingsEn
        chinese = $zhState
        store = $storeAfterZh
        capture = $firstCapture
        close = $firstClose
    }
    if (-not $firstClose.close_main_window -or $firstClose.main_alive -or $firstClose.main_residue.Count -ne 0 -or
        $firstClose.owned_webview_residue.Count -ne 0 -or $firstClose.listener_residue.Count -ne 0 -or
        $firstClose.lock_temp_residue.Count -ne 0 -or -not $firstClose.graceful_clear) {
        throw "first release close left residue"
    }

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
    Write-EvidenceCheckpoint ("RESTART_BACK_PASS locale={0} hash={1} active={2}/{3}" -f $restartInitial.locale, $afterBack.hash, $afterBack.activeTag, $afterBack.activeText)
    $restartCapture = Save-WindowCapture -WindowHandle ([IntPtr]$restart.Hwnd) -Path (Join-Path $EvidenceRoot "screenshots/release-restart-after-back.png")
    $taskbar = Get-TaskbarBinding -ExpectedPid $restart.Pid -ExpectedHwnd ([IntPtr]$restart.Hwnd)
    if ($taskbar.state -ne "PASS") { throw "taskbar direct binding failed: $($taskbar.error)" }
    $storeAfterBack = Get-StoreAudit
    if ($storeAfterBack.base64 -ne $storeAfterZh.base64 -or $storeAfterBack.lock_temp_count -ne 0) { throw "store changed across restart/Back" }
    $restartNotifications = @($script:CdpNotifications)
    $restartClose = Close-ReleaseRun -Run $restart
    if (-not $restartClose.close_main_window -or $restartClose.main_alive -or $restartClose.main_residue.Count -ne 0 -or
        $restartClose.owned_webview_residue.Count -ne 0 -or $restartClose.listener_residue.Count -ne 0 -or
        $restartClose.lock_temp_residue.Count -ne 0 -or -not $restartClose.graceful_clear) {
        throw "restart release close left residue"
    }

    $runtimeExceptions = @($firstNotifications + $restartNotifications | Where-Object method -eq "Runtime.exceptionThrown")
    $consoleErrors = @($firstNotifications + $restartNotifications | Where-Object { $_.method -eq "Runtime.consoleAPICalled" -and $_.params.type -eq "error" })
    $logErrors = @($firstNotifications + $restartNotifications | Where-Object { $_.method -eq "Log.entryAdded" -and $_.params.entry.level -eq "error" })
    if (@($zhState.unhandled).Count -ne 0 -or @($zhState.windowErrors).Count -ne 0 -or @($afterBack.unhandled).Count -ne 0 -or @($afterBack.windowErrors).Count -ne 0) {
        throw "two-run window error arrays were not empty"
    }
    if ($runtimeExceptions.Count -ne 0 -or $consoleErrors.Count -ne 0 -or $logErrors.Count -ne 0) { throw "release CDP error audit failed" }
    $run.restart = [ordered]@{
        launch = $restart | Select-Object Name, Port, Pid, Hwnd, StartTime, Path, Hash, Target, Listener
        initial = $restartInitial
        settings = $restartSettings
        after_back = $afterBack
        store = $storeAfterBack
        capture = $restartCapture
        taskbar = $taskbar
        close = $restartClose
    }
    $run.error_audit = [ordered]@{
        first_unhandled = @($zhState.unhandled)
        first_window_errors = @($zhState.windowErrors)
        restart_unhandled = @($afterBack.unhandled)
        restart_window_errors = @($afterBack.windowErrors)
        runtime_exceptions = $runtimeExceptions
        console_errors = $consoleErrors
        log_errors = $logErrors
    }
    $run.state = "pass"
    Write-EvidenceCheckpoint ("ERROR_AUDIT_PASS runtime={0} console={1} log={2}" -f $runtimeExceptions.Count, $consoleErrors.Count, $logErrors.Count)
}
catch {
    $run.state = "failed"
    $run.failure = [ordered]@{ message = $_.Exception.Message; position = $_.InvocationInfo.PositionMessage; stack = $_.ScriptStackTrace }
    Write-EvidenceCheckpoint ("HELPER_FAILURE {0}" -f $_.Exception.Message)
    foreach ($openRun in @($first, $restart)) {
        if ($null -ne $openRun -and -not $openRun.Process.HasExited) {
            try {
                if (-not $script:ClosedPids.Contains([int]$openRun.Pid)) {
                    $run["emergency_close_$($openRun.Name)"] = Close-ReleaseRun -Run $openRun
                }
                $openRun.Process.Refresh()
                if (-not $openRun.Process.HasExited) {
                    $run["identity_gated_$($openRun.Name)"] = @(Invoke-IdentityGatedStop -Run $openRun)
                }
            }
            catch { $run["emergency_close_error_$($openRun.Name)"] = $_.Exception.Message }
        }
    }
    if ($null -ne $script:PendingNativeRun -and -not $script:PendingNativeRun.Process.HasExited) {
        try {
            $pending = $script:PendingNativeRun
            $pendingPath = $pending.Process.Path
            $pendingHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $pendingPath).Hash
            $pendingStart = $pending.Process.StartTime.ToString("o")
            $matched = ($pendingPath -eq $releaseExe -and $pendingHash -eq $expectedHash)
            $closeResult = $pending.Process.CloseMainWindow()
            $pending.Process.WaitForExit(25000) | Out-Null
            $forced = $false
            $forceResult = $null
            if (-not $pending.Process.HasExited -and $matched) {
                Stop-Process -Id $pending.Process.Id -Force
                $forced = $true
                $forceResult = "Stop-Process -Id $($pending.Process.Id) -Force"
            }
            $run.pending_launch_close = [ordered]@{
                name = $pending.Name
                pid = $pending.Process.Id
                close_main_window = $closeResult
                alive_after = -not $pending.Process.HasExited
                identity_matched = $matched
                start_time = $pendingStart
                hash = $pendingHash
                forced_termination = $forced
                force_command = $forceResult
            }
        }
        catch { $run.pending_launch_close_error = $_.Exception.Message }
    }
}
finally {
    Start-Sleep -Milliseconds 400
    $hashesAfter = $null
    $hashDrift = $null
    try {
        $hashesAfter = Get-FrozenArtifactGate
        Assert-FrozenArtifactGate -Gate $hashesAfter -When "after_run"
    }
    catch {
        $hashDrift = $_.Exception.Message
        Write-EvidenceCheckpoint ("HASH_DRIFT {0}" -f $hashDrift)
        if ($run.state -eq "pass") { $run.state = "failed" }
        if ($null -eq $run.failure) { $run.failure = [ordered]@{ message = $hashDrift } }
    }
    $run.hashes_after = $hashesAfter
    $run.hash_drift = $hashDrift
    $run.final = [ordered]@{
        release_hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
        debug_hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $RepositoryRoot "target/debug/devsweep-desktop.exe")).Hash
        nsis_hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $RepositoryRoot "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe")).Hash
        main_processes = @(Get-CimInstance Win32_Process -Filter "Name='devsweep-desktop.exe'" | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        webview_processes = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        first_port_listeners = @(Get-PortListeners -Port $FirstPort)
        restart_port_listeners = @(Get-PortListeners -Port $RestartPort)
        store = Get-StoreAudit
    }
    $run.finished_at = (Get-Date).ToString("o")
    $run | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath (Join-Path $EvidenceRoot "verification.json") -Encoding utf8
    $cdpItem = Get-Item -LiteralPath $script:CdpLogPath
    $cdpLineCount = @(Get-Content -LiteralPath $script:CdpLogPath).Count
    Write-EvidenceCheckpoint ("FINAL_AUDIT state={0} release_hash={1} debug_hash={2} nsis_hash={3} main={4} webview={5} port1={6} port2={7} store_lock_temp={8} cdp_lines={9} cdp_sha256={10}" -f $run.state, $run.final.release_hash, $run.final.debug_hash, $run.final.nsis_hash, $run.final.main_processes.Count, $run.final.webview_processes.Count, $run.final.first_port_listeners.Count, $run.final.restart_port_listeners.Count, $run.final.store.lock_temp_count, $cdpLineCount, (Get-FileHash -Algorithm SHA256 -LiteralPath $cdpItem.FullName).Hash)
}

$helperExit = 0
if ($run.state -ne "pass") { $helperExit = 1 }
elseif ($run.final.release_hash -ne $expectedHash -or $run.final.debug_hash -ne $expectedDebugHash -or $run.final.nsis_hash -ne $expectedNsisHash) { $helperExit = 5 }
elseif ($run.final.main_processes.Count -ne 0 -or $run.final.webview_processes.Count -ne 0) { $helperExit = 2 }
elseif ($run.final.first_port_listeners.Count -ne 0 -or $run.final.restart_port_listeners.Count -ne 0) { $helperExit = 3 }
elseif ($run.final.store.base64 -ne [Convert]::ToBase64String($expectedBytes) -or $run.final.store.lock_temp_count -ne 0) { $helperExit = 4 }
Write-EvidenceCheckpoint ("HELPER_EXIT_CODE={0}" -f $helperExit)
if ($script:TranscriptStarted) {
    Stop-Transcript | Out-Null
    $script:TranscriptStarted = $false
}
exit $helperExit
