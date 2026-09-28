# Information gaps are not failed user premises

This engineering change advances interpretation and grounded expression. It does
not establish autonomous concept emergence, general language understanding or
human equivalence.

## Question ownership

A single positive, unquoted, independent non-past method question is recognized
using the existing Manner slot and compositional predicate mood/time. The word
`how` / `어떻게` alone cannot give it an EventOccurred presupposition. Past and
explicit factive questions retain their existing verification path. Unsupported
grammar is not silently declared understood by this helper.

Existing successful content retrieval and recorded-plan method answers retain
precedence. If those sources do not answer the question, the normal absent-answer
result remains NoMatchingRecord, not a fabricated answer or execution report.

## Realizing the gap

An unanswered single requested relation can be expressed as an unknown content
slot: method, reason, time and the other existing ContentSlotIR identities.
Known-value realization and unknown-value realization share bilingual slot
vocabulary. The epistemic predicate and register-sensitive endings reuse the
existing unknown-property construction. This changes how absence is expressed,
not what counts as evidence or a successful answer.

An explicitly owned inner question or nominal target must remain in the reply
when it distinguishes the request from an earlier topic. Its source-bound
argument is retained; mutable retrieval hints cannot replace it. A quoted
question is a query reference, never a world fact. If there is no such explicit
argument, a brief slot answer may rely on the current question under discussion.
Multiple unknown slots are not collapsed into an arbitrarily chosen single one.

All generation occurs from meaning, expression and grammar nodes before the
single final sentence check. No output text is reparsed, rewritten or regenerated.
No semantic concept payload, dictionary, execution permission or recursive
improvement policy changes.

Schemas: DiscourseAnswer 35, GenerativeLanguage 24, conversation response 103.
No persisted state migration, deployment, commit or push is performed.

## Coupled regression boundaries

A repair is not accepted solely because its motivating example improves. The
same run checks the opposing paths below; an existing assertion is not weakened
to accommodate a new output.

| Intended change | Behavior that must remain unchanged |
|---|---|
| Non-past method question can be an information gap | Explicit past/factive premise checks remain available |
| Follow-up can explain a recorded plan | A third party cannot inherit that plan; a retired plan cannot resume |
| Missing relation is expressed concisely | An explicit inner question or new nominal target retains its identity |
| Answer construction uses bounded stack storage | JSON meaning, evidence, one generation and one final check stay unchanged |

`scripts/verify_information_gap_boundary.ps1` exercises these boundaries through
the actual native executable, both with and without prior dialogue. This is a
developer regression diagnostic, not a blind benchmark or a naturalness score.
The adapter library tests and existing dialogue-continuity/event-reference CLI
regressions must also pass on the same source revision.

The native debug executable exposed a stack overflow that the library test
threads did not. Optional recorded-plan detail is now boxed (the optional field
uses 8 rather than 416 inline bytes on this host). That alone fixed the fresh
query but did not fix the query with prior context. Request-envelope dispatch is
also separated from record-query construction, so processing an inner question
does not retain an unused outer record-query frame and its large temporaries.
The native regression now covers both cases. No stack-limit increase, retry
thread, output rewrite, or bypassed validation was introduced. This is not a
proof of a stack bound for arbitrary nesting.

## Limits

This does not supply missing knowledge or guarantee natural wording for every
slot and context. Unsupported predicates, complex nested questions, unresolved
references and broader pragmatic inference remain separate limitations. A
concise unknown answer is not counted as successful question answering.
In particular, the factive-control diagnostic retains its unverified-premise
boundary but can still produce inappropriate execution-receipt wording. That
pre-existing expression/route limitation is not counted as repaired or natural.
