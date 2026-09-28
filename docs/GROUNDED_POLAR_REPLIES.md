# Grounded questions and short confirmation replies

The language layer supplies lexical polarity, not facts or execution authority.
`confirmation_polarity` is shared by conversational backchannel classification
and world observation binding. Korean/English confirmation aliases include
응, 네, 그래, 맞아, 맞아요, 그렇습니다, yes and yeah. This is a supplied
controlled lexical inventory, not universal paraphrase understanding. Questions,
uncertain predictions, mixed polarity, and mere acknowledgments such as 알겠어
or okay are not accepted as factual confirmations by this path.

## Meaning before wording

A current-state gap retrieves candidate primitives through the existing sense
index. Only a unique missing state atom with an identified actor and bilingual
expression knowledge can become a polar question. The existing core selects the
observation; the existing world-clause grammar realizes it. No complete request
sentence dispatch or new benefit primitive is introduced.

For this information-acquisition turn, the utterance plan contains one `Ask`
move. It does not first state that the same premise is unknown. This selection
happens before wording; no later module deletes or rewrites an emitted sentence.

`WorldQueryIR.clarification_goal` identifies a distinct pending action question.
Its bounded marker permits asking for that question's missing premise, while
ordinary factual queries retain their no-echo boundary. The marker is not
evidence: it cannot establish a conclusion, introduce a fact or authorize an
action. Normal language queries cannot supply this marker.

The emitted question is committed as `last_query`. A short reply becomes a
premise only when the core can replay the prior observation question and bind
the reply to its requested atom. The binding keeps the original reply, turn,
query and decision digest. Existing semantic-question resumption consumes this
verified update; it does not ask the user to repeat the original question.

Question ownership and lifetime are checked before input preparation. A
completed, canceled, displaced or expired action clarification cannot consume a
later confirmation as a new state fact. Social bridges do not renew the existing
three-turn window. State confirmation never executes the proposed action.

## Delivery and limits

State/response schemas advance to 121/117, utterance intent to 9, discourse answer
to 45 and generative language to 29. No deployed state is migrated, service
restarted, core payload changed or Git publication performed by this change.
Verified scoped mirrors belong in `pakage`; rollback must preserve unrelated
dirty worktree changes and reverse only this delta.

Tests preserve the single-generation/single-final-sentence-check boundary. The
new native suite is a developer regression, not a blind naturalness benchmark.
It covers lexical aliases, polarity, follow-up explanations, acknowledgment,
question-mark ambiguity, cancellation, topic changes, expiry and completion.
Existing native suites and all adapter library tests are also required.

This does not establish human-like general conversation, arbitrary affirmative
paraphrases, sarcasm comprehension, negative-question polarity conventions,
permission handling in general, or new action-benefit knowledge. In particular,
the supplied rest/fatigue relation remains a conditional possibility, never a
guaranteed outcome. Personality and affect cannot change its evidential status.

Evidence: `reports/language-cortex-completion/grounded_polar_replies_2026-09-07.json`.

Subsequent nonfactual replies and question-purpose ownership are documented in
`QUESTION_REPLY_OWNERSHIP.md`. The schema numbers above describe this earlier
checkpoint.
