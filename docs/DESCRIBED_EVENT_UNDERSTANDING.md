# Source-bound event understanding (2026-09-05)

## Product change and boundary

This increment extends the existing Rust observation -> attributed memory ->
judgment -> expression path. It does not introduce another response owner,
training loop, executable concept generation, external LLM, or action authority.

Previously, content roles mainly came from executable planner predicates. Ordinary
reports such as a person lending a book survived as text, but their participants
and follow-up questions were not reliably connected. `PropositionContentIR` now
retains a `DescribedEventIR` alongside the existing source-bound bindings.

| Representation | Meaning |
| --- | --- |
| `lexical_entry_ids` | Indexed dictionary alternatives for a predicate mention; not executable semantic effects |
| `roles` | Agent, theme, recipient, source, location, time, duration |
| `negated` | Local predicate negation, distinct from a positive event |
| `event_id` | Hash of the source proposition; every role belongs to that same description |
| `answer_focus.described_event` | Belief/event reference; never generated answer text used as evidence |

Grammar consumes one bounded clause: Korean particles and finite forms; English
subject/verb/object plus prepositions. Korean roles can change order. Predicate
mentions use the existing bounded dictionary index. Fifteen English irregular past
forms are lexical grammar attached to existing entries, not new semantic concepts.

The same-event query joins **all** supplied participants, object, predicate and
polarity. Multiple matches abstain, even if one happens to be newer. A role fragment
can bind to a valid answer focus (up to three turns) or an unambiguous immediately
preceding report. Explicit role/predicate constraints supersede that focus. Missing
roles, revoked evidence and non-actual records cannot supply an answer.

Source/attitude questions retain their existing owner. The event answer uses the
existing `DiscourseAnswerIR`/`ContentProjectionIR` and expression-graph realization.
Loose topic matching cannot consume the new event-bound roles. Source replay and
query matching protect the projected value. Korean copula allomorphs are selected
from the value's final sound rather than a fixed ending.

## Acceptance and reproduction

```powershell
$env:CARGO_BUILD_JOBS='1'
cargo fmt --all -- --check
cargo clippy --locked --offline -p semantic-core-adapters --lib --bin b-core-cognitive-api --example build_nikl_lexicon -- -D warnings
cargo test --locked --offline --workspace --lib
cargo build --locked --offline -p semantic-core-adapters --bin b-core-cognitive-api
./scripts/verify_described_event_understanding.ps1
./scripts/verify_utterance_understanding.ps1
./scripts/verify_world_conversation_planning.ps1
./scripts/verify_bilingual_lexicon.ps1
```

The new CLI diagnostic has 12 cases / 46 turns, including 33 questions. It checks
role values/abstentions, same-event continuity, output content, and no action grants.
It includes Korean order changes, English past forms, location, duration, source,
cross-event mismatch, ambiguity and negation. Unit tests also cover a generated
36-variant entity/order grid, source tampering, revoked/possible/denied records,
source-query ownership and bilingual lexical identity.

These are developer-controlled diagnostics, **not a blind benchmark or a general
understanding/naturalness percentage**. All outcomes and actual output text belong
in `reports/language-cortex-completion/described_event_understanding_2026-09-05.json`.

## Explicit remaining limits

- This is single-clause descriptive role understanding, not full causal/commonsense
  reasoning. Lending does not establish ownership, a return obligation, permission,
  benefit, or user intent to act. Those need separate semantic mechanisms/evidence.
- Dictionary alternative overlap is not complete word-sense disambiguation or
  valency checking. Entity identity is textual (plus a small speaker mapping), not
  general alias/coreference reasoning. Mixed-language clauses are not supported.
- Arbitrary subordinate/relative clauses, passives, idioms, multi-token English
  subjects, free modifiers, double-object syntax and broad tense/aspect are not
  covered. Known unsupported connectives and unconsumed tokens reject the new
  path; this is not a certification that every malformed clause is detected.
- Time/duration retain the stated surface. No absolute calendar grounding or
  arithmetic is inferred. A denied event is not an occurrence.
- Initial report acknowledgements are still verbose and repetitive. Role answers
  remain explicit/technical. Existing normalization lowercases English names.
  Free conversation and pragmatic understanding are **not established** here.
- No new general recommendation or hidden intention inference is claimed. Earlier
  bounded decision inquiries and state correction continue on their existing paths.

## Integration / rollback

Current state schema: `B_CORE_CONVERSATION_STATE_35`.
Current response schema: `B_CORE_CONVERSATION_TURN_RESPONSE_26`.
See [event references](EVENT_REFERENCE_UNDERSTANDING.md) for the follow-on
question-reference binding fields and its delivery status.
Consumers must accept the new content slots, `events`, optional binding `event_id`
and optional focus reference. Start a new session or explicitly migrate stored
state; source replay rules changed. No silent migration or service deployment.

Root sources and the portable `pakage` mirror contain this change. Canonical files
and promoted concepts are unchanged. Unrelated package-only changes are preserved.
Rollback requires reverting this scoped increment coherently (grammar, QA, schema,
realization and mirrored files), not removing another module's response path.
