$ErrorActionPreference = 'Stop'
$EvidenceRoot = 'D:\Documents\Code\Rust\Exp\devsweep\.trellis\tasks\08-29-optimize-cli-tui-desktop-native\evidence'
$Cwd = 'D:\Documents\Code\Rust\Exp\devsweep'
$Exe = Join-Path $Cwd 'target\debug\devsweep.exe'
$Lappdata = Join-Path $EvidenceRoot 'native-independent\lappdata'
$utf8 = New-Object System.Text.UTF8Encoding $false
New-Item -ItemType Directory -Force -Path $Lappdata | Out-Null

function Invoke-Utf8([string[]]$ArgList) {
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $Exe
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.StandardOutputEncoding = $utf8
  $psi.StandardErrorEncoding = $utf8
  $psi.CreateNoWindow = $true
  $psi.WorkingDirectory = $Cwd
  $psi.Arguments = ($ArgList -join ' ')
  $psi.EnvironmentVariables['LOCALAPPDATA'] = $Lappdata
  $proc = New-Object System.Diagnostics.Process
  $proc.StartInfo = $psi
  [void]$proc.Start()
  $out = $proc.StandardOutput.ReadToEnd()
  $err = $proc.StandardError.ReadToEnd()
  $proc.WaitForExit()
  [pscustomobject]@{ ExitCode = $proc.ExitCode; Combined = $out + $err }
}

function Write-Utf8Log([string]$Name, [string]$CommandLine, [int]$ExitCode, [string]$Body) {
  $started = Get-Date
  $text = "command: $CommandLine`r`ncwd: $Cwd`r`nstarted: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n----- output -----`r`n$Body`r`n-----`r`nexit=$ExitCode`r`nfinished: $($started.ToString('yyyy-MM-ddTHH:mm:ss.fffffffK'))`r`n"
  [System.IO.File]::WriteAllText((Join-Path $EvidenceRoot $Name), $text, $utf8)
}

$en = Invoke-Utf8 @('--language','en','optimize','list')
Write-Utf8Log 'independent-native-list-en.log' 'devsweep --language en optimize list' $en.ExitCode $en.Combined
$zh = Invoke-Utf8 @('--language','zh-CN','optimize','list')
Write-Utf8Log 'independent-native-list-zh.log' 'devsweep --language zh-CN optimize list' $zh.ExitCode $zh.Combined
$guidanceOut = Join-Path $EvidenceRoot 'native-independent\fixtures\guidance-drive-optimize-plan.json'
$g = Invoke-Utf8 @('optimize','plan','--operation','guidance.drive_optimize','--output', $guidanceOut)
$gBody = $g.Combined + "`r`nfile_exists=" + [System.IO.File]::Exists($guidanceOut)
Write-Utf8Log 'independent-native-guidance-refusal.log' 'devsweep optimize plan --operation guidance.drive_optimize' $g.ExitCode $gBody
Write-Host "en=$($en.ExitCode) zh=$($zh.ExitCode) guidance=$($g.ExitCode) file=$([System.IO.File]::Exists($guidanceOut))"
