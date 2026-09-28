param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('desktop-web-check', 'desktop-test', 'ci', 'desktop-build')]
    [string]$Gate
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../../..')).Path
$evidence = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location -LiteralPath $repo
$recordPath = Join-Path $evidence ($Gate + '.json')
$logPath = Join-Path $evidence ($Gate + '.log')
$record = [ordered]@{
    command = if ($Gate -in @('desktop-web-check', 'desktop-build')) { "mise exec node@22 -- just $Gate" } else { "just $Gate" }
    startedAt = (Get-Date).ToUniversalTime().ToString('o')
    runnerPid = $PID
    result = 'RUNNING'
}
$record | ConvertTo-Json | Set-Content -LiteralPath $recordPath -Encoding utf8
try {
    $ErrorActionPreference = 'Continue'
    if ($Gate -in @('desktop-web-check', 'desktop-build')) {
        & mise exec node@22 -- just $Gate 2>&1 | Tee-Object -FilePath $logPath
    } else {
        & just $Gate 2>&1 | Tee-Object -FilePath $logPath
    }
    $gateExit = $LASTEXITCODE
    $record.exitCode = $gateExit
    $record.result = if ($gateExit -eq 0) { 'PASS' } else { 'FAIL' }
} catch {
    $record.result = 'ERROR'
    $record.error = $_.Exception.Message
    $gateExit = 1
} finally {
    $record.finishedAt = (Get-Date).ToUniversalTime().ToString('o')
    $record | ConvertTo-Json | Set-Content -LiteralPath $recordPath -Encoding utf8
}
exit $gateExit
