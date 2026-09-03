# R48 Hash-Bound Guarded Workflow Evidence Lifecycle Report

Status: **PASS**

R48 closes the previously unverified boundary between R47's guarded discourse
program and the trusted condition-evidence lifecycle. Each guarded program step
now carries the exact `deferred_commitment_id` in its program hash. Conversation
state validation cross-checks that link against the commitment's condition
hash, normalized antecedent, subject, intent, predicate, source text, turn
provenance, and any activated GoalIR.

The lifecycle remains normalized: mutable status and accepted evidence stay in
`DeferredActionCommitmentIR`; the program stores only the hash-bound identity
link. This avoids two competing copies of lifecycle state.

## Frozen evaluation

- Diagnostic preimplementation baseline: **0/12**
- Diagnostic final: **12/12**
- Held-out transfer first exposure: **7/8**
- Held-out transfer final: **8/8**
- Fresh R48 tasks: **20/20**
- Oracle corrections: **0**
- Diagnostic source SHA-256:
  `9CBA5F07098B578DC192FDA7F9882BDFB6D5DC655F6EF4F2042707246EA2875F`
- Transfer source SHA-256:
  `AB43B7E8AE8EEAE435024DBFEF0108DF03FBA1CC16215C36A158885B0EB75DE3`
- Diagnostic JSON SHA-256:
  `26B9240BEE152ACCB1745D0D0AAA280EEB2A701840AEF9B7E4387EFF9EA4D9A8`
- Transfer JSON SHA-256:
  `449E2FE20A2D032A9A1027EA9666BA74F9DA20CFAB6670C17C86670C1E7F5287`

The first held-out failure exposed a cross-language realization gap. Korean
`검사` fell back to the canonical English label `investigate`; the deterministic
directive parser therefore did not reconstruct the conditional commitment.
The product realization now maps the general INVESTIGATE-family form
`검사 → inspect`. The frozen transfer file and oracle did not change.

Two harness-only mechanical corrections changed no case or expected answer:
the shared enum permits diagnostic-only variants in the transfer build, and a
closure borrow is ended by lexical scope instead of `drop(closure)` so the full
Clippy run can deny warnings. The diagnostic case file and transfer file hashes
remain unchanged.

## Structural and authority boundary

The schemas are now:

- `B_CORE_CONVERSATION_STATE_24`
- `B_CORE_DISCOURSE_PROGRAM_IR_3`
- `B_CORE_DISCOURSE_PROGRAM_GUARD_IR_2`

A verified satisfied receipt activates only the commitment named by the guard
link. Verified contradiction creates no GoalIR. Wrong condition hashes, foreign
commitment IDs, and replayed evidence fail closed. Evidence for the source
workflow cannot activate its rebound copy.

Natural-language claims and attributed reports may contribute dialogue evidence
for deliberation, but cannot transition the deferred commitment. The trusted
typed evidence channel remains separate. Even an accepted receipt records
`external_action_executed=false`; it activates planning state, not an external
side effect. Language, program templates, and guards retain no semantic or
execution authority.

## Regression evidence

- R48 diagnostic and held-out transfer: **20/20**
- R47 diagnostic and held-out transfer: **20/20**
- R25 deferred lifecycle diagnostic and transfer: **40/40**
- Modal-scope canary: **55/55**
- Conditional-guard canary: **56/56**
- Adapter library tests: **395/395**
- Root workspace substantive tests: **919/919**
- Root fmt, all-target Clippy with warnings denied, and `git diff --check`:
  **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`

The root test run again encountered optional missing-`pytest` probe output, but
the owning tests and the complete workspace run exited successfully. Python is
not used by the R48 language path.

## Portable package

The product-only `pakage` directory is synchronized as
`B_CORE_PORTABLE_PRODUCT_CORE_R48_WORKTREE_ABI1`:

- Adapter product sources: **43/43**, hash mismatches 0
- Dockable core files: **20/20**, hash mismatches 0
- R48 research canaries included: **0**
- Package workspace tests: **422/422**
- Minimal runtime canaries: **4/4**
- Package fmt and all-target Clippy: **PASS**
- Network/LLM Cargo dependency hits: **0**
- Default language runtime: **Rust only**

## Safety, cleanup, and boundary

External LLM calls, local teacher calls, network calls, Python calls in the R48
language path, external actions, and recursive source mutations are all **0**.
Sparse runtime checks retain `FULL_CATALOG_SCANS=0` and
`ROUTING_FALSE_NEGATIVES=0`. The pre-existing user edit in
`growth_supervisor.rs` remains unchanged.

After validation, root cleanup removed 5,665 files (4,417,261,885 bytes), and
package cleanup removed 3,581 files (1,890,229,720 bytes). Both `target`
directories are absent.

R48 is complete. The broader GPT-level objective is not complete, and no
unrestricted GPT-level equivalence is claimed. Assuming subsequent stages
succeed, two stages remain: R49 and R50. R50 is the final integration,
whole-system regression, and sealing stage. No commit or push was performed.
