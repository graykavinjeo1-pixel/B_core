# R63 Adversarial API Regression and Package Seal Report

`R63_STATUS=PASS`

R63 closes the currently defined Language Cortex engineering sequence by
adversarially regressing the public cognitive API, preserving all public
schemas, synchronizing the portable Rust package, and sealing the package
boundary. It does not claim unrestricted GPT-level dialogue.

## Frozen evaluation

The frozen diagnostic first scored 9/18 and reached 18/18 after product-only
repairs. The held-out transfer suite was kept unopened until that diagnostic
passed; its first exposure scored 4/12. Structural repairs progressed through
9/12, 10/12, and 11/12 to 12/12 without modifying the evaluator. The combined
fresh R63 result is 30/30.

The frozen files remained byte-identical:

- diagnostic: `bed2e07f39b9dd952b9f3bf40aee6f34ec57c9e3d1adaa23be9afc798ee4ca92`
- held-out: `ac1b58db33c19c3da8593388c93a75300539c22d517469aa8d6cb3c789b05865`
- public API seal: `b9e2d73720ee7c19dc3cae602968177339c958b1d856371c3d255b5379d809ee`
- shared R62 support: `deaa67306149bc1b984ba8fe860d7b6c814af37896ef0b618d79ef7ee26e6e51`

## Structural repairs

- Mixed immediate and conditional requests now produce separate typed
  commitments. The immediate action reaches current GoalIR; the conditional
  action remains a non-authoritative deferred commitment until evidence.
- Quoted commands stay inert while a live clause after a closing quote is
  still parsed as the user's request.
- Korean hold-floor fillers preserve the active task instead of creating a new
  goal or severing context.
- A singular result reference across independently active actions fails closed.
  A plural result reference binds to the persistent action group, preserving
  the older multi-result lifecycle contract.
- Independently introduced people are not silently merged behind `they`.
- Clarification outranks stale lifecycle text; late prohibitions bind only to
  their governed step; and plan wording explicitly denies execution.

## Public API and regression result

The public API seal passes 7/7: schema identity, serde round trip, live
validation, tamper rejection, package boundary, Rust-only default runtime, and
zero forbidden dependency counters. Public response schema 17, frontend 3,
integration schema 4, six-axis schema 2, conversation state 27, and core ABI 1
remain unchanged.

The selected historical canaries for structural realization,
evidence-grounded realization, and full-axis integration all pass. R62 transfer
returned to 10/10 after plural result phrases were routed through the existing
persistent action-group binding instead of singular ambiguity handling.

Verification totals:

- Fresh R63: 30/30.
- Adapter library: 465/465.
- Root workspace: 989/989 substantive Rust tests.
- Portable package: 492/492 Rust tests.
- Portable runtime boundary canaries: 4/4.
- Root check and all-target Clippy: pass.
- Product-source and complete package Rustfmt: pass.
- `git diff --check`: pass.

## Rust-only package boundary

All Language Cortex implementation changes are Rust. The canonical language
path made zero Python, external LLM, local-teacher, network, or recursive
source-mutation calls. Some root diagnostic tests intentionally invoke a
missing Python test environment to verify repository-failure classification;
that sandbox fixture is not a language-runtime dependency.

The portable `pakage` is now
`B_CORE_PORTABLE_PRODUCT_CORE_R63_WORKTREE_ABI1`. Its 50 adapter product sources
and 14 core product sources match the root workspace byte-for-byte. Research
canaries remain excluded, and only four minimal runtime boundary canaries are
included. Optional `python-paddle-ocr` compatibility remains disabled by
default and is not used by the Language Cortex.

## Safety and cleanup

Unsupported explanation facts, semantic-authority violations, recursive source
mutations, full catalog scans, and routing false negatives are all zero. The
pre-existing protected `growth_supervisor.rs` line remains unchanged.

After verification, `cargo clean` removed 19,730 root build-cache files
(13,778,152,778 bytes) and 5,434 package build-cache files (3,783,363,735
bytes). Both `target` directories are absent; these caches are fully
regenerable.

No commit was created and no push was performed. The worktree remains dirty
because the user-owned R13-R63 development history is intentionally preserved.
R63 is complete and no later stage has been started.
