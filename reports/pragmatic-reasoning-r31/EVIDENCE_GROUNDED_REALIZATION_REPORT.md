# R31 Evidence-Grounded Realization

Status: **PASS**

R31 closes the bounded sixth item in the current Language Cortex roadmap: the final conversational text now carries a claim-level provenance object built directly from typed semantic products. It does not reconstruct evidence by parsing its own prose.

## Result

The frozen 32-case diagnostic scored **0/32 before implementation** and **32/32 on the first product execution**. It covers plan claims, user-reported outcomes, host-verified execution, attributed dialogue records, temporal relations, dialogue relations, missing evidence, and nonfactual interaction responses.

The separately frozen 16-case transfer suite scored **16/16 on first exposure**. Neither frozen oracle changed. The diagnostic and transfer hashes remain `4fc9925d822fff462807d8aa47ee76316099dd8a982e4aa14fa4c6029652b7ec` and `45201b9e8aec42f8bc329e9565a02f70d4c8da0e47b63beb49e173f20d8bd080`.

Together these are **48/48 fresh R31 tasks** and **1251/1251 cumulative R1-R31 tasks**.

## Added boundary

- `GroundedClaimIR` records claim kind, proposition, epistemic status, support status, evidence IDs, source turns, and verification state.
- `EvidenceGroundedRealizationIR` binds the exact output text and claim set with a deterministic SHA-256 seal.
- Plans remain `PLANNED / STRUCTURALLY_GROUNDED` and cite a plan hash and goal ID.
- User outcome statements remain `REPORTED / REPORTED_ONLY`; they cannot become verified execution.
- Execution claims become `VERIFIED_OBSERVED / VERIFIED_EVIDENCE` only when the typed action ledger contains accepted host receipt IDs.
- Discourse, temporal, relation, and conditional-guard answers cite their existing belief, event, relation, path, or guard evidence.
- An absent execution result is an explicit `UNKNOWN / EVIDENCE_ABSENT` claim.
- Social and dialogue-management text is `INTERACTION / NON_FACTUAL`, not a world claim.

Every claim has a source turn. Every non-social claim has at least one evidence reference. The realization layer has no semantic authority and does not execute external actions.

## Verification

- R31 diagnostic: **0/32 baseline**, **32/32 first product execution and final**
- R31 held-out transfer: **16/16 first exposure and final**
- All canary binaries: **53/53**
- Counted canary rows: **947/947**; 15 legacy canaries without aggregate row fields also exited successfully
- Adapter unit tests: **296/296**
- Workspace library tests: **819/819**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::manual_is_multiple_of`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- External LLM, local teacher, network, Python language path, recursive source mutation, and language-triggered external action: **0**

The clippy exceptions apply only to already-frozen diagnostic harnesses: R29's argument-count fixture and R31's parity expression. The R31 file was not changed after its baseline hash was recorded. Cargo hard-link messages are filesystem cache warnings, not Rust lints.

## Relation to the six capabilities

1. Grammatical composition: R26 remains green within its bounded typed clause scope.
2. Discourse/topic state: R27 remains green within its bounded clause-aware focus scope.
3. Deixis/ellipsis: R28 remains green for tested focus binding and omitted arguments.
4. Speech intent/pragmatics: R29 remains green for its bounded intent families.
5. Plan/result distinction: R30 remains green for plan, report, observed execution, success, and failure.
6. Evidence-grounded realization: R31 now traces every current conversational output branch to typed evidence or marks it nonfactual.

These are bounded implementations, not unrestricted GPT-level language understanding. The next engineering dependency is an integrated adversarial suite that combines all six axes in the same long conversations, followed by broader vocabulary and multi-speaker transfer.

## Remaining boundary

The SHA-256 seal detects mutation of the internal realization artifact; it does not prove that an external source is true. Tool-specific result payloads still need provenance extractors. Open-domain paraphrase, implicature, humor, bridging reference, long multi-speaker discourse, and unrestricted fluent generation remain incomplete.

No commit or push was performed. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` was preserved and not edited by R31.
