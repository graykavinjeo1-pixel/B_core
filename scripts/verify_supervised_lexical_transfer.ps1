#requires -Version 7.0
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'),
    [string]$Artifact = (Join-Path $PSScriptRoot '../crates/semantic-core-adapters/data/world-vocabulary/g3_supervised_lexical_replay.json')
)

# Evaluator-only matched transfer check. This script never runs without an
# explicit host UPDATE_WORLD_VOCABULARY command and does not alter defaults.
$ErrorActionPreference = 'Stop'
$exePath = (Resolve-Path -LiteralPath $Executable).Path
$artifactPath = (Resolve-Path -LiteralPath $Artifact).Path
$artifactData = Get-Content -LiteralPath $artifactPath -Raw | ConvertFrom-Json -Depth 30
$sourcePath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../reports/language-cortex-completion/meaning-learning-2026-09-09/supervised_lexical_batch.json')).Path
$sourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($artifactData.source_batch_sha256.ToLowerInvariant() -ne $sourceHash) { throw 'ARTIFACT_SOURCE_HASH_MISMATCH' }
$source = Get-Content -LiteralPath $sourcePath -Raw | ConvertFrom-Json -Depth 30
if (($artifactData.update | ConvertTo-Json -Depth 30 -Compress) -ne ($source.update | ConvertTo-Json -Depth 30 -Compress)) { throw 'ARTIFACT_UPDATE_MISMATCH' }

$cases = @(
    @{ id = 'W710001'; language = 'ENGLISH'; predicate = 'W_USER_710001'; entity = 'aria'; object = $null; premise = 'aria is busy.'; query = 'Is aria busy?' },
    @{ id = 'W710002'; language = 'ENGLISH'; predicate = 'W_USER_710002'; entity = 'brio'; object = $null; premise = 'brio is calm.'; query = 'Is brio calm?' },
    @{ id = 'W710003'; language = 'ENGLISH'; predicate = 'W_USER_710003'; entity = 'cira'; object = $null; premise = 'cira is anxious.'; query = 'Is cira anxious?' },
    @{ id = 'W710004'; language = 'ENGLISH'; predicate = 'W_USER_710004'; entity = 'daro'; object = $null; premise = 'daro is happy.'; query = 'Is daro happy?' },
    @{ id = 'W710005'; language = 'ENGLISH'; predicate = 'W_USER_710005'; entity = 'erin'; object = 'finn'; premise = 'erin trusts finn.'; query = 'Does erin trust finn?' },
    @{ id = 'W710006'; language = 'ENGLISH'; predicate = 'W_USER_710006'; entity = 'gale'; object = 'hera'; premise = 'gale supports hera.'; query = 'Does gale support hera?' },
    @{ id = 'W710007'; language = 'ENGLISH'; predicate = 'W_USER_710007'; entity = 'ivan'; object = 'jora'; premise = 'ivan depends on jora.'; query = 'Does ivan depend on jora?' },
    @{ id = 'W710008'; language = 'ENGLISH'; predicate = 'W_USER_710008'; entity = 'kira'; object = 'luma'; premise = 'kira contributes to luma.'; query = 'Does kira contribute to luma?' },
    @{ id = 'W710001'; language = 'KOREAN'; predicate = 'W_USER_710001'; entity = '아리'; object = $null; premise = '아리는 분주해.'; query = '아리는 분주하나요?' },
    @{ id = 'W710002'; language = 'KOREAN'; predicate = 'W_USER_710002'; entity = '브리'; object = $null; premise = '브리는 차분해.'; query = '브리는 차분하나요?' },
    @{ id = 'W710003'; language = 'KOREAN'; predicate = 'W_USER_710003'; entity = '시라'; object = $null; premise = '시라는 불안해.'; query = '시라는 불안하나요?' },
    @{ id = 'W710004'; language = 'KOREAN'; predicate = 'W_USER_710004'; entity = '다로'; object = $null; premise = '다로는 행복해.'; query = '다로는 행복하나요?' },
    @{ id = 'W710005'; language = 'KOREAN'; predicate = 'W_USER_710005'; entity = '에린'; object = '핀'; premise = '에린은 핀을 신뢰해.'; query = '에린은 핀을 신뢰하나요?' },
    @{ id = 'W710006'; language = 'KOREAN'; predicate = 'W_USER_710006'; entity = '갈'; object = '헤라'; premise = '갈은 헤라를 지원해.'; query = '갈은 헤라를 지원하나요?' },
    @{ id = 'W710007'; language = 'KOREAN'; predicate = 'W_USER_710007'; entity = '이반'; object = '조라'; premise = '이반은 조라에 의존해.'; query = '이반은 조라에 의존하나요?' },
    @{ id = 'W710008'; language = 'KOREAN'; predicate = 'W_USER_710008'; entity = '키라'; object = '루마'; premise = '키라는 루마에 기여해.'; query = '키라는 루마에 기여하나요?' }
)
if ($cases.Count -ne 16) { throw 'MATCHED_CASE_COUNT' }

$rows = [Collections.Generic.List[object]]::new()
$errors = [Collections.Generic.List[string]]::new()
$failures = [Collections.Generic.List[string]]::new()
$processes = [Collections.Generic.List[Diagnostics.Process]]::new()
$peakWorkingSet = 0L

function New-Session {
    $start = [Diagnostics.ProcessStartInfo]::new($exePath)
    $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    $start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
    $p = [Diagnostics.Process]::Start($start); $processes.Add($p)
    [pscustomobject]@{ process = $p; stderr = $p.StandardError.ReadToEndAsync() }
}

function Send($session, $command, [string]$label) {
    try {
        $session.process.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 30 -Compress)); $session.process.StandardInput.Flush()
        $pending = $session.process.StandardOutput.ReadLineAsync()
        if (-not $pending.Wait(30000)) { throw 'CLI_TIMEOUT' }
        if ($null -eq $pending.Result) { throw 'CLI_EOF' }
        $response = $pending.Result | ConvertFrom-Json -Depth 100
        if (-not $session.process.HasExited) { $session.process.Refresh(); $script:peakWorkingSet = [Math]::Max($script:peakWorkingSet, $session.process.PeakWorkingSet64) }
        if (-not $response.ok) { $errors.Add("${label}:$($response.error | ConvertTo-Json -Compress)") }
        return $response
    } catch { $errors.Add("${label}:$($_.Exception.Message)"); return [pscustomobject]@{ ok = $false; payload = $null; error = $_.Exception.Message } }
}

function Close($session) {
    if (-not $session.process.HasExited) {
        $session.process.StandardInput.Close()
        if (-not $session.process.WaitForExit(30000)) { $session.process.Kill(); $session.process.WaitForExit(30000) | Out-Null }
    }
    $stderr = $session.stderr.GetAwaiter().GetResult()
    if (-not [string]::IsNullOrWhiteSpace($stderr)) { $errors.Add("stderr:$stderr") }
    $session.process.Dispose()
}

function Value($response) { if ($response.ok -and $null -ne $response.payload) { return $response.payload.value }; return $null }
function World($value) { if ($null -ne $value -and $null -ne $value.discourse_answer) { return $value.discourse_answer.world_reasoning }; return $null }
function Verdict($value) { $w = World $value; if ($null -ne $w) { return [string]$w.decision.verdict }; return 'NO_WORLD_DECISION' }
function Target($value) { $w = World $value; if ($null -ne $w -and @($w.query.target).Count -gt 0) { return $w.query.target[0] }; return $null }
function TargetPredicate($value) { $t = Target $value; if ($null -eq $t) { return $null }; $p = $t.property.psobject.Properties | Select-Object -First 1; if ($null -ne $p) { return [string]$p.Value }; return $null }

function Turn($session, $conversation, [int]$index, $case, [string]$text, [string]$phase) {
    $response = Send $session @{ operation = 'PROCESS_CONVERSATION_TURN'; request = @{
        schema = 'B_CORE_CONVERSATION_TURN_REQUEST_1'; conversation_id = $conversation; turn_index = $index
        request_id = "$conversation-$index"; modality = 'TEXT'; raw_text = $text; input_confidence_millis = 1000
        alternatives = @(); output_language = $case.language; context_tags = @(); max_plan_steps = 16
    }} "${phase}:$($case.id):$index"
    $value = Value $response; $target = Target $value; $world = World $value
    if ($phase -in @('before', 'after', 'known-limit') -and $index -eq 2) {
        $object = if ($null -ne $target -and $target.object) { [string]$target.object } else { $null }
        $rows.Add([ordered]@{ id = $case.id; language = $case.language; phase = $phase; premise = $case.premise; query = $case.query
            api_ok = [bool]$response.ok; error = $response.error; output = if ($null -ne $value) { $value.output.text } else { $null }
            verdict = Verdict $value; target_entity = if ($null -ne $target) { $target.entity } else { $null }
            target_object = $object; target_predicate_id = TargetPredicate $value; proof_steps = if ($null -ne $world) { @($world.decision.proof_mechanism_ids).Count } else { 0 } })
    }
    return $value
}

function Register($session, [string]$conversation) {
    return Send $session @{ operation = 'UPDATE_WORLD_VOCABULARY'; conversation_id = $conversation; update = $artifactData.update } "register:$conversation"
}

function Check([bool]$condition, [string]$label) { if (-not $condition) { $failures.Add($label) } }

# Baseline process: exact same inputs are run before any supplied contract exists.
$baseline = New-Session
try {
    foreach ($case in $cases) {
        $conversation = "G3-MATCH-BEFORE-$($case.language)-$($case.id)"
        $value = Turn $baseline $conversation 1 $case $case.premise 'before'
        $value = Turn $baseline $conversation 2 $case $case.query 'before'
        Check ((Verdict $value) -ne 'SUPPORTED') "before:$($case.language):$($case.id):NOT_SUPPORTED"
        Check ((TargetPredicate $value) -ne $case.predicate) "before:$($case.language):$($case.id):NO_TARGET_CONTRACT"
    }
} finally { Close $baseline }

# After process: register the same artifact per conversation, then replay exact inputs.
$after = New-Session
try {
    foreach ($case in $cases) {
        $conversation = "G3-MATCH-AFTER-$($case.language)-$($case.id)"
        $registration = Value (Register $after $conversation)
        Check ((@($registration.dialogue_world.vocabulary.predicates.psobject.Properties | Where-Object { $_.Name -eq $case.predicate }).Count -eq 1)) "after:$($case.language):$($case.id):REGISTERED_ID"
        $null = Turn $after $conversation 1 $case $case.premise 'after'
        $value = Turn $after $conversation 2 $case $case.query 'after'
        Check ((Verdict $value) -eq 'SUPPORTED') "after:$($case.language):$($case.id):SUPPORTED"
        Check ((TargetPredicate $value) -eq $case.predicate) "after:$($case.language):$($case.id):PREDICATE_ID"
        $target = Target $value
        Check ($null -ne $target -and $target.entity -eq $case.entity) "after:$($case.language):$($case.id):ENTITY"
        if ($null -ne $case.object) { Check ($null -ne $target -and $target.object -eq $case.object) "after:$($case.language):$($case.id):OBJECT" }
    }
    # Retain the original unsupported morphology as an explicit known limitation.
    $knownCase = @{ language = 'KOREAN'; id = 'W710001'; premise = 'bora는 분주해.'; query = 'bora는 분주한가?' }
    $knownConversation = 'G3-KO-KNOWN-MORPHOLOGY'
    $null = Register $after $knownConversation
    $null = Turn $after $knownConversation 1 $knownCase $knownCase.premise 'known-limit'
    $knownValue = Turn $after $knownConversation 2 $knownCase $knownCase.query 'known-limit'
    $knownVerdict = Verdict $knownValue
} finally { Close $after }

$matched = @($rows | Where-Object { $_.phase -eq 'after' }).Count
$knownRow = @($rows | Where-Object { $_.phase -eq 'known-limit' }) | Select-Object -Last 1
$report = [ordered]@{
    schema = 'B_CORE_SUPERVISED_LEXICAL_MATCHED_TRANSFER_1'
    status = if ($failures.Count -eq 0 -and $errors.Count -eq 0 -and $matched -eq 16) { 'PASS_WITH_KNOWN_LIMITATION' } else { 'FAIL' }
    source_batch_sha256 = $sourceHash
    artifact_sha256 = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_sha256 = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash.ToLowerInvariant()
    matched_groups = 16; before_after_exact_input_pairs = 16; before_after_processes = 2
    before_contract_available = $false; after_registered_predicates = 8; after_registered_aliases = 16
    api_errors = @($errors); failed_controls = @($failures); peak_working_set_bytes = $peakWorkingSet
    known_morphology_limit = [ordered]@{ input = $knownCase.query; verdict = $knownVerdict; output = $knownRow.output; api_ok = $knownRow.api_ok; error = $knownRow.error; expected = 'UNSUPPORTED_OR_UNRESOLVED'; retained = $true }
    rows = @($rows)
}
$report | ConvertTo-Json -Depth 30 -Compress
if ($report.status -eq 'FAIL') { exit 1 }
