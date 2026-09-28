# Decision evidence gaps

Checkpoint note: the later `DECISION_QUESTION_RESUMPTION.md` adds source-bound
resumption for state clarifications. The non-resumption limit below describes
this earlier gap-classification checkpoint; arbitrary input filling remains
unimplemented.

The decision pipeline distinguishes a missing user-supplied input from a limit
of the core's currently available action-effect knowledge. Neither means that
the action is objectively useless. This is bounded engineering, not autonomous
discovery or a claim of general conversational competence.

## Shared evidence owner

`world_dialogue::action_benefit_basis` performs the applicability checks used
both by successful conditional assessments and by unsuccessful assessments.
It returns either the actor, supplied primitive and source premise, or a typed
failure reason. The realization layer does not independently infer that reason
from the utterance, a failed answer string or an exception list.

`ActionBenefitGapIR` retains the world snapshot, original evaluation turn and
reason. `DecisionInquiryIR.knowledge_gap` is mutually exclusive with its
assessment. Source replay validates the diagnostic against the same world and
original proposal. A reason follow-up preserves the original continuation flag:
asking why cannot retroactively supply the original proposal's omitted actor.
New or corrected world premises invalidate reuse of a stale diagnostic, just as
they invalidate a stale assessment. No gap becomes a world fact or an execution
grant.

## Response policy

Actor, current-state, recent-state and conflicting-state gaps request the
corresponding information. Unknown effects, unsupported action structures,
negated-action effects, inapplicable state conditions and competing effect
interpretations instead disclose the current judgement limit. They do not
automatically ask the user to supply an expected benefit the core could then
treat as established evidence.

The semantic diagnostic selects a clarification or an epistemic-limit speech
act. Korean/English expressions name the missing information; shared clause
grammar composes the source action mention, epistemic force, question/reason
form and register. Generation and the final sentence check remain single-pass.
Independent IR/source validation remains separate from that count.

`ActionRoles` denotes the present affordance engine's Agent-only applicability
scope; it does NOT claim that a grammatical transitive action is invalid.
`InapplicableState` means the available primitive does not justify this state,
not that no other benefit exists. If primitive selection succeeds but core
inference subsequently fails, this diagnostic does not invent a failure reason;
the existing unresolved path remains.

## Circular-regression controls and limits

Old assertions remain unchanged. New tests exercise the same proposal before a
state is known, after relevant evidence is supplied, and after it is corrected.
Formal/informal policies must preserve exactly the same diagnostic receipts.
Corrupting a reason must fail source replay. Known-benefit paths must retain
their existing semantic requests/derivations and non-execution boundaries.

The first candidate passed its new diagnostic tests but failed one predecessor
Korean explanation check: it emitted an `알아야` formulation where the old
necessary-information explanation contract requires `필요해서`. The failure is
retained, not discarded. The common explanation grammar now expresses the
selected information-requirement meaning; no input sentence dispatch or old
assertion change was added.

The questions are still coarse: a current-state gap does not yet name its exact
missing property in the output, and a supplied clarification does not itself
resume the previous proposal automatically. Free-form goal completion, broader
affordances, unrestricted ambiguity resolution and human-like naturalness are
not established. The tests are developer regressions, not blind benchmarks.

Conversation state/response schemas are 119/115, discourse answer 43, utterance
intent 7, generative language 28. No deployed service or serialized user state
was migrated. The core ABI, promoted payloads, source dictionary and supplied
affordance/language data are unchanged. Only scoped, verified files are mirrored
into `pakage` after predecessor hash checks. Rollback must reverse this scoped
delta without resetting the shared dirty worktree. No commit/push is implied.

Evidence: `reports/language-cortex-completion/decision_epistemic_gaps_2026-09-07.json`.
