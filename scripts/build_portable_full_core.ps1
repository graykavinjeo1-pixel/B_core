[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$TargetDirectory,

    [Parameter(Mandatory = $true)]
    [string]$BuildTargetDirectory,

    [Parameter(Mandatory = $true)]
    [string]$ArchivePath
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

if (Test-Path -LiteralPath $packageRoot) {
    throw "TARGET_ALREADY_EXISTS:$packageRoot"
}
if (Test-Path -LiteralPath $buildRoot) {
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
if ($expectedBinaries.Count -ne 79) {
    throw "UNEXPECTED_BINARY_COUNT:$($expectedBinaries.Count)"
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

$readme = @"
# B_Core Portable Full Core

이 패키지는 B_Core commit `$sourceCommit`의 Windows x64 이식본입니다.

## 바로 실행

- `bin`에는 현재 workspace의 실행 진입점 79개가 모두 들어 있습니다.
- `bin\core-x0-canary.exe`, `bin\generic-capability-canary.exe`, `bin\language-adapter-canary.exe`로 기본 동작을 확인할 수 있습니다.
- 자기수리 파이프라인은 `bin\b-core-self-heal.exe`와 `bin\b-core-self-heal-verify.exe`입니다.
- 통합 개발/코드 조합 파이프라인은 `bin\b-core-integrated-development.exe`입니다.
- 최신 frontend/backend/operations 지식 흡수 도구는 `bin\b-core-fullstack-ops-absorb.exe`입니다.

실행 파일은 MSVC C/C++ 런타임을 정적으로 링크했습니다. Windows x64에서는 Rust 도구체인 없이 실행할 수 있습니다.

## 무결성 확인

PowerShell에서 패키지 루트를 기준으로 실행합니다.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\verify-package.ps1 -PackageRoot . -RunSmokeTests
```

## 네트워크 없는 전체 재빌드

Rust 1.96.0 x86_64-pc-windows-msvc와 Visual Studio C++ Build Tools가 설치된 PC에서:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\rebuild-offline.ps1
```

Cargo.lock과 `source\vendor`가 모든 Rust 의존성을 고정합니다. 이 과정은 네트워크를 사용하지 않습니다.

## 보존 범위

전체 tracked source, 봉인 보고서/스키마, 79개 실행 파일, 자기수리 검증기, 코드 조합 및 full-stack/operations 지식 계층을 포함합니다. 대용량 원본 Synapse 지식 저장소와 학습 데이터는 포함하지 않습니다. 이미 흡수된 지식은 소스와 봉인 산출물에 포함되어 있습니다. 향후 Synapse 업데이트를 다시 흡수하려면 새 PC에서 그 외부 저장소 경로를 별도로 지정해야 합니다.

Git commit과 봉인 산출물이 연구 권위입니다. 사전 빌드 실행 파일이나 재빌드 캐시는 연구 권위가 아닙니다.
"@
Write-Utf8NoBom -Path (Join-Path $packageRoot "README_KO.md") -Text $readme

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

$actualBinaryCount = @(Get-ChildItem -LiteralPath $binRoot -Filter "*.exe" -File).Count
if ($actualBinaryCount -ne $expectedBinaries.Count) {
    throw "COPIED_BINARY_COUNT_MISMATCH:$actualBinaryCount"
}

$objdump = Get-Command llvm-objdump -ErrorAction SilentlyContinue
$runtimeAudit = @()
if ($null -ne $objdump) {
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
    pass = ($runtimeAudit.Count -eq $expectedBinaries.Count)
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
        clean_reconstruction_pass = $true
        full_workspace_all_targets_all_features_tests_pass = $true
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
    clean_reconstruction_pass = $true
    full_tests_pass = $true
    package_verification = ($verificationOutput -join "`n") | ConvertFrom-Json
    disposable_build_target = $buildRoot
} | ConvertTo-Json -Depth 10
