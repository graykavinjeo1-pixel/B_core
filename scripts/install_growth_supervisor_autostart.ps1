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

$taskCommand = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "{0}" -PackageRoot "{1}" -ConfigPath "{2}" -Foreground' -f $runner, $root, $config
& schtasks.exe /Create /TN $TaskName /SC ONLOGON /RL LIMITED /TR $taskCommand /F | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "AUTOSTART_TASK_CREATE_FAILED:$LASTEXITCODE"
}

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
    state_recovery = "IMMUTABLE_SNAPSHOT_AND_JOURNAL"
    started_now = $started
} | ConvertTo-Json
