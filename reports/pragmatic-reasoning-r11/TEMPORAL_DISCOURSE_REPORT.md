# Pragmatic Reasoning R11 — Typed Event-Time and Temporal Discourse

Status: **PASS**
Long-term target: GPT-grade general language understanding
Current assessment: **NOT YET GPT-grade**

## Outcome

R11 adds a pure-Rust temporal boundary to the existing conversation, modality, epistemic-revision, and evidence-bounded QA layers. It now distinguishes an event's reported time from the turn in which it was mentioned, records typed temporal relations, and answers a controlled family of Korean and English `when`, `before`, `after`, `during`, and order-check questions.

The statement path is:

```text
normalized utterance
  -> bilingual temporal analysis
  -> typed events + typed relations
  -> validated temporal graph
```

The question path is:

```text
normalized question
  -> TemporalQueryIR
  -> entity-aware graph retrieval
  -> direct relation or bounded transitive proof path
  -> cited evidence + Korean/English realization
```

Temporal records remain dialogue evidence. They never become world truth, proof that an event occurred, or execution authority.

## What changed

- Added `TemporalGraphIR`, `TemporalEventIR`, `TemporalRelationIR`, and `TemporalConflictIR`.
- Kept event time distinct from the report turn and modal world.
- Added typed `Before`, `Simultaneous`, and `During` relations.
- Added English and Korean forms for before, after, while/during, and sequential deictic continuation.
- Added yesterday/today/tomorrow variants, relative weeks, ISO and Korean dates, and AM/PM clock expressions.
- Added cross-turn `after that` / `then` / `그 후` / `그 다음` chaining.
- Added direct and transitive relation answers with edge-by-edge proof evidence.
- Added entity-aware event matching so a shared predicate cannot substitute one event entity for another.
- Preserved opposite-order evidence as a typed conflict with both edges contested.
- Kept missing events, unrecorded order, ambiguity, and conflict as explicit abstention dispositions.
- Routed absent, untimed, or exclusively non-actual `when` premises back through R10's presupposition-aware QA instead of accepting them.
- Added graph and answer validation for truth, authority, endpoint, evidence, cycle, and unsupported-claim tampering.

## Safety boundary

Every temporal graph and answer validates these invariants:

- event time and report turn are separate fields;
- events and relations do not establish external-world truth;
- no event, relation, or answer authorizes execution;
- positive relation answers cite the relation edges used;
- transitive answers expose the complete bounded path;
- opposite temporal orders remain contested;
- the active `Before` graph is acyclic;
- missing events and unrecorded order are not invented;
- question turns do not mutate the graph or create goals and plans;
- unsupported explanation claims remain zero.

## Validation

| Check | Result |
|---|---:|
| Adapter unit/integration tests | 214 passed, 0 failed |
| Workspace tests | 675 passed, 0 failed |
| R11 temporal-discourse canary | 56 passed, 0 failed |
| Frozen R1–R11 canaries | 325 passed, 0 failed |
| `cargo fmt --all --check` | PASS |
| Adapter clippy, all targets, warnings denied | PASS |
| Workspace clippy, all targets, warnings denied | PASS |
| External LLM calls | 0 |
| Local teacher calls | 0 |
| Network calls | 0 |

The R11 canary contains 7 English relation-surface cases, 6 Korean relation-surface cases, 17 temporal-expression cases, 15 temporal-question cases, 2 cross-turn/transitive cases, and 9 conflict/tamper cases. All previous R1–R10 canaries were rerun, including R10's presupposition suite.

The initial R11 run was 55/56 because `day after tomorrow` was shortened to `tomorrow`. Longest-expression precedence repaired the semantics. Integration then exposed two R10 presupposition regressions and an entity-matching false positive; both were repaired at the routing and retrieval boundaries without weakening their tests.

## Honest capability assessment

This is a real improvement in multi-turn event reasoning, but it is not GPT-grade general language understanding. The grammar is deliberately bounded; relative times are symbolic rather than calendar-anchored; duration arithmetic, tense/aspect, recurring-event identity, broad paraphrase, and long-distance event coreference remain incomplete. Dialogue records also remain unverified reports by design.

The next high-value increment is verified conditional-guard activation without reverse inference, followed by longer-distance mixed-language coreference, compositional event paraphrase retrieval, and causal/concessive discourse relations with explicit proof paths.

## Repository state

- Branch: `main`
- HEAD: `603eb2a`
- Relative to `origin/main`: ahead 4, behind 0
- Commit created in R11: no
- Push performed: no
- Worktree clean: no; prior uncommitted work and unrelated user changes were preserved

Machine-readable details: [temporal_discourse_report.json](./temporal_discourse_report.json)
