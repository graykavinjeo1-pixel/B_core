# Read-only reproducible extractor; outputs JSON, never modifies the workspace.
$ErrorActionPreference='Stop'
$revision='74790861f652b15e4ac49015a90074ad62a27690'
$url="https://raw.githubusercontent.com/cmusphinx/cmudict/$revision/cmudict.dict"
$source=(Invoke-WebRequest -Uri $url -TimeoutSec 30).Content
if($source -is [byte[]]){$source=[Text.Encoding]::UTF8.GetString($source)}
$hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($source))).ToLowerInvariant()
if($hash -ne '81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22'){throw 'CMUDICT_SOURCE_HASH_MISMATCH'}
if((Get-FileHash -LiteralPath crates/semantic-core-adapters/data/lexical-knowledge/nikl-ko-en.jsonl).Hash -ne 'd614048f9df99ac9104cf0206ace8cb9238f0f6a7490bfb06980d6fbf69d23c8'){throw 'BASE_LEXICON_HASH_MISMATCH'}
$aliases=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($line in Get-Content -LiteralPath crates/semantic-core-adapters/data/lexical-knowledge/nikl-ko-en.jsonl){
 $entry=$line | ConvertFrom-Json
 if($entry.pos -ne '동사'){continue}
 foreach($sense in $entry.senses){
  foreach($alias in ($sense.english -split ';')){
   $word=$alias.Trim().ToLowerInvariant()
   if($word -cmatch '^[a-z]+$' -and $word -match '[^aeiou][aeiou][^aeiouwxy]$' -and ([regex]::Matches($word,'[aeiou]').Count -gt 1)) { [void]$aliases.Add($word) }
  }
 }
}
$allForms=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($line in ($source -split '\r?\n')){if($line -match '^([a-z]+)(?:\([0-9]+\))? '){[void]$allForms.Add($Matches[1])}}
$entries=[Collections.Generic.SortedDictionary[string,object]]::new([StringComparer]::Ordinal)
foreach($line in ($source -split '\r?\n')){
 if($line -notmatch '^([a-z]+)(?:\([0-9]+\))? (.+)$'){continue}
 $word=$Matches[1]; $phones=($Matches[2] -split '#')[0].Trim()
 if(-not $aliases.Contains($word)){continue}
 if(-not $entries.ContainsKey($word)){$entries[$word]=[Collections.Generic.List[string]]::new()}
 $entries[$word].Add($phones)
}
foreach($word in @($entries.Keys)){
 $pron=@($entries[$word]); $forms=@(($word+'ed'), ($word+$word.Substring($word.Length-1)+'ed')) | Where-Object {$allForms.Contains($_)}
 $entries[$word]=[pscustomobject]@{pronunciations=$pron;attested_forms=@($forms)}
}
[pscustomobject]@{schema='B_CORE_ENGLISH_FINAL_STRESS_LEXICON_1';source_url=$url;source_revision=$revision;source_sha256=$hash;
 selection='All single-word lowercase English aliases of existing Korean verb entries, ending in orthographic CVC excluding w/x/y and with more than one written vowel; no task/outcome selection.';
 base_dictionary_sha256=(Get-FileHash -LiteralPath crates/semantic-core-adapters/data/lexical-knowledge/nikl-ko-en.jsonl).Hash.ToLowerInvariant();
 candidate_aliases=$aliases.Count;entry_count=$entries.Count;entries=$entries} | ConvertTo-Json -Depth 8 -Compress
