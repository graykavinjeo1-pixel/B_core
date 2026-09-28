#requires -Version 7.0
param([string]$Executable=(Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
$base=Join-Path $PSScriptRoot '../reports/language-cortex-completion/learned-role-bridge-2026-09-09'
$setup=@(Get-Content (Join-Path $base 'setup-resolve.json') -Raw | ConvertFrom-Json -Depth 40 -AsHashtable)[0]
$vocab=(Get-Content (Join-Path $base 'manifest.json') -Raw | ConvertFrom-Json -Depth 40).update
$start=[Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Executable).Path)
$start.UseShellExecute=$false; $start.CreateNoWindow=$true
$start.RedirectStandardInput=$true; $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
$start.StandardInputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardErrorEncoding=[Text.UTF8Encoding]::new($false)
$p=[Diagnostics.Process]::Start($start)
$stderr=$p.StandardError.ReadToEndAsync()
$rows=[Collections.Generic.List[object]]::new()
function Send($command) {
    $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 50 -Compress)); $p.StandardInput.Flush()
    $pending=$p.StandardOutput.ReadLineAsync()
    if(-not $pending.Wait(30000)){throw 'TIMEOUT'}
    if($null-eq $pending.Result){throw 'EOF'}
    $pending.Result | ConvertFrom-Json -Depth 100
}
function Setup([string]$id) {
    $copy=$setup | ConvertTo-Json -Depth 40 -Compress | ConvertFrom-Json -AsHashtable -Depth 40
    $copy['conversation_id']=$id
    return $copy
}
function Register([string]$id) { Send @{operation='UPDATE_WORLD_VOCABULARY';conversation_id=$id;update=$vocab} }
function Turn([string]$id,[int]$index,[string]$text) {
    Send @{operation='PROCESS_CONVERSATION_TURN';request=@{
        schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$id;turn_index=$index;request_id="$id-$index"
        modality='TEXT';raw_text=$text;input_confidence_millis=1000;alternatives=@();output_language='KOREAN'
        context_tags=@();max_plan_steps=16
    }}
}
function Check([string]$name,[bool]$passed,$detail) { $rows.Add([ordered]@{name=$name;passed=$passed;detail=$detail}) }
try {
    $null=Register 'INSTALL-A'
    $first=Send (Setup 'INSTALL-A')
    Check 'native_training_and_install' ([bool]$first.ok) $first.error
    $duplicate=Send (Setup 'INSTALL-A')
    Check 'identical_pre_turn_install_idempotent' ($duplicate.ok -and $duplicate.payload.value.state_sha256-eq $first.payload.value.state_sha256) $duplicate.error
    $conflict=Setup 'INSTALL-A'; $conflict.model_version='conflicting-version'
    $rejected=Send $conflict
    Check 'conflicting_pre_turn_model_rejected' (-not $rejected.ok) $rejected.error
    $retained=Register 'INSTALL-A'
    Check 'failed_install_preserves_model' ($retained.ok -and $retained.payload.value.dialogue_world.vocabulary.syntax_model.training_sha256-eq $first.payload.value.dialogue_world.vocabulary.syntax_model.training_sha256) $retained.error
    $statement=Turn 'INSTALL-A' 1 '세린을 루빈은 지원해.'
    Check 'installed_model_reaches_memory' ($statement.ok -and @($statement.payload.value.conversation_state.dialogue_world.premises).Count-eq 1) $statement.error
    $late=Send (Setup 'INSTALL-A')
    Check 'post_turn_install_rejected' (-not $late.ok) $late.error
    $query=Turn 'INSTALL-A' 2 '루빈은 세린을 지원하나요?'
    Check 'late_rejection_preserves_old_meaning' ($query.ok -and $query.payload.value.discourse_answer.world_reasoning.decision.verdict-eq 'SUPPORTED') $query.error
    $null=Register 'INSTALL-B'
    $untrained=Turn 'INSTALL-B' 1 '세린을 루빈은 지원해.'
    $untrainedQuery=Turn 'INSTALL-B' 2 '루빈은 세린을 지원하나요?'
    Check 'cross_session_model_and_memory_isolation' ($untrained.ok -and $untrainedQuery.ok -and @($untrained.payload.value.conversation_state.dialogue_world.premises).Count-eq 0 -and $untrainedQuery.payload.value.discourse_answer.world_reasoning.decision.verdict-eq 'UNKNOWN') $untrainedQuery.error
    $modelOnly=Send (Setup 'INSTALL-C')
    $ungrounded=Turn 'INSTALL-C' 1 '세린을 루빈은 지원해.'
    Check 'model_cannot_register_predicate_or_fact' ($modelOnly.ok -and $ungrounded.ok -and $null-eq $modelOnly.payload.value.dialogue_world.vocabulary.predicates.W_USER_710006 -and @($ungrounded.payload.value.conversation_state.dialogue_world.premises).Count-eq 0) $ungrounded.error
    foreach($kind in @('colliding_positions','oversized_observations','invalid_grammar','overlong_source')) {
        $command=Setup "INVALID-$kind"
        switch($kind) {
            'colliding_positions' { $command.observations[0].subject_position=0; $command.observations[0].object_position=0 }
            'oversized_observations' { $command.observations=@(0..128 | ForEach-Object { $command.observations[0] }) }
            'invalid_grammar' { $command.observations[0].grammar='COPULAR' }
            'overlong_source' { $command.observations[0].source_ref=('x'*129) }
        }
        $bad=Send $command
        $state=Register "INVALID-$kind"
        Check "$kind-rejected-atomically" (-not $bad.ok -and $state.ok -and $null-eq $state.payload.value.dialogue_world.vocabulary.syntax_model) $bad.error
    }
} catch { Check 'unexpected_exception' $false $_.Exception.Message }
finally {
    $p.StandardInput.Close()
    if(-not $p.WaitForExit(30000)){$p.Kill();$p.WaitForExit(30000)|Out-Null}
    $errorText=$stderr.GetAwaiter().GetResult();$exitCode=$p.ExitCode;$p.Dispose()
}
$passed=@($rows|Where-Object passed).Count
[ordered]@{schema='B_CORE_LEARNED_ROLE_INSTALLATION_REVIEW_1';classification='SUPERVISOR_BOUNDARY_REVIEW_NOT_BLIND'
    executable_sha256=(Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
    status=if($rows.Count-eq 13 -and $passed-eq 13 -and $exitCode-eq 0 -and -not $errorText){'PASS'}else{'FAIL'}
    checks=$rows.Count;passed=$passed;native_exit_code=$exitCode;stderr=$errorText;rows=@($rows)
}|ConvertTo-Json -Depth 15 -Compress
if($passed-ne 13 -or $exitCode-ne 0 -or $errorText){exit 1}
