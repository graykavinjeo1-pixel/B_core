# Question-owned gaps and focused answer realization

This Rust engineering increment advances interpretation -> attributed memory ->
response. It improves selected conversational answers, not unrestricted dialogue
or autonomous concept emergence. Canonical semantics and quarantine are unchanged.

## Ownership and information structure

QuestionRequestIR now owns missing-evidence wording as well as successful
retrieval. The generic information-subject adapter cannot overwrite its query.
The generator reads the replay-validated inner question directly for the gap
topic; mutating lookup hints cannot change that displayed topic.

A single validated event-role gap licenses a focused answer. The role already
given by the question is represented by a focused language predicate rather than
a redundant surface slot noun such as `actor` or `장소`. The selected role value
and source remain grounded. No node is silently left uncovered. Multiple requested
roles retain explicit labels; requested detailed manner retains the longer format.
Detailed format still does not synthesize additional explanatory reasoning.

Korean focus uses the existing copular particle/ending grammar. English focus
reconstructs a phrase from the source-attested value, determiner and case marker:
`in/at`, `to`, `from`, or `for`, where applicable. Multiple possible source phrases
cannot be resolved by stylistic preference; unsupported cases use the explicit
role format. The attribution actor is distinguished by a concept identity, not
by matching its display name. Only the sentence-initial letter is capitalized;
proper-name casing lost upstream is not reconstructed by this change.

These are language-grammar rules over a verified projection, not stored answers
for fixture sentences. Claims/evidence are unchanged by focused realization.

## Failed version and current evidence

The first frozen version (state51 / response42) emitted English `You said cellar`.
It passed structural tests but failed the intended naturalness improvement. Its
outputs, hashes and four source files remain in focused_answer_failed_v1 artifacts.
It was not mirrored or released. The corrected version preserves source noun-
phrase grammar and was tested with a separately frozen fresh AA suite.

Current: state52 / response43 / discourse answer5. Adapter library 700 + core
library 29 = 729 passing tests. Six CLI suites, fmt, clippy and API build pass.

- Previous Y, now development: 9/10 semantic full, 0 partial, 1 failed (formerly
  8 full, 1 partial, 1 failed). The overwritten missing-map topic is repaired.
- Original U and R semantic counts remain 19/6/7 and 7/2/2. Ten U surfaces and
  one R surface changed; each retains identical claims, evidence and response act.
- First frozen Z: 10 requests / 19 turns, with English realization failures,
  Korean polite-request routing failure and third-party communication failure.
- Current fresh AA: 8 requests / 16 turns, 6 semantic full, 1 partial, 1 failed.
  Full-surface overlap search found none. Developer-authored/scored, not an
  independently blind evaluation or GPT comparison. This is not 75% naturalness.

## Remaining product failures

`알려주세요` can miss the question envelope, expose `DIALOGUE_USER` and ignore
requested politeness. Third-party message requests still produce a plan about
the recipient rather than grounded message content. English recipient names may
remain lowercase, e.g. `To savel`. Gap replies and repeated attribution still
sound mechanical. General discourse/pragmatics/realization completion is unproven.

## Deployment

Four Rust files and this/acceptance documents are mirrored into pakage. Equality
is not an independent package build. Use a fresh session; no state migration,
service restart, commit or push occurred. Prior failed evidence and unrelated
package edits remain intact. No autonomous mutation or external action was enabled.
