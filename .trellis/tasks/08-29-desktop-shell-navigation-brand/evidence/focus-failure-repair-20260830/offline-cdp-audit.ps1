param(
    [Parameter(Mandatory = $true)]
    [string]$CdpLog,
    [Parameter(Mandatory = $true)]
    [string]$VerificationJson
)

$ErrorActionPreference = "Stop"
$messages = @(
    Get-Content -LiteralPath $CdpLog |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        ForEach-Object {
            $envelope = $_ | ConvertFrom-Json -Depth 100
            $envelope.payload | ConvertFrom-Json -Depth 100
        }
)
$verification = Get-Content -Raw -LiteralPath $VerificationJson | ConvertFrom-Json -Depth 100

$runtimeExceptions = @($messages | Where-Object method -eq "Runtime.exceptionThrown")
$consoleErrors = @($messages | Where-Object {
    $_.method -eq "Runtime.consoleAPICalled" -and $_.params.type -eq "error"
})
$logErrors = @($messages | Where-Object {
    $_.method -eq "Log.entryAdded" -and $_.params.entry.level -eq "error"
})
$knownFaviconErrors = @($logErrors | Where-Object {
    $_.params.entry.source -eq "network" -and
    $_.params.entry.url -eq "http://127.0.0.1:4180/favicon.ico" -and
    $_.params.entry.text -eq "Failed to load resource: the server responded with a status of 404 (Not Found)"
})
$unexpectedLogErrors = @($logErrors | Where-Object {
    $_.params.entry.source -ne "network" -or
    $_.params.entry.url -ne "http://127.0.0.1:4180/favicon.ico" -or
    $_.params.entry.text -ne "Failed to load resource: the server responded with a status of 404 (Not Found)"
})
$stateSnapshots = @($messages | Where-Object {
    $null -ne $_.result.result.value -and
    $null -ne $_.result.result.value.PSObject.Properties["unhandled"] -and
    $null -ne $_.result.result.value.PSObject.Properties["windowErrors"]
} | ForEach-Object { $_.result.result.value })
$windowUnhandled = @($stateSnapshots | ForEach-Object { @($_.unhandled) })
$windowErrors = @($stateSnapshots | ForEach-Object { @($_.windowErrors) })

$result = [ordered]@{
    verification_state = $verification.state
    verification_failure = $verification.failure.message
    runtime_exception_count = $runtimeExceptions.Count
    console_error_count = $consoleErrors.Count
    log_error_count = $logErrors.Count
    known_vite_favicon_404_count = $knownFaviconErrors.Count
    unexpected_log_error_count = $unexpectedLogErrors.Count
    state_snapshot_count = $stateSnapshots.Count
    window_unhandled_count = $windowUnhandled.Count
    window_error_count = $windowErrors.Count
    runtime_exceptions = $runtimeExceptions
    console_errors = $consoleErrors
    log_errors = $logErrors
    window_unhandled = $windowUnhandled
    window_errors = $windowErrors
}
$result | ConvertTo-Json -Depth 100

if ($verification.state -ne "failed" -or $verification.failure.message -ne "CDP observed exception/error console output") { exit 10 }
if ($runtimeExceptions.Count -ne 0 -or $consoleErrors.Count -ne 0) { exit 11 }
if ($logErrors.Count -ne 1 -or $knownFaviconErrors.Count -ne 1 -or $unexpectedLogErrors.Count -ne 0) { exit 12 }
if ($stateSnapshots.Count -lt 3 -or $windowUnhandled.Count -ne 0 -or $windowErrors.Count -ne 0) { exit 13 }
exit 0
