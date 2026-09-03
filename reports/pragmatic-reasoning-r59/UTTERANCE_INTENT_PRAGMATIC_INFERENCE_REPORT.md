# R59 — Utterance Intent and Pragmatic Inference

## Outcome

R59 passes. The Language Cortex now represents the difference between literal
clause form and the response the user is actually requesting through a bounded,
hash-bound `UtteranceIntentGraphIR`. The graph covers controlled Korean and
English problem disclosures, evidence and recommendation requests,
explanation and summary requests, conditional decisions, and corrections to
the requested response goal. It remains an adapter artifact: it cannot become
semantic authority or authorize external execution.

## What was repaired

- A bare problem disclosure can request bounded diagnosis or a next step
  without granting repair or mutation authority.
- Evidence, recommendation, explanation, summary, and conditional-decision
  obligations use one typed candidate interface instead of whole-sentence
  dispatch.
- Explicit compositional action requests outrank generic disclosure readings.
- A response-goal correction replaces the stale requested response rather than
  being displaced by an earlier action candidate.
- A prior-context reference without an antecedent remains unresolved and fails
  closed.
- Korean prohibitions such as `…면 안 돼` and desiderative requests such as
  `…해줬으면 해` no longer become false conditionals.
- Retired referents are removed from suspended topic contexts as well as the
  active topic, preserving state validation after goal withdrawal.
- A failure utterance linked to an already active action remains an unverified
  action-state report. It does not silently become a second diagnostic task and
  it does not alter the verified execution result.

The graph is bounded to 32 signals and 8 candidates. Its source text, prior
context, selected candidate, and graph hash are validated. The response schema
advances additively to `B_CORE_CONVERSATION_TURN_RESPONSE_14`; frontend 3,
conversation state 27, pragmatic-memory IR_2, and core ABI 1 remain unchanged.

## Blind evidence

The frozen diagnostic began at 0/16 and finishes 16/16. The held-out transfer
suite was opened only after the diagnostic passed. Its first observation was
8/10: the two response-goal-correction cases lost priority to stale action
candidates. After repairing that general priority boundary, the unchanged
held-out suite finishes 10/10.

The original first-exposure terminal output handle was unavailable after
terminal compaction, so the preserved 8/10 observation is recorded explicitly
rather than reconstructed as a fresh run. The frozen evaluator sources stayed
byte-identical throughout:

- support: `5617ccf4eb53a471ae528ff5409f118c7cc0d9aeaa06084b9505aeeb7c34e13c`
- diagnostic: `748653bf10666e41a64d00840aa0b085d39d7111aae17197e7c90b6c15bfdf89`
- held-out: `68864e9000e4151f44c51bdff5d87299c46c98e071044ad7323d78e729006657`

## Regression and build verification

- R59 diagnostic and held-out: 26/26.
- Selected historical language-boundary families: 20/20 suites.
- R30 action-state diagnostic after the report/diagnosis boundary repair:
  32/32.
- Adapter library: 454/454.
- Root workspace: 978/978 substantive tests.
- Portable package: 481/481 tests and 4/4 runtime boundary canaries.
- Root workspace all-target Clippy passed with the two established structural
  allowances.
- Product sources and the complete package pass Rustfmt checking. The only
  full-root formatting exception is the previously sealed R57 transfer file,
  which remains byte-identical rather than invalidating its first-exposure
  seal.
- Canonical manifest: 10/10 files and matching self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`.
- Package source sync: 62/62 Rust product files, zero mismatches, and zero R59
  research canaries included.
- `git diff --check`: pass.

The frozen R50 response fixture is not counted as a behavioral regression: it
expects response schema 13 while the valid live response is additive schema
14. Its state alignment, hashes, authority boundaries, and unsupported-claim
count remain valid. The fixture was not rewritten to manufacture a pass.

## Portable package

`pakage` advances to `B_CORE_PORTABLE_PRODUCT_CORE_R59_WORKTREE_ABI1`. All 48
adapter product sources, including `utterance_intent.rs`, are synchronized with
the root workspace. Together with 14 core source files, all 62 product files
match byte-for-byte. Research canaries remain excluded, and the default
runtime is still Rust-only.

## Boundary and remaining stages

No external LLM, local teacher, network, Python language-path call, recursive
source mutation, full catalog scan, language-derived semantic authority, or
language-derived execution authority was used. The protected
`growth_supervisor.rs` user line remains unchanged.

R59 is complete and reintegrated. Assuming each stage succeeds, four macro
stages remain:

1. R60 — execution result versus plan-state separation
2. R61 — evidence-grounded natural realization
3. R62 — full-axis integration and cross-interference repair
4. R63 — adversarial regression, package/API seal, and final boundary report

R62 is the explicit final integration stage. R63 stress-tests and seals that
integrated result. This R59 report does not start R60.
