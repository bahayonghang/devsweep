#requires -Version 5.1
$ErrorActionPreference = 'Stop'
$Evidence = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $Evidence '..\..\..\..\..')).Path
$CleanDir = Join-Path $Evidence 'clean'
New-Item -ItemType Directory -Force -Path $CleanDir | Out-Null

$fixtureDir = Join-Path $CleanDir 'recycle-fixture'
New-Item -ItemType Directory -Force -Path $fixtureDir | Out-Null
$fixture = Join-Path $fixtureDir ("devsweep-five-mode-recycle-" + [guid]::NewGuid().ToString('n') + '.txt')
Set-Content -LiteralPath $fixture -Value 'task-owned Recycle Bin fixture; emptying is outside DevSweep' -Encoding UTF8
Add-Type -AssemblyName Microsoft.VisualBasic
[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($fixture, 'OnlyErrorDialogs', 'SendToRecycleBin')
$note = [ordered]@{
    created              = $fixture
    recycled             = -not (Test-Path -LiteralPath $fixture)
    emptying             = 'not performed; Recycle Bin emptying is outside DevSweep'
    software_uninstall   = 'UNVERIFIED; no confirmed disposable current-user MSIX identity'
    sleep_resume         = 'UNVERIFIED; host sleep/resume is not reproducible in this session'
}
$note | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $CleanDir 'recycle-note.json') -Encoding UTF8

$integrity = & "$env:WINDIR\System32\whoami.exe" /groups | Out-String
Set-Content -LiteralPath (Join-Path $Evidence 'integrity.txt') -Value $integrity -Encoding UTF8

$node = (Get-Command node).Source
Push-Location $RepoRoot
try {
    & $node $Evidence\capture-native-five-mode.mjs
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
