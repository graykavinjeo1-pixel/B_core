# Source-backed Korean / English lexical knowledge

See `LICENSE.md` before redistributing. This is dictionary knowledge, not a
legal advice database, world-fact authority, or a sentence-answer collection.

| Partition | Unique Korean headwords | Source entries |
| --- | ---: | ---: |
| General | 10,000 | 11,525 |
| Law / economics related | 5,000 | 5,691 |
| Supplementary grammar | Not included in the 15,000 | 2,157 |

Total: 19,373 source entries, 29,766 bilingual senses. Homographs and multiple
senses are retained; inflections and English aliases do not inflate headword
or semantic-generation counts. General and domain headword sets are disjoint.

General selection retains the user's seven requested state/discourse headwords,
then prioritizes NIKL learner grades and conversational parts of speech. It is
**not a measured corpus-frequency top-10,000 list**. Domain
selection uses source law/economics categories, explicit words in definitions,
and employment categories. It is **not 5,000 expert-reviewed technical legal
terms**. A headword partition does not label every homonymous sense as technical.

Reproduction (Rust only, no model or translation API):

1. Download the August 2026 JSON edition from
   https://krdict.korean.go.kr/download/downloadPopup (snapshot link:
   https://krdict.korean.go.kr/dicBatchDownload?seq=214).
2. Verify archive SHA-256
   `7cf41e62a2a36158a8be2b6d2f84c086221e9b29d4345c44e5497eebf21c8c40`.
   Extract the 11 JSON files to an ignored build-cache directory.
3. Run `cargo run --locked --offline -p semantic-core-adapters --example
   build_nikl_lexicon -- RAW_DIRECTORY CANDIDATES.jsonl`.
4. Run `cargo run --locked --offline -p semantic-core-adapters --example
   build_nikl_lexicon -- --select CANDIDATES.jsonl OUTPUT.jsonl`.
5. Compare output SHA-256 with `manifest.json`; do not replace an existing
   sealed pack without reviewing selection changes and snapshot compatibility.

The public source can change. A different archive must not silently pass as
this snapshot. Raw downloads, candidates, and build caches are not distributed.
Runtime builds embed only the selected JSONL and need no network or Python.
