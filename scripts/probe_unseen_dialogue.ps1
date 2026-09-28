param(
    [string]$Executable=(Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'),
    [string]$Cases=(Join-Path $PSScriptRoot '..\reports\language-cortex-completion\unseen_dialogue_cases_2026-09-05.json')
)
$ErrorActionPreference='Stop'
$fixture=Get-Content -LiteralPath $Cases -Raw | ConvertFrom-Json -Depth 100
$rows=[Collections.Generic.List[object]]::new()
$failures=[Collections.Generic.List[object]]::new()
foreach($case in $fixture.cases){
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
        for($i=0;$i -lt $case.turns.Count;$i++){
            # Only user text and public request fields enter the Rust process.
            # Expected meaning/category never enters a runtime request.
            $command=@{operation='PROCESS_CONVERSATION_TURN';request=@{
                schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id="UNSEEN-$($case.id)"
                turn_index=$i+1;request_id="$($case.id)-$i";modality='TEXT';raw_text=$case.turns[$i].text
                input_confidence_millis=1000;alternatives=@();output_language=$case.lang;context_tags=@();max_plan_steps=16
            }}
            $p.StandardInput.WriteLine(($command|ConvertTo-Json -Depth 12 -Compress))
        }
        $p.StandardInput.Close()
        if(-not $p.WaitForExit(30000)){$p.Kill();throw 'CLI_TIMEOUT'}
        $stdout=$outTask.GetAwaiter().GetResult()
        $stderr=$errTask.GetAwaiter().GetResult()
        $responses=@($stdout -split '\r?\n'|Where-Object{$_.Trim()}|ForEach-Object{$_|ConvertFrom-Json -Depth 100})
        if($p.ExitCode -ne 0 -or $responses.Count -ne $case.turns.Count){
            $failures.Add(@{case_id=$case.id;exit_code=$p.ExitCode;completed=$responses.Count;expected=$case.turns.Count;stderr=$stderr})
        }
        for($i=0;$i -lt $case.turns.Count;$i++){
            $r=if($i -lt $responses.Count){$responses[$i]}else{$null}
            $v=$r.payload.value
            $rows.Add([pscustomobject]@{
                id="$($case.id)-$i";category=$case.category;language=$case.lang
                input=$case.turns[$i].text;expectation=$case.turns[$i].expectation
                api_ok=($null -ne $r -and $r.ok);api_error=$r.error
                output=$v.output.text;response_act=$v.natural_realization.response_act
                conversation_contract=$v.conversation_contract
                action_state_analysis=$v.action_state_analysis
                predicate_frames=@($v.pragmatic_interpretation.compositional_analysis.frames | Select-Object frame_id,canonical_predicate,mood,temporal_reference,ability_polarity,polarity,theme,embedded_under_quote,external_execution_authorized)
                generation_action_references=@($v.natural_realization.generation_traces.meaning.nodes | Where-Object {$_.kind -eq 'EVENT_REFERENCE'} | Select-Object node_id,concept_id,grounding_refs)
                active_directives=@($v.conversation_state.dialogue_directive_ledger.directives | Where-Object {$_.status -eq 'ACTIVE'} | Select-Object kind,target_key,value_key,prohibited)
                stored_preferences=@($v.conversation_state.epistemic_ledger.records | Where-Object {$null -ne $_.content.interaction_preference} | ForEach-Object { [pscustomobject]@{belief_id=$_.belief_id;status=$_.status;source_actor=$_.source_actor;source=$_.proposition_surface;preference=$_.content.interaction_preference} })
                discourse_answer=$v.discourse_answer
                event_memory=@($v.conversation_state.epistemic_ledger.records | Select-Object belief_id,source_actor,proposition_surface,status,content)
                resolved_text=$v.reference_resolution.resolved_semantic_text
                reference_resolution=$v.reference_resolution
                has_grounded_plan=($null -ne $v.grounded_response)
                selected_plan_events=@($v.grounded_response.semantic_goal.selected_live_event_ids | Where-Object {$null -ne $_} | ForEach-Object {
                    $eventId=$_
                    $event=$v.grounded_response.semantic_goal.events | Where-Object {$_.event_id -eq $eventId}
                    [pscustomobject]@{event_id=$eventId;predicate=$event.predicate_concept_id;intent=$event.intent;targets=@($event.goal_subject_argument_ids | ForEach-Object {
                        $argumentId=$_
                        $v.grounded_response.semantic_goal.arguments | Where-Object {$_.argument_id -eq $argumentId} | ForEach-Object {$_.grounded_label}
                    })}
                })
                external_action_executed=$v.language_cortex_integration.external_action_executed
                unsupported_freeform_claims=$v.output.unsupported_freeform_claims
            })
        }
    }catch{
        $failures.Add(@{case_id=$case.id;harness_error=$_.Exception.Message})
    }finally{if(-not $p.HasExited){$p.Kill()};$p.Dispose()}
}
[pscustomobject]@{
    schema='UNSEEN_DIALOGUE_OBSERVATIONS_1';fixture_sha256=(Get-FileHash -LiteralPath $Cases).Hash.ToLowerInvariant()
    executable_sha256=(Get-FileHash -LiteralPath $Executable).Hash.ToLowerInvariant()
    cases=$fixture.cases.Count;rows=@($rows);process_failures=@($failures)
}|ConvertTo-Json -Depth 60 -Compress
