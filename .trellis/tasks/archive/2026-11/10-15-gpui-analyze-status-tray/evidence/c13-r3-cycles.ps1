$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class C13R3 {
    public const uint WM_CLOSE = 0x0010;
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
}
'@
$repo = 'D:\Documents\Code\Rust\Exp\devsweep'
$evidence = Join-Path $repo '.trellis\tasks\10-15-gpui-analyze-status-tray\evidence'
$binary = Join-Path $repo 'desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe'
$utf8 = New-Object System.Text.UTF8Encoding $false
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$log = New-Object System.Collections.Generic.List[string]
$log.Add("utc_start=$([DateTime]::UtcNow.ToString('o'))")
$log.Add("binary_sha256=$hash length=$((Get-Item $binary).Length)")
$log.Add('rustc_note=cargo 1.99.0 via mbx; RUSTFLAGS=-C link-arg=/Brepro; not an MSRV 1.88.0 pass')
$log.Add('note=23F7D614 and 5BFE59AF cycles are not reused')
$log.Add('note=displayed PID was not used to terminate any process')
$log.Add('os_crash_release=NOT_RUN')
Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force }
$clean = 0
$forced = 0
for ($cycle = 1; $cycle -le 20; $cycle++) {
    $bucket = Join-Path $env:TEMP "devsweep-gpui-c13r3-cycle-$cycle"
    if (Test-Path $bucket) { Remove-Item $bucket -Recurse -Force -ErrorAction SilentlyContinue }
    $local = Join-Path $bucket 'local'
    $roaming = Join-Path $bucket 'roaming'
    New-Item -ItemType Directory -Path $local, $roaming | Out-Null
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $repo
    $info.EnvironmentVariables['LOCALAPPDATA'] = $local
    $info.EnvironmentVariables['APPDATA'] = $roaming
    $proc = [System.Diagnostics.Process]::Start($info)
    $procId = $proc.Id
    $deadline = (Get-Date).AddSeconds(15)
    do { Start-Sleep -Milliseconds 100; $proc.Refresh() } while ($proc.MainWindowHandle -eq [IntPtr]::Zero -and -not $proc.HasExited -and (Get-Date) -lt $deadline)
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
        $log.Add("cycle=$cycle window=missing procId=$procId")
        $forced++
        Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        continue
    }
    [void][C13R3]::ShowWindow($proc.MainWindowHandle, 6)
    Start-Sleep -Milliseconds 100
    [void][C13R3]::ShowWindow($proc.MainWindowHandle, 9)
    Start-Sleep -Milliseconds 100
    [void][C13R3]::PostMessage($proc.MainWindowHandle, [C13R3]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
    $closed = $false
    $wait = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $wait) {
        if (-not (Get-Process -Id $procId -ErrorAction SilentlyContinue)) { $closed = $true; break }
        Start-Sleep -Milliseconds 100
    }
    if ($closed) { $clean++ } else { $forced++; Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue }
    $left = @(Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue).Count
    $log.Add("cycle=$cycle procId=$procId closed=$closed leftover=$left")
    if ($left -gt 0) { Get-Process -Name 'devsweep-gpui-spike' -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force } }
}
$log.Add("clean_exits=$clean forced=$forced")
$log.Add("spike_remaining=$(@(Get-Process -Name devsweep-gpui-spike -ErrorAction SilentlyContinue).Count)")
$log.Add("utc_end=$([DateTime]::UtcNow.ToString('o'))")
$log | ForEach-Object { Write-Host $_ }
[System.IO.File]::WriteAllLines((Join-Path $evidence 'c13-r3-cycles.txt'), $log, $utf8)
