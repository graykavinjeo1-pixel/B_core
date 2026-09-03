# R56 — Grammatical composition and scope generalization

Status: `PASS`

R56 adds an inspectable, hash-bound grammatical-scope graph to the bounded
Korean/English Language Cortex. Quantifiers, restrictions, conjunction,
disjunction, negation, and focus now remain typed structure through
UnderstandingIR and GoalIR instead of collapsing into a flat sentence-level
hint. The graph is language evidence only: it has no semantic or execution
authority.

## Frozen evaluation

The 12-case diagnostic was frozen before product implementation. The corrected
pre-repair baseline was `0/12`; no scope graph existed. The first raw baseline
also scored `0/12`, but the harness incorrectly counted graph absence as 12
authority violations. That instrumentation-only error was repaired before
product code changed, without changing a case or expectation.

The implementation first reached `11/12`. The remaining Korean case exposed a
general morphology defect: concessive `지만` was being split as the focus
particle `만`. Separating those boundaries raised the unchanged diagnostic to
`12/12`. The independent eight-case transfer suite was then frozen with SHA-256
`a86c377d0f5798f0a5fbde8fd4f39063b28af2d3d0b40f12b7ca1e8c5f45a784`
before first execution and passed `8/8` on first exposure. No held-out case,
expectation, or byte changed afterward.

## Product boundary

- `B_CORE_GRAMMATICAL_SCOPE_GRAPH_IR_1` binds typed scope nodes, edges, roots,
  ambiguity records, and its own SHA-256.
- Recursive restriction `AND`, `OR`, and child negation are preserved.
- Quantifier and focus nodes attach to typed semantic-role entities.
- `NONE` scope blocks the governed action.
- Negation plus a non-`NONE` quantifier records unresolved scope ambiguity and
  fails closed instead of guessing.
- Validated structure reaches UnderstandingIR and GoalIR constraints.
- The live six-axis integration validates the graph on every response.
- Korean `지만` is no longer treated as focus `만`.
- English auxiliary-negated coordination preserves the shared argument gap in
  forms such as `Inspect and do not delete the cache`.
- Graph nodes remain `semantic_authority=false` and
  `external_execution_authorized=false`.

The nested compositional schema advances to
`B_CORE_COMPOSITIONAL_ANALYSIS_IR_6`. Response schema 12, conversation state
26, pragmatic-memory IR_2, and core ABI 1 are unchanged. No persisted-state
migration is required; consumers that inspect nested response IR must accept
the additive grammatical-scope field.

## Regression and reintegration

The first selected historical run exposed a real R45 regression. The English
sequence `and do not` was extracted as a direct object, so the following real
object could not bind backward to both predicates. The frozen R45 diagnostic
and transfer suites initially scored `27/28` and `19/20`. A general auxiliary
boundary repair restored them to `28/28` and `20/20` without changing their
cases.

- Fresh R56 diagnostic and first-exposure transfer: `20/20`
- Selected R43/R45/R49/R53/R54/R55 historical regressions: `173/173`
- Selected historical plus R56 tasks: `193/193`
- Adapter library tests: `439/439`
- Root substantive library tests: `962/962`
- Additional SWE binary unit test: `1/1`
- Portable package tests: `466/466`
- Portable runtime boundary canaries: `4/4`
- Root and package format checks: pass
- Root and package all-target Clippy with warnings denied: pass
- Canonical manifest: pass, 10 files,
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Package adapter products: 45 exact source files, mismatches `0`; R56 research
  canaries included `0`
- Authority violations, unsupported explanation facts, external LLM calls,
  local teacher calls, network calls, recursive source mutations,
  full-catalog scans, and routing false negatives: `0`

Product sources are synchronized into `pakage`. The root build cache (13,487
files; 9,957,078,025 bytes) and package cache (5,434 files; 3,645,871,026
bytes) were removed with `cargo clean`. No commit or push was performed.

## Bounded residuals

This is a bounded deterministic Korean/English grammar, not unrestricted
natural-language parsing. Ambiguous scope is preserved rather than resolved
from broad world knowledge. Language structure cannot create semantic facts or
execution authority. These limits prevent a GPT-level equivalence claim.

## Completion boundary

R56 is complete and already reintegrated into the response boundary and
portable package. Seven success-assumed macro stages remain. R57 is discourse
and topic-state consolidation; the dedicated full-axis integration is R62,
followed by R63 adversarial regression and package/API sealing. R57 was not
started in this run.
