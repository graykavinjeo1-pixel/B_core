param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
# Developer path regression, not a blind language benchmark or a runtime corpus.
# Run the real executable: Rust test threads can conceal main-thread stack faults.
$cases = ConvertFrom-Json -InputObject @'
[
  {
    "id": "POLAR_0",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "응",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_0"
  },
  {
    "id": "POLAR_1",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "그래",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_1"
  },
  {
    "id": "POLAR_2",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "맞아",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_2"
  },
  {
    "id": "POLAR_3",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "맞아요",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_3"
  },
  {
    "id": "POLAR_4",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "그렇습니다",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_4"
  },
  {
    "id": "POLAR_5",
    "lang": "KOREAN",
    "turns": [
      "서윤이 쉬는 게 좋겠네?",
      "아니요",
      "왜 그렇게 생각해?"
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": false,
    "question": true,
    "group": "POLAR_5"
  },
  {
    "id": "POLAR_6",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Yeah!",
      "Why do you think that?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_6"
  },
  {
    "id": "POLAR_7",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Yes.",
      "Why do you think that?"
    ],
    "assess": [
      false,
      true,
      true
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": true,
    "question": true,
    "group": "POLAR_7"
  },
  {
    "id": "POLAR_8",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "No.",
      "Why do you think that?"
    ],
    "assess": [
      false,
      false,
      false
    ],
    "resumes": [
      false,
      true,
      true
    ],
    "facts": [
      0,
      1,
      1
    ],
    "positive": false,
    "question": true,
    "group": "POLAR_8"
  },
  {
    "id": "POLAR_9",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Never mind.",
      "그래"
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
    "question": true,
    "group": "POLAR_9"
  },
  {
    "id": "POLAR_10",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "What is entropy?",
      "그래"
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
    "question": true,
    "group": "POLAR_10"
  },
  {
    "id": "POLAR_11",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Thanks.",
      "Thanks.",
      "Thanks.",
      "그래"
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
    "question": true,
    "group": "POLAR_11"
  },
  {
    "id": "POLAR_12",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "알겠어"
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "facts": [
      0,
      0
    ],
    "question": true,
    "group": "POLAR_12"
  },
  {
    "id": "POLAR_13",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "그래?"
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "facts": [
      0,
      0
    ],
    "question": true,
    "group": "POLAR_13"
  },
  {
    "id": "POLAR_14",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "네 아니요"
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "facts": [
      0,
      0
    ],
    "question": true,
    "group": "POLAR_14"
  },
  {
    "id": "POLAR_15",
    "lang": "ENGLISH",
    "turns": [
      "Proceed?",
      "그래"
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "facts": [
      0,
      0
    ],
    "question": false,
    "group": "POLAR_15"
  },
  {
    "id": "POLAR_16",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Yes.",
      "No."
    ],
    "assess": [
      false,
      true,
      false
    ],
    "resumes": [
      false,
      true,
      false
    ],
    "facts": [
      0,
      1,
      1
    ],
    "question": true,
    "group": "POLAR_16"
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
            $boundary = $boundary -and $v.conversation_state.dialogue_world.premises.Count -eq $case.facts[$i]
            if ($i -eq 0 -and $case.question) {
                $boundary = $boundary -and ($v.output.text.Contains('피곤') -or $v.output.text.Contains('tired')) -and
                    $v.output.text.EndsWith('?') -and
                    $null -ne $v.conversation_state.dialogue_world.last_query.clarification_goal
            }
            if ($case.resumes[$i]) {
                $premise = $v.conversation_state.dialogue_world.premises[-1]
                $boundary = $boundary -and $null -ne $premise.answer_binding -and
                    $premise.source_text -ceq $case.turns[1]
                if ($null -ne $case.positive) { $boundary = $boundary -and $premise.value -eq $case.positive }
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
                tested_boundary='GROUNDED_POLAR_REPLY'; premise_count=$v.conversation_state.dialogue_world.premises.Count; disposition=$a.disposition
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
if ($failed) { throw 'POLAR_REPLY_REGRESSION' }
