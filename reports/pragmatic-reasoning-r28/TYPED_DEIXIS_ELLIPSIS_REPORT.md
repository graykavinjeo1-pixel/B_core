# R28 Typed Deixis and Ellipsis Resolution

Status: **PASS**

R28 connects possessive references, generic demonstratives, and omitted arguments to the typed discourse focus established in R27. The repair is a bounded Korean/English mechanism and does not claim general or GPT-level reference resolution.

## Result

The frozen 24-case diagnostic improved from **0/24** to **24/24**. It covers possessive focus binding, demonstrative nominals, direct and conditional zero arguments, open-vocabulary predicate ellipsis, and fail-closed handling when an antecedent is absent or ambiguous.

The diagnostic harness required one compile-only correction before any semantic execution because it initially read a non-public commitment field. No input, expected semantic outcome, or product code was changed in that correction; the authoritative frozen hash is `2078f4410f30345f7a7dbe5c1edd28b4ad6a2b9b1377573b941024c4c27eed13`.

The separately frozen 16-case transfer suite scored **15/16 on its first semantic execution**. The failing case used the unseen action verb `verify`. Possessive binding was already correct, but the shared action lexicon did not classify that predicate. Adding `verify` and `validate` to the general inspect/investigate family repaired the product without changing the held-out oracle or adding a sentence-specific route. The final transfer result is **16/16**.

Together these are **40/40 fresh R28 tasks** and **1107/1107 cumulative R1-R28 tasks**.

## Added semantic mechanisms

- `TypedDeixisEllipsisResolutionIR` distinguishes possessive focus references, demonstrative focus references, and zero-argument ellipsis.
- English `its` and Korean `그것의`/`그거의` bind to one current typed focus and preserve the property relation.
- Generic demonstratives bind only when there is one fresh focus. Korean replacement regenerates `을/를` and `이/가` from the actual antecedent, producing forms such as `매니페스트를` and `디스패처를`.
- Direct and conditional omitted arguments recover their object from typed focus. Conditional directives create a pending `DeferredActionCommitmentIR`; they do not claim execution.
- Predicate ellipsis can reuse the prior action over a new noun such as `bundle` or `묶음` without requiring a pre-existing concept ID for the noun.
- No-antecedent and competing-antecedent cases request clarification rather than guessing.
- Resolution evidence is non-authoritative. Every new path enforces `semantic_authority=false` and `external_execution_authorized=false`.

## Regression repair

Integration exposed five useful overreach cases, all repaired before sealing:

- Explicit Korean object pronouns such as `그걸` had been receiving a second inferred object.
- Korean action-like nouns in statements, including deployment and repair nouns, were being mistaken for zero-argument commands.
- English `run` inside `continue the run` was being mistaken for a directive predicate.
- The Korean contrast connective `반면` was being mistaken for a conditional `면` suffix.
- Demonstrative substitution initially retained the placeholder's particle and emitted forms such as `매니페스트을`; particles are now regenerated from the resolved noun.

The final full run passes **47/47 canary binaries**.

## Relation to the six language capabilities

1. **Grammatical composition:** R26 remains green. Its typed clause graph is bounded; unrestricted grammar and cleaner adverb attachment remain open.
2. **Discourse/topic state:** R27's typed focus remains green. General multi-speaker and unrestricted long-horizon discourse remain open.
3. **Deixis/ellipsis:** R28 completes the tested Korean/English focus-binding cases. General bridging, plural reference, implicit semantic roles, and broad ambiguity resolution remain open.
4. **Speech intent/pragmatics:** this is the next stage. Existing intent and authority gates pass regression, but broad indirect intent and implicature are not yet solved.
5. **Plan/result distinction:** prior typed boundaries remain green; arbitrary tool histories and observations are still open.
6. **Evidence-grounded realization:** prior realization checks remain green; open-domain claim-level provenance remains open.

The next bounded stage should therefore be **speech intent and pragmatic inference**, followed by plan/result interpretation and evidence-grounded realization.

## Verification

- R28 diagnostic: **24/24**
- R28 held-out transfer: **15/16 first execution**, **16/16 final**
- All adapter canary binaries: **47/47**
- Adapter unit tests: **286/286**
- Workspace library tests: **809/809**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Frozen diagnostic and transfer hashes: **UNCHANGED**
- Temporary R28 debug markers: **0**
- External LLM, local teacher, network, Python language-path, external action, and recursive source-mutation calls: **0**

The implementation is pure Rust. Cargo reported that incremental-cache hardlinks are unsupported on the current drive and copied those cache entries instead; this is an environment warning, not a Rust lint failure. Optional Python `pytest` probes printed that `pytest` is absent, but all 809 Rust library tests passed and the language path did not invoke Python.

No commit or push was performed. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` was preserved and not edited by R28.

## Remaining boundary

R28 does not establish unrestricted reference understanding. The zero-argument path still has residual adverb-attachment artifacts in some internal subject surfaces, and it does not solve bridging references, plural antecedents, missing semantic roles beyond the tested object slot, or unrestricted cross-speaker ambiguity. Those limits must not be hidden behind a PASS label.
