$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$started = Get-Date
Set-Location $Cwd
$output = & cmd.exe /c 'rtk just desktop-build 2>&1'
$exit = $LASTEXITCODE
$finished = Get-Date
$body = if ($null -eq $output) { '' } else { ($output | Out-String) }
$text = "command: rtk just desktop-build`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$exit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
[System.IO.File]::WriteAllText((Join-Path $EvidenceRoot '06-rtk-just-desktop-build.log'), $text)
Write-Host "WROTE 06-rtk-just-desktop-build.log exit=$exit"
exit $exit
