[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [string]$TaskName = "B_Core_Bounded_Growth_Supervisor",

    [switch]$StartNow
)

$ErrorActionPreference = "Stop"

$root = [IO.Path]::GetFullPath($PackageRoot)
$config = [IO.Path]::GetFullPath($ConfigPath)
$runner = Join-Path $root "tools\run-growth-supervisor.ps1"
$supervisor = Join-Path $root "bin\b-core-growth-supervisor.exe"
foreach ($path in @($runner, $supervisor, $config)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "AUTOSTART_INPUT_MISSING:$path"
    }
}

& $supervisor init $config | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "SUPERVISOR_INIT_FAILED:$LASTEXITCODE"
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$powershell = Join-Path $PSHOME "powershell.exe"
$taskArguments = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "{0}" -PackageRoot "{1}" -ConfigPath "{2}" -Foreground' -f $runner, $root, $config
$action = New-ScheduledTaskAction -Execute $powershell -Argument $taskArguments
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $identity
$principal = New-ScheduledTaskPrincipal -UserId $identity -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit ([TimeSpan]::Zero) `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)
# Task Scheduler defaults to priority 7 (below normal), which also lowers the
# memory/I/O scheduling priority inherited by Cargo and rustc.  The supervisor
# is already bounded by explicit CPU, time, and state budgets; priority 4 keeps
# validation responsive without elevating it above ordinary foreground work.
$settings.Priority = 4
$null = Register-ScheduledTask `
    -TaskName $TaskName `
    -Action $action `
    -Trigger $trigger `
    -Principal $principal `
    -Settings $settings `
    -Force

$started = $false
if ($StartNow) {
    & $runner -PackageRoot $root -ConfigPath $config | Out-Null
    $started = $true
}

[ordered]@{
    installed = $true
    task_name = $TaskName
    trigger = "ONLOGON"
    run_level = "LIMITED"
    task_priority = 4
    executable = $powershell
    arguments = $taskArguments
    state_recovery = "IMMUTABLE_SNAPSHOT_AND_JOURNAL"
    started_now = $started
} | ConvertTo-Json
