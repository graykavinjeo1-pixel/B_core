# B_Core Pragmatic Reasoning R8

Status: **PASS**
GPT-grade status: **NOT YET**

R8 adds a bounded, temporally versioned epistemic ledger to the pure-Rust Language Cortex. The adapter can now distinguish an active attributed belief from a reaffirmed, contested, superseded, or retracted version while continuing to treat every record as dialogue-local provenance rather than established world truth.

This is a verified structural capability increase, not a claim of GPT-level language understanding.

## Material capability increase

- Each attributed or user-stated proposition receives a tamper-evident belief record linked to its discourse referent.
- Proposition signatures separate subject, local state axis, state value, surface polarity, and coarse temporal anchor.
- An explicit same-source update can supersede an opposite earlier state without deleting its history.
- An unmarked same-source contradiction remains contested instead of silently selecting the most recent statement as truth.
- Opposite claims from different sources remain concurrently contested.
- Repeated equivalent claims create a typed reaffirmation relation and retain only the current reference-active version.
- Explicit retraction deactivates the targeted claim. If that removes one side of a conflict, the remaining record returns from contested to active.
- Explicit correction can supersede the latest same-source claim even when the state vocabulary is previously unknown.
- Past and present claims such as “yesterday down” and “today up” are not treated as a logical contradiction.
- Source-specific references can retrieve an older active claim while newer claims from other sources remain present.
- Active proposition memory no longer discards unrelated earlier sources or topics on every new assertion.
- Ledger storage is bounded to 64 records and 128 revision edges.

The implemented path is:

```text
attributed / dialogue proposition
  -> source + polarity + epistemic status
  -> subject/state/temporal signature
  -> bounded belief record
  -> compare with active compatible records
  -> reaffirm | contradict | supersede | retract
  -> source-aware active proposition references

ledger record
  -> always dialogue_truth_established=false
  -> always external_execution_authorized=false
```

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 174 passed, 0 failed |
| Workspace tests | 635 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| R4 discourse-program canary | 18/18 |
| R5 cross-turn discourse canary | 20/20 |
| R6 semantic-role/discourse canary | 24/24 |
| R7 attribution/discourse canary | 34/34 |
| R8 epistemic-revision canary | 30/30 |
| R1–R8 frozen canaries | 159/159 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R8 canary contains fourteen English cases, thirteen Korean cases, and three language-independent safety/boundedness cases. It covers marked and unmarked self-conflict, cross-source conflict, explicit correction, unknown-state correction, reaffirmation, retraction, conflict resolution after retraction, negative completion, enablement, document/person disagreement, unrelated topics, distinct temporal anchors, older source-specific references, unavailable retracted references, direct-command separation, attributed-command non-authority, and bounded retention.

Development exposed five structural counterexamples:

1. `corrected that` reused the old demonstrative interpretation of `that`. The complementizer classifier now includes correction predicates.
2. Korean `…에 따르면` was classified as a request by the legacy speech-act fallback even though the attribution graph was valid. Attribution-bearing, goal-free clauses now take the Inform path before request heuristics.
3. R7 proposition memory replaced every older proposition on a new turn. R8 retains unrelated active claims and lets typed revision state decide what becomes inactive.
4. The Korean negative state `비활성` initially risked matching the shorter positive substring `활성`. State signature extraction now selects the longest matching semantic marker.
5. Retracting one side of a conflict left the other record permanently contested. Contested states are now reconciled after every turn and stale conflict entries are removed.

The known independent `semantic-reasoning --all-features` module-identity issue remains outside this language change. The default workspace, standard deny-warnings Clippy gate, and all 635 workspace tests pass.

## Safety boundary

The ledger is a record of discourse commitments, not semantic authority. Every belief record is validated with `dialogue_truth_established=false` and `external_execution_authorized=false`; changing either value invalidates the conversation state even after recomputing its outer hash. Retraction and correction operate only on bounded discourse records. They do not rewrite promoted concepts, execute attributed commands, choose a winner between sources, or convert lexical opposition into canonical world knowledge.

## Honest boundary

The current proposition signature uses a compact local collection of state axes and surface-independent polarity composition. It is not open-domain entailment. Temporal scope is limited to coarse past, present, future, or unspecified anchors; exact dates, intervals, event time versus report time, recurring states, and temporal overlap are not modeled. Source reliability is not calibrated. Modality, uncertainty, counterfactual belief, nested quantifier scope, generalized contradiction, and pronoun-based actor continuity remain incomplete. The ledger is memory-bounded and has no transactional export/import or durable rollback format yet.

The next frontier is nested modal and counterfactual scope, attribution-aware question answering, exact temporal relations, uncertainty/evidence aggregation without truth promotion, long-distance actor resolution, transactional ledger snapshots, and larger family-held-out adversarial dialogue evaluation. The long-term objective remains active.

No commit or push was performed. The worktree contains current, prior-stage, and pre-existing changes.
