# Open world arguments and reference replies

The world-language adapter preserves a recognized query predicate even when one
entity role is unbound. This extends the existing reference-gap path; there is
no additional dialogue response owner or answer lookup table.

`WorldArgumentGapIR` contains the predicate identity, known subject/object,
missing role, polarity and explanation intent. It is not a complete world
query and cannot establish a fact. Recognition is replayed against the recorded
source, lexical revision and discourse context. A temporary parser probe is
not stored as an entity in the partial IR or submitted to the core deliberator.

The final realization layer asks for an identity using reference-question
grammar. It retains the same one-generation/one-final-check production path.
Existing two-candidate ambiguity uses its original alternative question.

An accepted identity reply completes the original query; it does not assert its
truth. The core then evaluates the query against actual premises. A subsequent
yes/no reply to a core-requested observation is a separate evidence operation.
Polarity and known arguments remain unchanged across reference binding.

The open-identity reply grammar accepts explicit self references, quoted
identifiers and Korean nominal identity endings. It also accepts a single bare
nominal/unknown identifier while an open identity role is pending. This is a
contextual identity interpretation, not a new world fact or autonomous concept.
The shared discourse lexicon and goal-withdrawal detector exclude social and
control replies. Sparse lexical POS evidence rejects verbs, adjectives,
adverbs and ambiguous forms; known Korean noun/하다-verb derivations require
explicit naming. Unknown predicates, multiple open arguments and quoted
questions remain outside this path. Unknown names and unknown commands can
still be linguistically ambiguous; this is not unrestricted entity recognition.
An unknown identifier with a recognized finite-ending shape is also ambiguous
and cannot fill the role merely because its lemma is missing from the dictionary.

Productive Korean finite morphology is resolved against the existing sparse
lemma index when dictionary principal forms omit it. The current additions are
the 겠 modal series and 하다 contractions. An ending without a matching lexical
lemma supplies no new meaning. Source entries, bilingual senses, concepts and
catalog size remain unchanged; each inverse-morphology index probe is counted.

## Questions about the clarification

A pending reference clarification owns its original query until it is resolved,
cancelled or replaced. A bounded causal interrogative about that dialogue act
now creates `WorldClarificationFollowupIR`, retaining the original gap and recording
the current request and turn separately. This is not a world query or a premise.
The final realizer expresses the actual missing-reference requirement. It does
not generate a draft during answer construction or source validation.

The current grammar covers bare causal WH and causal WH plus an asking predicate
in Korean and English. Explicit new world topics and commands do not match.
This does not claim unrestricted metadiscourse understanding.

The same followup record distinguishes `ReasonRequest`, `UnknownAnswer` and
`DeclinedAnswer`. A current-speaker, present epistemic abstention or refusal to
answer retains the pending gap without adding facts or repeating the question.
The response expresses that answering the clarification is optional. Cancellation
still removes the pending gap; an explicit identity can resume the original
query afterward only while that gap remains available.

Korean recognition combines source-linked predicate lemmas, finite morphology,
speaker scope and modifiers. English recognition composes speaker, negation and
epistemic/volitional predicates. Past reports, third-party reports, explicit
new content complements, quoted statements and prohibitions are not silently
current-speaker abstentions. The original narrow reason-only field was replaced,
not retained as a competing response owner. Followup source and kind are replayed
during validation, without generating a sentence.

This remains controlled grammar. Unrestricted indirect refusals, arbitrary
embedded clauses, spacing repairs and ambiguous topic switches are not established.
The existing `AnswerFocusIR` now retains the emitted clarification act, not its
sentence: request identity, explain the requirement, allow abstention, or explain
the interlocutor's choice. Its gap identity includes the full source/context/turn.
Abstention evidence is a bounded leaf; repeats do not accumulate response history.
World state and answer focus commit atomically through one memory owner.

Bare causal questions use that recent act (up to three intervening turns), while
explicit asking-predicate questions refer to the missing-reference requirement.
Existing reformulation grammar can re-express the latest act. Cancellation,
replacement, stale and cross-gap references cannot reuse its authority. An empty
explanation target alone is not treated as a complete clarification followup.
Source and state validation do not generate drafts; final realization still runs
once, followed by one completed-sentence check. The tests demonstrate these
bounded transitions, not unrestricted metadiscourse or natural conversation.

Version identifiers: discourse 23, state 93, response 84, generation 14,
natural realization 29, utterance intent 2 and lexical lookup 3. No persisted-state migration or
deployment is performed.
