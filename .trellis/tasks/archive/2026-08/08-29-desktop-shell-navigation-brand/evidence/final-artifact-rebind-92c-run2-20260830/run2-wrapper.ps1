param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$RunRoot,
    [int]$FirstPort = 9381,
    [int]$RestartPort = 9382
)

$ErrorActionPreference = "Stop"
$expected = [ordered]@{
    release = [ordered]@{
        path = "target/release/devsweep-desktop.exe"
        bytes = 9827328
        sha256 = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20"
    }
    debug = [ordered]@{
        path = "target/debug/devsweep-desktop.exe"
        bytes = 14758912
        sha256 = "F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5"
    }
    nsis = [ordered]@{
        path = "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"
        bytes = 3152372
        sha256 = "0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328"
    }
}

$resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$resolvedRunRoot = (Resolve-Path -LiteralPath $RunRoot).Path
$helper = Join-Path $resolvedRunRoot "release-rebind-run2.ps1"
$nativeRoot = Join-Path $resolvedRunRoot "native-release"
$transcript = Join-Path $resolvedRunRoot "02-native-release-rebind.log"
$preflightPath = Join-Path $resolvedRunRoot "01-preflight-hashes.json"
$postflightPath = Join-Path $resolvedRunRoot "03-post-run-hashes.json"

if (Test-Path -LiteralPath $transcript) { throw "run2 transcript already exists; refusing to overwrite" }
if (-not (Test-Path -LiteralPath $helper -PathType Leaf)) { throw "run2 helper is missing" }

function Get-ArtifactAudit {
    param([string]$Stage)
    $rows = foreach ($entry in $expected.GetEnumerator()) {
        $fullPath = Join-Path $resolvedRepository $entry.Value.path
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) { throw "$Stage missing artifact: $fullPath" }
        $item = Get-Item -LiteralPath $fullPath
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fullPath).Hash
        $row = [ordered]@{
            name = $entry.Key
            stage = $Stage
            path = $item.FullName
            bytes = $item.Length
            sha256 = $hash
            expected_bytes = $entry.Value.bytes
            expected_sha256 = $entry.Value.sha256
            matches = ($item.Length -eq $entry.Value.bytes -and $hash -eq $entry.Value.sha256)
        }
        if (-not $row.matches) { throw "$Stage artifact drift: $($entry.Key) bytes=$($item.Length) hash=$hash" }
        [pscustomobject]$row
    }
    return @($rows)
}

function Get-ResidueAudit {
    return [ordered]@{
        captured_at = (Get-Date).ToString("o")
        main = @(Get-CimInstance Win32_Process | Where-Object Name -eq "devsweep-desktop.exe" | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        webview = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
        first_port = @(Get-NetTCPConnection -LocalPort $FirstPort -State Listen -ErrorAction SilentlyContinue | Select-Object LocalAddress, LocalPort, OwningProcess)
        restart_port = @(Get-NetTCPConnection -LocalPort $RestartPort -State Listen -ErrorAction SilentlyContinue | Select-Object LocalAddress, LocalPort, OwningProcess)
        vite = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @("node.exe", "cmd.exe") -and $_.CommandLine -match "vite" -and $_.CommandLine -match "devsweep|4180" } | Select-Object ProcessId, ParentProcessId, Name, ExecutablePath, CreationDate, CommandLine)
    }
}

$wrapperExit = 99
Start-Transcript -Path $transcript -Force | Out-Null
try {
    Write-Output ("RUN2_WRAPPER_START={0}" -f (Get-Date).ToString("o"))
    Write-Output ("REPOSITORY_ROOT={0}" -f $resolvedRepository)
    Write-Output ("RUN_ROOT={0}" -f $resolvedRunRoot)
    Write-Output ("HELPER_PATH={0}" -f $helper)
    Write-Output ("WRAPPER_SHA256={0}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash)
    Write-Output ("HELPER_SHA256={0}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $helper).Hash)
    Write-Output ("PORTS={0},{1}" -f $FirstPort, $RestartPort)

    $preflight = Get-ArtifactAudit -Stage "preflight"
    $preflight | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $preflightPath -Encoding utf8
    foreach ($row in $preflight) {
        Write-Output ("PREFLIGHT_ARTIFACT name={0} bytes={1} sha256={2} matches={3}" -f $row.name, $row.bytes, $row.sha256, $row.matches)
    }

    $preResidue = Get-ResidueAudit
    Write-Output ("PREFLIGHT_RESIDUE main={0} webview={1} port1={2} port2={3} vite={4}" -f $preResidue.main.Count, $preResidue.webview.Count, $preResidue.first_port.Count, $preResidue.restart_port.Count, $preResidue.vite.Count)
    if ($preResidue.main.Count -ne 0 -or $preResidue.webview.Count -ne 0 -or $preResidue.first_port.Count -ne 0 -or $preResidue.restart_port.Count -ne 0 -or $preResidue.vite.Count -ne 0) {
        throw "preflight process/listener residue was not zero"
    }

    Write-Output ("HELPER_COMMAND=powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"{0}`" -RepositoryRoot `"{1}`" -EvidenceRoot `"{2}`" -FirstPort {3} -RestartPort {4} -ExpectedHash {5}" -f $helper, $resolvedRepository, $nativeRoot, $FirstPort, $RestartPort, $expected.release.sha256)
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $helper -RepositoryRoot $resolvedRepository -EvidenceRoot $nativeRoot -FirstPort $FirstPort -RestartPort $RestartPort -ExpectedHash $expected.release.sha256 2>&1 | ForEach-Object { Write-Output $_ }
    $helperExit = $LASTEXITCODE
    Write-Output ("HELPER_EXIT_CODE={0}" -f $helperExit)

    $postflight = Get-ArtifactAudit -Stage "postflight"
    $postResidue = Get-ResidueAudit
    [ordered]@{ artifacts = $postflight; residue = $postResidue } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $postflightPath -Encoding utf8
    foreach ($row in $postflight) {
        Write-Output ("POSTFLIGHT_ARTIFACT name={0} bytes={1} sha256={2} matches={3}" -f $row.name, $row.bytes, $row.sha256, $row.matches)
    }
    Write-Output ("POSTFLIGHT_RESIDUE main={0} webview={1} port1={2} port2={3} vite={4}" -f $postResidue.main.Count, $postResidue.webview.Count, $postResidue.first_port.Count, $postResidue.restart_port.Count, $postResidue.vite.Count)
    if ($helperExit -ne 0) { throw "native release helper exited $helperExit" }
    if ($postResidue.main.Count -ne 0 -or $postResidue.webview.Count -ne 0 -or $postResidue.first_port.Count -ne 0 -or $postResidue.restart_port.Count -ne 0 -or $postResidue.vite.Count -ne 0) {
        throw "postflight process/listener residue was not zero"
    }
    $wrapperExit = 0
}
catch {
    Write-Error ("RUN2_FAILURE={0}`n{1}" -f $_.Exception.Message, $_.ScriptStackTrace)
    if ($wrapperExit -eq 99) { $wrapperExit = 1 }
}
finally {
    Write-Output ("EXIT_CODE={0}" -f $wrapperExit)
    Write-Output ("RUN2_WRAPPER_FINISHED={0}" -f (Get-Date).ToString("o"))
    Stop-Transcript | Out-Null
}

exit $wrapperExit
