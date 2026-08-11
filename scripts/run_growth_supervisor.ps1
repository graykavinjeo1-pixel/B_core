[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [switch]$Foreground
)

$ErrorActionPreference = "Stop"

$root = [IO.Path]::GetFullPath($PackageRoot)
$config = [IO.Path]::GetFullPath($ConfigPath)
$configObject = Get-Content -Raw -LiteralPath $config | ConvertFrom-Json
$stateRoot = [IO.Path]::GetFullPath([string]$configObject.state_dir)
$runtimeBin = if ($null -ne $configObject.source_mutation -and $configObject.source_mutation.enabled) {
    [IO.Path]::GetFullPath([string]$configObject.source_mutation.runtime_bin_dir)
} else {
    Join-Path $root "bin"
}
$supervisor = Join-Path $runtimeBin "b-core-growth-supervisor.exe"
if (-not (Test-Path -LiteralPath $supervisor -PathType Leaf)) {
    throw "GROWTH_SUPERVISOR_MISSING:$supervisor"
}
if (-not (Test-Path -LiteralPath $config -PathType Leaf)) {
    throw "GROWTH_CONFIG_MISSING:$config"
}

if ($Foreground) {
    $updatesApplied = 0
    while ($true) {
        & $supervisor run $config
        $exitCode = $LASTEXITCODE
        $handoffPath = Join-Path $stateRoot "control\SELF_UPDATE_READY.json"
        if (-not (Test-Path -LiteralPath $handoffPath -PathType Leaf)) {
            exit $exitCode
        }
        if ($updatesApplied -ge 64) {
            throw "SELF_UPDATE_RESTART_BOUND_REACHED"
        }
        $handoff = Get-Content -Raw -LiteralPath $handoffPath | ConvertFrom-Json
        $stagedSupervisor = [IO.Path]::GetFullPath([string]$handoff.staged_supervisor)
        $stagedVerifier = [IO.Path]::GetFullPath([string]$handoff.staged_verifier)
        $runtimeSupervisor = [IO.Path]::GetFullPath([string]$handoff.runtime_supervisor)
        $runtimeVerifier = [IO.Path]::GetFullPath([string]$handoff.runtime_verifier)
        $runtimePrefix = $runtimeBin.TrimEnd('\') + '\'
        foreach ($destination in @($runtimeSupervisor, $runtimeVerifier)) {
            if (-not $destination.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw "SELF_UPDATE_DESTINATION_OUTSIDE_RUNTIME_BIN:$destination"
            }
        }
        foreach ($source in @($stagedSupervisor, $stagedVerifier)) {
            if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
                throw "SELF_UPDATE_STAGED_BINARY_MISSING:$source"
            }
        }
        $null = New-Item -ItemType Directory -Force -Path $runtimeBin
        Copy-Item -LiteralPath $stagedVerifier -Destination $runtimeVerifier -Force
        Copy-Item -LiteralPath $stagedSupervisor -Destination $runtimeSupervisor -Force
        $appliedPath = Join-Path (Split-Path -Parent ([string]$handoff.source_receipt)) "runtime_update_applied.json"
        Move-Item -LiteralPath $handoffPath -Destination $appliedPath
        & $runtimeSupervisor resume $config | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "SELF_UPDATE_RESUME_FAILED:$LASTEXITCODE"
        }
        $supervisor = $runtimeSupervisor
        $updatesApplied += 1
    }
}

$powershell = Join-Path $PSHOME "powershell.exe"
$arguments = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "{0}" -PackageRoot "{1}" -ConfigPath "{2}" -Foreground' -f $PSCommandPath, $root, $config
$process = Start-Process `
    -FilePath $powershell `
    -ArgumentList $arguments `
    -WindowStyle Hidden `
    -PassThru

[ordered]@{
    started = $true
    process_id = $process.Id
    executable = $powershell
    supervisor = $supervisor
    config = $config
    hidden_window = $true
} | ConvertTo-Json
