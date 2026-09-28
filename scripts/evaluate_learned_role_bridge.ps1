#requires -Version 7.0
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'),
    [string]$Manifest = (Join-Path $PSScriptRoot '../reports/language-cortex-completion/learned-role-bridge-2026-09-09/manifest.json'),
    [string]$SetupFile = '',
    [string]$Mode = 'ORIGINAL'
)
# Supervisor evaluator: the executable receives only vocabulary, optional
# explicit training commands and raw turns. Expected meanings stay here.
$ErrorActionPreference = 'Stop'
$suite = Get-Content -LiteralPath $Manifest -Raw | ConvertFrom-Json -Depth 40
if ($suite.schema -eq 'B_CORE_LEARNED_ROLE_BRIDGE_MATRIX_1') {
    if ($suite.cases.Count -ne 42 -or $suite.counts.turns -ne 86) { throw 'FROZEN_MATRIX_COUNT' }
} elseif ($suite.schema -eq 'B_CORE_LEARNED_ROLE_CHAIN_MATRIX_1') {
    if ($suite.cases.Count -ne 4 -or $suite.counts.turns -ne 14) { throw 'FROZEN_CHAIN_COUNT' }
} else { throw 'UNKNOWN_MATRIX_SCHEMA' }
$setup = if ($SetupFile) { Get-Content -LiteralPath $SetupFile -Raw | ConvertFrom-Json -Depth 50 } else { @() }
$exe = (Resolve-Path -LiteralPath $Executable).Path
$rows = [Collections.Generic.List[object]]::new()
$errors = [Collections.Generic.List[string]]::new()
$times = [Collections.Generic.List[double]]::new()
$peak = 0L
$start = [Diagnostics.ProcessStartInfo]::new($exe)
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
$p = [Diagnostics.Process]::Start($start)
$stderr = $p.StandardError.ReadToEndAsync()
function Send($command) {
    $script:lastCommand = $command
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 60 -Compress))
    $p.StandardInput.Flush()
    $pending = $p.StandardOutput.ReadLineAsync()
    if (-not $pending.Wait(30000)) { throw 'NATIVE_TIMEOUT' }
    if ($null -eq $pending.Result) { throw 'NATIVE_EOF' }
    $watch.Stop(); $times.Add($watch.Elapsed.TotalMilliseconds)
    $response = $pending.Result | ConvertFrom-Json -Depth 100
    if (-not $response.ok) { $errors.Add(($response.error | ConvertTo-Json -Compress)) }
    $p.Refresh(); $script:peak = [Math]::Max($script:peak,$p.PeakWorkingSet64)
    return $response
}
try {
    foreach ($case in $suite.cases) {
        $id = "$Mode-$($case.id)"
        $failures = [Collections.Generic.List[string]]::new()
        if ($case.register) {
            $registration = Send @{operation='UPDATE_WORLD_VOCABULARY';conversation_id=$id;update=$suite.update}
            if (-not $registration.ok) { $failures.Add('REGISTER') }
        }
        foreach ($command in @($setup)) {
            $copy = $command | ConvertTo-Json -Depth 50 -Compress | ConvertFrom-Json -Depth 50 -AsHashtable
            $copy['conversation_id'] = $id
            if (-not (Send $copy).ok) { $failures.Add('SETUP') }
        }
        $turnRows = [Collections.Generic.List[object]]::new()
        $i = 0
        foreach ($text in $case.turns) {
            $i++
            $response = Send @{operation='PROCESS_CONVERSATION_TURN';request=@{
                schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$id;turn_index=$i;request_id="$id-$i"
                modality='TEXT';raw_text=$text;input_confidence_millis=1000;alternatives=@();output_language=$case.language
                context_tags=@();max_plan_steps=16
            }}
            if (-not $response.ok) { $failures.Add('API') }
            $value = $response.payload.value
            $world = $value.discourse_answer.world_reasoning
            if ($null -ne $world -and $world.decision.external_action_authorized) { $failures.Add('EXECUTION_AUTHORITY') }
            $compactWorld = if ($null -ne $world) { [ordered]@{query=$world.query;decision=$world.decision;semantic_decision_sha256=$world.semantic_decision_sha256} } else { $null }
            $turnRows.Add([ordered]@{input=$text;output=$value.output.text;api_ok=$response.ok;error=$response.error
                world_reasoning=$compactWorld;premises=@($value.conversation_state.dialogue_world.premises)
                implications=@($value.conversation_state.dialogue_world.implications)
                grounding=$value.conversation_state.dialogue_world.last_grounding
                response_act=$value.natural_realization.response_act})
        }
        $last = $turnRows[$turnRows.Count-1]
        $world = $last.world_reasoning
        $target = if ($null -ne $world) { $world.query.target[0] } else { $null }
        $active = @($last.premises | Where-Object { $null -ne $_ -and $_.active })
        if ($active.Count -ne $case.expected.premises) { $failures.Add('ACTIVE_PREMISE_COUNT') }
        if ($null -ne $case.expected.implications -and @($last.implications | Where-Object { $null -ne $_ }).Count -ne $case.expected.implications) { $failures.Add('IMPLICATION_COUNT') }
        if ($case.expected.verdict -eq 'NO_SUPPORTED_WORLD') {
            if ($null -ne $world -and $world.decision.verdict -in @('SUPPORTED','REFUTED','CONFLICT')) { $failures.Add('LEXICON_ABLATION') }
        } else {
            if ($null -eq $world) { $failures.Add('NO_WORLD_QUERY') }
            else {
                if ($world.decision.verdict -ne $case.expected.verdict) { $failures.Add('VERDICT') }
                if ($target.property.REGISTERED -ne $case.expected.predicate_id) { $failures.Add('PREDICATE') }
                if ($target.entity -ne $case.expected.entity -or $target.object -ne $case.expected.object) { $failures.Add('ROLES') }
                if ($world.query.target[1] -ne $true) { $failures.Add('QUERY_POLARITY') }
                if ($world.decision.hypothetical) { $failures.Add('UNREQUESTED_HYPOTHETICAL') }
                if ($case.expected.verdict -in @('SUPPORTED','REFUTED') -and @($world.decision.premise_evidence_ids).Count -eq 0) { $failures.Add('NO_EVIDENCE') }
                if ($null -ne $case.expected.min_proof_steps -and @($world.decision.proof_mechanism_ids).Count -lt $case.expected.min_proof_steps) { $failures.Add('MISSING_DERIVATION') }
            }
        }
        if ($turnRows.Count -gt 1) {
            $before = $turnRows[$turnRows.Count-2].premises | ConvertTo-Json -Depth 50 -Compress
            $after = $last.premises | ConvertTo-Json -Depth 50 -Compress
            if ($before -ne $after) { $failures.Add('QUERY_MUTATED_PREMISES') }
        }
        foreach ($premise in @($last.premises | Where-Object { $null -ne $_ })) {
            if ($premise.source_text -cnotin $case.turns) { $failures.Add('SOURCE_REWRITTEN') }
            # World source hashes encode the JSON string, not raw text bytes.
            $sourceJson = ConvertTo-Json -InputObject ([string]$premise.source_text) -Compress
            $actualHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($sourceJson))).ToLowerInvariant()
            if ($actualHash -ne $premise.source_sha256.ToLowerInvariant()) { $failures.Add('SOURCE_HASH') }
        }
        $rows.Add([ordered]@{id=$case.id;family=$case.family;expected=$case.expected;passed=($failures.Count-eq 0);failures=@($failures);turns=@($turnRows)})
    }
} catch {
    $errors.Add("NATIVE_OR_HARNESS_EXCEPTION:$($_.Exception.Message)")
} finally {
    $p.StandardInput.Close()
    if (-not $p.WaitForExit(30000)) { $p.Kill(); $p.WaitForExit(30000) | Out-Null }
    $errorText=$stderr.GetAwaiter().GetResult()
    if (-not [string]::IsNullOrWhiteSpace($errorText)) { $errors.Add($errorText) }
    $nativeExitCode = $p.ExitCode
    $p.Dispose()
}
$sorted=@($times | Sort-Object)
$passed=@($rows | Where-Object passed).Count
[ordered]@{schema='B_CORE_LEARNED_ROLE_BRIDGE_EVALUATION_1';mode=$Mode;classification=$suite.classification
    status=if($passed-eq $suite.cases.Count -and $errors.Count-eq 0){'PASS'}else{'FAIL'}
    executable_sha256=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    manifest_sha256=(Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash
    setup_sha256=if($SetupFile){(Get-FileHash -LiteralPath $SetupFile -Algorithm SHA256).Hash}else{$null}
    cases=$rows.Count;expected_cases=$suite.cases.Count;passed=$passed;api_errors=@($errors);native_exit_code=$nativeExitCode
    last_command=if($errors.Count -gt 0){$script:lastCommand}else{$null}
    p50_ms=if($sorted.Count){$sorted[[Math]::Ceiling($sorted.Count*0.5)-1]}else{$null}
    p95_ms=if($sorted.Count){$sorted[[Math]::Ceiling($sorted.Count*0.95)-1]}else{$null};peak_working_set_bytes=$peak
    timing_scope='Native roundtrip including startup and explicit setup, excluding host JSON parsing';rows=@($rows)
} | ConvertTo-Json -Depth 85 -Compress
if ($passed-ne $suite.cases.Count -or $errors.Count-ne 0) { exit 1 }
