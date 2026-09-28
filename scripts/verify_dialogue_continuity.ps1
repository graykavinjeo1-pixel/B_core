param(
    [string]$Executable=(Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'),
    [string]$Cases=(Join-Path $PSScriptRoot '..\reports\language-cortex-completion\dialogue_continuity_cases_2026-09-05.json')
)
$ErrorActionPreference='Stop'
$fixture=Get-Content -LiteralPath $Cases -Raw | ConvertFrom-Json
$commands=[Collections.Generic.List[object]]::new()
$checks=[Collections.Generic.List[object]]::new()
foreach($case in $fixture.cases){
    for($i=0;$i -lt $case.turns.Count;$i++){
        $turn=$case.turns[$i]
        $commands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
            schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id="CONTINUITY-$($case.id)"
            turn_index=$i+1;request_id="$($case.id)-$i";modality='TEXT';raw_text=$turn.text
            input_confidence_millis=1000;alternatives=@();output_language=$case.lang;context_tags=@();max_plan_steps=16
        }})
        $checks.Add(@{case_id=$case.id;category=$case.category;turn=$turn})
    }
}
$start=[Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Executable).Path)
$start.UseShellExecute=$false
$start.CreateNoWindow=$true
$start.RedirectStandardInput=$true
$start.RedirectStandardOutput=$true
$start.RedirectStandardError=$true
$start.StandardInputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardErrorEncoding=[Text.UTF8Encoding]::new($false)
$p=[Diagnostics.Process]::Start($start)
$outTask=$p.StandardOutput.ReadToEndAsync()
$errTask=$p.StandardError.ReadToEndAsync()
try{
    foreach($command in $commands){$p.StandardInput.WriteLine(($command|ConvertTo-Json -Depth 12 -Compress))}
    $p.StandardInput.Close()
    if(-not $p.WaitForExit(30000)){$p.Kill();throw 'CLI_TIMEOUT'}
    $stdout=$outTask.GetAwaiter().GetResult()
    $stderr=$errTask.GetAwaiter().GetResult()
    if($p.ExitCode -ne 0){throw "CLI_CRASH_$($p.ExitCode): $stderr"}
    $responses=@($stdout -split '\r?\n'|Where-Object{$_.Trim()}|ForEach-Object{$_|ConvertFrom-Json -Depth 100})
    if($responses.Count -ne $commands.Count){throw 'INCOMPLETE_OUTPUT'}
    function EntityKey($value){
        if($null -eq $value){return $null}
        # English articles are not entity identity. This normalization is declared
        # before evaluation; the historical exact-match failure is not overwritten.
        return ([string]$value).ToLowerInvariant().Trim() -replace '^(the|a|an) ', ''
    }
    $rows=for($i=0;$i -lt $responses.Count;$i++){
        $r=$responses[$i];$v=$r.payload.value;$check=$checks[$i];$turn=$check.turn
        $text=[string]$v.output.text;$projection=$v.discourse_answer.content_projection
        $failures=[Collections.Generic.List[string]]::new()
        if(-not $r.ok){$failures.Add("API:$($r.error|ConvertTo-Json -Compress)")}
        if($v.output.unsupported_freeform_claims -ne 0){$failures.Add('UNSUPPORTED_CLAIMS')}
        if($v.language_cortex_integration.external_action_executed -or $null -ne $v.grounded_response -or @($v.conversation_state.action_state_ledger.records).Count -gt 0){$failures.Add('UNREQUESTED_ACTION_OR_PLAN')}
        if($turn.PSObject.Properties.Name -contains 'expected'){
            if((EntityKey $projection.binding.value) -cne (EntityKey $turn.expected)){$failures.Add('WRONG_BINDING')}
            if($null -ne $turn.expected -and -not $text.ToLowerInvariant().Contains((EntityKey $turn.expected))){$failures.Add('ANSWER_NOT_EXPRESSED')}
        }
        foreach($part in $turn.contains){if(-not $text.ToLowerInvariant().Contains($part.ToLowerInvariant())){$failures.Add("MISSING:$part")}}
        foreach($part in $turn.forbid){if($text.ToLowerInvariant().Contains($part.ToLowerInvariant())){$failures.Add("FORBIDDEN:$part")}}
        if($turn.binding_slot -and $projection.binding.slot -ne $turn.binding_slot){$failures.Add('WRONG_SLOT')}
        if($turn.disposition -and $v.discourse_answer.disposition -ne $turn.disposition){$failures.Add('WRONG_DISPOSITION')}
        [pscustomobject]@{
            id=$commands[$i].request.request_id;case_id=$check.case_id;category=$check.category
            input=$turn.text;output=$text;expectation=$turn;actual=$projection.binding.value
            bindings=@($projection.binding)+@($projection.additional_bindings)
            selected_act=$v.natural_realization.response_act;disposition=$v.discourse_answer.disposition
            resolved_text=$v.reference_resolution.resolved_semantic_text
            contextual_sources=@($projection.context_sources);failure_reasons=@($failures);pass=($failures.Count -eq 0)
        }
    }
    $failedCases=@($rows|Where-Object{-not $_.pass}|Select-Object -ExpandProperty case_id -Unique)
    [pscustomobject]@{status=$(if($failedCases.Count){'FAIL'}else{'PASS'});suite='BOUNDED_CONVERSATIONAL_CONTINUITY_NOT_BLIND'
        cases=$fixture.cases.Count;passed_cases=$fixture.cases.Count-$failedCases.Count;turns=$rows.Count
        passed_turns=@($rows|Where-Object{$_.pass}).Count
        fixture_sha256=(Get-FileHash -LiteralPath $Cases).Hash.ToLowerInvariant()
        executable_sha256=(Get-FileHash -LiteralPath $start.FileName).Hash.ToLowerInvariant()
        rows=@($rows);stderr=$stderr
    }|ConvertTo-Json -Depth 20
}finally{if(-not $p.HasExited){$p.Kill()};$p.Dispose()}

