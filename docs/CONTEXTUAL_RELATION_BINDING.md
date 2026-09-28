# Contextual relation binding

This Rust engineering increment advances observation -> attributed memory ->
interpretation -> grounded response. It does not add semantic generations,
autonomous source mutation, network learning, or external execution authority.

## Product change

A bounded relational request (cause/reason, manner/method, definition/meaning)
can refer to an existing proposition without naming it again. The adapter now
selects that proposition before searching its relations. A newer event lacking
a cause cannot make an older event's cause relevant.

The grammar consumes the source argument, not just PredicateFrameIR.theme.
That field can omit modifiers: the first evaluation caught a Korean genitive
target being discarded. The failed version and its source are preserved under
reports/language-cortex-completion/contextual_relation_failed_v1_2026-09-05.json
and contextual_relation_failed_v1_sources. It was never mirrored to pakage.

The current grammar accepts only a bounded independent positive information
request and a fully consumed relational head with at most one deictic/determiner.
Quotation, negation, conditional scope and explicit new complements are not
reinterpreted as this contextual form. Unsupported constructions remain outside
this grammar; that does not mean the rest of the language pipeline understands them.

AnswerFocusIR keeps an optional source belief ID, never generated answer text as
evidence. Recent focus is used only without intervening new substantive records.
Otherwise the immediately prior turn must identify one proposition. Same-source,
same-turn causal whole/part records are collapsed only for reference selection;
all stored records remain. Event-pronoun resolution applies its existing same-event
subsumption check before counting antecedents, not only after event matching.

ContextualContentTargetIR records the requested slot and selected belief/turn.
Validation rechecks request grammar, target/projection identity, current source
availability, attribution and relation evidence. Ambiguous, retracted or
hypothetical memory cannot silently supply an actual-world causal answer.
Memory eviction invalidates dependent focus. This receipt is not a proof of
unrestricted discourse interpretation or of truth of the user's report.

The operation owns its original argument before generic reference substitution.
Its unresolved gaps are not relabeled using the previous question's words.
Known answers still use existing content projection and realization knowledge;
no case-specific answer sentences or new answer templates were added.

## Evidence and remaining limits

- Adapter library 689 tests and dockable core library 29 tests passed.
- Six existing CLI regression suites, fmt, clippy and API build passed.
- Earlier R suite: 7 full / 2 partial / 2 failed, up from 5 / 3 / 3.
- Original 50-turn development output is unchanged (32 scored: 19 / 6 / 7).
- New frozen V suite: 9 scored requests, 6 full / 2 partial / 1 failed.
  All 17 complete input surfaces were absent from the searched source/scripts
  and prior language reports. This is developer-authored/scored, not independent
  blindness, GPT comparison, or a natural-conversation completion claim.

Explicit Korean targets no longer borrow the previous event's cause in the
tested cases, but the gap realizer still labels them with a stale subject.
An added manner phrase such as "in detail" can defeat contextual retrieval.
Source-side possessives, ambiguity-specific questions, compound Korean endings,
mixed answer/action requests, listening requests and natural gap wording remain
incomplete. Generated causal sentences are still stiff and sometimes preserve
connective fragments in quotes. Passing source attribution is not naturalness.

## Deployment boundary

Current state/turn-response/discourse-answer schemas: 47 / 38 / 3.
Use a new conversation session or an explicitly authorized state migration.
Six Rust files are mirrored into pakage; parity is not an independent package
build. No commit, push, service restart or stored-state migration was performed.
Canonical files, semantic generations and unrelated package changes remain intact.

