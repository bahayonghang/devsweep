$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-catalog-execution\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$ExpectedSettingsPath = 'C:\Windows\ImmersiveControlPanel\SystemSettings.exe'
$FixtureRoot = Join-Path $EvidenceRoot 'independent-fixtures'
New-Item -ItemType Directory -Force -Path $FixtureRoot | Out-Null

function Write-GateLog {
  param([string]$Name, [string]$CommandLine, [datetime]$Started, [datetime]$Finished, [int]$ExitCode, [string]$Body)
  $log = Join-Path $EvidenceRoot "independent-$Name.log"
  $text = "command: $CommandLine`r`ncwd: $Cwd`r`nstarted: $($Started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$Body`r`n-----`r`nexit=$ExitCode`r`nfinished: $($Finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
  [System.IO.File]::WriteAllText($log, $text)
  Write-Host "WROTE $log exit=$ExitCode"
}

function Quote-Arg([string]$Arg) {
  if ($Arg -match '[\s"]') {
    return '"' + ($Arg -replace '\\', '\\' -replace '"', '\"') + '"'
  }
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
  [pscustomobject]@{
    ExitCode = $proc.ExitCode
    StdOut   = $stdout
    StdErr   = $stderr
    Combined = ($stdout + $stderr)
  }
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
  } finally {
    $fs.Close()
  }
}

function Get-SystemSettingsSnapshot {
  $rows = @()
  Get-CimInstance Win32_Process -Filter "Name='SystemSettings.exe'" -ErrorAction SilentlyContinue | ForEach-Object {
    $gp = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
    $rows += [pscustomobject]@{
      Pid           = $_.ProcessId
      Path          = $_.ExecutablePath
      CreationDate  = $_.CreationDate
      StartTime     = if ($gp) { $gp.StartTime } else { $null }
      HWND          = if ($gp) { $gp.MainWindowHandle.ToInt64() } else { 0 }
      CommandLine   = $_.CommandLine
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
    [void]$sb.AppendLine(("PID={0} Path={1} StartTime={2} CreationDate={3} HWND={4} CommandLine={5}" -f $row.Pid, $row.Path, $row.StartTime, $row.CreationDate, $row.HWND, $row.CommandLine))
  }
  return $sb.ToString()
}

function Test-SettingsIdentity($row) {
  if (-not $row) { return $false }
  if ([string]::IsNullOrWhiteSpace($row.Path)) { return $false }
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
public static class DevSweepWinEnum {
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
  return [DevSweepWinEnum]::WindowsForPid([uint32]$ProcId)
}

function Test-JournalRedaction([string]$Path) {
  if (-not (Test-Path $Path)) { return "MISSING $Path" }
  $text = [System.IO.File]::ReadAllText($Path)
  $hits = @()
  foreach ($token in @('ipconfig', 'ms-settings', '/flushdns', 'SysWOW64', 'cmd.exe', 'UninstallString', 'program', 'argv', 'C:\\Windows')) {
    if ($text.IndexOf($token, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
      $hits += $token
    }
  }
  if ($hits.Count -eq 0) { return 'REDACTED_OK no forbidden tokens' }
  return ('REDACTION_LEAK ' + ($hits -join ','))
}

# -------------------- 1. Host integrity + leftover close --------------------
$hostStarted = Get-Date
$hostBody = New-Object System.Text.StringBuilder
[void]$hostBody.AppendLine(("whoami: {0}" -f (whoami)))
[void]$hostBody.AppendLine('integrity lines:')
[void]$hostBody.AppendLine((Get-IntegrityLine))
[void]$hostBody.AppendLine(("consent.exe before: {0}" -f (Get-Consent)))
$cv = Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
[void]$hostBody.AppendLine(("CurrentBuild={0} UBR={1} DisplayVersion={2} ProductName={3}" -f $cv.CurrentBuild, $cv.UBR, $cv.DisplayVersion, $cv.ProductName))
[void]$hostBody.AppendLine(("[Environment]::OSVersion={0} (manifest-sensitive; product uses RtlGetVersion, not this)" -f [Environment]::OSVersion.VersionString))
[void]$hostBody.AppendLine(("PROCESSOR_ARCHITECTURE={0}" -f $env:PROCESSOR_ARCHITECTURE))
[void]$hostBody.AppendLine(("Is64BitProcess={0}" -f [Environment]::Is64BitProcess))
[void]$hostBody.AppendLine(("EnumWindows available={0}" -f $script:EnumAvailable))

if (-not (Test-Path $Exe)) {
  [void]$hostBody.AppendLine('debug exe missing; building')
  Push-Location $Cwd
  & cargo build -p devsweep-cli --offline 2>&1 | Out-String | ForEach-Object { [void]$hostBody.AppendLine($_) }
  Pop-Location
}
$hash = (Get-FileHash -Algorithm SHA256 $Exe).Hash
[void]$hostBody.AppendLine(("x64 exe: {0} sha256={1} size={2} machine={3}" -f $Exe, $hash, (Get-Item $Exe).Length, (Get-PeMachine $Exe)))
[void]$hostBody.AppendLine(("ipconfig System32 exists={0}" -f (Test-Path 'C:\Windows\System32\ipconfig.exe')))
[void]$hostBody.AppendLine(("ipconfig Sysnative exists={0} (expected false from native x64 PowerShell)" -f (Test-Path 'C:\Windows\Sysnative\ipconfig.exe')))
[void]$hostBody.AppendLine(("ipconfig SysWOW64 exists={0}" -f (Test-Path 'C:\Windows\SysWOW64\ipconfig.exe')))

[void]$hostBody.AppendLine('')
[void]$hostBody.AppendLine( (Format-SettingsSnapshot (Get-SystemSettingsSnapshot) 'leftover SystemSettings BEFORE close') )
$closeLog = New-Object System.Text.StringBuilder
foreach ($row in @(Get-SystemSettingsSnapshot)) {
  Close-IdentifiedSettings $row $closeLog
}
[void]$hostBody.AppendLine($closeLog.ToString())
Start-Sleep -Milliseconds 800
[void]$hostBody.AppendLine( (Format-SettingsSnapshot (Get-SystemSettingsSnapshot) 'SystemSettings AFTER leftover close') )
[void]$hostBody.AppendLine(("consent.exe after leftover close: {0}" -f (Get-Consent)))
$hostFinished = Get-Date
Write-GateLog -Name 'host-integrity' -CommandLine 'host inventory (whoami, integrity, OS build, binary hash, leftover SystemSettings close)' -Started $hostStarted -Finished $hostFinished -ExitCode 0 -Body $hostBody.ToString()

# -------------------- 2. DNS plan/preview/run --------------------
$dnsStarted = Get-Date
$dnsBody = New-Object System.Text.StringBuilder
$dnsLapp = Join-Path $FixtureRoot 'lappdata-dns'
New-Item -ItemType Directory -Force -Path $dnsLapp | Out-Null
$dnsPlan = Join-Path $FixtureRoot 'dns-plan.json'
$dnsPreview = Join-Path $FixtureRoot 'dns-preview.json'
[void]$dnsBody.AppendLine(("LOCALAPPDATA={0}" -f $dnsLapp))
[void]$dnsBody.AppendLine(("resolved program candidate: C:\Windows\System32\ipconfig.exe exists={0}" -f (Test-Path 'C:\Windows\System32\ipconfig.exe')))
[void]$dnsBody.AppendLine('argv identity from catalogue/adapter: exactly [/flushdns]; program/argv kept separate; no shell')
[void]$dnsBody.AppendLine(("integrity={0}" -f ((Get-IntegrityLine) -replace '\s+', ' ').Trim()))
[void]$dnsBody.AppendLine(("consent before dns: {0}" -f (Get-Consent)))

$plan = Invoke-Devsweep -LocalAppData $dnsLapp -ArgList @('optimize','plan','--operation','dns.flush','--output',$dnsPlan)
[void]$dnsBody.AppendLine('== plan ==')
[void]$dnsBody.AppendLine($plan.Combined)
[void]$dnsBody.AppendLine(("plan exit={0} file_exists={1}" -f $plan.ExitCode, (Test-Path $dnsPlan)))
if (Test-Path $dnsPlan) { [void]$dnsBody.AppendLine((Get-Content -Raw $dnsPlan)) }

$preview = Invoke-Devsweep -LocalAppData $dnsLapp -ArgList @('optimize','preview','--plan',$dnsPlan,'--format','json','--output',$dnsPreview)
[void]$dnsBody.AppendLine('== preview ==')
[void]$dnsBody.AppendLine($preview.Combined)
[void]$dnsBody.AppendLine(("preview exit={0}" -f $preview.ExitCode))
$digest = $null
if (Test-Path $dnsPreview) {
  $previewJson = Get-Content -Raw $dnsPreview
  [void]$dnsBody.AppendLine($previewJson)
  try { $digest = (ConvertFrom-Json $previewJson).data.digest } catch { $digest = $null }
}
[void]$dnsBody.AppendLine(("digest={0}" -f $digest))

$noConfirm = Invoke-Devsweep -LocalAppData $dnsLapp -ArgList @('optimize','run','--plan',$dnsPlan,'--preview-digest',$digest)
[void]$dnsBody.AppendLine('== run no-confirm ==')
[void]$dnsBody.AppendLine($noConfirm.Combined)
[void]$dnsBody.AppendLine(("no-confirm exit={0} (expect 2)" -f $noConfirm.ExitCode))

$wrong = Invoke-Devsweep -LocalAppData $dnsLapp -ArgList @('optimize','run','--plan',$dnsPlan,'--preview-digest','sha256:0000000000000000000000000000000000000000000000000000000000000000','--confirm')
[void]$dnsBody.AppendLine('== run wrong-digest ==')
[void]$dnsBody.AppendLine($wrong.Combined)
[void]$dnsBody.AppendLine(("wrong-digest exit={0} (expect 3)" -f $wrong.ExitCode))

[void]$dnsBody.AppendLine('== run CONFIRMED (real ipconfig /flushdns) ==')
$confirmed = Invoke-Devsweep -LocalAppData $dnsLapp -ArgList @('optimize','run','--plan',$dnsPlan,'--preview-digest',$digest,'--confirm')
[void]$dnsBody.AppendLine($confirmed.Combined)
[void]$dnsBody.AppendLine(("confirmed-run exit={0} (expect 0)" -f $confirmed.ExitCode))

$journal = Join-Path $dnsLapp 'DevSweep\audit\v1\optimize.jsonl'
[void]$dnsBody.AppendLine(("journal path={0} exists={1}" -f $journal, (Test-Path $journal)))
if (Test-Path $journal) {
  [void]$dnsBody.AppendLine('-- journal --')
  [void]$dnsBody.AppendLine((Get-Content -Raw $journal))
  [void]$dnsBody.AppendLine((Test-JournalRedaction $journal))
}
[void]$dnsBody.AppendLine(("consent after dns: {0}" -f (Get-Consent)))
[void]$dnsBody.AppendLine(("integrity after dns={0}" -f ((Get-IntegrityLine) -replace '\s+', ' ').Trim()))
$dnsExit = 0
if ($plan.ExitCode -ne 0 -or $preview.ExitCode -ne 0 -or $confirmed.ExitCode -ne 0) { $dnsExit = 1 }
if ($noConfirm.ExitCode -ne 2 -or $wrong.ExitCode -ne 3) { $dnsExit = 1 }
$dnsFinished = Get-Date
Write-GateLog -Name 'dns-native' -CommandLine 'devsweep optimize plan/preview/run dns.flush isolated LOCALAPPDATA' -Started $dnsStarted -Finished $dnsFinished -ExitCode $dnsExit -Body $dnsBody.ToString()

# -------------------- 3. Settings CLEAN launched pair --------------------
$settingsStarted = Get-Date
$settingsBody = New-Object System.Text.StringBuilder
$settingsLapp = Join-Path $FixtureRoot 'lappdata-settings'
New-Item -ItemType Directory -Force -Path $settingsLapp | Out-Null
$settingsPlan = Join-Path $FixtureRoot 'settings-search-plan.json'
$settingsPreview = Join-Path $FixtureRoot 'settings-search-preview.json'
[void]$settingsBody.AppendLine(("LOCALAPPDATA={0}" -f $settingsLapp))
[void]$settingsBody.AppendLine('This recapture does NOT reuse implementer PID 27684 or probe PID 67816.')
[void]$settingsBody.AppendLine(("consent before settings: {0}" -f (Get-Consent)))

# Re-close any leftovers immediately before launch
$preClose = New-Object System.Text.StringBuilder
foreach ($row in @(Get-SystemSettingsSnapshot)) {
  Close-IdentifiedSettings $row $preClose
}
[void]$settingsBody.AppendLine($preClose.ToString())
Start-Sleep -Milliseconds 1000
$before = @(Get-SystemSettingsSnapshot)
$beforePids = @($before | ForEach-Object { [int]$_.Pid })
[void]$settingsBody.AppendLine( (Format-SettingsSnapshot $before 'processes BEFORE confirmed settings.search run') )
$launchAt = Get-Date

$splan = Invoke-Devsweep -LocalAppData $settingsLapp -ArgList @('optimize','plan','--operation','settings.search','--output',$settingsPlan)
[void]$settingsBody.AppendLine('== plan settings.search ==')
[void]$settingsBody.AppendLine($splan.Combined)
[void]$settingsBody.AppendLine(("plan exit={0}" -f $splan.ExitCode))
if (Test-Path $settingsPlan) { [void]$settingsBody.AppendLine((Get-Content -Raw $settingsPlan)) }

$spreview = Invoke-Devsweep -LocalAppData $settingsLapp -ArgList @('optimize','preview','--plan',$settingsPlan,'--format','json','--output',$settingsPreview)
[void]$settingsBody.AppendLine('== preview settings.search ==')
[void]$settingsBody.AppendLine($spreview.Combined)
$sdigest = $null
if (Test-Path $settingsPreview) {
  $spreviewJson = Get-Content -Raw $settingsPreview
  [void]$settingsBody.AppendLine($spreviewJson)
  try { $sdigest = (ConvertFrom-Json $spreviewJson).data.digest } catch { $sdigest = $null }
}
[void]$settingsBody.AppendLine(("digest={0}" -f $sdigest))

$sno = Invoke-Devsweep -LocalAppData $settingsLapp -ArgList @('optimize','run','--plan',$settingsPlan,'--preview-digest',$sdigest)
[void]$settingsBody.AppendLine('== run no-confirm ==')
[void]$settingsBody.AppendLine($sno.Combined)
[void]$settingsBody.AppendLine(("no-confirm exit={0} (expect 2)" -f $sno.ExitCode))

$swrong = Invoke-Devsweep -LocalAppData $settingsLapp -ArgList @('optimize','run','--plan',$settingsPlan,'--preview-digest','sha256:0000000000000000000000000000000000000000000000000000000000000000','--confirm')
[void]$settingsBody.AppendLine('== run wrong-digest ==')
[void]$settingsBody.AppendLine($swrong.Combined)
[void]$settingsBody.AppendLine(("wrong-digest exit={0} (expect 3)" -f $swrong.ExitCode))

[void]$settingsBody.AppendLine('== run CONFIRMED settings.search (launched only; not completion) ==')
$srun = Invoke-Devsweep -LocalAppData $settingsLapp -ArgList @('optimize','run','--plan',$settingsPlan,'--preview-digest',$sdigest,'--confirm')
[void]$settingsBody.AppendLine($srun.Combined)
[void]$settingsBody.AppendLine(("confirmed-run exit={0}" -f $srun.ExitCode))
$runFinished = Get-Date

$newRow = $null
$deadline = (Get-Date).AddSeconds(12)
while ((Get-Date) -lt $deadline) {
  foreach ($row in @(Get-SystemSettingsSnapshot)) {
    $isNewPid = $beforePids -notcontains [int]$row.Pid
    $startedAfter = $false
    if ($row.StartTime) { $startedAfter = $row.StartTime -ge $launchAt.AddSeconds(-1) }
    if ($isNewPid -and (Test-SettingsIdentity $row) -and $startedAfter) {
      $newRow = $row
      break
    }
  }
  if ($newRow) { break }
  Start-Sleep -Milliseconds 400
}

[void]$settingsBody.AppendLine( (Format-SettingsSnapshot (Get-SystemSettingsSnapshot) 'processes AFTER confirmed run (polled)') )
if ($newRow) {
  [void]$settingsBody.AppendLine('NEW PROCESS IDENTITY (distinct from pre-existing PIDs):')
  [void]$settingsBody.AppendLine(("PID={0} Path={1} StartTime={2} HWND(Get-Process)={3}" -f $newRow.Pid, $newRow.Path, $newRow.StartTime, $newRow.HWND))
  [void]$settingsBody.AppendLine('path matches C:\Windows\ImmersiveControlPanel\SystemSettings.exe: True')
  [void]$settingsBody.AppendLine(("pre-existing PIDs were: [{0}]" -f ($beforePids -join ',')))
  [void]$settingsBody.AppendLine(("PID is new: {0}" -f ($beforePids -notcontains [int]$newRow.Pid)))
  [void]$settingsBody.AppendLine("-- EnumWindows for PID $($newRow.Pid) --")
  [void]$settingsBody.AppendLine((Get-WindowsForPid ([int]$newRow.Pid)))
} else {
  [void]$settingsBody.AppendLine('FAIL: no new SystemSettings.exe with ImmersiveControlPanel path and start-time after launch')
  [void]$settingsBody.AppendLine(("pre-existing PIDs: [{0}]" -f ($beforePids -join ',')))
}

$sjournal = Join-Path $settingsLapp 'DevSweep\audit\v1\optimize.jsonl'
[void]$settingsBody.AppendLine(("journal path={0} exists={1}" -f $sjournal, (Test-Path $sjournal)))
if (Test-Path $sjournal) {
  [void]$settingsBody.AppendLine('-- journal --')
  [void]$settingsBody.AppendLine((Get-Content -Raw $sjournal))
  [void]$settingsBody.AppendLine((Test-JournalRedaction $sjournal))
}

# Close the recapture-started process after identity verification
$postClose = New-Object System.Text.StringBuilder
if ($newRow) {
  [void]$postClose.AppendLine('Closing recapture-started SystemSettings after identity verification')
  Close-IdentifiedSettings $newRow $postClose
} else {
  foreach ($row in @(Get-SystemSettingsSnapshot)) {
    if ($beforePids -notcontains [int]$row.Pid) {
      Close-IdentifiedSettings $row $postClose
    }
  }
}
[void]$settingsBody.AppendLine($postClose.ToString())
Start-Sleep -Milliseconds 600
[void]$settingsBody.AppendLine( (Format-SettingsSnapshot (Get-SystemSettingsSnapshot) 'SystemSettings AFTER recapture close') )
[void]$settingsBody.AppendLine(("consent after settings: {0}" -f (Get-Consent)))
[void]$settingsBody.AppendLine('Do not treat launched as maintenance completion. No Windows setting was changed. No signing.')

$settingsExit = 0
if ($splan.ExitCode -ne 0 -or $spreview.ExitCode -ne 0) { $settingsExit = 1 }
if ($sno.ExitCode -ne 2 -or $swrong.ExitCode -ne 3) { $settingsExit = 1 }
if (-not $newRow) { $settingsExit = 1 }
if ($srun.ExitCode -ne 0) { $settingsExit = 1 }
$settingsFinished = Get-Date
Write-GateLog -Name 'settings-native' -CommandLine 'CLEAN settings.search launched pair with new PID/HWND identity' -Started $settingsStarted -Finished $settingsFinished -ExitCode $settingsExit -Body $settingsBody.ToString()

# -------------------- 4. Adapter launch-failure control (not a catalogue id) --------------------
$probeStarted = Get-Date
$probeBody = New-Object System.Text.StringBuilder
[void]$probeBody.AppendLine('Adapter-level launch-failure control via ignored native probe.')
[void]$probeBody.AppendLine('Allowlisted PID from this probe is NOT used as CLI launch proof.')
[void]$probeBody.AppendLine('Unregistered-scheme hInstApp is host shell behavior, not catalogue leak.')
$probeOut = Join-Path $env:TEMP 'devsweep-indep-settings-probe.out.txt'
if (Test-Path $probeOut) { Remove-Item -Force $probeOut }
cmd /c "cd /d `"$Cwd`" && cargo test -p devsweep-core --lib native_settings_shell_execute_probe -- --ignored --nocapture > `"$probeOut`" 2>&1"
$probeExit = $LASTEXITCODE
if (Test-Path $probeOut) { [void]$probeBody.AppendLine([System.IO.File]::ReadAllText($probeOut)) }
[void]$probeBody.AppendLine(("cargo probe exit={0}" -f $probeExit))
# Close whatever Settings the probe opened, identity-checked
$probeClose = New-Object System.Text.StringBuilder
foreach ($row in @(Get-SystemSettingsSnapshot)) {
  Close-IdentifiedSettings $row $probeClose
}
[void]$probeBody.AppendLine($probeClose.ToString())
[void]$probeBody.AppendLine(("consent after probe: {0}" -f (Get-Consent)))
$probeFinished = Get-Date
Write-GateLog -Name 'settings-probe-failure' -CommandLine 'cargo test native_settings_shell_execute_probe --ignored --nocapture' -Started $probeStarted -Finished $probeFinished -ExitCode $probeExit -Body $probeBody.ToString()

# -------------------- 5. Hostile / unregistered URIs cannot enter plan/preview/run --------------------
$rejStarted = Get-Date
$rejBody = New-Object System.Text.StringBuilder
$rejLapp = Join-Path $FixtureRoot 'lappdata-rejections'
$rejOutDir = Join-Path $FixtureRoot 'rejected'
New-Item -ItemType Directory -Force -Path $rejLapp, $rejOutDir | Out-Null
[void]$rejBody.AppendLine(("LOCALAPPDATA={0}" -f $rejLapp))
[void]$rejBody.AppendLine('Prove plan/preview/run cannot dispatch non-allowlisted URIs or hostile ids.')

$hostileOps = @(
  'ms-settings:search',
  'ms-settings:evil',
  'ms-settings:storagerecommendations',
  'ms-settings:energyrecommendations',
  'devsweep-not-a-protocol:test',
  'guidance.drive_optimize',
  'guidance.system_integrity',
  'guidance.filesystem_check',
  'guidance.network_reset',
  'security.defender',
  'security.uac',
  'firewall.reset',
  'windows.update.reset',
  'registry.tweak',
  'cmd.exe /c calc',
  'ipconfig',
  'DNS.FLUSH'
)
$written = 0
foreach ($op in $hostileOps) {
  $safe = ($op -replace '[^A-Za-z0-9._-]', '_')
  $out = Join-Path $rejOutDir ($safe + '.json')
  if (Test-Path $out) { Remove-Item -Force $out }
  $r = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','plan','--operation',$op,'--output',$out)
  $exists = Test-Path $out
  if ($exists) { $written += 1 }
  [void]$rejBody.AppendLine(("== PLAN {0} exit={1} file_exists={2} ==" -f $op, $r.ExitCode, $exists))
  [void]$rejBody.AppendLine($r.Combined.Trim())
}

# Hostile saved plan with uri field -> preview must fail closed, no dispatch
$hostilePlan = Join-Path $rejOutDir 'hostile-uri-field.json'
[System.IO.File]::WriteAllText($hostilePlan, '{"version":1,"catalogue_version":1,"operation_id":"settings.search","uri":"ms-settings:evil"}' + "`n")
$hp = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','preview','--plan',$hostilePlan,'--format','json')
[void]$rejBody.AppendLine('== PREVIEW plan with extra uri field ==')
[void]$rejBody.AppendLine(("exit={0}" -f $hp.ExitCode))
[void]$rejBody.AppendLine($hp.Combined)

$uriIdPlan = Join-Path $rejOutDir 'uri-as-operation-id.json'
[System.IO.File]::WriteAllText($uriIdPlan, '{"version":1,"catalogue_version":1,"operation_id":"ms-settings:search"}' + "`n")
$up = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','preview','--plan',$uriIdPlan,'--format','json')
[void]$rejBody.AppendLine('== PREVIEW operation_id=ms-settings:search (URI is not a catalogue id) ==')
[void]$rejBody.AppendLine(("exit={0}" -f $up.ExitCode))
[void]$rejBody.AppendLine($up.Combined)

$ur = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','run','--plan',$uriIdPlan,'--preview-digest','sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','--confirm')
[void]$rejBody.AppendLine('== RUN operation_id=ms-settings:search ==')
[void]$rejBody.AppendLine(("exit={0}" -f $ur.ExitCode))
[void]$rejBody.AppendLine($ur.Combined)

$evilPlan = Join-Path $rejOutDir 'unregistered-scheme-id.json'
[System.IO.File]::WriteAllText($evilPlan, '{"version":1,"catalogue_version":1,"operation_id":"devsweep-not-a-protocol:test"}' + "`n")
$ep = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','preview','--plan',$evilPlan,'--format','json')
[void]$rejBody.AppendLine('== PREVIEW unregistered scheme as operation_id ==')
[void]$rejBody.AppendLine(("exit={0}" -f $ep.ExitCode))
[void]$rejBody.AppendLine($ep.Combined)
$er = Invoke-Devsweep -LocalAppData $rejLapp -ArgList @('optimize','run','--plan',$evilPlan,'--preview-digest','sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','--confirm')
[void]$rejBody.AppendLine('== RUN unregistered scheme as operation_id ==')
[void]$rejBody.AppendLine(("exit={0}" -f $er.ExitCode))
[void]$rejBody.AppendLine($er.Combined)

$rejJournal = Join-Path $rejLapp 'DevSweep\audit\v1\optimize.jsonl'
[void]$rejBody.AppendLine(("rejection journal exists={0} (expect False; no dispatch)" -f (Test-Path $rejJournal)))
[void]$rejBody.AppendLine(("hostile plan files written by CLI plan: {0} (expect 0)" -f $written))
[void]$rejBody.AppendLine(("consent after rejections: {0}" -f (Get-Consent)))

$rejExit = 0
if ($written -ne 0) { $rejExit = 1 }
if (Test-Path $rejJournal) { $rejExit = 1 }
if ($hp.ExitCode -eq 0 -or $up.ExitCode -eq 0 -or $ur.ExitCode -eq 0 -or $ep.ExitCode -eq 0 -or $er.ExitCode -eq 0) { $rejExit = 1 }
$rejFinished = Get-Date
Write-GateLog -Name 'rejections' -CommandLine 'devsweep optimize plan/preview/run hostile ids and non-allowlisted URIs' -Started $rejStarted -Finished $rejFinished -ExitCode $rejExit -Body $rejBody.ToString()

Write-Host "NATIVE HARNESS DONE dns=$dnsExit settings=$settingsExit probe=$probeExit rejections=$rejExit"
exit 0
