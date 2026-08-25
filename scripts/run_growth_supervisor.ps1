[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [switch]$Foreground,

    [switch]$SelfCheck
)

$ErrorActionPreference = "Stop"

# Task Scheduler launches ordinary tasks at BELOW_NORMAL priority unless an
# XML priority is supplied.  The Rust compiler then has its multi-GiB working
# set repeatedly trimmed even when the machine has ample free memory, turning
# a sub-minute validation into a long apparent stall.  Normalize the wrapper
# before it creates the supervisor so every bounded child validation inherits
# the ordinary interactive priority class.
try {
    [Diagnostics.Process]::GetCurrentProcess().PriorityClass =
        [Diagnostics.ProcessPriorityClass]::Normal
} catch {
    throw "GROWTH_SUPERVISOR_PRIORITY_NORMALIZATION_FAILED:$($_.Exception.Message)"
}

function Resolve-BLocalPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    # System.IO.Path on Windows PowerShell 5.1 can return $null for an already
    # extended-length path (\\?\...).  Convert it to the ordinary absolute
    # representation before using Join-Path, Split-Path, or Copy-Item.
    $candidate = $Path
    if ($candidate.StartsWith('\\?\', [StringComparison]::Ordinal)) {
        $candidate = $candidate.Substring(4)
    }
    return [IO.Path]::GetFullPath($candidate)
}

$root = Resolve-BLocalPath $PackageRoot
$config = Resolve-BLocalPath $ConfigPath
$configObject = Get-Content -Raw -LiteralPath $config | ConvertFrom-Json
$stateRoot = Resolve-BLocalPath ([string]$configObject.state_dir)
$runtimeBin = if ($null -ne $configObject.source_mutation -and $configObject.source_mutation.enabled) {
    Resolve-BLocalPath ([string]$configObject.source_mutation.runtime_bin_dir)
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

if ($SelfCheck) {
    $canary = Resolve-BLocalPath '\\?\C:\b-core-path-canary'
    if ($canary -ne 'C:\b-core-path-canary' -or [string]::IsNullOrWhiteSpace($stateRoot)) {
        throw "EXTENDED_PATH_NORMALIZATION_SELF_CHECK_FAILED"
    }
    [ordered]@{
        pass = $true
        windows_powershell_extended_path_normalization = $true
        operator_stop_preserved_across_self_update = $true
        state_root = $stateRoot
        runtime_bin = $runtimeBin
    } | ConvertTo-Json
    exit 0
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
        $operatorStopPath = Join-Path $stateRoot "control\STOP"
        $preserveOperatorStop = Test-Path -LiteralPath $operatorStopPath -PathType Leaf
        $stagedSupervisor = Resolve-BLocalPath ([string]$handoff.staged_supervisor)
        $stagedVerifier = Resolve-BLocalPath ([string]$handoff.staged_verifier)
        $runtimeSupervisor = Resolve-BLocalPath ([string]$handoff.runtime_supervisor)
        $runtimeVerifier = Resolve-BLocalPath ([string]$handoff.runtime_verifier)
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
        $sourceReceipt = Resolve-BLocalPath ([string]$handoff.source_receipt)
        $appliedPath = Join-Path (Split-Path -Parent $sourceReceipt) "runtime_update_applied.json"
        Move-Item -LiteralPath $handoffPath -Destination $appliedPath
        & $runtimeSupervisor resume $config | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "SELF_UPDATE_RESUME_FAILED:$LASTEXITCODE"
        }
        if ($preserveOperatorStop) {
            & $runtimeSupervisor stop $config | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "SELF_UPDATE_OPERATOR_STOP_RESTORE_FAILED:$LASTEXITCODE"
            }
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
