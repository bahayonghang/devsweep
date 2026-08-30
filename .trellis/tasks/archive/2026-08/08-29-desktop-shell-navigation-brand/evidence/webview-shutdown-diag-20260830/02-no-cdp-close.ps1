$ErrorActionPreference = "Stop"
$root = "D:\Documents\Code\Rust\Exp\devsweep"
Set-Location -LiteralPath $root

$runRoot = Join-Path $root ".trellis\tasks\08-29-desktop-shell-navigation-brand\evidence\webview-shutdown-diag-20260830"
$localAppData = Join-Path $runRoot "no-cdp-localappdata"
$transcript = Join-Path $runRoot "02-no-cdp-close.log"
$jsonOut = Join-Path $runRoot "02-no-cdp-close.json"
$expectedHash = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20"
$releaseExe = Join-Path $root "target\release\devsweep-desktop.exe"

Start-Transcript -LiteralPath $transcript -Force | Out-Null
$audit = [ordered]@{
    started_at = (Get-Date).ToString("o")
    expected_hash = $expectedHash
}

function Get-OwnedWebViews {
    param([int]$RootPid)
    @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object {
        [int]$_.ParentProcessId -eq $RootPid -or
        $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" -or
        $_.CommandLine -match ("mojo-named-platform-channel-pipe={0}\." -f $RootPid)
    } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
}

try {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseExe).Hash
    if ($hash -ne $expectedHash) { throw "release hash drift $hash" }
    $audit.release_hash = $hash
    $audit.release_bytes = (Get-Item -LiteralPath $releaseExe).Length

    if (Test-Path -LiteralPath $localAppData) {
        Remove-Item -LiteralPath $localAppData -Recurse -Force
    }
    New-Item -ItemType Directory -Path $localAppData | Out-Null

    $preMain = @(Get-Process -Name "devsweep-desktop" -ErrorAction SilentlyContinue)
    $preOwned = @(Get-OwnedWebViews -RootPid 0 | Where-Object { $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" })
    if ($preMain.Count -ne 0 -or $preOwned.Count -ne 0) {
        throw ("prelaunch residue main={0} owned_webview={1}" -f $preMain.Count, $preOwned.Count)
    }

    $oldLocal = $env:LOCALAPPDATA
    $oldBrowserArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    $oldUserData = $env:WEBVIEW2_USER_DATA_FOLDER
    try {
        $env:LOCALAPPDATA = $localAppData
        Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
        Remove-Item Env:WEBVIEW2_USER_DATA_FOLDER -ErrorAction SilentlyContinue
        $proc = Start-Process -FilePath $releaseExe -PassThru
    }
    finally {
        $env:LOCALAPPDATA = $oldLocal
        if ($null -eq $oldBrowserArgs) { Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue }
        else { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArgs }
        if ($null -eq $oldUserData) { Remove-Item Env:WEBVIEW2_USER_DATA_FOLDER -ErrorAction SilentlyContinue }
        else { $env:WEBVIEW2_USER_DATA_FOLDER = $oldUserData }
    }

    $readyDeadline = (Get-Date).AddSeconds(40)
    do {
        Start-Sleep -Milliseconds 100
        $proc.Refresh()
    } while ($proc.MainWindowHandle -eq 0 -and -not $proc.HasExited -and (Get-Date) -lt $readyDeadline)
    if ($proc.HasExited -or $proc.MainWindowHandle -eq 0) {
        throw "window did not appear"
    }

    $cimStart = Get-Date
    $ownedBefore = @(Get-OwnedWebViews -RootPid $proc.Id)
    $cimBeforeMs = ((Get-Date) - $cimStart).TotalMilliseconds
    $audit.launch = [ordered]@{
        pid = $proc.Id
        hwnd = [int64]$proc.MainWindowHandle
        start_time = $proc.StartTime.ToString("o")
        path = $proc.Path
        owned_webview_before = @($ownedBefore | ForEach-Object {
            [ordered]@{
                pid = $_.ProcessId
                parent = $_.ParentProcessId
                command = $_.CommandLine
            }
        })
        owned_webview_before_count = $ownedBefore.Count
        cim_filter_ms = [Math]::Round($cimBeforeMs)
    }
    Write-Output ("LAUNCH pid={0} hwnd=0x{1:X} webviews={2} cim_ms={3:N0}" -f $proc.Id, $proc.MainWindowHandle, $ownedBefore.Count, $cimBeforeMs)

    Start-Sleep -Seconds 2
    $closeRequestedAt = Get-Date
    $closeOk = $proc.CloseMainWindow()
    $exited = $proc.WaitForExit(25000)
    $mainExitAt = Get-Date
    $mainElapsedMs = [Math]::Round(($mainExitAt - $closeRequestedAt).TotalMilliseconds)
    Write-Output ("CLOSE close_main_window={0} wait_for_exit={1} main_elapsed_ms={2}" -f $closeOk, $exited, $mainElapsedMs)

    $samples = @()
    $deadline = (Get-Date).AddSeconds(30)
    $cheapIntervalMs = 100
    $identityEvery = 10
    $i = 0
    do {
        $i += 1
        Start-Sleep -Milliseconds $cheapIntervalMs
        $cheapStart = Get-Date
        $mainAlive = @(Get-Process -Id $proc.Id -ErrorAction SilentlyContinue)
        $wvProcs = @(Get-Process -Name "msedgewebview2" -ErrorAction SilentlyContinue)
        $cheapMs = ((Get-Date) - $cheapStart).TotalMilliseconds
        $sample = [ordered]@{
            i = $i
            time = (Get-Date).ToString("o")
            ms_after_close = [Math]::Round(((Get-Date) - $closeRequestedAt).TotalMilliseconds)
            cheap_ms = [Math]::Round($cheapMs)
            main_count = $mainAlive.Count
            webview_process_count = $wvProcs.Count
        }
        if ($i % $identityEvery -eq 1) {
            $idStart = Get-Date
            $owned = @(Get-OwnedWebViews -RootPid $proc.Id)
            $sample.identity_ms = [Math]::Round(((Get-Date) - $idStart).TotalMilliseconds)
            $sample.owned_webview_count = $owned.Count
            $sample.owned = @($owned | ForEach-Object {
                [ordered]@{
                    pid = $_.ProcessId
                    parent = $_.ParentProcessId
                    command = $_.CommandLine
                }
            })
            Write-Output ("SAMPLE i={0} after_ms={1} main={2} get_process_webview={3} owned={4} cheap_ms={5} identity_ms={6}" -f $i, $sample.ms_after_close, $sample.main_count, $sample.webview_process_count, $owned.Count, $sample.cheap_ms, $sample.identity_ms)
        }
        else {
            $sample.owned_webview_count = $null
            Write-Output ("SAMPLE i={0} after_ms={1} main={2} get_process_webview={3} cheap_ms={4}" -f $i, $sample.ms_after_close, $sample.main_count, $sample.webview_process_count, $sample.cheap_ms)
        }
        $samples += $sample
        $ownedCount = if ($null -ne $sample.owned_webview_count) { $sample.owned_webview_count } else { 1 }
    } while (($mainAlive.Count -ne 0 -or $ownedCount -ne 0) -and (Get-Date) -lt $deadline)

    $finalOwned = @(Get-OwnedWebViews -RootPid $proc.Id)
    $audit.close = [ordered]@{
        close_main_window = $closeOk
        wait_for_exit = $exited
        main_elapsed_ms = $mainElapsedMs
        close_requested_at = $closeRequestedAt.ToString("o")
        main_exit_at = $mainExitAt.ToString("o")
        samples = $samples
        final_owned_count = $finalOwned.Count
        final_owned = @($finalOwned | ForEach-Object {
            [ordered]@{
                pid = $_.ProcessId
                parent = $_.ParentProcessId
                command = $_.CommandLine
            }
        })
    }

    if ($finalOwned.Count -ne 0) {
        $terminated = @()
        foreach ($row in $finalOwned) {
            $cmd = [string]$row.CommandLine
            $okName = $row.Name -eq "msedgewebview2.exe"
            $okOwner = $cmd -match "webview-exe-name=devsweep-desktop\.exe" -or [int]$row.ParentProcessId -eq $proc.Id
            if ($okName -and $okOwner) {
                Stop-Process -Id $row.ProcessId -Force -ErrorAction SilentlyContinue
                $terminated += [ordered]@{ pid = $row.ProcessId; parent = $row.ParentProcessId; command = $cmd }
            }
        }
        $audit.identity_gated_termination = $terminated
        Write-Output ("TERMINATED count={0}" -f $terminated.Count)
    }
    else {
        $audit.identity_gated_termination = @()
        Write-Output "FINAL owned=0"
    }
    $audit.state = if ($finalOwned.Count -eq 0 -and $exited) { "pass" } else { "fail" }
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
