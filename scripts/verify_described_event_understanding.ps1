param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
# Diagnostic path matrix, not a blind benchmark or runtime sentence knowledge.
$cases=ConvertFrom-Json -InputObject @'
[
  {
    "id": "KO_TRANSFER",
    "lang": "KOREAN",
    "turns": [
      [
        "어제 민수는 지연에게 책을 빌려줬어."
      ],
      [
        "누가 책을 빌려줬어?",
        "민수"
      ],
      [
        "누구에게?",
        "지연"
      ],
      [
        "뭘?",
        "책"
      ],
      [
        "언제?",
        "어제"
      ]
    ]
  },
  {
    "id": "EN_TRANSFER",
    "lang": "ENGLISH",
    "turns": [
      [
        "Mina lent a book to Jin yesterday."
      ],
      [
        "Who lent a book to Jin?",
        "mina"
      ],
      [
        "To whom?",
        "jin"
      ],
      [
        "What?",
        "book"
      ],
      [
        "When?",
        "yesterday"
      ]
    ]
  },
  {
    "id": "KO_REORDER",
    "lang": "KOREAN",
    "turns": [
      [
        "우산을 도윤에게 오늘 하린이 빌려줬어."
      ],
      [
        "누가 우산을 빌려줬어?",
        "하린"
      ],
      [
        "누구에게?",
        "도윤"
      ],
      [
        "언제?",
        "오늘"
      ]
    ]
  },
  {
    "id": "KO_LOCATION",
    "lang": "KOREAN",
    "turns": [
      [
        "서아는 도서관에서 편지를 읽었어."
      ],
      [
        "누가 편지를 읽었어?",
        "서아"
      ],
      [
        "어디서?",
        "도서관"
      ],
      [
        "뭘?",
        "편지"
      ]
    ]
  },
  {
    "id": "EN_LOCATION",
    "lang": "ENGLISH",
    "turns": [
      [
        "Nora read a letter in the library."
      ],
      [
        "Who read a letter?",
        "nora"
      ],
      [
        "Where?",
        "library"
      ],
      [
        "What?",
        "letter"
      ]
    ]
  },
  {
    "id": "KO_DURATION",
    "lang": "KOREAN",
    "turns": [
      [
        "나는 어제 세 시간 잤어."
      ],
      [
        "얼마나 잤어?",
        "세 시간"
      ],
      [
        "언제?",
        "어제"
      ]
    ]
  },
  {
    "id": "EN_DURATION",
    "lang": "ENGLISH",
    "turns": [
      [
        "I slept for three hours yesterday."
      ],
      [
        "How long did I sleep?",
        "three hours"
      ],
      [
        "When?",
        "yesterday"
      ]
    ]
  },
  {
    "id": "KO_SOURCE",
    "lang": "KOREAN",
    "turns": [
      [
        "유나는 지호에게서 우산을 빌렸어."
      ],
      [
        "유나는 누구에게서 우산을 빌렸어?",
        "지호"
      ],
      [
        "뭘?",
        "우산"
      ]
    ]
  },
  {
    "id": "EN_SEND",
    "lang": "ENGLISH",
    "turns": [
      [
        "Aria sent a parcel to Theo today."
      ],
      [
        "To whom did Aria send a parcel?",
        "theo"
      ],
      [
        "What?",
        "parcel"
      ],
      [
        "When?",
        "today"
      ]
    ]
  },
  {
    "id": "KO_JOIN",
    "lang": "KOREAN",
    "turns": [
      [
        "민수는 지연에게 책을 빌려줬어."
      ],
      [
        "하린은 도윤에게 우산을 빌려줬어."
      ],
      [
        "누가 빌려줬어?",
        null
      ],
      [
        "민수는 누구에게 우산을 빌려줬어?",
        null
      ],
      [
        "하린은 누구에게 우산을 빌려줬어?",
        "도윤"
      ]
    ]
  },
  {
    "id": "KO_NEGATED",
    "lang": "KOREAN",
    "turns": [
      [
        "민수가 책을 안 읽었어."
      ],
      [
        "누가 책을 읽었어?",
        null
      ],
      [
        "누가 책을 안 읽었어?",
        "민수"
      ]
    ]
  },
  {
    "id": "EN_NEGATED",
    "lang": "ENGLISH",
    "turns": [
      [
        "Nora did not read a letter."
      ],
      [
        "Who read a letter?",
        null
      ],
      [
        "Who did not read a letter?",
        "nora"
      ]
    ]
  }
]
'@
$commands=[Collections.Generic.List[object]]::new()
$expectations=[Collections.Generic.List[object]]::new()
foreach($case in $cases) {
  for($i=0;$i -lt $case.turns.Count;$i++) {
    $turn=$case.turns[$i]
    $commands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
      schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id="EVENT-$($case.id)";turn_index=$i+1
      request_id="$($case.id)-$i";modality='TEXT';raw_text=$turn[0];input_confidence_millis=1000
      alternatives=@();output_language=$case.lang;context_tags=@();max_plan_steps=16
    }})
    $expectations.Add(@{query=($turn.Count -eq 2);value=$(if($turn.Count -eq 2){$turn[1]}else{$null})})
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
  $rows=for($i=0;$i -lt $responses.Count;$i++){
    $r=$responses[$i];$v=$r.payload.value;$id=$commands[$i].request.request_id;$expect=$expectations[$i]
    $projection=$v.discourse_answer.content_projection
    $value=$projection.binding.value
    $storedEvents=@($v.conversation_state.epistemic_ledger.records | ForEach-Object {$_.content.events} | Where-Object {$null -ne $_})
    $correct=if($expect.query){$value -ceq $expect.value}else{$storedEvents.Count -gt 0}
    $noAction=@($v.conversation_state.action_state_ledger.records).Count -eq 0 -and $null -eq $v.grounded_response -and -not $v.language_cortex_integration.external_action_executed
    $faithful=$null -eq $projection -or ($v.output.text.ToLowerInvariant().Contains($value.ToLowerInvariant()) -and -not $v.discourse_answer.dialogue_truth_established -and @($v.natural_realization.response_plan.moves).Count -eq 1)
    [pscustomobject]@{id=$id;input=$commands[$i].request.raw_text;output=$v.output.text
      query=$expect.query;expected=$expect.value;actual=$value;api_ok=$r.ok
      stored_events=$storedEvents.Count;role_correct=$correct;no_action_authority=$noAction
      source_faithful=$faithful;unsupported_claims=$v.output.unsupported_freeform_claims
      pass=($r.ok -and $correct -and $noAction -and $faithful -and $v.output.unsupported_freeform_claims -eq 0)
      selected_act=$v.natural_realization.response_act;disposition=$v.discourse_answer.disposition
    }
  }
  [pscustomobject]@{boundary_status=$(if(@($rows|Where-Object{-not $_.pass}).Count -eq 0){'PASS'}else{'FAIL'})
    general_understanding_status='NOT_ESTABLISHED';cases=$cases.Count;completed_turns=$rows.Count
    role_questions=@($rows|Where-Object{$_.query}).Count;passed=@($rows|Where-Object{$_.pass}).Count
    stderr=$stderr;executable_sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $start.FileName).Hash.ToLowerInvariant()
    rows=@($rows)
  } | ConvertTo-Json -Depth 10
} finally {
  if(-not $p.HasExited){$p.Kill()}
  $p.Dispose()
}

