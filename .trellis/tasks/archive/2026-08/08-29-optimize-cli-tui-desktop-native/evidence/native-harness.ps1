$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$ExpectedSettingsPath = 'C:\Windows\ImmersiveControlPanel\SystemSettings.exe'
$NativeRoot = Join-Path $EvidenceRoot 'native'
$FixtureRoot = Join-Path $NativeRoot 'fixtures'
$Lappdata = Join-Path $NativeRoot 'lappdata'
New-Item -ItemType Directory -Force -Path $FixtureRoot, $Lappdata, (Join-Path $EvidenceRoot 'tui-renders') | Out-Null

function Write-GateLog {
  param([string]$Name, [string]$CommandLine, [datetime]$Started, [datetime]$Finished, [int]$ExitCode, [string]$Body)
  $log = Join-Path $EvidenceRoot $Name
  $text = "command: $CommandLine`r`ncwd: $Cwd`r`nstarted: $($Started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$Body`r`n-----`r`nexit=$ExitCode`r`nfinished: $($Finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
  [System.IO.File]::WriteAllText($log, $text)
}

function Quote-Arg([string]$Arg) {
  if ($Arg -match '[\s"]') { return '"' + ($Arg -replace '\\', '\\' -replace '"', '\"') + '"' }
  return $Arg
}

function Invoke-Devsweep {
  param([string]$LocalAppData, [string[]]$ArgList)
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $Exe
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.CreateNoWindow = $true
  $psi.WorkingDirectory = $Cwd
  $psi.Arguments = (($ArgList | ForEach-Object { Quote-Arg $_ }) -join ' ')
  $psi.EnvironmentVariables['LOCALAPPDATA'] = $LocalAppData
  $proc = New-Object System.Diagnostics.Process
  $proc.StartInfo = $psi
  [void]$proc.Start()
  $stdout = $proc.StandardOutput.ReadToEnd()
  $stderr = $proc.StandardError.ReadToEnd()
  $proc.WaitForExit()
  [pscustomobject]@{ ExitCode = $proc.ExitCode; StdOut = $stdout; StdErr = $stderr; Combined = ($stdout + $stderr) }
}

function Get-Consent {
  $p = Get-Process -Name consent -ErrorAction SilentlyContinue
  if ($p) { return ($p | ForEach-Object { $_.Id }) -join ',' }
  return 'none'
}

function Get-IntegrityLine {
  $groups = whoami /groups
  $line = $groups | Where-Object { $_ -match 'S-1-16-' }
  if ($line) { return ($line -join "`n") }
  return 'INTEGRITY_LINE_MISSING'
}

function Get-PeMachine([string]$Path) {
  $fs = [IO.File]::OpenRead($Path)
  try {
    $br = New-Object IO.BinaryReader $fs
    $fs.Position = 0x3C
    $pe = $br.ReadInt32()
    $fs.Position = $pe + 4
    $machine = $br.ReadUInt16()
    switch ($machine) {
      0x8664 { return 'amd64 (0x8664)' }
      0x14c { return 'i386 (0x14c)' }
      default { return ('unknown (0x{0:x})' -f $machine) }
    }
  } finally { $fs.Close() }
}

function Get-FileSha256([string]$Path) {
  return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash
}

function Get-SystemSettingsSnapshot {
  $rows = @()
  Get-CimInstance Win32_Process -Filter "Name='SystemSettings.exe'" -ErrorAction SilentlyContinue | ForEach-Object {
    $gp = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
    $rows += [pscustomobject]@{
      Pid = $_.ProcessId
      Path = $_.ExecutablePath
      CreationDate = $_.CreationDate
      StartTime = if ($gp) { $gp.StartTime } else { $null }
      HWND = if ($gp) { $gp.MainWindowHandle.ToInt64() } else { 0 }
      CommandLine = $_.CommandLine
    }
  }
  return $rows
}

function Format-SettingsSnapshot($rows, [string]$Label) {
  $sb = New-Object System.Text.StringBuilder
  [void]$sb.AppendLine("-- $Label --")
  if (-not $rows -or $rows.Count -eq 0) {
    [void]$sb.AppendLine('(no SystemSettings.exe processes)')
    return $sb.ToString()
  }
  foreach ($row in $rows) {
    [void]$sb.AppendLine(("PID={0} Path={1} StartTime={2} HWND={3} CommandLine={4}" -f $row.Pid, $row.Path, $row.StartTime, $row.HWND, $row.CommandLine))
  }
  return $sb.ToString()
}

function Test-SettingsIdentity($row) {
  if (-not $row -or [string]::IsNullOrWhiteSpace($row.Path)) { return $false }
  return [string]::Equals($row.Path, $ExpectedSettingsPath, [StringComparison]::OrdinalIgnoreCase)
}

function Close-IdentifiedSettings($row, [System.Text.StringBuilder]$Log) {
  if (-not (Test-SettingsIdentity $row)) {
    [void]$Log.AppendLine(("REFUSE_KILL unverified identity PID={0} Path={1}" -f $row.Pid, $row.Path))
    return
  }
  $gp = Get-Process -Id $row.Pid -ErrorAction SilentlyContinue
  if (-not $gp) {
    [void]$Log.AppendLine(("already exited PID={0}" -f $row.Pid))
    return
  }
  $closed = $false
  try { $closed = $gp.CloseMainWindow() } catch { $closed = $false }
  [void]$Log.AppendLine(("CloseMainWindow PID={0} returned={1} HWND={2}" -f $row.Pid, $closed, $gp.MainWindowHandle.ToInt64()))
  $deadline = (Get-Date).AddSeconds(8)
  while ((Get-Date) -lt $deadline) {
    if (-not (Get-Process -Id $row.Pid -ErrorAction SilentlyContinue)) {
      [void]$Log.AppendLine(("closed gracefully PID={0}" -f $row.Pid))
      return
    }
    Start-Sleep -Milliseconds 250
  }
  $again = Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $row.Pid) -ErrorAction SilentlyContinue
  if (-not $again) {
    [void]$Log.AppendLine(("exited before force-kill PID={0}" -f $row.Pid))
    return
  }
  if (-not [string]::Equals($again.ExecutablePath, $ExpectedSettingsPath, [StringComparison]::OrdinalIgnoreCase)) {
    [void]$Log.AppendLine(("REFUSE_FORCE_KILL path mismatch PID={0} Path={1}" -f $row.Pid, $again.ExecutablePath))
    return
  }
  Stop-Process -Id $row.Pid -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 400
  $still = Get-Process -Id $row.Pid -ErrorAction SilentlyContinue
  [void]$Log.AppendLine(("force-kill after identity re-check PID={0} still_present={1}" -f $row.Pid, [bool]$still))
}

try {
  Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class DevSweepOptimizeEnum {
  public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  public static string WindowsForPid(uint targetPid) {
    var lines = new List<string>();
    EnumWindows((hWnd, lParam) => {
      uint pid;
      GetWindowThreadProcessId(hWnd, out pid);
      if (pid == targetPid) {
        var sb = new StringBuilder(512);
        GetWindowText(hWnd, sb, sb.Capacity);
        lines.Add(string.Format("HWND={0} visible={1} title={2}", hWnd.ToInt64(), IsWindowVisible(hWnd), sb.ToString()));
      }
      return true;
    }, IntPtr.Zero);
    return string.Join("\n", lines.ToArray());
  }
}
"@ -ErrorAction Stop
  $script:EnumAvailable = $true
} catch {
  $script:EnumAvailable = $false
  $script:EnumError = $_.Exception.Message
}

function Get-WindowsForPid([int]$ProcId) {
  if (-not $script:EnumAvailable) { return "EnumWindows unavailable: $script:EnumError" }
  return [DevSweepOptimizeEnum]::WindowsForPid([uint32]$ProcId)
}

function Test-JournalRedaction([string]$Path) {
  if (-not (Test-Path $Path)) { return "MISSING $Path" }
  $text = [System.IO.File]::ReadAllText($Path)
  $hits = @()
  foreach ($token in @('ipconfig', 'ms-settings', '/flushdns', 'SysWOW64', 'cmd.exe', 'UninstallString', '"program"', '"argv"')) {
    if ($text.IndexOf($token, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $hits += $token }
  }
  if ($hits.Count -eq 0) { return 'REDACTED_OK no forbidden tokens' }
  return ('LEAK ' + ($hits -join ','))
}

function Extract-Digest([string]$Text) {
  if ($Text -match 'sha256:[0-9a-f]{64}') { return $Matches[0] }
  return $null
}

if (-not (Test-Path $Exe)) {
  Write-Host 'building debug devsweep.exe'
  Push-Location $Cwd
  cargo build --locked --bin devsweep
  Pop-Location
}

$integrity = Get-IntegrityLine
$consentBefore = Get-Consent
$pe = Get-PeMachine $Exe
$sha = Get-FileSha256 $Exe
$hostLog = @"
integrity:
$integrity
consent.exe before=$consentBefore
exe=$Exe
pe=$pe
sha256=$sha
LOCALAPPDATA override=$Lappdata
ipconfig exists=$(Test-Path 'C:\Windows\System32\ipconfig.exe')
SystemSettings exists=$(Test-Path $ExpectedSettingsPath)
NOTE: opened Settings pages are launched, never maintenance completion.
"@
Write-GateLog -Name 'native\00-host-integrity.log' -CommandLine 'whoami /groups; file hash' -Started (Get-Date) -Finished (Get-Date) -ExitCode 0 -Body $hostLog

function Invoke-List([string]$Lang, [string]$LogName) {
  $started = Get-Date
  $r = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('--language', $Lang, 'optimize', 'list')
  Write-GateLog -Name $LogName -CommandLine ("devsweep --language $Lang optimize list") -Started $started -Finished (Get-Date) -ExitCode $r.ExitCode -Body $r.Combined
  return $r
}

$listEn = Invoke-List 'en' 'native\01-list-en.log'
$listZh = Invoke-List 'zh-CN' 'native\01-list-zh.log'

$started = Get-Date
$guidance = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('optimize', 'plan', '--operation', 'guidance.drive_optimize', '--output', (Join-Path $FixtureRoot 'guidance-plan.json'))
Write-GateLog -Name 'native\02-guidance-refusal.log' -CommandLine 'devsweep optimize plan --operation guidance.drive_optimize' -Started $started -Finished (Get-Date) -ExitCode $guidance.ExitCode -Body ($guidance.Combined + "`r`nfile_exists=$(Test-Path (Join-Path $FixtureRoot 'guidance-plan.json'))")

function Invoke-ConfirmedRun([string]$Operation, [string]$LogName) {
  $plan = Join-Path $FixtureRoot ($Operation.Replace('.', '-') + '-plan.json')
  $previewOut = Join-Path $FixtureRoot ($Operation.Replace('.', '-') + '-preview.txt')
  $log = New-Object System.Text.StringBuilder
  $before = Get-SystemSettingsSnapshot
  [void]$log.AppendLine((Format-SettingsSnapshot $before 'before'))
  $consent0 = Get-Consent
  [void]$log.AppendLine("consent before=$consent0")

  $p = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('optimize', 'plan', '--operation', $Operation, '--output', $plan)
  [void]$log.AppendLine("== plan exit=$($p.ExitCode) ==")
  [void]$log.AppendLine($p.Combined)

  $v = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('--language', 'en', 'optimize', 'preview', '--plan', $plan)
  [void]$log.AppendLine("== preview en exit=$($v.ExitCode) ==")
  [void]$log.AppendLine($v.Combined)
  $digest = Extract-Digest $v.Combined
  [void]$log.AppendLine("digest=$digest")
  [System.IO.File]::WriteAllText($previewOut, $v.Combined)

  $stale = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('optimize', 'run', '--plan', $plan, '--preview-digest', 'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', '--confirm')
  [void]$log.AppendLine("== stale digest exit=$($stale.ExitCode) ==")
  [void]$log.AppendLine($stale.Combined)

  $run = Invoke-Devsweep -LocalAppData $Lappdata -ArgList @('--language', 'en', 'optimize', 'run', '--plan', $plan, '--preview-digest', $digest, '--confirm')
  [void]$log.AppendLine("== confirmed run en exit=$($run.ExitCode) ==")
  [void]$log.AppendLine($run.Combined)

  $runZh = $null
  if ($Operation -ne 'dns.flush') {
    Start-Sleep -Seconds 2
    $after = Get-SystemSettingsSnapshot
    [void]$log.AppendLine((Format-SettingsSnapshot $after 'after'))
    $new = @()
    $beforeIds = @{}
    foreach ($row in $before) { $beforeIds[$row.Pid] = $true }
    foreach ($row in $after) {
      if (-not $beforeIds.ContainsKey($row.Pid)) { $new += $row }
    }
    if ($new.Count -eq 0) { $new = $after }
    foreach ($row in $new) {
      [void]$log.AppendLine(("candidate PID={0} Path={1} HWND={2} identity_ok={3}" -f $row.Pid, $row.Path, $row.HWND, (Test-SettingsIdentity $row)))
      [void]$log.AppendLine((Get-WindowsForPid $row.Pid))
      Close-IdentifiedSettings $row $log
    }
  }

  [void]$log.AppendLine("consent after=$(Get-Consent)")
  [void]$log.AppendLine("NOTE: launched is not maintenance completion.")
  Write-GateLog -Name $LogName -CommandLine ("devsweep optimize plan/preview/run $Operation") -Started (Get-Date) -Finished (Get-Date) -ExitCode $run.ExitCode -Body $log.ToString()
  return $run
}

$dns = Invoke-ConfirmedRun 'dns.flush' 'native\03-dns-run.log'
$search = Invoke-ConfirmedRun 'settings.search' 'native\04-settings-search.log'
$storage = Invoke-ConfirmedRun 'settings.storage_recommendations' 'native\05-settings-storage.log'
$energy = Invoke-ConfirmedRun 'settings.energy_recommendations' 'native\06-settings-energy.log'

$journal = Join-Path $Lappdata 'DevSweep\audit\v1\optimize.jsonl'
$journalText = if (Test-Path $journal) { Get-Content -Raw $journal } else { 'MISSING journal' }
$redaction = Test-JournalRedaction $journal
$journalHash = if (Test-Path $journal) { Get-FileSha256 $journal } else { 'none' }
$auditLog = @"
journal=$journal
sha256=$journalHash
redaction=$redaction
residue:
$journalText
"@
Write-GateLog -Name 'native\07-audit-journal.log' -CommandLine 'read optimize.jsonl' -Started (Get-Date) -Finished (Get-Date) -ExitCode 0 -Body $auditLog

Copy-Item -Force $journal (Join-Path $NativeRoot 'optimize.jsonl') -ErrorAction SilentlyContinue

$summary = @"
list_en_exit=$($listEn.ExitCode)
list_zh_exit=$($listZh.ExitCode)
guidance_plan_exit=$($guidance.ExitCode)
dns_exit=$($dns.ExitCode)
search_exit=$($search.ExitCode)
storage_exit=$($storage.ExitCode)
energy_exit=$($energy.ExitCode)
consent_final=$(Get-Consent)
list_en_has_runs_here=$($listEn.Combined -match 'Runs here')
list_zh_has_badge=$($listZh.Combined -match '在此运行')
dns_has_succeeded=$($dns.Combined -match 'Succeeded')
search_has_launched=$($search.Combined -match 'Launched')
storage_has_launched=$($storage.Combined -match 'Launched')
energy_has_launched=$($energy.Combined -match 'Launched')
claims_completion=$($dns.Combined + $search.Combined + $storage.Combined + $energy.Combined -match 'optimization complete')
"@
Write-GateLog -Name 'native\08-summary.log' -CommandLine 'native presentation summary' -Started (Get-Date) -Finished (Get-Date) -ExitCode 0 -Body $summary
Write-Host $summary
