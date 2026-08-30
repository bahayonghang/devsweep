$ErrorActionPreference = "Stop"
$root = "D:\Documents\Code\Rust\Exp\devsweep"
Set-Location -LiteralPath $root

$exe = "target/release/devsweep-desktop.exe"
$dbg = "target/debug/devsweep-desktop.exe"
$nsis = "target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe"
foreach ($p in @($exe, $dbg, $nsis)) {
    $item = Get-Item -LiteralPath $p
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $p
    Write-Output ("HASH path={0} bytes={1} sha256={2}" -f $p, $item.Length, $hash.Hash)
}

$t1 = Get-Date
$all = @(Get-CimInstance Win32_Process)
$d1 = ((Get-Date) - $t1).TotalMilliseconds
$t2 = Get-Date
$wv = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'")
$d2 = ((Get-Date) - $t2).TotalMilliseconds
$t3 = Get-Date
$ds = @(Get-Process -Name "devsweep-desktop","msedgewebview2" -ErrorAction SilentlyContinue)
$d3 = ((Get-Date) - $t3).TotalMilliseconds
Write-Output ("CIM_ALL count={0} ms={1:N0}" -f $all.Count, $d1)
Write-Output ("CIM_WEBVIEW count={0} ms={1:N0}" -f $wv.Count, $d2)
Write-Output ("GET_PROCESS count={0} ms={1:N0}" -f $ds.Count, $d3)

$live = @(Get-CimInstance Win32_Process | Where-Object {
    $_.Name -eq "devsweep-desktop.exe" -or
    ($_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "webview-exe-name=devsweep-desktop\.exe")
})
Write-Output ("LIVE_OWNED count={0}" -f $live.Count)
foreach ($row in $live) {
    Write-Output ("LIVE pid={0} parent={1} name={2}" -f $row.ProcessId, $row.ParentProcessId, $row.Name)
}
