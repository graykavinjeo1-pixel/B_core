# Conversation pipeline repair (2026-09-04)

Baseline: ff0a25feba08f05e29a0e2100dde4351d2f349ef. Preserve the preceding
diagnostic and historical passing reports; their structural scores were not
answer-completeness scores.

## Deliverables and order

1. Separate communicative obligations from executable/planning predicates.
   An information question cannot become a report or an action merely because
   its complement contains an action verb. An explanation request needs answer
   content or an explicit knowledge gap, never a promise to explain.
2. Repair reference/QA routing, state snapshot consistency, and unsupported
   response fallbacks. Reuse typed evidence; do not promote reported facts to
   verified facts. Retain ambiguity and missing knowledge explicitly.
3. Add a bounded affective-pragmatic field with inspectable evidence, decay,
   confidence and no semantic/execution authority. It conditions realization,
   not facts, reference identity, or action permission. No response-time signal
   is invented when the API has no timing observation.
4. Exercise the finite route model and real API separately. The route matrix is
   17 categories x 2^5 context dimensions (544 cells), including adversarial
   competing signals. These are routing states, not 544 proven linguistic
   interpretations. This is not exhaustive natural language.
5. Verify Rust formatting, lint, tests, root/package parity, and preserve the
   diagnostic outcomes, including remaining knowledge/grammar limitations.

## Acceptance

- Questions and explanation-only requests cannot emit PlanPreview or create
  action records from question predicates.
- A known answer must carry its actual evidence; a missing answer is a gap,
  not a successful answer. Unknown and ambiguous remain separate outcomes.
- Turn state and response validation use the same committed snapshot.
- Unsupported fallbacks cannot panic or invent illocutionary evidence.
- Tone changes cannot change semantic claims, references, goals, or permissions.
- Affect is a heuristic estimate, not a calibrated psychological diagnosis.
- Existing unrelated package edits, canonical files, and sealed evidence remain
  untouched. No automatic commit/push or autonomous source mutation.

Node storage continues to separate language-independent semantic payloads from
linked language expression/grammar nodes. Adding aliases is not concept learning.

## Implemented boundary

`ConversationContractIR` separates information/explanation obligations, reports
and independent action requests before planning. Later response-mode refinement
cannot turn an information request into a new action. `request_semantics` retains
the interpreted GoalIR without claiming a plan or execution was produced.

The existing epistemic ledger now retains source-bound proposition slots.
Agent/theme use semantic role edges; explicit causal/definition complements use
connective grammar. Retrieval filters world, polarity, active status and event
predicate, and abstains when the projection is not unique. Realization consumes
the selected slot and attribution through expression nodes and syntax, not a
whole-sentence response lookup. Reported content is not verified world truth.

The 11-axis affect field is signed fixed-point [-1000,1000], with bounded cue
evidence, turn decay and tone-change magnitude. Confidence is a heuristic,
not a calibrated probability or diagnosis. The implementation handles a small
inspectable cue vocabulary, position, repetition, exclamation density, polite
endings and limited negation. Token count is recorded, not treated as emotion.
Response timing is unknown in the current conversation API; no time is invented.
Double/curly quotes and code spans are masked. General quotation/report scope,
sarcasm and arbitrary morphological variants are not solved.

Affect reaches morphology through `AffectiveRealizationPolicyIR`. Formal register
is exercised end-to-end. Playfulness now selects a traceable grammar marker for
greeting/gratitude-only graphs, suppressed by urgency, brevity pressure or formal
register. It does not decorate factual answers, refusals or action plans. This
is a narrow social style choice, not general humor or emotional intelligence.

Brevity pressure now enters the existing response-plan composer. Explicit length
directives take precedence; only optional affect/topic bridges are omitted.
Corrective feedback and the complete primary task/answer obligations remain.
No generated string is truncated. Meaning, speech intent, syntax roles, source
and execution authority are not writable through the affect interface.
Generation validation recomputes morphology from its graph/context: relabeling
an invented token as grammar and rehashing is insufficient to pass validation.

`ConversationStateIR.answer_focus` stores one typed prior question, never an
answer string. A bounded compositional re-expression grammar (operation,
reference, repetition, manner) binds requests such as “핵심만 다시 설명해” or
“Explain that again briefly”. The QA engine queries the current evidence anew;
removing the evidence removes the answer. New content words, quoted requests,
negation and mixed actions reject this binding. Non-answer turns invalidate
focus except short social backchannels (maximum three intervening turns).
This is not arbitrary follow-up reasoning or automatic summary generation.

Conversation response schema is now `B_CORE_CONVERSATION_TURN_RESPONSE_19` and
generation schema is `B_CORE_GENERATIVE_LANGUAGE_IR_2`. Conversation state is
`B_CORE_CONVERSATION_STATE_29` (additive answer focus and updated state hash);
generation emotion accepts `PLAYFUL`. Consumers validating exact schemas/enums
must update and recompute persisted-state hashes or start a new conversation.
There is no automatic migration of sealed historical snapshots or reports.

## Evaluation interpretation

- `pipeline_route_tests`: finite 544-state routing arbitration (both candidate
  orders), 17-category real-API smoke coverage, known source/role answers,
  missing-answer boundaries, affect invariance and payload tampering.
- Existing explanation tests previously required PlanPreview. Those assertions
  now require an answer/gap while preserving interpreted subject, prohibitions,
  references and lifecycle checks. This is a contract migration, not evidence
  that missing answers became correct answers.
- Actual dialogue outputs and remaining failures are recorded separately in
  `reports/language-cortex-completion/pipeline_repair_2026-09-04.json`.
- The subsequent realization/focus repair is recorded in
  `reports/language-cortex-completion/realization_followup_2026-09-04.json`:
  1,152 root library tests and 637 package library tests pass; the 15-turn CLI
  diagnostic includes five source-attributed answers and three explicit gaps.
  These counts are not unrestricted-language accuracy. Two previous optional-
  affect fixtures now explicitly request detail; their original composition
  assertions remain, testing that user instructions override inferred brevity.
  Existing optional pytest-dependent research checks were unavailable on this
  host; Rust test success does not imply those Python integrations were tested.

## Still out of scope / incomplete

This is not GPT-level general understanding or a complete world-knowledge QA
system. Unknown general definitions, arbitrary ellipsis, multi-event ambiguity,
causal interpretation of ambiguous Korean -서, and explanations of the system's
previous decision remain limited. A gap/clarification is not scored as a correct
answer. Matching an answer to a complex discourse topic remains conservative
and bounded; this work does not establish generalization to unrestricted text.
The category smoke suite also exposes coarse acknowledgement for agreement/
disagreement and some fillers, and incomplete topic-transition understanding.
Those are not counted as successful conversational understanding merely because
the response passes structural validation.
