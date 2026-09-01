$ErrorActionPreference = "Stop"
$runRoot = $PSScriptRoot
$repository = (Resolve-Path -LiteralPath (Join-Path $runRoot "../../../../..")).Path
$manifestPath = Join-Path $runRoot "evidence-manifest.json"
$native = Join-Path $runRoot "native-release"

$artifactPaths = @(
    "target/release/devsweep-desktop.exe",
    "target/debug/devsweep-desktop.exe",
    "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"
)
$artifacts = foreach ($relative in $artifactPaths) {
    $path = Join-Path $repository $relative
    $item = Get-Item -LiteralPath $path
    [pscustomobject][ordered]@{ path = $relative; bytes = $item.Length; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash }
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
    ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-artifact-rebind-92c-20260830/verification.md",
    ".trellis/tasks/08-29-desktop-shell-navigation-brand/evidence/final-artifact-rebind-92c-run2-20260830/verification.md"
)
$reports = foreach ($relative in $reportPaths) {
    $path = Join-Path $repository $relative
    $item = Get-Item -LiteralPath $path
    [pscustomobject][ordered]@{ path = $relative; bytes = $item.Length; lines = @(Get-Content -LiteralPath $path).Count; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash }
}

$outer = Get-Content -LiteralPath (Join-Path $runRoot "outer-exit.json") -Raw | ConvertFrom-Json -Depth 100
$transcriptPath = Join-Path $native "02-native-release-rebind.log"
$cdpPath = Join-Path $native "cdp-messages.jsonl"
$uiaPath = Join-Path $native "shell-tray-wnd-uia-raw.json"
$manifest = [ordered]@{
    generated_at = (Get-Date).ToString("o")
    decision = "failed_final_run3_no_run4"
    tool_observed_exit_code = 1
    helper_exit_code = $outer.helper_exit
    transcript = [ordered]@{ path = "native-release/02-native-release-rebind.log"; lines = @(Get-Content -LiteralPath $transcriptPath).Count; bytes = (Get-Item $transcriptPath).Length; sha256 = (Get-FileHash -Algorithm SHA256 $transcriptPath).Hash }
    cdp = [ordered]@{ path = "native-release/cdp-messages.jsonl"; lines = @(Get-Content -LiteralPath $cdpPath).Count; bytes = (Get-Item $cdpPath).Length; sha256 = (Get-FileHash -Algorithm SHA256 $cdpPath).Hash }
    uia = if (Test-Path -LiteralPath $uiaPath) { [ordered]@{ exists = $true; path = "native-release/shell-tray-wnd-uia-raw.json"; bytes = (Get-Item $uiaPath).Length; lines = @(Get-Content $uiaPath).Count; sha256 = (Get-FileHash -Algorithm SHA256 $uiaPath).Hash } } else { [ordered]@{ exists = $false; reason = "run3 stopped at first-close residue gate before UIA" } }
    missing_expected_outputs = @(
        "native-release/shell-tray-wnd-uia-raw.json",
        "native-release/screenshots/release-restart-after-back.png",
        "native-release/screenshots/release-taskbar-bound-*.png"
    )
    artifacts = @($artifacts)
    files = @($files)
    external_reports = @($reports)
}
$manifest | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $manifestPath -Encoding utf8
$item = Get-Item $manifestPath
[pscustomobject]@{ path = $item.FullName; bytes = $item.Length; lines = @(Get-Content $item.FullName).Count; sha256 = (Get-FileHash -Algorithm SHA256 $item.FullName).Hash }
