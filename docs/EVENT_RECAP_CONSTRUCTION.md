# Source-grounded event recap

This increment advances observation -> attributed memory -> interpretation ->
response construction. It does not promote semantic concepts or enable recursive
source mutation. It repairs a missing response operation, not a vocabulary deficit.

Previously SUMMARIZE was classified as response content, but QA searched for a
stored Summary slot. PropositionContentIR never populated that slot. A valid event
could therefore produce "no matching record" instead of a recap.

The new path is:

1. Consume a bounded operation + deictic event-argument grammar.
2. Select the immediate unambiguous event, or a still-valid event answer focus.
3. Create EventSummaryIR with event roles, predicate identity, negation and source
   provenance. Replay the source/context compiler to validate the event.
4. Assemble source-attributed clauses through the existing generation graph,
   expression selection, syntax and morphology stages. No stored summary sentence
   or whole-request-to-answer dispatch is consulted.
5. Validate the generated text, claims and event against the live memory record;
   retain the event identity for a subsequent role question.

## Boundaries

- Reported events remain attributed reports, not independently verified facts.
- Negative/hypothetical or revoked evidence cannot silently support a positive
  actual event. Negation, changed roles and payload tampering are checked.
- The recap projection supports one event in Korean or English, with same-language
  arguments. It preserves finite predicate morphology from the source; the
  generation tense is SourcePreserved, not an inferred present/past assignment.
- This is not general summarization. Multiple-event aggregation, explicit topic
  selection, nominal requests, unrestricted paraphrases, passive-to-active tense
  conversion and cross-language event translation are not established.
- Source normalization can lose English proper-name capitalization. This increment
  does not guess proper nouns from title case or count that style defect as solved.
- The unsupported path still uses existing generic gaps in some cases. A safe gap
  is not automatically a useful or natural answer.
- No external action is authorized by a recap. Lexical expressions do not mutate
  promoted concepts. Existing canonical artifacts and package-only edits remain
  unchanged.

## Deployment and evaluation

Conversation state 41 / response 32 require a new session or separately approved
migration. No running service restart, state migration, commit or push is included.
Package source parity does not imply a separate package build.

Development regressions and fresh diagnostic observations are reported separately
under reports/language-cortex-completion. New tests are developer-authored and
developer-scored, not independently blind. Semantic adequacy and conversational
style are separate judgments; failures remain in the report. A recap pass does not
complete the overall natural-dialogue goal.
