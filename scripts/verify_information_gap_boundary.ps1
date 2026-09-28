param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
# Developer path regression, not a blind language benchmark or a runtime corpus.
# Run the real executable: Rust test threads can conceal main-thread stack faults.
$cases = ConvertFrom-Json -InputObject @'
[
  {
    "id": "KNOWN_PLAN",
    "lang": "KOREAN",
    "turns": [
      "파일을 조사해.",
      "어떻게 조사하는데?"
    ],
    "check": "PLAN"
  },
  {
    "id": "THIRD_PARTY",
    "lang": "ENGLISH",
    "turns": [
      "Inspect the file.",
      "How does Mira inspect the file?"
    ],
    "check": "GAP"
  },
  {
    "id": "RETIRED_PLAN",
    "lang": "KOREAN",
    "turns": [
      "파일을 조사해.",
      "취소해.",
      "어떻게 조사해?"
    ],
    "check": "RETIRED"
  },
  {
    "id": "INNER_WITH_CONTEXT",
    "lang": "ENGLISH",
    "turns": [
      "Avel wrote a message yesterday.",
      "Tell me who wrote the map."
    ],
    "check": "OWNED",
    "target": "who wrote the map"
  },
  {
    "id": "INNER_FRESH",
    "lang": "ENGLISH",
    "turns": [
      "Tell me who wrote the map."
    ],
    "check": "OWNED",
    "target": "who wrote the map"
  },
  {
    "id": "NEW_NOMINAL",
    "lang": "KOREAN",
    "turns": [
      "비가 그쳐서 유란은 편지를 읽었어.",
      "그 공장 정전의 원인을 설명해줘."
    ],
    "check": "OWNED",
    "target": "공장 정전"
  },
  {
    "id": "FRESH_METHOD",
    "lang": "ENGLISH",
    "turns": [
      "How do you inspect the repository?"
    ],
    "check": "GAP"
  },
  {
    "id": "PAST",
    "lang": "ENGLISH",
    "turns": [
      "How did Mira inspect the file?"
    ],
    "check": "PREMISE"
  },
  {
    "id": "FACTIVE",
    "lang": "ENGLISH",
    "turns": [
      "How did Mira know that the file failed?"
    ],
    "check": "PREMISE"
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
            $last = $i -eq ($case.turns.Count - 1)
            $a = $v.discourse_answer
            $boundary = $true
            if ($last) {
                $noAction = $null -eq $v.grounded_response
                $gap = $a.disposition -eq 'NO_MATCHING_RECORD' -and $null -eq $a.plan_method
                switch ($case.check) {
                    'PLAN' { $boundary = $null -ne $a.plan_method -and $a.disposition -eq 'ANSWERED_FROM_DIALOGUE_RECORDS' }
                    'GAP' { $boundary = $gap -and $a.query.presuppositions.Count -eq 0 }
                    'RETIRED' { $boundary = $gap -and $a.query.presuppositions.Count -eq 0 -and $v.conversation_state.active_goals.Count -eq 0 }
                    'OWNED' { $boundary = $gap -and $v.output.text.Contains($case.target) }
                    'PREMISE' {
                        # Preserving the premise flag is insufficient if a later
                        # candidate replaces the answer with execution-receipt text.
                        $boundary = $a.query.presuppositions.Count -gt 0 -and
                            $null -eq $a.plan_method -and
                            $v.natural_realization.response_act -eq 'DISCOURSE_ANSWER'
                    }
                    default { throw 'UNDEFINED_BOUNDARY' }
                }
                $boundary = $boundary -and $noAction -and -not $a.dialogue_truth_established
            }
            $pass = $null -ne $v -and $r.ok -and $boundary -and
                -not $v.language_cortex_integration.external_action_executed -and
                $v.output.unsupported_freeform_claims -eq 0
            $rows.Add([pscustomobject]@{
                case=$case.id; turn=$i+1; input=$case.turns[$i]; output=$v.output.text
                tested_boundary=$(if($last){$case.check}else{'SETUP'}); disposition=$a.disposition
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
if ($failed) { throw 'INFORMATION_GAP_BOUNDARY_REGRESSION' }
