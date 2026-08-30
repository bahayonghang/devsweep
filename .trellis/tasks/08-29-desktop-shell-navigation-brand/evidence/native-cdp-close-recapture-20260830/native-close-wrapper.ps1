param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$RunRoot,
    [Parameter(Mandatory = $true)]
    [string]$PwshPath,
    [int]$FirstPort = 9401,
    [int]$RestartPort = 9402
)

$ErrorActionPreference = "Stop"
$repository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$run = (Resolve-Path -LiteralPath $RunRoot).Path
$pwsh = (Resolve-Path -LiteralPath $PwshPath).Path
$helper = Join-Path $run "native-close-recapture.ps1"
$nativeRoot = Join-Path $run "native-release"
$outerExitPath = Join-Path $run "outer-exit.json"

if ($PSVersionTable.PSVersion.Major -lt 7) { throw "native-close wrapper itself must execute under PowerShell 7" }
if ((Get-Process -Id $PID).Path -ne $pwsh) { throw "native-close wrapper executable path mismatch" }
if (Test-Path -LiteralPath $outerExitPath) { throw "native-close outer-exit already exists; refusing overwrite" }
if (-not (Test-Path -LiteralPath $helper -PathType Leaf)) { throw "native-close helper missing" }

Write-Output ("NATIVE_CLOSE_WRAPPER_START={0}" -f (Get-Date).ToString("o"))
Write-Output ("WRAPPER_PWSH_PATH={0}" -f $pwsh)
Write-Output ("WRAPPER_PWSH_VERSION={0}" -f $PSVersionTable.PSVersion.ToString())
Write-Output ("WRAPPER_PID={0}" -f $PID)
Write-Output ("WRAPPER_COMMANDLINE={0}" -f (Get-CimInstance Win32_Process -Filter "ProcessId = $PID").CommandLine)
Write-Output ("WRAPPER_SHA256={0}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash)
Write-Output ("HELPER_SHA256={0}" -f (Get-FileHash -Algorithm SHA256 -LiteralPath $helper).Hash)

& $pwsh -NoProfile -ExecutionPolicy Bypass -File $helper `
    -RepositoryRoot $repository `
    -EvidenceRoot $nativeRoot `
    -FirstPort $FirstPort `
    -RestartPort $RestartPort `
    -ExpectedHash "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20" `
    -PwshPath $pwsh `
    -WrapperPath $PSCommandPath
$helperExit = $LASTEXITCODE
$completedAt = (Get-Date).ToString("o")

function Get-FileAudit {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [ordered]@{ path = $Path; exists = $false }
    }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{
        path = $item.FullName
        exists = $true
        bytes = $item.Length
        lines = @(Get-Content -LiteralPath $item.FullName).Count
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash
    }
}

$outer = [ordered]@{
    completed_at = $completedAt
    helper_exit = $helperExit
    wrapper_pid = $PID
    wrapper_pwsh_path = $pwsh
    wrapper_pwsh_version = $PSVersionTable.PSVersion.ToString()
    wrapper_command_line = (Get-CimInstance Win32_Process -Filter "ProcessId = $PID").CommandLine
    wrapper = Get-FileAudit -Path $PSCommandPath
    helper = Get-FileAudit -Path $helper
    transcript = Get-FileAudit -Path (Join-Path $nativeRoot "02-native-release-rebind.log")
    cdp = Get-FileAudit -Path (Join-Path $nativeRoot "cdp-messages.jsonl")
    uia = Get-FileAudit -Path (Join-Path $nativeRoot "shell-tray-wnd-uia-raw.json")
    verification = Get-FileAudit -Path (Join-Path $nativeRoot "verification.json")
}
$outer | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $outerExitPath -Encoding utf8
Write-Output ("OUTER_EXIT_JSON={0}" -f $outerExitPath)
Write-Output ("OUTER_HELPER_EXIT={0}" -f $helperExit)
Write-Output ("NATIVE_CLOSE_WRAPPER_FINISHED={0}" -f $completedAt)
exit $helperExit
