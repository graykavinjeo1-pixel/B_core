param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
# Developer regression. Expected acts stay outside the runtime request.
$cases = ConvertFrom-Json -InputObject @'
[
  {
    "lang": "KOREAN",
    "turns": [
      "예린이 쉬는 게 좋겠네?",
      "그건 잘 모르겠어요.",
      "Why?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "UNKNOWN_ANSWER",
      "REASON_REQUEST",
      "REASON_REQUEST"
    ],
    "abstains": [
      false,
      true,
      true,
      true
    ],
    "explains": [
      false,
      false,
      false,
      true
    ],
    "id": "QUESTION_REPLY_0",
    "group": "QUESTION_REPLY_0"
  },
  {
    "lang": "KOREAN",
    "turns": [
      "예린이 쉬는 게 좋겠네?",
      "저는 대답하고 싶지 않아요.",
      "Why?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "DECLINED_ANSWER",
      "REASON_REQUEST",
      "REASON_REQUEST"
    ],
    "abstains": [
      false,
      true,
      true,
      true
    ],
    "explains": [
      false,
      false,
      false,
      true
    ],
    "id": "QUESTION_REPLY_1",
    "group": "QUESTION_REPLY_1"
  },
  {
    "lang": "KOREAN",
    "turns": [
      "예린이 쉬는 게 좋겠네?",
      "무슨 뜻인가요?",
      "맞아"
    ],
    "assess": [
      false,
      false,
      true
    ],
    "resumes": [
      false,
      false,
      true
    ],
    "facts": [
      0,
      0,
      1
    ],
    "kinds": [
      null,
      "RESTATEMENT",
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_2",
    "group": "QUESTION_REPLY_2"
  },
  {
    "lang": "KOREAN",
    "turns": [
      "예린이 쉬는 게 좋겠네?",
      "왜 물었어요?",
      "맞아"
    ],
    "assess": [
      false,
      false,
      true
    ],
    "resumes": [
      false,
      false,
      true
    ],
    "facts": [
      0,
      0,
      1
    ],
    "kinds": [
      null,
      "REASON_REQUEST",
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      true,
      false
    ],
    "id": "QUESTION_REPLY_3",
    "group": "QUESTION_REPLY_3"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I'm not sure.",
      "Why?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "UNKNOWN_ANSWER",
      "REASON_REQUEST",
      "REASON_REQUEST"
    ],
    "abstains": [
      false,
      true,
      true,
      true
    ],
    "explains": [
      false,
      false,
      false,
      true
    ],
    "id": "QUESTION_REPLY_4",
    "group": "QUESTION_REPLY_4"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I would rather not answer.",
      "Why?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "DECLINED_ANSWER",
      "REASON_REQUEST",
      "REASON_REQUEST"
    ],
    "abstains": [
      false,
      true,
      true,
      true
    ],
    "explains": [
      false,
      false,
      false,
      true
    ],
    "id": "QUESTION_REPLY_5",
    "group": "QUESTION_REPLY_5"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "What do you mean?",
      "맞아"
    ],
    "assess": [
      false,
      false,
      true
    ],
    "resumes": [
      false,
      false,
      true
    ],
    "facts": [
      0,
      0,
      1
    ],
    "kinds": [
      null,
      "RESTATEMENT",
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_6",
    "group": "QUESTION_REPLY_6"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Why did you ask?",
      "맞아"
    ],
    "assess": [
      false,
      false,
      true
    ],
    "resumes": [
      false,
      false,
      true
    ],
    "facts": [
      0,
      0,
      1
    ],
    "kinds": [
      null,
      "REASON_REQUEST",
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      true,
      false
    ],
    "id": "QUESTION_REPLY_7",
    "group": "QUESTION_REPLY_7"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I don't know.",
      "Yes."
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "UNKNOWN_ANSWER",
      null
    ],
    "abstains": [
      false,
      true,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_8",
    "group": "QUESTION_REPLY_8"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I'd rather not say.",
      "No."
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "DECLINED_ANSWER",
      null
    ],
    "abstains": [
      false,
      true,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_9",
    "group": "QUESTION_REPLY_9"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Never mind.",
      "What do you mean?"
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0
    ],
    "kinds": [
      null,
      null,
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_10",
    "group": "QUESTION_REPLY_10"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "What is entropy?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0
    ],
    "kinds": [
      null,
      null,
      null
    ],
    "abstains": [
      false,
      false,
      false
    ],
    "explains": [
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_11",
    "group": "QUESTION_REPLY_11"
  },
  {
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I don't know.",
      "Why?",
      "Why?",
      "Why did you ask?"
    ],
    "assess": [
      false,
      false,
      false,
      false,
      false
    ],
    "resumes": [
      false,
      false,
      false,
      false,
      false
    ],
    "facts": [
      0,
      0,
      0,
      0,
      0
    ],
    "kinds": [
      null,
      "UNKNOWN_ANSWER",
      "REASON_REQUEST",
      "REASON_REQUEST",
      null
    ],
    "abstains": [
      false,
      true,
      true,
      true,
      false
    ],
    "explains": [
      false,
      false,
      false,
      false,
      false
    ],
    "id": "QUESTION_REPLY_12",
    "group": "QUESTION_REPLY_12"
  }
]
'@
$exe = (Resolve-Path -LiteralPath $Executable).Path
$rows = [Collections.Generic.List[object]]::new()
$equivalent = @{}
foreach ($case in $cases) {
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
    try {
        for ($i = 0; $i -lt $case.turns.Count; $i++) {
            $command = @{operation='PROCESS_CONVERSATION_TURN'; request=@{
                schema='B_CORE_CONVERSATION_TURN_REQUEST_1'
                conversation_id=$case.id; turn_index=$i+1; request_id="$($case.id)-$i"
                modality='TEXT'; raw_text=$case.turns[$i]; input_confidence_millis=1000
                alternatives=@(); output_language=$case.lang; context_tags=@(); max_plan_steps=16
            }}
            $watch = [Diagnostics.Stopwatch]::StartNew()
            $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 12 -Compress))
            $p.StandardInput.Flush()
            $task = $p.StandardOutput.ReadLineAsync()
            if (-not $task.Wait(30000)) { throw "RESPONSE_TIMEOUT:$($case.id):$i" }
            $line = $task.GetAwaiter().GetResult()
            if ($null -eq $line) {
                if (-not $p.WaitForExit(5000)) { throw "CLOSED_OUTPUT_WITHOUT_EXIT:$($case.id)" }
                throw "NATIVE_EXIT:$($case.id):${i}:$($p.ExitCode):$($stderr.GetAwaiter().GetResult())"
            }
            $watch.Stop()
            $r = $line | ConvertFrom-Json -Depth 100
            $v = $r.payload.value
            $a = $v.discourse_answer
            $assessment = $a.decision_inquiry.assessment
            $boundary = (($null -ne $assessment) -eq $case.assess[$i]) -and
                $null -eq $v.grounded_response -and
                $v.conversation_state.action_state_ledger.records.Count -eq 0
            $boundary = $boundary -and (($null -ne $a.decision_inquiry.resumption) -eq $case.resumes[$i])
            $reply = $a.decision_inquiry.clarification_reply
            $boundary = $boundary -and $v.conversation_state.dialogue_world.premises.Count -eq $case.facts[$i]
            $boundary = $boundary -and $reply.kind -ceq $case.kinds[$i]
            if ($null -ne $case.kinds[$i]) {
                $boundary = $boundary -and $reply.original_source -ceq $case.turns[0] -and
                    $reply.question_turn -eq 1 -and $reply.turn -eq ($i+1) -and
                    (($null -ne $reply.abstention) -eq $case.abstains[$i]) -and
                    $reply.explains_question -eq $case.explains[$i]
            }
            if ($case.abstains[$i]) {
                $boundary = $boundary -and $null -eq $v.conversation_state.dialogue_world.last_query -and
                    -not $v.output.text.EndsWith('?')
            }
            if ($case.resumes[$i]) {
                $boundary = $boundary -and $a.decision_inquiry.resumption.original_source -ceq $case.turns[0] -and
                    $a.decision_inquiry.resumption.question_turn -eq 1
            }
            if ($case.assess[$i]) {
                $boundary = $boundary -and $assessment.derivation.disposition -eq 'GOAL_REACHABLE' -and
                    ($assessment.derivation.selected_plan.mechanism_ids -contains 'ABP_0001') -and
                    -not $a.dialogue_truth_established -and $assessment.derivation.external_action_execution_events -eq 0 -and
                    ($v.output.text.Contains('도움이 될 수') -or $v.output.text.Contains('may help')) -and
                    -not $v.output.text.Contains('알려줄래') -and -not $v.output.text.Contains('could you tell')
                if ($case.turns[$i] -match '^(왜|Why)') {
                    $boundary = $boundary -and ($v.output.text.Contains('위한 행동') -or $v.output.text.Contains('purpose'))
                }
            }
            if ($case.assess[$i] -and $null -ne $assessment) {
                $current = $assessment.request | ConvertTo-Json -Depth 40 -Compress
                $current += $assessment.derivation | ConvertTo-Json -Depth 40 -Compress
                if ($equivalent.ContainsKey($case.group)) {
                    $boundary = $boundary -and $equivalent[$case.group] -ceq $current
                } else { $equivalent[$case.group] = $current }
            }
            $pass = $null -ne $v -and $r.ok -and $boundary -and
                -not $v.language_cortex_integration.external_action_executed -and
                $v.output.unsupported_freeform_claims -eq 0
            $rows.Add([pscustomobject]@{
                case=$case.id; turn=$i+1; input=$case.turns[$i]; output=$v.output.text
                tested_boundary='QUESTION_REPLY_OWNERSHIP'; reply_kind=$reply.kind; explains_question=$reply.explains_question; abstention=$reply.abstention; facts=$v.conversation_state.dialogue_world.premises.Count; disposition=$a.disposition
                expected_assessment=$case.assess[$i]; assessment_present=($null -ne $assessment)
                resumption_present=($null -ne $a.decision_inquiry.resumption); gap_reason=$a.decision_inquiry.knowledge_gap.reason; core_disposition=$assessment.derivation.disposition; core_mechanisms=$assessment.derivation.selected_plan.mechanism_ids
                core_hash=$assessment.derivation.deliberation_sha256
                response_act=$v.natural_realization.response_act
                api_ok=$r.ok; pass=$pass; latency_ms=$watch.Elapsed.TotalMilliseconds
            })
        }
        $p.StandardInput.Close()
        if (-not $p.WaitForExit(30000)) { throw "EXIT_TIMEOUT:$($case.id)" }
        $err = $stderr.GetAwaiter().GetResult()
        if ($p.ExitCode -ne 0 -or $err) { throw "NATIVE_FAILURE:$($case.id):$($p.ExitCode):$err" }
    } finally {
        if (-not $p.HasExited) { $p.Kill() }
        $p.Dispose()
    }
}
$failed = @($rows | Where-Object { -not $_.pass }).Count
[pscustomobject]@{
    status=$(if($failed){'FAIL'}else{'PASS'}); cases=$cases.Count; turns=$rows.Count
    failed=$failed; executable_sha256=(Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant()
    blind=$false; natural_conversation_status='NOT_ESTABLISHED'
    timing_scope='Debug executable; write-to-complete-JSON-line; fresh process per case; initialization included in first turns; no SLA claim.'
    rows=@($rows)
} | ConvertTo-Json -Depth 15
if ($failed) { throw 'QUESTION_REPLY_REGRESSION' }
