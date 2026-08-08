[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$TargetDirectory
)

$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$targetRoot = [IO.Path]::GetFullPath($TargetDirectory)
$userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
$cargoHome = if ($env:CARGO_HOME) {
    [IO.Path]::GetFullPath($env:CARGO_HOME)
} else {
    [IO.Path]::GetFullPath((Join-Path $userProfile ".cargo"))
}
$rustupHome = if ($env:RUSTUP_HOME) {
    [IO.Path]::GetFullPath($env:RUSTUP_HOME)
} else {
    [IO.Path]::GetFullPath((Join-Path $userProfile ".rustup"))
}

$rustcIdentity = (& rustc -vV) -join "`n"
if ($LASTEXITCODE -ne 0) {
    throw "rustc identity query failed"
}
if ($rustcIdentity -notmatch "release: 1\.96\.0" -or
    $rustcIdentity -notmatch "commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96") {
    throw "portable R0 requires rustc 1.96.0 (ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96)"
}

$encodedFlags = @(
    "-C",
    "link-arg=/Brepro",
    "--remap-path-prefix=$repositoryRoot=/b_core",
    "--remap-path-prefix=$cargoHome=/cargo_home",
    "--remap-path-prefix=$rustupHome=/rustup_home"
) -join [char]0x1f

$env:SOURCE_DATE_EPOCH = "1786132966"
$env:CARGO_INCREMENTAL = "0"
$env:CARGO_ENCODED_RUSTFLAGS = $encodedFlags
Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR = $targetRoot

& cargo build `
    -p dockable-semantic-core `
    --release `
    --bin core-x0-canary `
    --target x86_64-pc-windows-msvc `
    --locked `
    --offline
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$binary = Join-Path $targetRoot "x86_64-pc-windows-msvc\release\core-x0-canary.exe"
$binaryInfo = Get-Item -LiteralPath $binary
$binaryHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash.ToLowerInvariant()

[ordered]@{
    binary = $binaryInfo.FullName
    bytes = $binaryInfo.Length
    sha256 = $binaryHash
    toolchain = "rustc 1.96.0 (ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96)"
    target = "x86_64-pc-windows-msvc"
    source_date_epoch = 1786132966
    linker_reproducible = $true
    path_remapping = $true
} | ConvertTo-Json
