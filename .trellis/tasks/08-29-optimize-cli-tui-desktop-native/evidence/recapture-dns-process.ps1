$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$NativeRoot = Join-Path $EvidenceRoot 'native-independent'
$FixtureRoot = Join-Path $NativeRoot 'fixtures'
$Lappdata = Join-Path $NativeRoot 'lappdata'
$ExpectedIpconfig = 'C:\Windows\System32\ipconfig.exe'
New-Item -ItemType Directory -Force -Path $FixtureRoot, $Lappdata | Out-Null

function Quote-Arg([string]$Arg) {
  if ($Arg -match '[\s"]') { return '"' + ($Arg -replace '\\', '\\' -replace '"', '\"') + '"' }
  return $Arg
}

function Invoke-Devsweep {
  param([string[]]$ArgList)
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $Exe
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.CreateNoWindow = $true
  $psi.WorkingDirectory = $Cwd
  $psi.Arguments = (($ArgList | ForEach-Object { Quote-Arg $_ }) -join ' ')
  $psi.EnvironmentVariables['LOCALAPPDATA'] = $Lappdata
  $proc = New-Object System.Diagnostics.Process
  $proc.StartInfo = $psi
  [void]$proc.Start()
  $stdout = $proc.StandardOutput.ReadToEnd()
  $stderr = $proc.StandardError.ReadToEnd()
  $proc.WaitForExit()
  [pscustomobject]@{ ExitCode = $proc.ExitCode; Combined = ($stdout + $stderr) }
}

function Extract-Digest([string]$Text) {
  if ($Text -match 'sha256:[0-9a-f]{64}') { return $Matches[0] }
  return $null
}

function Get-Consent {
  $p = Get-Process -Name consent -ErrorAction SilentlyContinue
  if ($p) { return ($p | ForEach-Object { $_.Id }) -join ',' }
  return 'none'
}

$started = Get-Date
$samples = [System.Collections.Concurrent.ConcurrentBag[string]]::new()

$plan = Join-Path $FixtureRoot 'dns-flush-plan.json'
$p = Invoke-Devsweep -ArgList @('optimize', 'plan', '--operation', 'dns.flush', '--output', $plan)
$v = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'preview', '--plan', $plan)
$digest = Extract-Digest $v.Combined
$stale = Invoke-Devsweep -ArgList @('optimize', 'run', '--plan', $plan, '--preview-digest', 'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', '--confirm')
$consentBefore = Get-Consent

$runspace = [runspacefactory]::CreateRunspace()
$runspace.Open()
$runspace.SessionStateProxy.SetVariable('samples', $samples)
$ps = [powershell]::Create()
$ps.Runspace = $runspace
[void]$ps.AddScript({
  $deadline = (Get-Date).AddSeconds(12)
  while ((Get-Date) -lt $deadline) {
    Get-CimInstance Win32_Process -Filter "Name='ipconfig.exe'" -ErrorAction SilentlyContinue | ForEach-Object {
      $samples.Add(("CIM PID={0} Path={1} CommandLine={2} CreationDate={3}" -f $_.ProcessId, $_.ExecutablePath, $_.CommandLine, $_.CreationDate))
    }
    Get-Process -Name ipconfig -ErrorAction SilentlyContinue | ForEach-Object {
      $samples.Add(("GP PID={0} Path={1} StartTime={2}" -f $_.Id, $_.Path, $_.StartTime))
    }
    Start-Sleep -Milliseconds 10
  }
})
$handle = $ps.BeginInvoke()
Start-Sleep -Milliseconds 50
$run = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'run', '--plan', $plan, '--preview-digest', $digest, '--confirm')
$consentAfter = Get-Consent
Start-Sleep -Milliseconds 200
$ps.Stop() | Out-Null
try { $ps.EndInvoke($handle) | Out-Null } catch {}
$ps.Dispose()
$runspace.Close()

$uniqueSamples = $samples.ToArray() | Select-Object -Unique
$pathHit = ($uniqueSamples | Where-Object { $_ -match [regex]::Escape($ExpectedIpconfig) }).Count -gt 0
$argvHit = ($uniqueSamples | Where-Object { $_ -match '/flushdns' }).Count -gt 0
$uac = ($consentBefore -ne 'none') -or ($consentAfter -ne 'none')
$integrity = (whoami /groups | Where-Object { $_ -match 'S-1-16-' }) -join "`n"
$journal = Join-Path $Lappdata 'DevSweep\audit\v1\optimize.jsonl'
$journalText = if (Test-Path $journal) { Get-Content -Raw $journal } else { 'MISSING' }
$redactionHits = @()
foreach ($token in @('ipconfig', 'ms-settings', '/flushdns', '"program"', '"argv"')) {
  if ($journalText.IndexOf($token, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $redactionHits += $token }
}
$redaction = if ($redactionHits.Count -eq 0) { 'REDACTED_OK' } else { 'LEAK ' + ($redactionHits -join ',') }

$exit = 0
if ($run.ExitCode -ne 0 -or $stale.ExitCode -eq 0 -or $uac -or -not ($run.Combined -match 'Succeeded')) { $exit = 1 }

$body = @"
integrity:
$integrity
consent_before=$consentBefore
consent_after=$consentAfter
uac_observed=$uac
plan_exit=$($p.ExitCode)
preview_exit=$($v.ExitCode)
preview:
$($v.Combined)
digest=$digest
stale_exit=$($stale.ExitCode)
stale:
$($stale.Combined)
run_exit=$($run.ExitCode)
run:
$($run.Combined)
ipconfig_path_observed=$pathHit
ipconfig_argv_observed=$argvHit
expected_path=$ExpectedIpconfig
process_samples_count=$($uniqueSamples.Count)
process_samples:
$($uniqueSamples -join "`n")
journal_redaction=$redaction
NOTE: presentation journal redacts program/argv; live CIM/Get-Process samples are the process-identity evidence.
"@

$finished = Get-Date
$logPath = Join-Path $EvidenceRoot 'independent-native-dns-process.log'
$text = "command: independent dns.flush with concurrent ipconfig process poll`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$exit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
[System.IO.File]::WriteAllText($logPath, $text)
Write-Host ("WROTE $logPath exit=$exit pathHit=$pathHit argvHit=$argvHit samples=$($uniqueSamples.Count)")
exit $exit
