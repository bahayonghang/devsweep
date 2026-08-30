$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../../../../..")).Path
$logPath = Join-Path $PSScriptRoot "06-final-readonly-audit.log"
$expected = @(
    [pscustomobject]@{ path = "target/release/devsweep-desktop.exe"; bytes = 9827328; sha256 = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20" },
    [pscustomobject]@{ path = "target/debug/devsweep-desktop.exe"; bytes = 14758912; sha256 = "F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5" },
    [pscustomobject]@{ path = "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"; bytes = 3152372; sha256 = "0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328" }
)

Start-Transcript -Path $logPath -Force | Out-Null
try {
    Set-Location -LiteralPath $repositoryRoot
    foreach ($row in $expected) {
        $item = Get-Item -LiteralPath $row.path
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $row.path).Hash
        $match = $item.Length -eq $row.bytes -and $hash -eq $row.sha256
        Write-Output ("ARTIFACT path={0} bytes={1} sha256={2} matches={3}" -f $row.path, $item.Length, $hash, $match)
        if (-not $match) { throw "final artifact drift: $($row.path)" }
    }

    $main = @(Get-Process devsweep-desktop -ErrorAction SilentlyContinue)
    $webview = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" })
    $port1 = @(Get-NetTCPConnection -LocalPort 9381 -State Listen -ErrorAction SilentlyContinue)
    $port2 = @(Get-NetTCPConnection -LocalPort 9382 -State Listen -ErrorAction SilentlyContinue)
    $lockTemp = @(Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot "native-release/localappdata/DevSweep/settings") -File -Force -ErrorAction SilentlyContinue)
    Write-Output ("RESIDUE main={0} webview={1} port9381={2} port9382={3} lock_temp={4}" -f $main.Count, $webview.Count, $port1.Count, $port2.Count, $lockTemp.Count)
    if ($main.Count -ne 0 -or $webview.Count -ne 0 -or $port1.Count -ne 0 -or $port2.Count -ne 0 -or $lockTemp.Count -ne 0) { throw "final residue was not zero" }

    $oldRaw = ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-artifact-rebind-92c-20260830/02-native-release-rebind.log"
    Write-Output ("OLD_RUN1_02_EXISTS={0}" -f (Test-Path -LiteralPath $oldRaw))
    $run2Raw = Join-Path $PSScriptRoot "02-native-release-rebind.log"
    Write-Output ("RUN2_02 lines={0} bytes={1} sha256={2}" -f @(Get-Content -LiteralPath $run2Raw).Count, (Get-Item -LiteralPath $run2Raw).Length, (Get-FileHash -Algorithm SHA256 -LiteralPath $run2Raw).Hash)
    $cdp = Join-Path $PSScriptRoot "native-release/cdp-messages.jsonl"
    Write-Output ("RUN2_CDP lines={0} bytes={1} sha256={2}" -f @(Get-Content -LiteralPath $cdp).Count, (Get-Item -LiteralPath $cdp).Length, (Get-FileHash -Algorithm SHA256 -LiteralPath $cdp).Hash)

    & rtk git diff --check
    $diffExit = $LASTEXITCODE
    Write-Output ("DIFF_CHECK_EXIT={0}" -f $diffExit)
    & python ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand
    $validateExit = $LASTEXITCODE
    Write-Output ("TASK_VALIDATE_EXIT={0}" -f $validateExit)
    $status = @(& rtk git status --short)
    $statusExit = $LASTEXITCODE
    Write-Output ("GIT_STATUS_EXIT={0} entries={1}" -f $statusExit, $status.Count)
    $status | ForEach-Object { Write-Output ("STATUS {0}" -f $_) }
    if ($diffExit -ne 0 -or $validateExit -ne 0 -or $statusExit -ne 0) { throw "read-only repository audit failed" }
    Write-Output "FINAL_READONLY_AUDIT=PASS"
}
finally {
    Stop-Transcript | Out-Null
}
