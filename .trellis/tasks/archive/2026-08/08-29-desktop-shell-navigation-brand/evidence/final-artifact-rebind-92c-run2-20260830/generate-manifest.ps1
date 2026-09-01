$ErrorActionPreference = "Stop"
$runRoot = $PSScriptRoot
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $runRoot "../../../../..")).Path
$manifestPath = Join-Path $runRoot "evidence-manifest.json"

$artifactPaths = @(
    "target/release/devsweep-desktop.exe",
    "target/debug/devsweep-desktop.exe",
    "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"
)
$artifacts = foreach ($relative in $artifactPaths) {
    $path = Join-Path $repositoryRoot $relative
    $item = Get-Item -LiteralPath $path
    [pscustomobject][ordered]@{
        path = $relative
        bytes = $item.Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
    }
}

$files = Get-ChildItem -LiteralPath $runRoot -Recurse -File -Force |
    Where-Object FullName -ne $manifestPath |
    Sort-Object FullName |
    ForEach-Object {
        [pscustomobject][ordered]@{
            path = $_.FullName.Substring($runRoot.Length + 1).Replace("\", "/")
            bytes = $_.Length
            lines = if ($_.Extension -in @(".log", ".json", ".jsonl", ".md", ".ps1", ".txt")) { @(Get-Content -LiteralPath $_.FullName).Count } else { $null }
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash
        }
    }

$reportPaths = @(
    ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/verification-report.md",
    ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-artifact-rebind-92c-20260830/verification.md"
)
$reports = foreach ($relative in $reportPaths) {
    $path = Join-Path $repositoryRoot $relative
    $item = Get-Item -LiteralPath $path
    [pscustomobject][ordered]@{
        path = $relative
        bytes = $item.Length
        lines = @(Get-Content -LiteralPath $path).Count
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
    }
}

$cdpPath = Join-Path $runRoot "native-release/cdp-messages.jsonl"
$transcriptPath = Join-Path $runRoot "02-native-release-rebind.log"
$manifest = [ordered]@{
    generated_at = (Get-Date).ToString("o")
    decision = "rejected_incomplete_run2"
    wrapper_observed_exit_code = 1
    helper_exit_code = 1
    transcript_internal_exit_sentinel = 99
    transcript = [ordered]@{
        path = "02-native-release-rebind.log"
        lines = @(Get-Content -LiteralPath $transcriptPath).Count
        bytes = (Get-Item -LiteralPath $transcriptPath).Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $transcriptPath).Hash
    }
    cdp = [ordered]@{
        path = "native-release/cdp-messages.jsonl"
        lines = @(Get-Content -LiteralPath $cdpPath).Count
        bytes = (Get-Item -LiteralPath $cdpPath).Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $cdpPath).Hash
    }
    missing_expected_outputs = @(
        "native-release/shell-tray-wnd-uia-raw.json",
        "native-release/screenshots/*.png",
        "native-release/localappdata/DevSweep/settings/presentation-v1.json"
    )
    old_run1_raw_02_exists = Test-Path -LiteralPath (Join-Path $repositoryRoot ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-artifact-rebind-92c-20260830/02-native-release-rebind.log")
    artifacts = @($artifacts)
    files = @($files)
    external_reports = @($reports)
}
$manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $manifestPath -Encoding utf8

$item = Get-Item -LiteralPath $manifestPath
[pscustomobject]@{
    path = $item.FullName
    bytes = $item.Length
    lines = @(Get-Content -LiteralPath $item.FullName).Count
    sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash
}
