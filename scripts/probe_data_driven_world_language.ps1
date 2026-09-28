param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference='Stop'
$commands=[Collections.Generic.List[object]]::new()
$conversation='DATA-ONLY-LANGUAGE-PROBE'
$commands.Add(@{operation='UPDATE_WORLD_VOCABULARY';conversation_id=$conversation;update=@{
    predicates=@(@{predicate_id='W_USER_78143';arity='UNARY'});remove_alias_ids=@()
    aliases=@(
        @{alias_id='meticulous.ko';predicate_id='W_USER_78143';language='KOREAN';root='꼼꼼';grammar='KOREAN_HADA_STATE'},
        @{alias_id='meticulous.en';predicate_id='W_USER_78143';language='ENGLISH';root='meticulous';grammar='COPULAR'}
    )
}})
$turn=0
foreach($item in @(
    @('지안은 꼼꼼하다.','KOREAN'),
    @('Is 지안 meticulous?','ENGLISH'),
    @('아니, 지안은 꼼꼼하지 않아.','KOREAN'),
    @('Is 지안 meticulous?','ENGLISH')
)) {
    $turn++
    $commands.Add(@{operation='PROCESS_CONVERSATION_TURN';request=@{
        schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$conversation;turn_index=$turn
        request_id="DATA-$turn";modality='TEXT';raw_text=$item[0];output_language=$item[1]
        input_confidence_millis=1000;alternatives=@();context_tags=@();max_plan_steps=16
    }})
}
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
$stderr=$p.StandardError.ReadToEndAsync()
$rows=[Collections.Generic.List[object]]::new()
try {
    foreach($command in $commands) {
        $timer=[Diagnostics.Stopwatch]::StartNew()
        $p.StandardInput.WriteLine(($command|ConvertTo-Json -Depth 15 -Compress))
        $line=$p.StandardOutput.ReadLineAsync()
        if(-not $line.Wait(30000)){throw 'CLI_TIMEOUT'}
        $timer.Stop()
        $r=$line.Result | ConvertFrom-Json -Depth 100
        $v=$r.payload.value
        $rows.Add([pscustomobject]@{
            operation=$command.operation;input=$command.request.raw_text;api_ok=$r.ok;error=$r.error
            output=$v.output.text;elapsed_ms=$timer.Elapsed.TotalMilliseconds
            verdict=$v.discourse_answer.world_reasoning.decision.verdict
            target=$v.discourse_answer.world_reasoning.query.target
            remembered=@($v.conversation_state.dialogue_world.premises|Select-Object atom,value)
            plan=($null -ne $v.grounded_response)
        })
    }
    $p.StandardInput.Close()
    if(-not $p.WaitForExit(30000)){throw 'CLI_EXIT_TIMEOUT'}
    [pscustomobject]@{
        schema='DATA_ONLY_WORLD_LANGUAGE_PROBE_1'
        scope='Supplied unary predicate and bilingual aliases; not autonomous learning or general natural conversation.'
        timing='Single-process sequential request round trips including serialization and response transmission; not a comparative latency benchmark.'
        executable_sha256=(Get-FileHash -LiteralPath $Executable).Hash.ToLowerInvariant()
        exit_code=$p.ExitCode;stderr=$stderr.GetAwaiter().GetResult();rows=$rows
    } | ConvertTo-Json -Depth 25 -Compress
} finally {
    if(-not $p.HasExited){$p.Kill()}
    $p.Dispose()
}
