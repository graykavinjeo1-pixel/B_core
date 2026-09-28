param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
$cases = @(
    @{ id='KO_ADVICE'; lang='KOREAN'; turns=@('오늘은 좀 지쳤어.','일이 많았거든.','너라면 어떻게 하겠어?') },
    @{ id='EN_ADVICE'; lang='ENGLISH'; turns=@('I am tired today.','There was a lot of work.','If you were me, how would you handle it?') },
    @{ id='EN_ELLIPSIS'; lang='ENGLISH'; turns=@('If it happened, how would you respond?') },
    @{ id='EN_NEGATIVE'; lang='ENGLISH'; turns=@('If it did not happen, how would you respond?') },
    @{ id='EN_FACTUAL'; lang='ENGLISH'; turns=@('How did Mira inspect the file?') },
    @{ id='EN_FRONTED_CONDITION'; lang='ENGLISH'; turns=@('If the server failed, how did Mira inspect the file?') },
    @{ id='EN_FACTUAL_WITH_EMBEDDED_CONDITION'; lang='ENGLISH'; turns=@('How did Mira know that if the server failed, the file was inspected?') },
    @{ id='EN_FACTUAL_WITH_QUOTED_CONDITION'; lang='ENGLISH'; turns=@('How did Mira explain "if the server failed, how would we recover?"?') },
    @{ id='EN_QUOTED_HYPOTHETICAL'; lang='ENGLISH'; turns=@('Mira quoted the question "If the server failed, how did she inspect the file?".') }
)
$exe = (Resolve-Path -LiteralPath $Executable).Path
$rows = [Collections.Generic.List[object]]::new()
foreach ($case in $cases) {
    $start = [Diagnostics.ProcessStartInfo]::new($exe)
    $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $p = [Diagnostics.Process]::Start($start)
    try {
        $stderrText = ''
        $stderr = $p.StandardError.ReadToEndAsync()
        for ($i=0; $i -lt $case.turns.Count; $i++) {
            $command = @{ operation='PROCESS_CONVERSATION_TURN'; request=@{
                schema='B_CORE_CONVERSATION_TURN_REQUEST_1'; conversation_id=$case.id; turn_index=$i+1
                request_id="$($case.id)-$i"; modality='TEXT'; raw_text=$case.turns[$i]
                input_confidence_millis=1000; alternatives=@(); output_language=$case.lang; context_tags=@(); max_plan_steps=16
            }}
            $json = $command | ConvertTo-Json -Depth 12 -Compress
            # Windows PowerShell's ProcessStartInfo has no UTF-8 stream encoding;
            # escape non-ASCII input so the native JSON parser reconstructs it.
            $json = [regex]::Replace($json, '[^\x00-\x7F]', { param($m) '\u{0:X4}' -f [int][char]$m.Value })
            $p.StandardInput.WriteLine($json); $p.StandardInput.Flush()
            $read = $p.StandardOutput.ReadLineAsync()
            if (-not $read.Wait(10000)) { throw "NATIVE_TIMEOUT:$($case.id):$($i+1)" }
            $line = $read.Result
            if ($null -eq $line) { throw "NATIVE_EXIT:$($case.id):${i}:$($p.ExitCode)" }
            $response = $line | ConvertFrom-Json
            if (-not $response.ok) {
                $rows.Add([ordered]@{case=$case.id; turn=$i+1; input=$case.turns[$i]; ok=$false; error=$response.error})
                break
            }
            $v = $response.payload.value
            if ($i -eq ($case.turns.Count - 1)) {
                $a = $v.discourse_answer
                $presuppositions = if ($null -eq $a.query) { 0 } else { @($a.query.presuppositions).Count }
                $rows.Add([ordered]@{case=$case.id; turn=$i+1; input=$case.turns[$i]; ok=$true; output=$v.output.text
                    query_kind=if ($null -eq $a.query) { $null } else { $a.query.kind }; presuppositions=$presuppositions; disposition=$a.disposition
                    response_act=$v.natural_realization.response_act; value_keys=@($v.psobject.Properties.Name) })
            }
        }
    } catch {
        $rows.Add([ordered]@{case=$case.id; ok=$false; error=$_.Exception.Message})
    } finally {
        try { if ($p -and -not $p.HasExited) { $p.StandardInput.Close(); if (-not $p.WaitForExit(2000)) { $p.Kill(); $p.WaitForExit(2000) } } } catch {}
        try { if ($stderr -and $stderr.Wait(2000)) { $stderrText = $stderr.Result } } catch {}
        if ($stderrText) { [Console]::Error.WriteLine("NATIVE_STDERR:$($case.id)`n$stderrText") }
        try { if ($p) { $p.Dispose() } } catch {}
    }
}
$expectations = @{
    KO_ADVICE = 'MODAL_STATUS'; EN_ADVICE = 'MODAL_STATUS'; EN_ELLIPSIS = 'MODAL_STATUS'; EN_NEGATIVE = 'MODAL_STATUS';
    EN_FRONTED_CONDITION = 'MODAL_STATUS'; EN_FACTUAL = 'PRESUPPOSITION_CHECK';
    EN_FACTUAL_WITH_EMBEDDED_CONDITION = 'PRESUPPOSITION_CHECK'; EN_FACTUAL_WITH_QUOTED_CONDITION = 'PRESUPPOSITION_CHECK'; EN_QUOTED_HYPOTHETICAL = $null
}
$failures = @($rows | Where-Object { -not $_.ok })
foreach ($row in $rows) {
    if ($row.ok -and $expectations.ContainsKey($row.case) -and $row.query_kind -ne $expectations[$row.case]) {
        $failures += "QUERY_KIND:$($row.case):expected=$($expectations[$row.case]):actual=$($row.query_kind)"
    }
    if ($row.ok -and $row.case -in @('KO_ADVICE','EN_ADVICE','EN_ELLIPSIS','EN_NEGATIVE','EN_FRONTED_CONDITION') -and $row.presuppositions -ne 0) {
        $failures += "PRESUPPOSITIONS:$($row.case):expected=0:actual=$($row.presuppositions)"
    }
    if ($row.ok -and $row.case -in @('EN_FACTUAL','EN_FACTUAL_WITH_EMBEDDED_CONDITION','EN_FACTUAL_WITH_QUOTED_CONDITION') -and $row.presuppositions -lt 1) {
        $failures += "PRESUPPOSITIONS:$($row.case):expected>=1:actual=$($row.presuppositions)"
    }
    if ($row.ok -and $row.case -eq 'EN_QUOTED_HYPOTHETICAL' -and ($null -ne $row.query_kind -or $row.presuppositions -ne 0 -or $row.response_act -ne 'INFORM_ACKNOWLEDGEMENT')) {
        $failures += "QUOTED_OWNERSHIP:$($row.case):expected=no-query-inform-ack"
    }
}
$result = [ordered]@{schema='B_CORE_HYPOTHETICAL_QUESTION_BOUNDARY_PROBE_1'; executable_sha256=(Get-FileHash $exe -Algorithm SHA256).Hash; rows=$rows; failures=$failures}
$result | ConvertTo-Json -Depth 20
if ($failures.Count -gt 0) { exit 1 }
