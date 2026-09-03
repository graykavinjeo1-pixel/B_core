# R19 Question Under Discussion and Adjacency Pairs

Status: `PASS`

R19 repairs the missing relation between a clarification question and the user's short answer. Conversation state now contains a typed, hash-bound Question Under Discussion with explicit options. A fragment such as `두 번째`, `API 조사 쪽`, or `the backup option` selects one pending semantic option instead of becoming a new fact or a new command.

The QUD lifecycle is explicit. A hesitation, thanks, or bounded wait expression preserves the pending question. A direct new request clears it and follows the new request without inheriting clarification authority. An invalid ordinal, uncertain answer, or third-party reported choice fails closed and asks again. A resolved short answer carries `ClarificationAnswer` evidence while explicitly adding no new execution authority.

Cross-language selection binds the same option identity and changes only the selected surface used by the answer-language adapter. QUD state and option payloads are covered by the conversation-state hash. Tampering with an option after hashing is rejected.

## Final evidence

- Frozen diagnostic suite: initial `0/24`; intermediate `13/24`, `22/24`; final `24/24`
- Held-out transfer and attack suite: first `13/16`; final `16/16`
- Held-out suite SHA-256 unchanged: `23573467d02018f8d0774c4cc05ebda51c51dbe43dfd205bb2c0aa1b19fb1acb`
- R19 fresh total: `40/40`
- Prior sealed R1-R18 tasks: `707/707`
- Cumulative R1-R19 canaries: `747/747`
- Canary binaries: `29/29`
- Adapter unit tests: `260/260`
- Workspace tests: `784/784`
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- Final build-cache cleanup: 11,205 files / 9.6 GiB removed; `target/` absent

The initial diagnostic, first held-out failures, diagnostic-harness corrections, and a typed-coreference regression found by the full canary run are preserved under `failed_runs/`. The held-out suite and its predicates were not edited. The regression repair distinguishes a real embedded substring (`그가` inside `로그가`) from valid Korean particle continuations (`그녀의 주장을`).

## Next completion boundary

R19 does not establish GPT-class open-domain language understanding. The next integrated work must audit and close six interacting axes: grammatical composition; discourse and topic state; deixis and ellipsis; speech intent and pragmatic inference; execution-result versus plan distinction; and evidence-grounded realization. Several bounded mechanisms already exist, but none should be declared generally solved until cross-axis blind transfer, long-context interference, ambiguity, contradiction, and authority attacks pass together.

The canonical R19 path made zero external LLM, local teacher, network, Python, or recursive source-mutation calls. No commit or push was performed. The worktree remains intentionally dirty because the sealed but uncommitted R13-R19 increments coexist. The broader GPT-level language goal remains open.
