# R54 — Topic-scoped pragmatic state generalization

Status: `PASS`

R54 replaces global-recency continuation state with topic-scoped pragmatic
memory. Returning to a suspended topic now restores only that topic's typed
task and pending continuation gate. A new topic with no known task fails closed
with `CURRENT_TASK` unresolved instead of borrowing work from another topic.

## Frozen evaluation

The ten-case diagnostic suite was frozen before product repair. Its first run
was `0/10`: topic identity restored correctly, but every case selected the
globally latest task or leaked that task into an unseen topic. All response
contracts remained valid and no language string obtained execution authority.
The general product repair raised the unchanged semantic cases to `10/10`.

The independent eight-case transfer suite was frozen before first execution
and passed `8/8` on first exposure. It covers new Korean and English topic
surfaces, indexed returns across two topics, two separately suspended gates,
cross-language restoration, similar topic names, and unseen-topic rejection.
`rustfmt` changed support and held-out source bytes after freezing, but changed
no case, expectation, or oracle semantics; both suites remain at their original
post-repair scores.

## Product boundary

- `PragmaticTaskFrameIR` and `PendingContinuationGateIR` carry optional topic
  IDs and are selected by exact restored topic identity.
- Multiple suspended topics retain independent continuation gates.
- Explicit topic scope never falls back to the globally latest task or gate.
- Named, indexed, long-horizon, and cross-language returns share the same
  deterministic selection path.
- Continuation-task anaphora is kept out of generic entity resolution.
- An unresolved current task requires clarification and cannot create an
  immediate `CONTINUE` goal.
- A historical UTF-8 regression found by R21/R40 was repaired generally:
  source-frame slicing now checks `is_char_boundary` and otherwise fails
  closed.
- No whole-sentence dispatch, semantic-payload mutation, new concept
  generation, external action, LLM dependency, network call, or recursive
  source mutation was introduced.

The public response schema remains 12, conversation state remains 25, and the
core ABI remains 1. The nested pragmatic-memory schema changed from IR_1 to
`B_CORE_PRAGMATIC_MEMORY_STATE_IR_2`. Consumers that inspect or persist this
nested state must accept the additive topic fields. R54 does not claim a
persisted-state migration loader.

## Verification and reintegration

- Fresh diagnostic and held-out tasks: `18/18`
- Selected R21/R39/R40/R51/R52/R53/R54 language regressions: `215/215`
- Adapter library tests: `431/431`
- Root workspace substantive tests: `954/954`
- Portable package tests: `458/458`
- Portable runtime boundary canaries: `4/4`
- Root and package format checks: pass
- Root and package all-target Clippy with warnings denied: pass
- Canonical manifest: pass, 10 files,
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Package adapter sources: 44 files, hash mismatches `0`; R54 research
  canaries included `0`
- Authority violations, unsupported explanation facts, external LLM calls,
  local teacher calls, network calls, recursive source mutations, full-catalog
  scans, and routing false negatives: `0`

Product sources are synchronized into `pakage`. After verification, the root
build cache (14,175 files; 10,735,795,627 bytes) and package build cache (6,048
files; 4,354,804,603 bytes) were removed. No commit or push was performed.

## Bounded residuals

R54 does not yet make ResultIR or QUD state topic-scoped, and it does not solve
open-ended reference and ellipsis resolution. Topic identity remains bounded
by the deterministic discourse router. These limits prevent the result from
supporting an unrestricted GPT-level language claim.

## Completion boundary

R54 includes response-boundary regression, package synchronization, runtime
verification, and cache cleanup; it needs no separate integration step. Under
the current success-assumed roadmap, seven macro stages remain. The next is
R55 reference/ellipsis plus topic-scoped result and QUD restoration. The broad
GPT-level objective remains active and unproven.
