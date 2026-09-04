
param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
$taskCommands=[System.Collections.Generic.List[object]]::new()
$taskMeta=[System.Collections.Generic.List[object]]::new()
foreach($taskLang in @('KOREAN','ENGLISH')){
  foreach($taskFamily in @('PERSONAL','SLOT','REFERENCE','SPEAKER')){
    $taskId="CONVERSATION-$taskFamily-$taskLang"
    if($taskFamily -eq 'REFERENCE' -or $taskFamily -eq 'SPEAKER'){
      $taskCommands.Add(@{operation='UPDATE_WORLD_VOCABULARY';conversation_id=$taskId;update=@{
        predicates=@(@{predicate_id='W_USER_202';arity='BINARY'});remove_alias_ids=@()
        aliases=@(
          @{alias_id='ref.en';predicate_id='W_USER_202';language='ENGLISH';root='depend on';grammar='ENGLISH_REGULAR_VERB'},
          @{alias_id='ref.ko';predicate_id='W_USER_202';language='KOREAN';root='의존';grammar='KOREAN_HADA_LOCATIVE'}
        )
      }})
      $taskMeta.Add(@{operation='register';language=$taskLang;family=$taskFamily})
    }
    if($taskFamily -eq 'PERSONAL'){
      $taskTexts=if($taskLang -eq 'KOREAN'){@('음, 나 피곤해.','왜?','아니, 피곤하지 않아.','왜?','나 답답해.','그럼?')}else{@('Well, I am tired.','Why?','No, not tired.','Why?','I am frustrated.','Then?')}
      $taskExpected=@('MEMORY','SUPPORTED','MEMORY','SUPPORTED','MEMORY','SUPPORTED')
    }elseif($taskFamily -eq 'SLOT'){
      $taskTexts=if($taskLang -eq 'KOREAN'){@('lamp가 가동 상태이면 gate는 열림 상태다.','gate는 열림 상태인가?','가동 상태야.','왜?','고마워!','다른문은?','아니, lamp는 가동 상태가 아니야.','gate는 열림 상태인가?')}else{@('If lamp is active, then gate is open.','Is gate open?','Active.','Why?','Thanks!','What about sidegate?','No, lamp is not active.','Is gate open?')}
      $taskExpected=@('MEMORY','UNKNOWN','SUPPORTED','SUPPORTED','SOCIAL','UNKNOWN','MEMORY','UNKNOWN')
    }elseif($taskFamily -eq 'SPEAKER'){
      $taskTexts=if($taskLang -eq 'KOREAN'){@('나는 beta에 의존해.','그것은 안전 상태인가?','나야','alpha는 나에 의존해.')}else{@('I depend on beta.','Is it safe?','me','alpha depends on me.')}
      $taskExpected=@('MEMORY','CLARIFICATION','UNKNOWN','MEMORY')
    }else{
      $taskTexts=if($taskLang -eq 'KOREAN'){@('alpha는 beta에 의존한다.','그것은 안전 상태인가?','beta야','그것은 안전 상태다.','왜?')}else{@('alpha depends on beta.','Is it safe?','beta','It is safe.','Why?')}
      $taskExpected=@('MEMORY','CLARIFICATION','UNKNOWN','MEMORY','SUPPORTED')
    }
    for($taskIndex=0;$taskIndex -lt $taskTexts.Count;$taskIndex++){
      $taskCommands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
        schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$taskId;turn_index=$taskIndex+1;request_id="$taskId-$taskIndex"
        modality='TEXT';raw_text=$taskTexts[$taskIndex];input_confidence_millis=1000;alternatives=@();output_language=$taskLang;context_tags=@();max_plan_steps=16
      }})
      $taskMeta.Add(@{operation='turn';language=$taskLang;family=$taskFamily;turn=$taskIndex+1;input=$taskTexts[$taskIndex];expected=$taskExpected[$taskIndex]})
    }
  }
}
$taskStart=[System.Diagnostics.ProcessStartInfo]::new()
$taskStart.FileName=(Resolve-Path -LiteralPath $Executable).Path
$taskStart.UseShellExecute=$false
$taskStart.CreateNoWindow=$true
$taskStart.RedirectStandardInput=$true
$taskStart.RedirectStandardOutput=$true
$taskStart.RedirectStandardError=$true
$taskStart.StandardInputEncoding=[System.Text.UTF8Encoding]::new($false)
$taskStart.StandardOutputEncoding=[System.Text.UTF8Encoding]::new($false)
$taskStart.StandardErrorEncoding=[System.Text.UTF8Encoding]::new($false)
$taskProcess=[System.Diagnostics.Process]::new()
$taskProcess.StartInfo=$taskStart
[void]$taskProcess.Start()
$taskOut=$taskProcess.StandardOutput.ReadToEndAsync()
$taskErr=$taskProcess.StandardError.ReadToEndAsync()
foreach($taskCommand in $taskCommands){$taskProcess.StandardInput.WriteLine(($taskCommand | ConvertTo-Json -Depth 16 -Compress))}
$taskProcess.StandardInput.Close()
if(-not $taskProcess.WaitForExit(15000)){$taskProcess.Kill();throw 'CLI_TIMEOUT'}
$taskOutput=$taskOut.GetAwaiter().GetResult()
$taskErrors=$taskErr.GetAwaiter().GetResult()
if($taskProcess.ExitCode -ne 0){throw "CLI_EXIT_$($taskProcess.ExitCode): $taskErrors"}
$taskResponses=@($taskOutput -split '\r?\n' | Where-Object {$_.Trim()} | ForEach-Object {$_ | ConvertFrom-Json -Depth 100})
if($taskResponses.Count -ne $taskMeta.Count){throw 'CLI_RESPONSE_COUNT'}
$taskRows=[System.Collections.Generic.List[object]]::new()
foreach($taskIndex in 0..($taskResponses.Count-1)){
  $taskResponse=$taskResponses[$taskIndex]
  $taskInfo=$taskMeta[$taskIndex]
  if(-not $taskResponse.ok){throw "$($taskInfo | ConvertTo-Json -Compress) $($taskResponse | ConvertTo-Json -Depth 8 -Compress)"}
  $taskValue=$taskResponse.payload.value
  if($taskInfo.operation -ne 'turn'){$taskRows.Add(@{operation='register';language=$taskInfo.language});continue}
  $taskWorld=$taskValue.discourse_answer.world_reasoning
  $taskVerdict=if($null -ne $taskWorld){$taskWorld.decision.verdict}elseif($null -ne $taskValue.discourse_answer.world_clarification){'CLARIFICATION'}elseif($null -ne $taskValue.discourse_answer.world_memory_update){'MEMORY'}else{'SOCIAL'}
  if($taskVerdict -ne $taskInfo.expected){throw "VERDICT $($taskInfo.input): $taskVerdict expected $($taskInfo.expected), $($taskValue.output.text)"}
  if(@($taskValue.conversation_state.action_state_ledger.records).Count -ne 0){throw 'ACTION_AUTHORITY_LEAK'}
  if($taskInfo.family -eq 'PERSONAL' -and ($taskInfo.turn -eq 2 -or $taskInfo.turn -eq 4)){
    if($taskWorld.utterance_plan.moves[0].purpose -ne 'CAUSE_UNKNOWN'){throw 'SOURCE_IS_NOT_CAUSE'}
  }
  if($taskInfo.family -eq 'PERSONAL' -and $taskInfo.turn -eq 6){
    if(($taskWorld.query.target[0].property | ConvertTo-Json -Compress) -notmatch 'W_USER_900003'){throw 'STALE_FOCUS'}
  }
  if($taskInfo.family -eq 'REFERENCE' -and ($taskInfo.turn -eq 3 -or $taskInfo.turn -eq 5)){
    if($taskWorld.query.target[0].entity -ne 'beta'){throw 'WRONG_REFERENCE'}
  }
  if($taskValue.output.text -match '__user__'){throw 'INTERNAL_SPEAKER_ID_LEAK'}
  if($taskInfo.family -eq 'SPEAKER' -and $taskInfo.turn -eq 3 -and $taskWorld.query.target[0].entity -ne '__user__'){throw 'SPEAKER_REFERENCE_BINDING'}
  $taskRows.Add(@{operation='turn';family=$taskInfo.family;language=$taskInfo.language;turn=$taskInfo.turn;input=$taskInfo.input;output=$taskValue.output.text;verdict=$taskVerdict;action_records=0;
    utterance_moves=if($null -ne $taskWorld){@($taskWorld.utterance_plan.moves).Count}else{0}
    proof_steps=if($null -ne $taskWorld){@($taskWorld.decision.proof_mechanism_ids).Count}else{0}
  })
}
[pscustomobject]@{status='PASS';commands=$taskResponses.Count;conversation_turns=46;registrations=4;stderr=$taskErrors;rows=$taskRows;
  executable_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $taskStart.FileName).Hash.ToLowerInvariant()} | ConvertTo-Json -Depth 10
