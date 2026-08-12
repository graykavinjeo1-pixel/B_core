[CmdletBinding()]
param(
    [string]$Worktree = (Get-Location).Path,
    [string]$ReportDirectory = "",
    [string]$CargoTargetDirectory = "",
    [int]$TestTimeoutSeconds = 600,
    [string]$WorkerLane = ""
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
if ([string]::IsNullOrWhiteSpace($WorkerLane)) {
    if (Test-Path -LiteralPath $ReportDirectory) {
        throw "IMMUTABLE_REPORT_DIRECTORY_ALREADY_EXISTS:$ReportDirectory"
    }
    New-Item -ItemType Directory -Path $ReportDirectory | Out-Null
} elseif (-not (Test-Path -LiteralPath $ReportDirectory -PathType Container)) {
    throw "AUDIT_WORKER_REPORT_DIRECTORY_MISSING:$ReportDirectory"
}
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
    if (-not [string]::IsNullOrWhiteSpace($WorkerLane)) {
        # The two surface lanes are concurrent. Bound each Cargo scheduler to
        # half of the logical processors so nested rustc workers do not turn
        # DAG parallelism into CPU oversubscription and memory pressure.
        $jobsPerLane = [Math]::Max(1, [int][Math]::Floor([Environment]::ProcessorCount / 2))
        $process.StartInfo.Environment["CARGO_BUILD_JOBS"] = [string]$jobsPerLane
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

if (-not [string]::IsNullOrWhiteSpace($WorkerLane)) {
    $targetBase = if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
        Join-Path $resolvedWorktree "target"
    } else {
        [IO.Path]::GetFullPath($CargoTargetDirectory)
    }
    $laneProbes = switch ($WorkerLane) {
        "historical" {
            $CargoTargetDirectory = $targetBase
            @(
                # Historical and runtime-core are alternative module surfaces.
                # Keep each surface serial inside its own persistent Cargo
                # cache lane while the two independent lanes run concurrently.
                (Invoke-CargoProbe "cargo_compile_historical_clean_canary" @(
                    "check", "--workspace", "--lib", "--bins", "--features", "historical-campaigns", "--quiet", "-j", "1"
                ) $TestTimeoutSeconds @{
                    CARGO_INCREMENTAL = "0"
                }),
                (Invoke-CargoProbe "cargo_test_historical_libraries" @(
                    "test", "--workspace", "--lib", "--features", "historical-campaigns", "--quiet"
                ) $TestTimeoutSeconds),
                (Invoke-CargoProbe "cargo_test_docs" @(
                    "test", "--workspace", "--doc", "--quiet"
                ) $TestTimeoutSeconds),
                (Invoke-CargoProbe "cargo_clippy_historical_strict" @(
                    "clippy", "--workspace", "--lib", "--bins", "--features", "historical-campaigns", "--", "-D", "warnings"
                ) $TestTimeoutSeconds)
            )
        }
        "runtime" {
            # Reuse the persistent runtime test lane already used by autonomous
            # source validation; do not create another multi-GiB cache merely
            # to gain audit parallelism.
            $CargoTargetDirectory = Join-Path $targetBase "bcore-source-test-lane"
            @(
                (Invoke-CargoProbe "cargo_compile_runtime_core_clean_canary" @(
                    "check", "-p", "semantic-reasoning", "--lib", "--bins", "--no-default-features", "--features", "runtime-core", "--quiet", "-j", "1"
                ) $TestTimeoutSeconds @{
                    CARGO_INCREMENTAL = "0"
                }),
                (Invoke-CargoProbe "cargo_test_runtime_core" @(
                    "test", "-p", "semantic-reasoning", "--lib", "--no-default-features", "--features", "runtime-core", "--quiet"
                ) $TestTimeoutSeconds),
                (Invoke-CargoProbe "cargo_clippy_runtime_core_strict" @(
                    "clippy", "-p", "semantic-reasoning", "--lib", "--bins", "--no-default-features", "--features", "runtime-core", "--", "-D", "warnings"
                ) $TestTimeoutSeconds)
            )
        }
        default {
            throw "UNKNOWN_AUDIT_WORKER_LANE:$WorkerLane"
        }
    }
    $laneReceiptPath = Join-Path $resolvedReportDirectory "cargo_lane_$WorkerLane.json"
    Write-ImmutableText $laneReceiptPath (($laneProbes | ConvertTo-Json -Depth 8) + "`n")
    exit 0
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

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
$workerScript = $PSCommandPath
$workerCargoTarget = if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    Join-Path $resolvedWorktree "target"
}
else {
    [IO.Path]::GetFullPath($CargoTargetDirectory)
}
$workerBlock = {
    param($PowerShellExecutable, $ScriptPath, $Lane, $WorkerWorktree, $WorkerReport, $WorkerTarget, $WorkerTimeout)
    $arguments = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath,
        "-Worktree", $WorkerWorktree,
        "-ReportDirectory", $WorkerReport,
        "-CargoTargetDirectory", $WorkerTarget,
        "-TestTimeoutSeconds", [string]$WorkerTimeout,
        "-WorkerLane", $Lane
    )
    & $PowerShellExecutable @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "AUDIT_WORKER_FAILED:${Lane}:$LASTEXITCODE"
    }
}
$jobs = @(
    (Start-Job -ScriptBlock $workerBlock -ArgumentList @(
        $powershell, $workerScript, "historical", $resolvedWorktree,
        $resolvedReportDirectory, $workerCargoTarget, $TestTimeoutSeconds
    )),
    (Start-Job -ScriptBlock $workerBlock -ArgumentList @(
        $powershell, $workerScript, "runtime", $resolvedWorktree,
        $resolvedReportDirectory, $workerCargoTarget, $TestTimeoutSeconds
    ))
)
try {
    # Formatting does not acquire either Cargo build-cache lane and runs while
    # both compile/test/clippy DAG branches are active.
    $formatProbe = Invoke-CargoProbe "cargo_fmt" @("fmt", "--check") 120
    $jobs | Wait-Job | Out-Null
    foreach ($job in $jobs) {
        Receive-Job -Job $job | ForEach-Object { Write-Host $_ }
        if ($job.State -ne "Completed") {
            throw "AUDIT_WORKER_JOB_FAILED:$($job.State)"
        }
    }
} finally {
    $jobs | Remove-Job -Force -ErrorAction SilentlyContinue
}
$historicalProbes = Get-Content -LiteralPath (Join-Path $resolvedReportDirectory "cargo_lane_historical.json") -Raw | ConvertFrom-Json
$runtimeProbes = Get-Content -LiteralPath (Join-Path $resolvedReportDirectory "cargo_lane_runtime.json") -Raw | ConvertFrom-Json
$probes = @($metadataProbe, $formatProbe) + @($historicalProbes) + @($runtimeProbes)

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
    cargo_probe_dag_parallel = $true
    cargo_probe_parallel_lanes = 2
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
