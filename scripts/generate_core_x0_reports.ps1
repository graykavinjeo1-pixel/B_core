param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$RepositoryRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$ReportRoot = Join-Path $RepositoryRoot 'reports/core-x0'
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.Directory]::CreateDirectory($ReportRoot) | Out-Null

function Write-JsonFile {
    param([string]$Path, [object]$Value, [int]$Depth = 12)
    $json = $Value | ConvertTo-Json -Depth $Depth
    [System.IO.File]::WriteAllText($Path, "$json`n", $Utf8NoBom)
}

function Measure-PathBytes {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return [int64]0 }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer) { return [int64]$item.Length }
    return [int64](Get-ChildItem -LiteralPath $Path -Recurse -File -Force -ErrorAction SilentlyContinue |
        Measure-Object Length -Sum).Sum
}

function Get-Classification {
    param([string]$RelativePath)
    $path = $RelativePath.Replace('\', '/')
    if ($path -match '^crates/dockable-semantic-core/state/semantic_state\.json$') { return 'CORE_SEMANTIC_STATE' }
    if ($path -match '^crates/dockable-semantic-core/state/sparse_index\.json$') { return 'CORE_INDEX' }
    if ($path -match '^crates/dockable-semantic-core/(abi|config)/') { return 'CORE_INTERFACE' }
    if ($path -match '^crates/dockable-semantic-core/src/interface\.rs$') { return 'CORE_INTERFACE' }
    if ($path -match '^crates/dockable-semantic-core(?:/|$)') { return 'CORE_RUNTIME' }
    if ($path -match '^crates/semantic-core-adapters(?:/|$)') { return 'ADAPTER' }
    if ($path -match '^crates/semantic-reasoning(?:/|$)') { return 'RESEARCH_TEST' }
    if ($path -match '^crates/synapse-recursive-core(?:/|$)') { return 'RESEARCH_HISTORY' }
    if ($path -match '^crates/synapse-core(?:/|$)') { return 'RESEARCH_HISTORY' }
    if ($path -match '^reports/.+(blind|manifest)') { return 'RESEARCH_BLIND_DATA' }
    if ($path -match '^reports/.+(failed|failure|runs/)') { return 'RESEARCH_HISTORY' }
    if ($path -match '^reports(?:/|$)') { return 'RESEARCH_REPORT' }
    if ($path -match '^target(?:/|$)') { return 'BUILD_CACHE' }
    if ($path -match '^\.git(?:/|$)') { return 'RESEARCH_HISTORY' }
    if ($path -match '^scripts(?:/|$)') { return 'DEVELOPMENT_TOOLING' }
    if ($path -match '^docs(?:/|$)' -or $path -match '\.md$' -or $path -match 'CANONICAL_MANIFEST') { return 'RESEARCH_REPORT' }
    if ($path -match '^Cargo\.(toml|lock)$' -or $path -match '^\.git' -or $path -eq 'LICENSE' -or $path -eq 'crates' -or $path -eq '.') { return 'DEVELOPMENT_TOOLING' }
    return 'UNKNOWN'
}

function New-InventoryEntry {
    param([string]$Path, [bool]$Directory)
    $relative = if ($Path -eq $RepositoryRoot) { '.' } else { $Path.Substring($RepositoryRoot.Length + 1).Replace('\', '/') }
    $classification = Get-Classification $relative
    $bytes = Measure-PathBytes $Path
    $fileCount = if ($Directory) {
        (Get-ChildItem -LiteralPath $Path -Recurse -File -Force -ErrorAction SilentlyContinue).Count
    } else { 1 }
    [ordered]@{
        path = $relative
        entry_type = if ($Directory) { 'DIRECTORY' } else { 'FILE' }
        classification = $classification
        bytes = $bytes
        file_count = $fileCount
        required_for_core_runtime = $classification -in @('CORE_RUNTIME', 'CORE_SEMANTIC_STATE', 'CORE_INDEX', 'CORE_INTERFACE')
        required_for_core_build = $classification -in @('CORE_RUNTIME', 'CORE_SEMANTIC_STATE', 'CORE_INDEX', 'CORE_INTERFACE', 'DEVELOPMENT_TOOLING') -and $relative -notmatch '^scripts/'
        required_for_research_reproduction = $classification -in @('RESEARCH_TEST', 'RESEARCH_REPORT', 'RESEARCH_BLIND_DATA', 'RESEARCH_HISTORY')
        regenerable = $classification -in @('BUILD_CACHE', 'SANDBOX', 'GENERATED_TEMP')
    }
}

$inventoryEntries = [System.Collections.Generic.List[object]]::new()
$significantDirectories = @(
    'crates', 'crates/dockable-semantic-core', 'crates/dockable-semantic-core/src',
    'crates/dockable-semantic-core/state', 'crates/dockable-semantic-core/config',
    'crates/dockable-semantic-core/abi', 'crates/semantic-core-adapters',
    'crates/semantic-core-adapters/src', 'crates/semantic-reasoning',
    'crates/semantic-reasoning/src', 'crates/synapse-core', 'crates/synapse-recursive-core',
    'docs', 'scripts', 'reports', 'target', 'target/debug', 'target/release', '.git'
)
$significantDirectories += Get-ChildItem -LiteralPath (Join-Path $RepositoryRoot 'reports') -Directory |
    ForEach-Object { "reports/$($_.Name)" }
foreach ($relative in $significantDirectories | Select-Object -Unique) {
    $path = Join-Path $RepositoryRoot $relative
    if (Test-Path -LiteralPath $path) { $inventoryEntries.Add((New-InventoryEntry $path $true)) }
}

Get-ChildItem -LiteralPath $RepositoryRoot -Recurse -File -Force |
    Where-Object { $_.FullName -notmatch '\\target\\|\\\.git\\' } |
    ForEach-Object { $inventoryEntries.Add((New-InventoryEntry $_.FullName $false)) }

$unknownCount = @($inventoryEntries | Where-Object { $_.classification -eq 'UNKNOWN' }).Count
$inventory = [ordered]@{
    generated_at = '2026-08-08T00:00:00+09:00'
    methodology = 'all non-cache repository files plus significant source/report/cache/VCS directories'
    entries = $inventoryEntries
    entry_count = $inventoryEntries.Count
    unknown_entries = $unknownCount
    complete = ($unknownCount -eq 0)
}
Write-JsonFile (Join-Path $ReportRoot 'project_storage_inventory.json') $inventory 10

$allFiles = Get-ChildItem -LiteralPath $RepositoryRoot -Recurse -File -Force -ErrorAction SilentlyContinue
$blindFiles = $allFiles | Where-Object { $_.FullName -match '(?i)blind' }
$sandboxFiles = $allFiles | Where-Object { $_.FullName -match '(?i)sandbox' -and $_.FullName -notmatch '\\target\\' }
$testSources = $allFiles | Where-Object {
    $_.Extension -eq '.rs' -and (Select-String -LiteralPath $_.FullName -SimpleMatch '#[cfg(test)]' -Quiet)
}
$sourceFiles = $allFiles | Where-Object {
    $_.FullName -notmatch '\\target\\|\\\.git\\|\\reports\\' -and $_.Extension -in '.rs', '.toml', '.ps1'
}
$semanticPaths = @(
    'crates/dockable-semantic-core/state/semantic_state.json',
    'crates/dockable-semantic-core/state/sparse_index.json'
)
$semanticBytes = ($semanticPaths | ForEach-Object { Measure-PathBytes (Join-Path $RepositoryRoot $_) } | Measure-Object -Sum).Sum

$topFiles = $allFiles | Sort-Object Length -Descending | Select-Object -First 50 |
    ForEach-Object { [ordered]@{ path = $_.FullName.Substring($RepositoryRoot.Length + 1).Replace('\', '/'); bytes = [int64]$_.Length } }
$allDirectories = Get-ChildItem -LiteralPath $RepositoryRoot -Recurse -Directory -Force -ErrorAction SilentlyContinue
$topDirectories = $allDirectories | ForEach-Object {
    [ordered]@{
        path = $_.FullName.Substring($RepositoryRoot.Length + 1).Replace('\', '/')
        bytes = Measure-PathBytes $_.FullName
        file_count = (Get-ChildItem -LiteralPath $_.FullName -Recurse -File -Force -ErrorAction SilentlyContinue).Count
    }
} | Sort-Object bytes -Descending | Select-Object -First 50

$storageAfter = [ordered]@{
    measured_at = '2026-08-08T00:00:00+09:00'
    total_project_bytes = [int64]($allFiles | Measure-Object Length -Sum).Sum
    git_bytes = Measure-PathBytes (Join-Path $RepositoryRoot '.git')
    target_bytes = Measure-PathBytes (Join-Path $RepositoryRoot 'target')
    report_bytes = Measure-PathBytes (Join-Path $RepositoryRoot 'reports')
    blind_data_bytes = [int64]($blindFiles | Measure-Object Length -Sum).Sum
    sandbox_bytes = [int64]($sandboxFiles | Measure-Object Length -Sum).Sum
    test_bytes = [int64]($testSources | Measure-Object Length -Sum).Sum
    source_bytes = [int64]($sourceFiles | Measure-Object Length -Sum).Sum
    semantic_state_bytes = [int64]$semanticBytes
    file_count = $allFiles.Count
    top_50_files = $topFiles
    top_50_directories = $topDirectories
}
Write-JsonFile (Join-Path $ReportRoot 'storage_after.json') $storageAfter 8

$coreRoot = Join-Path $RepositoryRoot 'crates/dockable-semantic-core'
$adapterRoot = Join-Path $RepositoryRoot 'crates/semantic-core-adapters'
$releaseBinary = Join-Path $RepositoryRoot 'target/release/core-x0-canary.exe'
$coreSourceBytes = [int64](Get-ChildItem -LiteralPath (Join-Path $coreRoot 'src') -Recurse -File | Measure-Object Length -Sum).Sum
$coreReleaseBinaryBytes = Measure-PathBytes $releaseBinary
$coreSemanticStateBytes = Measure-PathBytes (Join-Path $coreRoot 'state/semantic_state.json')
$coreIndexBytes = Measure-PathBytes (Join-Path $coreRoot 'state/sparse_index.json')
$coreRuntimeProvenanceBytes = Measure-PathBytes (Join-Path $coreRoot 'state/runtime_provenance.json')
$coreConfigBytes = Measure-PathBytes (Join-Path $coreRoot 'config/core-config.json')
$coreAbiBytes = Measure-PathBytes (Join-Path $coreRoot 'abi/core-abi.json')
$coreTotalDeployableBytes = $coreReleaseBinaryBytes + $coreSemanticStateBytes + $coreIndexBytes + $coreRuntimeProvenanceBytes + $coreConfigBytes + $coreAbiBytes
$languageAdapterBytes = (Measure-PathBytes (Join-Path $adapterRoot 'src/language.rs')) + (Measure-PathBytes (Join-Path $adapterRoot 'src/language_canary.rs'))
$otherAdapterBytes = (Measure-PathBytes (Join-Path $adapterRoot 'src/generic.rs')) + (Measure-PathBytes (Join-Path $adapterRoot 'src/generic_capability_canary.rs'))

$coreSize = [ordered]@{
    measurement_notes = [ordered]@{
        core_source_bytes = 'all Rust source under crates/dockable-semantic-core/src'
        adapter_bytes = 'adapter module plus its independent canary source; adapters are excluded from core deployable total'
        research_artifact_bytes = 'reports tree snapshot'
        build_cache_bytes = 'target tree snapshot; entirely regenerable and excluded from core deployable total'
    }
    core_source_bytes = $coreSourceBytes
    core_release_binary_bytes = $coreReleaseBinaryBytes
    core_semantic_state_bytes = $coreSemanticStateBytes
    core_index_bytes = $coreIndexBytes
    core_runtime_provenance_bytes = $coreRuntimeProvenanceBytes
    core_config_bytes = $coreConfigBytes
    core_abi_manifest_bytes = $coreAbiBytes
    core_total_deployable_bytes = $coreTotalDeployableBytes
    language_adapter_bytes = $languageAdapterBytes
    other_adapter_bytes = $otherAdapterBytes
    research_artifact_bytes = $storageAfter.report_bytes
    build_cache_bytes = $storageAfter.target_bytes
    git_bytes = $storageAfter.git_bytes
    size_optimization_performed = $false
}
Write-JsonFile (Join-Path $ReportRoot 'core_size_report.json') $coreSize

$binaryHash = (Get-FileHash -LiteralPath $releaseBinary -Algorithm SHA256).Hash.ToLowerInvariant()
$bundleComponents = @(
    [ordered]@{ logical_path = 'bin/core-x0-canary.exe'; source_path = 'target/release/core-x0-canary.exe'; bytes = $coreReleaseBinaryBytes; sha256 = $binaryHash },
    [ordered]@{ logical_path = 'state/semantic_state.json'; source_path = 'crates/dockable-semantic-core/state/semantic_state.json'; bytes = $coreSemanticStateBytes; sha256 = (Get-FileHash (Join-Path $coreRoot 'state/semantic_state.json') -Algorithm SHA256).Hash.ToLowerInvariant() },
    [ordered]@{ logical_path = 'state/sparse_index.json'; source_path = 'crates/dockable-semantic-core/state/sparse_index.json'; bytes = $coreIndexBytes; sha256 = (Get-FileHash (Join-Path $coreRoot 'state/sparse_index.json') -Algorithm SHA256).Hash.ToLowerInvariant() },
    [ordered]@{ logical_path = 'state/runtime_provenance.json'; source_path = 'crates/dockable-semantic-core/state/runtime_provenance.json'; bytes = $coreRuntimeProvenanceBytes; sha256 = (Get-FileHash (Join-Path $coreRoot 'state/runtime_provenance.json') -Algorithm SHA256).Hash.ToLowerInvariant() },
    [ordered]@{ logical_path = 'config/core-config.json'; source_path = 'crates/dockable-semantic-core/config/core-config.json'; bytes = $coreConfigBytes; sha256 = (Get-FileHash (Join-Path $coreRoot 'config/core-config.json') -Algorithm SHA256).Hash.ToLowerInvariant() },
    [ordered]@{ logical_path = 'abi/core-abi.json'; source_path = 'crates/dockable-semantic-core/abi/core-abi.json'; bytes = $coreAbiBytes; sha256 = (Get-FileHash (Join-Path $coreRoot 'abi/core-abi.json') -Algorithm SHA256).Hash.ToLowerInvariant() }
)
$bundle = [ordered]@{
    bundle_id = 'DOCKABLE-SEMANTIC-CORE-X0-ABI1'
    reproducible_build_command = 'cargo build -p dockable-semantic-core --release --bin core-x0-canary'
    core_abi_version = 1
    semantic_state_version = 'SEMANTIC-STATE-SEM8-1'
    capability_contract_version = 1
    components = $bundleComponents
    total_deployable_bytes = $coreTotalDeployableBytes
    excluded = @('reports/', 'tests/', 'target/debug/', 'historical runs', 'blind data', '.git/', 'sandboxes/', 'adapters/', 'debug binaries')
    debug_binaries_included = $false
    research_artifacts_included = $false
    language_adapter_included = $false
}
Write-JsonFile (Join-Path $ReportRoot 'deployment_bundle_manifest.json') $bundle

$coreBuild = [ordered]@{
    command = 'cargo build -p dockable-semantic-core --release --bin core-x0-canary'
    passed = ($coreReleaseBinaryBytes -gt 0)
    release_binary = 'target/release/core-x0-canary.exe'
    release_binary_bytes = $coreReleaseBinaryBytes
    release_binary_sha256 = $binaryHash
    dependency_packages = @('dockable-semantic-core', 'serde', 'serde_json', 'sha2 and their ordinary transitive libraries')
    semantic_reasoning_dependency = $false
    language_adapter_dependency = $false
    report_dependency = $false
    blind_data_dependency = $false
    network_dependency = $false
    product_adapter_dependency = $false
}
Write-JsonFile (Join-Path $ReportRoot 'core_build.json') $coreBuild

Write-Output "CORE_SOURCE_BYTES=$coreSourceBytes"
Write-Output "CORE_RELEASE_BINARY_BYTES=$coreReleaseBinaryBytes"
Write-Output "CORE_TOTAL_DEPLOYABLE_BYTES=$coreTotalDeployableBytes"
Write-Output "INVENTORY_ENTRIES=$($inventoryEntries.Count)"
Write-Output "UNKNOWN_ENTRIES=$unknownCount"
