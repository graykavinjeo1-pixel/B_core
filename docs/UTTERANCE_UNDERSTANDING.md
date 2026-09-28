# Utterance understanding boundary repair

2026-09-05; baseline: `950ec37a4e8252ca93ce2608f5d686e51246cc21`.

## Product change

This Rust-only change advances observation -> source-bound semantic memory ->
communicative judgment. It does not add another response owner, promote dictionary
definitions into executable knowledge, or enable recursive source improvement.

1. Registered Korean 하다 state roots share current-state modifier and ending
   composition. Experiencer ellipsis is resolved in the discourse layer, not the
   lexical parser. Existing referents take precedence; an unbound question cannot
   silently become a statement about the user. Unknown/historical material stays
   unconsumed. English copular state modifiers share the corresponding predicate.
2. Contrastive state correction compiles to two atoms in one source-bound episode:
   reject the old state, assert the replacement. Capacity is checked before either
   write. Old premises are retracted, the replacement becomes focus, and both
   halves must survive replay/serialization. Same-turn unrelated assertions are
   not accepted as a correction episode.
3. The existing utterance-intent graph distinguishes deliberative wh-questions and
   recommendation imperatives from factual lookup/execution. `DecisionInquiryIR`
   records the requested missing input (outcome, deadline, constraints, preference).
   It has no action authority. The existing information-answer owner realizes a
   clarification from that role through expression nodes and grammar. This is a
   bounded clarification capability, **not** successful recommendation generation.
4. Grammar-to-pragmatics integration now includes the selected, validated utterance
   intent. Invalid integrations return a transactional error instead of panicking.
5. Evidence-answer acts cannot receive generic affect-support moves that introduce
   inferred scenarios. Tone may condition realization; it cannot append unsupported
   causes or actions. Explicit source-bound user feedback remains a separate
   obligation. The plan validator enforces the same boundary as the composer,
   including across all auxiliary-signal combinations.

## Reproduce

```powershell
cargo fmt --all -- --check
cargo clippy --locked --offline -p semantic-core-adapters --lib --bin b-core-cognitive-api --example build_nikl_lexicon -- -D warnings
cargo test --locked --offline --workspace --lib
cargo build --locked --offline -p semantic-core-adapters --bin b-core-cognitive-api
./scripts/verify_utterance_understanding.ps1
./scripts/verify_world_conversation_planning.ps1
```

The 26-turn diagnostic includes unchanged failures, not only favorable examples.
Its PASS field covers explicit route/authority/atomicity checks, not naturalness.
Grammar tests include a freshly registered opaque state root, source/context
tampering, negation, incomplete clauses, spoken questions, and capacity rollback.
No training corpus, external LLM, or teacher is called by these paths.

## Remaining limits

- Only registered executable predicates acquire world meaning. The bilingual
  dictionary remains lexical evidence, not general event/causal knowledge.
- Sleep duration, everyday loan/return relations, arbitrary temporal correction,
  and unrestricted conversation are not solved by this change.
- A missing-input inquiry does not yet accumulate arbitrary user preferences or
  derive a domain recommendation. It can still feel generic or ask for information
  present in an utterance the semantic parser does not support.
- Intensity/current-time modifiers are retained in source text but not represented
  as quantitative degree or temporal intervals in the boolean predicate model.
- Unsupported compounds, reported/quoted speech and historical deliberation are
  outside the bounded decision grammar; they must use other existing paths.
- Legacy affect-only generation elsewhere is not certified by this repair.
  Passing internal claim counters alone is insufficient: the diagnostic exposed
  an unsupported affect preface despite those counters reporting zero.

## Integration

Current state schema: `B_CORE_CONVERSATION_STATE_35`.
Current response schema: `B_CORE_CONVERSATION_TURN_RESPONSE_26`.
See [event references](EVENT_REFERENCE_UNDERSTANDING.md) for the latest increment.
The follow-on [described-event increment](DESCRIBED_EVENT_UNDERSTANDING.md) adds
bounded event roles and role-question continuity. The limits/results above describe
this earlier state/decision increment, not a claim that arbitrary events are solved.
Consumers must handle `KOREAN_HADA_EXPERIENCER`, `DECISION_SUPPORT`, and optional
`discourse_answer.decision_inquiry`. Start a fresh conversation or explicitly
migrate old state. No silent saved-state migration or running-service deployment.
The root and `pakage` language sources are mirrored; unrelated package-only edits
are preserved. Canonical documents and promoted semantic payloads are unchanged.
