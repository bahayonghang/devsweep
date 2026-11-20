param([long]$HandleNumber,[int]$OwnerPid)
$ErrorActionPreference="Stop"
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
$root=[System.Windows.Automation.AutomationElement]::FromHandle([intptr]$HandleNumber)
if ($root.Current.ProcessId -ne $OwnerPid) {throw "unexpected owner"}
$condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::AutomationIdProperty,"host-action-confirm")
for($attempt=0;$attempt -lt 24;$attempt++){
  $hit=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
  if ($null -ne $hit) {Write-Output $hit.Current.Name;exit 0}
  Start-Sleep -Milliseconds 140
}
throw "expected the existing host review's confirmation control"
