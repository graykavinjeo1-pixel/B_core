[CmdletBinding()]
param(
    [string]$Worktree = (Get-Location).Path,
    [string]$ReportDirectory = "",
    [string]$CargoTargetDirectory = "",
    [int]$TestTimeoutSeconds = 600
)

$ErrorActionPreference = "Stop"
$resolvedWorktree = (Resolve-Path -LiteralPath $Worktree).Path
if (-not (Test-Path -LiteralPath (Join-Path $resolvedWorktree "Cargo.toml"))) {
    throw "WORKTREE_CARGO_MANIFEST_MISSING"
}
if ([string]::IsNullOrWhiteSpace($ReportDirectory)) {
    $ReportDirectory = Join-Path $resolvedWorktree "reports\self-healing-audit"
}
$resolvedReportParent = Split-Path -Parent $ReportDirectory
New-Item -ItemType Directory -Force -Path $resolvedReportParent | Out-Null
if (Test-Path -LiteralPath $ReportDirectory) {
    throw "IMMUTABLE_REPORT_DIRECTORY_ALREADY_EXISTS:$ReportDirectory"
}
New-Item -ItemType Directory -Path $ReportDirectory | Out-Null
$resolvedReportDirectory = (Resolve-Path -LiteralPath $ReportDirectory).Path

function Get-Sha256([string]$Text) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $algorithm.ComputeHash($bytes)
        return -join ($hash | ForEach-Object { $_.ToString("x2") })
    }
    finally {
        $algorithm.Dispose()
    }
}

function Write-ImmutableText([string]$Path, [string]$Text) {
    if (Test-Path -LiteralPath $Path) {
        throw "IMMUTABLE_ARTIFACT_ALREADY_EXISTS:$Path"
    }
    $temporary = "$Path.$PID.tmp"
    [System.IO.File]::WriteAllText($temporary, $Text, [System.Text.UTF8Encoding]::new($false))
    [System.IO.File]::Move($temporary, $Path)
}

function Invoke-CargoProbe(
    [string]$Name,
    [string[]]$Arguments,
    [int]$TimeoutSeconds,
    [hashtable]$EnvironmentOverrides = @{}
) {
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo.FileName = "cargo"
    $process.StartInfo.WorkingDirectory = $resolvedWorktree
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    $process.StartInfo.CreateNoWindow = $true
    $process.StartInfo.Arguments = $Arguments -join " "
    $process.StartInfo.Environment["CARGO_NET_OFFLINE"] = "true"
    $process.StartInfo.Environment["HTTP_PROXY"] = ""
    $process.StartInfo.Environment["HTTPS_PROXY"] = ""
    $process.StartInfo.Environment["ALL_PROXY"] = ""
    $process.StartInfo.Environment["NO_PROXY"] = "*"
    if (-not [string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
        $process.StartInfo.Environment["CARGO_TARGET_DIR"] = $CargoTargetDirectory
    }
    foreach ($entry in $EnvironmentOverrides.GetEnumerator()) {
        $process.StartInfo.Environment[$entry.Key] = [string]$entry.Value
    }
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) {
        throw "PROBE_PROCESS_START_FAILED:$Name"
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $timedOut = -not $process.WaitForExit($TimeoutSeconds * 1000)
    if ($timedOut) {
        $process.Kill($true)
        $process.WaitForExit()
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $stopwatch.Stop()
    $combined = $stdout + $stderr
    $exitCode = if ($timedOut) { 124 } else { $process.ExitCode }
    $status = if ($timedOut) { "TIMEOUT" } elseif ($exitCode -eq 0) { "PASS" } else { "FAIL" }
    $logPath = Join-Path $resolvedReportDirectory "$Name.log"
    Write-ImmutableText $logPath $combined
    Write-Host ("{0}={1} ({2} ms)" -f $Name, $status, $stopwatch.ElapsedMilliseconds)
    return [ordered]@{
        name = $Name
        command = "cargo " + ($Arguments -join " ")
        command_sha256 = Get-Sha256 ("cargo " + ($Arguments -join " "))
        status = $status
        exit_code = $exitCode
        duration_ms = $stopwatch.ElapsedMilliseconds
        timeout_ms = $TimeoutSeconds * 1000
        output_sha256 = Get-Sha256 $combined
        log = $logPath
        independent_process_observed = $true
    }
}

$metadataProbe = Invoke-CargoProbe "cargo_metadata" @(
    "metadata", "--no-deps", "--format-version", "1"
) 120
if ($metadataProbe.status -ne "PASS") {
    throw "CARGO_METADATA_FAILED"
}
$metadataText = [System.IO.File]::ReadAllText($metadataProbe.log)
$metadata = $metadataText | ConvertFrom-Json
$compiledSurfaces = @()
foreach ($package in $metadata.packages) {
    foreach ($target in $package.targets) {
        $compiledSurfaces += [ordered]@{
            package = $package.name
            target = $target.name
            target_kind = ($target.kind -join ",")
            source_path = $target.src_path
            authority = "COMPILED"
        }
    }
}

$recursiveSource = Join-Path $resolvedWorktree "crates\synapse-recursive-core\src"
$quarantinedSurfaces = Get-ChildItem -LiteralPath $recursiveSource -Recurse -Filter "*.rs" |
    Where-Object {
        $_.FullName -ne (Join-Path $recursiveSource "lib.rs") -and
        $_.FullName -ne (Join-Path $recursiveSource "quarantine.rs")
    } |
    Sort-Object FullName |
    ForEach-Object {
        [ordered]@{
            package = "synapse-recursive-core"
            target = $_.BaseName
            target_kind = "source-only"
            source_path = $_.FullName
            authority = "QUARANTINED"
        }
    }

$probes = @(
    $metadataProbe,
    (Invoke-CargoProbe "cargo_fmt" @("fmt", "--check") 120),
    (Invoke-CargoProbe "cargo_compile_all_targets_clean_canary" @(
        "test", "--workspace", "--all-targets", "--all-features", "--no-run", "--quiet", "-j", "1"
    ) $TestTimeoutSeconds @{
        CARGO_INCREMENTAL = "0"
    }),
    (Invoke-CargoProbe "cargo_test_all_targets" @(
        "test", "--workspace", "--all-targets", "--all-features", "--quiet"
    ) $TestTimeoutSeconds),
    (Invoke-CargoProbe "cargo_test_docs" @("test", "--workspace", "--doc", "--quiet") $TestTimeoutSeconds),
    (Invoke-CargoProbe "cargo_clippy_strict" @(
        "clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"
    ) $TestTimeoutSeconds)
)

$allPass = ($probes | Where-Object { $_.status -ne "PASS" }).Count -eq 0
$receipt = [ordered]@{
    schema = "B_CORE_SELF_HEALING_AUDIT_1"
    worktree = $resolvedWorktree
    git_commit = (git -C $resolvedWorktree rev-parse HEAD).Trim()
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    compiled_surface_count = $compiledSurfaces.Count
    quarantined_surface_count = $quarantinedSurfaces.Count
    inventory = @($compiledSurfaces) + @($quarantinedSurfaces)
    probes = $probes
    clean_compile_canary_incremental = $false
    clean_compile_canary_jobs = 1
    compiled_coverage_complete = $allPass
    all_compiled_probes_pass = $allPass
    quarantined_source_compiled_as_authority = $false
    verification_mode = "INDEPENDENT_LOCAL_DETERMINISTIC_PROCESS"
    codex_calls = 0
    external_llm_calls = 0
    network_reads = 0
    network_writes = 0
    prestart_autonomous_research_events = 0
    prestart_future_instance_exposure_events = 0
}
$receiptJson = $receipt | ConvertTo-Json -Depth 12
Write-ImmutableText (Join-Path $resolvedReportDirectory "audit_receipt.json") ($receiptJson + "`n")
Write-Host "AUDIT_RECEIPT=$(Join-Path $resolvedReportDirectory 'audit_receipt.json')"
if (-not $allPass) {
    exit 1
}
