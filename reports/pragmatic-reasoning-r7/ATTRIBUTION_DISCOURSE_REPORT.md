# B_Core Pragmatic Reasoning R7

Status: **PASS**
GPT-grade status: **NOT YET**

R7 adds a typed attribution and epistemic-provenance graph to the pure-Rust Language Cortex. The adapter can now preserve who said, reported, claimed, believed, doubted, denied, heard, observed, inferred, wanted, or expected a proposition without treating that proposition as an established fact or executable user request.

This is a verified structural capability increment. It is not evidence of GPT-level breadth.

## Material capability increase

- Korean and English attribution sources are represented as discourse-local actors.
- Proposition polarity is distinct from the source's stance. “The deployment did not finish” and “Alice does not believe the deployment finished” therefore produce different structures.
- Speech, hearsay, direct observation, inference, and document provenance remain explicit.
- Nested attribution preserves parent propositions, such as Alice saying that Bob believes a proposition.
- Source-specific later references can select Bob's belief from multiple active propositions; a generic reference remains ambiguous and fails closed.
- English complementizer `that` is no longer misclassified as an unbound demonstrative reference after an attribution predicate.
- Commands and desired actions inside attributed propositions cannot gain dialogue execution authority.
- Contrast boundaries preserve a genuine outer request: the reported command in “Alice said delete the file, but now inspect logs” is blocked while “inspect logs” remains actionable.
- Nominal report events such as “the agent reported the issue” and “사용자는 결과를 보고했다” are not falsely promoted into propositional attribution.
- Attribution source, attitude, and epistemic status are stored in bounded, tamper-evident conversation state.

The implemented path is:

```text
utterance
  -> attribution predicate / provenance construction
  -> actor + proposition + stance + evidence graph
  -> nested-parent and attributed-frame bindings
  -> non-authority enforcement during candidate selection
  -> GoalIR only for an independent outer request

later source-specific reference
  -> bounded attributed proposition referent
  -> source-aware disambiguation
  -> quoted metalinguistic reconstruction
  -> same authority checks
```

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 164 passed, 0 failed |
| Workspace tests | 625 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| R4 discourse-program canary | 18/18 |
| R5 cross-turn discourse canary | 20/20 |
| R6 semantic-role/discourse canary | 24/24 |
| R7 attribution/discourse canary | 34/34 |
| R1–R7 frozen canaries | 129/129 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R7 canary contains eleven Korean attribution cases, eleven English cases, four cross-turn cases, and eight safety/contrast cases. It covers reporting, positive and negative belief, proposition negation, denial, hearsay source, observation, inference, desire, expectation, document provenance, Korean and English nesting, open-vocabulary infinitive desire, source-specific recall, generic ambiguity, reported command recall, direct-command preservation, nominal-report rejection, and attribution-scope reset.

Development exposed four structural counterexamples:

1. Negation inside a Korean proposition was initially lifted into a negative speaker stance. The repair confines stance negation to the attribution predicate itself.
2. English complementizer `that` was initially treated as an unresolved demonstrative on the first conversation turn. The repair uses attribution-predicate context to distinguish the two grammatical roles.
3. An attributed English complement initially extended through a later contrastive request. The repair adds an explicit contrast boundary both to proposition span calculation and reported-action scope.
4. A transitive report event with a noun object was initially eligible for proposition construction. The repair requires proposition-shaped complement evidence while preserving quoted, clausal, and infinitival content.

The known independent `semantic-reasoning --all-features` module-identity issue remains outside this language change. The default workspace, standard deny-warnings Clippy gate, and all 625 workspace tests pass.

## Safety boundary

Every attributed proposition has `dialogue_truth_established=false` and `external_execution_authorized=false`. Actors and proposition surfaces remain discourse-local IR rather than promoted concepts. Attribution metadata is valid only on proposition referents; attaching it to an event fails state validation even after an attacker recomputes the state hash. Generic competing references require clarification. The adapter performs no external action, network request, LLM call, semantic payload mutation, source self-mutation, or direct text-to-solution conversion.

## Honest boundary

This is still a deterministic Korean/English analyzer. Actor extraction and complement recognition are controlled rather than open-domain; unrestricted morphology, free indirect discourse, arbitrary scope islands, pronoun attribution, long-distance dependency, and deeply mixed nested clauses remain incomplete. The graph records attribution but does not yet perform belief consistency, contradiction history, source reliability calibration, temporal belief revision, or general entailment. It lacks broad world knowledge and has not been calibrated against humans or frontier language models.

The next frontier is temporally versioned belief and contradiction state, explicit belief revision and retraction, nested modal/negation/quantifier scope, evidential reliability without lexical authority, attribution-aware question answering, and a larger family-held-out adversarial dialogue suite. The long-term objective remains active.

No commit or push was performed. The worktree contains current, prior-stage, and pre-existing changes.
