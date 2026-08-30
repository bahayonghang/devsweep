param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,
    [Parameter(Mandatory = $true)]
    [string]$RunRoot,
    [Parameter(Mandatory = $true)]
    [string]$PwshPath
)

$ErrorActionPreference = "Stop"
$repository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$run = (Resolve-Path -LiteralPath $RunRoot).Path
$pwsh = (Resolve-Path -LiteralPath $PwshPath).Path
$log = Join-Path $run "01-toolchain-preflight.log"
$helper = Join-Path $run "release-rebind-run3.ps1"
$wrapper = Join-Path $run "run3-wrapper.ps1"

if (Test-Path -LiteralPath $log) { throw "preflight log exists; refusing overwrite" }
Start-Transcript -Path $log | Out-Null
try {
    $actualPwsh = (Get-Process -Id $PID).Path
    $commandLine = (Get-CimInstance Win32_Process -Filter "ProcessId = $PID").CommandLine
    Write-Output ("PWSH path={0} version={1} pid={2} commandline={3}" -f $actualPwsh, $PSVersionTable.PSVersion.ToString(), $PID, $commandLine)
    if ($actualPwsh -ne $pwsh -or $PSVersionTable.PSVersion.Major -lt 7) { throw "PowerShell 7 path/version gate failed" }

    $fixture = '{"id":7,"method":"Runtime.evaluate","params":{"expression":"1+1","nested":{"levels":[{"value":2}]}},"result":{"result":{"type":"number","value":2}},"error":null}'
    $roundTrip = $fixture | ConvertFrom-Json -Depth 100 | ConvertTo-Json -Depth 100 -Compress | ConvertFrom-Json -Depth 100
    if ($roundTrip.params.nested.levels[0].value -ne 2 -or $roundTrip.result.result.value -ne 2) { throw "nested JSON round-trip failed" }
    Write-Output "NESTED_JSON_DEPTH100=PASS"

    $socket = [System.Net.WebSockets.ClientWebSocket]::new()
    $socket.Dispose()
    Write-Output "CLIENT_WEBSOCKET=PASS"

    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    if ($null -eq [System.Windows.Automation.AutomationElement]) { throw "UIAutomation load failed" }
    Write-Output "UIAUTOMATION=PASS"

    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Run3OuterToolGateNative {
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public UIntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public INPUTUNION union; }
    [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
'@
    if ($null -eq [Run3OuterToolGateNative].GetMethod("SendInput") -or $null -eq [Run3OuterToolGateNative].GetMethod("SetCursorPos")) { throw "SendInput preload failed" }
    Write-Output "SENDINPUT_PINVOKE=PASS"

    foreach ($scriptPath in @($PSCommandPath, $helper, $wrapper)) {
        $tokens = $null
        $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$errors) | Out-Null
        Write-Output ("STATIC_PARSE path={0} errors={1} sha256={2}" -f $scriptPath, $errors.Count, (Get-FileHash -Algorithm SHA256 -LiteralPath $scriptPath).Hash)
        if ($errors.Count -ne 0) { throw "static parse failed: $scriptPath" }
    }

    $expected = @(
        [pscustomobject]@{ path = "target/release/devsweep-desktop.exe"; bytes = 9827328; sha256 = "92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20" },
        [pscustomobject]@{ path = "target/debug/devsweep-desktop.exe"; bytes = 14758912; sha256 = "F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5" },
        [pscustomobject]@{ path = "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"; bytes = 3152372; sha256 = "0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328" }
    )
    foreach ($entry in $expected) {
        $path = Join-Path $repository $entry.path
        $item = Get-Item -LiteralPath $path
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
        $match = $item.Length -eq $entry.bytes -and $hash -eq $entry.sha256
        Write-Output ("ARTIFACT path={0} bytes={1} sha256={2} matches={3}" -f $entry.path, $item.Length, $hash, $match)
        if (-not $match) { throw "artifact drift: $($entry.path)" }
    }

    $main = @(Get-CimInstance Win32_Process | Where-Object Name -eq "devsweep-desktop.exe")
    $webview = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe" })
    $port1 = @(Get-NetTCPConnection -LocalPort 9391 -State Listen -ErrorAction SilentlyContinue)
    $port2 = @(Get-NetTCPConnection -LocalPort 9392 -State Listen -ErrorAction SilentlyContinue)
    Write-Output ("PRELAUNCH_RESIDUE main={0} webview={1} port9391={2} port9392={3}" -f $main.Count, $webview.Count, $port1.Count, $port2.Count)
    if ($main.Count -ne 0 -or $webview.Count -ne 0 -or $port1.Count -ne 0 -or $port2.Count -ne 0) { throw "prelaunch residue gate failed" }
    Write-Output "TOOLCHAIN_PREFLIGHT=PASS"
}
finally {
    Stop-Transcript | Out-Null
}
