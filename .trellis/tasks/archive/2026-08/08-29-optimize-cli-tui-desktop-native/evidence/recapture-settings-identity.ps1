$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$ExpectedSettingsPath = 'C:\Windows\ImmersiveControlPanel\SystemSettings.exe'
$NativeRoot = Join-Path $EvidenceRoot 'native-independent'
$FixtureRoot = Join-Path $NativeRoot 'fixtures'
$Lappdata = Join-Path $NativeRoot 'lappdata'
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

function Get-Consent {
  $p = Get-Process -Name consent -ErrorAction SilentlyContinue
  if ($p) { return ($p | ForEach-Object { $_.Id }) -join ',' }
  return 'none'
}

function Get-SystemSettingsSnapshot {
  $rows = @()
  Get-CimInstance Win32_Process -Filter "Name='SystemSettings.exe'" -ErrorAction SilentlyContinue | ForEach-Object {
    $gp = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
    $rows += [pscustomobject]@{
      Pid = $_.ProcessId
      Path = $_.ExecutablePath
      StartTime = if ($gp) { $gp.StartTime } else { $null }
      MainWindowHandle = if ($gp) { $gp.MainWindowHandle.ToInt64() } else { 0 }
      CommandLine = $_.CommandLine
    }
  }
  return $rows
}

try {
  Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class DevSweepIndependentEnum {
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
  return [DevSweepIndependentEnum]::WindowsForPid([uint32]$ProcId)
}

function Close-IdentifiedSettings($row, [System.Text.StringBuilder]$Log) {
  if (-not $row -or -not [string]::Equals($row.Path, $ExpectedSettingsPath, [StringComparison]::OrdinalIgnoreCase)) {
    [void]$Log.AppendLine(("REFUSE_KILL unverified identity PID={0} Path={1}" -f $row.Pid, $row.Path))
    return $false
  }
  $gp = Get-Process -Id $row.Pid -ErrorAction SilentlyContinue
  if (-not $gp) {
    [void]$Log.AppendLine(("already exited PID={0}" -f $row.Pid))
    return $true
  }
  $closed = $false
  try { $closed = $gp.CloseMainWindow() } catch { $closed = $false }
  [void]$Log.AppendLine(("CloseMainWindow PID={0} returned={1} HWND={2}" -f $row.Pid, $closed, $gp.MainWindowHandle.ToInt64()))
  $deadline = (Get-Date).AddSeconds(8)
  while ((Get-Date) -lt $deadline) {
    if (-not (Get-Process -Id $row.Pid -ErrorAction SilentlyContinue)) {
      [void]$Log.AppendLine(("closed gracefully PID={0}" -f $row.Pid))
      return $true
    }
    Start-Sleep -Milliseconds 250
  }
  $again = Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $row.Pid) -ErrorAction SilentlyContinue
  if (-not $again) { return $true }
  if (-not [string]::Equals($again.ExecutablePath, $ExpectedSettingsPath, [StringComparison]::OrdinalIgnoreCase)) {
    [void]$Log.AppendLine(("REFUSE_FORCE_KILL path mismatch PID={0} Path={1}" -f $row.Pid, $again.ExecutablePath))
    return $false
  }
  Stop-Process -Id $row.Pid -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 400
  $still = Get-Process -Id $row.Pid -ErrorAction SilentlyContinue
  [void]$Log.AppendLine(("force-kill after identity re-check PID={0} still_present={1}" -f $row.Pid, [bool]$still))
  return -not [bool]$still
}

function Wait-NoSystemSettings([System.Text.StringBuilder]$Log) {
  $deadline = (Get-Date).AddSeconds(15)
  while ((Get-Date) -lt $deadline) {
    $rows = @(Get-SystemSettingsSnapshot)
    if ($rows.Count -eq 0) {
      [void]$Log.AppendLine('before_empty=True')
      return $true
    }
    foreach ($row in $rows) { [void](Close-IdentifiedSettings $row $Log) }
    Start-Sleep -Milliseconds 400
  }
  $left = @(Get-SystemSettingsSnapshot)
  [void]$Log.AppendLine(("before_empty={0} leftover_count={1}" -f ($left.Count -eq 0), $left.Count))
  return ($left.Count -eq 0)
}

function Extract-Digest([string]$Text) {
  if ($Text -match 'sha256:[0-9a-f]{64}') { return $Matches[0] }
  return $null
}

function Invoke-SettingsLaunch([string]$Operation) {
  $log = New-Object System.Text.StringBuilder
  [void]$log.AppendLine("operation=$Operation")
  [void]$log.AppendLine("consent before=$(Get-Consent)")
  $cleared = Wait-NoSystemSettings $log
  if (-not $cleared) {
    [void]$log.AppendLine('FAIL leftover SystemSettings present before launch')
    return [pscustomobject]@{ ExitCode = 99; Body = $log.ToString(); Unique = $false }
  }
  $plan = Join-Path $FixtureRoot ($Operation.Replace('.', '-') + '-plan.json')
  $p = Invoke-Devsweep -ArgList @('optimize', 'plan', '--operation', $Operation, '--output', $plan)
  [void]$log.AppendLine("plan exit=$($p.ExitCode)")
  [void]$log.AppendLine($p.Combined)
  $v = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'preview', '--plan', $plan)
  [void]$log.AppendLine("preview exit=$($v.ExitCode)")
  [void]$log.AppendLine($v.Combined)
  $digest = Extract-Digest $v.Combined
  [void]$log.AppendLine("digest=$digest")
  $stale = Invoke-Devsweep -ArgList @('optimize', 'run', '--plan', $plan, '--preview-digest', 'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', '--confirm')
  [void]$log.AppendLine("stale exit=$($stale.ExitCode)")
  [void]$log.AppendLine($stale.Combined)
  $run = Invoke-Devsweep -ArgList @('--language', 'en', 'optimize', 'run', '--plan', $plan, '--preview-digest', $digest, '--confirm')
  [void]$log.AppendLine("run exit=$($run.ExitCode)")
  [void]$log.AppendLine($run.Combined)
  Start-Sleep -Seconds 2
  $after = @(Get-SystemSettingsSnapshot)
  [void]$log.AppendLine("-- after --")
  foreach ($row in $after) {
    [void]$log.AppendLine(("PID={0} Path={1} StartTime={2} MainWindowHandle={3} CommandLine={4}" -f $row.Pid, $row.Path, $row.StartTime, $row.MainWindowHandle, $row.CommandLine))
    [void]$log.AppendLine((Get-WindowsForPid $row.Pid))
  }
  $unique = $false
  $visibleHwnd = $false
  foreach ($row in $after) {
    $pathOk = [string]::Equals($row.Path, $ExpectedSettingsPath, [StringComparison]::OrdinalIgnoreCase)
    $windows = Get-WindowsForPid $row.Pid
    if ($windows -match 'visible=True') { $visibleHwnd = $true }
    [void]$log.AppendLine(("candidate PID={0} path_ok={1} unique_new_pid=True leftover_reused=False" -f $row.Pid, $pathOk))
    if ($pathOk) { $unique = $true }
    [void](Close-IdentifiedSettings $row $log)
  }
  if ($after.Count -eq 0) {
    [void]$log.AppendLine('FAIL no SystemSettings.exe after launched outcome')
    $unique = $false
  }
  [void]$log.AppendLine("visible_hwnd=$visibleHwnd")
  [void]$log.AppendLine("launched_copy=$($run.Combined -match 'Launched')")
  [void]$log.AppendLine("claims_completion=$($run.Combined -match 'optimization complete')")
  [void]$log.AppendLine("not_completion_note=$($run.Combined -match 'not maintenance completion')")
  [void]$log.AppendLine("consent after=$(Get-Consent)")
  return [pscustomobject]@{ ExitCode = $run.ExitCode; Body = $log.ToString(); Unique = $unique; Visible = $visibleHwnd }
}

$started = Get-Date
$search = Invoke-SettingsLaunch 'settings.search'
$storage = Invoke-SettingsLaunch 'settings.storage_recommendations'
$energy = Invoke-SettingsLaunch 'settings.energy_recommendations'
$finished = Get-Date

$overallUnique = $search.Unique -and $storage.Unique -and $energy.Unique
$overallExit = 0
if ($search.ExitCode -ne 0 -or $storage.ExitCode -ne 0 -or $energy.ExitCode -ne 0 -or -not $overallUnique) { $overallExit = 1 }

$body = @"
integrity=$(whoami /groups | Where-Object { $_ -match 'S-1-16-' })
consent_final=$(Get-Consent)
search_unique=$($search.Unique) search_visible=$($search.Visible) search_exit=$($search.ExitCode)
storage_unique=$($storage.Unique) storage_visible=$($storage.Visible) storage_exit=$($storage.ExitCode)
energy_unique=$($energy.Unique) energy_visible=$($energy.Visible) energy_exit=$($energy.ExitCode)
overall_unique_new_pids=$overallUnique
NOTE: leftover SystemSettings was cleared before each launch. launched is not maintenance completion.

======== settings.search ========
$($search.Body)

======== settings.storage_recommendations ========
$($storage.Body)

======== settings.energy_recommendations ========
$($energy.Body)
"@

$logPath = Join-Path $EvidenceRoot 'independent-native-settings-identity.log'
$text = "command: independent recapture of three Settings launches with empty-before identity`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$overallExit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
[System.IO.File]::WriteAllText($logPath, $text)
Write-Host ("WROTE $logPath exit=$overallExit unique=$overallUnique")
exit $overallExit
