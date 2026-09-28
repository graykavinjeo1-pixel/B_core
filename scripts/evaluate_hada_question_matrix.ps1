#requires -Version 7.0
param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/debug/b-core-cognitive-api.exe'),
    [string]$Manifest = (Join-Path $PSScriptRoot '../reports/language-cortex-completion/hada-question-2026-09-09/manifest.json')
)
# Supervisor-only inputs and checks. Runtime never loads this matrix.
$ErrorActionPreference = 'Stop'
$suite = Get-Content -LiteralPath $Manifest -Raw | ConvertFrom-Json -Depth 30
if ($suite.cases.Count -ne 106 -or $suite.counts.queries -ne 88 -or $suite.counts.controls -ne 18) { throw 'MATRIX_COUNT_MISMATCH' }
$exe = (Resolve-Path -LiteralPath $Executable).Path
$rows = [Collections.Generic.List[object]]::new()
$apiErrors = [Collections.Generic.List[string]]::new()
$elapsed = [Collections.Generic.List[double]]::new()
$peak = 0L
$start = [Diagnostics.ProcessStartInfo]::new($exe)
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
$p = [Diagnostics.Process]::Start($start)
$stderr = $p.StandardError.ReadToEndAsync()
function Send($command) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $p.StandardInput.WriteLine(($command | ConvertTo-Json -Depth 30 -Compress))
    $p.StandardInput.Flush()
    $pending = $p.StandardOutput.ReadLineAsync()
    if (-not $pending.Wait(30000)) { throw 'NATIVE_TIMEOUT' }
    if ($null -eq $pending.Result) { throw 'NATIVE_EOF' }
    $watch.Stop()
    $elapsed.Add($watch.Elapsed.TotalMilliseconds)
    $response = $pending.Result | ConvertFrom-Json -Depth 100
    if (-not $response.ok) { $apiErrors.Add(($response.error | ConvertTo-Json -Compress)) }
    $p.Refresh()
    $script:peak = [Math]::Max($script:peak, $p.PeakWorkingSet64)
    return $response
}
function Turn($case, [int]$turn, [string]$text) {
    Send @{operation='PROCESS_CONVERSATION_TURN';request=@{
        schema='B_CORE_CONVERSATION_TURN_REQUEST_1';conversation_id=$case.id;turn_index=$turn;request_id="$($case.id)-$turn"
        modality='TEXT';raw_text=$text;input_confidence_millis=1000;alternatives=@();output_language=$case.language
        context_tags=@();max_plan_steps=16
    }}
}
try {
    foreach ($case in $suite.cases) {
        $fail = [Collections.Generic.List[string]]::new()
        $registration = Send @{operation='UPDATE_WORLD_VOCABULARY';conversation_id=$case.id;update=$suite.update}
        if (-not $registration.ok) { $fail.Add('REGISTRATION') }
        $premise = Turn $case 1 $case.premise
        $response = Turn $case 2 $case.query
        if (-not $premise.ok -or -not $response.ok) { $fail.Add('API') }
        $before = $premise.payload.value.conversation_state.dialogue_world
        $value = $response.payload.value
        $world = $value.discourse_answer.world_reasoning
        $target = if ($null -ne $world) { $world.query.target[0] } else { $null }
        $targetValue = if ($null -ne $world) { $world.query.target[1] } else { $null }
        if (@($before.premises).Count -ne 1) { $fail.Add('SETUP_PREMISE_NOT_GROUNDED') }
        $beforePremises = @($before.premises) | ConvertTo-Json -Depth 30 -Compress
        $afterPremises = @($value.conversation_state.dialogue_world.premises) | ConvertTo-Json -Depth 30 -Compress
        if ($beforePremises -ne $afterPremises) { $fail.Add('QUESTION_OR_CONTROL_MUTATED_WORLD_FACTS') }
        if (@($value.conversation_state.action_state_ledger.records).Count -gt 0 -or $null -ne $value.grounded_response) { $fail.Add('EXECUTION_OR_PLAN_LEAK') }
        if ($case.expected.kind -eq 'QUERY') {
            if ($null -eq $world) { $fail.Add('NO_TYPED_WORLD_QUERY') }
            else {
                if ($world.decision.verdict -ne $case.expected.verdict) { $fail.Add('VERDICT') }
                if ($target.property.REGISTERED -ne $case.expected.predicate_id) { $fail.Add('PREDICATE_ID') }
                if ($target.entity -ne $case.expected.entity -or $null -ne $target.object) { $fail.Add('ROLE') }
                if ($targetValue -ne $case.expected.value) { $fail.Add('NEGATION_SCOPE') }
                if (@($world.decision.premise_evidence_ids).Count -eq 0) { $fail.Add('NO_PREMISE_EVIDENCE') }
                if ($world.decision.hypothetical) { $fail.Add('UNREQUESTED_HYPOTHETICAL') }
            }
        } elseif ($null -ne $world) { $fail.Add('CONTROL_BECAME_WORLD_QUERY') }
        $rows.Add([ordered]@{id=$case.id;family=$case.family;input=$case.query;premise=$case.premise;expected=$case.expected
            passed=($fail.Count -eq 0);failures=@($fail);api_ok=[bool]$response.ok;error=$response.error;output=$value.output.text
            target=$target;target_value=$targetValue;verdict=$world.decision.verdict;response_act=$value.natural_realization.response_act
            premise_evidence_ids=@($world.decision.premise_evidence_ids)})
    }
} finally {
    $p.StandardInput.Close()
    if (-not $p.WaitForExit(30000)) { $p.Kill(); $p.WaitForExit(30000) | Out-Null }
    $errors = $stderr.GetAwaiter().GetResult()
    if (-not [string]::IsNullOrWhiteSpace($errors)) { $apiErrors.Add($errors) }
    $p.Dispose()
}
$times = @($elapsed | Sort-Object)
$passed = @($rows | Where-Object passed).Count
[ordered]@{schema='B_CORE_HADA_QUESTION_EVALUATION_1';classification=$suite.classification
    status=if($passed -eq 106 -and $apiErrors.Count -eq 0){'PASS'}else{'FAIL'}
    executable_sha256=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    manifest_sha256=(Get-FileHash -LiteralPath $Manifest -Algorithm SHA256).Hash
    cases=$rows.Count;passed=$passed;failed=$rows.Count-$passed;turns=$rows.Count*2;api_errors=@($apiErrors)
    p50_ms=$times[[Math]::Ceiling($times.Count*0.5)-1];p95_ms=$times[[Math]::Ceiling($times.Count*0.95)-1]
    timing_scope='Native JSON round trips including registration and first initialization; excludes host JSON parsing'
    peak_working_set_bytes=$peak;rows=@($rows)
} | ConvertTo-Json -Depth 40 -Compress
if ($passed -ne 106 -or $apiErrors.Count -ne 0) { exit 1 }
