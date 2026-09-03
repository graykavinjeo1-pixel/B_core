# R57 — Discourse and Topic-State Consolidation

## Outcome

R57 passes. The Language Cortex now stores one bounded, hash-bound context per
live discourse topic and restores that topic's own focus, pending question, and
discourse referents on return. A topic switch no longer restores only the topic
label while leaving the globally newest object as the active center.

The new `B_CORE_TOPIC_CONTEXT_GRAPH_IR_1` is part of
`B_CORE_CONVERSATION_STATE_27`. Its `ACTIVATE`, `CONTINUE`, and `RESUME`
transitions bind the topic ID and hash, restored focus, local QUD, and local
referents. Every context and transition remains non-authoritative and cannot
execute an external action.

## What was repaired

- Named, indexed, cross-language, social-interruption, QUD, coordinated-focus,
  and three-topic returns use the same typed topic-context path.
- An explicit return restores the last live local focus for that topic. Missing
  or stale snapshots fail closed instead of borrowing global recency.
- Standalone acknowledgements and hold-floor phrases such as `그래`, `좋아`,
  `yes`, `good`, `one moment`, and `잠깐 생각할게` normalize to non-content
  dialogue acts and cannot displace semantic focus.
- A shared QUD object may become a typed question focus without establishing
  its truth.
- A distinct-target prohibited clause no longer poisons the safe single active
  goal used by parallel ellipsis. Same-target negated programs and deferred
  cross-target workflows remain non-replayable and fail closed.

## Blind evidence

The diagnostic source remains byte-identical at
`61ea9b0f0abbfa9d22a96e6c1e08936193d10034be6c12d28ee01c3e439578c6`.
Its baseline was 0/12 and its final result is 12/12. The held-out transfer
source remains byte-identical at
`168c0539d19407905fccb828e62c0944ac540dbb136cc9ff5092ed55eb8563fb`
and finishes 8/8.

The first raw held-out exposure was 6/8 because the evaluator compared Korean
and English aliases as literal strings. The product had restored the correct
concepts in both failures. The support evaluator was corrected without changing
any case, turn, expected topic, expected focus, or the frozen transfer source.
All 20 final cases validate their graph and full response contracts, with zero
authority violations and zero unsupported explanation facts.

## Regression and build verification

- R57 diagnostic and held-out: 20/20.
- Selected historical suites: 387/388 by frozen raw harness and 388/388 by
  semantic outcome. The sole raw failure is an old R20 evaluator comparing the
  internal Korean alias `로그` directly to the expected English string `log`;
  the resolved text and selected subject both denote `log`.
- R20 safe parallel ellipsis: 24/24 after the cohort-boundary repair.
- R21 long-horizon discourse: 40/40 after neutral social turns stopped replacing
  focus.
- R46 typed discourse-program safety: 20/20, including same-target negation and
  deferred mixed-target fail-closed cases.
- Adapter library: 440/440.
- Root workspace: 963/963 substantive library tests plus 1/1 additional binary
  unit test.
- Portable package: 467/467 tests and 4/4 runtime boundary canaries.
- Root and package Clippy: pass with the same two established structural lint
  allowances.
- Package `cargo fmt --all -- --check`: pass. Root product sources pass direct
  Rustfmt checking. The only full-root format exception is the frozen R57
  transfer source, which Rustfmt would line-wrap; it is intentionally preserved
  byte-for-byte to retain the pre-exposure seal.
- Canonical manifest: pass, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`.
- `git diff --check`: pass.

## Portable package

The `pakage` directory is advanced to
`B_CORE_PORTABLE_PRODUCT_CORE_R57_WORKTREE_ABI1`. All 46 product adapter source
files match the root workspace byte-for-byte, `topic_context.rs` is included,
and no R57 research canary is packaged. The public response schema and core ABI
remain unchanged. Persisted conversation-state consumers must accept additive
schema 27 topic-context data; no automatic migration loader is claimed.

## Boundary and residuals

R57 establishes topic-local discourse-state behavior for a bounded deterministic
Korean/English cortex. It does not establish unrestricted GPT-level language
understanding or globally natural realization. One held-out trace is still
stylistically mixed-language although its topic and focus semantics are correct;
that belongs to later realization/integration work rather than this state gate.

No external LLM, local teacher, network, Python language-path call, recursive
source mutation, full catalog scan, or language-derived semantic/execution
authority was used. The protected `growth_supervisor.rs` user line remains
unchanged.

R57 is complete and reintegrated. Six success-assumed macro stages remain. R62
is the dedicated full-axis integration stage and R63 is the final adversarial
regression plus package/API seal. R58 has not been started by this report.
