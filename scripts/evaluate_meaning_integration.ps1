#requires -Version 7.0
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'),
    [string]$Manifest = (Join-Path $PSScriptRoot '../reports/language-cortex-completion/meaning-learning-2026-09-09/integration_manifest.json')
)
# Supervisor-only evaluator. No production component may load its expectations.
$ErrorActionPreference = 'Stop'
$suite = Get-Content -LiteralPath $Manifest -Raw | ConvertFrom-Json
$exe = (Resolve-Path -LiteralPath $Executable).Path
$binaryHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
$manifestHash = (Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash
if ($suite.cases.Count -ne 120 -or $suite.memory_sessions_cases.Count -ne 24) { throw 'INVALID_SUITE_COUNTS' }
function Matches-Roles($event, $roles) {
    foreach ($field in $roles.psobject.Properties) {
        if ($event.roles.($field.Name) -ine $field.Value) { return $false }
    }
    return $true
}
function Check-Meaning($value, $check) {
    $answer = $value.discourse_answer
    $world = $answer.world_reasoning
    $records = @($value.conversation_state.epistemic_ledger.records)
    switch ($check.kind) {
        'BINDING_VALUE' { return $null -ne $answer.content_projection -and $answer.content_projection.binding.value -ieq $check.value }
        'NO_CONTENT_BINDING' {
            return $null -eq $answer.content_projection -and
                ($answer.disposition -eq 'NO_MATCHING_RECORD' -or
                 $value.natural_realization.response_act -eq 'CLARIFICATION_REQUEST')
        }
        'ACTIVE_ATTRIBUTED_EVENT' {
            foreach ($record in $records) {
                if ($record.status -ne 'ACTIVE' -or $record.signature.modal_world -ne 'ACTUAL') { continue }
                foreach ($event in $record.content.events) {
                    if ((Matches-Roles $event $check.roles) -and $event.negated -eq $check.negated) { return $true }
                }
            }
            return $false
        }
        'NO_ACTUAL_EVENT' {
            foreach ($record in $records) {
                if ($record.status -ne 'ACTIVE' -or $record.signature.modal_world -ne 'ACTUAL' -or $record.source_actor -ne 'DIALOGUE_USER') { continue }
                foreach ($event in $record.content.events) {
                    if ((Matches-Roles $event $check.roles) -and -not $event.negated) { return $false }
                }
            }
            return $true
        }
        'WORLD_VERDICT' { return $null -ne $world -and $world.decision.verdict -eq $check.value -and -not $world.decision.hypothetical }
        'PROOF_USES_MECHANISM' { return $null -ne $world -and $world.decision.proof_mechanism_ids.Count -gt 0 -and $world.decision.premise_evidence_ids.Count -gt 0 }
        'NO_CAUSAL_MECHANISM_CLAIM' {
            return $null -ne $world -and $world.decision.proof_mechanism_ids.Count -eq 0
        }
        'SUPERSEDED_WORLD_PREMISE' {
            return @($value.conversation_state.dialogue_world.premises | Where-Object { $_.active -eq $false }).Count -gt 0
        }
        'ATTRIBUTED_SPEAKER' {
            return $answer.disposition -eq 'ANSWERED_FROM_DIALOGUE_RECORDS' -and
                @($answer.evidence | Where-Object { $_.source_actor -ieq $check.value }).Count -gt 0
        }
        'NO_NAMED_SPEAKER_ANSWER' {
            # AmbiguousQuery is the native typed request for a source/referent,
            # realized as DiscourseAnswer; it is not a named speaker answer.
            return $null -ne $answer -and $answer.evidence.Count -eq 0 -and $answer.claims.Count -eq 0 -and
                $answer.disposition -in @('NO_MATCHING_RECORD', 'AMBIGUOUS_QUERY')
        }
        'NO_LIVE_QUERY' { return $null -eq $answer.query }
        'SOURCE_BOUND_PRONOUN' {
            $projection = $answer.content_projection
            if ($null -eq $projection -or $projection.context_sources.Count -eq 0) { return $false }
            $pattern = if ($check.surface -eq 'it') { '\bit\b' } else { [regex]::Escape($check.surface) }
            return $projection.source_proposition -match $pattern
        }
        'RESPONSE_ACT' { return $value.natural_realization.response_act -eq $check.value }
        'NOT_PLAN_PREVIEW' { return $value.natural_realization.response_act -ne 'PLAN_PREVIEW' }
        'NO_EXECUTION_AUTHORITY' {
            return @($value.conversation_state.action_state_ledger.records | Where-Object {
                $_.execution_status -ne 'NOT_OBSERVED' -or $_.verified_outcome -or $_.external_action_execution_observed
            }).Count -eq 0 -and @($records | Where-Object { $_.external_execution_authorized -or $_.dialogue_truth_established }).Count -eq 0 -and
                ($null -eq $world -or -not $world.decision.external_action_authorized)
        }
        default { throw "UNKNOWN_EVALUATOR_CHECK:$($check.kind)" }
    }
}
$start = [Diagnostics.ProcessStartInfo]::new($exe)
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
$p = [Diagnostics.Process]::Start($start)
$stderr = $p.StandardError.ReadToEndAsync()
$rows = [Collections.Generic.List[object]]::new()
$latencies = [Collections.Generic.List[double]]::new()
$peakWorkingSet = 0L
$turnCount = 0
try {
    foreach ($case in @($suite.cases) + @($suite.memory_sessions_cases)) {
        # Keep single-turn cases as an array; a scalar string indexes characters.
        $texts = @(if ($null -ne $case.target) { @($case.setup) + @($case.target) } else { @($case.turns) })
        if ($texts.Count -eq 0 -or @($texts | Where-Object { $_ -isnot [string] -or [string]::IsNullOrWhiteSpace($_) }).Count -gt 0) { throw "INVALID_CASE_TEXT:$($case.id)" }
        $outputs = [Collections.Generic.List[string]]::new()
        $failures = [Collections.Generic.List[string]]::new()
        $value = $null
        for ($i = 0; $i -lt $texts.Count; $i++) {
            $command = @{operation='PROCESS_CONVERSATION_TURN'; request=@{
                schema='B_CORE_CONVERSATION_TURN_REQUEST_1'; conversation_id=$case.id; turn_index=$i+1;
                request_id="$($case.id)-$i"; modality='TEXT'; raw_text=$texts[$i];
                input_confidence_millis=1000; alternatives=@(); output_language=$case.language; context_tags=@(); max_plan_steps=16
            }}
            $watch = [Diagnostics.Stopwatch]::StartNew()
            $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 12 -Compress))
            $p.StandardInput.Flush()
            $pending = $p.StandardOutput.ReadLineAsync()
            if (-not $pending.Wait(20000)) { throw "NATIVE_TIMEOUT:$($case.id):$i" }
            if ($null -eq $pending.Result) { throw "NATIVE_EXIT:$($case.id):$i" }
            $watch.Stop()
            $latencies.Add($watch.Elapsed.TotalMilliseconds)
            $response = $pending.Result | ConvertFrom-Json
            $turnCount++
            $p.Refresh()
            $peakWorkingSet = [Math]::Max($peakWorkingSet, $p.PeakWorkingSet64)
            if (-not $response.ok) { $failures.Add("API:$($response.error | ConvertTo-Json -Compress)"); break }
            $value = $response.payload.value
            $outputs.Add($value.output.text)
            if (-not (Check-Meaning $value ([pscustomobject]@{kind='NO_EXECUTION_AUTHORITY'}))) { $failures.Add("TURN_${i}:EXECUTION_BOUNDARY") }
        }
        if ($failures.Count -eq 0) {
            foreach ($check in $case.checks) { if (-not (Check-Meaning $value $check)) { $failures.Add($check.kind) } }
        }
        $rows.Add([ordered]@{id=$case.id;family=$case.family;language=$case.language;classification=$case.classification;
            passed=$failures.Count -eq 0;failures=@($failures);inputs=$texts;outputs=@($outputs)})
    }
} finally {
    if (-not $p.HasExited) { $p.StandardInput.Close(); if (-not $p.WaitForExit(2000)) { $p.Kill(); $p.WaitForExit(2000) | Out-Null } }
    $p.Dispose()
}
if ($stderr.Wait(2000) -and $stderr.Result) { throw "NATIVE_STDERR:$($stderr.Result)" }
if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $binaryHash -or
    (Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash -ne $manifestHash) { throw 'EVALUATION_ARTIFACT_CHANGED' }
$orderedLatency = @($latencies | Sort-Object)
$failed = @($rows | Where-Object { -not $_.passed })
[ordered]@{schema='B_CORE_LANGUAGE_INTEGRATION_EVALUATION_1';classification=$suite.classification;
    executable_sha256=$binaryHash;manifest_sha256=$manifestHash;cases=$rows.Count;passed=$rows.Count-$failed.Count;failed=$failed.Count;
    total_turns=$turnCount;p50_ms=$orderedLatency[[int][Math]::Floor(($orderedLatency.Count-1)*0.5)];
    p95_ms=$orderedLatency[[int][Math]::Floor(($orderedLatency.Count-1)*0.95)];peak_working_set_bytes=$peakWorkingSet;
    timing_scope='UTF8 JSON request write through complete response line, one process, includes first turn initialization; excludes JSON parsing in evaluator';
    naturalness_assessment='NOT_PERFORMED';rows=@($rows)} | ConvertTo-Json -Depth 20
if ($failed.Count -gt 0) { exit 1 }
