$ErrorActionPreference = "Stop"
$repository = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../../../../..")).Path
$log = Join-Path $PSScriptRoot "03-final-readonly-audit.log"
$native = Join-Path $PSScriptRoot "native-release"
$expected = @(
    [pscustomobject]@{ path = "target/release/devsweep-desktop.exe"; bytes = 9827328; sha256 = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20" },
    [pscustomobject]@{ path = "target/debug/devsweep-desktop.exe"; bytes = 14758912; sha256 = "F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5" },
    [pscustomobject]@{ path = "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"; bytes = 3152372; sha256 = "0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328" }
)

if (Test-Path -LiteralPath $log) { throw "final audit log exists; refusing overwrite" }
Start-Transcript -Path $log | Out-Null
try {
    Set-Location -LiteralPath $repository
    foreach ($row in $expected) {
        $item = Get-Item -LiteralPath $row.path
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $row.path).Hash
        $match = $item.Length -eq $row.bytes -and $hash -eq $row.sha256
        Write-Output ("ARTIFACT path={0} bytes={1} sha256={2} matches={3}" -f $row.path, $item.Length, $hash, $match)
        if (-not $match) { throw "artifact drift: $($row.path)" }
    }

    $outer = Get-Content -LiteralPath (Join-Path $PSScriptRoot "outer-exit.json") -Raw | ConvertFrom-Json -Depth 100
    $transcript = Join-Path $native "02-native-release-rebind.log"
    $cdp = Join-Path $native "cdp-messages.jsonl"
    $cdpRows = @(Get-Content -LiteralPath $cdp | ForEach-Object { $_ | ConvertFrom-Json -Depth 100 })
    Write-Output ("OUTER helper_exit={0} transcript_lines={1} transcript_hash={2} cdp_lines={3} cdp_hash={4}" -f $outer.helper_exit, $outer.transcript.lines, $outer.transcript.sha256, $outer.cdp.lines, $outer.cdp.sha256)
    Write-Output ("LIVE_FILES transcript_lines={0} transcript_hash={1} cdp_lines={2} cdp_hash={3} valid_cdp_json={4}" -f @(Get-Content -LiteralPath $transcript).Count, (Get-FileHash -Algorithm SHA256 -LiteralPath $transcript).Hash, $cdpRows.Count, (Get-FileHash -Algorithm SHA256 -LiteralPath $cdp).Hash, $cdpRows.Count)
    if ($outer.helper_exit -ne 1 -or $outer.transcript.sha256 -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $transcript).Hash -or $outer.cdp.sha256 -ne (Get-FileHash -Algorithm SHA256 -LiteralPath $cdp).Hash -or $cdpRows.Count -ne 22) { throw "raw evidence consistency audit failed" }

    $main = @(Get-CimInstance Win32_Process | Where-Object Name -eq "devsweep-desktop.exe")
    $webview = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" })
    $port1 = @(Get-NetTCPConnection -LocalPort 9391 -State Listen -ErrorAction SilentlyContinue)
    $port2 = @(Get-NetTCPConnection -LocalPort 9392 -State Listen -ErrorAction SilentlyContinue)
    $lockTemp = @(Get-ChildItem -LiteralPath (Join-Path $native "localappdata/DevSweep/settings") -File -Force -ErrorAction SilentlyContinue | Where-Object Name -ne "presentation-v1.json")
    Write-Output ("RESIDUE main={0} webview={1} port9391={2} port9392={3} lock_temp={4}" -f $main.Count, $webview.Count, $port1.Count, $port2.Count, $lockTemp.Count)
    if ($main.Count -ne 0 -or $webview.Count -ne 0 -or $port1.Count -ne 0 -or $port2.Count -ne 0 -or $lockTemp.Count -ne 0) { throw "final residue was not zero" }

    & rtk git diff --check
    $diffExit = $LASTEXITCODE
    Write-Output ("DIFF_CHECK_EXIT={0}" -f $diffExit)
    & python ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand
    $validateExit = $LASTEXITCODE
    Write-Output ("TASK_VALIDATE_EXIT={0}" -f $validateExit)
    $status = @(& rtk git status --short)
    $statusExit = $LASTEXITCODE
    Write-Output ("GIT_STATUS_EXIT={0} entries={1}" -f $statusExit, $status.Count)
    if ($diffExit -ne 0 -or $validateExit -ne 0 -or $statusExit -ne 0) { throw "repository read-only audit failed" }
    Write-Output "FINAL_READONLY_AUDIT=PASS"
}
finally {
    Stop-Transcript | Out-Null
}
