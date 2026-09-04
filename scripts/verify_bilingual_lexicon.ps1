param([string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\b-core-cognitive-api.exe'))
$ErrorActionPreference = 'Stop'
$cases = @(
    @('먹었어','먹다'), @('먹을래','먹다'), @('먹자','먹다'), @('먹었거든','먹다'),
    @('먹긴 했는데','먹다'), @('먹으려나','먹다'), @('갔어요','가다'), @('왔거든','오다'),
    @('봤잖아','보다'), @('했는데','하다'), @('읽었어','읽다'), @('마셨어요','마시다'),
    @('들었어','듣다'), @('걸었어요','걷다'), @('도왔거든','돕다'), @('지었어','짓다'),
    @('몰랐어요','모르다'), @('썼는데','쓰다'), @('들을래','듣다'), @('걸으려나','걷다'),
    @('도우려나','돕다'), @('마실래','마시다'), @('읽자','읽다'), @('먹지','먹다'),
    @('피곤했어','피곤하다'), @('답답하잖아','답답하다'), @('애매하네요','애매하다'),
    @('솔직히','솔직히'), @('글쎄','글쎄'), @('어쩐지','어쩐지'),
    @('계약서를','계약서'), @('보험료는','보험료'), @('재산권에서','재산권'),
    @('계약','계약'), @('contract','계약'), @('zzqvnoncezz','')
)
$commands = [Collections.Generic.List[object]]::new()
$commands.Add(@{operation='LEXICAL_KNOWLEDGE_PACK_STATISTICS'})
foreach ($case in $cases) { $commands.Add(@{operation='LOOKUP_LEXICAL_KNOWLEDGE';text=$case[0]}) }
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = (Resolve-Path -LiteralPath $Executable).Path
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
$start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
$process = [Diagnostics.Process]::new()
$process.StartInfo = $start
$watch = [Diagnostics.Stopwatch]::StartNew()
[void]$process.Start()
$outTask = $process.StandardOutput.ReadToEndAsync()
$errTask = $process.StandardError.ReadToEndAsync()
foreach ($command in $commands) { $process.StandardInput.WriteLine(($command | ConvertTo-Json -Compress)) }
$process.StandardInput.Close()
if (-not $process.WaitForExit(30000)) { $process.Kill(); throw 'LEXICON_CLI_TIMEOUT' }
$watch.Stop()
$stdout = $outTask.GetAwaiter().GetResult()
$stderr = $errTask.GetAwaiter().GetResult()
if ($process.ExitCode -ne 0) { throw "LEXICON_CLI_EXIT:$stderr" }
$responses = @($stdout -split '\r?\n' | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json -Depth 100 })
if ($responses.Count -ne $commands.Count -or @($responses | Where-Object { -not $_.ok }).Count) { throw 'LEXICON_API_RESPONSE_FAILURE' }
$stats = $responses[0].payload.value
if ($stats.general_unique_lemmas -ne 10000 -or $stats.law_economics_related_unique_lemmas -ne 5000) { throw 'LEXICON_COUNTS' }
$rows = [Collections.Generic.List[object]]::new()
$koreanIds = @()
for ($i = 0; $i -lt $cases.Count; $i++) {
    $case = $cases[$i]
    $lookup = $responses[$i + 1].payload.value
    if ($lookup.semantic_authority -or $lookup.execution_authority -or $lookup.full_catalog_scans -ne 0) { throw 'LEXICON_BOUNDARY' }
    if ($lookup.pack_sha256 -ne $stats.pack_sha256) { throw 'LEXICON_PACK_BINDING' }
    if ($case[1]) {
        $found = @($lookup.matches | Where-Object { $_.entry.lemma -ceq $case[1] })
        if (-not $found.Count) { throw "LEXICON_MISSING:$($case[0])->$($case[1])" }
        if ($case[0] -ceq '계약') { $koreanIds = @($found.concept_ids) }
        if ($case[0] -ceq 'contract' -and -not @($found.concept_ids | Where-Object { $_ -in $koreanIds }).Count) { throw 'LEXICON_SHARED_SENSE' }
    } elseif (@($lookup.matches).Count -ne 0 -or 'zzqvnoncezz' -notin $lookup.unmatched_tokens) { throw 'LEXICON_UNKNOWN_TERM' }
    $rows.Add(@{input=$case[0];expected_lemma=$case[1];passed=$true;index_probes=$lookup.index_probes;truncated=$lookup.truncated;
        observed_lemmas=@($lookup.matches.entry.lemma | Sort-Object -Unique)})
}
[ordered]@{status='PASS';suite='CONTROLLED_LEXICAL_COVERAGE_NOT_BLIND_DIALOGUE_QUALITY';commands=$responses.Count;
    lookup_cases=$cases.Count;statistics=$stats;rows=$rows;elapsed_milliseconds=$watch.ElapsedMilliseconds;
    executable_sha256=(Get-FileHash -LiteralPath $start.FileName -Algorithm SHA256).Hash.ToLowerInvariant();stderr=$stderr
} | ConvertTo-Json -Depth 12
