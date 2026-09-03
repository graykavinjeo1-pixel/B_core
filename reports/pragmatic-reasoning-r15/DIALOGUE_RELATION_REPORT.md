# R15 Typed Causal and Concessive Multi-Turn Discourse

Status: `PASS`

R15 adds one bounded capability: an utterance beginning with a cross-turn connector such as `Because of that`, `Therefore`, `Even so`, `그 때문에`, `그래서`, or `그럼에도` can bind to the latest unique proposition in conversation state. The resulting edge is stored in the hashed `B_CORE_DIALOGUE_RELATION_GRAPH_IR_1` graph and can support a Korean or English relation question later.

This is discourse structure, not a learned world mechanism. Every edge and answer is forced to retain `dialogue_claim_only=true`, `causal_truth_established=false`, `semantic_authority=false`, and `external_execution_authorized=false`. A relation question produces no grounded plan. Ambiguous, stale, or absent antecedents require clarification, quoted connectors do not trigger, and an existing result-reference marker cannot steal a relation connector.

## Final evidence

- Frozen diagnostic suite: `36/36`
- Held-out transfer and adversarial suite: `15/15`
- R15 fresh total: `51/51`
- Prior sealed R1-R14 tasks: `526/526`
- Cumulative R1-R15 canaries: `577/577`
- Adapter unit tests: `239/239`
- Workspace tests: `763/763`
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Final build-cache cleanup: 9,948 files / 8.3 GiB removed; `target/` absent

The initial diagnostic failure (`21/36`), the first adapter regression (`237/239`), and an overconstrained QA-gate attempt are preserved under `failed_runs/` with their causes and repairs.

## Boundary

The canonical R15 path is pure Rust and made zero external LLM, local teacher, network, or recursive source-mutation calls. No commit or push was performed. The worktree remains intentionally dirty because the sealed but uncommitted R13, R14, and R15 increments coexist.
