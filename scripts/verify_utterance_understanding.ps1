param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
# Diagnostic corpus, not a blind benchmark or runtime sentence knowledge.
$cases=ConvertFrom-Json -InputObject @'
{
  "DAILY": [
    "오늘 진짜 피곤하네.",
    "어제 세 시간밖에 못 잤거든.",
    "그럼 지금 뭘 하는 게 좋을까?",
    "길게 말고 하나만 추천해줘."
  ],
  "PERSONAL": [
    "나 피곤해.",
    "왜?",
    "아니, 피곤한 게 아니라 답답한 거야.",
    "그럼 어떻게 하지?"
  ],
  "CORRECTION": [
    "회의는 내일이 아니라 모레야.",
    "그럼 언제 준비하면 좋을까?",
    "자료 정리는 이미 끝났어.",
    "그다음은?"
  ],
  "SOCIAL": [
    "고마워. 네 덕분에 정리가 됐어.",
    "ㅋㅋ 그러게, 혼자 끙끙댔네.",
    "근데 오늘 저녁은 뭘 먹지?"
  ],
  "REFERENCE": [
    "민수는 지연에게 책을 빌려줬어.",
    "그 사람한테 돌려달라고 해도 될까?",
    "민수 말이야.",
    "내가 말한 건 책을 돌려받는 거야."
  ],
  "MEAL": [
    "먹었어?",
    "아직 안 먹었어.",
    "뭐 먹을까?"
  ],
  "CONTROL": [
    "lamp가 가동 상태이면 gate는 열림 상태다.",
    "lamp는 가동 상태다.",
    "gate는 열림 상태인가?",
    "왜?"
  ]
}
'@ -AsHashtable
$commands=[Collections.Generic.List[object]]::new()
foreach($family in @('DAILY','PERSONAL','CORRECTION','SOCIAL','REFERENCE','MEAL','CONTROL')){
  for($i=0;$i -lt $cases[$family].Count;$i++){
    $commands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
      schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id="INQUIRY-$family";turn_index=$i+1
      request_id="$family-$i";modality='TEXT';raw_text=$cases[$family][$i];input_confidence_millis=1000
      alternatives=@();output_language='KOREAN';context_tags=@();max_plan_steps=16
    }})
  }
}
$start=[Diagnostics.ProcessStartInfo]::new()
$start.FileName=(Resolve-Path -LiteralPath $Executable).Path
$start.UseShellExecute=$false
$start.CreateNoWindow=$true
$start.RedirectStandardInput=$true
$start.RedirectStandardOutput=$true
$start.RedirectStandardError=$true
$start.StandardInputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
$start.StandardErrorEncoding=[Text.UTF8Encoding]::new($false)
$p=[Diagnostics.Process]::new()
$p.StartInfo=$start
[void]$p.Start()
$outTask=$p.StandardOutput.ReadToEndAsync()
$errTask=$p.StandardError.ReadToEndAsync()
try {
  foreach($command in $commands){$p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 12 -Compress))}
  $p.StandardInput.Close()
  if(-not $p.WaitForExit(30000)){$p.Kill();throw 'CLI_TIMEOUT'}
  $raw=$outTask.GetAwaiter().GetResult()
  $stderr=$errTask.GetAwaiter().GetResult()
  if($p.ExitCode -ne 0){throw "CLI_EXIT_$($p.ExitCode): $stderr"}
  $responses=@($raw -split '\r?\n' | Where-Object{$_.Trim()} | ForEach-Object {$_ | ConvertFrom-Json -Depth 100})
  if($responses.Count -ne $commands.Count){throw 'MISSING_RESPONSES'}
  $decisionIds=@('DAILY-2','DAILY-3','PERSONAL-3','CORRECTION-1','SOCIAL-2','MEAL-2')
  $rows=for($i=0;$i -lt $responses.Count;$i++){
    $r=$responses[$i];$v=$r.payload.value;$id=$commands[$i].request.request_id
    if(-not $r.ok){throw "$id : $($r.error | ConvertTo-Json -Depth 8)"}
    $typed=$null -ne $v.discourse_answer.world_memory_update -or $null -ne $v.discourse_answer.world_reasoning -or $null -ne $v.discourse_answer.decision_inquiry
    if($typed -and @($v.natural_realization.response_plan.moves).Count -ne 1){throw "ANSWER_CONTENT_INTRUSION:$id"}
    if($id -in $decisionIds){
      if($null -eq $v.discourse_answer.decision_inquiry -or -not $v.conversation_contract.information_requested -or $v.conversation_contract.independent_action_requested){throw "DECISION_ROUTE:$id"}
    }
    if($id -eq 'DAILY-0' -and $null -eq $v.discourse_answer.world_memory_update){throw 'MODIFIER_STATE_LOST'}
    if($id -eq 'PERSONAL-2'){
      $current=@($v.conversation_state.dialogue_world.premises | Where-Object {$_.introduced_turn -eq 3})
      if($current.Count -ne 2 -or $current[0].value -or -not $current[1].value){throw 'NONATOMIC_CORRECTION'}
    }
    if(@($v.conversation_state.action_state_ledger.records).Count -ne 0 -or $null -ne $v.grounded_response){throw "ACTION_LEAK:$id"}
    [pscustomobject]@{id=$id;input=$commands[$i].request.raw_text;output=$v.output.text
      typed_world_or_decision=$typed;missing_decision_input=$v.discourse_answer.decision_inquiry.missing_input
      world_verdict=$v.discourse_answer.world_reasoning.decision.verdict
      selected_act=$v.natural_realization.response_act;response_moves=@($v.natural_realization.response_plan.moves).Count
    }
  }
  [pscustomobject]@{boundary_status='PASS';natural_conversation_status='NOT_ESTABLISHED'
    completed_turns=$responses.Count;decision_inquiries=$decisionIds.Count;stderr=$stderr
    executable_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $start.FileName).Hash.ToLowerInvariant()
    rows=@($rows)
  } | ConvertTo-Json -Depth 10
} finally {
  if(-not $p.HasExited){$p.Kill()}
  $p.Dispose()
}

