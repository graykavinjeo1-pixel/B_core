# B_Core Pragmatic Reasoning R4

Status: **PASS**
GPT-grade status: **NOT YET**

R4 turns independently grounded clauses into an inspectable discourse program. Connected commands are no longer forced into one winning intent: each goal remains a node, sequence or coordination becomes an edge, and conditions and prohibitions remain explicit constraints. Definition-grounded predicate knowledge can also cross process boundaries through a canonical SHA-256-verified snapshot.

## Material capability increase

- Korean and English requests such as read → transform → save become ordered multi-goal graphs.
- A later conditional action does not retroactively make an earlier action hypothetical.
- “do not deploy” / “배포하지 마” remains an explicit prohibition while adjacent creation and save goals remain viable.
- Requests separated only by a semicolon and lacking an explicit relation remain ambiguous and require clarification.
- Learned predicate lexemes are sorted, validated, duplicate-checked, hashed, exported, and verified again before import.
- Snapshot tampering or identity conflicts fail closed.
- Multi-goal language is projected to `PlanIntentIR::Plan` and then to the existing GoalIR path; it never dispatches directly to a solution or external action.

The implemented path is:

```text
surface language
  -> predicate frames + semantic scopes
  -> interpretation candidates
  -> ordered/conditional/prohibited goal graph
  -> pragmatic plan
  -> GoalIR
  -> existing semantic reasoner
```

No external LLM, network inference, direct text-to-solution path, or promoted semantic payload mutation was introduced.

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 134 passed, 0 failed |
| Workspace tests | 595 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| R4 discourse-program canary | 18/18 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R4 canary contains sixteen controlled Korean/English graph and ambiguity cases plus canonical snapshot round-trip and tamper-rejection cases. Backward execution of the R3 suite exposed two real boundary regressions: `지웠더라면` was rejected by the tightened inflection check, and Korean curly quotes were not treated as lexical delimiters. Both rules and the formerly vacuous counterfactual unit assertion were repaired before the whole workspace was rerun.

An additional non-gating probe, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, still fails in the pre-existing `semantic-reasoning` feature combination because SEM-26's `DirectorState` is included through two distinct Rust module identities. The default workspace build, standard Clippy gate, and all 595 tests pass. This independent configuration defect is recorded rather than silently attributed to R4.

## Honest boundary

This increment is not GPT-grade general language understanding. It provides reusable structural machinery for compound intent, authority, ordering, conditions, and durable learned lexemes, but it still lacks broad pretrained linguistic/world knowledge, complete dependency parsing, logical quantifier and temporal scope, robust long-distance coreference, and large-scale open-domain calibration.

The next frontier is a semantic-role/dependency graph for nested clauses, logical condition propositions, cross-turn goal-graph revision and ellipsis recovery, transactional durable language memory, and a substantially larger family-held-out adversarial suite. The long-term goal remains active; R4 is a verified capability increment, not completion.

No commit or push was performed. The worktree still contains current, prior-stage, and pre-existing changes.
