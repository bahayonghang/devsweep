$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
Set-Location $Cwd
$log = Join-Path $EvidenceRoot '03b-tui-bilingual-nocapture.log'
$started = Get-Date
cmd.exe /c "rtk cargo test -p devsweep-cli tui::modes::optimize::tests::bilingual -- --nocapture > `"$log`" 2>&1"
$exit = $LASTEXITCODE
Add-Content -Path $log -Value "`r`nexit=$exit`r`nstarted=$($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))"
Write-Host "WROTE 03b-tui-bilingual-nocapture.log exit=$exit"
