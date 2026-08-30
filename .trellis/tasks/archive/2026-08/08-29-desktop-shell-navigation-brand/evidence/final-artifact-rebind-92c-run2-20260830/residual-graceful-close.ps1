$ErrorActionPreference = "Stop"
$pidToClose = 16920
$expectedPath = "D:\Documents\Code\Rust\Exp\devsweep\target\release\devsweep-desktop.exe"
$expectedHash = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20"
$expectedStart = "2026-08-30T20:32:40.8018019+08:00"
$port = 9381
$logPath = Join-Path $PSScriptRoot "05-residual-graceful-close.log"

Start-Transcript -Path $logPath -Force | Out-Null
try {
    $cim = Get-CimInstance Win32_Process -Filter "ProcessId = $pidToClose"
    if ($null -eq $cim) { throw "expected residual PID $pidToClose was absent before graceful close" }
    $process = Get-Process -Id $pidToClose -ErrorAction Stop
    $actualPath = $process.Path
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $actualPath).Hash
    $actualStart = $process.StartTime.ToString("o")
    Write-Output ("IDENTITY pid={0} path={1} hash={2} start={3} hwnd=0x{4:X} title={5} command={6}" -f $pidToClose, $actualPath, $actualHash, $actualStart, $process.MainWindowHandle, $process.MainWindowTitle, $cim.CommandLine)
    if ($actualPath -ne $expectedPath -or $actualHash -ne $expectedHash -or $actualStart -ne $expectedStart -or $process.MainWindowHandle -eq 0) {
        throw "residual identity changed; refusing graceful-close action"
    }
    $closeRequestedAt = (Get-Date).ToString("o")
    $closeResult = $process.CloseMainWindow()
    Write-Output ("CLOSE_MAIN_WINDOW requested_at={0} result={1}" -f $closeRequestedAt, $closeResult)
    $samples = @()
    $deadline = (Get-Date).AddSeconds(25)
    do {
        Start-Sleep -Milliseconds 250
        $main = @(Get-CimInstance Win32_Process -Filter "ProcessId = $pidToClose")
        $webview = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and ($_.ParentProcessId -eq $pidToClose -or $_.CommandLine -match "mojo-named-platform-channel-pipe=$pidToClose\.") })
        $listener = @(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
        $sample = [ordered]@{ time = (Get-Date).ToString("o"); main = $main.Count; webview = $webview.Count; listener = $listener.Count }
        $samples += [pscustomobject]$sample
        Write-Output ("SAMPLE time={0} main={1} webview={2} listener={3}" -f $sample.time, $sample.main, $sample.webview, $sample.listener)
    } while (($main.Count -ne 0 -or $webview.Count -ne 0 -or $listener.Count -ne 0) -and (Get-Date) -lt $deadline)
    Write-Output ("FINAL main={0} webview={1} listener={2} forced_termination=false" -f $main.Count, $webview.Count, $listener.Count)
    if (-not $closeResult -or $main.Count -ne 0 -or $webview.Count -ne 0 -or $listener.Count -ne 0) { exit 1 }
    exit 0
}
finally {
    Stop-Transcript | Out-Null
}
