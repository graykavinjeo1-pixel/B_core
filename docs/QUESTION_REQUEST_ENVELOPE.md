# Source-bound question requests

This Rust engineering increment improves observation -> interpretation ->
attributed retrieval -> response. It does not establish unrestricted dialogue,
autonomous concept emergence or a new semantic research stage.

## Product boundary

ContentRequestIR and QuestionRequestIR share a source-complete response-argument
parser. It separates an addressed information governor, self-recipient and
response manner from the inner argument. Quoted, negative, conditional, mixed
action and third-party requests cannot acquire answer-only authority here.

QuestionRequestIR retains the normalized source, inner question and manner.
Its source-to-question transformation is replayable. The existing indexed
event/role grammar must consume the entire inner question. English embedded
object questions receive only a grammatical auxiliary; Korean final `는지`
complements are converted to a dictionary-checkable predicate form. This is
bounded morphology, not support for every irregular Korean ending.

The existing direct question engine performs the same event matching, role
projection, ambiguity checks and source-attributed realization. No sentence-to-
answer lookup, new answer template, semantic generation or execution permission
was introduced. The public response validator rejects changed or removed
question receipts. Inner queries cannot recursively acquire another envelope.

The clause graph now recognizes pronominal recipients before a communication
governor's wh complement. Previously `tell me where ... read ...` incorrectly
created two matrix roots. Recipient authorization remains separately checked.
Direct contextual Korean why questions also use the shared relation reference
path; explicit ordinary why questions retain prior presupposition checking.

Relational nouns and event-role gaps remain distinct. Development regressions
caught `cause` being treated as a possible event predicate; the envelope now
requires a supported role gap, preserving cause/summary response recombination.

## Verified scope and remaining limits

- Adapter library: 697 passing tests; dockable core library: 29.
- Six existing CLI regression suites, fmt, clippy and API build pass.
- Direct/wrapped equivalence tests compare evidence and claims across six
  Korean/English role requests, plus a direct/embedded causal pair.
- Prior X evaluation, now development: 7/8 semantic full, previously 5/8.
- New frozen Y: 10 requests / 21 turns; 8 full, 1 partial, 1 failed.
  Complete surfaces were absent from searched code/scripts/prior reports.
  Tests and scoring remain developer-authored, not independently blind.
- Original U and R outputs are unchanged, including their known failures.

Y08 preserves missing map evidence but its gap topic becomes `me who wrote map`:
the later generic gap relabeler still uses the outer request's subject. Y10 still
plans an explanation about the recipients instead of preparing a grounded
third-party message. No delivery occurred. These failures remain recorded; the
frozen source was not changed after evaluation. Detailed manner is metadata,
not richer explanatory synthesis. Source-attributed role wording remains stiff.

## Deployment

State50 / turn response41 / discourse answer5. Eight changed Rust files and this
document plus the acceptance document are mirrored into pakage. Hash parity is
not an independent package build. Use a fresh conversation session; no stored-
state migration or running-service restart was performed. No commit or push.
Canonical files, semantic payloads, quarantine and unrelated package edits are
unchanged. This checkpoint is progress, not completion of natural conversation.
