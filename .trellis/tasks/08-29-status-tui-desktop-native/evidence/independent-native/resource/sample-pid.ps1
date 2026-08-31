param(
    [Parameter(Mandatory = $true)][int]$Id,
    [Parameter(Mandatory = $true)][int]$DurationMs,
    [Parameter(Mandatory = $true)][int]$PeriodMs,
    [Parameter(Mandatory = $true)][string]$OutFile
)

$ErrorActionPreference = 'Continue'
$origin = Get-Date
$sw = [Diagnostics.Stopwatch]::StartNew()
$samples = New-Object System.Collections.Generic.List[object]
$next = 0
while ($sw.ElapsedMilliseconds -lt $DurationMs) {
    $now = $sw.ElapsedMilliseconds
    if ($now -ge $next) {
        $t = ((Get-Date) - $origin).TotalMilliseconds
        $p = Get-Process -Id $Id -ErrorAction SilentlyContinue
        if ($null -eq $p) {
            [void]$samples.Add([ordered]@{
                    wall_ms   = [math]::Round($t, 3)
                    present   = $false
                    user_ms   = 0.0
                    kernel_ms = 0.0
                    cpu_ms    = 0.0
                    private   = [int64]0
                    threads   = 0
                })
        }
        else {
            [void]$samples.Add([ordered]@{
                    wall_ms   = [math]::Round($t, 3)
                    present   = $true
                    user_ms   = [math]::Round($p.UserProcessorTime.TotalMilliseconds, 3)
                    kernel_ms = [math]::Round($p.PrivilegedProcessorTime.TotalMilliseconds, 3)
                    cpu_ms    = [math]::Round($p.TotalProcessorTime.TotalMilliseconds, 3)
                    private   = [int64]$p.PrivateMemorySize64
                    threads   = [int]$p.Threads.Count
                })
        }
        $next += $PeriodMs
    }
    else {
        $sleep = [Math]::Min(15, $next - $now)
        if ($sleep -gt 0) { Start-Sleep -Milliseconds $sleep }
    }
}

$dir = Split-Path -Parent $OutFile
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
$json = $samples | ConvertTo-Json -Compress -Depth 4
$utf8 = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($OutFile, $json, $utf8)
Write-Output $samples.Count
