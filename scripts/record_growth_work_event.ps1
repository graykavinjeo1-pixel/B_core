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
    [ValidateSet("CODE_CHANGE", "DEFECT_REPAIR", "REGRESSION_TEST", "PERFORMANCE_OPTIMIZATION", "REFACTOR", "FRONTEND_CHANGE", "BACKEND_CHANGE", "OPERATIONS_CHANGE", "DOCUMENTATION", "VERIFICATION")]
    [string]$Kind,

    [Parameter(Mandatory = $true)]
    [ValidateSet("PASS", "FAIL", "UNKNOWN")]
    [string]$Outcome,

    [Parameter(Mandatory = $true)]
    [string[]]$Paths,

    [Parameter(Mandatory = $true)]
    [ValidateLength(1, 512)]
    [string]$Summary,

    [string[]]$EvidenceSha256 = @(),

    [string[]]$EvidencePaths = @(),

    [string]$PerformanceMetricsPath = ""
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
$controlRoot = [IO.Path]::Combine($stateRoot, "control")
$null = [IO.Directory]::CreateDirectory($controlRoot)
$eventId = "{0}-{1}" -f $Actor.ToLowerInvariant(), ([guid]::NewGuid().ToString("N"))
$eventPath = [IO.Path]::Combine($controlRoot, (".{0}.event.json" -f $eventId))
$performanceMetrics = @()
if (-not [string]::IsNullOrWhiteSpace($PerformanceMetricsPath)) {
    $metricsPath = [IO.Path]::GetFullPath($PerformanceMetricsPath)
    if (-not [IO.File]::Exists($metricsPath)) {
        throw "PERFORMANCE_METRICS_FILE_MISSING:$metricsPath"
    }
    $decodedMetrics = Get-Content -Raw -LiteralPath $metricsPath -Encoding UTF8 | ConvertFrom-Json
    # Windows PowerShell preserves a top-level JSON array as one pipeline
    # object. Enumerate it explicitly so both a single metric object and a
    # metric array serialize to the WorkEvent's flat Vec contract.
    $performanceMetrics = @()
    foreach ($metric in $decodedMetrics) {
        $performanceMetrics += $metric
    }
}
$event = [ordered]@{
    event_id = $eventId
    actor = $Actor
    kind = $Kind
    paths = @($Paths | ForEach-Object { [IO.Path]::GetFullPath($_) })
    outcome = $Outcome
    summary = $Summary
    evidence_sha256 = @($EvidenceSha256)
    evidence_artifacts = @($EvidencePaths | ForEach-Object { [IO.Path]::GetFullPath($_) })
    performance_metrics = $performanceMetrics
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
    if ([IO.File]::Exists($eventPath)) {
        [IO.File]::Delete($eventPath)
    }
}
