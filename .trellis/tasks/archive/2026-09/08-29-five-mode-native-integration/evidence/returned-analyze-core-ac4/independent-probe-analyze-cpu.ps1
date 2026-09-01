#requires -Version 5.1
# Independent native analyze scan CPU recapture for five-mode AC4.
# Does not overwrite implementer native-cpu-summary.json.
# CPU% = delta TotalProcessorTime / Stopwatch wall * 100 (100% = one logical core).
# Nearest-rank p95: ceil(0.95 * n) - 1. Stop at process exit. No 600 s pad.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..\..\..\..')).Path
$OutDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RawDir = Join-Path $OutDir ('independent-cpu-raw-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ'))
New-Item -ItemType Directory -Force -Path $RawDir | Out-Null
$cliExe = Join-Path $RepoRoot 'target\release\devsweep.exe'
if (-not (Test-Path -LiteralPath $cliExe)) {
    throw "missing release CLI: $cliExe"
}

function Get-NearestRank([double[]]$Values, [double]$P) {
    $sorted = @($Values | Sort-Object)
    $n = @($sorted).Count
    if ($n -eq 0) { return $null }
    $index = [Math]::Max(0, [Math]::Ceiling($P * $n) - 1)
    if ($index -ge $n) { $index = $n - 1 }
    return [double]$sorted[$index]
}

function Get-Median([double[]]$Values) {
    return Get-NearestRank $Values 0.5
}

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
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
    $manifest = @{
        id             = 'analysis-250k-v1'
        directories    = 5000
        files_each     = 49
        expected_nodes = 250000
    } | ConvertTo-Json
    [System.IO.File]::WriteAllText($marker, $manifest, [System.Text.UTF8Encoding]::new($false))
    return $root
}

function Get-CpuPercents($Samples) {
    $pcts = New-Object System.Collections.Generic.List[double]
    for ($i = 1; $i -lt @($Samples).Count; $i++) {
        $a = $Samples[$i - 1]
        $b = $Samples[$i]
        if (-not $a.present -or -not $b.present) { continue }
        $dt = [double]$b.wall_ms - [double]$a.wall_ms
        if ($dt -le 0) { continue }
        [void]$pcts.Add(100.0 * (([double]$b.cpu_ms - [double]$a.cpu_ms) / $dt))
    }
    return , $pcts
}

function Invoke-AnalyzeSampled([string]$Label, [string]$Root) {
    $outFile = Join-Path $RawDir "$Label-output.json"
    $sampleFile = Join-Path $RawDir "$Label.jsonl"
    $errFile = Join-Path $RawDir "$Label-stderr.txt"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $cliExe
    $psi.Arguments = "analyze scan --root `"$Root`" --format json --output `"$outFile`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = $RepoRoot
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    $started = [Diagnostics.Stopwatch]::StartNew()
    [void]$proc.Start()
    $pidValue = $proc.Id
    $errCopy = $proc.StandardError.ReadToEndAsync()
    $outCopy = $proc.StandardOutput.ReadToEndAsync()
    $samples = New-Object System.Collections.ArrayList
    $next = 0
    $seen = $false
    while (-not $proc.HasExited) {
        $now = $started.ElapsedMilliseconds
        if ($now -ge $next) {
            $row = $null
            $alive = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
            if ($null -eq $alive) {
                $row = [ordered]@{
                    wall_ms = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
                    present = $false
                    cpu_ms  = 0.0
                }
                if ($seen) { [void]$samples.Add($row); break }
            }
            else {
                $seen = $true
                $row = [ordered]@{
                    wall_ms = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
                    present = $true
                    cpu_ms  = [math]::Round($alive.TotalProcessorTime.TotalMilliseconds, 3)
                }
            }
            [void]$samples.Add($row)
            $next += 200
        }
        else {
            $sleep = [Math]::Min([int64]15, [int64]($next - $now))
            if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
        }
        if ($started.ElapsedMilliseconds -gt 600000) {
            try { $proc.Kill() } catch {}
            throw "timeout: $Label"
        }
    }
    if (-not $proc.HasExited) { [void]$proc.WaitForExit(5000) }
    $stderr = $errCopy.GetAwaiter().GetResult()
    $null = $outCopy.GetAwaiter().GetResult()
    [System.IO.File]::WriteAllText($errFile, $stderr, [System.Text.UTF8Encoding]::new($false))
    $builder = New-Object System.Text.StringBuilder
    foreach ($row in $samples) {
        [void]$builder.AppendLine(($row | ConvertTo-Json -Compress -Depth 6))
    }
    [System.IO.File]::WriteAllText($sampleFile, $builder.ToString(), [System.Text.UTF8Encoding]::new($false))
    $cpu = Get-CpuPercents $samples
    $completeness = $null
    $nodes = $null
    if (Test-Path -LiteralPath $outFile) {
        try {
            $doc = Get-Content -LiteralPath $outFile -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($null -ne $doc.PSObject.Properties['data']) {
                if ($null -ne $doc.data.PSObject.Properties['completeness']) {
                    $completeness = [string]$doc.data.completeness
                }
                if ($null -ne $doc.data.PSObject.Properties['nodes']) {
                    $nodes = @($doc.data.nodes).Count
                }
            }
        }
        catch { }
    }
    $cpuArr = @($cpu)
    return [ordered]@{
        label        = $Label
        elapsed_ms   = [math]::Round($started.Elapsed.TotalMilliseconds, 3)
        exit         = $proc.ExitCode
        pid          = $pidValue
        samples      = @($samples).Count
        cpu_n        = $cpuArr.Count
        cpu_p95      = if ($cpuArr.Count -gt 0) { Get-NearestRank $cpuArr 0.95 } else { $null }
        cpu_median   = if ($cpuArr.Count -gt 0) { Get-Median $cpuArr } else { $null }
        cpu_max      = if ($cpuArr.Count -gt 0) { ($cpuArr | Measure-Object -Maximum).Maximum } else { $null }
        completeness = $completeness
        nodes        = $nodes
        sample_file  = $sampleFile
        output_file  = $outFile
    }
}

$root = Ensure-AnalyzeFixture
$cliHash = Get-FileSha256 $cliExe
$cliInfo = Get-Item -LiteralPath $cliExe
Write-Host "fixture $root"
Write-Host ("cli sha256={0} bytes={1}" -f $cliHash, $cliInfo.Length)
Write-Host "warmup analyze scan"
$warmup = Invoke-AnalyzeSampled -Label 'warmup' -Root $root
Write-Host ("warmup pid={0} elapsed={1}ms exit={2} p95={3} completeness={4}" -f $warmup.pid, $warmup.elapsed_ms, $warmup.exit, $warmup.cpu_p95, $warmup.completeness)

$runs = @()
for ($i = 1; $i -le 5; $i++) {
    Write-Host "measure #$i"
    $run = Invoke-AnalyzeSampled -Label "run$i" -Root $root
    Write-Host ("run{0} pid={1} elapsed={2}ms exit={3} samples={4} p95={5} median={6} completeness={7}" -f $i, $run.pid, $run.elapsed_ms, $run.exit, $run.cpu_n, $run.cpu_p95, $run.cpu_median, $run.completeness)
    $runs += $run
}

$p95s = @($runs | ForEach-Object { [double]$_.cpu_p95 })
$pids = @($runs | ForEach-Object { [int]$_.pid })
$summary = [ordered]@{
    clock              = 'Stopwatch from Process.Start; sample until process exit; no 600s pad'
    cpu_formula        = 'delta TotalProcessorTime / wall_ms * 100'
    p95                = 'nearest-rank ceil(0.95 * n) - 1'
    fixture            = $root
    cli                = $cliExe
    cli_sha256         = $cliHash
    cli_bytes          = $cliInfo.Length
    warmup_pid         = [int]$warmup.pid
    measured_pids      = $pids
    raw_dir            = $RawDir
    warmup             = $warmup
    runs               = $runs
    run_p95s           = $p95s
    median_of_p95s     = Get-Median $p95s
    max_run_p95        = ($p95s | Measure-Object -Maximum).Maximum
    walk_window_p95    = Get-NearestRank $p95s 0.95
    all_run_p95_le_200 = -not (@($p95s | Where-Object { $_ -gt 200.0 }).Count)
}
$summaryPath = Join-Path $OutDir 'independent-native-cpu-summary.json'
$json = $summary | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText($summaryPath, $json, [System.Text.UTF8Encoding]::new($false))
Write-Host ("median_of_p95s={0} walk_window_p95={1} max_run_p95={2} all_le_200={3}" -f $summary.median_of_p95s, $summary.walk_window_p95, $summary.max_run_p95, $summary.all_run_p95_le_200)
Write-Host ("measured_pids={0}" -f ($pids -join ','))
if (-not $summary.all_run_p95_le_200) { exit 1 }
exit 0
