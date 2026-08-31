$ErrorActionPreference = 'Continue'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
New-Item -ItemType Directory -Force -Path $EvidenceRoot | Out-Null

function Invoke-Logged {
  param([string]$Name, [string]$CommandLine)
  $started = Get-Date
  Push-Location $Cwd
  $output = & cmd.exe /c "$CommandLine 2>&1"
  $exit = $LASTEXITCODE
  Pop-Location
  $finished = Get-Date
  $body = if ($null -eq $output) { '' } else { ($output | Out-String) }
  $text = "command: $CommandLine`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$body`r`n-----`r`nexit=$exit`r`nfinished: $($finished.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
  [System.IO.File]::WriteAllText((Join-Path $EvidenceRoot $Name), $text)
  Write-Host "WROTE $Name exit=$exit"
  return $exit
}

$script:LastExit = 0
function Run-Step([string]$Name, [string]$CommandLine) {
  $code = Invoke-Logged -Name $Name -CommandLine $CommandLine
  if ($code -ne 0) { $script:LastExit = $code }
}

Run-Step '01-rtk-cli-optimize.log' 'rtk cargo test -p devsweep-cli optimize'
Run-Step '02-rtk-desktop-optimize.log' 'rtk cargo test -p devsweep-desktop optimize'
Run-Step '03-rtk-cli-tui-optimize.log' 'rtk cargo test -p devsweep-cli tui::modes::optimize'
Run-Step '04-rtk-just-desktop-web-check.log' 'rtk just desktop-web-check'
Run-Step '04b-npm-types-generate.log' 'cd /d desktop && npm run types:generate'
Run-Step '04c-npm-lint.log' 'cd /d desktop && npm run lint'
Run-Step '04d-npm-typecheck.log' 'cd /d desktop && npm run typecheck'
Run-Step '04e-npm-test.log' 'cd /d desktop && npm test'
Run-Step '04f-npm-build.log' 'cd /d desktop && npm run build'
Run-Step '05-rtk-just-desktop-test.log' 'rtk just desktop-test'
Run-Step '07-rtk-git-diff-check.log' 'rtk git diff --check'
Write-Host "capture-gates last_nonzero=$script:LastExit"
exit 0
