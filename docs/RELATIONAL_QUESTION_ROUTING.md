# Requested relations survive absent evidence

This change repairs final response selection in the existing Rust dialogue path.
It does not add an output rewriter, sentence examples to runtime, another response
owner, or a new semantic concept generation.

## Failure and authority

The query backend could retain the correct question and an explicit unknown or
unverified-premise answer while the native result heuristic noticed a word such
as `failed`, `repaired` or `verified`. Final arbitration then selected a generic
execution-receipt answer. A missing method, actor or time was incorrectly treated
as a missing execution result. This also made an ordinary person appear to be a
host execution receipt in the output.

The central answer selection now consumes the existing requested ContentSlotIR
relations: agent, recipient, source, location, time, duration, cause, manner,
intention and condition. Under an information-request contract, these obligations
retain information-answer priority even when the answer value is missing.
This generalizes the prior cause-only preservation at the same ownership point.
It does not replace arbitration precedence values or alter question parsing.

Outcome-content and truth/status requests are not reclassified by this rule.
They retain lifecycle verification. A premise check can remain unverified; a
source report still cannot establish an event as true. Existing successful
content projections, required reference clarifications, prohibitions and ledger
authority are unchanged. Personality still operates after content selection and
cannot invent an answer or verification evidence.

## Coupled checks

- Relation question with outcome/status vocabulary -> relation answer or gap.
- Actual verified result / success-versus-failure question -> lifecycle answer.
- Past/factive premise check -> its discourse answer, not merely a preserved
  internal flag beneath an unrelated final output.
- Known content and multi-turn reference regressions remain in the same run.
- One generation and one final sentence check are asserted before independent
  replay validation; no final-output repair cycle is introduced.

`scripts/verify_information_gap_boundary.ps1` now checks the selected final act
for its premise controls, not only the internal query flag. The earlier sealed
report remains unchanged; its stated limitation is the pre-repair baseline.

Conversation response schema: 104. No state migration, service deployment, git
commit or push is performed. Only scoped files are mirrored to `pakage`.

## Scope of evidence

Developer tests demonstrate this ownership boundary, not unrestricted natural
conversation. The native lexical heuristic remains broad; central arbitration
must still honor explicit semantic obligations. Missing knowledge, unsupported
event descriptions and verbose premise wording remain limitations. A correctly
expressed unknown answer is not a successfully answered question.

The diagnostic `Mina repaired a file in the library.` followed by a location
question still returns an unknown location: generic repaired-event memory/retrieval
is not established by this routing repair. The Korean outcome-alternative control
also asks for an unnecessary target in both the pre-repair and repaired path;
that retained baseline is not counted as successful natural conversation.
