# Lexical stress supplies missing inflection knowledge

The native dialogue failure for `opened` was a lexical morphology gap: the base
form existed but a conservative spelling rule rejected multi-vowel CVC endings
without stress information. This change supplies that missing lexical knowledge
and keeps sentence understanding in the existing event-role pipeline.

## Sources and bounds

- [CMUdict](https://github.com/cmusphinx/cmudict), pinned revision
  `74790861f652b15e4ac49015a90074ad62a27690`, supplies pronunciation and word-form
  attestations. Original source SHA-256:
  `81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22`.
- Selection is the entire relevant spelling class among existing single-word
  English aliases of Korean verb entries, not words selected from dialogue tests.
  There are 217 candidates, with 212 pronunciation entries found. Of those, 183
  have at least one candidate suffix form attested. These are data coverage
  counts, not language competence or autonomous generation scores.
- [Cambridge spelling guidance](https://dictionary.cambridge.org/grammar/british-grammar/spelling-and-verb-forms)
  describes stress-sensitive final-consonant doubling. The new bounded rule uses
  the North American pronunciation data and single-consonant American spelling
  where final stress is absent; it does not claim every British variant.

`scripts/extract_english_stress_metadata.ps1` is a read-only reproducible extractor.
It verifies the pinned source and original bilingual dictionary hashes, prints
JSON and never writes source files. Runtime loads only the sealed small JSON;
there is no network or teacher call in conversation processing.

The complete CMU license is retained in
`crates/semantic-core-adapters/data/lexical-knowledge/CMUDICT-LICENSE.txt` and must
accompany redistributions, including binary distributions. Pronunciation data is
not guaranteed error-free by its supplier.

## Rule and epistemic boundary

The existing irregular-verb exclusions and one-syllable/e/y spelling rules remain.
For an otherwise uncertain multisyllable CVC stem, all available pronunciation
variants must agree about final stress. Primary and secondary stress are treated
as stressed. Unknown or conflicting stress yields no new form.

The stress-derived candidate must also occur in the source dictionary. This is
important because a Korean verb's English equivalent is not automatically an
English base verb. Pronunciation of an adjective or already inflected word alone
must not license another past suffix. Attestation alone is also insufficient:
the source contains nonstandard spellings, and only the grammar-derived candidate
is eligible. Unsupported paradigms remain unresolved.

No bilingual semantic entry or concept ID changes. The auxiliary metadata hash
is attached to lexical lookups so validation cannot silently omit its provenance.
One-time metadata parsing is hash-checked, bounded to at most 512 entries, and
subsequent access uses an indexed word lookup. No full concept-catalog scan,
sentence cache, new reasoning authority or recursive-improvement policy is added.

## Verification and release boundary

Tests check stressed/unstressed families, unsupported and conflicting readings,
invalid suffix spellings, semantic-ID invariance, provenance tampering and actual
event-memory follow-ups. Existing dialogue regressions remain part of the run.
Output still follows meaning -> grammar -> one generation -> one final check.
Personality and affect may affect expression, not the stored event or truth.

Schemas: lexical lookup 5, conversation state 110, response 106, discourse answer
37. No automatic state migration, deployment, commit or push is performed.
Unrestricted natural conversation is not established by this lexical increment.
