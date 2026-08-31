$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$NativeRoot = Join-Path $EvidenceRoot 'native-independent'
$FixtureRoot = Join-Path $NativeRoot 'fixtures'
$Lappdata = Join-Path $NativeRoot 'lappdata'
$ExpectedIpconfig = 'C:\Windows\System32\ipconfig.exe'
$SampleFile = Join-Path $EvidenceRoot 'independent-dns-process-samples.txt'
New-Item -ItemType Directory -Force -Path $FixtureRoot, $Lappdata | Out-Null
[System.IO.File]::WriteAllText($SampleFile, '')

Add-Type -TypeDefinition @"
using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
public static class DevSweepIpconfigProbe {
  [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
  static extern bool QueryFullProcessImageName(IntPtr hProcess, int flags, StringBuilder name, ref int size);
  public static volatile bool Stop;
  public static int Hits;
  public static void Run(string path) {
    var seen = new System.Collections.Generic.HashSet<int>();
    while (!Stop) {
      Process[] procs = Process.GetProcessesByName("ipconfig");
      foreach (var p in procs) {
        try {
          if (!seen.Add(p.Id)) continue;
          Interlocked.Increment(ref Hits);
          string image = "";
          try {
            var sb = new StringBuilder(1024);
            int size = sb.Capacity;
            if (QueryFullProcessImageName(p.Handle, 0, sb, ref size)) image = sb.ToString();
          } catch {}
          File.AppendAllText(path, DateTime.Now.ToString("o") + " PID=" + p.Id + " Image=" + image + " Start=" + p.StartTime.ToString("o") + Environment.NewLine);
        } catch {}
        try { p.Dispose(); } catch {}
      }
    }
  }
}
"@

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
$plan = Join-Path $FixtureRoot 'dns-flush-plan.json'
$p = Invoke-Devsweep -ArgList @('optimize', 'plan', '--operation', 'dns.flush', '--output', $plan)
$v = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'preview', '--plan', $plan)
$digest = Extract-Digest $v.Combined
$stale = Invoke-Devsweep -ArgList @('optimize', 'run', '--plan', $plan, '--preview-digest', 'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', '--confirm')

[DevSweepIpconfigProbe]::Stop = $false
[DevSweepIpconfigProbe]::Hits = 0
$thread = New-Object System.Threading.Thread ([System.Threading.ThreadStart]{ [DevSweepIpconfigProbe]::Run($SampleFile) })
$thread.IsBackground = $true
$thread.Start()
Start-Sleep -Milliseconds 80
$consentBefore = Get-Consent
$integrity = (whoami /groups | Where-Object { $_ -match 'S-1-16-' }) -join "`n"
$run = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'run', '--plan', $plan, '--preview-digest', $digest, '--confirm')
$consentAfter = Get-Consent
Start-Sleep -Milliseconds 200
[DevSweepIpconfigProbe]::Stop = $true
$thread.Join(2000) | Out-Null

$samples = if (Test-Path $SampleFile) { Get-Content -Raw $SampleFile } else { '' }
$pathHit = $samples.IndexOf($ExpectedIpconfig, [StringComparison]::OrdinalIgnoreCase) -ge 0
$pidHit = $samples -match 'PID='
$uac = ($consentBefore -ne 'none') -or ($consentAfter -ne 'none')
$journal = Join-Path $Lappdata 'DevSweep\audit\v1\optimize.jsonl'
$journalText = if (Test-Path $journal) { Get-Content -Raw $journal } else { 'MISSING' }
$redactionHits = @()
foreach ($token in @('ipconfig', 'ms-settings', '/flushdns', '"program"', '"argv"')) {
  if ($journalText.IndexOf($token, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $redactionHits += $token }
}
$redaction = if ($redactionHits.Count -eq 0) { 'REDACTED_OK' } else { 'LEAK ' + ($redactionHits -join ',') }

$probeHits = [DevSweepIpconfigProbe]::Hits
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
probe_hits=$probeHits
ipconfig_path_observed=$pathHit
ipconfig_pid_observed=$pidHit
expected_path=$ExpectedIpconfig
ipconfig_exists=$(Test-Path $ExpectedIpconfig)
process_samples:
$samples
journal_redaction=$redaction
NOTE: presentation journal redacts program/argv by contract. Live probe is GetProcessesByName spin; argv is owned by core runner (separate program/argv, not reconstructed here).
"@

$finished = Get-Date
$logPath = Join-Path $EvidenceRoot 'independent-native-dns-process-round2.log'
$text = "command: independent dns.flush with C# GetProcessesByName spin probe`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$exit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
[System.IO.File]::WriteAllText($logPath, $text)
Write-Host ("WROTE $logPath exit=$exit hits=$probeHits pathHit=$pathHit")
exit $exit
