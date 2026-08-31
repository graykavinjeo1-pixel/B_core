# B_Core Pragmatic Reasoning R2

Status: **PASS**

R2 extends the Language Cortex with bounded discourse state. The system can now carry a typed task/benefit gate across turns, restore an omitted task from that state, keep a user's rejection active, and distinguish selected nonliteral readings without treating surface wording as execution authority.

## What changed

- `PragmaticMemory` stores at most eight task frames and sixteen turn summaries. Its state has a SHA-256 identity and rejects invalid turn order.
- An active continuation gate can restore an elliptical follow-up such as “그 정도면 계속할 만하지” to the prior task and required benefit.
- A user rejection changes the gate to `SUSPENDED_BY_USER`; later ellipsis cannot silently reopen it.
- Sarcastic positive wording after a failure state becomes a negative evaluation, not approval.
- A bounded nonliteral layer maps recognized software metaphors and idioms to semantic states rather than literal physical actions.
- If literal and figurative readings remain plausible, the API asks for clarification and emits no grounded execution plan.

The path remains:

```text
language -> conversation normalization -> pragmatic/nonliteral IR
         -> GoalIR -> existing semantic planner/reasoner
```

No external LLM, network inference, or semantic payload mutation was introduced.

## Regression evidence

| Check | Result |
|---|---:|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Workspace tests | 576 passed, 0 failed |
| Adapter tests | 115 passed, 0 failed |
| Pragmatic R1 canary | 8/8 |
| Pragmatic context R2 canary | 5/5 |
| `git diff --check` | PASS |

The Rust toolchain emitted filesystem notices because the I: drive does not support hard links for incremental compilation caches. These were not code or Clippy diagnostics.

## R2 canary outcomes

1. Multi-turn ellipsis restored `마이그레이션 -> 장애 빈도가 감소한다`.
2. Rejection persisted as `SUSPENDED_BY_USER`.
3. Incongruous praise after failure became `NEGATIVE_EVALUATION_NOT_APPROVAL`.
4. Context-free “불이 났어” required literal/figurative clarification.
5. Software context grounded the same expression as `C_CRITICAL_INCIDENT`.

## Honest boundary

This is a material improvement in discourse reasoning, not GPT-level general language competence. The memory and figurative catalog are intentionally bounded; unfamiliar cultural sarcasm and novel metaphors may remain unresolved. The current implementation also records user turns rather than a full two-party dialogue-event history. In uncertain cases it preserves ambiguity and asks instead of guessing.

No commit or push was performed in this stage. The worktree remains dirty with current, prior-stage, and pre-existing changes.
