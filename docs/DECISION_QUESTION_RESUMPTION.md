# Resuming a semantic question after clarification

The missing-state dialogue now retains its original semantic question across a
relevant factual clarification. It no longer has to acknowledge the fact and
wait for the user to ask the original question again. This remains a bounded
dialogue capability, not general human-level conversation or new affordance
knowledge.

## One question, source-bound continuation

`DecisionInquiryIR.resumption` contains an `ActionBenefitResumptionIR`: the
original question and turn, the original gap snapshot, and a source-verified
`WorldMemoryUpdateIR`. The gap must be a current/recent/conflicting-state gap
that actually requested information, not an unsupported-effect notification.
The new premise must concern a property used by an indexed candidate primitive
for the original actor. A different person or an unrelated property cannot fill
the slot.

The world update is replayed from the gap snapshot. The resulting premises,
vocabulary and implications must match the received update. Questions, quoted
text, unsupported facts, mismatched histories and inference-only additions do
not gain authority as clarifications. The input assertion remains in the shared
attributed history; the assessment does not replace its observation record.

Only this receipt licenses `current_clarification` on an assessment/gap. Ordinary
questions still cannot treat their own sentence as a new factual premise. Both
positive and negative source states can resume the judgement: a positive state
may yield the existing conditional benefit; a negative/inapplicable state yields
the corresponding gap, not a fabricated benefit. Successful resumed and reasked
queries use exactly the same core evidence and derivation under equal premises.

The existing decision answer remains the single output owner. It is selected
before a standalone world-update acknowledgement only when the continuation is
validated. No new runtime module, execution grant, source-to-answer dispatch,
output repair pass or semantic primitive is added.

## Focus and explanation

The pending question must still own answer focus. New topics/cancellation do not
resume it; a social or explanation bridge can retain it within three turns of
the original question. These bridges never renew its lifetime. A later `why`
refers to the original question while retaining the clarification's evaluation
turn and source evidence. Corrected evidence invalidates stale reuse.

Scope is intentionally recorded, not relabeled as completion: this version does
not fill an omitted actor from a bare name, resolve arbitrary `yes`/`no` replies,
or resume every other decision-input kind. The existing current-state question
is still coarse, and the phrasing is not yet human-like. Semantic correctness
and a successful regression suite do not prove general naturalness.

## Delivery boundary

Conversation state/response schemas are 120/116, utterance intent 8 and discourse
answer 44. No deployed service or persisted user state was migrated. Core ABI,
canonical files, promoted payloads, source dictionary and action-benefit data
are unchanged. Verified scoped files are mirrored into `pakage` only after
predecessor hash checks. Rollback must reverse this scoped delta without resetting
the shared dirty worktree. No commit, push or deployment is implied.

Evidence: `reports/language-cortex-completion/decision_question_resumption_2026-09-07.json`.

Follow-up: `GROUNDED_POLAR_REPLIES.md` describes the subsequent precise-question
and short-confirmation path. The scope and schemas above document this earlier
checkpoint, not the later extension.
