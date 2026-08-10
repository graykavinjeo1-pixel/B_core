[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [Parameter(Mandatory = $true)]
    [ValidateSet("USER", "CODEX", "LOCAL_TOOL")]
    [string]$Actor,

    [Parameter(Mandatory = $true)]
    [ValidateSet("CODE_CHANGE", "DEFECT_REPAIR", "REGRESSION_TEST", "REFACTOR", "FRONTEND_CHANGE", "BACKEND_CHANGE", "OPERATIONS_CHANGE", "DOCUMENTATION", "VERIFICATION")]
    [string]$Kind,

    [Parameter(Mandatory = $true)]
    [ValidateSet("PASS", "FAIL", "UNKNOWN")]
    [string]$Outcome,

    [Parameter(Mandatory = $true)]
    [string[]]$Paths,

    [Parameter(Mandatory = $true)]
    [ValidateLength(1, 512)]
    [string]$Summary,

    [string[]]$EvidenceSha256 = @()
)

$ErrorActionPreference = "Stop"

$root = [IO.Path]::GetFullPath($PackageRoot)
$configPath = [IO.Path]::GetFullPath($ConfigPath)
$supervisor = Join-Path $root "bin\b-core-growth-supervisor.exe"
if (-not (Test-Path -LiteralPath $supervisor -PathType Leaf)) {
    throw "GROWTH_SUPERVISOR_MISSING:$supervisor"
}
$config = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
$stateRoot = [IO.Path]::GetFullPath([string]$config.state_dir)
$controlRoot = Join-Path $stateRoot "control"
$null = New-Item -ItemType Directory -Path $controlRoot -Force
$eventId = "{0}-{1}" -f $Actor.ToLowerInvariant(), ([guid]::NewGuid().ToString("N"))
$eventPath = Join-Path $controlRoot (".{0}.event.json" -f $eventId)
$event = [ordered]@{
    event_id = $eventId
    actor = $Actor
    kind = $Kind
    paths = @($Paths | ForEach-Object { [IO.Path]::GetFullPath($_) })
    outcome = $Outcome
    summary = $Summary
    evidence_sha256 = @($EvidenceSha256)
    occurred_at_ms = 0
}
[IO.File]::WriteAllText(
    $eventPath,
    ($event | ConvertTo-Json -Depth 5),
    (New-Object Text.UTF8Encoding($false))
)
try {
    & $supervisor record-event $configPath $eventPath
    if ($LASTEXITCODE -ne 0) {
        throw "WORK_EVENT_RECORD_FAILED:$LASTEXITCODE"
    }
} finally {
    Remove-Item -LiteralPath $eventPath -Force -ErrorAction SilentlyContinue
}
