[CmdletBinding()]
param([switch]$Apply)
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath 'I:\B_Core').Path
if ((Get-Location).Path -ne $taskRoot) { throw 'Run from I:\B_Core.' }
$receiptRoot = Join-Path $taskRoot 'reports\workspace-cleanup-2026-09-09'
if (Test-Path -LiteralPath $receiptRoot) { throw 'Receipt already exists; do not repeat this one-time cleanup.' }
$keepTest = Join-Path $taskRoot 'target\debug\deps\semantic_core_adapters-ad056d4b16f8ec31.exe'
if (!(Test-Path -LiteralPath $keepTest)) { throw 'Expected preserved test executable missing.' }
$candidatePaths = @(
    'I:\B_Core\target\debug\incremental',
    'I:\B_Core\target\debug\build',
    'I:\B_Core\target\debug\.fingerprint',
    'I:\B_Core\target\debug\examples',
    'I:\B_Core\target\event-validation\aa1ec033-a83a-488e-a7c3-e75ee4bf608d\target'
)
$candidatePaths += @(Get-ChildItem -LiteralPath 'I:\B_Core\target\debug\deps' -Force | Where-Object { $_.FullName -ne $keepTest } | Select-Object -ExpandProperty FullName)
$buildPrefix = $taskRoot + '\target\'
$targets = @($candidatePaths | ForEach-Object {
    $item = Get-Item -LiteralPath $_ -Force
    $resolved = (Resolve-Path -LiteralPath $item.FullName).Path
    if (!$resolved.StartsWith($buildPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "Outside build tree: $resolved" }
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Reparse target: $resolved" }
    if ($item.PSIsContainer -and @(Get-ChildItem -LiteralPath $resolved -Recurse -Force -Attributes ReparsePoint).Count) { throw "Reparse descendant: $resolved" }
    $files = if ($item.PSIsContainer) { @(Get-ChildItem -LiteralPath $resolved -Recurse -File -Force) } else { @($item) }
    [pscustomobject]@{ path=$resolved; directory=$item.PSIsContainer; bytes=[long](($files | Measure-Object Length -Sum).Sum); files=$files.Count }
})
$trackedBuildFiles = @(git ls-files -- target)
if ($LASTEXITCODE -ne 0 -or $trackedBuildFiles.Count -ne 0) { throw 'Tracked build files or Git failure; stop.' }
function Assert-NoBuildProcess {
    $busy = @(Get-CimInstance Win32_Process | Where-Object {
        $_.Name -match '^(cargo|rustc|rustfmt|clippy-driver)\.exe$' -or
        ($_.ExecutablePath -and $_.ExecutablePath.StartsWith($buildPrefix, [StringComparison]::OrdinalIgnoreCase))
    })
    if ($busy.Count) { throw ('Build/workspace process active: ' + ($busy.ProcessId -join ',')) }
}
Assert-NoBuildProcess
$totalBytes = [long](($targets | Measure-Object bytes -Sum).Sum)
if (!$Apply) {
    [pscustomobject]@{mode='PREVIEW'; targets=$targets.Count; files=($targets | Measure-Object files -Sum).Sum; bytes=$totalBytes; GiB=[math]::Round($totalBytes/1GB,3); kept='All source, knowledge, reports, scratch recovery, Git, native executable and current test executable'} | ConvertTo-Json
    exit 0
}
# Capture every non-Git, non-deletion file, including ignored knowledge and binaries.
$targetSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($t in $targets) { $null=$targetSet.Add($t.path) }
$preserved = [Collections.Generic.List[object]]::new()
function Visit-Protected([string]$directory) {
    foreach ($item in Get-ChildItem -LiteralPath $directory -Force) {
        if ($item.FullName -eq (Join-Path $taskRoot '.git') -or $targetSet.Contains($item.FullName)) { continue }
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Unaccounted reparse point: $($item.FullName)" }
        if ($item.PSIsContainer) { Visit-Protected $item.FullName }
        else { $preserved.Add([pscustomobject]@{path=$item.FullName;bytes=$item.Length;sha256=(Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash}) }
    }
}
Visit-Protected $taskRoot
$headBefore = git rev-parse HEAD
$statusBefore = @(git status --porcelain=v1 --untracked-files=all)
$freeBefore = (Get-PSDrive I).Free
$null = New-Item -Path $receiptRoot -ItemType Directory
@{schema='B_CORE_BUILD_CLEANUP_BEFORE_1';head=$headBefore;git_status=$statusBefore;preserved=$preserved;deletion_targets=$targets;free_bytes=$freeBefore} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $receiptRoot 'before.json') -Encoding utf8
Assert-NoBuildProcess
$removed=[Collections.Generic.List[object]]::new()
try {
    foreach ($t in $targets) {
        # Resolve and validate the exact absolute target again immediately before removal.
        $resolved=(Resolve-Path -LiteralPath $t.path).Path
        if ($resolved -ne $t.path -or !$resolved.StartsWith($buildPrefix,[StringComparison]::OrdinalIgnoreCase)) { throw "Target changed: $resolved" }
        if ($t.directory) { Remove-Item -LiteralPath $resolved -Recurse -Force }
        else { Remove-Item -LiteralPath $resolved -Force }
        $removed.Add($t)
    }
    $changed=@(foreach($p in $preserved) {
        if (!(Test-Path -LiteralPath $p.path) -or (Get-FileHash -LiteralPath $p.path -Algorithm SHA256).Hash -ne $p.sha256) { $p.path }
    })
    $statusAfter=@(git status --porcelain=v1 --untracked-files=all | Where-Object { $_ -notmatch ' reports/workspace-cleanup-2026-09-09/' })
    $statusDiff=@(Compare-Object $statusBefore $statusAfter)
    $headAfter=git rev-parse HEAD
    if ($changed.Count -or $statusDiff.Count -or $headBefore -ne $headAfter) { throw 'Protected content or Git state changed; inspect receipt.' }
    & (Join-Path $taskRoot 'scripts\verify_canonical_manifest.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'Canonical verification failed.' }
    $result=@{schema='B_CORE_BUILD_CLEANUP_RESULT_1';status='PASS';removed_bytes=[long](($removed | Measure-Object bytes -Sum).Sum);removed_files=($removed | Measure-Object files -Sum).Sum;preserved_files=$preserved.Count;protected_hashes_unchanged=$true;git_state_unchanged=$true;canonical='PASS';head=$headAfter;free_before=$freeBefore;free_after=(Get-PSDrive I).Free;recovery='Deleted compiler artifacts are not in Recycle Bin; rebuild with Cargo. Protected source, knowledge, evidence and current executables remain unchanged.'}
    $result | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $receiptRoot 'result.json') -Encoding utf8
    $result | ConvertTo-Json -Depth 4
} catch {
    @{status='FAIL';error=$_.Exception.Message;removed=$removed} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $receiptRoot 'failure.json') -Encoding utf8
    throw
}
