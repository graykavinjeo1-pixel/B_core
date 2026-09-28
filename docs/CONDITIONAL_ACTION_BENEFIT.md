# Conditional action-benefit reasoning

Implemented and regression-tested on 2026-09-07. Natural conversation as a whole
is not established. This is one supplied affordance, not autonomous discovery.

## Executable product path

The existing decision-request owner can now resolve its expected-benefit input:

`proposal roles + attributed state -> conditional core request -> core derivation -> possible-benefit meaning -> realization`

`ActionBenefitAssessmentIR` retains the source world snapshot, evaluated turn,
actor, supplied primitive ID, `DeliberationRequestIR` and `DeliberationIR`.
The adapter compiles two distinct premises: the user's stated condition and an
explicit hypothetical action-sense interpretation. A supplied inference relates
them to a possible-benefit proposition. The existing core deliberator must reach
that proposition. No recovery, actual state transition or external execution is
inferred. The conclusion is not written into the world or action ledger.

The first hash-checked engineering primitive records NIKL entry 71280, sense 1:
rest is directed at fatigue relief. Semantic fields live in
`action-benefit-primitives.json`; sense bindings and Korean/English expressions
live separately in `action-benefit-language.json`. Core inference consumes
neither the source sentence nor those labels. The supplied synonym link from
NIKL 88442/1 (휴식하다) joins the same primitive without copying its semantics.
The same action meaning is not a separately copied Korean and English concept.
This is an explicitly supplied, defeasible possibility relation, NOT a proved
medical effect, guaranteed outcome, promoted concept or discovered law. Only
this affordance is supplied at present. Other aliases/affordances still
need grounded knowledge; no universal action-effect coverage is claimed.

The source predicate and positive/negative polarity must match. Conflicting
active premises, a different actor, action negation, transitive uses, stale
premises or an incompatible/unknown frame cannot use this primitive. An omitted
actor can bind only in a recent explicitly continuing self-state context. The
system does not infer an actor from a third party's condition. These are bounded
applicability conditions, not a general reference-resolution solution.

## One owner and one output

The optional assessment stays inside the existing `DecisionInquiryIR` receipt
(a historical type name for the decision-input interaction). It is not a second
competing dialogue route. When available, disposition is an answered conditional
assessment; otherwise the existing information-gap behavior remains. The query,
world facts, execution authority and personality are not overwritten.

Realization composes separate action, beneficiary and effect nodes. It states
both the interpretation condition and possibility. A reason follow-up retains
the original core derivation but selects a purpose/basis clause, not the same
conclusion again. Corrected premises invalidate reuse. Formal and informal
outputs differ while the assessment remains identical. Generation and the
final sentence check each occur once; independent IR/source replay is separate
and still has a measured cost.

## Verification and circular-defect evidence

- All 900 adapter-library tests pass, including the unchanged predecessor tests.
- New tests cover actual core contribution (mechanism ablation), forged receipts,
  actor/polarity/sense/conflict exclusions, unchanged world facts and action
  ledgers, source correction, register invariance, and one generation/check.
- The new native suite passes 7 conversations / 19 turns. Four predecessor native
  suites pass 229 further turns. These are developer regressions, not blind
  naturalness benchmarks.
- The added alias suite passes 6 conversations / 16 turns (264 combined native
  turns). Korean 쉬다 / 휴식하다 and English rest produce exactly equal core
  requests and derivations under equal source premises, including explanation
  follow-ups. Expression renaming/removal preserves serialized semantic fields.
- The first native candidate reached the possible-benefit goal but repeated its
  conclusion when asked why. That output is retained in the report as a failed
  explanation boundary, despite passing the earlier narrower assertions. The
  final candidate adds purpose/basis realization and strengthened assertions.
- Formatting, warning-free library Clippy, offline tests and native build pass.

The primitive collection is currently one record, bounded at 32, with checked
identity/hash. Selection uses a sense-keyed index and a primitive-ID index;
only grounded compatible frames supply lookup keys. Multiple aliases for the
same primitive are deduplicated before hypothesis evaluation, never counted as
extra evidence. Tests add 31 irrelevant semantic records without changing the
selected set. This protects indexed selection, not a large-scale latency claim.
The semantic-field projection hash before and after physical language separation
is identical. Both data files are hash-checked; unresolved sense/primitive links
and duplicates are rejected. Missing aliases do not delete a semantic record.

## Remaining limits and delivery

### Self-deliberative modal boundary

English `should + I + lexical infinitive` now enters the same proposal role,
sense and conditional assessment path. The explicit grammatical subject binds
Agent; it is not inferred from whoever was previously tired. Local negation and
object/prepositional roles use the shared event grammar. No proposal becomes an
observed event or execution grant. The previous evaluative envelope retains the
same infinitival checks. There is no verb-specific sentence dispatch or new
action-effect knowledge in this change.

The predecessor executable failed 8 of 19 new native boundary turns: the modal
was incorrectly treated as dialogue-record retrieval. The repaired executable
passes all 19, including the original envelope, capitalization, reason follow-up,
negation, different-actor state, unsupported frame, unknown effect and historical
question controls. Equivalent supported proposals produce exactly equal core
requests and derivations. Two added library tests protect role/source replay,
non-execution, unchanged world premises and one generation/final check. The
existing tests and their expectations remain intact; this is a routing/semantic
regression gate, not a blind naturalness score.

This bounded addition does not cover arbitrary modal subjects, perfect/progressive
auxiliary constructions, Korean self-deliberative inflections or compound
conditional proposals. Existing unknown-effect responses still ask a stiff
expected-benefit question; accepting the correct speech act does not establish
natural response quality. Evidence:
`reports/language-cortex-completion/deliberative_modal_boundary_2026-09-07.json`.

This now answers one known class instead of always requesting its benefit, but
the wording remains cautious/stiff. General benefit trade-offs, competing goals,
costs, broader action effects, natural ambiguity resolution and unrestricted
conversation are not complete. No general naturalness or GPT-equivalence claim.

Schemas advance to utterance intent 6, conversation state 118, conversation
response 114, discourse answer 42 and generative language 27. No deployed service
or serialized user state was migrated. Canonical files, the source dictionary,
promoted semantic payloads and unrelated package edits are unchanged.
Only verified scoped files are copied into `pakage`, after predecessor-hash
checks. No commit, push, deployment, restart or autonomous source mutation.
Rollback must reverse this scoped delta without resetting the dirty worktree.

Evidence: `reports/language-cortex-completion/action_benefit_semantic_aliases_2026-09-07.json`.
The initial reasoning/pilot failure remains in
`reports/language-cortex-completion/conditional_action_benefit_2026-09-07.json`.
