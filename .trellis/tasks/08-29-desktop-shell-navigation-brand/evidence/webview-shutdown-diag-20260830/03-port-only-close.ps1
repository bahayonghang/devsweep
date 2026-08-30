$ErrorActionPreference = "Stop"
$root = "D:\Documents\Code\Rust\Exp\devsweep"
Set-Location -LiteralPath $root

$runRoot = Join-Path $root ".trellis\tasks\08-29-desktop-shell-navigation-brand\evidence\webview-shutdown-diag-20260830"
$localAppData = Join-Path $runRoot "port-only-localappdata"
$transcript = Join-Path $runRoot "03-port-only-close.log"
$jsonOut = Join-Path $runRoot "03-port-only-close.json"
$expectedHash = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20"
$releaseExe = Join-Path $root "target\release\devsweep-desktop.exe"
$port = 9397

Start-Transcript -LiteralPath $transcript -Force | Out-Null
$audit = [ordered]@{ started_at = (Get-Date).ToString("o"); port = $port }

function Get-OwnedWebViews {
    param([int]$RootPid)
    @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object {
        [int]$_.ParentProcessId -eq $RootPid -or
        $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" -or
        $_.CommandLine -match ("mojo-named-platform-channel-pipe={0}\." -f $RootPid)
    } | Select-Object ProcessId, ParentProcessId, Name, CommandLine)
}

try {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    if ($hash -ne $expectedHash) { throw "release hash drift $hash" }
    $audit.release_hash = $hash
    if (@(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "port occupied"
    }
    if (Test-Path -LiteralPath $localAppData) { Remove-Item -LiteralPath $localAppData -Recurse -Force }
    New-Item -ItemType Directory -Path $localAppData | Out-Null
    if (@(Get-Process -Name "devsweep-desktop" -ErrorAction SilentlyContinue).Count -ne 0) { throw "prelaunch main residue" }

    $oldLocal = $env:LOCALAPPDATA
    $oldBrowserArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    try {
        $env:LOCALAPPDATA = $localAppData
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$port"
        $proc = Start-Process -FilePath $releaseExe -PassThru
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        if ($null -eq $oldBrowserArgs) { Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue }
        else { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArgs }
    }

    $readyDeadline = (Get-Date).AddSeconds(40)
    do {
        Start-Sleep -Milliseconds 100
        $proc.Refresh()
        try { $targets = @(Invoke-RestMethod -Uri "http://127.0.0.1:$port/json/list" -TimeoutSec 1) } catch { $targets = @() }
        $matching = @($targets | Where-Object { $_.type -eq "page" -and $_.title -eq "devsweep" })
    } while (($proc.MainWindowHandle -eq 0 -or $matching.Count -lt 1) -and -not $proc.HasExited -and (Get-Date) -lt $readyDeadline)
    if ($proc.HasExited -or $proc.MainWindowHandle -eq 0) { throw "window did not appear" }

    $ownedBefore = @(Get-OwnedWebViews -RootPid $proc.Id)
    Write-Output ("LAUNCH pid={0} hwnd=0x{1:X} targets={2} webviews={3} (no CDP websocket)" -f $proc.Id, $proc.MainWindowHandle, $matching.Count, $ownedBefore.Count)
    Start-Sleep -Seconds 2

    $closeRequestedAt = Get-Date
    $closeOk = $proc.CloseMainWindow()
    $exited = $proc.WaitForExit(25000)
    Write-Output ("CLOSE close_main_window={0} wait_for_exit={1} main_elapsed_ms={2}" -f $closeOk, $exited, [Math]::Round(((Get-Date) - $closeRequestedAt).TotalMilliseconds))

    $samples = @()
    $deadline = (Get-Date).AddSeconds(20)
    $i = 0
    do {
        $i += 1
        Start-Sleep -Milliseconds 200
        $owned = @(Get-OwnedWebViews -RootPid $proc.Id)
        $listeners = @(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
        $sample = [ordered]@{
            i = $i
            ms_after_close = [Math]::Round(((Get-Date) - $closeRequestedAt).TotalMilliseconds)
            owned = $owned.Count
            listeners = $listeners.Count
            pids = @($owned | ForEach-Object { $_.ProcessId })
        }
        $samples += $sample
        Write-Output ("SAMPLE i={0} after_ms={1} owned={2} listeners={3} pids={4}" -f $i, $sample.ms_after_close, $owned.Count, $listeners.Count, ($sample.pids -join ","))
    } while (($owned.Count -ne 0 -or $listeners.Count -ne 0) -and (Get-Date) -lt $deadline)

    $finalOwned = @(Get-OwnedWebViews -RootPid $proc.Id)
    $audit.close = [ordered]@{
        close_main_window = $closeOk
        wait_for_exit = $exited
        samples = $samples
        final_owned = @($finalOwned | ForEach-Object { [ordered]@{ pid = $_.ProcessId; parent = $_.ParentProcessId; command = $_.CommandLine } })
    }
    if ($finalOwned.Count -ne 0) {
        foreach ($row in $finalOwned) {
            if ([string]$row.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" -or [int]$row.ParentProcessId -eq $proc.Id) {
                Stop-Process -Id $row.ProcessId -Force -ErrorAction SilentlyContinue
            }
        }
        $audit.state = "fail"
        Write-Output ("TERMINATED leftover={0}" -f $finalOwned.Count)
    }
    else {
        $audit.state = "pass"
        Write-Output "FINAL owned=0 listeners=0"
    }
}
catch {
    $audit.state = "error"
    $audit.error = $_.Exception.Message
    Write-Output ("ERROR {0}" -f $_.Exception.Message)
    throw
}
finally {
    $audit.finished_at = (Get-Date).ToString("o")
    $audit | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $jsonOut -Encoding utf8
    Stop-Transcript | Out-Null
}
