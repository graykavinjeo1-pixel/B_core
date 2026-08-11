[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [switch]$RunSmokeTests
)

$ErrorActionPreference = "Stop"

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

$root = [IO.Path]::GetFullPath($PackageRoot).TrimEnd('\')
if (-not (Test-Path -LiteralPath $root -PathType Container)) {
    throw "PACKAGE_ROOT_MISSING:$root"
}

$manifestPath = Join-Path $root "PACKAGE_MANIFEST.json"
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "PACKAGE_MANIFEST_MISSING:$manifestPath"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.schema -ne "b_core.portable_full_core.v1") {
    throw "PACKAGE_SCHEMA_UNSUPPORTED:$($manifest.schema)"
}
if ($manifest.authority.warm_state_is_semantic_authority -ne $false -or
    $manifest.authority.warm_state_is_research_authority -ne $false) {
    throw "PACKAGE_AUTHORITY_BOUNDARY_INVALID"
}
if ([int]$manifest.authority.raw_training_data_files -ne 0) {
    throw "RAW_TRAINING_DATA_PRESENT"
}

$declared = @{}
foreach ($entry in @($manifest.files)) {
    $relative = [string]$entry.path
    if ([string]::IsNullOrWhiteSpace($relative) -or
        $relative.Contains('..') -or
        [IO.Path]::IsPathRooted($relative)) {
        throw "INVALID_MANIFEST_PATH:$relative"
    }
    if ($declared.ContainsKey($relative)) {
        throw "DUPLICATE_MANIFEST_PATH:$relative"
    }
    $declared[$relative] = $entry
}

$actualFiles = @(Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object {
    $_.FullName -ne $manifestPath
})
if ($actualFiles.Count -ne $declared.Count) {
    throw "FILE_COUNT_MISMATCH:actual=$($actualFiles.Count):declared=$($declared.Count)"
}

foreach ($file in $actualFiles) {
    $relative = Get-NormalizedRelativePath -Root $root -Path $file.FullName
    if (-not $declared.ContainsKey($relative)) {
        throw "UNDECLARED_FILE:$relative"
    }
    $entry = $declared[$relative]
    if ([int64]$entry.bytes -ne [int64]$file.Length) {
        throw "FILE_SIZE_MISMATCH:$relative"
    }
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne ([string]$entry.sha256).ToLowerInvariant()) {
        throw "FILE_HASH_MISMATCH:$relative"
    }
}

foreach ($relative in $declared.Keys) {
    $nativeRelative = $relative.Replace('/', '\')
    $candidate = [IO.Path]::GetFullPath((Join-Path $root $nativeRelative))
    $null = Get-NormalizedRelativePath -Root $root -Path $candidate
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "DECLARED_FILE_MISSING:$relative"
    }
}

$expectedBinaries = @($manifest.binaries.expected_names | ForEach-Object { [string]$_ } | Sort-Object)
$actualBinaries = @(Get-ChildItem -LiteralPath (Join-Path $root "bin") -Filter "*.exe" -File |
    ForEach-Object { $_.BaseName } | Sort-Object)
if ($actualBinaries.Count -ne [int]$manifest.binaries.count -or
    $expectedBinaries.Count -ne [int]$manifest.binaries.count) {
    throw "BINARY_COUNT_MISMATCH:actual=$($actualBinaries.Count):expected=$($expectedBinaries.Count)"
}
if ((Compare-Object -ReferenceObject $expectedBinaries -DifferenceObject $actualBinaries).Count -ne 0) {
    throw "BINARY_NAME_SET_MISMATCH"
}
if ($manifest.growth_supervisor.included -ne $true -or
    $expectedBinaries -notcontains "b-core-growth-supervisor" -or
    $expectedBinaries -notcontains "b-core-growth-verifier" -or
    $manifest.growth_supervisor.raw_source_retention -ne $false -or
    $manifest.growth_supervisor.codex_runtime_dependency -ne $false -or
    $manifest.growth_supervisor.external_llm_runtime_dependency -ne $false -or
    $manifest.growth_supervisor.network_runtime_dependency -ne $false -or
    $manifest.growth_supervisor.semantic_duplicate_promotion_blocked -ne $true -or
    $manifest.growth_supervisor.measured_performance_evidence_supported -ne $true) {
    throw "GROWTH_SUPERVISOR_MANIFEST_BOUNDARY_INVALID"
}

$smokeResults = @()
if ($RunSmokeTests) {
    foreach ($name in @($manifest.binaries.smoke_test_names)) {
        $binaryPath = Join-Path $root ("bin\{0}.exe" -f $name)
        $output = & $binaryPath 2>&1
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0) {
            throw "SMOKE_TEST_FAILED:$name`:exit=$exitCode`:output=$($output -join ' ')"
        }
        $smokeResults += [ordered]@{
            binary = [string]$name
            exit_code = $exitCode
            output_sha256 = [BitConverter]::ToString(
                [Security.Cryptography.SHA256]::Create().ComputeHash(
                    [Text.Encoding]::UTF8.GetBytes(($output -join "`n"))
                )
            ).Replace('-', '').ToLowerInvariant()
        }
    }
    $growthBinary = Join-Path $root "bin\b-core-growth-supervisor.exe"
    $growthOutput = & $growthBinary self-check 2>&1
    $growthExitCode = $LASTEXITCODE
    if ($growthExitCode -ne 0) {
        throw "GROWTH_SUPERVISOR_SELF_CHECK_FAILED:exit=$growthExitCode"
    }
    $growthCheck = ($growthOutput -join "`n") | ConvertFrom-Json
    if (-not $growthCheck.pass -or
        -not $growthCheck.raw_source_retention_forbidden -or
        -not $growthCheck.network_and_llm_disabled -or
        -not $growthCheck.plateau_difficulty_escalation_disabled -or
        -not $growthCheck.core_self_approval_enabled -or
        -not $growthCheck.autonomous_source_patch_install_enabled -or
        -not $growthCheck.source_patch_rollback_enabled -or
        -not $growthCheck.semantic_duplicate_promotion_blocked -or
        -not $growthCheck.measured_performance_evidence_supported) {
        throw "GROWTH_SUPERVISOR_SELF_CHECK_BOUNDARY_INVALID"
    }
    $smokeResults += [ordered]@{
        binary = "b-core-growth-supervisor"
        operation = "self-check"
        exit_code = $growthExitCode
        output_sha256 = [BitConverter]::ToString(
            [Security.Cryptography.SHA256]::Create().ComputeHash(
                [Text.Encoding]::UTF8.GetBytes(($growthOutput -join "`n"))
            )
        ).Replace('-', '').ToLowerInvariant()
    }
}

[ordered]@{
    schema = "b_core.portable_verification_receipt.v1"
    verified = $true
    package_id = [string]$manifest.package_id
    source_commit = [string]$manifest.source.commit
    file_count = $declared.Count
    binary_count = $actualBinaries.Count
    raw_training_data_files = [int]$manifest.authority.raw_training_data_files
    manifest_sha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    smoke_tests_run = [bool]$RunSmokeTests
    smoke_results = $smokeResults
} | ConvertTo-Json -Depth 8
