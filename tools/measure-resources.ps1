#requires -Version 5.1
<#
.SYNOPSIS
  Frozen five-mode-v1 resource sampler.

.DESCRIPTION
  One recorded Windows 11 standard-user host, release binaries, 200 ms samples.
  CPU% = delta process TotalProcessorTime / wall * 100 (100% = one logical core).
  Nearest-rank p95. Do not change thresholds after viewing results.
#>
[CmdletBinding()]
param(
    [ValidateSet('five-mode-v1')]
    [string]$Protocol = 'five-mode-v1',
    [string]$RepoRoot = '',
    [string]$OutDir = '',
    [string]$CleanBaseline = '',
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:IdlePid = 0
$script:DesktopPid = 0
$script:CdpServe = $null
$script:SizingWorktree = $null
trap {
    $line = 0
    if ($null -ne $_.InvocationInfo) { $line = $_.InvocationInfo.ScriptLineNumber }
    Write-Host ("SAMPLER_ERROR line={0} {1}" -f $line, $_.Exception.Message)
    try {
        if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
            Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue
        }
        if ($script:IdlePid) { Stop-OwnedProcess -ProcessId ([int]$script:IdlePid) -ExpectedPath $desktopExe }
        if ($script:DesktopPid) { Stop-OwnedProcess -ProcessId ([int]$script:DesktopPid) -ExpectedPath $desktopExe }
        if ($script:SizingWorktree -and (Get-Command Remove-SizingWorktree -ErrorAction SilentlyContinue)) {
            Remove-SizingWorktree
        }
    }
    catch {}
    Write-Error $_
    exit 1
}
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}
if (-not $OutDir) {
    $OutDir = Join-Path $RepoRoot '.trellis\tasks\08-29-five-mode-native-integration\evidence\resources'
}

$rawDir = Join-Path $OutDir ('raw-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ'))
if (-not $SelfTest) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    New-Item -ItemType Directory -Force -Path $rawDir | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $OutDir 'raw-latest.txt'), $rawDir, [System.Text.UTF8Encoding]::new($false))
}

$cliExe = Join-Path $RepoRoot 'target\release\devsweep.exe'
$desktopExe = Join-Path $RepoRoot 'target\release\devsweep-desktop.exe'
if (-not $SelfTest) {
    if (-not (Test-Path -LiteralPath $cliExe)) {
        throw "missing release CLI: $cliExe"
    }
    if (-not (Test-Path -LiteralPath $desktopExe)) {
        throw "missing release desktop: $desktopExe"
    }
}

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function ConvertTo-Utf8Json($Value, [string]$Path) {
    $json = $Value | ConvertTo-Json -Depth 12 -Compress:$false
    [System.IO.File]::WriteAllText($Path, $json, [System.Text.UTF8Encoding]::new($false))
}

function Get-CollectionCount($Value) {
    if ($null -eq $Value) { return 0 }
    if ($Value -is [System.Collections.ICollection]) { return [int]$Value.Count }
    $prop = $Value.PSObject.Properties['Count']
    if ($null -ne $prop -and $null -ne $prop.Value) { return [int]$prop.Value }
    return 1
}

function Get-NearestRank([double[]]$Values, [double]$P) {
    $sorted = @($Values | Sort-Object)
    $n = Get-CollectionCount $sorted
    if ($n -eq 0) { return $null }
    $index = [Math]::Max(0, [Math]::Ceiling($P * $n) - 1)
    if ($index -ge $n) { $index = $n - 1 }
    return [double]$sorted[$index]
}

function Get-Median([double[]]$Values) {
    return Get-NearestRank $Values 0.5
}

function Write-SampleJsonl($Samples, [string]$Path) {
    $builder = New-Object System.Text.StringBuilder
    foreach ($row in $Samples) {
        [void]$builder.AppendLine(($row | ConvertTo-Json -Compress -Depth 8))
    }
    [System.IO.File]::WriteAllText($Path, $builder.ToString(), [System.Text.UTF8Encoding]::new($false))
}

function Read-SampleJsonl([string]$Path) {
    $list = New-Object System.Collections.ArrayList
    if (-not (Test-Path -LiteralPath $Path)) { return , $list }
    foreach ($line in [System.IO.File]::ReadAllLines($Path)) {
        if ($line.Trim().Length -eq 0) { continue }
        [void]$list.Add(($line | ConvertFrom-Json))
    }
    return , $list
}

function Get-SampleRows($Samples) {
    $list = New-Object System.Collections.ArrayList
    if ($null -eq $Samples) {
        return , $list
    }
    $presentProp = $Samples.PSObject.Properties['present']
    if ($null -ne $presentProp) {
        [void]$list.Add($Samples)
        return , $list
    }
    foreach ($row in $Samples) {
        if ($null -ne $row -and $null -ne $row.PSObject.Properties['present']) {
            [void]$list.Add($row)
        }
    }
    return , $list
}

function Get-CpuPercents($Samples) {
    $rows = Get-SampleRows $Samples
    $pcts = New-Object System.Collections.Generic.List[double]
    $n = Get-CollectionCount $rows
    for ($i = 1; $i -lt $n; $i++) {
        $a = $rows[$i - 1]
        $b = $rows[$i]
        if (-not $a.present -or -not $b.present) { continue }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { continue }
        [void]$pcts.Add(100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt))
    }
    return , $pcts
}

# A post-stop window is only evidence when every scheduled sample observed the
# same live process. Zero samples, a gap, or an absent PID is a failure, never
# a quiet pass.
function Test-PostStopSamples {
    param(
        $Samples,
        [int]$ProcessId,
        [int]$Expected = 25
    )
    $rows = Get-SampleRows $Samples
    if ((Get-CollectionCount $rows) -ne $Expected) { return $false }
    foreach ($row in $rows) {
        if (-not $row.present) { return $false }
        if ($null -eq $row.PSObject.Properties['pid']) { return $false }
        if ([int]$row.pid -ne $ProcessId) { return $false }
    }
    return $true
}

# The archived quiescence predicate: the final five consecutive samples stay at
# or below the idle CPU median plus 0.5 percentage points and the idle thread
# maximum plus one. It requires present samples, so an absent process fails.
function Test-FinalFiveHold {
    param(
        $Samples,
        [double]$IdleMedianCpu,
        [int]$IdleMaxThreads
    )
    $rows = Get-SampleRows $Samples
    $count = Get-CollectionCount $rows
    if ($count -lt 6) { return $false }
    for ($idx = $count - 5; $idx -lt $count; $idx++) {
        $b = $rows[$idx]
        $a = $rows[$idx - 1]
        if (-not $a.present -or -not $b.present) { return $false }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { return $false }
        $cpuPct = 100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt)
        if ($cpuPct -gt ($IdleMedianCpu + 0.5)) { return $false }
        if ([int]$b.threads -gt ($IdleMaxThreads + 1)) { return $false }
    }
    return $true
}

# Desktop quiescence. The CPU clause is the archived one, and every sample must
# be present. There is no per-rep thread clause: WebView2 parks pool threads
# that hold zero CPU and decay over one to two minutes, so a thread count in a
# 5 s window measures the browser pool and not the operation. Thread growth is
# checked across the five stops instead (Test-DesktopFloorStable).
function Test-DesktopQuiescence {
    param(
        $Samples,
        [double]$BaselineMedianCpu
    )
    $rows = Get-SampleRows $Samples
    $count = Get-CollectionCount $rows
    if ($count -lt 6) { return $false }
    for ($idx = $count - 5; $idx -lt $count; $idx++) {
        $b = $rows[$idx]
        $a = $rows[$idx - 1]
        if (-not $a.present -or -not $b.present) { return $false }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { return $false }
        $cpuPct = 100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt)
        if ($cpuPct -gt ($BaselineMedianCpu + 0.5)) { return $false }
    }
    $present = @($rows | Where-Object { $_.present })
    if ((Get-CollectionCount $present) -ne $count) { return $false }
    return $true
}

# A producer or worker thread that survives each stop raises the post-stop
# thread floor by one per stop. Over five stops on one app PID the last floor
# must stay within one thread of the first.
function Test-DesktopFloorStable {
    param($Floors)
    $values = @($Floors)
    if ((Get-CollectionCount $values) -lt 2) { return $false }
    foreach ($value in $values) { if ($null -eq $value) { return $false } }
    return ([int]$values[-1] -le ([int]$values[0] + 1))
}

# The Clean comparison is only valid against a preserved baseline binary built
# on the same host and toolchain, over the same root and a stable entry count.
function Test-CleanComparable {
    param($Manifest)
    if ($null -eq $Manifest) { return $false }
    foreach ($key in @('baseline_sha256_recorded', 'baseline_sha256_live', 'candidate_rustc', 'baseline_rustc', 'scan_root', 'repo_root', 'entry_count_before', 'entry_count_after')) {
        if ($null -eq $Manifest.PSObject.Properties[$key]) { return $false }
    }
    if (-not $Manifest.baseline_sha256_recorded) { return $false }
    if (-not $Manifest.baseline_sha256_live) { return $false }
    if ($Manifest.baseline_sha256_recorded -ne $Manifest.baseline_sha256_live) { return $false }
    $baselineToolchain = ([string]$Manifest.baseline_rustc) -replace "`r`n", "`n"
    $candidateToolchain = ([string]$Manifest.candidate_rustc) -replace "`r`n", "`n"
    if ($baselineToolchain -ne $candidateToolchain) { return $false }
    if ($Manifest.scan_root -ne $Manifest.repo_root) { return $false }
    foreach ($key in @('cpu_name', 'logical_cores', 'windows_build', 'power_plan')) {
        $recorded = "host_${key}_recorded"
        $observed = "host_${key}_observed"
        if ($null -eq $Manifest.PSObject.Properties[$recorded] -or $null -eq $Manifest.PSObject.Properties[$observed]) { return $false }
        if ([string]$Manifest.$recorded -ne [string]$Manifest.$observed) { return $false }
    }
    $before = [double]$Manifest.entry_count_before
    $after = [double]$Manifest.entry_count_after
    if ($before -le 0) { return $false }
    if (([math]::Abs($after - $before) / $before) -gt 0.02) { return $false }
    return $true
}

if ($SelfTest) {
    $selfFailures = New-Object System.Collections.ArrayList
    function Assert-SelfTest([string]$Name, [bool]$Actual, [bool]$Expected) {
        if ($Actual -ne $Expected) { [void]$selfFailures.Add("$Name :: expected $Expected, got $Actual") }
        Write-Host ("selftest {0} expected={1} actual={2}" -f $Name, $Expected, $Actual)
    }
    $selfPid = [int]$PID
    $presentRows = @(0..24 | ForEach-Object {
            [pscustomobject]@{ wall_ms = ($_ * 200.0); present = $true; pid = $selfPid; cpu_ms = 0.0; private = 1; threads = 1 }
        })
    $absentRows = @(0..24 | ForEach-Object {
            [pscustomobject]@{ wall_ms = ($_ * 200.0); present = $false; pid = $selfPid; cpu_ms = 0.0; private = 0; threads = 0 }
        })
    $otherRows = @(0..24 | ForEach-Object {
            [pscustomobject]@{ wall_ms = ($_ * 200.0); present = $true; pid = ($selfPid + 1); cpu_ms = 0.0; private = 1; threads = 1 }
        })
    Assert-SelfTest 'poststop.present_same_pid' (Test-PostStopSamples $presentRows $selfPid 25) $true
    Assert-SelfTest 'poststop.absent_pid' (Test-PostStopSamples $absentRows $selfPid 25) $false
    Assert-SelfTest 'poststop.different_pid' (Test-PostStopSamples $otherRows $selfPid 25) $false
    Assert-SelfTest 'poststop.no_samples' (Test-PostStopSamples @() $selfPid 25) $false
    Assert-SelfTest 'finalfive.present_quiet' (Test-FinalFiveHold $presentRows 0.5 4) $true
    Assert-SelfTest 'finalfive.absent_pid' (Test-FinalFiveHold $absentRows 0.5 4) $false
    Assert-SelfTest 'finalfive.no_samples' (Test-FinalFiveHold @() 0.5 4) $false
    $busyRows = @(0..24 | ForEach-Object {
            [pscustomobject]@{ wall_ms = ($_ * 200.0); present = $true; pid = $selfPid; cpu_ms = ($_ * 20.0); private = 1; threads = 1 }
        })
    Assert-SelfTest 'quiescence.cpu_quiet' (Test-DesktopQuiescence $presentRows 0.5) $true
    Assert-SelfTest 'quiescence.cpu_busy' (Test-DesktopQuiescence $busyRows 0.5) $false
    Assert-SelfTest 'quiescence.absent_pid' (Test-DesktopQuiescence $absentRows 0.5) $false
    Assert-SelfTest 'quiescence.no_samples' (Test-DesktopQuiescence @() 0.5) $false
    Assert-SelfTest 'floor.stable' (Test-DesktopFloorStable @(42, 44, 42, 42, 42)) $true
    Assert-SelfTest 'floor.grows_per_stop' (Test-DesktopFloorStable @(42, 43, 44, 45, 46)) $false
    Assert-SelfTest 'floor.missing_rep' (Test-DesktopFloorStable @(42, $null, 42, 42, 42)) $false
    Assert-SelfTest 'floor.no_reps' (Test-DesktopFloorStable @()) $false
    $comparable = [pscustomobject]@{
        baseline_sha256_recorded    = 'abc'
        baseline_sha256_live        = 'abc'
        baseline_rustc              = 'rustc 1.0.0'
        candidate_rustc             = 'rustc 1.0.0'
        scan_root                   = 'D:/repo'
        repo_root                   = 'D:/repo'
        host_cpu_name_recorded      = 'cpu'
        host_cpu_name_observed      = 'cpu'
        host_logical_cores_recorded = 24
        host_logical_cores_observed = 24
        host_windows_build_recorded = '26200'
        host_windows_build_observed = '26200'
        host_power_plan_recorded    = 'balanced'
        host_power_plan_observed    = 'balanced'
        entry_count_before          = 1000
        entry_count_after           = 1005
    }
    Assert-SelfTest 'clean.comparable' (Test-CleanComparable $comparable) $true
    Assert-SelfTest 'clean.missing_manifest' (Test-CleanComparable $null) $false
    $missingHash = $comparable.PSObject.Copy()
    $missingHash.baseline_sha256_recorded = ''
    Assert-SelfTest 'clean.missing_baseline_hash' (Test-CleanComparable $missingHash) $false
    $changedHash = $comparable.PSObject.Copy()
    $changedHash.baseline_sha256_live = 'def'
    Assert-SelfTest 'clean.baseline_hash_differs' (Test-CleanComparable $changedHash) $false
    $otherToolchain = $comparable.PSObject.Copy()
    $otherToolchain.baseline_rustc = 'rustc 0.9.0'
    Assert-SelfTest 'clean.toolchain_drift' (Test-CleanComparable $otherToolchain) $false
    $otherHost = $comparable.PSObject.Copy()
    $otherHost.host_logical_cores_observed = 16
    Assert-SelfTest 'clean.host_drift' (Test-CleanComparable $otherHost) $false
    $otherRoot = $comparable.PSObject.Copy()
    $otherRoot.scan_root = 'D:/other'
    Assert-SelfTest 'clean.root_drift' (Test-CleanComparable $otherRoot) $false
    $entryDrift = $comparable.PSObject.Copy()
    $entryDrift.entry_count_after = 1500
    Assert-SelfTest 'clean.entry_count_drift' (Test-CleanComparable $entryDrift) $false
    if ($selfFailures.Count -gt 0) {
        Write-Error ("sampler self-test FAILED" + [Environment]::NewLine + ($selfFailures -join [Environment]::NewLine))
        exit 1
    }
    Write-Host 'sampler self-test PASS'
    exit 0
}

function Get-ProcessSnapshot([int]$ProcessId, [double]$WallMs) {
    $absent = [ordered]@{
        wall_ms   = [math]::Round($WallMs, 3)
        present   = $false
        pid       = $ProcessId
        user_ms   = 0.0
        kernel_ms = 0.0
        cpu_ms    = 0.0
        private   = [int64]0
        threads   = 0
    }
    $p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if ($null -eq $p) { return $absent }
    $privateBytes = [int64]0
    $threadCount = 0
    $userMs = 0.0
    $kernelMs = 0.0
    $cpuMs = 0.0
    try {
        $userMs = [math]::Round($p.UserProcessorTime.TotalMilliseconds, 3)
        $kernelMs = [math]::Round($p.PrivilegedProcessorTime.TotalMilliseconds, 3)
        $cpuMs = [math]::Round($p.TotalProcessorTime.TotalMilliseconds, 3)
        $privateBytes = [int64]$p.PrivateMemorySize64
        $threadCol = $p.Threads
        if ($null -ne $threadCol) { $threadCount = [int]$threadCol.Count }
    }
    catch {
        return $absent
    }
    return [ordered]@{
        wall_ms   = [math]::Round($WallMs, 3)
        present   = $true
        pid       = [int]$p.Id
        user_ms   = $userMs
        kernel_ms = $kernelMs
        cpu_ms    = $cpuMs
        private   = $privateBytes
        threads   = $threadCount
    }
}

function Get-ProcessTree([int]$Id) {
    $rows = @()
    try {
        $all = @(Get-CimInstance Win32_Process)
        $keep = @{}
        $keep[$Id] = $true
        $changed = $true
        while ($changed) {
            $changed = $false
            foreach ($procRow in $all) {
                $childId = [int]$procRow.ProcessId
                $parentValue = [int]$procRow.ParentProcessId
                if ($keep.ContainsKey($parentValue) -and -not $keep.ContainsKey($childId)) {
                    $keep[$childId] = $true
                    $changed = $true
                }
            }
        }
        $rows = @($all | Where-Object { $keep.ContainsKey([int]$_.ProcessId) } | ForEach-Object {
                [ordered]@{
                    pid        = $_.ProcessId
                    parent     = $_.ParentProcessId
                    name       = $_.Name
                    executable = $_.ExecutablePath
                    command    = $_.CommandLine
                }
            })
    }
    catch {
        $rows = @(@{ error = $_.Exception.Message })
    }
    return $rows
}

function Get-ProcessSamples {
    param(
        [Parameter(Mandatory = $true)][int]$ProcessId,
        [Parameter(Mandatory = $true)][int]$DurationMs,
        [int]$PeriodMs = 200,
        [string]$OutFile
    )
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    while ($sw.ElapsedMilliseconds -lt $DurationMs) {
        $now = $sw.ElapsedMilliseconds
        if ($now -ge $next) {
            $t = $sw.Elapsed.TotalMilliseconds
            [void]$samples.Add((Get-ProcessSnapshot -ProcessId $ProcessId -WallMs $t))
            $next += $PeriodMs
        }
        else {
            $sleep = [Math]::Min([int64]15, [int64]($next - $now))
            if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
        }
    }
    Write-SampleJsonl $samples $OutFile
}

function ConvertTo-ArgumentString([string[]]$Arguments) {
    ($Arguments | ForEach-Object {
            if ($_ -match '[\s"]') { '"' + (($_ -replace '\\', '\\') -replace '"', '\"') + '"' } else { $_ }
        }) -join ' '
}

function New-DevSweepStartInfo {
    param(
        [string[]]$Arguments,
        [string]$LocalAppData,
        [string]$Exe = ''
    )
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = if ($Exe) { $Exe } else { $cliExe }
    $psi.Arguments = ConvertTo-ArgumentString $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = $RepoRoot
    if ($LocalAppData) {
        $psi.EnvironmentVariables['LOCALAPPDATA'] = $LocalAppData
    }
    return $psi
}

function Stop-OwnedProcess {
    param(
        [int]$ProcessId,
        [string]$ExpectedPath
    )
    if ($ProcessId -le 0) { return }
    $leftover = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $leftover) { return }
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId").ExecutablePath } catch { $path = $null }
    if ($path -and $ExpectedPath -and ($path.ToLowerInvariant() -eq $ExpectedPath.ToLowerInvariant())) {
        Get-CimInstance Win32_Process -Filter "ParentProcessId=$ProcessId" -ErrorAction SilentlyContinue | ForEach-Object {
            $childName = [string]$_.Name
            if ($childName -match 'msedgewebview2|devsweep-desktop') {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue } catch {}
            }
        }
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Stop-NewSettingsProcesses($BeforeIds) {
    $after = @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue)
    foreach ($proc in $after) {
        if ($BeforeIds.ContainsKey([int]$proc.Id)) { continue }
        $path = $null
        try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $proc.Id)).ExecutablePath } catch { $path = $null }
        if (-not $path) { continue }
        $lower = $path.ToLowerInvariant()
        if ($lower -match 'systemsettings\.exe$' -or $lower -match 'applicationframehost\.exe$') {
            try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
        }
    }
}

function Start-DesktopNoDebug([string]$Profile) {
    New-Item -ItemType Directory -Force -Path $Profile | Out-Null
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $desktopExe
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $false
    $psi.WorkingDirectory = $RepoRoot
    $psi.EnvironmentVariables['LOCALAPPDATA'] = $Profile
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()
    return $proc
}

function Invoke-Cli {
    param(
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 300000
    )
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = New-DevSweepStartInfo -Arguments $Arguments -LocalAppData $LocalAppData
    $started = [Diagnostics.Stopwatch]::StartNew()
    [void]$proc.Start()
    $pidValue = $proc.Id
    if (-not $proc.WaitForExit($TimeoutMs)) {
        try { $proc.Kill() } catch {}
        throw "timeout: devsweep $($Arguments -join ' ')"
    }
    return [ordered]@{
        elapsed_ms = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
        exit       = $proc.ExitCode
        pid        = $pidValue
        stdout     = $proc.StandardOutput.ReadToEnd()
        stderr     = $proc.StandardError.ReadToEnd()
    }
}

function Wait-ProcessSamples {
    param(
        [Parameter(Mandatory = $true)][int]$ProcessId,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [int]$PeriodMs = 200,
        [int]$MaxMs = 300000
    )
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    $seen = $false
    try {
        while ($sw.ElapsedMilliseconds -lt $MaxMs) {
            $now = $sw.ElapsedMilliseconds
            if ($now -ge $next) {
                $t = $sw.Elapsed.TotalMilliseconds
                $row = Get-ProcessSnapshot -ProcessId $ProcessId -WallMs $t
                [void]$samples.Add($row)
                if ($row.present) {
                    $seen = $true
                }
                else {
                    $alive = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
                    if (-not $alive -and ($seen -or $sw.ElapsedMilliseconds -ge 200)) { break }
                }
                $next += $PeriodMs
            }
            else {
                $sleep = [Math]::Min([int64]15, [int64]($next - $now))
                if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
            }
        }
    }
    finally {
        Write-SampleJsonl $samples $OutFile
    }
}

function Measure-CliRun {
    param(
        [string]$Label,
        [int]$Run,
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 300000,
        [string]$Exe = ''
    )
    $stdoutPath = Join-Path $rawDir "$Label-stdout.json"
    $runArgs = if ($Arguments -contains '--output') { $Arguments } else { $Arguments + @('--output', $stdoutPath) }
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = New-DevSweepStartInfo -Arguments $runArgs -LocalAppData $LocalAppData -Exe $Exe
    $started = [Diagnostics.Stopwatch]::StartNew()
    [void]$proc.Start()
    $pidValue = $proc.Id
    $tree = Get-ProcessTree -Id $pidValue
    $pipePath = Join-Path $rawDir "$Label-pipe.txt"
    $errPath = Join-Path $rawDir "$Label-stderr.txt"
    $outFs = [System.IO.File]::Create($pipePath)
    $errFs = [System.IO.File]::Create($errPath)
    $outCopy = $proc.StandardOutput.BaseStream.CopyToAsync($outFs)
    $errCopy = $proc.StandardError.BaseStream.CopyToAsync($errFs)
    $sampleFile = Join-Path $rawDir "$Label.jsonl"
    try {
        Wait-ProcessSamples -ProcessId ([int]$pidValue) -OutFile $sampleFile -PeriodMs 200 -MaxMs $TimeoutMs
        if (-not $proc.HasExited) {
            if (-not $proc.WaitForExit(5000)) {
                try { $proc.Kill() } catch {}
                throw "timeout: $Label"
            }
        }
        $outCopy.Wait()
        $errCopy.Wait()
    }
    finally {
        $outFs.Dispose()
        $errFs.Dispose()
    }
    $stdoutBytes = 0
    if (Test-Path -LiteralPath $pipePath) { $stdoutBytes = [int64](Get-Item -LiteralPath $pipePath).Length }
    $rows = Read-SampleJsonl $sampleFile
    $present = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        if ($row.present) { [void]$present.Add($row) }
    }
    $cpu = Get-CpuPercents $rows
    # --output already holds the machine document; do not overwrite it with redirected stdout.
    return [ordered]@{
        run          = $Run
        elapsed_ms   = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
        exit         = $proc.ExitCode
        pid          = $pidValue
        exe          = $proc.StartInfo.FileName
        peak_private = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object private -Maximum).Maximum } else { 0 }
        max_threads  = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object threads -Maximum).Maximum } else { 0 }
        cpu_p95      = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
        cpu_median   = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
        tree         = $tree
        stdout_bytes = $stdoutBytes
        stdout_file  = $pipePath
    }
}

function Get-CliRunSummary($Runs, [string]$Exe) {
    $elapsed = @($Runs | ForEach-Object { [double]$_.elapsed_ms })
    $priv = @($Runs | ForEach-Object { [double]$_.peak_private })
    $thr = @($Runs | ForEach-Object { [double]$_.max_threads })
    $cpuP95s = @($Runs | Where-Object { $null -ne $_.cpu_p95 } | ForEach-Object { [double]$_.cpu_p95 })
    return [ordered]@{
        exe            = $Exe
        runs           = $Runs
        median_elapsed = Get-Median $elapsed
        p95_elapsed    = Get-NearestRank $elapsed 0.95
        peak_private   = if ($priv) { ($priv | Measure-Object -Maximum).Maximum } else { 0 }
        max_threads    = if ($thr) { ($thr | Measure-Object -Maximum).Maximum } else { 0 }
        p95_cpu        = if ($cpuP95s) { Get-NearestRank $cpuP95s 0.95 } else { $null }
        exits          = @($Runs | ForEach-Object { $_.exit })
    }
}

# Alternating A/B, B/A pairs so ordering, cache state, and background drift hit
# both binaries equally. One discarded warm-up per binary precedes the window.
function Measure-CleanPair {
    param(
        [string]$BaselineExe,
        [string]$CandidateExe,
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 600000
    )
    foreach ($side in @('baseline', 'candidate')) {
        $exe = if ($side -eq 'baseline') { $BaselineExe } else { $CandidateExe }
        Write-Host "warmup clean $side (discarded)"
        $null = Measure-CliRun -Label "clean-$side-warmup" -Run 0 -Arguments $Arguments -LocalAppData $LocalAppData -TimeoutMs $TimeoutMs -Exe $exe
    }
    $baselineRuns = @()
    $candidateRuns = @()
    $order = @()
    for ($i = 1; $i -le 5; $i++) {
        $first = if (($i % 2) -eq 1) { 'baseline' } else { 'candidate' }
        $second = if ($first -eq 'baseline') { 'candidate' } else { 'baseline' }
        $order += "$first/$second"
        foreach ($side in @($first, $second)) {
            $exe = if ($side -eq 'baseline') { $BaselineExe } else { $CandidateExe }
            Write-Host "measure clean $side pair $i"
            $row = Measure-CliRun -Label "clean-$side-run$i" -Run $i -Arguments $Arguments -LocalAppData $LocalAppData -TimeoutMs $TimeoutMs -Exe $exe
            $row['side'] = $side
            if ($side -eq 'baseline') { $baselineRuns += $row } else { $candidateRuns += $row }
        }
    }
    return [ordered]@{
        order     = $order
        baseline  = (Get-CliRunSummary $baselineRuns $BaselineExe)
        candidate = (Get-CliRunSummary $candidateRuns $CandidateExe)
    }
}

function Measure-CliWorkload {
    param(
        [string]$Name,
        [string[]]$Arguments,
        [string]$LocalAppData,
        [int]$TimeoutMs = 300000
    )
    Write-Host "warmup $Name"
    $warmupOut = Join-Path $rawDir "$Name-warmup.json"
    $warmupArgs = if ($Arguments -contains '--output') { $Arguments } else { $Arguments + @('--output', $warmupOut) }
    $null = Invoke-Cli -Arguments $warmupArgs -LocalAppData $LocalAppData -TimeoutMs $TimeoutMs
    $runs = @()
    for ($i = 1; $i -le 5; $i++) {
        Write-Host "measure $Name #$i"
        $runs += Measure-CliRun -Label "$Name-run$i" -Run $i -Arguments $Arguments -LocalAppData $LocalAppData -TimeoutMs $TimeoutMs
    }
    $elapsed = @($runs | ForEach-Object { [double]$_.elapsed_ms })
    $priv = @($runs | ForEach-Object { [double]$_.peak_private })
    $thr = @($runs | ForEach-Object { [double]$_.max_threads })
    $cpuP95s = @($runs | Where-Object { $null -ne $_.cpu_p95 } | ForEach-Object { [double]$_.cpu_p95 })
    return [ordered]@{
        name            = $Name
        runs            = $runs
        median_elapsed  = Get-Median $elapsed
        p95_elapsed     = Get-NearestRank $elapsed 0.95
        peak_private    = if ($priv) { ($priv | Measure-Object -Maximum).Maximum } else { 0 }
        max_threads     = if ($thr) { ($thr | Measure-Object -Maximum).Maximum } else { 0 }
        p95_cpu         = if ($cpuP95s) { Get-NearestRank $cpuP95s 0.95 } else { $null }
        exits           = @($runs | ForEach-Object { $_.exit })
    }
}

function Ensure-AnalyzeFixture {
    $root = Join-Path $env:TEMP 'devsweep-five-mode-v1\analysis-250k-v1'
    $marker = Join-Path $root 'MANIFEST.json'
    if (Test-Path -LiteralPath $marker) {
        return $root
    }
    Write-Host "generating analysis-250k-v1 under $root"
    New-Item -ItemType Directory -Force -Path $root | Out-Null
    for ($dir = 0; $dir -lt 5000; $dir++) {
        $dirPath = Join-Path $root ('d{0:D4}' -f $dir)
        New-Item -ItemType Directory -Force -Path $dirPath | Out-Null
        for ($file = 0; $file -lt 49; $file++) {
            $path = Join-Path $dirPath ('f{0:D2}.dat' -f $file)
            [System.IO.File]::WriteAllBytes($path, [byte[]]@())
        }
        if (($dir % 250) -eq 0) { Write-Host "  dir $dir / 5000" }
    }
    ConvertTo-Utf8Json ([ordered]@{
            id          = 'analysis-250k-v1'
            directories = 5000
            files_each  = 49
            expected_nodes = 250000
        }) $marker
    return $root
}

function Write-DesktopDriver([string]$Path) {
    $source = @'
import { spawn } from "node:child_process";
import { writeFileSync, writeSync } from "node:fs";
import { createInterface } from "node:readline";

const action = process.argv[2];
const port = Number(process.argv[3] || "9588");
const exe = process.argv[4];
const profile = process.argv[5];
const scale = process.argv[6] || "";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function cdp() {
  const deadline = Date.now() + 120000;
  let targets = null;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json`);
      if (r.ok) {
        targets = await r.json();
        break;
      }
    } catch {}
    await sleep(400);
  }
  if (!targets) throw new Error("cdp timeout");
  const page = targets.find((t) => t.type === "page" && /tauri\\.localhost|analyze-render-benchmark|:4181/i.test(t.url || ""))
    ?? targets.find((t) => t.type === "page");
  if (!page) throw new Error("no page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("ws")); });
  let seq = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    }
  };
  const send = (method, params = {}) => {
    const id = ++seq;
    ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      setTimeout(() => { if (pending.has(id)) { pending.delete(id); reject(new Error("cdp timeout " + method)); } }, 30000);
    });
  };
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
    return result.result.value;
  };
  await send("Runtime.enable");
  await send("Page.enable");
  return { send, evaluate, ws };
}

if (action === "launch") {
  const extra = scale ? ` --force-device-scale-factor=${scale}` : "";
  const app = spawn(exe, [], {
    env: { ...process.env, LOCALAPPDATA: profile, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-allow-origins=*${extra}` },
    stdio: "ignore",
    cwd: process.cwd(),
    detached: true,
    windowsHide: false,
  });
  app.unref();
  writeFileSync(process.argv[7], JSON.stringify({ pid: app.pid, port }), "utf8");
  process.exit(0);
}

let statusJoinWatch = null;

async function dispatch(session, argv) {
  const actionName = argv[0];
  const arg = (index) => argv[index] ?? "";
  const statusOf = () => session.evaluate("document.querySelector('.status-mode') ? document.querySelector('.status-mode').getAttribute('data-status') : null");
  if (actionName === "statusStart") {
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/status'" });
    await sleep(800);
    // The mode captures one snapshot on enter, which disables the start
    // control. Wait for that operation to settle before every click attempt.
    const idleOf = async () => {
      for (let i = 0; i < 600; i += 1) {
        const st = await statusOf();
        if (st !== null && st !== "snapshot" && st !== "canceling" && st !== "live") return st;
        if (st === "live") return st;
        await sleep(50);
      }
      return await statusOf();
    };
    let startClicked = false;
    let live = false;
    for (let attempt = 0; attempt < 5 && !live; attempt += 1) {
      if ((await idleOf()) === "live") { live = true; break; }
      const clicked = await session.evaluate(`(() => {
        const button = document.querySelector(".status-toolbar .secondary-button");
        if (!button || button.disabled) return false;
        button.click();
        return true;
      })()`);
      startClicked = startClicked || clicked;
      for (let i = 0; i < 100; i += 1) {
        if ((await statusOf()) === "live") { live = true; break; }
        await sleep(50);
      }
    }
    return JSON.stringify({ startClicked, live, status: await statusOf() });
  }
  if (actionName === "statusStopRequest") {
    const before = await statusOf();
    const requestedAt = Date.now();
    const cancelClicked = await session.evaluate(`(() => {
      const button = document.querySelector(".status-toolbar .danger-button");
      if (button) button.click();
      return !!button;
    })()`);
    let acked = false;
    for (let i = 0; i < 600; i += 1) {
      if ((await statusOf()) === "canceling") { acked = true; break; }
      await sleep(10);
    }
    const ackAt = Date.now();
    // The release build exposes no coordinator handle to CDP, so the join is
    // watched here while the sampler holds the post-stop window open.
    statusJoinWatch = (async () => {
      const deadline = Date.now() + 20000;
      let status = null;
      while (Date.now() < deadline) {
        status = await statusOf();
        if (status === "ready" || status === "idle") {
          return { joined: true, status, join_ms: Date.now() - ackAt, joined_at: Date.now() };
        }
        await sleep(250);
      }
      return { joined: false, status, join_ms: null, joined_at: null };
    })();
    return JSON.stringify({
      before, cancelClicked, acked, request_ms: ackAt - requestedAt, requested_at: requestedAt, ack_at: ackAt, join_poll_ms: 250,
    });
  }
  if (actionName === "statusStopJoin") {
    if (!statusJoinWatch) return JSON.stringify({ joined: false, status: null, join_ms: null, joined_at: null });
    const row = await statusJoinWatch;
    statusJoinWatch = null;
    return JSON.stringify(row);
  }
  if (actionName === "hash") {
    await session.send("Runtime.evaluate", { expression: "location.hash = " + JSON.stringify(arg(2)) });
    await sleep(800);
    return "";
  }
  if (actionName === "click") {
    const pattern = arg(2);
    const started = Date.now();
    const clicked = await session.evaluate(`(() => {
      const re = new RegExp(${JSON.stringify(pattern)});
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    return JSON.stringify({ clicked, elapsed_ms: Date.now() - started });
  }
  if (actionName === "state") {
    const value = await session.evaluate(`(() => ({
      hash: location.hash,
      status: document.querySelector(".status-mode")?.getAttribute("data-status") ?? null,
      analyze: document.querySelector(".analyze-mode") ? document.querySelector(".analyze-mode").getAttribute("data-status") : null,
      text: (document.body.innerText || "").slice(0, 4000),
    }))()`);
    return JSON.stringify(value);
  }
  if (actionName === "fill") {
    const selector = arg(2);
    const value = arg(3);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${JSON.stringify(value)});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    return JSON.stringify({ filled });
  }
  if (actionName === "eval") {
    const value = await session.evaluate(arg(2));
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "waitEval") {
    const expression = arg(2);
    const deadline = Date.now() + Number(arg(3) || 180000);
    let value = null;
    while (Date.now() < deadline) {
      try { value = await session.evaluate(expression); } catch { value = null; }
      if (value) break;
      await sleep(500);
    }
    return typeof value === "string" ? value : JSON.stringify(value);
  }
  if (actionName === "analyzeCancel") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const clickNamed = async (source) => {
      const srcJson = JSON.stringify(source);
      return session.evaluate(`(() => {
        const re = new RegExp(${srcJson});
        const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
        if (button) button.click();
        return !!button;
      })()`);
    };
    const statusOf = () => session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    const startClicked = await clickNamed("Analyze path|分析路径");
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await statusOf();
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    const t0 = Date.now();
    const cancelClicked = await clickNamed("Cancel analysis|取消分析");
    let acked = false;
    let ackAt = null;
    let joined = false;
    let joinedAt = null;
    let status = null;
    for (let i = 0; i < 400; i += 1) {
      status = await statusOf();
      if (!acked && status === "canceling") {
        acked = true;
        ackAt = Date.now();
      }
      if (["canceled", "partial", "complete", "idle", "empty"].includes(status || "")) {
        joined = true;
        joinedAt = Date.now();
        break;
      }
      await sleep(10);
    }
    return JSON.stringify({
      filled,
      startClicked,
      cancelClicked,
      started,
      acked,
      joined,
      status,
      request_ms: ackAt === null ? null : ackAt - t0,
      ack_join_ms: ackAt === null || joinedAt === null ? null : joinedAt - ackAt,
      elapsed_ms: Date.now() - t0,
    });
  }
  if (actionName === "analyzeStart") {
    const rootJson = JSON.stringify(arg(2) || "");
    await session.send("Runtime.evaluate", { expression: "location.hash = '#/analyze'" });
    await sleep(800);
    const filled = await session.evaluate(`(() => {
      const el = document.querySelector(".analyze-root-input input");
      if (!el) return false;
      const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value");
      proto.set.call(el, ${rootJson});
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    })()`);
    const startClicked = await session.evaluate(`(() => {
      const re = /Analyze path|分析路径/;
      const button = Array.from(document.querySelectorAll("button")).find((b) => re.test(b.textContent || ""));
      if (button) button.click();
      return !!button;
    })()`);
    let started = false;
    for (let i = 0; i < 400; i += 1) {
      const st = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
      if (st === "loading") { started = true; break; }
      await sleep(20);
    }
    return JSON.stringify({ filled, startClicked, started });
  }
  if (actionName === "analyzePoll") {
    const status = await session.evaluate("document.querySelector('.analyze-mode') ? document.querySelector('.analyze-mode').getAttribute('data-status') : null");
    return JSON.stringify({ status });
  }
  if (actionName === "close") {
    try { await session.send("Browser.close"); } catch {}
    return "";
  }
  throw new Error("unknown action " + actionName);
}

if (action === "serve") {
  const session = await cdp();
  const ping = setInterval(() => { session.evaluate("void 0").catch(() => {}); }, 20000);
  writeSync(1, "{\"event\":\"ready\"}\n");
  const rl = createInterface({ input: process.stdin });
  for await (const line of rl) {
    if (!line.trim()) continue;
    let msg = {};
    try { msg = JSON.parse(line); } catch {
      writeSync(1, "{\"id\":\"\",\"error\":\"bad json\"}\n");
      continue;
    }
    try {
      if ((msg.argv || [])[0] === "close") {
        clearInterval(ping);
        await dispatch(session, msg.argv || ["close"]);
        writeSync(1, JSON.stringify({ id: msg.id || "", result: "" }) + "\n");
        process.exit(0);
      }
      const result = await dispatch(session, msg.argv || []);
      writeSync(1, JSON.stringify({ id: msg.id || "", result }) + "\n");
    } catch (error) {
      writeSync(1, JSON.stringify({ id: msg.id || "", error: String(error && error.message ? error.message : error) }) + "\n");
    }
  }
  process.exit(0);
}

const session = await cdp();
const output = await dispatch(session, process.argv.slice(2));
if (output) process.stdout.write(output);
session.ws.close();
'@
    [System.IO.File]::WriteAllText($Path, $source, [System.Text.UTF8Encoding]::new($false))
}

$nodeCmd = Get-Command node -ErrorAction SilentlyContinue
if ($nodeCmd) { $node = $nodeCmd.Source } else { $node = 'node' }

function Invoke-DesktopCdp {
    param([string[]]$Arguments)
    if ($null -eq $script:CdpServe -or $script:CdpServe.HasExited) {
        throw "cdp serve is not running"
    }
    $script:CdpSeq += 1
    $id = [string]$script:CdpSeq
    $payload = ([ordered]@{ id = $id; argv = @($Arguments) } | ConvertTo-Json -Compress -Depth 6)
    $script:CdpServe.StandardInput.WriteLine($payload)
    $script:CdpServe.StandardInput.Flush()
    $line = $script:CdpServe.StandardOutput.ReadLine()
    if ([string]::IsNullOrEmpty($line)) { throw "cdp serve closed: $Arguments" }
    $row = $line | ConvertFrom-Json
    if ($null -ne $row.PSObject.Properties['error'] -and $row.error) { throw [string]$row.error }
    if ($null -eq $row.PSObject.Properties['result'] -or $null -eq $row.result) { return "" }
    return [string]$row.result
}

function Invoke-DesktopDriver {
    param([string[]]$Arguments)
    if ($script:CdpServe -and -not $script:CdpServe.HasExited -and $Arguments.Count -gt 0 -and $Arguments[0] -ne 'launch') {
        return Invoke-DesktopCdp -Arguments $Arguments
    }
    $driver = Join-Path $OutDir 'desktop-driver.mjs'
    $last = 'desktop driver failed'
    $attempts = 4
    if ($Arguments.Count -gt 0 -and @('launch', 'close') -contains $Arguments[0]) { $attempts = 1 }
    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        $output = & $node $driver @Arguments
        if ($LASTEXITCODE -eq 0) { return $output }
        $last = "desktop driver failed (attempt $attempt): $Arguments"
        Write-Host $last
        if ($attempt -lt $attempts) { Start-Sleep -Seconds 3 }
    }
    throw $last
}

$script:CdpServe = $null
$script:CdpSeq = 0

# --- host manifest ---
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$cs = Get-CimInstance Win32_ComputerSystem
$power = powercfg /getactivescheme
$defender = $null
try { $defender = Get-MpComputerStatus | Select-Object AMServiceEnabled, AntivirusEnabled, RealTimeProtectionEnabled, NisEnabled } catch { $defender = @{ error = $_.Exception.Message } }
$integrity = (whoami /groups | Select-String 'Mandatory Label').Line
$commit = (git -C $RepoRoot rev-parse HEAD).Trim()
$rustc = (rustc -vV | Out-String).Trim()
$nodev = (& $node --version).Trim()
$locale = (Get-WinSystemLocale).Name
$cliSha = Get-FileSha256 $cliExe
$desktopSha = Get-FileSha256 $desktopExe
$consent = @(Get-Process -Name consent -ErrorAction SilentlyContinue).Count

$hostManifest = [ordered]@{
    protocol        = $Protocol
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    cpu_name        = $cpu.Name.Trim()
    logical_cores   = [int]$cs.NumberOfLogicalProcessors
    ram_bytes       = [int64]$os.TotalVisibleMemorySize * 1024
    arch            = $env:PROCESSOR_ARCHITECTURE
    windows_caption = $os.Caption
    windows_build   = $os.BuildNumber
    power_plan      = ($power | Out-String).Trim()
    rustc           = $rustc
    node            = $nodev
    commit          = $commit
    cli_exe         = $cliExe
    cli_sha256      = $cliSha
    desktop_exe     = $desktopExe
    desktop_sha256  = $desktopSha
    defender        = $defender
    locale          = $locale
    integrity       = $integrity
    consent_count   = $consent
    user            = $env:USERNAME
}
ConvertTo-Utf8Json $hostManifest (Join-Path $OutDir 'host-manifest.json')
Write-Host "host $($hostManifest.windows_build) cores=$($hostManifest.logical_cores) cli=$cliSha"

$gates = [ordered]@{}
$failures = New-Object System.Collections.Generic.List[string]
function Set-Gate([string]$Name, [bool]$Pass, [string]$Detail) {
    $gates[$Name] = [ordered]@{ pass = $Pass; detail = $Detail }
    if (-not $Pass) { [void]$failures.Add("$Name :: $Detail") }
}

Get-Process -Name devsweep-desktop, devsweep, SystemSettings -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

$localApp = Join-Path $OutDir 'localappdata'
New-Item -ItemType Directory -Force -Path $localApp | Out-Null

# --- CLI workloads (list is deferred until after idle/status/software/clean, before Analyze) ---
$optimizePlan = Join-Path $rawDir 'dns-plan.json'
$null = Invoke-Cli -Arguments @('optimize', 'plan', '--operation', 'dns.flush', '--output', $optimizePlan) -LocalAppData $localApp
$dnsPreviewOut = Join-Path $rawDir 'optimize-preview-dns.json'
$null = Invoke-Cli -Arguments @('optimize', 'preview', '--plan', $optimizePlan, '--format', 'json', '--output', $dnsPreviewOut) -LocalAppData $localApp
$previewJson = Get-Content -LiteralPath $dnsPreviewOut -Raw -Encoding UTF8 | ConvertFrom-Json
if ($null -eq $previewJson -or $null -eq $previewJson.PSObject.Properties['data'] -or $null -eq $previewJson.data.PSObject.Properties['digest']) {
    throw 'optimize preview output is missing data.digest'
}
$dnsDigest = [string]$previewJson.data.digest
$dnsExecute = Measure-CliWorkload -Name 'optimize-dns' -Arguments @('optimize', 'run', '--plan', $optimizePlan, '--preview-digest', $dnsDigest, '--confirm', '--format', 'json') -LocalAppData $localApp

$software = Measure-CliWorkload -Name 'software-inventory' -Arguments @('software', 'inventory', '--source', 'all', '--format', 'json') -LocalAppData $localApp -TimeoutMs 180000

$analyzeRoot = Ensure-AnalyzeFixture
$accounted = $null
$analyzeAccountedOut = Join-Path $rawDir 'analyze-250k-accounted.json'
Write-Host 'analyze accounted (CLI --output file only; not used for 512 MiB / 200% gates)'
$null = Invoke-Cli -Arguments @('analyze', 'scan', '--root', $analyzeRoot, '--format', 'json', '--output', $analyzeAccountedOut) -LocalAppData $localApp -TimeoutMs 600000
if (Test-Path -LiteralPath $analyzeAccountedOut) {
    try {
        $analyzeDoc = Get-Content -LiteralPath $analyzeAccountedOut -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($null -ne $analyzeDoc.PSObject.Properties['data'] -and $null -ne $analyzeDoc.data.PSObject.Properties['accounted_owned_bytes']) {
            $accounted = [int64]$analyzeDoc.data.accounted_owned_bytes
        }
    }
    catch { $accounted = $null }
}

$cleanRoot = $RepoRoot
Write-Host "counting clean-scan entries under live repo $cleanRoot"
$entryCount = (Get-ChildItem -LiteralPath $cleanRoot -Force -Recurse -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host "clean entries $entryCount"
$hostManifest['fixture_digests'] = [ordered]@{
    analysis_250k_v1_root            = $analyzeRoot
    analysis_250k_v1_manifest_sha256 = Get-FileSha256 (Join-Path $analyzeRoot 'MANIFEST.json')
    clean_scan_root                  = $cleanRoot
    clean_scan_entry_count           = $entryCount
    clean_scan_command               = 'clean scan --root <repo> --scope projects --format json'
    clean_comparability              = 'paired baseline and candidate runs on one host, one toolchain, one absolute root, and the projects scope; comparability is gated, not assumed'
}
$cleanBaselineDoc = $null
if ($CleanBaseline) {
    if (-not (Test-Path -LiteralPath $CleanBaseline)) { throw "missing clean baseline manifest: $CleanBaseline" }
    $cleanBaselineDoc = Get-Content -LiteralPath $CleanBaseline -Raw -Encoding UTF8 | ConvertFrom-Json
}
$cleanBaselineExe = if ($cleanBaselineDoc) { [string]$cleanBaselineDoc.exe } else { '' }
$cleanBaselineReady = [bool]($cleanBaselineExe -and (Test-Path -LiteralPath $cleanBaselineExe))
$hostManifest['clean_baseline'] = [ordered]@{
    manifest_path   = $CleanBaseline
    commit          = if ($cleanBaselineDoc) { [string]$cleanBaselineDoc.commit } else { '' }
    exe             = $cleanBaselineExe
    sha256_recorded = if ($cleanBaselineDoc) { [string]$cleanBaselineDoc.sha256 } else { '' }
    sha256_live     = if ($cleanBaselineReady) { Get-FileSha256 $cleanBaselineExe } else { '' }
    rustc           = if ($cleanBaselineDoc) { [string]$cleanBaselineDoc.rustc } else { '' }
    build_command   = if ($cleanBaselineDoc) { [string]$cleanBaselineDoc.build_command } else { '' }
}
ConvertTo-Utf8Json $hostManifest (Join-Path $OutDir 'host-manifest.json')
$cleanArgs = @('clean', 'scan', '--root', $cleanRoot, '--scope', 'projects', '--format', 'json')
$cleanBaselineResult = $null
$cleanRunOrder = @()
if ($cleanBaselineReady) {
    $cleanPair = Measure-CleanPair -BaselineExe $cleanBaselineExe -CandidateExe $cliExe -Arguments $cleanArgs -LocalAppData $localApp -TimeoutMs 600000
    $clean = $cleanPair.candidate
    $cleanBaselineResult = $cleanPair.baseline
    $cleanRunOrder = @($cleanPair.order)
}
else {
    Write-Host 'clean baseline binary is unavailable; the candidate still runs and every comparison gate fails'
    $clean = Measure-CliWorkload -Name 'clean-scan-projects' -Arguments $cleanArgs -LocalAppData $localApp -TimeoutMs 600000
}
Write-Host 'recounting clean-scan entries after the exclusive window'
$entryCountAfter = (Get-ChildItem -LiteralPath $cleanRoot -Force -Recurse -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host "clean entries after $entryCountAfter"

$settingsRuns = @()
foreach ($op in @('settings.storage_recommendations', 'settings.search', 'settings.energy_recommendations')) {
    $planPath = Join-Path $rawDir "$op.json"
    $null = Invoke-Cli -Arguments @('optimize', 'plan', '--operation', $op, '--output', $planPath) -LocalAppData $localApp
    $previewPath = Join-Path $rawDir "$op-preview.json"
    $null = Invoke-Cli -Arguments @('optimize', 'preview', '--plan', $planPath, '--format', 'json', '--output', $previewPath) -LocalAppData $localApp
    $previewDoc = Get-Content -LiteralPath $previewPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $digest = $null
    if ($null -ne $previewDoc.PSObject.Properties['data'] -and $null -ne $previewDoc.data.PSObject.Properties['digest']) {
        $digest = [string]$previewDoc.data.digest
    }
    if (-not $digest) { throw "missing digest for $op" }
    Write-Host "warmup $op"
    $null = Invoke-Cli -Arguments @('optimize', 'run', '--plan', $planPath, '--preview-digest', $digest, '--confirm', '--format', 'json') -LocalAppData $localApp
    Stop-NewSettingsProcesses @{}
    $opRuns = @()
    for ($i = 1; $i -le 5; $i++) {
        $beforeIds = @{}
        foreach ($existing in @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue)) {
            $beforeIds[[int]$existing.Id] = $true
        }
        $run = Invoke-Cli -Arguments @('optimize', 'run', '--plan', $planPath, '--preview-digest', $digest, '--confirm', '--format', 'json') -LocalAppData $localApp
        $after = @(Get-Process -Name SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue | Select-Object Id, ProcessName)
        Stop-NewSettingsProcesses $beforeIds
        $opRuns += [ordered]@{
            run         = $i
            elapsed_ms  = $run.elapsed_ms
            exit        = $run.exit
            clock       = 'devsweep_launch_return'
            os_settings = $after
        }
    }
    $settingsRuns += [ordered]@{ operation = $op; runs = $opRuns; p95_elapsed = (Get-NearestRank @($opRuns | ForEach-Object { [double]$_.elapsed_ms }) 0.95) }
}

Write-DesktopDriver (Join-Path $OutDir 'desktop-driver.mjs')

# --- Analyze layout p95 (Node, product treemap) ---
$layoutResult = $null
try {
    $layoutScript = Join-Path $OutDir 'analyze-layout-benchmark.mjs'
    $layoutSource = @'
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

const mod = await import(pathToFileURL(process.argv[2]).href);
const layoutTreemap = mod.layoutTreemap;
const children = Array.from({ length: 10000 }, (_, index) => ({
  id: index + 2,
  parent_id: 1,
  name: `wide-${index.toString().padStart(5, "0")}.bin`,
  kind: "file",
  bytes: (index % 97) + 1,
  evidence: "complete",
  completeness: "complete",
  immediate_count: 0,
}));
const fixtureHash = `sha256:${createHash("sha256").update(JSON.stringify(children)).digest("hex")}`;
const samples = [];
let rectangleCount = 0;
for (let index = 0; index < 35; index += 1) {
  const started = performance.now();
  const tiles = layoutTreemap(children, { width: 1200, height: 700 });
  const elapsed = performance.now() - started;
  rectangleCount = tiles.length;
  if (index >= 5) samples.push(elapsed);
}
const sorted = [...samples].sort((left, right) => left - right);
const rank = Math.ceil(0.95 * sorted.length);
const layoutP95 = sorted[rank - 1];
process.stdout.write(JSON.stringify({
  layout_p95_ms: layoutP95,
  nearest_rank: rank,
  layout_samples_ms: samples,
  rectangle_count: rectangleCount,
  fixture_children_sha256: fixtureHash,
  pass: rectangleCount <= 513 && layoutP95 <= 50,
}));
'@
    [System.IO.File]::WriteAllText($layoutScript, $layoutSource, [System.Text.UTF8Encoding]::new($false))
    $treemapPath = Join-Path $RepoRoot 'desktop\src\modes\analyze\treemap.ts'
    Write-Host 'analyze layout p95 (5 warm-up + 30 measured)'
    $layoutJson = & $node --experimental-strip-types $layoutScript $treemapPath
    if ($LASTEXITCODE -ne 0) { throw "layout benchmark exit $LASTEXITCODE" }
    $layoutResult = $layoutJson | ConvertFrom-Json
    ConvertTo-Utf8Json $layoutResult (Join-Path $rawDir 'analyze-layout.json')
}
catch {
    $layoutResult = [ordered]@{ pass = $false; error = $_.Exception.Message }
    ConvertTo-Utf8Json $layoutResult (Join-Path $rawDir 'analyze-layout.json')
}

# --- Analyze React commit p95 (product Vite bench + headless CDP, free port) ---
$commitResult = $null
$previewProc = $null
try {
    Write-Host 'analyze react-commit: building product Vite harness'
    Push-Location (Join-Path $RepoRoot 'desktop')
    try {
        & npm.cmd run benchmark:analyze:build
        if ($LASTEXITCODE -ne 0) { throw "benchmark:analyze:build exit $LASTEXITCODE" }
    }
    finally { Pop-Location }
    $previewOut = Join-Path $rawDir 'analyze-commit-preview.log'
    $previewErr = Join-Path $rawDir 'analyze-commit-preview.err'
    $previewProc = Start-Process -FilePath 'npm.cmd' -ArgumentList @('run', 'benchmark:analyze:preview') -WorkingDirectory (Join-Path $RepoRoot 'desktop') -PassThru -WindowStyle Hidden -RedirectStandardOutput $previewOut -RedirectStandardError $previewErr
    $previewReady = $false
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $tcp = New-Object System.Net.Sockets.TcpClient
            $tcp.Connect('127.0.0.1', 4181)
            $tcp.Close()
            $previewReady = $true
            break
        }
        catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $previewReady) { throw 'vite preview :4181 did not become ready' }
    $commitOut = Join-Path $rawDir 'analyze-react-commit.json'
    $commitLog = Join-Path $rawDir 'analyze-react-commit-cdp.jsonl'
    $commitDriver = Join-Path $PSScriptRoot 'run-analyze-react-commit.mjs'
    Write-Host 'analyze react-commit: headless Chrome/Edge CDP on a free port (not 9223)'
    & $node $commitDriver 'http://127.0.0.1:4181/analyze-render-benchmark.html' $commitOut $commitLog
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $commitOut)) {
        throw "react-commit driver exit $LASTEXITCODE"
    }
    $commitResult = Get-Content -LiteralPath $commitOut -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $commitResult) { throw 'react-commit result missing' }
    $sampleCount = @( $commitResult.react_commit_samples_ms ).Count
    if ($sampleCount -ne 30) { throw "react-commit expected 30 measured samples, got $sampleCount" }
}
catch {
    $commitResult = [ordered]@{ status = 'error'; react_commit_p95_ms = $null; error = $_.Exception.Message }
    ConvertTo-Utf8Json $commitResult (Join-Path $rawDir 'analyze-react-commit.json')
}
finally {
    if ($previewProc -and -not $previewProc.HasExited) {
        try { Stop-Process -Id $previewProc.Id -Force -ErrorAction SilentlyContinue } catch {}
        Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $previewProc.Id } | ForEach-Object {
            try { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue } catch {}
        }
    }
}

# --- Desktop idle (no remote-debug). Status is CLI, not WebView2 CDP. ---
Get-Process -Name devsweep-desktop, devsweep -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}
Start-Sleep -Seconds 2
$cancelRuns = @()
$cancelP95 = $null
$cdpPort = 9588

function Measure-IdleLike([string]$Label, [int]$PidValue, [int]$SettleSeconds, [int]$RetainSeconds) {
    Write-Host "settle $Label ${SettleSeconds}s"
    Start-Sleep -Seconds $SettleSeconds
    $file = Join-Path $rawDir "$Label.jsonl"
    Get-ProcessSamples -ProcessId ([int]$PidValue) -DurationMs ($RetainSeconds * 1000) -PeriodMs 200 -OutFile $file
    $rows = Read-SampleJsonl $file
    $present = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        if ($row.present) { [void]$present.Add($row) }
    }
    $cpu = Get-CpuPercents $rows
    return [ordered]@{
        samples      = Get-CollectionCount $rows
        present      = Get-CollectionCount $present
        cpu_median   = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
        cpu_p95      = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
        peak_private = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object private -Maximum).Maximum } else { 0 }
        max_threads  = if ((Get-CollectionCount $present) -gt 0) { ($present | Measure-Object threads -Maximum).Maximum } else { 0 }
        file         = $file
    }
}

$idleProfile = Join-Path $OutDir 'desktop-idle-localappdata'
New-Item -ItemType Directory -Force -Path $idleProfile | Out-Null
Write-Host 'launch desktop idle instance (no WEBVIEW2 remote-debug argument)'
$idleProc = Start-DesktopNoDebug $idleProfile
$idlePid = [int]$idleProc.Id
$script:IdlePid = $idlePid
Write-Host "idle desktop pid $idlePid"
Start-Sleep -Seconds 8
Write-Host 'warmup idle (unrecorded, no remote-debug)'
$null = Measure-IdleLike 'idle-warmup' $idlePid 30 60
$idleRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $idleRuns += Measure-IdleLike "idle-$i" $idlePid 30 60
}
$idlePresentRuns = @($idleRuns | Where-Object { [int]$_.present -gt 0 -and $null -ne $_.cpu_median })
if ((Get-CollectionCount $idlePresentRuns) -eq 0) { throw 'idle recorded no present samples' }
$idleMedianCpu = Get-Median @($idlePresentRuns | ForEach-Object { [double]$_.cpu_median })
$idleMaxPrivate = ($idlePresentRuns | ForEach-Object { [double]$_.peak_private } | Measure-Object -Maximum).Maximum
$idleMaxThreads = ($idlePresentRuns | ForEach-Object { [double]$_.max_threads } | Measure-Object -Maximum).Maximum
Write-Host "idle done medianCpu=$idleMedianCpu maxPrivate=$idleMaxPrivate maxThreads=$idleMaxThreads"
Stop-OwnedProcess -ProcessId $idlePid -ExpectedPath $desktopExe
Start-Sleep -Seconds 2

Write-Host 'status snapshot (CLI release; process-limit 15; no WebView2 remote-debug)'
$statusSnapshot = Measure-CliWorkload -Name 'status-snapshot' -Arguments @('status', 'snapshot', '--process-limit', '15', '--format', 'json') -LocalAppData $localApp
$snapshotLatencies = @($statusSnapshot.runs | ForEach-Object { [double]$_.elapsed_ms })
$snapshotPeaks = @($statusSnapshot.runs | ForEach-Object { [double]$_.peak_private })
$snapshotThreads = @($statusSnapshot.runs | ForEach-Object { [double]$_.max_threads })

function Measure-CliLiveRep([string]$Label) {
    Write-Host $Label
    $liveOut = Join-Path $rawDir "$Label-stdout.ndjson"
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = New-DevSweepStartInfo -Arguments @('status', 'live', '--interval', '2', '--process-limit', '15', '--format', 'ndjson', '--output', $liveOut) -LocalAppData $localApp
    [void]$proc.Start()
    $livePid = [int]$proc.Id
    $live = Measure-IdleLike $Label $livePid 30 60
    $live['pid'] = $livePid
    $live['window'] = 'CLI status live --interval 2 --process-limit 15 --format ndjson; no WebView2 remote-debug'
    if (-not $proc.HasExited) {
        try { $proc.CloseMainWindow() | Out-Null } catch {}
        if (-not $proc.WaitForExit(2000)) {
            Stop-OwnedProcess -ProcessId $livePid -ExpectedPath $cliExe
            if (-not $proc.WaitForExit(2000)) {
                try { $proc.Kill() } catch {}
            }
        }
    }
    $postFile = Join-Path $rawDir "$Label-poststop.jsonl"
    Get-ProcessSamples -ProcessId $livePid -DurationMs 5000 -PeriodMs 200 -OutFile $postFile
    $post = Read-SampleJsonl $postFile
    $postCount = Get-CollectionCount $post
    $finalHold = $postCount -eq 25
    if ($finalHold) {
        for ($idx = 20; $idx -le 24; $idx++) {
            $b = $post[$idx]
            $a = $post[$idx - 1]
            $cpuPct = 0.0
            $thr = 0
            $dt = [double]$b.wall_ms - [double]$a.wall_ms
            if ($dt -gt 0 -and $a.present -and $b.present) {
                $cpuPct = 100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt)
                $thr = [int]$b.threads
            }
            elseif ($b.present) {
                $thr = [int]$b.threads
            }
            if (-not (($cpuPct -le ($idleMedianCpu + 0.5)) -and ($thr -le ([int]$idleMaxThreads + 1)))) {
                $finalHold = $false
            }
        }
    }
    $live['post_samples'] = $postCount
    $live['post_hold'] = $finalHold
    $live['final_five'] = @($post | Select-Object -Last 5)
    return $live
}

Write-Host 'warmup status live (CLI, unrecorded)'
$null = Measure-CliLiveRep 'live-warmup'
$liveRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $liveRuns += Measure-CliLiveRep "live-$i"
}

Write-Host 'optimize list (closed eight; after idle/status/software/clean; before Analyze)'
$optimizeList = Measure-CliWorkload -Name 'optimize-list' -Arguments @('optimize', 'list', '--format', 'json') -LocalAppData $localApp
$listPreviewElapsed = @($optimizeList.runs | ForEach-Object { [double]$_.elapsed_ms })
$listPreviewPrivate = @($optimizeList.runs | ForEach-Object { [double]$_.peak_private })
$listPreviewThreads = @($optimizeList.runs | ForEach-Object { [double]$_.max_threads })
$catalogueCount = $null
$listDocPath = Join-Path $rawDir 'optimize-list-run1-stdout.json'
if (Test-Path -LiteralPath $listDocPath) {
    try {
        $listDoc = Get-Content -LiteralPath $listDocPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($null -ne $listDoc.PSObject.Properties['data'] -and $null -ne $listDoc.data.PSObject.Properties['entries']) {
            $catalogueCount = @( $listDoc.data.entries ).Count
        }
        elseif ($null -ne $listDoc.PSObject.Properties['data'] -and $null -ne $listDoc.data) {
            $maybe = $listDoc.data
            if ($maybe -is [System.Collections.IEnumerable] -and -not ($maybe -is [string])) {
                $catalogueCount = @($maybe).Count
            }
        }
    }
    catch { $catalogueCount = $null }
}
Write-Host "optimize list catalogue_count=$catalogueCount"

Write-Host 'analyze cancel-to-join then five desktop IPC Analyze-until-complete (main PID; CDP poll only)'
$desktopProfile = Join-Path $OutDir 'desktop-localappdata'
New-Item -ItemType Directory -Force -Path $desktopProfile | Out-Null
$launchMeta = Join-Path $OutDir 'desktop-launch.json'
Invoke-DesktopDriver @('launch', "$cdpPort", $desktopExe, $desktopProfile, '', $launchMeta) | Out-Null
$launchInfo = Get-Content -LiteralPath $launchMeta -Raw -Encoding UTF8 | ConvertFrom-Json
$desktopPid = [int]$launchInfo.pid
$script:DesktopPid = $desktopPid
Write-Host "cancel desktop pid $desktopPid"
Start-Sleep -Seconds 8
$hashOk = $false
for ($i = 0; $i -lt 8; $i++) {
    try {
        Invoke-DesktopDriver @('hash', "$cdpPort", '#/analyze') | Out-Null
        $hashOk = $true
        break
    }
    catch {
        Write-Host "cdp retry $($i + 1): $($_.Exception.Message)"
        Start-Sleep -Seconds 5
    }
}
if (-not $hashOk) { throw "desktop CDP never became ready on port $cdpPort" }
$driverPath = Join-Path $OutDir 'desktop-driver.mjs'
$serveInfo = New-Object System.Diagnostics.ProcessStartInfo
$serveInfo.FileName = $node
$serveInfo.Arguments = ('"{0}" serve {1}' -f $driverPath, $cdpPort)
$serveInfo.UseShellExecute = $false
$serveInfo.RedirectStandardInput = $true
$serveInfo.RedirectStandardOutput = $true
$serveInfo.RedirectStandardError = $false
$serveInfo.CreateNoWindow = $true
$serveInfo.WorkingDirectory = $RepoRoot
$script:CdpServe = New-Object System.Diagnostics.Process
$script:CdpServe.StartInfo = $serveInfo
[void]$script:CdpServe.Start()
$ready = $script:CdpServe.StandardOutput.ReadLine()
Write-Host "cdp serve pid $($script:CdpServe.Id) $ready"
if ($ready -notmatch 'ready') { throw "cdp serve failed: $ready" }

function Measure-AnalyzeCancel([string]$Label) {
    Write-Host $Label
    $json = Invoke-DesktopDriver @('analyzeCancel', "$cdpPort", $analyzeRoot)
    $row = $json | ConvertFrom-Json
    return [ordered]@{
        label       = $Label
        started     = [bool]$row.started
        acked       = [bool]$row.acked
        joined      = [bool]$row.joined
        request_ms  = $(if ($null -ne $row.request_ms) { [math]::Round([double]$row.request_ms, 3) } else { $null })
        ack_join_ms = $(if ($null -ne $row.ack_join_ms) { [math]::Round([double]$row.ack_join_ms, 3) } else { $null })
        elapsed_ms  = [math]::Round([double]$row.elapsed_ms, 3)
        status      = $(if ($null -ne $row.PSObject.Properties['status']) { [string]$row.status } else { $null })
        filled      = [bool]$row.filled
    }
}

Write-Host 'warmup analyze-cancel (unrecorded)'
$null = Measure-AnalyzeCancel 'analyze-cancel-warmup'
$cancelRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $cancelRuns += Measure-AnalyzeCancel "analyze-cancel-$i"
}
$cancelElapsed = @($cancelRuns | ForEach-Object { [double]$_.elapsed_ms })
$cancelP95 = Get-NearestRank $cancelElapsed 0.95
ConvertTo-Utf8Json ([ordered]@{ runs = $cancelRuns; p95_ms = $cancelP95 }) (Join-Path $rawDir 'analyze-cancel.json')

function Get-CdpThreadCount([int]$PidValue) {
    $n = 0
    try {
        $p = Get-Process -Id $PidValue -ErrorAction SilentlyContinue
        if ($p -and $p.Threads) { $n = [int]$p.Threads.Count }
    }
    catch { $n = 0 }
    return $n
}

function Measure-DesktopAnalyzeUntilComplete([string]$Label) {
    Write-Host $Label
    $startJson = Invoke-DesktopDriver @('analyzeStart', "$cdpPort", $analyzeRoot)
    $start = $startJson | ConvertFrom-Json
    $file = Join-Path $rawDir "$Label.jsonl"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    $finalStatus = $null
    $cdpThreads = New-Object System.Collections.ArrayList
    $MaxMs = 600000
    try {
        while ($sw.ElapsedMilliseconds -lt $MaxMs) {
            $now = $sw.ElapsedMilliseconds
            if ($now -ge $next) {
                $t = $sw.Elapsed.TotalMilliseconds
                [void]$samples.Add((Get-ProcessSnapshot -ProcessId ([int]$desktopPid) -WallMs $t))
                [void]$cdpThreads.Add((Get-CdpThreadCount ([int]$desktopPid)))
                $next += 200
                $poll = Invoke-DesktopDriver @('analyzePoll', "$cdpPort") | ConvertFrom-Json
                $finalStatus = $poll.status
                if ($samples.Count -gt 0) { $samples[$samples.Count - 1]['analyze_status'] = $finalStatus }
                if (@('partial', 'complete', 'canceled', 'error') -contains $finalStatus) { break }
            }
            else {
                $sleep = [Math]::Min([int64]15, [int64]($next - $now))
                if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
            }
        }
    }
    finally {
        Write-SampleJsonl $samples $file
    }
    $rows = Read-SampleJsonl $file
    $present = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        if ($row.present) { [void]$present.Add($row) }
    }
    $walkRows = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        $st = $null
        if ($null -ne $row.PSObject.Properties['analyze_status']) { $st = [string]$row.analyze_status }
        if ([string]::IsNullOrEmpty($st) -or ($st -eq 'loading') -or ($st -eq 'partial') -or ($st -eq 'complete')) {
            [void]$walkRows.Add($row)
        }
        if (@('partial', 'complete', 'canceled', 'error') -contains $st) { break }
    }
    $walkPresent = New-Object System.Collections.ArrayList
    foreach ($row in $walkRows) {
        if ($row.present) { [void]$walkPresent.Add($row) }
    }
    $cpu = Get-CpuPercents $walkRows
    $cdpMax = if ((Get-CollectionCount $cdpThreads) -gt 0) { ($cdpThreads | Measure-Object -Maximum).Maximum } else { 0 }
    return [ordered]@{
        label          = $Label
        started        = [bool]$start.started
        filled         = [bool]$start.filled
        startClicked   = [bool]$start.startClicked
        status         = $finalStatus
        elapsed_ms     = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)
        samples        = Get-CollectionCount $rows
        walk_samples   = Get-CollectionCount $walkRows
        peak_private   = if ((Get-CollectionCount $walkPresent) -gt 0) { ($walkPresent | Measure-Object private -Maximum).Maximum } else { 0 }
        max_threads    = if ((Get-CollectionCount $walkPresent) -gt 0) { ($walkPresent | Measure-Object threads -Maximum).Maximum } else { 0 }
        cdp_max_threads = $cdpMax
        cpu_p95        = if ((Get-CollectionCount $cpu) -gt 0) { Get-NearestRank @($cpu) 0.95 } else { $null }
        cpu_median     = if ((Get-CollectionCount $cpu) -gt 0) { Get-Median @($cpu) } else { $null }
        window         = 'desktop Analyze IPC until data-status=partial|complete (250k cap); Stopwatch wall_ms on walk samples only; main desktop PID'
        file           = $file
    }
}

Write-Host 'warmup desktop Analyze until complete (unrecorded)'
$null = Measure-DesktopAnalyzeUntilComplete 'analyze-250k-desktop-warmup'
$analyzeDesktopRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $analyzeDesktopRuns += Measure-DesktopAnalyzeUntilComplete "analyze-250k-desktop-$i"
}
$analyzeCpuP95s = @($analyzeDesktopRuns | Where-Object { $null -ne $_.cpu_p95 } | ForEach-Object { [double]$_.cpu_p95 })
$analyzePriv = @($analyzeDesktopRuns | ForEach-Object { [double]$_.peak_private })
$analyzeThr = @($analyzeDesktopRuns | ForEach-Object { [double]$_.max_threads })
$analyzeElapsed = @($analyzeDesktopRuns | ForEach-Object { [double]$_.elapsed_ms })
$allComplete = -not (@($analyzeDesktopRuns | Where-Object { @('partial', 'complete') -notcontains $_.status }).Count)
$analyze = [ordered]@{
    name            = 'analyze-250k-desktop-ipc-five'
    window          = 'five desktop IPC reps until data-status=partial|complete; Stopwatch CPU% on walk samples only; main PID; no CLI JSON for 512/200'
    runs            = $analyzeDesktopRuns
    peak_private    = if ($analyzePriv) { ($analyzePriv | Measure-Object -Maximum).Maximum } else { 0 }
    max_threads     = if ($analyzeThr) { ($analyzeThr | Measure-Object -Maximum).Maximum } else { 0 }
    p95_cpu         = if ($analyzeCpuP95s) { Get-NearestRank $analyzeCpuP95s 0.95 } else { $null }
    median_elapsed  = if ($analyzeElapsed) { Get-Median $analyzeElapsed } else { $null }
    p95_elapsed     = if ($analyzeElapsed) { Get-NearestRank $analyzeElapsed 0.95 } else { $null }
    all_complete    = $allComplete
}
ConvertTo-Utf8Json $analyze (Join-Path $rawDir 'analyze-250k-desktop.json')

function Get-ProcessExecutablePath([int]$ProcessId) {
    try { return [string](Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $ProcessId)).ExecutablePath }
    catch { return '' }
}

# Desktop quiescence. The stop is the UI control only: CloseMainWindow, Kill,
# and process exit never count, and the post-stop window samples the same live
# app PID that served the live stream.
function Measure-DesktopStatusStop([string]$Label) {
    Write-Host $Label
    # Baseline on this same CDP-attached instance, on the Status route and
    # before live starts. The separate no-remote-debug idle instance carries
    # neither remote-debugging threads nor prior Analyze work, so it is not a
    # thread comparator for this process.
    Invoke-DesktopDriver @('hash', "$cdpPort", '#/status') | Out-Null
    Start-Sleep -Seconds 3
    $baseFile = Join-Path $rawDir "$Label-prelive.jsonl"
    Get-ProcessSamples -ProcessId $desktopPid -DurationMs 5000 -PeriodMs 200 -OutFile $baseFile
    $baseRows = Get-SampleRows (Read-SampleJsonl $baseFile)
    $basePresent = @($baseRows | Where-Object { $_.present })
    if ((Get-CollectionCount $basePresent) -lt 5) { throw "$Label : the pre-live baseline did not observe the app PID" }
    $baseCpu = Get-CpuPercents $baseRows
    $baseMedianCpu = if ((Get-CollectionCount $baseCpu) -gt 0) { Get-Median @($baseCpu) } else { 0.0 }
    $baseMaxThreads = [int]($basePresent | Measure-Object threads -Maximum).Maximum
    $baseMinThreads = [int]($basePresent | Measure-Object threads -Minimum).Minimum
    $startRow = Invoke-DesktopDriver @('statusStart', "$cdpPort") | ConvertFrom-Json
    if (-not $startRow.live) { throw "$Label : desktop status live never started (status=$($startRow.status))" }
    $live = Measure-IdleLike "$Label-live" $desktopPid 30 60
    $exeBefore = Get-ProcessExecutablePath $desktopPid
    $stopRow = Invoke-DesktopDriver @('statusStopRequest', "$cdpPort") | ConvertFrom-Json
    $postFile = Join-Path $rawDir "$Label-poststop.jsonl"
    Get-ProcessSamples -ProcessId $desktopPid -DurationMs 5000 -PeriodMs 200 -OutFile $postFile
    $joinRow = Invoke-DesktopDriver @('statusStopJoin', "$cdpPort") | ConvertFrom-Json
    $exeAfter = Get-ProcessExecutablePath $desktopPid
    $post = Read-SampleJsonl $postFile
    $samplesOk = Test-PostStopSamples $post $desktopPid 25
    $postPresent = @((Get-SampleRows $post) | Where-Object { $_.present })
    $postFloor = $(if ((Get-CollectionCount $postPresent) -gt 0) { [int]($postPresent | Measure-Object threads -Minimum).Minimum } else { $null })
    $holdOk = Test-DesktopQuiescence $post $baseMedianCpu
    $identityOk = [bool]($exeBefore -and $exeAfter -and ($exeBefore -eq $exeAfter) -and ($exeBefore.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()))
    $joinMs = $(if ($null -ne $joinRow.join_ms) { [double]$joinRow.join_ms } else { $null })
    $joinInWindow = [bool]([bool]$joinRow.joined -and ($null -ne $joinMs) -and ($joinMs -le 5000.0))
    return [ordered]@{
        label            = $Label
        pid              = $desktopPid
        baseline_file    = $baseFile
        baseline_cpu_median = $baseMedianCpu
        baseline_max_threads = $baseMaxThreads
        baseline_min_threads = $baseMinThreads
        post_thread_floor = $postFloor
        live_thread_max = [int]$live.max_threads
        exe_before       = $exeBefore
        exe_after        = $exeAfter
        identity_ok      = $identityOk
        cancel_clicked   = [bool]$stopRow.cancelClicked
        acked            = [bool]$stopRow.acked
        request_ms       = [double]$stopRow.request_ms
        joined           = [bool]$joinRow.joined
        join_ms          = $joinMs
        join_in_window   = $joinInWindow
        join_poll_ms     = [int]$stopRow.join_poll_ms
        terminal_status  = [string]$joinRow.status
        post_samples     = Get-CollectionCount (Get-SampleRows $post)
        post_samples_ok  = $samplesOk
        post_hold        = $holdOk
        final_five       = @((Get-SampleRows $post) | Select-Object -Last 5)
        live             = $live
        post_file        = $postFile
        stop_control     = 'status-toolbar danger-button -> bridge.statusCancel -> status_cancel'
    }
}

Write-Host 'warmup desktop status stop (unrecorded)'
$null = Measure-DesktopStatusStop 'status-desktop-warmup'
$statusDesktopRuns = @()
for ($i = 1; $i -le 5; $i++) {
    $statusDesktopRuns += Measure-DesktopStatusStop "status-desktop-$i"
}
ConvertTo-Utf8Json ([ordered]@{ runs = $statusDesktopRuns }) (Join-Path $rawDir 'status-desktop-stop.json')
$statusDesktopSamplesOk = -not (@($statusDesktopRuns | Where-Object { -not ($_.post_samples_ok -and $_.identity_ok) }).Count)
$statusDesktopHoldOk = -not (@($statusDesktopRuns | Where-Object { -not $_.post_hold }).Count)
$statusDesktopFloors = @($statusDesktopRuns | ForEach-Object { $_.post_thread_floor })
$statusDesktopFloorOk = Test-DesktopFloorStable $statusDesktopFloors
$statusDesktopJoinOk = $statusDesktopFloorOk -and -not (@($statusDesktopRuns | Where-Object { -not ($_.acked -and $_.join_in_window -and $_.post_hold) }).Count)
$statusDesktopRequestMs = @($statusDesktopRuns | Where-Object { $null -ne $_.request_ms } | ForEach-Object { [double]$_.request_ms })
$statusDesktopJoinMs = @($statusDesktopRuns | Where-Object { $null -ne $_.join_ms } | ForEach-Object { [double]$_.join_ms })

try { Invoke-DesktopDriver @('close', "$cdpPort") | Out-Null } catch {}
if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
    try { Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue } catch {}
}
Start-Sleep -Seconds 2
Stop-OwnedProcess -ProcessId $desktopPid -ExpectedPath $desktopExe

# --- evaluate frozen thresholds ---
$MiB = 1024 * 1024
Set-Gate 'idle.median_cpu_le_1' ($idleMedianCpu -le 1.0) "medianCpu=$idleMedianCpu"
Set-Gate 'idle.max_private_le_192mib' ($idleMaxPrivate -le (192 * $MiB)) "maxPrivate=$idleMaxPrivate"
Set-Gate 'idle.max_threads_le_40' ($idleMaxThreads -le 40) "maxThreads=$idleMaxThreads"

$snapshotP95 = Get-NearestRank $snapshotLatencies 0.95
$snapshotPeak = if ($snapshotPeaks) { ($snapshotPeaks | Measure-Object -Maximum).Maximum } else { 0 }
$snapshotThr = if ($snapshotThreads) { ($snapshotThreads | Measure-Object -Maximum).Maximum } else { 0 }
Set-Gate 'status.snapshot.p95_le_2s' ($snapshotP95 -le 2000) "p95=$snapshotP95"
Set-Gate 'status.snapshot.private_le_idle_plus_64mib' ($snapshotPeak -le ($idleMaxPrivate + 64 * $MiB)) "peak=$snapshotPeak idle=$idleMaxPrivate"
Set-Gate 'status.snapshot.threads_le_idle_plus_4' ($snapshotThr -le ($idleMaxThreads + 4)) "threads=$snapshotThr idle=$idleMaxThreads"

$liveMedianCpu = Get-Median @($liveRuns | ForEach-Object { [double]$_.cpu_median })
$liveP95Cpu = Get-NearestRank @($liveRuns | ForEach-Object { [double]$_.cpu_p95 }) 0.95
$livePeak = ($liveRuns | ForEach-Object { [double]$_.peak_private } | Measure-Object -Maximum).Maximum
$liveThr = ($liveRuns | ForEach-Object { [double]$_.max_threads } | Measure-Object -Maximum).Maximum
$liveHold = -not (@($liveRuns | Where-Object { -not $_.post_hold }).Count)
$liveSamples = -not (@($liveRuns | Where-Object { $_.post_samples -ne 25 }).Count)
Set-Gate 'status.live.median_cpu_le_5' ($liveMedianCpu -le 5.0) "median=$liveMedianCpu"
Set-Gate 'status.live.p95_cpu_le_15' ($liveP95Cpu -le 15.0) "p95=$liveP95Cpu"
Set-Gate 'status.live.private_le_idle_plus_64mib' ($livePeak -le ($idleMaxPrivate + 64 * $MiB)) "peak=$livePeak"
Set-Gate 'status.live.threads_le_idle_plus_4' ($liveThr -le ($idleMaxThreads + 4)) "threads=$liveThr"
Set-Gate 'status.live.poststop_25_samples' $liveSamples 'each measured live rep has exactly 25 post-stop samples'
Set-Gate 'status.live.poststop_final_five_hold' $liveHold 'cli-exit evidence: final five consecutive samples meet idle+0.5pp CPU and idle+1 threads'

$statusDesktopRequestP95 = if ($statusDesktopRequestMs) { Get-NearestRank $statusDesktopRequestMs 0.95 } else { $null }
$statusDesktopJoinP95 = if ($statusDesktopJoinMs) { Get-NearestRank $statusDesktopJoinMs 0.95 } else { $null }
Set-Gate 'status.desktop.poststop_25_samples' $statusDesktopSamplesOk "every measured stop holds 25 present samples on the same live app PID with the same executable path; runs=$((Get-CollectionCount $statusDesktopRuns))"
Set-Gate 'status.desktop.poststop_final_five_hold' $statusDesktopHoldOk "final five consecutive samples meet baseline+0.5pp CPU, where the baseline is this same app instance on the Status route before live started; no per-rep thread clause (WebView2 pool threads decay over 1-2 min)"
Set-Gate 'status.desktop.stop_joined' $statusDesktopJoinOk "ack via data-status=canceling, terminal data-status inside the 5.0 s window, the CPU hold, and a post-stop thread floor that does not grow across the stops (floors=$($statusDesktopFloors -join ',')); request_ack_p95_ms=$statusDesktopRequestP95 ack_join_p95_ms=$statusDesktopJoinP95 join_poll_ms=250; the join ordering itself is proved by desktop/src-tauri/src/status.rs::run_live_returns_only_after_the_producer_thread_is_joined"

$csNow = Get-CimInstance Win32_ComputerSystem
$osNow = Get-CimInstance Win32_OperatingSystem
$cpuNow = Get-CimInstance Win32_Processor | Select-Object -First 1
$powerNow = (powercfg /getactivescheme | Out-String).Trim()
$cleanManifest = [pscustomobject]@{
    baseline_manifest           = $CleanBaseline
    baseline_commit             = [string]$hostManifest.clean_baseline.commit
    baseline_exe                = [string]$hostManifest.clean_baseline.exe
    baseline_sha256_recorded    = [string]$hostManifest.clean_baseline.sha256_recorded
    baseline_sha256_live        = [string]$hostManifest.clean_baseline.sha256_live
    baseline_rustc              = [string]$hostManifest.clean_baseline.rustc
    candidate_commit            = [string]$commit
    candidate_exe               = [string]$cliExe
    candidate_sha256            = [string]$cliSha
    candidate_rustc             = [string]$rustc
    scan_command                = 'clean scan --root <repo> --scope projects --format json'
    scan_root                   = (Resolve-Path -LiteralPath $cleanRoot).Path
    repo_root                   = (Resolve-Path -LiteralPath $RepoRoot).Path
    run_order                   = $cleanRunOrder
    entry_count_before          = $entryCount
    entry_count_after           = $entryCountAfter
    host_cpu_name_recorded      = [string]$hostManifest.cpu_name
    host_cpu_name_observed      = $cpuNow.Name.Trim()
    host_logical_cores_recorded = [int]$hostManifest.logical_cores
    host_logical_cores_observed = [int]$csNow.NumberOfLogicalProcessors
    host_windows_build_recorded = [string]$hostManifest.windows_build
    host_windows_build_observed = [string]$osNow.BuildNumber
    host_power_plan_recorded    = [string]$hostManifest.power_plan
    host_power_plan_observed    = $powerNow
}
$cleanComparable = Test-CleanComparable $cleanManifest
$cleanBaselineMedian = $null
$cleanBaselinePrivate = $null
$cleanBaselineThreads = $null
$cleanRatio = $null
if ($cleanBaselineResult) {
    $cleanBaselineMedian = [double]$cleanBaselineResult.median_elapsed
    $cleanBaselinePrivate = [double]$cleanBaselineResult.peak_private
    $cleanBaselineThreads = [int]$cleanBaselineResult.max_threads
    if ($cleanBaselineMedian -gt 0) { $cleanRatio = [double]$clean.median_elapsed / $cleanBaselineMedian }
}
$cleanDetail = "baseline_exe=$($cleanManifest.baseline_exe) sha_recorded=$($cleanManifest.baseline_sha256_recorded) sha_live=$($cleanManifest.baseline_sha256_live) rustc=$($cleanManifest.baseline_rustc) vs $($cleanManifest.candidate_rustc) root=$($cleanManifest.scan_root) vs $($cleanManifest.repo_root) entries=$entryCount to $entryCountAfter cores=$($cleanManifest.host_logical_cores_recorded) vs $($cleanManifest.host_logical_cores_observed) build=$($cleanManifest.host_windows_build_recorded) vs $($cleanManifest.host_windows_build_observed)"
Set-Gate 'clean.baseline_comparable' $cleanComparable $cleanDetail
Set-Gate 'clean.median_ratio_le_1_20' ($cleanComparable -and ($null -ne $cleanRatio) -and ($cleanRatio -le 1.20)) "ratio=$cleanRatio candidate_median=$($clean.median_elapsed) baseline_median=$cleanBaselineMedian order=$($cleanRunOrder -join ',') comparable=$cleanComparable"
Set-Gate 'clean.peak_private_le_baseline_plus_64mib' ($cleanComparable -and ($null -ne $cleanBaselinePrivate) -and ([double]$clean.peak_private -le ($cleanBaselinePrivate + 64 * $MiB))) "candidate_peak_private=$($clean.peak_private) baseline_peak_private=$cleanBaselinePrivate comparable=$cleanComparable"
Set-Gate 'clean.max_threads_le_baseline_plus_2' ($cleanComparable -and ($null -ne $cleanBaselineThreads) -and ([int]$clean.max_threads -le ($cleanBaselineThreads + 2))) "candidate_max_threads=$($clean.max_threads) baseline_max_threads=$cleanBaselineThreads comparable=$cleanComparable"

Set-Gate 'analyze.accounted_le_256mib' (($null -ne $accounted) -and ($accounted -le 268435456)) "accounted=$accounted"
Set-Gate 'analyze.peak_private_le_512mib' ($analyze.peak_private -le (512 * $MiB)) "peak=$($analyze.peak_private)"
Set-Gate 'analyze.p95_cpu_le_200' (($null -ne $analyze.p95_cpu) -and ($analyze.p95_cpu -le 200.0)) "p95cpu=$($analyze.p95_cpu) window=walk-until-partial-or-complete"
$layoutPass = $false
$layoutP95 = $null
if ($layoutResult -and ($layoutResult.PSObject.Properties.Name -contains 'layout_p95_ms')) {
    $layoutP95 = [double]$layoutResult.layout_p95_ms
    $layoutPass = [bool]$layoutResult.pass
}
Set-Gate 'analyze.layout_p95_le_50ms' $layoutPass "layout_p95_ms=$layoutP95"
$commitP95 = $null
$commitPass = $false
if ($commitResult -and ($commitResult.PSObject.Properties.Name -contains 'react_commit_p95_ms') -and ($null -ne $commitResult.react_commit_p95_ms)) {
    $commitP95 = [double]$commitResult.react_commit_p95_ms
    $commitPass = ($commitResult.status -eq 'pass') -and ($commitP95 -le 100.0)
}
Set-Gate 'analyze.react_commit_p95_le_100ms' $commitPass "react_commit_p95_ms=$commitP95 status=$($commitResult.status)"
$cancelJoined = -not (@($cancelRuns | Where-Object { -not $_.joined }).Count)
Set-Gate 'analyze.cancel_ack_p95_le_500ms' (($null -ne $cancelP95) -and ($cancelP95 -le 500.0) -and $cancelJoined) "p95=$cancelP95 joined=$cancelJoined"

Set-Gate 'software.p95_elapsed_le_5s' ($software.p95_elapsed -le 5000) "p95=$($software.p95_elapsed)"
Set-Gate 'software.private_le_idle_plus_128mib' ($software.peak_private -le ($idleMaxPrivate + 128 * $MiB)) "peak=$($software.peak_private)"
Set-Gate 'software.threads_le_idle_plus_6' ($software.max_threads -le ($idleMaxThreads + 6)) "threads=$($software.max_threads)"
Set-Gate 'software.no_uac' ($consent -eq @(Get-Process -Name consent -ErrorAction SilentlyContinue).Count) "consent still $consent"

$listPreviewP95 = Get-NearestRank $listPreviewElapsed 0.95
$listPreviewPeak = ($listPreviewPrivate | Measure-Object -Maximum).Maximum
$listPreviewThrMax = ($listPreviewThreads | Measure-Object -Maximum).Maximum
Set-Gate 'optimize.list_preview.p95_le_1s' ($listPreviewP95 -le 1000) "p95=$listPreviewP95"
Set-Gate 'optimize.list_preview.private_le_idle_plus_32mib' ($listPreviewPeak -le ($idleMaxPrivate + 32 * $MiB)) "peak=$listPreviewPeak"
Set-Gate 'optimize.list_preview.threads_le_idle_plus_2' ($listPreviewThrMax -le ($idleMaxThreads + 2)) "threads=$listPreviewThrMax"
Set-Gate 'optimize.dns.p95_le_10s' ($dnsExecute.p95_elapsed -le 10000) "p95=$($dnsExecute.p95_elapsed)"
foreach ($row in $settingsRuns) {
    Set-Gate ("optimize.{0}.launch_p95_le_2s" -f $row.operation) ($row.p95_elapsed -le 2000) "p95=$($row.p95_elapsed)"
}

$analyzeRequestMs = @($cancelRuns | Where-Object { $null -ne $_.request_ms } | ForEach-Object { [double]$_.request_ms })
$analyzeAckJoinMs = @($cancelRuns | Where-Object { $null -ne $_.ack_join_ms } | ForEach-Object { [double]$_.ack_join_ms })
# Rows the release sampler cannot reach carry no invented number. Their
# evidence source is named and the cause/fix/evidence record assigns status.
$operationTable = @(
    [ordered]@{
        operation          = 'Analyze'
        cancel             = 'cooperative'
        request            = 'bridge.analyzeCancel'
        acknowledgement    = '.analyze-mode[data-status] = canceling'
        join               = 'data-status in canceled/partial/complete/idle'
        bound              = 'ack p95 <= 500 ms over 5 runs, all joined'
        evidence_source    = 'release sampler: analyzeCancel driver action'
        request_ack_p95_ms = if ($analyzeRequestMs) { Get-NearestRank $analyzeRequestMs 0.95 } else { $null }
        ack_join_p95_ms    = if ($analyzeAckJoinMs) { Get-NearestRank $analyzeAckJoinMs 0.95 } else { $null }
        joined             = $cancelJoined
        gate               = 'analyze.cancel_ack_p95_le_500ms'
    }
    [ordered]@{
        operation          = 'Status live'
        cancel             = 'cooperative stop'
        request            = 'bridge.statusCancel'
        acknowledgement    = '.status-mode[data-status] = canceling'
        join               = 'data-status = ready/idle'
        bound              = 'joined; 25 same-PID post-stop samples; final-five hold'
        evidence_source    = 'release sampler: desktop status stop experiment'
        request_ack_p95_ms = $statusDesktopRequestP95
        ack_join_p95_ms    = $statusDesktopJoinP95
        joined             = $statusDesktopJoinOk
        gate               = 'status.desktop.stop_joined'
    }
    [ordered]@{
        operation          = 'Clean scan'
        cancel             = 'cooperative'
        request            = 'bridge.scanCancel'
        acknowledgement    = 'reducer scan_cancel_requested'
        join               = 'scan_canceled or terminal report'
        bound              = 'joined; latency recorded, no numeric bound'
        evidence_source    = 'focused coordinator and mode tests'
        request_ack_p95_ms = $null
        ack_join_p95_ms    = $null
        joined             = $null
        gate               = $null
    }
    [ordered]@{
        operation          = 'Clean dry-run / execute'
        cancel             = 'wait-only'
        request            = 'none'
        acknowledgement    = 'none'
        join               = 'planDryRun or planExecute result'
        bound              = 'completes before replacement or close; never force-killed'
        evidence_source    = 'focused tests'
        request_ack_p95_ms = $null
        ack_join_p95_ms    = $null
        joined             = $null
        gate               = $null
    }
    [ordered]@{
        operation          = 'Software inventory/preview/uninstall'
        cancel             = 'cooperative'
        request            = 'bridge.softwareCancel'
        acknowledgement    = 'reducer cancel_requested'
        join               = 'terminal result after finish'
        bound              = 'joined; latency recorded, no numeric bound'
        evidence_source    = 'focused tests'
        request_ack_p95_ms = $null
        ack_join_p95_ms    = $null
        joined             = $null
        gate               = $null
    }
    [ordered]@{
        operation          = 'Optimize list/preview/run'
        cancel             = 'cooperative'
        request            = 'bridge.optimizeCancel'
        acknowledgement    = 'reducer cancel_requested'
        join               = 'terminal result after finish'
        bound              = 'joined; latency recorded, no numeric bound'
        evidence_source    = 'focused tests'
        request_ack_p95_ms = $null
        ack_join_p95_ms    = $null
        joined             = $null
        gate               = $null
    }
    [ordered]@{
        operation          = 'Route change / window close'
        cancel             = 'coordinator cancelAndJoin or close'
        request            = 'shell route or close request'
        acknowledgement    = "active operation's cancel"
        join               = 'operation.join resolved'
        bound              = 'no owned work after join'
        evidence_source    = 'lifecycle tests'
        request_ack_p95_ms = $null
        ack_join_p95_ms    = $null
        joined             = $null
        gate               = $null
    }
)
ConvertTo-Utf8Json $operationTable (Join-Path $OutDir 'operation-table.json')

$summary = [ordered]@{
    protocol          = $Protocol
    host              = $hostManifest
    idle              = @{
        runs         = $idleRuns
        median_cpu   = $idleMedianCpu
        max_private  = $idleMaxPrivate
        max_threads  = $idleMaxThreads
        window       = 'release desktop with no WEBVIEW2 --remote-debugging-port; five 30s settle + 60s sample reps'
        process      = 'devsweep-desktop no remote-debug'
    }
    status_snapshot   = @{
        window        = 'CLI status snapshot --process-limit 15 --format json; five reps; no WebView2 remote-debug'
        latencies_ms  = $snapshotLatencies
        p95_ms        = $snapshotP95
        peak_private  = $snapshotPeak
        max_threads   = $snapshotThr
    }
    status_live       = @{
        evidence      = 'cli-exit'
        window        = 'CLI status live --interval 2 --process-limit 15 --format ndjson; 30s settle + 60s sample; post-stop 25/5.0s'
        runs          = $liveRuns
        median_cpu    = $liveMedianCpu
        p95_cpu       = $liveP95Cpu
        peak_private  = $livePeak
        max_threads   = $liveThr
    }
    status_desktop    = @{
        evidence            = 'desktop-ui-stop'
        window              = 'release desktop with CDP; per stop: 5s pre-live baseline on the Status route, UI start live, 30s settle, 60s sample on the app PID, UI stop, 25 scheduled 200 ms post-stop samples'
        runs                = $statusDesktopRuns
        request_ack_p95_ms  = $statusDesktopRequestP95
        ack_join_p95_ms     = $statusDesktopJoinP95
        join_ordering_proof = 'desktop/src-tauri/src/status.rs::run_live_returns_only_after_the_producer_thread_is_joined'
    }
    operation_table   = $operationTable
    clean             = @{
        window        = 'one discarded warm-up per binary, then five alternating baseline/candidate pairs of clean scan --root <repo> --scope projects --format json'
        candidate     = $clean
        baseline      = $cleanBaselineResult
        run_order     = $cleanRunOrder
        entry_count   = $entryCount
        entry_count_after = $entryCountAfter
        ratio         = $cleanRatio
        comparable    = $cleanComparable
        manifest      = $cleanManifest
    }
    analyze           = @{
        result                 = $analyze
        accounted_owned_bytes  = $accounted
        accounted_window       = 'CLI analyze scan --format json --output file; not used for 512 MiB / 200% gates'
        peak_cpu_window        = 'five desktop IPC Analyze reps on analysis-250k-v1 until data-status=partial|complete; Stopwatch wall_ms on walk samples only; no post-terminal idle'
        fixture                = $analyzeRoot
        layout                 = $layoutResult
        react_commit           = $commitResult
        cancel                 = @{ runs = $cancelRuns; p95_ms = $cancelP95 }
    }
    software          = $software
    optimize_list     = @{
        result           = $optimizeList
        catalogue_count  = $catalogueCount
        window           = 'five optimize list runs of the closed eight-entry catalogue; DNS preview is not merged into this p95; sampling starts at process create; stdout to --output file'
    }
    optimize_dns      = $dnsExecute
    optimize_settings = $settingsRuns
    gates             = $gates
    pass              = ($failures.Count -eq 0)
    failures          = @($failures)
}
ConvertTo-Utf8Json $summary (Join-Path $OutDir 'summary.json')
ConvertTo-Utf8Json $gates (Join-Path $OutDir 'gates.json')

if ($script:CdpServe -and -not $script:CdpServe.HasExited) {
    try { Stop-Process -Id $script:CdpServe.Id -Force -ErrorAction SilentlyContinue } catch {}
}
if ($script:IdlePid) { Stop-OwnedProcess -ProcessId ([int]$script:IdlePid) -ExpectedPath $desktopExe }
if ($script:DesktopPid) { Stop-OwnedProcess -ProcessId ([int]$script:DesktopPid) -ExpectedPath $desktopExe }
Get-Process -Name devsweep, devsweep-desktop -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $null
    try { $path = (Get-CimInstance Win32_Process -Filter ("ProcessId={0}" -f $_.Id)).ExecutablePath } catch { $path = $null }
    if ($path -and (($path.ToLowerInvariant() -eq $desktopExe.ToLowerInvariant()) -or ($path.ToLowerInvariant() -eq $cliExe.ToLowerInvariant()))) {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}
if ($failures.Count -gt 0) {
    Write-Error ("five-mode-v1 FAILED`n" + ($failures -join "`n"))
    exit 1
}
Write-Host 'five-mode-v1 PASS'
exit 0
