# R55 — Topic-scoped reference, ellipsis, and QUD

Status: `PASS`

R55 makes result reference and questions under discussion follow exact discourse
topic identity rather than global recency. Returning to a suspended topic now
restores only that topic's result slot and pending question. An unseen topic
cannot borrow the latest result or QUD from another topic.

## Frozen evaluation

The ten-case diagnostic was frozen before product repair. Its first run was
`1/10`: only the unseen-topic QUD safety case passed. Result references selected
the globally latest event, while a topic return restored the wrong question and
action. All ten response contracts nevertheless remained structurally valid,
and language obtained no semantic or execution authority. The general state
repair raised the unchanged diagnostic to `10/10`.

The independent eight-case transfer suite was frozen after the repair and
before first execution. It passed `8/8` on first exposure. It covers novel
Korean and English topic surfaces, bare-result ellipsis, cross-language
restoration, a three-topic indexed long return, unseen-topic rejection, and
independent restoration of multiple QUDs. After first exposure, only an
`#[allow(dead_code)]` lint attribute was added to the transfer wrapper; no case,
expectation, support logic, or oracle changed. The final suite remains `8/8`.

## Product boundary

- Result and event discourse referents carry a non-authoritative topic ID.
- `QuestionUnderDiscussionIR` carries topic identity and conversation state
  retains a bounded topic-indexed QUD collection plus its active compatibility
  projection.
- Named, indexed, long-horizon, and cross-language topic returns use exact
  topic matching.
- Explicitly scoped result lookup may cross the ordinary recency window but
  never falls back to another topic.
- Typed result markers are kept out of generic entity-pronoun resolution.
- Bounded Korean and English bare-result forms compile through the same typed
  reference path.
- A missing topic result remains unresolved and requests clarification.
- A referenced result is still only a planned or discussed result slot unless
  trusted execution evidence exists; R55 does not fabricate execution.
- No whole-sentence dispatch, promoted semantic-payload mutation, new concept
  generation, external action, LLM dependency, network call, or recursive
  source mutation was introduced.

The public response schema remains 12, nested pragmatic memory remains IR_2,
and the core ABI remains 1. Conversation state advances to
`B_CORE_CONVERSATION_STATE_26`. Persisted-state consumers must accept additive
referent/question `topic_id` fields and `topic_pending_questions`; R55 does not
claim a migration loader.

## Verification and reintegration

- Fresh R55 diagnostic and held-out tasks: `18/18`
- Selected historical R21/R39/R40/R49/R51/R52/R53/R54 regressions: `235/235`
- Historical plus R55 selected tasks: `253/253`
- Adapter library tests: `434/434`
- Root workspace substantive tests: `957/957`
- Portable package tests: `461/461`
- Portable runtime boundary canaries: `4/4`
- Root and package format checks: pass
- Root and package all-target Clippy with warnings denied: pass
- Canonical manifest: pass, 10 files,
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Package adapter sources: 44 exact files, hash mismatches `0`; R55 research
  canaries included `0`
- Authority violations, unsupported explanation facts, external LLM calls,
  local teacher calls, network calls, recursive source mutations,
  full-catalog scans, and routing false negatives: `0`

The first package compile after copying only `conversation.rs` and
`cognitive.rs` failed because package-local test constructors in
`discourse_qa.rs` and `discourse_relations.rs` lacked the additive topic fields.
That integration failure is preserved in the JSON report. Copying the exact
root product versions restored source parity and the final package passed
`461/461`.

Product sources are synchronized into `pakage`. After verification, the root
build cache (12,651 files; 11,038,845,455 bytes) and package build cache (5,434
files; 3,598,050,430 bytes) were removed. No commit or push was performed.

## Bounded residuals

Reference and ellipsis resolution remains deliberately bounded to inspectable
typed forms. Topic identity still depends on the deterministic discourse
router, and no persisted conversation-state migration loader exists. These
limits prevent an unrestricted GPT-level language claim.

## Completion boundary

R55 includes response-boundary regression, portable-package synchronization,
runtime verification, and cache cleanup; it needs no separate R55 integration
step. The broader final product integration remains in the success-assumed
roadmap. Eight macro stages remain, beginning with R56 grammatical composition
and scope generalization and ending with integration, adversarial hardening,
and package/API sealing. The broad GPT-level objective remains active and
unproven.
