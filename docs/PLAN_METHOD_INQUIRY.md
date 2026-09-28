# Recorded plan method inquiries

This product change advances grounded interpretation, memory and expression.
It does not claim autonomous concept emergence or human-equivalent language.

## Ownership

The conversation API retains the validated semantic goal, plan bundle and their
projected dialogue goal identities after an accepted planning response. It stores
no output sentence and performs no new planning or execution when reading them.
One bundle is retained per live conversation; eviction follows conversation
memory. This cache is local to the API instance, not a persisted-state migration.

A method question may read that bundle only when the existing composition has
one positive, unquoted, independent interrogative predicate, requests the Manner
slot, is not past-tense, and identifies exactly one matching recorded predicate
and target. Nominal identity comes from the existing semantic role graph rather
than raw frame text (articles are not part of an object's identity). Explicit
third-party agents and unhandled roles cannot be replaced by the assistant's
plan. Questions with quantifier scopes are not handled by this first binding
path; nominal normalization must not silently discard their constraints.
The same nominal normalizer is applied to the recorded target as to the query.
An omitted target is usable only under unique matching. The selected
goal must still exist in the same authoritative conversation state. Other subjects,
past execution questions, unrelated predicates and ambiguous matches cannot
obtain an answer from this path. Recognizing a method question never authorizes
an external action or re-registers an earlier goal.

`PlanMethodAnswerIR` carries the recorded semantic planning product and selected
index. Validation checks the retained bundle, predicate/target identity and
question binding. Response validation also checks conversation identity and the
current active goal. A
recorded plan is evidence of a proposal, not evidence of execution or success.

## Realization

The method view is an explicitly partial summary of operations actually present
in PlanIR: current-state observation, competing hypotheses, diagnostic testing,
candidate validation, selected action application, explanation construction and
outcome verification. Other internal scheduler operations remain in the IR
but are not expanded in this compact view. The summary does not invent concrete
commands, tool parameters, diagnostics or outcomes absent from the plan.

Each expressed event keeps its plan hash, step ID and operation as grounding.
Human-supplied bilingual lexical knowledge maps those operations to words and
roles. Expression metadata does not mutate the semantic substrate or count as a
new autonomous generation. The existing meaning-to-speech pipeline selects the
register and realizes the graph once.

Compatible DescribePlan predicates joined by explicit semantic Sequence edges
share one plan frame. Korean uses connective verb morphology and one final
nominal complement; English uses coordinated infinitives. Different speech
acts, explicit agents, negation or unsupported morphology break this grouping.
This happens during original morphological construction, before the single
final sentence check. There is no output-string rewrite or regeneration loop.

Schemas: DiscourseAnswer 34, GenerativeLanguage 23, conversation response 102.

## Limits and release boundary

The compact method view is not a detailed executable procedure. New predicates,
multiple questions, rich ellipsis, post-restart plan recall and general free
conversation are not established here. Personality currently affects register
through the existing policy; this change does not establish broad emotional
variation or human cognition. No service deployment, state migration, commit,
push, core semantic promotion or recursive source mutation is performed.
