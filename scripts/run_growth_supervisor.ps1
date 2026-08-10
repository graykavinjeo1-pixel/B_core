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
$supervisor = Join-Path $root "bin\b-core-growth-supervisor.exe"
if (-not (Test-Path -LiteralPath $supervisor -PathType Leaf)) {
    throw "GROWTH_SUPERVISOR_MISSING:$supervisor"
}
if (-not (Test-Path -LiteralPath $config -PathType Leaf)) {
    throw "GROWTH_CONFIG_MISSING:$config"
}

if ($Foreground) {
    & $supervisor run $config
    exit $LASTEXITCODE
}

$process = Start-Process `
    -FilePath $supervisor `
    -ArgumentList @("run", $config) `
    -WindowStyle Hidden `
    -PassThru

[ordered]@{
    started = $true
    process_id = $process.Id
    executable = $supervisor
    config = $config
    hidden_window = $true
} | ConvertTo-Json
