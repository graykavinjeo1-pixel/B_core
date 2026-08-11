[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$TargetDirectory,

    [Parameter(Mandatory = $true)]
    [string]$BuildTargetDirectory,

    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,

    [switch]$ReuseBuildTarget,

    [string]$FastReusePackageRoot = "",

    [string]$RefreshBinaryDirectory = ""
)

$ErrorActionPreference = "Stop"

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Text
    )

    [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}

function Get-NormalizedRelativePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,

        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $rootPath = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $fullPath = [IO.Path]::GetFullPath($Path)
    $prefix = $rootPath + '\'
    if (-not $fullPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "PATH_OUTSIDE_PACKAGE:$fullPath"
    }
    return $fullPath.Substring($prefix.Length).Replace('\', '/')
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..")).TrimEnd('\')
$packageRoot = [IO.Path]::GetFullPath($TargetDirectory).TrimEnd('\')
$buildRoot = [IO.Path]::GetFullPath($BuildTargetDirectory).TrimEnd('\')
$archive = [IO.Path]::GetFullPath($ArchivePath)
$fastPackaging = -not [string]::IsNullOrWhiteSpace($FastReusePackageRoot)
$basePackage = if ($fastPackaging) { [IO.Path]::GetFullPath($FastReusePackageRoot).TrimEnd('\') } else { "" }
$refreshBinRoot = if ($fastPackaging) { [IO.Path]::GetFullPath($RefreshBinaryDirectory).TrimEnd('\') } else { "" }

if (Test-Path -LiteralPath $packageRoot) {
    throw "TARGET_ALREADY_EXISTS:$packageRoot"
}
if ((Test-Path -LiteralPath $buildRoot) -and -not $ReuseBuildTarget) {
    throw "BUILD_TARGET_ALREADY_EXISTS:$buildRoot"
}
if (Test-Path -LiteralPath $archive) {
    throw "ARCHIVE_ALREADY_EXISTS:$archive"
}

Push-Location $repositoryRoot
try {
    $status = (& git status --porcelain=v1) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw "GIT_STATUS_FAILED"
    }
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        throw "SOURCE_WORKTREE_NOT_CLEAN"
    }
    $sourceCommit = (& git rev-parse HEAD).Trim()
    $sourceBranch = (& git branch --show-current).Trim()
    $trackedFiles = @(& git ls-files)
    if ($LASTEXITCODE -ne 0 -or $trackedFiles.Count -eq 0) {
        throw "GIT_TRACKED_FILE_DISCOVERY_FAILED"
    }
    $metadata = & cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        throw "CARGO_METADATA_FAILED"
    }
} finally {
    Pop-Location
}

$toolchain = (& rustc -vV) -join "`n"
if ($LASTEXITCODE -ne 0 -or
    $toolchain -notmatch "release: 1\.96\.0" -or
    $toolchain -notmatch "commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96" -or
    $toolchain -notmatch "host: x86_64-pc-windows-msvc") {
    throw "TOOLCHAIN_MISMATCH:requires rustc 1.96.0 (ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96) x86_64-pc-windows-msvc"
}

$expectedBinaries = @($metadata.packages.targets |
    Where-Object { $_.kind -contains "bin" } |
    ForEach-Object { [string]$_.name } |
    Sort-Object -Unique)
if ($expectedBinaries.Count -eq 0) {
    throw "NO_WORKSPACE_BINARIES_DISCOVERED"
}

$sourceRoot = Join-Path $packageRoot "source"
$binRoot = Join-Path $packageRoot "bin"
$toolsRoot = Join-Path $packageRoot "tools"
$receiptRoot = Join-Path $packageRoot "receipts"
$null = New-Item -ItemType Directory -Path $sourceRoot, $binRoot, $toolsRoot, $receiptRoot, $buildRoot -Force

foreach ($relative in $trackedFiles) {
    $sourcePath = Join-Path $repositoryRoot $relative
    $destinationPath = Join-Path $sourceRoot $relative
    $destinationParent = Split-Path -Parent $destinationPath
    if (-not (Test-Path -LiteralPath $destinationParent)) {
        $null = New-Item -ItemType Directory -Path $destinationParent -Force
    }
    Copy-Item -LiteralPath $sourcePath -Destination $destinationPath
}

$cargoConfigRoot = Join-Path $sourceRoot ".cargo"
$null = New-Item -ItemType Directory -Path $cargoConfigRoot -Force
$cargoConfig = @'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
'@
Write-Utf8NoBom -Path (Join-Path $cargoConfigRoot "config.toml") -Text $cargoConfig

Push-Location $repositoryRoot
try {
    & cargo vendor --locked --offline --versioned-dirs (Join-Path $sourceRoot "vendor") | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "CARGO_VENDOR_FAILED:$LASTEXITCODE"
    }
} finally {
    Pop-Location
}

Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\verify_portable_full_core.ps1") -Destination (Join-Path $toolsRoot "verify-package.ps1")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\rebuild_portable_full_core.ps1") -Destination (Join-Path $toolsRoot "rebuild-offline.ps1")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\run_growth_supervisor.ps1") -Destination (Join-Path $toolsRoot "run-growth-supervisor.ps1")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\install_growth_supervisor_autostart.ps1") -Destination (Join-Path $toolsRoot "install-growth-autostart.ps1")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\uninstall_growth_supervisor_autostart.ps1") -Destination (Join-Path $toolsRoot "uninstall-growth-autostart.ps1")
Copy-Item -LiteralPath (Join-Path $repositoryRoot "scripts\record_growth_work_event.ps1") -Destination (Join-Path $toolsRoot "record-growth-work-event.ps1")

$readmeTemplate = @'
# B_Core Portable Full Core

This package is the complete Windows x64 portable build of B_Core commit `{{SOURCE_COMMIT}}`.

## Included

- All `{{BINARY_COUNT}}` current workspace executables and tracked source files.
- Locked and vendored Rust dependency closure for network-free reconstruction.
- Self-healing, independent verification, typed code composition, and full-stack/operations knowledge.
- The bounded always-on growth supervisor and its separate deterministic verifier.
- Semantic-deduplicated promotion and bound before/after performance learning.

No raw training dataset or original Synapse knowledge store is included. Knowledge already absorbed into the core remains present in source and sealed artifacts.

## Verify

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\verify-package.ps1 -PackageRoot . -RunSmokeTests
```

## Offline rebuild

Rust 1.96.0 x86_64-pc-windows-msvc and Visual Studio C++ Build Tools are required only for rebuilding:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\rebuild-offline.ps1
```

## Bounded always-on growth

Create a config with explicit watched and state roots:

```powershell
.\bin\b-core-growth-supervisor.exe make-config .\config\growth.json D:\WatchedWorkspace D:\B_Core_Growth_State
.\bin\b-core-growth-supervisor.exe init .\config\growth.json
powershell -ExecutionPolicy Bypass -File .\tools\run-growth-supervisor.ps1 -PackageRoot . -ConfigPath .\config\growth.json
```

Autostart is opt-in. `tools\install-growth-autostart.ps1` registers a limited-privilege ONLOGON task. See `source\docs\b_core_growth_supervisor.md` for trust boundaries, learning-value rules, crash recovery, plateau waiting, and work-event integration.

Git commits and sealed artifacts remain scientific authority. Portable binaries, runtime memory, and build caches do not replace that authority.
'@
$readme = $readmeTemplate.Replace("{{SOURCE_COMMIT}}", $sourceCommit).Replace("{{BINARY_COUNT}}", [string]$expectedBinaries.Count)
Write-Utf8NoBom -Path (Join-Path $packageRoot "README.md") -Text $readme

$rebuildOutput = @()
if ($fastPackaging) {
    $baseBinRoot = Join-Path $basePackage "bin"
    if (-not (Test-Path -LiteralPath (Join-Path $basePackage "PACKAGE_MANIFEST.json") -PathType Leaf) -or
        -not (Test-Path -LiteralPath $baseBinRoot -PathType Container) -or
        -not (Test-Path -LiteralPath $refreshBinRoot -PathType Container)) {
        throw "FAST_PACKAGE_REUSE_INPUT_INVALID"
    }
    Copy-Item -Path (Join-Path $baseBinRoot "*") -Destination $binRoot
    $refreshed = @(
        "b-core-growth-supervisor",
        "b-core-growth-verifier",
        "sem26-run",
        "sem26-probe"
    )
    foreach ($name in $refreshed) {
        $binaryPath = Join-Path $refreshBinRoot ("$name.exe")
        if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
            throw "FAST_PACKAGE_REFRESH_BINARY_MISSING:$name"
        }
        Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $binRoot ("$name.exe")) -Force
    }
    $baseManifest = Get-Content -Raw -LiteralPath (Join-Path $basePackage "PACKAGE_MANIFEST.json") | ConvertFrom-Json
    $fastReceipt = [ordered]@{
        schema = "b_core.portable_fast_reuse.v1"
        source_commit = $sourceCommit
        base_package = $basePackage
        base_commit = [string]$baseManifest.source.commit
        refreshed_binaries = $refreshed
        clean_rebuild_run = $false
        full_tests_run = $false
        package_hash_inventory_and_smoke_tests_required = $true
    }
    Write-Utf8NoBom -Path (Join-Path $receiptRoot "fast_reuse.json") -Text ($fastReceipt | ConvertTo-Json -Depth 5)
} else {
    $rebuildReceiptPath = Join-Path $receiptRoot "clean_rebuild.json"
    $rebuildOutput = & (Join-Path $toolsRoot "rebuild-offline.ps1") `
        -PackageRoot $packageRoot `
        -TargetDirectory $buildRoot 2>&1
    $rebuildExitCode = $LASTEXITCODE
    if ($rebuildExitCode -ne 0) {
        throw "CLEAN_REBUILD_FAILED:$rebuildExitCode`n$($rebuildOutput -join "`n")"
    }
    Write-Utf8NoBom -Path $rebuildReceiptPath -Text (($rebuildOutput -join "`n").Trim())

    $releaseRoot = Join-Path $buildRoot "x86_64-pc-windows-msvc\release"
    foreach ($name in $expectedBinaries) {
        $binaryPath = Join-Path $releaseRoot ("$name.exe")
        if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
            throw "BUILT_BINARY_MISSING:$name"
        }
        Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $binRoot ("$name.exe"))
    }
}

$actualBinaryCount = @(Get-ChildItem -LiteralPath $binRoot -Filter "*.exe" -File).Count
if ($actualBinaryCount -ne $expectedBinaries.Count) {
    throw "COPIED_BINARY_COUNT_MISMATCH:$actualBinaryCount"
}

$growthSelfCheckOutput = & (Join-Path $binRoot "b-core-growth-supervisor.exe") self-check 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "GROWTH_SUPERVISOR_SELF_CHECK_FAILED:$LASTEXITCODE`n$($growthSelfCheckOutput -join "`n")"
}
$growthSelfCheck = ($growthSelfCheckOutput -join "`n") | ConvertFrom-Json
if (-not $growthSelfCheck.pass -or
    -not $growthSelfCheck.network_and_llm_disabled -or
    -not $growthSelfCheck.plateau_difficulty_escalation_disabled -or
    -not $growthSelfCheck.prediction_before_composition_enabled -or
    -not $growthSelfCheck.valuable_combination_memory_enabled -or
    $growthSelfCheck.generative_memory_self_application_enabled -or
    -not $growthSelfCheck.heuristic_composition_value_excluded_from_frontier -or
    -not $growthSelfCheck.behavioral_evidence_required_for_generative_self_application -or
    -not $growthSelfCheck.behavioral_composition_execution_enabled -or
    -not $growthSelfCheck.redundant_generative_verifier_search_disabled -or
    -not $growthSelfCheck.classifier_refinement_requires_capability_evidence -or
    -not $growthSelfCheck.classifier_refinement_delta_ledger_enabled -or
    -not $growthSelfCheck.source_patch_diagnostics_use_recent_engine_window -or
    -not $growthSelfCheck.source_synthesis_exhaustion_is_capability_gap -or
    -not $growthSelfCheck.core_self_approval_enabled -or
    -not $growthSelfCheck.autonomous_source_patch_install_enabled -or
    -not $growthSelfCheck.source_patch_rollback_enabled -or
    -not $growthSelfCheck.semantic_duplicate_promotion_blocked -or
    -not $growthSelfCheck.measured_performance_evidence_supported) {
    throw "GROWTH_SUPERVISOR_BOUNDARY_CHECK_FAILED"
}
Write-Utf8NoBom -Path (Join-Path $receiptRoot "growth_supervisor_self_check.json") -Text ($growthSelfCheck | ConvertTo-Json -Depth 6)

$objdump = Get-Command llvm-objdump -ErrorAction SilentlyContinue
$runtimeAudit = @()
if ($null -ne $objdump -and -not $fastPackaging) {
    foreach ($name in $expectedBinaries) {
        $binaryPath = Join-Path $binRoot ("$name.exe")
        $imports = (& $objdump.Source -p $binaryPath | Select-String -Pattern "DLL Name:" | ForEach-Object { $_.Line.Trim() })
        $forbidden = @($imports | Where-Object { $_ -match "(?i)(VCRUNTIME|MSVCP|ucrtbase)" })
        if ($forbidden.Count -ne 0) {
            throw "DYNAMIC_MSVC_RUNTIME_DEPENDENCY:$name`:$($forbidden -join ',')"
        }
        $runtimeAudit += [ordered]@{
            binary = $name
            forbidden_runtime_imports = 0
        }
    }
}
Write-Utf8NoBom -Path (Join-Path $receiptRoot "static_crt_audit.json") -Text (([ordered]@{
    schema = "b_core.static_crt_audit.v1"
    tool = if ($null -eq $objdump) { "not_available" } else { $objdump.Source }
    binaries_audited = $runtimeAudit.Count
    pass = if ($fastPackaging) { $null } else { ($runtimeAudit.Count -eq $expectedBinaries.Count) }
    results = $runtimeAudit
} | ConvertTo-Json -Depth 6))

$sourceFileCount = @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -File | Where-Object {
    $_.FullName -notlike ((Join-Path $sourceRoot "vendor") + "\*")
}).Count
$vendorFileCount = @(Get-ChildItem -LiteralPath (Join-Path $sourceRoot "vendor") -Recurse -File).Count

$inventory = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File |
    Where-Object { $_.Name -ne "PACKAGE_MANIFEST.json" } |
    ForEach-Object {
        [ordered]@{
            path = Get-NormalizedRelativePath -Root $packageRoot -Path $_.FullName
            bytes = [int64]$_.Length
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    } | Sort-Object path)

$manifest = [ordered]@{
    schema = "b_core.portable_full_core.v1"
    package_id = "B_Core_PORTABLE_$($sourceCommit.Substring(0, 7))_WIN64"
    created_utc = [DateTime]::UtcNow.ToString("o")
    source = [ordered]@{
        commit = $sourceCommit
        branch = $sourceBranch
        tracked_file_count = $trackedFiles.Count
    }
    build = [ordered]@{
        toolchain = $toolchain
        target_triple = "x86_64-pc-windows-msvc"
        profile = "release"
        cargo_locked = $true
        cargo_offline = $true
        dependency_closure_vendored = $true
        static_crt = $true
        linker_reproducible = $true
        path_remapping = $true
        clean_reconstruction_pass = (-not $fastPackaging)
        full_workspace_all_targets_all_features_tests_pass = (-not $fastPackaging)
        fast_verified_binary_reuse = $fastPackaging
    }
    binaries = [ordered]@{
        count = $expectedBinaries.Count
        expected_names = $expectedBinaries
        smoke_test_names = @("core-x0-canary", "generic-capability-canary", "language-adapter-canary")
    }
    contents = [ordered]@{
        source_file_count = $sourceFileCount
        vendor_file_count = $vendorFileCount
        file_count_excluding_manifest = $inventory.Count
    }
    growth_supervisor = [ordered]@{
        included = $true
        executable = "bin/b-core-growth-supervisor.exe"
        independent_verifier = "bin/b-core-growth-verifier.exe"
        self_check_pass = $true
        autonomous_campaigns_bounded = $true
        plateau_difficulty_escalation_disabled = $true
        raw_source_retention = $false
        codex_runtime_dependency = $false
        external_llm_runtime_dependency = $false
        network_runtime_dependency = $false
        semantic_duplicate_promotion_blocked = $true
        measured_performance_evidence_supported = $true
    }
    authority = [ordered]@{
        git_and_sealed_artifacts_authoritative = $true
        warm_state_is_semantic_authority = $false
        warm_state_is_research_authority = $false
        raw_training_data_files = 0
        original_synapse_knowledge_store_included = $false
    }
    files = $inventory
}
Write-Utf8NoBom -Path (Join-Path $packageRoot "PACKAGE_MANIFEST.json") -Text ($manifest | ConvertTo-Json -Depth 9)

$verificationOutput = & (Join-Path $toolsRoot "verify-package.ps1") -PackageRoot $packageRoot -RunSmokeTests 2>&1
$verificationExitCode = $LASTEXITCODE
if ($verificationExitCode -ne 0) {
    throw "PACKAGE_VERIFICATION_FAILED:$verificationExitCode`n$($verificationOutput -join "`n")"
}

$archiveParent = Split-Path -Parent $archive
if (-not (Test-Path -LiteralPath $archiveParent)) {
    $null = New-Item -ItemType Directory -Path $archiveParent -Force
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($packageRoot, $archive, [IO.Compression.CompressionLevel]::Optimal, $true)
$archiveInfo = Get-Item -LiteralPath $archive
$archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
$shaPath = "$archive.sha256"
Write-Utf8NoBom -Path $shaPath -Text ("{0}  {1}`n" -f $archiveHash, $archiveInfo.Name)

[ordered]@{
    schema = "b_core.portable_build_receipt.v1"
    package_root = $packageRoot
    package_id = $manifest.package_id
    source_commit = $sourceCommit
    archive = $archive
    archive_bytes = [int64]$archiveInfo.Length
    archive_sha256 = $archiveHash
    archive_sha256_file = $shaPath
    binary_count = $expectedBinaries.Count
    source_file_count = $sourceFileCount
    vendor_file_count = $vendorFileCount
    clean_reconstruction_pass = (-not $fastPackaging)
    full_tests_pass = (-not $fastPackaging)
    fast_verified_binary_reuse = $fastPackaging
    package_verification = ($verificationOutput -join "`n") | ConvertFrom-Json
    disposable_build_target = $buildRoot
} | ConvertTo-Json -Depth 10
