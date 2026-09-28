#requires -Version 7.0
param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
# Supervisor diagnostic inputs only; production must not read this file.
$ErrorActionPreference='Stop'
$cases=@(
 @{id='EN_SAME';lang='ENGLISH';turns=@('Tovin read a book. Tovin gave it to Rina.','What did Tovin give to Rina?')},
 @{id='EN_SPLIT';lang='ENGLISH';turns=@('Tovin read a book.','Tovin gave it to Rina.','What did Tovin give to Rina?')},
 @{id='KO_SAME';lang='KOREAN';turns=@('도윤은 책을 읽었어. 도윤은 그것을 민서에게 주었어.','도윤은 민서에게 무엇을 주었어?')},
 @{id='EN_ABSENT';lang='ENGLISH';turns=@('Tovin gave it to Rina.','What did Tovin give to Rina?')},
 @{id='EN_AMBIGUOUS';lang='ENGLISH';turns=@('Tovin read a book. Neri read a report. Tovin gave it to Rina.','What did Tovin give to Rina?')},
@{id='KO_TRANSFER';lang='KOREAN';turns=@('서하는 편지를 읽었어. 서하는 그것을 지안에게 주었어.','서하는 지안에게 무엇을 주었어?')},
 @{id='EN_TRANSFER';lang='ENGLISH';turns=@('Arlen read a letter. Mevi gave it to Jora.','What did Mevi give to Jora?')},
 @{id='EN_NEGATIVE';lang='ENGLISH';turns=@('Arlen read a letter. Mevi did not give it to Jora.','What did Mevi give to Jora?')},
 @{id='EN_CONDITIONAL';lang='ENGLISH';turns=@('If Arlen read a letter, Mevi gave it to Jora.','What did Mevi give to Jora?')},
 @{id='EN_QUOTED';lang='ENGLISH';turns=@('Arlen said "Mevi read a letter." Jora gave it to Kavi.','What did Jora give to Kavi?')},
 @{id='KO_AMBIGUOUS';lang='KOREAN';turns=@('서하는 책을 읽었어. 지안은 편지를 읽었어. 서하는 그것을 민서에게 주었어.','서하는 민서에게 무엇을 주었어?')}
)
$rows=[Collections.Generic.List[object]]::new()
foreach($case in $cases) {
 $p=$null
 try {
  $start=[Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $Executable).Path)
  $start.UseShellExecute=$false
  $start.CreateNoWindow=$true
  $start.RedirectStandardInput=$true
  $start.RedirectStandardOutput=$true
  $start.RedirectStandardError=$true
  $start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
  $p=[Diagnostics.Process]::Start($start)
  $stderr=$p.StandardError.ReadToEndAsync()
  for($i=0;$i -lt $case.turns.Count;$i++){
   $command=@{operation='PROCESS_CONVERSATION_TURN';request=@{schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$case.id;turn_index=$i+1;request_id="$($case.id)-$i";modality='TEXT';raw_text=$case.turns[$i];input_confidence_millis=1000;alternatives=@();output_language=$case.lang;context_tags=@();max_plan_steps=16}}
   $json=$command | ConvertTo-Json -Depth 12 -Compress
   $json=[regex]::Replace($json,'[^\x00-\x7F]',{param($m) '\u{0:X4}' -f [int][char]$m.Value})
   $p.StandardInput.WriteLine($json)
   $p.StandardInput.Flush()
   $read=$p.StandardOutput.ReadLineAsync()
   if(-not $read.Wait(10000)){throw 'TIMEOUT'}
   $response=$read.Result | ConvertFrom-Json
   $v=$response.payload.value
   $rows.Add(@{case=$case.id;turn=$i+1;input=$case.turns[$i];ok=$response.ok;error=$response.error;output=$v.output.text;query=$v.discourse_answer.query;projection=$v.discourse_answer.content_projection;beliefs=@($v.conversation_state.epistemic_ledger.records | ForEach-Object { @{source=$_.proposition_surface;actor=$_.source_actor;world=$_.signature.modal_world;status=$_.status;events=$_.content.events;context=$_.content.context_sources} })})
  }
 } finally {
  if($p -and -not $p.HasExited){$p.StandardInput.Close();if(-not $p.WaitForExit(2000)){$p.Kill();$p.WaitForExit(2000)|Out-Null}}
  if($p){$p.Dispose()}
 }
}
[ordered]@{schema='B_CORE_MEANING_MEMORY_SUPERVISOR_PROBE_1'; executable_sha256=(Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash; classification='DIAGNOSTIC_NOT_BLIND'; rows=$rows} | ConvertTo-Json -Depth 30
if (@($rows | Where-Object { -not $_.ok }).Count -gt 0) { exit 1 }
