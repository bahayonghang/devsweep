param(
  [Parameter(Mandatory = $true)][string]$Name,
  [Parameter(Mandatory = $true)][string]$CommandLine
)

$ErrorActionPreference = 'Continue'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$log = Join-Path $EvidenceRoot $Name
$started = Get-Date
Set-Location $Cwd

$stdoutFile = Join-Path $env:TEMP ("devsweep-ind-" + [guid]::NewGuid().ToString() + ".out")
$stderrFile = Join-Path $env:TEMP ("devsweep-ind-" + [guid]::NewGuid().ToString() + ".err")
try {
  cmd.exe /c $CommandLine > $stdoutFile 2> $stderrFile
  $exit = $LASTEXITCODE
} catch {
  $exit = 1
  [System.IO.File]::AppendAllText($stderrFile, "`r`nCAPTURE_EXCEPTION: $($_.Exception.Message)`r`n")
}
$finished = Get-Date
$stdout = if (Test-Path $stdoutFile) { [System.IO.File]::ReadAllText($stdoutFile) } else { '' }
$stderr = if (Test-Path $stderrFile) { [System.IO.File]::ReadAllText($stderrFile) } else { '' }
$body = $stdout
if (-not [string]::IsNullOrWhiteSpace($stderr)) {
  $body = $body + "`r`n----- stderr -----`r`n" + $stderr
}
$text = "command: $CommandLine`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$exit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
[System.IO.File]::WriteAllText($log, $text)
Write-Host ("WROTE " + $log + " exit=" + $exit)
exit $exit
