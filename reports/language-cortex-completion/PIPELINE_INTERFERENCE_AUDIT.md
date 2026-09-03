# Language Pipeline Interference Audit

## Finding

The main failure is not simply missing lexical rules. The input path often
constructs the relevant typed information and then loses operational access to
it at compatibility projections and mutually exclusive response branches.

## Actual data path

```text
normalizer
  -> reference/QUD resolution
  -> authoritative native-surface selector
  -> native circuit (exactly one surface)
  -> compositional + pragmatic analyzers
  -> LanguageCenter graph and Goal projection
  -> lossless SemanticPlanGoalIR
  -> semantic-core SemanticPlanBundleIR
  -> legacy PlanIR compatibility view
  -> response-mode producers
  -> natural realization
  -> evidence boundary
```

The named modules are not inherently the problem. The problem is that several
of them still behave as local decision owners rather than immutable evidence
producers.

## Confirmed loss and interference points

### 1. Utterance intent collapses before composition

This point is now repaired. `utterance_intent.rs::infer_candidates` emits every
compatible intent contribution before selection. A deterministic precedence
sort chooses exactly one primary response goal, while
the ordered non-primary candidates retain co-occurring demands such as evidence for a
conditional decision. Supporting intents cannot replace the primary GoalIR;
their constraints and evidence are carried into semantic tags and the central
Language Center projection receipt as `PreserveConstraint` decisions.

Subsumption is explicit at the intent boundary. A response-goal correction
that asks for an explanation owns that explanation and does not create a
second generic explanation or problem-disclosure goal. Graph validation proves
that every inferred candidate is classified exactly once as primary or
supporting, and rejects missing, duplicate, or forged candidate references.

### 2. Native and pragmatic paths arbitrate in parallel

`cognitive.rs::process_conversation_turn` formerly let the native circuit and
the reference-resolved compositional path parse different surfaces without a
declared ownership boundary. Making every reference-resolver output globally
authoritative was also incorrect: local anaphora and a merely salient focus
could then rewrite an otherwise complete source or silently resolve a genuinely
ambiguous target.

This boundary is now repaired by `authoritative_native_source`. An
unambiguous, structurally typed cross-turn binding (topic, repeated action,
ordered event, result, or discourse-program binding) may supply the resolved
surface. A no-op resolver, local antecedent, generic focus guess, or
clarification answer does not gain rewrite authority; the original utterance
remains the single native source. Creation, later reconciliation, response
boundary refinement, and response validation all use that same selected
surface.

This boundary is now synchronized by
`PragmaticInterpretationIR::reconcile_native_projection`. The cognitive
orchestrator submits the native record once; the Language Center and its Goal
projection receipt are rebuilt together. Cross-analyzer events align by
predicate and nearest source position, consuming each compositional frame at
most once, so repeated predicates cannot collapse into the first frame.

The dialogue-context input now has an explicit source order as well. Resolved
typed entities and the current discourse focus own identity and salience; raw
native mention memory is used only when those sources are empty. For an entity
that is already selected, raw memory may restore the user's phenotype spelling
but cannot add a candidate. This removes the false ambiguity where incidental
nouns from a prior sentence displaced a unique contextual target.

Response-goal correction now enters the central routing receipt immediately
after pragmatic inference, before lifecycle-query routing or native response
candidate production. Both consumers consult that signal. A native
`AnswerVerifiedResult` guess can therefore no longer mask an explicit “explain
why” correction, and the action-state module cannot independently reclaim the
turn. The Korean/English correction regression validates the final response
binding as well as the selected plan.

### 3. A valid compositional request could be blocked by feedback

The old projection gate required `user_feedback.is_none()`. Any turn containing
feedback could therefore suppress an otherwise valid request. This has been
replaced with typed cross-analyzer agreement: feedback and a request may
coexist when their inferred intent and subject agree; disagreement preserves
the prior discourse target instead of binding a focus word as a new target.

### 4. Typed graphs were flattened into strings

`PragmaticInterpretationIR::apply_to_understanding` converts semantic roles,
quantifier scope, modal scope, relations, and attribution into formatted
strings in `LanguageUnderstandingIR.constraints`. It also overwrites scalar
`intent`, `subject`, and `desired_outcomes`, with early returns for continuation
gates and multi-goal graphs.

This point is now repaired. `LanguageCenterIR::to_semantic_plan_goal` projects
typed events, arguments, roles, scopes, and event relations into
`SemanticPlanGoalIR`. `dockable-semantic-core` creates one checked `PlanIR` per
selected live event and binds them in `SemanticPlanBundleIR`; the first plan is
exposed only as a compatibility view. `LanguageUnderstandingIR` can no longer
select the planner target or erase topology.

The one-shot central materializer's selected candidate set owns the final live
event selection. Earlier `REPORTED` or `SUPPRESSED` module proposals remain in
the Language Center conflict audit but cannot overwrite that selection.
Goal-graph relations are merged with clause-graph relations, and deictic
arguments remain typed reference concepts instead of disappearing as
stopwords. A compound request regression proves three selected events, two
sequence relations, and one prohibited event all reach the semantic planner.

The final selected goal set now also has one owner. When the native circuit has
a complete live-goal set, compositional and pragmatic peers may enrich matching
events but cannot append another selected event. Matching is one-to-one, so
legitimate repeated or multi-goal requests do not collapse. When no native
selection exists, a context-restored pragmatic goal displaces same-intent
surface placeholders instead of becoming an additional plan. This removed the
duplicate realization previously produced by feedback plus elliptical
re-explanation.

Argument ownership now follows the same boundary. Conversational openers are
removed before predicate-role extraction, so a discourse particle such as
`아니`, `음`, or `잠깐` cannot become the first event's Theme. If a prohibited
event then has a genuine argument gap and the native graph provides exactly one
discourse-grounded subject, the Language Center records one typed ellipsis
binding to that subject. Multiple distinct subjects leave the gap unresolved.
The planner and realizer consume that binding and are forbidden to recover a
target from the sentence independently.

### 5. Conversation memory stores actions but not all dialogue propositions

This point is now repaired at the shared-state boundary.
`DialogueDirectiveLedgerIR` is the one bounded, hash-bound owner for ordinary
response preferences, requested formats, interaction policies, and general
dialogue constraints. Language analyzers may submit immutable
`DialogueDirectiveCandidateIR` values, but only `ConversationMemory` may commit
them. The ledger retains exactly one active value per typed axis, marks a
replaced value `SUPERSEDED` instead of erasing it, and rejects semantic or
execution authority even after attacker-controlled rehashing.

The first live producer is existing typed response-length feedback; no new
sentence matcher was added. A Korean “too verbose” assessment becomes the
phenotype-neutral `RESPONSE_LENGTH / ASSISTANT_RESPONSE / CONCISE` value. On an
English follow-up task, that same value reaches the planning compatibility
context, the central response plan, the generation meaning graph, and every
realized sentence's source provenance. Every response act crosses the same
response-plan policy point. A concise value removes only optional affect and
topic-bridge moves; it retains the primary task and current-turn corrective
feedback. The plan-preview generator then consumes the value before expression
selection and builds a smaller typed graph containing the requested action and
the plan/not-executed truth boundary. It does not truncate completed text or
store a canned short sentence. This closes the previous recognize-then-forget
path while leaving wording outside semantic state. Additional response-format
and interaction-policy producers can use the same commit API rather than
creating more private memory modules.

### 6. Response selection was controlled by source-code order

The previous `natural_response_act` logic was one long `if/else` chain. A newly
inserted producer could mask every producer below it, and masked alternatives
were not represented in the final response record.

This point is now repaired. Producers submit candidates to
`NaturalResponseArbitrationIR`; arbitration sorts them independently of call
order, retains selected and suppressed candidates, and binds the result into
`NaturalRealizationIR`. A permutation regression proves identical selection
and hash under candidate reordering.

Temporal, dialogue-relation, and discourse QA gates are now owned by one
monotonic `LanguagePipelineRoutingIR`. It is created immediately after
normalization and accumulates typed `LanguagePipelineSignalIR` facts in a
`BTreeSet`. Specialty analyzers no longer duplicate or privately vary the
peer-module exclusion expression. They cannot remove a prior signal, and
duplicate or reordered contributions are idempotent.

The larger pre-realization overwrite chain has also been removed from the live
path. Definition, action-state, QA, topic, feedback, affect, and interaction
modules no longer draft text before final arbitration. They contribute typed
evidence and `PlanProjectionBlockerIR` values to one order-independent
`PlanProjectionDecisionIR`. Only an allowed decision may materialize a semantic
plan; every surface sentence is produced later by `NaturalRealizationIR`.
Legacy render helpers remain explicitly quarantined and unreachable while old
fixtures are migrated.

### 7. Realization previously reselected a second plan view

Plan-preview realization formerly inspected native, compositional, and
inferred candidates again after semantic planning. That made it another goal
selector and allowed duplicate or stale plans to reappear.

This point is now repaired. Plan realization consumes only
`SemanticPlanGoalIR` plus its checked `SemanticPlanBundleIR`. Every selected
event, prohibited event, selected relation, and response move becomes an
explicit `NaturalRealizationCoverageIR` obligation. Validation rejects omitted,
orphaned, duplicate, wrongly typed, or hash-mismatched evidence.

### 8. Validation previously detected invention better than omission

The validator already rejected unsupported claims, empty promises, IR leaks,
execution claims, and stage overwrites. It now additionally proves complete
coverage against the semantic planner input. Selected-event and relation
obligations must point to traces containing the exact semantic-goal hash and
event grounding; response-move bindings must match the selected act.

## Repair sequence

1. **Completed:** replace response call-order arbitration with one retained,
   order-independent candidate lattice.
2. **Completed:** allow feedback plus task composition only through typed
   cross-analyzer intent/subject agreement.
3. **Completed:** `LanguageCenterIR` owns synchronized native,
   compositional, scope, pragmatic, and illocutionary projection. Specialty QA
   execution gates share one policy.
4. **Completed:** remove pre-realization module render/overwrite control flow.
   One retained blocker receipt now controls plan materialization and one
   candidate arbitration controls response selection.
5. **Completed:** replace scalar/string `LanguageUnderstandingIR -> PlanGoalIR`
   planning input with a lossless typed projection. Keep the old struct only as
   an API compatibility view.
6. **Completed:** make realization consume an explicit semantic-obligation set
   and fail when required input meaning is omitted, not only when output meaning
   is invented.
7. **Completed:** remove the remaining `*_consumes_turn` compatibility-owner
   variables from the conversation pipeline. Definition, group update, native
   goal, action-state, plan-result, continuation, QA, topic, interaction,
   feedback, affect, inform, and guard ownership are accumulated as typed
   signals in one routing receipt. QA, temporal analysis, plan projection,
   memory commit, and response arbitration consume that same receipt rather
   than re-reading module-local booleans.
8. **Completed:** remove the last post-commit plan overwrite. A possible plan is
   retained as a pre-commit candidate while conditional-guard evidence is
   evaluated. After all signals are present, `PlanProjectionDecisionIR` makes
   one final adoption decision. A guard can suppress the candidate through a
   typed blocker, but no module assigns `grounded_response = None` or clears an
   already published plan hash.
9. **Completed:** replace utterance-intent early-return selection with a typed
   primary/supporting contribution set. Exactly one primary may project a
   response goal; every compatible supporting demand survives as a retained
   constraint in the same central projection receipt.
10. **Completed:** move response-goal correction evidence ahead of lifecycle
    and native-answer routing. Those paths now consume the central signal and
    cannot submit a competing result answer for the corrected explanation goal.
11. **Completed:** add one typed dialogue-directive ledger to conversation
    state. Response preferences and general conversational constraints now
    cross the same central commit, supersession, retrieval, planner-context,
    response-plan, generation-meaning, and realization-provenance path instead
    of disappearing in module-local turn state. All response acts cross one
    policy point; concise mode removes only optional affect/topic bridges while
    retaining the primary task and corrective feedback. The active concise
    value also changes the typed plan-preview meaning graph before wording is
    selected; post-generation text truncation remains absent.
12. **Completed:** compile explicit response-length instructions from typed
    lexical atoms already owned by `LanguageKnowledgeBase`. The compiler
    requires a response target, a directive operator, and one compatible value;
    it does not dispatch on complete sentences. Quoted or merely descriptive
    wording cannot promote a directive, and contradictory values fail closed.
    After Native goal selection, the authoritative live-goal set is now the
    only source used to decide whether the utterance also contains a real task;
    lower compositional candidates cannot be re-read to revive a discarded
    response-shaped goal. Directive-only turns therefore update the central
    ledger without projecting or persisting a fake task, while directive-plus-
    task turns preserve the independently selected task. The Native binder now
    also treats an immediately pre-predicate coordinating conjunction as a
    left clause boundary, so an entity owned by the directive clause cannot be
    absorbed into the following action Theme. A fresh frozen 12-case blind
     canary passes 12/12.
13. **Completed:** remove the remaining goal-memory reinterpreters. Conversation
    memory is now projected only from the selected `SemanticPlanGoalIR`; Native
    and compatibility candidates can supply predicate display evidence but
    cannot create or replace remembered goal events. Deferred commitments and
    guarded discourse programs reuse that selected subject identity. Generic
    action-state and plan-result analyzers submit candidates first and receive
    ownership only if temporal/dialogue/discourse QA did not produce a typed
    answer. Likewise, the existence of a stored conditional guard is evidence,
    not response ownership; ownership requires a current-turn typed guard
    evaluation. The response-format directive axis now composes plain, bullet,
    numbered, and table values through the same lexical/ledger pipeline, with a
    fresh frozen 16-case suite passing 16/16.

## Current verification

- central response arbitration permutation test: pass;
- central routing signal permutation/idempotence test: pass;
- conversation ownership compatibility variables: 0;
- post-commit plan/response overwrite assignments: 0;
- response-plan act/signal/policy cross-product: 320/320;
- predicate/argument/discourse matrix: 216/216;
- dockable-semantic-core library regression: 29/29;
- semantic-core-adapters library regression: 592/592;
- typed semantic planner sequence/scope/event regression: pass;
- plan-projection blocker permutation/retention test: pass;
- frozen real-user dialogue blind suite: 16/16 (structural and realization 100%);
- language-cortex integration canary: 12/12;
- fresh GPT-gap structural transfer: 24/24;
- explicit dialogue-directive composition blind suite: 12/12, with zero
  descriptive/quoted promotions, zero committed conflicts, zero fake plans,
  and zero persisted fake goals;
- response-format composition blind suite: 16/16, including four
  directive-plus-task cases with selected-plan/current-turn-memory alignment;
- legacy R59 exact-label diagnostic: 13/16; its three misses are historical
  target/speech-label expectations (`failure` versus the bound object,
  `ASK` versus the current explicit explanation request, and a Korean compound
  topic reduced to its latest member), not a response-integration, authority,
  or stage-overwrite failure;
- full-axis integration diagnostic: 15/16; the sole diagnostic miss is a
  frozen literal-token expectation for `verification` while the verified
  generator intentionally emits the equivalent verb `verify`;
- `cargo fmt --all -- --check`: pass;
- `cargo clippy -p dockable-semantic-core -p semantic-core-adapters --all-targets -- -D warnings`: pass;
- external LLM and teacher calls: zero.

This repair is intentionally architectural. It does not add a sentence-specific
reply rule and it does not treat a higher canary count as proof of general
conversation quality.
