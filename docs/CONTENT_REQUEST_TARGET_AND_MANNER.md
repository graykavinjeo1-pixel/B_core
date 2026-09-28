# Source-bound content target and response manner

This human-authored Rust change improves observation -> interpretation ->
attributed retrieval -> response. It is not semantic concept promotion,
autonomous emergence, or a new research stage.

## One request, independent axes

ContentRequestIR carries the source text, requested relation, complete argument,
optional explicit target, and optional response manner. A contextual target is
not a wildcard. The deterministic grammar consumes source spans rather than
trusting the shortened predicate-frame theme. Korean genitive arguments and
English `of` complements retain their targets. Bounded concise/detailed manner
markers are removed only at argument/predicate boundaries and stored separately.
Scope, polarity, quotation and independent-request checks remain required.

The shared pure manner extractor also participates in imperative recognition:
a fronted manner adverb is not mistaken for an agent. It neither retrieves
answers nor authorizes execution independently of the existing pipeline.

The typed request enters DiscourseAnswerIR and owns nominal relational requests
before generic reference substitution. Named retrieval requires every content
term of the target in the effect proposition, not just one shared word or a word
in its causal tail. This is conservative source matching, not unrestricted
semantic identity resolution or general causal discovery.

Gap realization reads the same source-bound argument rather than active_subject
or lossy lookup terms. A later stage cannot relabel a motor-vibration question
as the previous reading event. Validation recompiles the request from source,
rejects removed/modified request metadata, binds the relation to its projection,
and disallows mixing an explicit target with a contextual target receipt.

No new whole-answer templates were added. Existing cause and gap realization
knowledge supplies the output. Manner recognition preserves content and records
style intent; Detailed currently does not synthesize additional explanatory
structure or evidence. This is a remaining limitation, not completed rich explanation.

## Verification and limitations

- Adapter library 692 + core library 29 = 721 passing tests.
- Six existing CLI regression suites, fmt, clippy and API build pass.
- Previous V suite improves from 6 full / 2 partial / 1 failed to 9 full semantic
  responses. These cases are now development data, not fresh evidence.
- New W suite: 8 scored requests over 16 turns, 6 semantic full / 2 failed.
  Complete surfaces were absent from searched source, scripts and prior reports.
  The suite is developer-authored/scored, not independently blind or GPT comparison.
- Original 50-turn outputs and previous R suite outputs remain unchanged.

Expanded manner `in more detail` still loses a known cause. The natural Korean
request `왜 그랬는지 좀 자세히 말해줘` still becomes an explanation plan about
`자세히` instead of answering from the event. Successful replies are often stiff,
with quoted causal connective fragments and verbose source disclaimers. These
tests do not establish natural free conversation or fulfill the overall goal.

## Deployment

State48 / turn response39 / discourse answer4. Start a fresh conversation or use
an explicitly authorized migration; no migration or service restart was performed.
Eight changed Rust files and this documentation are mirrored to pakage. Hash
parity is not an independent package build. No commit or push was performed.
Canonical files, protected semantic generations, quarantine, failed reports and
unrelated package edits remain intact.

See reports/language-cortex-completion/CONTENT_REQUEST_REPAIR_2026-09-05.md
and its JSON for frozen hashes, actual outputs and grading qualifications.
