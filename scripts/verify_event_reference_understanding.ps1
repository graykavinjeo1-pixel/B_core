param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
# Developer path diagnostic; not blind evaluation or a runtime sentence corpus.
$cases=ConvertFrom-Json -InputObject @'
[
  {
    "id": "KO_CHAIN",
    "lang": "KOREAN",
    "turns": [
      [
        "민수는 지연에게 책을 빌려줬어."
      ],
      [
        "하린은 도서관에서 책을 읽었어."
      ],
      [
        "누가 책을 빌려줬어?",
        "민수"
      ],
      [
        "그 사람은 누구에게 책을 빌려줬어?",
        "지연"
      ],
      [
        "누가 그것을 읽었어?",
        "하린"
      ],
      [
        "어디서?",
        "도서관"
      ]
    ]
  },
  {
    "id": "EN_CHAIN",
    "lang": "ENGLISH",
    "turns": [
      [
        "Mina lent a book to Jin."
      ],
      [
        "Nora read a book in the library."
      ],
      [
        "Who lent a book?",
        "mina"
      ],
      [
        "To whom did that person lend it?",
        "jin"
      ],
      [
        "Who read it?",
        "nora"
      ],
      [
        "Where?",
        "library"
      ]
    ]
  },
  {
    "id": "KO_NAMED",
    "lang": "KOREAN",
    "turns": [
      [
        "윤슬은 서점에서 잡지를 읽었어."
      ],
      [
        "누가 그 잡지를 읽었어?",
        "윤슬"
      ],
      [
        "누가 그곳에서 잡지를 읽었어?",
        "윤슬"
      ]
    ]
  },
  {
    "id": "EN_NAMED",
    "lang": "ENGLISH",
    "turns": [
      [
        "Aria read a magazine in the shop."
      ],
      [
        "Who read that magazine?",
        "aria"
      ],
      [
        "Who read a magazine in that place?",
        "aria"
      ]
    ]
  },
  {
    "id": "KO_AMBIGUOUS",
    "lang": "KOREAN",
    "turns": [
      [
        "유나는 지호에게 우산을 빌려줬어."
      ],
      [
        "그 사람은 누구에게 우산을 빌려줬어?",
        null
      ]
    ]
  },
  {
    "id": "EN_AMBIGUOUS",
    "lang": "ENGLISH",
    "turns": [
      [
        "Aria lent an umbrella to Theo."
      ],
      [
        "To whom did that person lend an umbrella?",
        null
      ]
    ]
  },
  {
    "id": "KO_MISMATCH",
    "lang": "KOREAN",
    "turns": [
      [
        "민수는 책을 빌려줬어."
      ],
      [
        "하린은 잡지를 읽었어."
      ],
      [
        "누가 책을 빌려줬어?",
        "민수"
      ],
      [
        "누가 그것을 읽었어?",
        null
      ]
    ]
  },
  {
    "id": "EN_MISMATCH",
    "lang": "ENGLISH",
    "turns": [
      [
        "Mina lent a book to Jin."
      ],
      [
        "Nora read a magazine."
      ],
      [
        "Who lent a book?",
        "mina"
      ],
      [
        "Who read it?",
        null
      ]
    ]
  },
  {
    "id": "KO_EXPLICIT",
    "lang": "KOREAN",
    "turns": [
      [
        "민수는 책을 읽었어."
      ],
      [
        "하린은 편지를 읽었어."
      ],
      [
        "누가 책을 읽었어?",
        "민수"
      ],
      [
        "누가 편지를 읽었어?",
        "하린"
      ],
      [
        "누가 그것을 읽었어?",
        "하린"
      ]
    ]
  },
  {
    "id": "EN_EXPLICIT",
    "lang": "ENGLISH",
    "turns": [
      [
        "Mina read a book."
      ],
      [
        "Nora read a letter."
      ],
      [
        "Who read a book?",
        "mina"
      ],
      [
        "Who read a letter?",
        "nora"
      ],
      [
        "Who read it?",
        "nora"
      ]
    ]
  },
  {
    "id": "KO_NO_PRIOR",
    "lang": "KOREAN",
    "turns": [
      [
        "누가 그것을 읽었어?",
        null
      ]
    ]
  },
  {
    "id": "EN_NO_PRIOR",
    "lang": "ENGLISH",
    "turns": [
      [
        "Who read it?",
        null
      ]
    ]
  },
  {
    "id": "ACTIVE_TO_PASSIVE",
    "lang": "ENGLISH",
    "turns": [
      ["Mina repaired a file in the library."],
      ["Where was the file repaired?", "library"],
      ["Who was the file repaired by?", "mina"]
    ]
  },
  {
    "id": "PASSIVE_TO_ACTIVE",
    "lang": "ENGLISH",
    "turns": [
      ["The gate was cleaned by Lior in the workshop."],
      ["Who cleaned the gate?", "lior"],
      ["Where did Lior clean the gate?", "workshop"]
    ]
  },
  {
    "id": "IRREGULAR_VOICE",
    "lang": "ENGLISH",
    "turns": [
      ["Nara wrote a letter in the park."],
      ["Where was the letter written?", "park"]
    ]
  },
  {
    "id": "NEGATED_VOICE",
    "lang": "ENGLISH",
    "turns": [
      ["Mina did not repair a file in the library."],
      ["Where was the file not repaired?", "library"],
      ["Where was the file repaired?", null]
    ]
  },
  {
    "id": "STRESS_METADATA_UNSTRESSED",
    "lang": "ENGLISH",
    "turns": [
      ["The window was opened by Sora in the studio."],
      ["Who opened the window?", "sora"],
      ["Where was the window opened?", "studio"]
    ]
  },
  {
    "id": "STRESS_METADATA_STRESSED",
    "lang": "ENGLISH",
    "turns": [
      ["Rhea admitted a visitor in the lobby."],
      ["Who admitted the visitor?", "rhea"],
      ["Where was the visitor admitted?", "lobby"]
    ]
  },
  {
    "id": "PERSONAL_REFERENCE_PASSIVE",
    "lang": "ENGLISH",
    "turns": [
      ["The window was opened by Sora in the studio."],
      ["Who opened the window?", "sora"],
      ["Where did she open the window?", "studio"]
    ]
  },
  {
    "id": "PERSONAL_REFERENCE_PP_ORDER",
    "lang": "ENGLISH",
    "turns": [
      ["The file was repaired in the library by Mina."],
      ["Who repaired the file?", "mina"],
      ["Where did he repair the file?", "library"]
    ]
  },
  {
    "id": "PERSONAL_REFERENCE_DESCRIPTIVE",
    "lang": "ENGLISH",
    "turns": [
      ["Lior cleaned a gate in the workshop."],
      ["Who cleaned the gate?", "lior"],
      ["Where did she clean the gate?", "workshop"]
    ]
  },
  {
    "id": "PERSONAL_REFERENCE_AMBIGUOUS",
    "lang": "ENGLISH",
    "turns": [
      ["Mina lent a book to Jin in the library."],
      ["Where did she lend a book?", null]
    ]
  },
  {
    "id": "PERSONAL_REFERENCE_KOREAN",
    "lang": "KOREAN",
    "turns": [
      ["하린은 도서관에서 책을 읽었어."],
      ["누가 책을 읽었어?", "하린"],
      ["그녀는 어디서 책을 읽었어?", "도서관"]
    ]
  },
  {
    "id": "RETAIN_AFTER_UNKNOWN",
    "lang": "ENGLISH",
    "turns": [
      ["Sora opened a window in the studio."],
      ["Who repaired a file?", null],
      ["Where did she open the window?", "studio"]
    ]
  },
  {
    "id": "RETAIN_AFTER_SOCIAL",
    "lang": "ENGLISH",
    "turns": [
      ["Nora wrote a letter in the garden."],
      ["Thanks!"],
      ["Where did she write the letter?", "garden"]
    ]
  },
  {
    "id": "RETAIN_AFTER_UNANSWERED_FOCUS",
    "lang": "ENGLISH",
    "turns": [
      ["Sora opened a window in the studio."],
      ["Who opened the window?", "sora"],
      ["Who repaired a file?", null],
      ["Where did she open the window?", "studio"]
    ]
  },
  {
    "id": "RETAIN_AFTER_UNKNOWN_KO",
    "lang": "KOREAN",
    "turns": [
      ["하린은 도서관에서 책을 읽었어."],
      ["누가 파일을 수리했어?", null],
      ["그녀는 어디서 책을 읽었어?", "도서관"]
    ]
  },
  {
    "id": "RETAIN_TOPIC_BOUNDARY",
    "lang": "ENGLISH",
    "turns": [
      ["Sora opened a window in the studio."],
      ["Switch to the database topic."],
      ["Where did she open the window?", null]
    ]
  },
  {
    "id": "RETAIN_TOPIC_OVER_ANSWER",
    "lang": "ENGLISH",
    "turns": [
      ["Sora opened a window in the studio."],
      ["Who opened the window?", "sora"],
      ["Switch to the database topic."],
      ["Where did she open the window?", null]
    ]
  },
  {
    "id": "RETAIN_NEW_OBSERVATION_BOUNDARY",
    "lang": "ENGLISH",
    "turns": [
      ["Sora opened a window in the studio."],
      ["Nara wrote a letter in the park."],
      ["Where did she open the window?", null]
    ]
  },
  {
    "id": "SOURCE_ASSERTION_PASSIVE",
    "lang": "ENGLISH",
    "turns": [
      ["The report was printed at the office by Keira."],
      ["Who printed the report?", "keira"],
      ["Where did she print the report?", "office"]
    ]
  },
  {
    "id": "SOURCE_ASSERTION_ACTIVE",
    "lang": "ENGLISH",
    "turns": [
      ["Rhea printed a report at the office."],
      ["Who printed the report?", "rhea"]
    ]
  },
  {
    "id": "SOURCE_ASSERTION_NEGATED",
    "lang": "ENGLISH",
    "turns": [
      ["The report was not printed by Keira at the office."],
      ["Who printed the report?", null],
      ["Who was the report not printed by?", "keira"]
    ]
  }
]
'@
$commands=[Collections.Generic.List[object]]::new()
$expectations=[Collections.Generic.List[object]]::new()
foreach($case in $cases){for($i=0;$i -lt $case.turns.Count;$i++){
 $turn=$case.turns[$i]
 $commands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
 schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id="REF-$($case.id)";turn_index=$i+1
 request_id="$($case.id)-$i";modality='TEXT';raw_text=$turn[0];input_confidence_millis=1000
 alternatives=@();output_language=$case.lang;context_tags=@();max_plan_steps=16}})
 $expectations.Add(@{query=($turn.Count -eq 2);value=$(if($turn.Count -eq 2){$turn[1]}else{$null})})
}}
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
try {
 foreach($command in $commands){$p.StandardInput.WriteLine(($command|ConvertTo-Json -Depth 12 -Compress))}
 $p.StandardInput.Close()
 if(-not $p.WaitForExit(30000)){$p.Kill();throw 'CLI_TIMEOUT'}
 $stdout=$outTask.GetAwaiter().GetResult()
 $stderr=$errTask.GetAwaiter().GetResult()
 if($p.ExitCode -ne 0){throw "CLI_EXIT_$($p.ExitCode): $stderr"}
 $responses=@($stdout -split '\r?\n'|Where-Object{$_.Trim()}|ForEach-Object{$_|ConvertFrom-Json -Depth 100})
 if($responses.Count -ne $commands.Count){throw 'MISSING_RESPONSES'}
 $rows=for($i=0;$i -lt $responses.Count;$i++){
  $r=$responses[$i];$v=$r.payload.value;$expect=$expectations[$i];$projection=$v.discourse_answer.content_projection
  $value=$projection.binding.value
  # Entity matching is case-insensitive; original source spelling is now kept
  # for realization. Separate Rust tests enforce that spelling is not invented.
  $correct=-not $expect.query -or [string]::Equals($value,$expect.value,[StringComparison]::OrdinalIgnoreCase)
  $noAction=$null -eq $v.grounded_response -and -not $v.language_cortex_integration.external_action_executed -and @($v.conversation_state.action_state_ledger.records).Count -eq 0
  $faithful=$null -eq $projection -or ($v.output.text.ToLowerInvariant().Contains($value.ToLowerInvariant()) -and -not $v.discourse_answer.dialogue_truth_established)
  [pscustomobject]@{id=$commands[$i].request.request_id;input=$commands[$i].request.raw_text
   output=$v.output.text;query=$expect.query;expected=$expect.value;actual=$value;api_ok=$r.ok
   reference_context=$projection.reference_context;reference_bindings=@($projection.reference_bindings)
   selected_act=$v.natural_realization.response_act;disposition=$v.discourse_answer.disposition
   no_action=$noAction;source_faithful=$faithful;unsupported_claims=$v.output.unsupported_freeform_claims
   pass=($r.ok -and $correct -and $noAction -and $faithful -and $v.output.unsupported_freeform_claims -eq 0)}
 }
 [pscustomobject]@{status=$(if(@($rows|Where-Object{-not $_.pass}).Count){'FAIL'}else{'PASS'})
  cases=$cases.Count;turns=$rows.Count;questions=@($rows|Where-Object{$_.query}).Count
  passed_questions=@($rows|Where-Object{$_.query -and $_.pass}).Count
  executable_sha256=(Get-FileHash -LiteralPath $start.FileName).Hash.ToLowerInvariant()
  naturalness_status='NOT_ESTABLISHED';rows=@($rows);stderr=$stderr}|ConvertTo-Json -Depth 15
}finally{if(-not $p.HasExited){$p.Kill()};$p.Dispose()}
