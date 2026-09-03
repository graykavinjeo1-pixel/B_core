# R58 — Reference-Resolution Composition

## Outcome

R58 passes. The Language Cortex now resolves multiple compatible references in
one Korean or English utterance through a bounded, hash-bound
`ReferenceResolutionGraphIR`. It no longer stops after the first possessive or
demonstrative, and it exposes every mention, antecedent candidate, selected
edge, unresolved mention, and source/resolution hash without granting language
semantic or execution authority.

## What was repaired

- Repeated possessives, multiple demonstratives, and possessive plus
  demonstrative combinations resolve compositionally.
- Typed person references and the active discourse focus may both resolve in
  one utterance without competing for one global substitution.
- `former/latter` and `전자/후자` establish a local ordered anchor. A following
  possessive inherits that anchor instead of leaking to the global focus.
- Reference markers inside balanced straight or curly quotations remain inert.
  A marker after the closing quotation is live and resolves normally.
- Multiple missing antecedents remain explicitly unresolved. The cortex does
  not guess or synthesize a binding.
- Validation is bound to the original text, resolved text, selected edge set,
  graph hash, and binding count. Rehashed text or selected-edge tampering is
  rejected.

The graph is bounded to 32 mentions, 32 antecedent candidates, and 256 edges.
Composition selects antecedents left to right, then applies byte-safe surface
replacements right to left. The response schema advances to
`B_CORE_CONVERSATION_TURN_RESPONSE_13` and the frontend schema advances to
`B_CORE_CONVERSATION_FRONTEND_3`. Conversation state 27 and core ABI 1 do not
change.

## Blind evidence

The frozen diagnostic started at 0/14 and finishes 14/14. The held-out transfer
suite remained unavailable until the diagnostic passed and succeeded 8/8 on
its first exposure. All 22 graphs validate, with zero authority violations.

The frozen evaluator hashes remain:

- support: `95ef07817624417f1b29e40cca590513ee011afb40aa81e02c1ecdf98a9cf25c`
- diagnostic: `d0c4bd3cf7a9553be32017393fc965d6467791b8445594c956ab8e4aa7160de3`
- held-out: `b4c0df25d5aebc061810cc6bb8b7dfbb45ec8f08be2f08f7d3e0208b8ae63aa5`

## Regression and build verification

- R58 diagnostic and held-out: 22/22.
- Selected historical reference, topic, grammar, and discourse suites: 166/166.
- Adapter library: 443/443.
- Root workspace: 966/966 substantive library tests plus 1/1 additional binary
  unit test.
- Portable package: 470/470 tests and 4/4 runtime boundary canaries.
- Root workspace all-target Clippy passed with the two established structural
  allowances; the final adapter test target and the full package also pass.
- Product sources and the complete package pass Rustfmt checking. The only
  full-root format exception is the previously sealed R57 transfer file, which
  is preserved byte-for-byte rather than invalidating its first-exposure seal.
- Canonical manifest: 10/10 files and matching self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`.
- Package source sync: 61/61 Rust product files, zero mismatches, and zero R58
  research canaries included.
- `git diff --check`: pass.

## Portable package

`pakage` is advanced to `B_CORE_PORTABLE_PRODUCT_CORE_R58_WORKTREE_ABI1`.
All 47 adapter product sources, including `reference_resolution_graph.rs`, are
synced with the root workspace. Together with 14 core source files, all 61
verified product files match byte-for-byte. Research canaries and build caches
remain excluded. The runtime path is still Rust-only by default.

## Boundary and remaining stages

No external LLM, local teacher, network, Python language-path call, recursive
source mutation, full catalog scan, language-derived semantic authority, or
language-derived execution authority was used. The protected
`growth_supervisor.rs` user line remains unchanged.

R58 is complete and reintegrated. Assuming each stage succeeds, five macro
stages remain:

1. R59 — utterance intent and pragmatic inference
2. R60 — execution result versus plan-state separation
3. R61 — evidence-grounded natural realization
4. R62 — full-axis integration and cross-interference repair
5. R63 — adversarial regression, package/API seal, and final boundary report

R62 is therefore the explicit integration stage, not an omitted afterthought.
R63 validates and seals the integrated result. R59 has not been started by this
report.
