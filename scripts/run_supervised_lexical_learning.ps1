#requires -Version 7.0
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'),
    [string]$Artifact = (Join-Path $PSScriptRoot '../crates/semantic-core-adapters/data/world-vocabulary/g3_supervised_lexical_replay.json'),
    [switch]$Apply
)

# This is an explicit host/evaluator runner, not a runtime vocabulary loader.
# Registration is deliberately refused unless the caller supplies -Apply.
$ErrorActionPreference = 'Stop'
if (-not $Apply) { throw 'EXPLICIT_APPLY_REQUIRED' }

$artifactPath = (Resolve-Path -LiteralPath $Artifact).Path
$exePath = (Resolve-Path -LiteralPath $Executable).Path
$artifactData = Get-Content -LiteralPath $artifactPath -Raw | ConvertFrom-Json -Depth 30
$sourcePath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../reports/language-cortex-completion/meaning-learning-2026-09-09/supervised_lexical_batch.json')).Path
$source = Get-Content -LiteralPath $sourcePath -Raw | ConvertFrom-Json -Depth 30
$sourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash.ToUpperInvariant()
if ([string]$artifactData.schema -ne 'B_CORE_SUPERVISED_WORLD_LEXICAL_REPLAY_1' -or
    [string]$artifactData.authorship -ne 'SUPPLIED_BY_SUPERVISOR_NOT_AUTONOMOUS' -or
    [string]$artifactData.source_batch_sha256 -ne $sourceHash -or
    [string]$artifactData.source_batch_sha256 -ne '36572D97F2483D9E8D22B37F7F37E48FB0181D55854607E8099B7E7595197BBF') {
    throw ("ARTIFACT_SOURCE_HASH_MISMATCH schema={0} authorship={1} artifact_hash={2} source_hash={3}" -f $artifactData.schema, $artifactData.authorship, $artifactData.source_batch_sha256, $sourceHash)
}
$artifactJson = $artifactData.update | ConvertTo-Json -Depth 30 -Compress
$sourceJson = $source.update | ConvertTo-Json -Depth 30 -Compress
if ($artifactJson -ne $sourceJson) { throw 'ARTIFACT_UPDATE_MISMATCH' }
$predicates = @($artifactData.update.predicates)
$aliases = @($artifactData.update.aliases)
if ($predicates.Count -ne 8 -or $aliases.Count -ne 16) { throw 'FIXED_BATCH_CARDINALITY_MISMATCH' }
if ($source.construction_records_added -ne 0) { throw 'CONSTRUCTION_COUNT_MISMATCH' }

$rows = [Collections.Generic.List[object]]::new()
$apiErrors = [Collections.Generic.List[string]]::new()
$checkFailures = [Collections.Generic.List[string]]::new()
$processes = [Collections.Generic.List[Diagnostics.Process]]::new()
$script:registrationCount = 0
$script:registrationElapsedMs = [Collections.Generic.List[double]]::new()
$script:peakWorkingSetBytes = 0L

function Hash-Text([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text))) -replace '-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function New-Process {
    $start = [Diagnostics.ProcessStartInfo]::new($exePath)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
    $p = [Diagnostics.Process]::Start($start)
    $processes.Add($p)
    [pscustomobject]@{ process = $p; stderr = $p.StandardError.ReadToEndAsync() }
}

function Invoke-Native($session, $command, [string]$label) {
    $p = $session.process
    try {
        $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 30 -Compress))
        $p.StandardInput.Flush()
        $pending = $p.StandardOutput.ReadLineAsync()
        if (-not $pending.Wait(30000)) { throw 'CLI_TIMEOUT' }
        if ($null -eq $pending.Result) { throw 'CLI_EOF' }
        $response = $pending.Result | ConvertFrom-Json -Depth 100
        if (-not $response.ok) { $apiErrors.Add("${label}:$($response.error | ConvertTo-Json -Compress)") }
        return $response
    } catch {
        $apiErrors.Add("${label}:$($_.Exception.Message)")
        return [pscustomobject]@{ ok = $false; payload = $null; error = $_.Exception.Message }
    }
}

function Get-Value($response) {
    if ($response.ok -and $null -ne $response.payload) { return $response.payload.value }
    return $null
}

function Get-ConversationState($value) {
    if ($null -eq $value) { return $null }
    if ($null -ne $value.conversation_state) { return $value.conversation_state }
    if ($null -ne $value.dialogue_world -and $null -ne $value.conversation_id) { return $value }
    return $null
}

function Get-World($value) {
    if ($null -ne $value -and $null -ne $value.discourse_answer) { return $value.discourse_answer.world_reasoning }
    return $null
}

function Get-Verdict($value) {
    $world = Get-World $value
    if ($null -ne $world) { return [string]$world.decision.verdict }
    return 'NO_WORLD_DECISION'
}

function Get-PredicateIds($value) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return @() }
    $predicates = $state.dialogue_world.vocabulary.predicates
    if ($null -eq $predicates) { return @() }
    return @($predicates.psobject.Properties | ForEach-Object { $_.Name } | Sort-Object)
}

function Get-AliasIds($value) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return @() }
    $history = @($state.dialogue_world.vocabulary.lexical_history)
    if ($history.Count -eq 0) { return @() }
    return @($history[$history.Count - 1].psobject.Properties | ForEach-Object { $_.Name } | Sort-Object)
}

function Get-HistoryCount($value) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return 0 }
    return @($state.dialogue_world.vocabulary.lexical_history).Count
}

function Get-PredicateProjection($value) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return '' }
    $predicates = $state.dialogue_world.vocabulary.predicates
    $ordered = [Collections.Generic.List[object]]::new()
    foreach ($property in @($predicates.psobject.Properties | Sort-Object Name)) {
        $ordered.Add([ordered]@{ predicate_id = $property.Name; arity = [string]$property.Value.arity })
    }
    return ($ordered | ConvertTo-Json -Depth 10 -Compress)
}

function Get-Target($value) {
    $world = Get-World $value
    if ($null -eq $world -or @($world.query.target).Count -eq 0) { return $null }
    return $world.query.target[0]
}

function Get-TargetPredicateId($value) {
    $target = Get-Target $value
    if ($null -eq $target -or $null -eq $target.property) { return $null }
    $property = $target.property.psobject.Properties | Select-Object -First 1
    if ($null -eq $property) { return $null }
    return [string]$property.Value
}

function Get-ActiveRegisteredIds($value) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return @() }
    $records = @($state.dialogue_world.premises)
    $ids = [Collections.Generic.List[string]]::new()
    foreach ($record in $records) {
        if (-not $record.active) { continue }
        $property = $record.atom.property.psobject.Properties | Select-Object -First 1
        if ($null -ne $property -and $property.Name -eq 'REGISTERED') { $ids.Add([string]$property.Value) }
    }
    return @($ids | Sort-Object -Unique)
}

function Has-ActiveRegisteredEntity($value, [string]$predicateId, [string]$entity) {
    $state = Get-ConversationState $value
    if ($null -eq $state) { return $false }
    foreach ($record in @($state.dialogue_world.premises)) {
        if (-not $record.active -or $record.atom.entity -ne $entity) { continue }
        $property = $record.atom.property.psobject.Properties | Select-Object -First 1
        if ($null -ne $property -and $property.Name -eq 'REGISTERED' -and $property.Value -eq $predicateId) { return $true }
    }
    return $false
}

function Register($session, [string]$conversationId) {
    $command = @{ operation = 'UPDATE_WORLD_VOCABULARY'; conversation_id = $conversationId; update = $artifactData.update }
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $response = Invoke-Native $session $command "register:$conversationId"
    $watch.Stop()
    $script:registrationCount++
    $script:registrationElapsedMs.Add($watch.Elapsed.TotalMilliseconds)
    $value = Get-Value $response
    $registered = @(Get-PredicateIds $value | Where-Object { $_ -like 'W_USER_710*' })
    Check ($registered.Count -eq 8) "register:${conversationId}:PREDICATE_COUNT"
    Check ((Get-PredicateProjection $value).Length -gt 0) "register:${conversationId}:SEMANTIC_PROJECTION_PRESENT"
    return $response
}

function Turn($session, [string]$conversationId, [int]$turn, [string]$text, [string]$language, [string]$label) {
    $command = @{ operation = 'PROCESS_CONVERSATION_TURN'; request = @{
        schema = 'B_CORE_CONVERSATION_TURN_REQUEST_1'; conversation_id = $conversationId; turn_index = $turn
        request_id = "$conversationId-$turn"; modality = 'TEXT'; raw_text = $text
        input_confidence_millis = 1000; alternatives = @(); output_language = $language; context_tags = @(); max_plan_steps = 16
    }}
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $response = Invoke-Native $session $command "turn:$label"
    $watch.Stop()
    $value = Get-Value $response
    if ($null -ne $session.process -and -not $session.process.HasExited) {
        $session.process.Refresh()
        $script:peakWorkingSetBytes = [Math]::Max($script:peakWorkingSetBytes, $session.process.PeakWorkingSet64)
    }
    $world = Get-World $value
    $target = Get-Target $value
    $rows.Add([ordered]@{
        label = $label; conversation_id = $conversationId; turn = $turn; language = $language
        input = $text; elapsed_ms = $watch.Elapsed.TotalMilliseconds; api_ok = [bool]$response.ok; error = $response.error
        verdict = Get-Verdict $value; target_entity = if ($null -ne $target) { $target.entity } else { $null }
        target_predicate_id = Get-TargetPredicateId $value; proof_steps = if ($null -ne $world) { @($world.decision.proof_mechanism_ids).Count } else { 0 }
        active_registered_ids = @(Get-ActiveRegisteredIds $value); output = if ($null -ne $value) { $value.output.text } else { $null }
        predicate_inventory_hash = if ($null -ne $value) { Hash-Text (Get-PredicateProjection $value) } else { $null }
        predicate_ids = @(Get-PredicateIds $value)
    })
    return $value
}

function Check([bool]$Condition, [string]$Label) {
    if (-not $Condition) { $checkFailures.Add($Label) }
}

function Close-Session($session) {
    $p = $session.process
    if (-not $p.HasExited) {
        $p.StandardInput.Close()
        if (-not $p.WaitForExit(30000)) { $p.Kill(); $p.WaitForExit(30000) | Out-Null }
    }
    $stderr = $session.stderr.GetAwaiter().GetResult()
    if (-not [string]::IsNullOrWhiteSpace($stderr)) { $apiErrors.Add("stderr:$stderr") }
    $p.Dispose()
}

function Assert-Supported($value, [string]$label, [string]$predicateId, [string]$entity, [int]$minimumProof = 0) {
    Check ((Get-Verdict $value) -eq 'SUPPORTED') "${label}:SUPPORTED"
    Check ((Get-TargetPredicateId $value) -eq $predicateId) "${label}:PREDICATE_ID"
    $target = Get-Target $value
    Check ($null -ne $target -and $target.entity -eq $entity) "${label}:BOUND_ENTITY"
    $world = Get-World $value
    Check ($null -ne $world -and @($world.decision.proof_mechanism_ids).Count -ge $minimumProof) "${label}:PROOF"
}

$main = New-Process
try {
    # Before registration: the supplied surface must not acquire capability by itself.
    $before = Turn $main 'G3-BEFORE' 1 'Ari is busy.' 'ENGLISH' 'before:unregistered-label'
    Check ((Get-ActiveRegisteredIds $before) -notcontains 'W_USER_710001') 'before:NO_REGISTERED_EVENT'
    Check ((Get-Verdict $before) -ne 'SUPPORTED') 'before:NOT_SUPPORTED'

    foreach ($id in @('G3-EN-MAIN', 'G3-KO-MAIN', 'G3-EN-ROLE', 'G3-KO-ROLE', 'G3-EN-NEG', 'G3-KO-NEG', 'G3-EN-EXTRA', 'G3-KO-EXTRA')) { [void](Register $main $id) }
    $registered = Turn $main 'G3-EN-MAIN' 1 'Ari is busy.' 'ENGLISH' 'after:en-unary-premise'
    $answer = Turn $main 'G3-EN-MAIN' 2 'Is Ari busy?' 'ENGLISH' 'after:en-unary-query'
    Assert-Supported $answer 'after:en-unary' 'W_USER_710001' 'Ari'
    $null = Turn $main 'G3-EN-MAIN' 3 'Dara is calm.' 'ENGLISH' 'after:en-composition-premise'
    $null = Turn $main 'G3-EN-MAIN' 4 'If Dara is calm, then Evan supports Faye.' 'ENGLISH' 'after:en-composition-rule'
    $composition = Turn $main 'G3-EN-MAIN' 5 'Does Evan support Faye?' 'ENGLISH' 'after:en-composition-query'
    Assert-Supported $composition 'after:en-composition' 'W_USER_710006' 'Evan' 1

    $koPremise = Turn $main 'G3-KO-MAIN' 1 'bora는 분주해.' 'KOREAN' 'after:ko-unary-premise'
    $koAnswer = Turn $main 'G3-KO-MAIN' 2 'bora는 분주하나요?' 'KOREAN' 'after:ko-unary-query'
    Assert-Supported $koAnswer 'after:ko-unary' 'W_USER_710001' 'bora'
    $null = Turn $main 'G3-KO-MAIN' 3 'dara는 차분해.' 'KOREAN' 'after:ko-composition-premise'
    $null = Turn $main 'G3-KO-MAIN' 4 'dara가 차분하면 evan이 faye를 지원한다.' 'KOREAN' 'after:ko-composition-rule'
    $koComposition = Turn $main 'G3-KO-MAIN' 5 'evan이 faye를 지원하나요?' 'KOREAN' 'after:ko-composition-query'
    Assert-Supported $koComposition 'after:ko-composition' 'W_USER_710006' 'evan' 1

    $null = Turn $main 'G3-EN-EXTRA' 1 'Gina is anxious.' 'ENGLISH' 'coverage:en-anxious-premise'
    $enAnxious = Turn $main 'G3-EN-EXTRA' 2 'Is Gina anxious?' 'ENGLISH' 'coverage:en-anxious-query'
    Assert-Supported $enAnxious 'coverage:en-anxious' 'W_USER_710003' 'gina'
    $null = Turn $main 'G3-EN-EXTRA' 3 'Hana depends on Ivo.' 'ENGLISH' 'coverage:en-depend-premise'
    $enDepend = Turn $main 'G3-EN-EXTRA' 4 'Does Hana depend on Ivo?' 'ENGLISH' 'coverage:en-depend-query'
    Assert-Supported $enDepend 'coverage:en-depend' 'W_USER_710007' 'hana'
    $null = Turn $main 'G3-EN-EXTRA' 5 'Jin contributes to Kira.' 'ENGLISH' 'coverage:en-contribute-premise'
    $enContribute = Turn $main 'G3-EN-EXTRA' 6 'Does Jin contribute to Kira?' 'ENGLISH' 'coverage:en-contribute-query'
    Assert-Supported $enContribute 'coverage:en-contribute' 'W_USER_710008' 'jin'
    $null = Turn $main 'G3-KO-EXTRA' 1 'gina는 불안해.' 'KOREAN' 'coverage:ko-anxious-premise'
    $koAnxious = Turn $main 'G3-KO-EXTRA' 2 'gina는 불안하나요?' 'KOREAN' 'coverage:ko-anxious-query'
    Assert-Supported $koAnxious 'coverage:ko-anxious' 'W_USER_710003' 'gina'
    $null = Turn $main 'G3-KO-EXTRA' 3 'hana는 ivo에 의존해.' 'KOREAN' 'coverage:ko-depend-premise'
    $koDepend = Turn $main 'G3-KO-EXTRA' 4 'hana는 ivo에 의존하나요?' 'KOREAN' 'coverage:ko-depend-query'
    Assert-Supported $koDepend 'coverage:ko-depend' 'W_USER_710007' 'hana'
    $null = Turn $main 'G3-KO-EXTRA' 5 'jin은 kira에 기여해.' 'KOREAN' 'coverage:ko-contribute-premise'
    $koContribute = Turn $main 'G3-KO-EXTRA' 6 'jin은 kira에 기여하나요?' 'KOREAN' 'coverage:ko-contribute-query'
    Assert-Supported $koContribute 'coverage:ko-contribute' 'W_USER_710008' 'jin'

    $enRelation = Turn $main 'G3-EN-ROLE' 1 'Ari trusts Bea.' 'ENGLISH' 'after:en-relation-premise'
    $enRelationAnswer = Turn $main 'G3-EN-ROLE' 2 'Does Ari trust Bea?' 'ENGLISH' 'after:en-relation-query'
    Assert-Supported $enRelationAnswer 'after:en-relation' 'W_USER_710005' 'Ari'
    $enReverse = Turn $main 'G3-EN-ROLE' 3 'Does Bea trust Ari?' 'ENGLISH' 'control:en-role-reversal'
    Check ((Get-Verdict $enReverse) -eq 'UNKNOWN') 'control:en-role-reversal:UNKNOWN'
    $koRelation = Turn $main 'G3-KO-ROLE' 1 '아리는 보라를 신뢰해.' 'KOREAN' 'after:ko-relation-premise'
    $koRelationAnswer = Turn $main 'G3-KO-ROLE' 2 '아리는 보라를 신뢰하나요?' 'KOREAN' 'after:ko-relation-query'
    Assert-Supported $koRelationAnswer 'after:ko-relation' 'W_USER_710005' '아리'
    $koReverse = Turn $main 'G3-KO-ROLE' 3 '보라는 아리를 신뢰하나요?' 'KOREAN' 'control:ko-role-reversal'
    Check ((Get-Verdict $koReverse) -eq 'UNKNOWN') 'control:ko-role-reversal:UNKNOWN'

    $enNegative = Turn $main 'G3-EN-NEG' 1 'Ari does not trust Bea.' 'ENGLISH' 'control:en-negative-premise'
    $enNegativeAnswer = Turn $main 'G3-EN-NEG' 2 'Does Ari trust Bea?' 'ENGLISH' 'control:en-negative-query'
    Check ((Get-Verdict $enNegativeAnswer) -eq 'REFUTED') 'control:en-negative:REFUTED'
    $koNegative = Turn $main 'G3-KO-NEG' 1 '아리는 보라를 신뢰하지 않아.' 'KOREAN' 'control:ko-negative-premise'
    $koNegativeAnswer = Turn $main 'G3-KO-NEG' 2 '아리는 보라를 신뢰하나요?' 'KOREAN' 'control:ko-negative-query'
    Check ((Get-Verdict $koNegativeAnswer) -eq 'REFUTED') 'control:ko-negative:REFUTED'

    $beforeRename = Get-Value (Register $main 'G3-EN-RENAME')
    $beforeProjection = Get-PredicateProjection $beforeRename
    $beforeRevision = Get-HistoryCount $beforeRename
    $renameUpdate = @{ predicates = @(); aliases = @(@{ alias_id = 'G3.0.en'; predicate_id = 'W_USER_710001'; language = 'ENGLISH'; root = 'engaged'; grammar = 'COPULAR' }); remove_alias_ids = @('G3.0.en') }
    $renameResponse = Invoke-Native $main @{ operation = 'UPDATE_WORLD_VOCABULARY'; conversation_id = 'G3-EN-RENAME'; update = $renameUpdate } 'rename:G3-EN-RENAME'
    $renamedState = Get-Value $renameResponse
    $afterRenameProjection = Get-PredicateProjection $renamedState
    Check ((Hash-Text $beforeProjection) -eq (Hash-Text $afterRenameProjection)) 'alias-rename:SEMANTIC_HASH_INVARIANT'
    Check ((Get-HistoryCount $renamedState) -gt $beforeRevision) 'alias-rename:REVISION_ADVANCED'
    $null = Turn $main 'G3-EN-RENAME' 1 'Cato is engaged.' 'ENGLISH' 'alias-rename:new-surface'
    $renamedAnswer = Turn $main 'G3-EN-RENAME' 2 'Is Cato engaged?' 'ENGLISH' 'alias-rename:new-query'
    Assert-Supported $renamedAnswer 'alias-rename:new-query' 'W_USER_710001' 'Cato'
    $removedResponse = Invoke-Native $main @{ operation = 'UPDATE_WORLD_VOCABULARY'; conversation_id = 'G3-EN-RENAME'; update = @{ predicates = @(); aliases = @(); remove_alias_ids = @('G3.0.en') } } 'alias-remove:G3-EN-RENAME'
    $removedState = Get-Value $removedResponse
    Check ((Hash-Text $beforeProjection) -eq (Hash-Text (Get-PredicateProjection $removedState))) 'alias-remove:SEMANTIC_HASH_INVARIANT'
    Check ((Get-HistoryCount $removedState) -gt (Get-HistoryCount $renamedState)) 'alias-remove:REVISION_ADVANCED'
    $ablated = Turn $main 'G3-EN-RENAME' 3 'Dela is engaged.' 'ENGLISH' 'alias-remove:surface-ablation'
    Check (-not (Has-ActiveRegisteredEntity $ablated 'W_USER_710001' 'Dela')) 'alias-remove:NO_NEW_REGISTERED_EVENT'
    Check ((Get-Verdict $ablated) -ne 'SUPPORTED') 'alias-remove:NOT_SUPPORTED'
    Check ((Get-ActiveRegisteredIds $renamedAnswer) -contains 'W_USER_710001') 'unnamed:MEMORY_RETAINS_REGISTERED_PREDICATE'
    Check ((Get-PredicateIds $removedState) -contains 'W_USER_710001') 'unnamed:PREDICATE_CONTRACT_REMAINS'
} finally {
    Close-Session $main
}

# Semantic ablation: the label alone cannot create the supplied predicate.
$ablation = New-Process
try {
    $value = Turn $ablation 'G3-ABLATION' 1 'Ari is busy.' 'ENGLISH' 'semantic-ablation:without-supplied-contract'
    Check ((Get-ActiveRegisteredIds $value) -notcontains 'W_USER_710001') 'semantic-ablation:NO_REGISTERED_EVENT'
    Check ((Get-Verdict $value) -ne 'SUPPORTED') 'semantic-ablation:NOT_SUPPORTED'
} finally { Close-Session $ablation }

# Cold-process startup replay: durable data is useful only when a host explicitly replays it.
$replay = New-Process
try {
    $replayRegistration = Register $replay 'G3-COLD-REPLAY'
    $replayPremise = Turn $replay 'G3-COLD-REPLAY' 1 'Neri is happy.' 'ENGLISH' 'cold-replay:premise'
    $replayAnswer = Turn $replay 'G3-COLD-REPLAY' 2 'Is Neri happy?' 'ENGLISH' 'cold-replay:query'
    Assert-Supported $replayAnswer 'cold-replay:query' 'W_USER_710004' 'Neri'
    Check (@(Get-PredicateIds $replayAnswer | Where-Object { $_ -like 'W_USER_710*' }).Count -eq 8) 'cold-replay:REGISTERED_PREDICATE_COUNT'
    Check (@(Get-AliasIds $replayAnswer | Where-Object { $_ -like 'G3.*' }).Count -eq 16) 'cold-replay:REGISTERED_ALIAS_COUNT'
} finally { Close-Session $replay }

$failed = @($checkFailures | Sort-Object -Unique)
$report = [ordered]@{
    schema = 'B_CORE_SUPERVISED_LEXICAL_LEARNING_RUN_1'
    status = if ($failed.Count -eq 0 -and $apiErrors.Count -eq 0 -and $rows.Count -gt 0) { 'PASS' } else { 'FAIL' }
    authorship = $artifactData.authorship
    scope = $artifactData.scope
    apply_explicit = $true
    source_batch_sha256 = $sourceHash.ToLowerInvariant()
    artifact_sha256 = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_sha256 = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash.ToLowerInvariant()
    registered_predicates = $predicates.Count
    registered_aliases = $aliases.Count
    construction_records_added = 0
    runtime_processes = $processes.Count
    runtime_update_commands = $script:registrationCount
    registration_elapsed_ms = @($script:registrationElapsedMs)
    peak_working_set_bytes = $script:peakWorkingSetBytes
    artifact_bytes = (Get-Item -LiteralPath $artifactPath).Length
    api_errors = @($apiErrors)
    failed_controls = $failed
    rows = @($rows)
    limitations = @($artifactData.limitations)
    replay_policy = $artifactData.startup_policy
}
$report | ConvertTo-Json -Depth 30 -Compress
if ($report.status -ne 'PASS') { exit 1 }
