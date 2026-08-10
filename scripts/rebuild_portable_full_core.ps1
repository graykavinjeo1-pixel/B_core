[CmdletBinding()]
param(
    [string]$PackageRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..")),
    [string]$TargetDirectory = "",
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

$root = [IO.Path]::GetFullPath($PackageRoot).TrimEnd('\')
$source = Join-Path $root "source"
if (-not (Test-Path -LiteralPath (Join-Path $source "Cargo.toml") -PathType Leaf)) {
    throw "PORTABLE_SOURCE_MISSING:$source"
}
if (-not (Test-Path -LiteralPath (Join-Path $source "vendor") -PathType Container)) {
    throw "VENDORED_DEPENDENCIES_MISSING:$source\vendor"
}

if ([string]::IsNullOrWhiteSpace($TargetDirectory)) {
    $TargetDirectory = Join-Path $root "rebuild-target"
}
$target = [IO.Path]::GetFullPath($TargetDirectory)

$rustcIdentity = (& rustc -vV) -join "`n"
if ($LASTEXITCODE -ne 0) {
    throw "RUSTC_NOT_READY"
}
if ($rustcIdentity -notmatch "release: 1\.96\.0" -or
    $rustcIdentity -notmatch "host: x86_64-pc-windows-msvc") {
    throw "TOOLCHAIN_MISMATCH:requires rustc 1.96.0 x86_64-pc-windows-msvc"
}

$cargoHome = if ($env:CARGO_HOME) {
    [IO.Path]::GetFullPath($env:CARGO_HOME)
} else {
    [IO.Path]::GetFullPath((Join-Path ([Environment]::GetFolderPath('UserProfile')) ".cargo"))
}
$rustupHome = if ($env:RUSTUP_HOME) {
    [IO.Path]::GetFullPath($env:RUSTUP_HOME)
} else {
    [IO.Path]::GetFullPath((Join-Path ([Environment]::GetFolderPath('UserProfile')) ".rustup"))
}

$env:SOURCE_DATE_EPOCH = "1786132966"
$env:CARGO_INCREMENTAL = "0"
$env:CARGO_TARGET_DIR = $target
$env:CARGO_NET_OFFLINE = "true"
$env:CARGO_ENCODED_RUSTFLAGS = @(
    "-C",
    "target-feature=+crt-static",
    "-C",
    "link-arg=/Brepro",
    "--remap-path-prefix=$source=/b_core",
    "--remap-path-prefix=$cargoHome=/cargo_home",
    "--remap-path-prefix=$rustupHome=/rustup_home"
) -join [char]0x1f
Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue

Push-Location $source
try {
    & cargo build --quiet --workspace --bins --release --target x86_64-pc-windows-msvc --locked --offline
    if ($LASTEXITCODE -ne 0) {
        throw "PORTABLE_RELEASE_BUILD_FAILED:$LASTEXITCODE"
    }
    if (-not $SkipTests) {
        & cargo test --quiet --workspace --all-targets --all-features --target x86_64-pc-windows-msvc --locked --offline
        if ($LASTEXITCODE -ne 0) {
            throw "PORTABLE_TEST_FAILED:$LASTEXITCODE"
        }
    }
} finally {
    Pop-Location
}

[ordered]@{
    schema = "b_core.portable_rebuild_receipt.v1"
    rebuilt = $true
    source = $source
    target = $target
    target_triple = "x86_64-pc-windows-msvc"
    static_crt = $true
    locked = $true
    offline = $true
    tests_run = (-not $SkipTests)
    toolchain = $rustcIdentity
} | ConvertTo-Json -Depth 5
