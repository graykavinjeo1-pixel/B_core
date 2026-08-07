[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'docs/CANONICAL_MANIFEST.json'
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Canonical manifest not found: $manifestPath"
}

$manifestRaw = [System.IO.File]::ReadAllText($manifestPath, $utf8NoBom)
$manifest = $manifestRaw | ConvertFrom-Json
$failures = [System.Collections.Generic.List[string]]::new()

foreach ($entry in $manifest.canonical_files) {
    $relativePath = [string]$entry.relative_path
    $canonicalPath = Join-Path $repositoryRoot ($relativePath -replace '/', [System.IO.Path]::DirectorySeparatorChar)
    if (-not (Test-Path -LiteralPath $canonicalPath -PathType Leaf)) {
        $failures.Add("missing:$relativePath")
        continue
    }

    $actualLength = (Get-Item -LiteralPath $canonicalPath).Length
    $actualHash = (Get-FileHash -LiteralPath $canonicalPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualLength -ne [long]$entry.byte_length) {
        $failures.Add("length:$relativePath expected=$($entry.byte_length) actual=$actualLength")
    }
    if ($actualHash -ne ([string]$entry.sha256).ToLowerInvariant()) {
        $failures.Add("sha256:$relativePath expected=$($entry.sha256) actual=$actualHash")
    }
}

$selfHashPattern = '("manifest_self_hash_sha256"\s*:\s*")[0-9a-fA-F]{64}(")'
$selfHashMatches = [regex]::Matches($manifestRaw, $selfHashPattern)
if ($selfHashMatches.Count -ne 1) {
    $failures.Add("manifest_self_hash_field_count:expected=1 actual=$($selfHashMatches.Count)")
} else {
    $zeroHash = '0' * 64
    $normalizedManifest = [regex]::Replace(
        $manifestRaw,
        $selfHashPattern,
        "`${1}$zeroHash`${2}"
    )
    $normalizedBytes = $utf8NoBom.GetBytes($normalizedManifest)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $actualSelfHash = ([System.BitConverter]::ToString($sha.ComputeHash($normalizedBytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
    if ($actualSelfHash -ne ([string]$manifest.manifest_self_hash_sha256).ToLowerInvariant()) {
        $failures.Add("manifest_self_hash:expected=$($manifest.manifest_self_hash_sha256) actual=$actualSelfHash")
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output "CANONICAL_HASH_VERIFICATION=PASS"
Write-Output "CANONICAL_FILE_COUNT=$($manifest.canonical_files.Count)"
Write-Output "CANONICAL_MANIFEST_SHA256=$($manifest.manifest_self_hash_sha256)"
