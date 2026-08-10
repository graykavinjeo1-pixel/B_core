[CmdletBinding()]
param(
    [string]$TaskName = "B_Core_Bounded_Growth_Supervisor"
)

$ErrorActionPreference = "Stop"

& schtasks.exe /Delete /TN $TaskName /F | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "AUTOSTART_TASK_DELETE_FAILED:$LASTEXITCODE"
}

[ordered]@{
    uninstalled = $true
    task_name = $TaskName
    learned_state_deleted = $false
} | ConvertTo-Json
