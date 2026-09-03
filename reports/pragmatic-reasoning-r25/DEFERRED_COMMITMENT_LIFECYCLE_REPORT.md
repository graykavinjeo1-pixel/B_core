# R25 Deferred Commitment Lifecycle

Status: **PASS**

R25 closes the lifecycle that R24 intentionally left open. A conditional request is now stored as a hash-bound pending commitment. A later sentence claiming that the condition is true cannot activate it. Only a separate typed host evidence receipt can transition the commitment to `ACTIVATED` or `CONTRADICTED`, and a successful transition creates a GoalIR without executing an external action.

## Result

The frozen 24-case diagnostic improved from **0/24** to **24/24**. It covers pending-state persistence, the language/evidence authority boundary, verified activation, mismatch and contradiction handling, withdrawal, and exactly-once replay protection.

The separately frozen 16-case transfer suite scored **15/16** on its first semantic execution. The failure exposed a general Korean morphology error: the evidential sentence `백업이 확인됐어` contained `됐어` and was therefore mistaken for a withdrawal. Withdrawal recognition was narrowed to standalone or leading discourse-marker use; the oracle remained unchanged and the final transfer score is **16/16**.

Together these are **40/40 fresh R25 tasks** and **987/987 cumulative R1-R25 tasks**.

## Added semantic mechanisms

- `DeferredActionCommitmentIR` persists the condition, semantic action, lifecycle status, turn transitions, and accepted evidence identifiers in the conversation state hash.
- `ConditionEvidenceRequestIR` and `ConditionEvidenceReceiptIR` form a channel separate from natural-language input.
- The evidence receipt binds the conversation, commitment, normalized condition hash, disposition, evidence source, and evidence identifier.
- Verified satisfaction creates exactly one active GoalIR. Verified contradiction and user withdrawal retire the pending commitment.
- Replayed, tampered, mismatched, or stale evidence fails closed.
- Activation is planning state only: every receipt records `external_action_executed=false`.

The SHA-256 binding is an integrity mechanism, not proof that a verifier is truthful and not caller authentication. The host remains responsible for admitting trusted evidence into the typed API. A credentialed verifier channel is still future work.

## Relation to the six language capabilities

This stage does **not** claim that all six capabilities are solved.

1. Grammatical composition advanced for bounded Korean and English postposed conditionals; general compositional grammar remains open.
2. Discourse/topic state now carries pending, activated, contradicted, and withdrawn commitments across turns.
3. General deixis and ellipsis resolution was not expanded in R25 and remains bounded by earlier stages.
4. Pragmatic inference now separates a user's linguistic assertion from host-admitted condition evidence and action authority.
5. A conditional plan, an activated goal, and an executed result are distinct typed states.
6. Evidence-grounded output is faithful for typed lifecycle receipts, but substantive domain answer realization remains open.

The next program is therefore fixed in dependency order: **general grammatical composition → discourse/topic state → deixis/ellipsis → speech intent/pragmatics → plan/result separation → evidence-grounded realization**. Each layer must be tested through held-out structural transfer, not sentence-specific dispatch.

## Verification

- R25 diagnostic: **24/24**
- R25 held-out transfer: **16/16**
- All canary binaries: **41/41**
- Adapter unit tests: **280/280**
- Workspace tests: **804/804**
- `cargo fmt --all --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Frozen diagnostic and transfer hashes: **UNCHANGED**
- External LLM, local teacher, network, Python language-path, external action, and recursive source-mutation calls: **0**

The implementation is pure Rust. No commit or push was performed.

## Remaining boundary

The next hard cases are nested or conflicting multi-speaker commitments, compound condition graphs, authenticated external evidence, long-distance or ambiguous reference resolution, broad morphology, and substantive evidence-backed answers. Open-vocabulary GPT-level fluency remains an active project goal rather than a completed property.
