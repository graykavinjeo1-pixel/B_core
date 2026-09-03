# R24 Illocutionary Commitment and Result-Claim Control

Status: **PASS**

R24 repairs the boundary between grammatical predicate detection and actual conversational authority. It does not treat every action word as an assistant command. The Language Cortex now records who is committed, who is addressed, which illocutionary force applies, whether the commitment is active, and whether current external execution is authorized.

## Result

The frozen 24-case diagnostic improved from **0/24** to **24/24**. Its six balanced categories cover participant commitments, capability questions versus indirect requests, deferred conditional authorization, answer-only requests, active-goal withdrawal, and verified-outcome-only reporting.

The separately frozen 16-case transfer suite scored **13/16** on its first semantic execution. The remaining failures exposed two general state-ordering defects: a typed answer-only force did not discharge lower-level intent competition/local pronoun ambiguity, and a typed withdrawal force did not discharge the local `that request` binding. The product rules were repaired without changing the oracle; the final transfer score is **16/16**.

Together these are **40/40 fresh R24 tasks** and **947/947 cumulative R1–R24 tasks**.

## Added semantic mechanisms

- `DialogueParticipantIR` separates user, assistant, system, third party, and unknown roles.
- `IllocutionaryForceIR` distinguishes self-commitment, reported commitment, capability questions, indirect action requests, deferred requests, answer-only requests, goal withdrawal, and outcome-claim constraints.
- `CommitmentActivationIR` separates immediate, condition-pending, and inactive commitments.
- `GoalWithdrawalIR` represents all-goal and ordinal withdrawal. The conversation memory retires the selected active goal plus its event/result referents and recomputes its state hash.
- `OutcomeClaimPolicyIR` records a `VERIFIED_OUTCOME_ONLY` boundary and typed evidence requirements.
- `DELETE` and `DEPLOY` are distinct semantic predicates, allowing an information request about an operation to remain active while that operation itself stays prohibited.

These structures regulate GoalIR projection. User promises, reported promises, system capability questions, pending conditional requests, withdrawals, and outcome-reporting constraints create no current execution goal. Addressee-directed indirect requests may create one. Answer-only requests may keep a safe explain/investigate goal while destructive predicates remain non-authoritative.

## Six requested capabilities

1. Grammatical composition remains intact through the R23 relative-clause, quantifier, and event-sequence regressions and now exposes more precise destructive/deployment predicates.
2. Discourse/topic state now includes an explicit hash-bound active-goal lifecycle rather than only adding goals.
3. Deixis and ellipsis resolve bounded local commitment pronouns and ordinal withdrawal targets without silently guessing unrelated referents.
4. Speech intent and pragmatic inference now use typed participant, addressee, force, activation, and authority fields.
5. Plans, user/third-party commitments, pending actions, withdrawals, and verified outcomes are represented separately.
6. Korean/English pragmatic realization is selected from those typed fields; all R24 outputs report `unsupported_freeform_claims=0`.

## Verification

- R24 diagnostic: **24/24**
- R24 held-out transfer: **16/16**
- All canary binaries: **39/39**
- Adapter unit tests: **278/278**
- Workspace tests: **802/802**
- `cargo fmt --all --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- External LLM, local teacher, network, Python language-path, and recursive source-mutation calls: **0**

The implementation is pure Rust. No commit or push was performed.

## Remaining boundary

This stage establishes the missing structural circuit, not GPT-level open-ended language mastery. The next high-value work is persistent condition tracking that can activate a deferred request only after verified antecedent evidence, multi-speaker nested/conflicting commitments, broader morphology and indirectness, and substantive evidence-backed answers when domain capability facts are available. Open-vocabulary generative fluency remains a separate residual.
