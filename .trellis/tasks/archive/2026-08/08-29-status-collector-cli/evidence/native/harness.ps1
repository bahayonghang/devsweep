$ErrorActionPreference = 'Stop'
$Native = Split-Path -Parent $MyInvocation.MyCommand.Path
$Repo = (Resolve-Path (Join-Path $Native '..\..\..\..\..')).Path
if (-not (Test-Path (Join-Path $Repo 'crates\devsweep-cli'))) {
    $Repo = 'D:\Documents\Code\Rust\Exp\devsweep'
}
$Exe = Join-Path $Repo 'target\release\devsweep.exe'
$SummaryPath = Join-Path $Native 'summary.json'
$LogPath = Join-Path $Native 'harness.log'

function Write-Log([string]$Message) {
    $line = '{0} {1}' -f ([DateTimeOffset]::Now.ToString('o')), $Message
    Add-Content -LiteralPath $LogPath -Value $line -Encoding utf8
    Write-Output $line
}

function Get-Pct($Values, [double]$P) {
    $list = @()
    if ($null -ne $Values) { $list = @($Values) }
    if ($list.Count -eq 0) { return $null }
    $sorted = @($list | Sort-Object { [double]$_ })
    $idx = [int][Math]::Ceiling($P * $sorted.Count) - 1
    if ($idx -lt 0) { $idx = 0 }
    if ($idx -ge $sorted.Count) { $idx = $sorted.Count - 1 }
    return [double]$sorted[$idx]
}

function Get-ProcSample {
    param([int]$Id, [datetime]$Origin)
    $t = ((Get-Date) - $Origin).TotalMilliseconds
    $p = Get-Process -Id $Id -ErrorAction SilentlyContinue
    if ($null -eq $p) {
        return [pscustomobject]@{
            wall_ms     = [math]::Round($t, 3)
            present     = $false
            user_ms     = 0.0
            kernel_ms   = 0.0
            cpu_ms      = 0.0
            private     = [int64]0
            threads     = 0
        }
    }
    return [pscustomobject]@{
        wall_ms     = [math]::Round($t, 3)
        present     = $true
        user_ms     = [math]::Round($p.UserProcessorTime.TotalMilliseconds, 3)
        kernel_ms   = [math]::Round($p.PrivilegedProcessorTime.TotalMilliseconds, 3)
        cpu_ms      = [math]::Round($p.TotalProcessorTime.TotalMilliseconds, 3)
        private     = [int64]$p.PrivateMemorySize64
        threads     = [int]$p.Threads.Count
    }
}

function Convert-CpuSeries {
    param($Samples)
    $pcts = New-Object System.Collections.Generic.List[double]
    for ($i = 1; $i -lt $Samples.Count; $i++) {
        $a = $Samples[$i - 1]
        $b = $Samples[$i]
        if (-not $a.present -or -not $b.present) { continue }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { continue }
        $dcpu = [double]$b.cpu_ms - [double]$a.cpu_ms
        [void]$pcts.Add((100.0 * $dcpu / $dt))
    }
    $out = New-Object double[] $pcts.Count
    if ($pcts.Count -gt 0) { $pcts.CopyTo($out) }
    return $out
}

function Sample-PidFor {
    param([int]$Id, [int]$DurationMs, [int]$PeriodMs)
    $origin = Get-Date
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $samples = New-Object System.Collections.Generic.List[object]
    $next = 0
    while ($sw.ElapsedMilliseconds -lt $DurationMs) {
        $now = $sw.ElapsedMilliseconds
        if ($now -ge $next) {
            [void]$samples.Add((Get-ProcSample -Id $Id -Origin $origin))
            $next += $PeriodMs
        } else {
            $sleep = [Math]::Min(15, $next - $now)
            if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
        }
    }
    $arr = New-Object object[] $samples.Count
    if ($samples.Count -gt 0) { $samples.CopyTo($arr) }
    return $arr
}

function Invoke-Devsweep {
    param([string[]]$Arguments)
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & $Exe @Arguments
    $code = $LASTEXITCODE
    $ErrorActionPreference = $saved
    return $code
}

function Start-Devsweep {
    param([string[]]$Arguments, [string]$OutFile, [string]$ErrFile)
    if (Test-Path $OutFile) { Remove-Item -LiteralPath $OutFile -Force }
    if (Test-Path $ErrFile) { Remove-Item -LiteralPath $ErrFile -Force }
    return Start-Process -FilePath $Exe -ArgumentList $Arguments -WorkingDirectory $Repo -NoNewWindow -PassThru `
        -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile
}

function Stop-PidQuiet([int]$Id) {
    $p = Get-Process -Id $Id -ErrorAction SilentlyContinue
    if ($null -ne $p) {
        Stop-Process -Id $Id -Force -ErrorAction SilentlyContinue
        try { Wait-Process -Id $Id -Timeout 8 -ErrorAction SilentlyContinue } catch { }
    }
}

if (Test-Path $LogPath) { Remove-Item -LiteralPath $LogPath -Force }
Write-Log "repo=$Repo"
Write-Log "exe=$Exe"
if (-not (Test-Path $Exe)) { throw "release binary missing: $Exe" }

$os = Get-CimInstance Win32_OperatingSystem
$integrity = (whoami /groups | Select-String 'Mandatory Label')
Write-Log ("os={0} build={1}" -f $os.Caption, $os.BuildNumber)
Write-Log ("integrity={0}" -f ($integrity -join '; '))
$consent = Get-Process -Name consent -ErrorAction SilentlyContinue
Write-Log ("consent.exe_count={0}" -f @($consent).Count)

$hash = (Get-FileHash -LiteralPath $Exe -Algorithm SHA256).Hash
$bytes = [IO.File]::ReadAllBytes($Exe)
$peOff = [BitConverter]::ToInt32($bytes, 60)
$machine = [BitConverter]::ToUInt16($bytes, $peOff + 4)
$peLabel = switch ($machine) {
    0x8664 { 'amd64 (0x8664)' }
    0x14c { 'i386 (0x14c)' }
    default { "machine=0x{0:X}" -f $machine }
}
Write-Log "sha256=$hash pe=$peLabel"

Get-Process -Name devsweep -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe } | ForEach-Object {
    Write-Log ("stopping leftover pid={0}" -f $_.Id)
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Milliseconds 300

# --- Warm-up snapshot ---
$warmupOut = Join-Path $Native 'warmup-snapshot.json'
$warmupErr = Join-Path $Native 'warmup-snapshot.stderr.log'
if (Test-Path $warmupOut) { Remove-Item -LiteralPath $warmupOut -Force }
$sw = [Diagnostics.Stopwatch]::StartNew()
$warmupExit = Invoke-Devsweep @('status','snapshot','--format','json','--output',$warmupOut)
$warmupMs = $sw.ElapsedMilliseconds
Write-Log "warmup_snapshot exit=$warmupExit latency_ms=$warmupMs"

# --- Five timed snapshots ---
$snapshotRows = @()
for ($i = 1; $i -le 5; $i++) {
    $out = Join-Path $Native ("snapshot-{0}.json" -f $i)
    $err = Join-Path $Native ("snapshot-{0}.stderr.log" -f $i)
    if (Test-Path $out) { Remove-Item -LiteralPath $out -Force }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $exit = Invoke-Devsweep @('status','snapshot','--format','json','--output',$out)
    $ms = $sw.ElapsedMilliseconds
    $doc = Get-Content -LiteralPath $out -Raw -Encoding utf8 | ConvertFrom-Json
    $cpuState = $doc.data.cpu.state
    $power = $doc.data.power
    $procs = $doc.data.processes
    $powerValue = $null
    $procValue = $null
    if ($null -ne $power.PSObject.Properties['value']) { $powerValue = $power.value }
    if ($null -ne $procs.PSObject.Properties['value']) { $procValue = $procs.value }
    $row = [ordered]@{
        index              = $i
        exit               = $exit
        latency_ms         = $ms
        outcome            = $doc.outcome
        cpu_state          = $cpuState
        memory_state       = $doc.data.memory.state
        volumes_state      = $doc.data.volumes.state
        network_state      = $doc.data.network.state
        power_state        = $power.state
        battery_present    = $(if ($null -ne $powerValue) { $powerValue.battery_present } else { $null })
        processes_state    = $procs.state
        enumerated_count   = $(if ($null -ne $procValue) { $procValue.enumerated_count } else { $null })
        returned_count     = $(if ($null -ne $procValue) { $procValue.returned_count } else { $null })
        requested_limit    = $(if ($null -ne $procValue) { $procValue.requested_limit } else { $null })
        truncated_by_limit = $(if ($null -ne $procValue) { $procValue.truncated_by_limit } else { $null })
        budget_exhausted   = $(if ($null -ne $procValue) { $procValue.budget_exhausted } else { $null })
        unsupported        = @($doc.data.unsupported_capabilities | ForEach-Object { $_.code })
        sample_window_ms   = $doc.data.sample_window_ms
        logical_processors = $doc.data.logical_processor_count
    }
    $snapshotRows += $row
    Write-Log ("snapshot[{0}] exit={1} latency_ms={2} outcome={3} cpu={4} power={5} battery={6} enumerated={7} truncated={8}" -f `
        $i, $exit, $ms, $doc.outcome, $cpuState, $power.state, $row.battery_present, $row.enumerated_count, $row.truncated_by_limit)
}

$snapshotLatencies = @($snapshotRows | ForEach-Object { [double]$_.latency_ms })
$snapshotP95 = Get-Pct $snapshotLatencies 0.95

# --- Idle baseline: live interval 60 after first sample, mostly sleeping ---
$idleOut = Join-Path $Native 'idle-live.ndjson'
$idleErr = Join-Path $Native 'idle-live.stderr.log'
$idleProc = Start-Devsweep -Arguments @('status','live','--format','ndjson','--interval','60') -OutFile $idleOut -ErrFile $idleErr
Start-Sleep -Milliseconds 2500
$idleSamples = Sample-PidFor -Id $idleProc.Id -DurationMs 2000 -PeriodMs 200
$idlePrivate = @($idleSamples | Where-Object present | ForEach-Object { [int64]$_.private })
$idleThreads = @($idleSamples | Where-Object present | ForEach-Object { [int]$_.threads })
$idleCpu = Convert-CpuSeries $idleSamples
$idlePrivateMedian = if ($idlePrivate.Count) { Get-Pct ([double[]]$idlePrivate) 0.5 } else { 0 }
$idleThreadsMedian = if ($idleThreads.Count) { Get-Pct ([double[]]$idleThreads) 0.5 } else { 0 }
$idleCpuMedian = if ($idleCpu.Count) { Get-Pct $idleCpu 0.5 } else { 0 }
Write-Log ("idle_pid={0} private_median={1} threads_median={2} cpu_median_pct={3} samples={4}" -f `
    $idleProc.Id, $idlePrivateMedian, $idleThreadsMedian, [math]::Round($idleCpuMedian, 3), $idleSamples.Count)
Stop-PidQuiet $idleProc.Id

# --- 60-second live at default 2 s, 200 ms process sampling ---
$liveOut = Join-Path $Native 'live-60s.ndjson'
$liveErr = Join-Path $Native 'live-60s.stderr.log'
$liveProc = Start-Devsweep -Arguments @('status','live','--format','ndjson','--interval','2') -OutFile $liveOut -ErrFile $liveErr
$livePid = $liveProc.Id
Write-Log "live60_start pid=$livePid"
$liveSamples = Sample-PidFor -Id $livePid -DurationMs 60000 -PeriodMs 200
$liveStillRunning = -not $liveProc.HasExited
Stop-PidQuiet $livePid
$liveExitObserved = $liveProc.HasExited
try { $liveExitCode = $liveProc.ExitCode } catch { $liveExitCode = $null }
Write-Log "live60_stop running_before_stop=$liveStillRunning has_exited=$liveExitObserved exit=$liveExitCode samples=$($liveSamples.Count)"

$liveCpu = Convert-CpuSeries $liveSamples
$livePrivate = @($liveSamples | Where-Object present | ForEach-Object { [int64]$_.private })
$liveThreads = @($liveSamples | Where-Object present | ForEach-Object { [int]$_.threads })
$liveCpuMedian = if ($liveCpu.Count) { Get-Pct $liveCpu 0.5 } else { $null }
$liveCpuP95 = if ($liveCpu.Count) { Get-Pct $liveCpu 0.95 } else { $null }
$livePrivatePeak = if ($livePrivate.Count) { ($livePrivate | Measure-Object -Maximum).Maximum } else { 0 }
$liveThreadsPeak = if ($liveThreads.Count) { ($liveThreads | Measure-Object -Maximum).Maximum } else { 0 }
$livePresent = @($liveSamples | Where-Object present).Count
Write-Log ("live60 cpu_median={0} cpu_p95={1} private_peak={2} threads_peak={3} present={4}/{5}" -f `
    [math]::Round($liveCpuMedian, 3), [math]::Round($liveCpuP95, 3), $livePrivatePeak, $liveThreadsPeak, $livePresent, $liveSamples.Count)

# Post-exit 25 x 200 ms
$postSamples = Sample-PidFor -Id $livePid -DurationMs 5000 -PeriodMs 200
if ($postSamples.Count -gt 25) { $postSamples = $postSamples[0..24] }
while ($postSamples.Count -lt 25) {
    $postSamples += Get-ProcSample -Id $livePid -Origin (Get-Date)
}
$postCpu = Convert-CpuSeries $postSamples
$finalFive = @($postSamples | Select-Object -Last 5)
$finalFiveHold = $true
foreach ($s in $finalFive) {
    $cpuHold = 0.0
    if ($s.present -and $postCpu.Count) { $cpuHold = $postCpu[-1] }
    if ($s.present) { $finalFiveHold = $false }
    if ($s.threads -gt ([int]$idleThreadsMedian + 1)) { $finalFiveHold = $false }
}
$leftover = @(Get-Process -Name devsweep -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe })
Write-Log ("post_exit samples={0} leftover_same_exe={1} final_five_absent={2}" -f $postSamples.Count, $leftover.Count, $finalFiveHold)

# Parse live NDJSON for skipped ticks / events
$events = @()
$skipped = 0
if (Test-Path $liveOut) {
    Get-Content -LiteralPath $liveOut -Encoding utf8 | ForEach-Object {
        if ($_.Trim().Length -eq 0) { return }
        try {
            $ev = $_ | ConvertFrom-Json
            $events += $ev
            if ($ev.event -eq 'tick_skipped') { $skipped = [int]$ev.data.skipped_total }
        } catch { }
    }
}
$eventNames = @($events | ForEach-Object { $_.event })
Write-Log ("live60 events={0} skipped_total={1} kinds={2}" -f $events.Count, $skipped, ($eventNames -join ','))

# --- Interval bounds 1 and 60 ---
function Read-StartedInterval([string[]]$Arguments, [string]$Tag) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.Arguments = ($Arguments -join ' ')
    $psi.WorkingDirectory = $Repo
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $line = $proc.StandardOutput.ReadLine()
    $interval = $null
    if ($line) {
        try { $interval = ($line | ConvertFrom-Json).data.interval_ms } catch { }
    }
    $proc.StandardOutput.Close()
    if (-not $proc.WaitForExit(15000)) {
        $proc.Kill()
        [void]$proc.WaitForExit(5000)
    }
    Write-Log ("{0} started_interval_ms={1} exit={2} line_prefix={3}" -f $Tag, $interval, $proc.ExitCode, ($(if ($line) { $line.Substring(0, [Math]::Min(80, $line.Length)) } else { '' })))
    return [ordered]@{ tag = $Tag; interval_ms = $interval; exit = $proc.ExitCode }
}

$bound1 = Read-StartedInterval @('status','live','--format','ndjson','--interval','1') 'interval_1s'
$bound60 = Read-StartedInterval @('status','live','--format','ndjson','--interval','60') 'interval_60s'

$invalid0 = Invoke-Devsweep @('status','live','--format','ndjson','--interval','0')
$invalid61 = Invoke-Devsweep @('status','live','--format','ndjson','--interval','61')
Write-Log "interval_invalid 0=>$invalid0 61=>$invalid61"

# --- Broken pipe ---
$pipe = Read-StartedInterval @('status','live','--format','ndjson','--interval','1') 'broken_pipe'

# --- Two-process overlap (in-process coordinator is unit-tested; cross-process is expected to run) ---
$c1Out = Join-Path $Native 'coordinator-a.ndjson'
$c2Out = Join-Path $Native 'coordinator-b.ndjson'
$c1 = Start-Devsweep -Arguments @('status','live','--format','ndjson','--interval','2') -OutFile $c1Out -ErrFile (Join-Path $Native 'coordinator-a.stderr.log')
Start-Sleep -Milliseconds 400
$c2 = Start-Devsweep -Arguments @('status','live','--format','ndjson','--interval','2') -OutFile $c2Out -ErrFile (Join-Path $Native 'coordinator-b.stderr.log')
Start-Sleep -Milliseconds 1500
$c1Alive = -not $c1.HasExited
$c2Alive = -not $c2.HasExited
Write-Log "two_process_live a_alive=$c1Alive b_alive=$c2Alive (in-process permit only; both OS processes may run)"
Stop-PidQuiet $c1.Id
Stop-PidQuiet $c2.Id

# --- No leftover collector after a short joined pipe-close ---
Start-Sleep -Milliseconds 500
$leftoverAfter = @(Get-Process -Name devsweep -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe })
Write-Log ("leftover_after_all leftover={0}" -f $leftoverAfter.Count)

$idlePlus64 = [int64]$idlePrivateMedian + 64MB
$idlePlus4 = [int]$idleThreadsMedian + 4
$gates = [ordered]@{
    snapshot_p95_le_2000ms          = ($snapshotP95 -le 2000)
    live_private_le_idle_plus_64mib = ($livePrivatePeak -le $idlePlus64)
    live_threads_le_idle_plus_4     = ($liveThreadsPeak -le $idlePlus4)
    live_cpu_median_le_5            = ($liveCpuMedian -le 5.0)
    live_cpu_p95_le_15              = ($liveCpuP95 -le 15.0)
    post_exit_25_samples            = ($postSamples.Count -eq 25)
    post_exit_final_five_hold       = [bool]$finalFiveHold
    interval_1s_wire_1000           = ($bound1.interval_ms -eq 1000 -and $bound1.exit -eq 0)
    interval_60s_wire_60000         = ($bound60.interval_ms -eq 60000 -and $bound60.exit -eq 0)
    interval_0_exit_2               = ($invalid0 -eq 2)
    interval_61_exit_2              = ($invalid61 -eq 2)
    broken_pipe_exit_0              = ($pipe.exit -eq 0)
    no_leftover_release_exe         = ($leftoverAfter.Count -eq 0)
}

$summary = [ordered]@{
    host = [ordered]@{
        os         = $os.Caption
        build      = $os.BuildNumber
        integrity  = "$integrity"
        consent    = @($consent).Count
        sha256     = $hash
        pe         = $peLabel
        exe        = $Exe
    }
    warmup = [ordered]@{ exit = $warmupExit; latency_ms = $warmupMs }
    snapshots = $snapshotRows
    snapshot_latency_ms = $snapshotLatencies
    snapshot_p95_ms = $snapshotP95
    idle = [ordered]@{
        private_median = $idlePrivateMedian
        threads_median = $idleThreadsMedian
        cpu_median_pct = $idleCpuMedian
        sample_count   = $idleSamples.Count
    }
    live60 = [ordered]@{
        pid            = $livePid
        sample_count   = $liveSamples.Count
        present_count  = $livePresent
        cpu_median_pct = $liveCpuMedian
        cpu_p95_pct    = $liveCpuP95
        private_peak   = $livePrivatePeak
        threads_peak   = $liveThreadsPeak
        skipped_total  = $skipped
        event_count    = $events.Count
        event_kinds    = $eventNames
        cpu_series     = @($liveCpu | ForEach-Object { [math]::Round($_, 4) })
        raw_samples    = $liveSamples
    }
    post_exit = [ordered]@{
        sample_count = $postSamples.Count
        leftover     = $leftoverAfter.Count
        final_five_process_absent = [bool]$finalFiveHold
        samples      = $postSamples
    }
    bounds = [ordered]@{ interval_1 = $bound1; interval_60 = $bound60; invalid_0 = $invalid0; invalid_61 = $invalid61; broken_pipe = $pipe }
    two_process_overlap = [ordered]@{ a_alive = $c1Alive; b_alive = $c2Alive }
    gates = $gates
}

($summary | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $SummaryPath -Encoding utf8
Write-Log ("summary_written gates={0}" -f (($gates.GetEnumerator() | ForEach-Object { '{0}={1}' -f $_.Key, $_.Value }) -join ' '))
Write-Log 'harness complete'
exit 0
