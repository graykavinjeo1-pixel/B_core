# Answer availability is not discourse expiry

The native failure was not erased memory. After an unanswered question, the
original event and its proposition-derived discourse center remained intact.
Event reference retrieval nevertheless required an observation introduced in
the immediately preceding turn whenever there was no usable event answer focus.

The event resolver now uses the already validated discourse center for both
state and event questions. A proposition center selects event records from its
source turn, rather than demanding a new observation in every conversational
turn. The existing bounded lifetime (16 turns), active/non-denied/actual-world
record checks, source replay and ambiguity handling remain in force.

An explicit topic center restricts event roles to that topic. A newer center
also supersedes an older answer focus; an old successful answer cannot bypass
a subsequent topic change. A task/goal center is not silently reinterpreted as
an observed event. A new observation supplies its own center instead of an
arbitrary globally nearest person. Existing topic-qualified questions retain
their own explicit selection semantics.

This changes retrieval selection, not the stored propositions, the latest
question, user authority or generated output. No new state store, sentence
dispatch, rewrite loop, semantic generation or runtime LLM is introduced.

## Verification boundaries

Positive paths include unknown question, thanks, successful question followed
by unknown question, and Korean personal-pronoun continuation. Negative paths
include explicit topic switch with/without prior answer focus, a newer observed
event and 17 intervening social turns exceeding the existing focus lifetime.
The original tests remain unchanged. Pipeline positives assert one generation,
one final check, unchanged epistemic records and no executable action.

These developer-authored checks are not a blind natural-language benchmark.
General conversation, broad vocabulary and human-like personality realization
are not established by this change. In particular, this does not repair the
previous printed-report lexical/grammar gap.

Schemas: conversation state 112, response 108, discourse answer 39. No automatic
state migration, service restart, deployment, commit or push is performed.
