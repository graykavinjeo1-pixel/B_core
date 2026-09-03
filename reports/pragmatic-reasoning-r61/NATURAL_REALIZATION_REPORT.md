# R61 — Evidence-Grounded Natural Realization

## Outcome

R61 passes. The Language Cortex now converts typed semantic, discourse,
pragmatic, reference, and lifecycle state into bounded Korean or English
responses without exposing internal IR labels as conversational prose. Each
emitted sentence carries typed source references, and the exact final text is
hash-bound into the full response receipt.

The public conversation response advances additively to
`B_CORE_CONVERSATION_TURN_RESPONSE_16`. The full response integration advances
to `B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_3`, which validates the new
`B_CORE_NATURAL_REALIZATION_IR_1` payload and hash. Conversation state 27,
pragmatic-memory IR_2, frontend 3, and core ABI 1 remain unchanged.

## What was repaired

- Plan previews describe concrete intended work and explicitly say that the
  work has not yet executed.
- Plan, execution, and terminal-result questions are realized from the R60
  lifecycle boundary rather than inferred from surface wording.
- User-provided information is acknowledged as user testimony, not silently
  promoted to a confirmed fact.
- Feedback, affect, missing references, topic returns, social backchannels, and
  hold-floor turns use typed response acts instead of generic planning prose.
- Korean controlled noun phrases and user-grounded proper labels survive into
  output without broad capitalization changes to semantic option text.
- English “missed the main/key/my point” is classified as feedback.
- `Fix that again` and `Do that one again` fail closed when no antecedent is
  available; the following adverb no longer makes `that` look like a local
  determiner.
- Structured multi-goal responses retain their existing compositional
  realization and cannot leak `compositional_goal_graph` or other internal IR.
- Generic cause-seeking plans receive natural realization while sarcasm,
  metaphor, ambiguity, indirect-problem, and special semantic responses retain
  their more specific typed behavior.
- Empty meta-promises, unsupported claims, internal IR leaks, unbound output,
  or a tampered realization hash fail validation.

## Blind evidence

The frozen diagnostic baseline was 0/16 and finishes 16/16. The held-out suite
was opened only after the diagnostic passed. Its first exposure was 8/10,
failing only Korean controlled noun-phrase display-label recovery and the
general English missed-point feedback cue. General structural repairs brought
the unchanged held-out suite to 10/10.

The frozen evaluator files remain byte-identical:

- support: `be85ba163c31c5bf58eeabb92a97a8a23424fcfe515f75f3ae495bb0dba8adff`
- diagnostic: `87c7ea1c41cb4374d1717bdb08f1778c14628a5fba95de2ff13de7680bf2fa69`
- held-out: `f023a110aa5cdafb898c87be63a7849869323da317faa7b50b40fcad9af602e2`

All 26 fresh cases have zero unsupported claims, zero internal IR leaks, zero
empty promises, zero language-derived authority, and zero external execution.

## Regression and build verification

- R61 diagnostic and held-out: 26/26.
- R22 structural realization: 24/24 diagnostic and 16/16 transfer.
- R31 evidence-grounded realization: 32/32 diagnostic and 16/16 transfer.
- Adapter library: 463/463.
- Root workspace: 987/987 substantive tests.
- Portable package: 490/490 tests and 4/4 runtime boundary canaries.
- Root and package all-target Clippy passed with the two established structural
  allowances.
- Product sources and the complete package pass Rustfmt.
- The full root format check reports only three byte-frozen evaluator sources:
  the R61 diagnostic, R61 transfer, and pre-existing R57 transfer fixtures.
- Canonical manifest: 10/10 files with self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`.
- `git diff --check`: pass.

The old R17 surface fixture remains 10/24 because fourteen cases require
superseded mechanical stage names. The old R18 fixture remains 22/24 because
two feedback-plus-request cases require those labels. The frozen R60 fixture is
0/16 solely because it requires response schema 15 and integration IR_2; the
live additive response is schema 16 and integration IR_3 and preserves the R60
lifecycle component. None of these frozen sources was rewritten to manufacture
a pass.

## Portable package

`pakage` advances to `B_CORE_PORTABLE_PRODUCT_CORE_R61_WORKTREE_ABI1`. All 50
adapter product sources, including `natural_realization.rs`, match the root
workspace byte-for-byte. Together with 14 core sources, all 64 Rust product
files are synchronized. R61 research canaries remain excluded. Package format,
Clippy, 490 tests, and four runtime boundary canaries pass; the default runtime
remains Rust-only.

## Boundary and remaining stages

No external LLM, local teacher, network, Python language-path call, recursive
source mutation, full catalog scan, language-derived semantic authority, or
language-derived execution authority was used. The protected
`growth_supervisor.rs` user line remains unchanged. Verification build caches
were removed from both workspaces: 12,505 root files (10,205,096,946 bytes) and
5,434 package files (3,758,042,525 bytes).

R61 is complete. Assuming continued success, two macro stages remain:

1. R62 — integrate grammar, discourse/topic state, deixis/ellipsis, pragmatic
   inference, plan/result separation, and natural realization into one
   cross-interference-tested dialogue path.
2. R63 — adversarial regression, package/API seal, and final boundary report.

R62 is the final integration stage; R63 is the final stress-test and seal. This
report does not start R62.
