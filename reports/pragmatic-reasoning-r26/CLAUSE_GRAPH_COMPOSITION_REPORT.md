# R26 Typed Clause-Graph Composition

Status: **PASS**

R26 replaces pairwise connector guesses in the language-to-GoalIR path with a typed `ClauseGraphIR`. The adapter now records clause spans, clause functions, directed semantic relations, and shared arguments before candidate selection. This is a bounded Korean/English grammar repair; it is not a claim of general natural-language understanding.

## Result

The frozen 24-case diagnostic improved from **0/24** to **24/24**. It covers fronted and postposed relation direction, Korean subordinate direction, shared argument composition, subordinate authority, and surface-order invariance.

The separately frozen 16-case transfer suite scored **15/16** on its first semantic execution. The sole failure exposed a general lexical-category error: `document` after infinitival `to` was treated as a noun. The nominal-context classifier was repaired to recognize `to` as a verb prior. The held-out oracle remained unchanged and the final score is **16/16**.

An initial diagnostic draft had expected Korean `검사한 뒤 수리해` to select only the second event. That conflicted with the sealed R23 contract, so the expectation was corrected before the authoritative pre-product freeze. The final frozen hashes remained unchanged throughout implementation and final verification.

Together these are **40/40 fresh R26 tasks** and **1027/1027 cumulative R1-R26 tasks**.

## Added semantic mechanisms

- `ClauseGraphIR` stores typed clause nodes and directed `CONDITION`, `CAUSE`, `PURPOSE`, `CONTRAST`, `COORDINATION`, `SEQUENCE`, and `TEMPORAL_BEFORE` edges.
- Fronted and postposed English forms compile to the same semantic direction rather than inheriting text order.
- Bounded Korean connective morphology reaches the same graph types without placing lexical tokens in semantic concepts.
- Conditions, causes, purposes, concessions, and temporal subordinate clauses cannot acquire independent execution authority.
- Coordinated directive inheritance reaches a fixed point, so a terminal imperative can license all compatible events in a multi-step chain.
- Shared themes propagate only through licensed coordination and sequence. A temporal consumer can instead bind the prior event result explicitly.
- Coordinate `but` and subordinate `although` are distinct. This preserves direct commands after a contrast and keeps attributed or subordinate commands blocked.
- Clause boundaries stop an attribution complement from absorbing a later independent command such as `but now inspect ...`.

## Relation to the six language capabilities

This stage advances the first capability and preserves the other five; it does **not** close all six.

1. **Grammatical composition:** typed bounded clause composition is now present for the tested Korean and English structures. General morphology and unrestricted nesting remain open.
2. **Discourse/topic state:** earlier hash-bound topic and commitment state remains intact, but it is not yet fully driven by `ClauseGraphIR`.
3. **Deixis/ellipsis:** earlier bounded same-turn and cross-turn resolution still passes; general ambiguity and long-distance ellipsis remain open.
4. **Speech intent/pragmatics:** prior intent and authority gates pass regression, but broad indirect speech-act inference is not solved.
5. **Plan/result distinction:** typed pending, active, prohibited, and prior-result states remain distinct; all conversational result cases are not yet covered.
6. **Evidence-grounded realization:** existing typed realization remains faithful on its tested scope; open-domain claim-level realization is still future work.

The next dependency order is therefore: **discourse/topic state over ClauseGraphIR → deixis/ellipsis → speech intent/pragmatics → plan/result separation → evidence-grounded realization**.

## Verification

- R26 diagnostic: **24/24**
- R26 held-out transfer: **16/16**
- All adapter canary binaries: **43/43**
- Adapter unit tests: **281/281**
- Workspace tests: **805/805**
- `cargo fmt --all --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Frozen diagnostic and transfer hashes: **UNCHANGED**
- External LLM, local teacher, network, Python language-path, external action, and recursive source-mutation calls: **0**

The implementation is pure Rust. No commit or push was performed. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` was preserved and not edited by R26.

## Remaining boundary

R26 does not establish GPT-level language understanding. The main remaining failures are unrestricted nested grammar, multi-speaker and long-horizon topic competition, ambiguous reference and ellipsis, broad pragmatic intent inference, result-versus-plan interpretation across tool interactions, and evidence-backed open-domain realization.
