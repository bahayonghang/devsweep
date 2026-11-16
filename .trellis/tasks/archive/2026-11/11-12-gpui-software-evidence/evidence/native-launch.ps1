param(
    [ValidateSet('en', 'zh-CN')][string]$Language = 'en',
    [ValidateSet('dark', 'light')][string]$Theme = 'dark',
    [string]$Inventory = ''
)
$ErrorActionPreference = 'Stop'
$repoRoot = (& git -C $PSScriptRoot rev-parse --show-toplevel)
if ($LASTEXITCODE -ne 0) { throw 'Cannot locate the repository root' }
$repoRoot = $repoRoot.Trim()
$binary = Join-Path $repoRoot 'desktop-gpui/dist/goal-gpui-release/release/devsweep-gpui-spike.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Missing binary: $binary" }
$caseRoot = Join-Path $PSScriptRoot ('runtime/' + $Language + '-' + $Theme + '-' + [guid]::NewGuid().ToString('N'))
$settings = Join-Path $caseRoot 'local/DevSweep/settings'
$roaming = Join-Path $caseRoot 'roaming'
New-Item -ItemType Directory -Force -Path $settings, $roaming | Out-Null
$encoding = [System.Text.UTF8Encoding]::new($false)
$presentation = @{ schema_version = 1; language = $Language }
$preferences = @{
    schema_version = 2; theme = $Theme; font = @{ kind = 'system' }
    text_scale_percent = 100; motion = 'reduced'; planet_fps = 30
    status_interval_seconds = 2; status_process_limit = 15; hud_interval_seconds = 2
}
[System.IO.File]::WriteAllText((Join-Path $settings 'presentation-v1.json'), ($presentation | ConvertTo-Json), $encoding)
[System.IO.File]::WriteAllText((Join-Path $settings 'desktop-preferences-v2.json'), ($preferences | ConvertTo-Json), $encoding)
$env:LOCALAPPDATA = Join-Path $caseRoot 'local'
$env:APPDATA = $roaming
if ($Inventory) {
    $env:DEVSWEEP_SOFTWARE_INVENTORY = (Resolve-Path -LiteralPath $Inventory).Path
} else {
    Remove-Item Env:DEVSWEEP_SOFTWARE_INVENTORY -ErrorAction SilentlyContinue
}
$process = Start-Process -FilePath $binary -WorkingDirectory $repoRoot -PassThru
[pscustomobject]@{
    process_id = $process.Id
    language = $Language
    theme = $Theme
    runtime = $caseRoot
    binary_sha256 = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
} | ConvertTo-Json
