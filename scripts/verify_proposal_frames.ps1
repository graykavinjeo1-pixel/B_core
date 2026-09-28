param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
# Developer path regression, not a blind language benchmark or a runtime corpus.
# Run the real executable: Rust test threads can conceal main-thread stack faults.
$cases = ConvertFrom-Json -InputObject @'
[
  {
    "id": "KO_ROLES",
    "lang": "KOREAN",
    "turns": [
      "윤서가 작업실에서 메모를 읽는 게 좋겠네?",
      "왜 그렇게 생각해?"
    ],
    "actor": "윤서",
    "theme": "메모",
    "location": "작업실",
    "negative": false
  },
  {
    "id": "KO_ORDER_NEGATION",
    "lang": "KOREAN",
    "turns": [
      "메모를 작업실에서 윤서가 안 읽는 게 좋겠네?",
      "왜 그렇게 생각해?"
    ],
    "actor": "윤서",
    "theme": "메모",
    "location": "작업실",
    "negative": true
  },
  {
    "id": "EN_CASE",
    "lang": "ENGLISH",
    "turns": [
      "Then Would it be good to read CedarNotes at SouthDesk?",
      "Why do you ask?"
    ],
    "actor": null,
    "theme": "CedarNotes",
    "location": "SouthDesk",
    "negative": false
  },
  {
    "id": "EN_NEGATION",
    "lang": "ENGLISH",
    "turns": [
      "Would it be good to not read AmberNotes in NorthHall?",
      "Why do you think that?"
    ],
    "actor": null,
    "theme": "AmberNotes",
    "location": "NorthHall",
    "negative": true
  },
  {
    "id": "REST_INTRANSITIVE",
    "lang": "KOREAN",
    "turns": [
      "쉬는 게 좋겠네?",
      "왜?"
    ],
    "actor": null,
    "theme": null,
    "location": null,
    "negative": false,
    "rest_incompatible": false
  },
  {
    "id": "REST_TRANSITIVE",
    "lang": "KOREAN",
    "turns": [
      "회사를 쉬는 게 좋겠네?",
      "왜?"
    ],
    "actor": null,
    "theme": "회사",
    "location": null,
    "negative": false,
    "rest_incompatible": true
  }
]
'@
$exe = (Resolve-Path -LiteralPath $Executable).Path
$rows = [Collections.Generic.List[object]]::new()
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
            $proposal = $a.decision_inquiry.proposed_action
            $roles = $proposal.event.roles
            $boundary = $null -ne $proposal -and
                $roles.AGENT -ceq $case.actor -and $roles.THEME -ceq $case.theme -and
                $roles.LOCATION -ceq $case.location -and $proposal.event.negated -eq $case.negative -and
                $null -eq $v.grounded_response -and -not $a.dialogue_truth_established -and
                $v.conversation_state.epistemic_ledger.records.Count -eq 0 -and
                $v.conversation_state.action_state_ledger.records.Count -eq 0
            foreach ($expected in @($case.actor, $case.theme, $case.location)) {
                if ($null -ne $expected) { $boundary = $boundary -and $v.output.text.Contains($expected) }
            }
            if ($null -ne $case.rest_incompatible) {
                $sense = @($proposal.frame_candidates | Where-Object { $_.entry_id -eq '71280' -and $_.sense_id -eq '1' })
                $boundary = $boundary -and $sense.Count -eq 1 -and $sense[0].pattern_understood -and
                    (($sense[0].incompatible_roles -contains 'THEME') -eq $case.rest_incompatible)
            }
            if ($i -gt 0) {
                $boundary = $boundary -and $a.decision_inquiry.explanation_of.asked_turn -eq 1 -and
                    ($proposal | ConvertTo-Json -Depth 30 -Compress) -ceq $previous
            }
            $previous = $proposal | ConvertTo-Json -Depth 30 -Compress
            $pass = $null -ne $v -and $r.ok -and $boundary -and
                -not $v.language_cortex_integration.external_action_executed -and
                $v.output.unsupported_freeform_claims -eq 0
            $rows.Add([pscustomobject]@{
                case=$case.id; turn=$i+1; input=$case.turns[$i]; output=$v.output.text
                tested_boundary='PROPOSAL_ROLE_AND_SENSE_FRAME'; disposition=$a.disposition
                proposal=$proposal
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
} | ConvertTo-Json -Depth 40
if ($failed) { throw 'PROPOSAL_FRAME_REGRESSION' }
