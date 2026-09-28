param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
# Developer path regression, not a blind language benchmark or a runtime corpus.
# Run the real executable: Rust test threads can conceal main-thread stack faults.
$cases = ConvertFrom-Json -InputObject @'
[
  {
    "id": "RESUME_SELF",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I am tired.",
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
    "group": "RESUME_SELF"
  },
  {
    "id": "RESUME_EXPLICIT",
    "lang": "KOREAN",
    "turns": [
      "도윤이 쉬는 게 좋겠네?",
      "도윤은 피곤해.",
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
    "group": "RESUME_EXPLICIT"
  },
  {
    "id": "RESUME_NEGATIVE",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "나는 피곤하지 않아.",
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
    "group": "RESUME_NEGATIVE"
  },
  {
    "id": "OTHER_ACTOR",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "하윤은 피곤해."
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "group": "OTHER_ACTOR"
  },
  {
    "id": "OTHER_PROPERTY",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "I am ready."
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "group": "OTHER_PROPERTY"
  },
  {
    "id": "CANCELLATION",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Never mind.",
      "I am tired."
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
    "group": "CANCELLATION"
  },
  {
    "id": "NEW_TOPIC",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "What is entropy?",
      "I am tired."
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
    "group": "NEW_TOPIC"
  },
  {
    "id": "SOCIAL_BRIDGE",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Thanks.",
      "I am tired.",
      "Why do you think that?"
    ],
    "assess": [
      false,
      false,
      true,
      true
    ],
    "resumes": [
      false,
      false,
      true,
      true
    ],
    "group": "SOCIAL_BRIDGE"
  },
  {
    "id": "EXPLANATION_BRIDGE",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Why do you think that?",
      "I am tired.",
      "Why do you think that?"
    ],
    "assess": [
      false,
      false,
      true,
      true
    ],
    "resumes": [
      false,
      false,
      true,
      true
    ],
    "group": "EXPLANATION_BRIDGE"
  },
  {
    "id": "STALE",
    "lang": "ENGLISH",
    "turns": [
      "Should I rest?",
      "Thanks.",
      "Thanks.",
      "Thanks.",
      "I am tired."
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
    "group": "STALE"
  },
  {
    "id": "UNKNOWN_EFFECT",
    "lang": "ENGLISH",
    "turns": [
      "Should I sleep?",
      "I am tired."
    ],
    "assess": [
      false,
      false
    ],
    "resumes": [
      false,
      false
    ],
    "group": "UNKNOWN_EFFECT"
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
                tested_boundary='SEMANTIC_QUESTION_RESUMPTION'; disposition=$a.disposition
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
if ($failed) { throw 'DECISION_RESUMPTION_REGRESSION' }
