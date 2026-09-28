# Nonfactual replies belong to the active question

This extends the existing decision-inquiry owner. It does not add an alternate
response pipeline, a sentence lookup table or another final-output repair pass.

## Product path

The prior requested information, source question and turn are retained in a flat
`DecisionClarificationReplyIR`. Shared clarification grammar distinguishes a
purpose request, restatement, unknown answer and declined answer. A reply does
not become evidence about the requested world property merely because it is
recognized as a conversational act.

The optional receipt is heap-backed (`Option<Box<...>>`), so its source text and
abstention metadata do not enlarge every stack-resident decision/query/response
envelope. The initial inline version passed library tests but overflowed the
native main stack on a retained information-gap path. The failure is retained;
neither the process stack limit nor that regression's assertions are changed.

The existing information-gap snapshot must validate against the current world.
New topics, cancellation, expired focus and changed evidence do not inherit the
old question. Followups retain its original three-turn lifetime rather than
starting a new one. Unknown/declined replies close the polar observation slot;
a subsequent bare yes/no cannot invent a state fact.

Restatement reuses the original requested semantic atom and existing clause
grammar. A following factual confirmation can resume the original decision.
Unknown/declined replies reuse the existing optional-response expression
knowledge. An explicit asking predicate in a reason request refers to the
original question's information requirement. Bare why after an optional-response
act refers to that latest response-choice act. This distinction is chosen in IR
before realization, not by rewriting the output.

The shared Korean grammar also composes communication predicates with negated
volition, and recognizes past asking predicates in bounded causal questions.
It rejects third-party statements, quoted or compound utterances and unrelated
content complements in this controlled reply path. This is supplied linguistic
knowledge, not autonomous semantic discovery.

## Verification and limits

`verify_question_replies.ps1` runs 13 developer cases / 45 turns through the real
executable. It checks reply kind, original question identity and turn, purpose
target, zero invented facts/actions, continued reasoning, and closed-question
boundaries. All predecessor native suites and adapter library tests must also
pass without weakening their assertions. These are regressions, not a blind
naturalness benchmark. The mixed open-dialogue probe is preserved separately,
including unresolved failures rather than excluding them from the goal.

In particular, this change does not solve general conversational hypotheticals
such as asking what the assistant would do, nor restore every decision after a
later evidence correction. Broad natural conversation remains unestablished.
Personality/emotion may vary expression but not factual or action authority.

State/response schemas are 122/118, utterance intent 10, discourse answer 46 and
generative language 30. No deployment, persisted-state migration, core payload
change, Git commit or push is performed by this checkpoint. Scoped mirrors are
copied only after predecessor hash checks. Rollback reverses only this change,
preserving unrelated changes in the shared worktree.

Evidence: `reports/language-cortex-completion/question_reply_ownership_2026-09-07.json`.
