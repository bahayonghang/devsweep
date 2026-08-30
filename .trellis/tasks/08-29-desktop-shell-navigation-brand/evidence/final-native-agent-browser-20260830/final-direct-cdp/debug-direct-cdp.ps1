param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot,
    [int]$CdpPort = 9341,
    [string]$ExpectedHash = "5677E0FE2DF24B6A454A2CEA378B38985EF6B0760DA12BC7D19E1384929C07D9"
)

$ErrorActionPreference = "Stop"
$expectedHash = $ExpectedHash
$debugExe = (Resolve-Path (Join-Path $RepositoryRoot "target/debug/devsweep-desktop.exe")).Path
$localAppData = (Resolve-Path (Join-Path $EvidenceRoot "localappdata")).Path
$rawCdpLog = Join-Path $EvidenceRoot "cdp-messages.jsonl"
$runLog = Join-Path $EvidenceRoot "verification.json"
$script:CdpRequestId = 0
$script:CdpNotifications = [System.Collections.Generic.List[object]]::new()

function Write-CdpMessage {
    param([string]$Direction, [string]$Payload)
    $line = [ordered]@{
        time = (Get-Date).ToString("o")
        direction = $Direction
        payload = $Payload
    } | ConvertTo-Json -Compress
    Add-Content -LiteralPath $rawCdpLog -Value $line -Encoding utf8
}

function Receive-CdpMessage {
    param([System.Net.WebSockets.ClientWebSocket]$Socket)
    $memory = [System.IO.MemoryStream]::new()
    try {
        do {
            $buffer = [byte[]]::new(65536)
            $segment = [System.ArraySegment[byte]]::new($buffer)
            $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
            try {
                $received = $Socket.ReceiveAsync($segment, $cts.Token).GetAwaiter().GetResult()
            }
            finally {
                $cts.Dispose()
            }
            if ($received.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
                throw "CDP websocket closed before the expected response"
            }
            $memory.Write($buffer, 0, $received.Count)
        } while (-not $received.EndOfMessage)
        $payload = [System.Text.Encoding]::UTF8.GetString($memory.ToArray())
        Write-CdpMessage -Direction "receive" -Payload $payload
        return $payload | ConvertFrom-Json -Depth 100
    }
    finally {
        $memory.Dispose()
    }
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
    Write-CdpMessage -Direction "send" -Payload $request
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
    $segment = [System.ArraySegment[byte]]::new($bytes)
    $cts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try {
        $Socket.SendAsync(
            $segment,
            [System.Net.WebSockets.WebSocketMessageType]::Text,
            $true,
            $cts.Token
        ).GetAwaiter().GetResult()
    }
    finally {
        $cts.Dispose()
    }

    while ($true) {
        $message = Receive-CdpMessage -Socket $Socket
        if ($null -ne $message.id -and [int]$message.id -eq $requestId) {
            if ($null -ne $message.error) {
                throw "CDP $Method failed: $($message.error | ConvertTo-Json -Compress)"
            }
            return $message.result
        }
        $script:CdpNotifications.Add($message)
    }
}

function Invoke-CdpEvaluation {
    param(
        [System.Net.WebSockets.ClientWebSocket]$Socket,
        [string]$Expression
    )
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

function Save-WindowCapture {
    param([IntPtr]$WindowHandle, [string]$Path)
    Add-Type -AssemblyName System.Drawing
    if (-not ("DirectCdpNativeWindow" -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DirectCdpNativeWindow {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
'@
    }
    $rect = New-Object DirectCdpNativeWindow+RECT
    if (-not [DirectCdpNativeWindow]::GetWindowRect($WindowHandle, [ref]$rect)) {
        throw "GetWindowRect failed"
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    try {
        $captured = [DirectCdpNativeWindow]::PrintWindow($WindowHandle, $hdc, 2)
    }
    finally {
        $graphics.ReleaseHdc($hdc)
        $graphics.Dispose()
    }
    if (-not $captured) {
        $bitmap.Dispose()
        throw "PrintWindow failed"
    }
    try {
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bitmap.Dispose()
    }
    return [ordered]@{
        path = $Path
        width = $width
        height = $height
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash
    }
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
            if ($ids.Add([int]$child.ProcessId)) {
                $queue.Enqueue([int]$child.ProcessId)
            }
        }
    }
    return [ordered]@{ ids = @($ids); rows = @(
        $all | Where-Object { $ids.Contains([int]$_.ProcessId) } |
            Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine
    ) }
}

New-Item -ItemType Directory -Force -Path $EvidenceRoot, (Join-Path $EvidenceRoot "screenshots") | Out-Null
Set-Content -LiteralPath $rawCdpLog -Value "" -Encoding utf8
$run = [ordered]@{
    started_at = (Get-Date).ToString("o")
    expected_hash = $expectedHash
    exe = $debugExe
    localappdata = $localAppData
    cdp_port = $CdpPort
    fault_env = "route_cancel_once"
    state = "started"
}
$socket = $null
$process = $null
$descendants = $null
$failure = $null

try {
    $hashBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $debugExe).Hash
    if ($hashBefore -ne $expectedHash) {
        throw "debug hash drift before launch: $hashBefore"
    }
    if (@(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "CDP port $CdpPort was occupied before launch"
    }
    if (@(Get-ChildItem -LiteralPath $localAppData -Recurse -Force -File -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "debug LOCALAPPDATA was not empty before launch"
    }

    $oldLocal = $env:LOCALAPPDATA
    $oldBrowserArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    $oldFault = $env:DEVSWEEP_TASK_NATIVE_FAULT
    try {
        $env:LOCALAPPDATA = $localAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$CdpPort"
        $env:DEVSWEEP_TASK_NATIVE_FAULT = "route_cancel_once"
        $process = Start-Process -FilePath $debugExe -PassThru
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArgs
        $env:DEVSWEEP_TASK_NATIVE_FAULT = $oldFault
    }

    $deadline = (Get-Date).AddSeconds(40)
    $targets = @()
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        try {
            $targets = @(Invoke-RestMethod -Uri "http://127.0.0.1:$CdpPort/json/list" -TimeoutSec 2)
        }
        catch {
            $targets = @()
        }
        $matchingTargets = @($targets | Where-Object {
            $_.type -eq "page" -and $_.title -eq "devsweep" -and
            $_.url -like "http://127.0.0.1:4180/*"
        })
    } while (($process.MainWindowHandle -eq 0 -or $matchingTargets.Count -ne 1) -and (Get-Date) -lt $deadline)

    if ($process.HasExited) { throw "debug process exited before CDP target selection" }
    if ($process.MainWindowHandle -eq 0) { throw "debug HWND was unavailable" }
    if ($matchingTargets.Count -ne 1) {
        throw "expected one DevSweep CDP target, found $($matchingTargets.Count)"
    }
    $target = $matchingTargets[0]
    $listeners = @(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue)
    if ($listeners.Count -ne 1 -or $listeners[0].LocalAddress -notin @("127.0.0.1", "::1")) {
        throw "CDP listener was not a unique loopback listener"
    }
    $hashAfterLaunch = (Get-FileHash -Algorithm SHA256 -LiteralPath $debugExe).Hash
    if ($hashAfterLaunch -ne $expectedHash) {
        throw "debug hash drift after launch: $hashAfterLaunch"
    }
    $portOwner = Get-CimInstance Win32_Process -Filter "ProcessId=$($listeners[0].OwningProcess)" |
        Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine
    $run.launch = [ordered]@{
        pid = $process.Id
        start_time = $process.StartTime.ToString("o")
        hwnd = "0x{0:X}" -f $process.MainWindowHandle
        title = $process.MainWindowTitle
        size = (Get-Item -LiteralPath $debugExe).Length
        hash_before = $hashBefore
        hash_after_launch = $hashAfterLaunch
        target = $target
        listener = $listeners | Select-Object LocalAddress, LocalPort, OwningProcess
        port_owner = $portOwner
    }

    $socket = [System.Net.WebSockets.ClientWebSocket]::new()
    $connectCts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(10))
    try {
        $socket.ConnectAsync([Uri]$target.webSocketDebuggerUrl, $connectCts.Token).GetAwaiter().GetResult()
    }
    finally {
        $connectCts.Dispose()
    }
    if ($socket.State -ne [System.Net.WebSockets.WebSocketState]::Open) {
        throw "CDP websocket did not open"
    }

    Invoke-Cdp -Socket $socket -Method "Runtime.enable" | Out-Null
    Invoke-Cdp -Socket $socket -Method "Page.enable" | Out-Null
    Invoke-Cdp -Socket $socket -Method "Log.enable" | Out-Null
    Invoke-CdpEvaluation -Socket $socket -Expression @'
(() => {
  window.__devsweepDirectCdp = { unhandled: [], errors: [] };
  window.addEventListener("unhandledrejection", event => {
    window.__devsweepDirectCdp.unhandled.push(String(event.reason));
  });
  window.addEventListener("error", event => {
    window.__devsweepDirectCdp.errors.push(String(event.error ?? event.message));
  });
  return true;
})()
'@ | Out-Null

    $stateExpression = @'
(() => {
  const active = document.activeElement;
  const alert = document.querySelector('[role="alert"]');
  return {
    hash: location.hash,
    locale: document.querySelector('[data-locale]')?.getAttribute('data-locale') ?? null,
    activeTag: active?.tagName ?? null,
    activeText: active?.textContent?.trim() ?? null,
    alertText: alert?.querySelector('span')?.textContent?.trim() ?? null,
    alertButton: alert?.querySelector('button')?.textContent?.trim() ?? null,
    settingsPresent: Boolean(document.querySelector('select[aria-label="Language"]')),
    languageButtons: Array.from(document.querySelectorAll('button')).filter(button => button.textContent?.trim() === 'Language').length,
    unhandled: window.__devsweepDirectCdp?.unhandled ?? [],
    windowErrors: window.__devsweepDirectCdp?.errors ?? []
  };
})()
'@

    $initial = Invoke-CdpEvaluation -Socket $socket -Expression $stateExpression
    if ($initial.hash -ne "#/clean" -or $initial.locale -ne "en" -or $initial.languageButtons -ne 1) {
        throw "unexpected initial native state: $($initial | ConvertTo-Json -Compress)"
    }

    $firstClick = Invoke-CdpEvaluation -Socket $socket -Expression @'
(() => {
  const button = Array.from(document.querySelectorAll('button')).find(item => item.textContent?.trim() === 'Language');
  if (!button) return { clicked: false };
  button.focus();
  button.click();
  return { clicked: true, active: document.activeElement?.textContent?.trim() ?? null };
})()
'@
    if (-not $firstClick.clicked) { throw "Language button was not clicked" }
    Start-Sleep -Milliseconds 750
    $faultState = Invoke-CdpEvaluation -Socket $socket -Expression $stateExpression
    if ($faultState.hash -ne "#/clean") { throw "fault changed route to $($faultState.hash)" }
    if ($faultState.alertText -ne "Could not change destination. The current page remains active.") {
        throw "fault alert was not canonical: $($faultState.alertText)"
    }
    if ($faultState.alertButton -ne "Dismiss") { throw "fault dismiss action was unavailable" }
    if ($faultState.activeTag -ne "BUTTON" -or $faultState.activeText -ne "Language") {
        throw "fault committed incorrect focus: $($faultState.activeTag)/$($faultState.activeText)"
    }
    if ($faultState.settingsPresent) { throw "fault incorrectly rendered Settings" }
    $faultCapture = Save-WindowCapture -WindowHandle ([IntPtr]$process.MainWindowHandle) `
        -Path (Join-Path $EvidenceRoot "screenshots/debug-route-error-native.png")

    $dismissed = Invoke-CdpEvaluation -Socket $socket -Expression @'
(() => {
  const button = document.querySelector('[role="alert"] button');
  if (!button) return false;
  button.click();
  return true;
})()
'@
    if (-not $dismissed) { throw "Dismiss button was not clicked" }
    Start-Sleep -Milliseconds 250
    $afterDismiss = Invoke-CdpEvaluation -Socket $socket -Expression $stateExpression
    if ($null -ne $afterDismiss.alertText -or $afterDismiss.hash -ne "#/clean") {
        throw "dismiss did not restore the clean route"
    }

    $secondClick = Invoke-CdpEvaluation -Socket $socket -Expression @'
(() => {
  const button = Array.from(document.querySelectorAll('button')).find(item => item.textContent?.trim() === 'Language');
  if (!button) return false;
  button.focus();
  button.click();
  return true;
})()
'@
    if (-not $secondClick) { throw "second Language click was not sent" }
    Start-Sleep -Milliseconds 750
    $recovered = Invoke-CdpEvaluation -Socket $socket -Expression $stateExpression
    if ($recovered.hash -ne "#/settings" -or -not $recovered.settingsPresent) {
        throw "one-shot did not recover to Settings: $($recovered | ConvertTo-Json -Compress)"
    }
    if ($null -ne $recovered.alertText) { throw "route error remained after recovery" }
    if ($recovered.unhandled.Count -ne 0 -or $recovered.windowErrors.Count -ne 0) {
        throw "window observed an unhandled rejection or error"
    }
    $settingsCapture = Save-WindowCapture -WindowHandle ([IntPtr]$process.MainWindowHandle) `
        -Path (Join-Path $EvidenceRoot "screenshots/debug-settings-after-retry-native.png")

    $runtimeExceptions = @($script:CdpNotifications | Where-Object method -eq "Runtime.exceptionThrown")
    $errorConsole = @($script:CdpNotifications | Where-Object {
        $_.method -eq "Runtime.consoleAPICalled" -and $_.params.type -eq "error"
    })
    $errorLogEntries = @($script:CdpNotifications | Where-Object {
        $_.method -eq "Log.entryAdded" -and $_.params.entry.level -eq "error"
    })
    $knownViteFaviconErrors = @($errorLogEntries | Where-Object {
        $_.params.entry.source -eq "network" -and
        $_.params.entry.url -eq "http://127.0.0.1:4180/favicon.ico" -and
        $_.params.entry.text -eq "Failed to load resource: the server responded with a status of 404 (Not Found)"
    })
    $unexpectedErrorLogEntries = @($errorLogEntries | Where-Object {
        $_.params.entry.source -ne "network" -or
        $_.params.entry.url -ne "http://127.0.0.1:4180/favicon.ico" -or
        $_.params.entry.text -ne "Failed to load resource: the server responded with a status of 404 (Not Found)"
    })
    if ($runtimeExceptions.Count -ne 0 -or $errorConsole.Count -ne 0 -or $unexpectedErrorLogEntries.Count -ne 0) {
        throw "CDP observed exception/error console output"
    }

    $run.interaction = [ordered]@{
        initial = $initial
        first_click = $firstClick
        fault = $faultState
        fault_capture = $faultCapture
        after_dismiss = $afterDismiss
        second_click = $secondClick
        recovered = $recovered
        settings_capture = $settingsCapture
        runtime_exception_count = $runtimeExceptions.Count
        error_console_count = $errorConsole.Count
        error_log_entry_count = $errorLogEntries.Count
        known_vite_favicon_404_count = $knownViteFaviconErrors.Count
        unexpected_error_log_entry_count = $unexpectedErrorLogEntries.Count
        notification_count = $script:CdpNotifications.Count
    }
    $run.state = "interaction_pass"
}
catch {
    $failure = [ordered]@{
        message = $_.Exception.Message
        type = $_.Exception.GetType().FullName
        position = $_.InvocationInfo.PositionMessage
        stack = $_.ScriptStackTrace
    }
    $run.state = "failed"
    $run.failure = $failure
}
finally {
    if ($null -ne $socket) {
        try {
            if ($socket.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
                $closeCts = [System.Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(3))
                try {
                    $socket.CloseAsync(
                        [System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure,
                        "evidence complete",
                        $closeCts.Token
                    ).GetAwaiter().GetResult()
                }
                finally {
                    $closeCts.Dispose()
                }
            }
        }
        catch {
            $run.websocket_close_error = $_.Exception.Message
        }
        $socket.Dispose()
    }

    if ($null -ne $process) {
        try {
            if (-not $process.HasExited) {
                $descendants = Get-DescendantProcesses -RootPid $process.Id
                $closeStarted = Get-Date
                $closeResult = $process.CloseMainWindow()
                $process.WaitForExit(25000) | Out-Null
                $closeEnded = Get-Date
                $remainingDescendants = @(
                    foreach ($id in $descendants.ids) {
                        Get-Process -Id $id -ErrorAction SilentlyContinue |
                            Select-Object Id, ProcessName, Path, StartTime
                    }
                )
                $run.close = [ordered]@{
                    CloseMainWindow = $closeResult
                    elapsed_ms = [Math]::Round(($closeEnded - $closeStarted).TotalMilliseconds)
                    main_alive = -not $process.HasExited
                    descendants_before = $descendants.rows
                    descendant_residue = $remainingDescendants
                }
                if (-not $process.HasExited) {
                    $live = Get-Process -Id $process.Id -ErrorAction Stop
                    $livePath = $live.Path
                    $liveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $livePath).Hash
                    if ($livePath -ne $debugExe -or $liveHash -ne $expectedHash) {
                        throw "refused forced cleanup because the live process identity changed"
                    }
                    Stop-Process -Id $process.Id -ErrorAction Stop
                    $process.WaitForExit(5000) | Out-Null
                    $run.close.forced_cleanup = [ordered]@{
                        command = "Stop-Process -Id $($process.Id)"
                        identity_path = $livePath
                        identity_hash = $liveHash
                        alive_after = -not $process.HasExited
                    }
                }
            }
        }
        catch {
            $run.close_error = $_.Exception.Message
        }
    }

    Start-Sleep -Milliseconds 750
    $settingsRoot = Join-Path $localAppData "DevSweep/settings"
    $run.final = [ordered]@{
        hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $debugExe).Hash
        main_processes = @(Get-CimInstance Win32_Process | Where-Object {
            $_.Name -eq "devsweep-desktop.exe"
        } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        webview_processes = @(Get-CimInstance Win32_Process | Where-Object {
            $_.Name -eq "msedgewebview2.exe" -and
            $_.CommandLine -match "webview-exe-name=devsweep-desktop\\.exe"
        } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        cdp_listeners = @(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue |
            Select-Object LocalAddress, LocalPort, OwningProcess)
        settings_files = @(Get-ChildItem -LiteralPath $settingsRoot -Force -File -ErrorAction SilentlyContinue |
            Select-Object Name, Length, FullName)
    }
    $run.finished_at = (Get-Date).ToString("o")
    $run | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $runLog -Encoding utf8
}

if ($run.state -ne "interaction_pass") { exit 1 }
if ($run.close.CloseMainWindow -ne $true -or $run.close.main_alive -or $run.close.descendant_residue.Count -ne 0) { exit 2 }
if ($run.final.hash -ne $expectedHash) { exit 3 }
if ($run.final.main_processes.Count -ne 0 -or $run.final.webview_processes.Count -ne 0) { exit 4 }
if ($run.final.cdp_listeners.Count -ne 0 -or $run.final.settings_files.Count -ne 0) { exit 5 }
exit 0
