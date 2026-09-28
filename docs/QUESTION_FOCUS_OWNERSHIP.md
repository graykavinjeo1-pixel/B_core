# Question content and discourse focus ownership

This Rust adapter change repairs input interpretation and topic ownership. It
does not create a second response owner or alter promoted concept payloads.

## Ownership rules

- A target-free explanation operation can bind the currently focused world
  proposition. Its query asks for explanation; it does not assert a cause.
  The same grammatical missing-target recognizer is used by world grounding
  and clarification. Unknown content is an explicit target, never an empty slot.
- A new explicit explanation target, negated request or unsupported topic cannot
  inherit the old world query. The existing context-clear boundary remains.
- A bare English deliberative light verb with an unfilled action complement can
  continue context without a transition marker. A lexical verb or supplied
  complement does not receive this implicit continuation classification.
- In a missing-answer fallback, existing query content is preserved. Current
  query content terms take precedence over a prior inferred operation/subject.
- For feedback followed solely by grammatical answer-reformulation clauses,
  retain the previous answer topic. Feedback clause evidence identifies the
  evaluation; a remaining explicit content question is not a reformulation.

## Product path and evidence

Input -> operation/target binding -> source-bound WorldQuery or information gap
-> existing reasoning -> one final language realization -> one final sentence
validation. Explanation reuses the focused proposition and existing proof;
it never fabricates missing causal knowledge.

Tests exercise Korean/English continuation, opaque registered predicates,
missing focus, explicit new targets, negation, feedback/rephrasing, current
question ownership, serialization/source checks and production generation counts.
They are author-written regression tests, not a blind natural-language benchmark.

Discourse schema 17, conversation state 86 and response 77 identify the change.
No persisted state migration, service deployment, commit or push is performed.
The package mirror is updated only for the files changed in this increment.

## Remaining limits

This does not establish unrestricted natural conversation. In particular, a
recognized state word without a referent can still receive an information-gap
reply instead of a precise missing-experiencer question. General advice still
requires causal/affordance knowledge; a decision-input question is not advice.

The subsequent [open-argument increment](OPEN_WORLD_ARGUMENTS.md) addresses one
missing entity role with a typed clarification and reference-reply binding. The
limits above describe the focus-ownership increment, not a claim that this later
capability is absent. General advice and unrestricted conversation remain open.
