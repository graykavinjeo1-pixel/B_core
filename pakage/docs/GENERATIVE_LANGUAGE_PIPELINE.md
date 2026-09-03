# Generative Language Cortex

The canonical generation path is:

```text
language-independent meaning graph
  -> speech-intent graph
  -> discourse plan
  -> language-specific expression-node selection
  -> syntax plan
  -> morphological realization
  -> semantic coverage and round-trip verification
```

The complementary input path is now:

```text
surface evidence
  -> stored lexeme and construction activation
  -> typed reference/QUD resolution
  -> one authoritative parser-surface decision
  -> typed entities and scoped events
  -> discourse relations and reference bindings
  -> authoritative live-goal arbitration
  -> lossless SemanticPlanGoalIR
  -> one checked plan per live event in SemanticPlanBundleIR
  -> compatibility PlanIR view for older consumers
```

This input circuit does not dispatch on completed sentences. Conditional,
prohibited, reported, and possible events remain distinct from live requests;
ambiguous alternatives and unlicensed unrelated goals fail closed. Korean and
English variants converge on the same predicate, role, relation, scope, and
goal identities before any response is realized.

Utterance-level intent detection is additive before it is selective. Each
matched communicative function becomes a typed candidate; one deterministic
primary candidate may select the response goal, while compatible supporting
candidates survive as constraints in the central projection receipt. Thus a
compound request such as “decide whether to continue and show the evidence”
does not depend on analyzer call order and does not create two competing
GoalIR plans.

The reference layer cannot rewrite every downstream parser input merely because
it emitted a candidate. Only an unambiguous typed cross-turn binding may own the
native surface. Local antecedents and generic focus candidates remain evidence
against the original utterance. The selected source is reused for every native
operation and for final validation, so a later module cannot switch surfaces.

The stored unit is construction knowledge, not a completed sentence. Semantic
nodes contain opaque concept identities and typed relations. Korean and English
surface roots, register, valency class, and morphology class live only in the
expression-node store. Adding or removing an expression therefore cannot alter
the semantic graph hash.

For cross-turn input, typed entity state and the current discourse focus select
the candidate set before raw mention memory is consulted. Raw memory is a
fallback and a phenotype-spelling source only; it cannot repopulate the set
with every noun from an earlier sentence. This prevents downstream reference
and clarification modules from manufacturing ambiguity after focus resolution.

Fronted themes and resumptive deictics are likewise construction knowledge.
The input circuit binds the explicit prior theme to the scoped event, records
that binding independently, and permits a later same-operation ellipsis to
inherit the typed predicate without copying either utterance.

Every stage creates a new immutable IR. Later stages reference earlier node,
edge, expression, and grammar-rule identities; they cannot overwrite the input
meaning or grant language execution authority. Activation, confidence, and
context fit are bounded integer scores accompanied by explicit reasons and
provenance.

The verifier reconstructs semantic coverage from the syntax and morphology
trace. A realization fails closed if a meaning node or relation is omitted, a
surface token lacks an expression or grammar-rule source, or the reconstructed
semantic identity differs from the input graph.

The final natural-realization boundary retains each complete generation trace,
not only its hash. It classifies every response as `GENERATIVE`, `HYBRID`, or
`LEGACY`, requires the emitted surface to contain the retained morphology, and
records a zero stage-overwrite count. This keeps incomplete migrations visible
instead of allowing a drafted surface to masquerade as pipeline output.

The response boundary is an ordered `NaturalResponsePlanIR`, not a
winner-takes-all response label. A turn may therefore preserve relational
support or a discourse bridge before its primary task move. For example,
feedback plus a corrective request and affect plus a diagnostic request retain
both contributions; neither auxiliary move can replace the task, create
semantic authority, or grant execution. Exactly one primary task move remains
last in the plan, and each move owns its own generation trace and sentence
function. The legacy scalar response act is only a checked projection of that
primary move.

Primary-response modules also no longer arbitrate by call-site `if/else`
ordering. Each module submits a non-authoritative `NaturalResponseCandidateIR`.
One order-independent arbitration pass retains every candidate, selects by the
single declared precedence lattice, and records its hash in the final
realization. Adding or reordering a producer can no longer silently change
which existing producer wins.

Specialty execution permissions are centralized as well. One monotonic
`LanguagePipelineRoutingIR` is created after normalization and receives typed
signals from definition, discourse, native-goal, action, QA, topic,
continuation, interaction, and guard analyzers. Temporal, dialogue-relation,
and discourse QA, temporal analysis, plan projection, memory commit, and final
response arbitration consult that same set instead of carrying drifting copies
of peer-module exclusions. Signal insertion is idempotent and there is no
remove/overwrite operation; analyzers remain evidence producers rather than
turn-precedence owners.

The receipt is populated before dependent routing begins. Response-goal
correction, for example, is activated before lifecycle and native-answer
selection; both later consumers must reject their competing answer candidates
when that signal is present. This prevents a result-shaped word inside a goal
correction from bypassing the intended explanation path.

Cross-turn response constraints do not live in producer-specific caches.
Typed `DialogueDirectiveCandidateIR` values enter one hash-bound conversation
ledger, where replacement is explicit supersession rather than overwrite.
The active phenotype-neutral directive is supplied to the next GoalIR context,
the response-plan policy shared by every response act, and the generation
meaning-graph builder before language-specific wording is chosen. Concise mode
removes optional affect/topic bridges but preserves the primary task and
current-turn correction. A concise plan preview is therefore also a smaller
verified semantic graph, not a completed sentence cut down by a postprocessor.
Its action and plan/not-executed boundary remain mandatory, and directive
evidence is carried into sentence provenance. This lets feedback recognized in
one language constrain a later turn in another language without storing or
replaying either complete sentence.

An explicit response-length instruction reaches this boundary only when the
existing lexical store composes `response target + directive operator + one
value`. This is a typed atom rule rather than a whole-sentence matcher. Quotes
and descriptions remain lexical evidence only, incompatible values request
clarification, and the analysis has neither semantic nor execution authority.
After authoritative Native goal arbitration, lower compositional candidates
are not consulted again for task ownership. This prevents a discarded
response-shaped candidate from becoming a fake plan or active goal downstream.
At the Native binder, an immediately pre-predicate conjunction closes the left
clause as well, so the directive target cannot leak into the following action's
Theme set.

The response-format axis uses the same lexical composition and ledger path for
plain, bullet, numbered, and table layouts. Layout is applied to the already
selected response plan; it never participates in GoalIR selection or memory
commit.

Plan materialization follows the same ownership rule. Analyzer modules cannot
render provisional responses or clear one another's text. They submit typed
blockers to `PlanProjectionDecisionIR`; only an empty blocker set permits a
semantic plan to be built. The planner reads typed Language Center events,
arguments, scopes, and relations, not scalar fields or formatted constraints
from `LanguageUnderstandingIR`. Each selected event is bound to its own plan;
non-live and prohibited events remain explicit constraints in the checked
bundle. The retained blockers are sorted and deduplicated, so module call order
cannot change the decision. `NaturalRealizationIR` is the sole live
surface-text writer; older renderer helpers are quarantined from the product
path.

Conditional guards that require the freshly committed dialogue state use a
two-phase candidate protocol. A pre-commit plan candidate remains private and
unchanged. A stored guard first contributes evidence only; it contributes
`ConditionalGuardOwnsTurn` only after the current turn yields a typed guard
evaluation. One final `PlanProjectionDecisionIR` then adopts or suppresses the
candidate. The guard never erases a published response or plan hash.

Action-state and plan-result detection also enter as generic candidates. The
specific temporal, dialogue-relation, and discourse QA paths run before central
owner promotion. If one returns a typed answer it owns the turn; otherwise one
generic candidate is promoted. This prevents a broad lifecycle recognizer from
blocking a more specific question that it only partially matched.

Selected goals are likewise materialized once. A complete native live-goal set
owns selection; peer analyses may attach evidence only to one-to-one matching
events. If no native set exists, a context-restored pragmatic goal replaces a
same-intent placeholder instead of being appended beside it. Natural
realization then reads only the checked semantic goal and plan bundle, never
the competing parser candidates.

The same selected `SemanticPlanGoalIR` is the only source for current-turn
conversation goal memory. Native and compatibility views are evidence-only at
that boundary. Deferred commitments and guarded program steps reuse the
selected semantic subject whenever they describe the same target, so a lexical
variant cannot reappear next turn as a second active goal.

Argument gaps are resolved once as well. Predicate-role extraction receives a
clause prefix with conversational openers already removed. When a prohibited
event omits its target, the Language Center may bind it to the one unambiguous
native discourse subject; competing subjects fail closed. This prevents a
particle such as `아니` from leaking through Theme into GoalIR, while retaining
the prohibited event as an explicit realization obligation.

`NaturalRealizationCoverageIR` makes omission observable. It binds each
response move, selected plan event, prohibited event, and selected event
relation to its exact generation trace and semantic-goal hash. A response with
an omitted or duplicated obligation fails validation even when every emitted
sentence is individually grounded.

Selected figurative and sarcastic readings use a dedicated typed
`InterpretationBoundary`. They no longer depend on manufacturing a placeholder
plan merely to reach language generation. Conversely, a selected explicit
information request is not confused with external-action authorization and
cannot be masked by a generic interaction boundary.

Current product integration uses this pipeline for plan previews, typed
plan/execution/result lifecycle answers, action-state answers,
reported-content acknowledgement, typed user-feedback repair, affect support,
applied topic transitions, and continuation-gate decisions. The
unbound-demonstrative branch of clarification also uses it. Applied discourse
group updates now use the same typed path: the operation and resulting member
cardinality are carried as semantic nodes and relations, then expressed in
Korean or English without reading the drafted sentence.
These paths share the same stages while supplying different meaning graphs:

- reported content: speaker, proposition, memory, fact-status negation, and
  evidence requirement;
- affect support: situation, affective quality, and invitation to inspect the
  most recent failure;
- clarification: unresolved demonstrative, reference question, target, and
  singularity constraint;
- topic return: topic motion plus an explicit topic-only/non-execution
  boundary.
- lifecycle answer: action identity plus ordered plan, report, verified
  execution, and result-availability claims derived from the bound lifecycle
  snapshot. Multi-action answers retain one generation trace per selected
  action instead of flattening their semantic sources into one drafted string.
- action-state answer: one immutable generation trace per selected action, or a
  typed action-set graph containing cardinality, quantifier, predicate, and
  three-valued truth. Untrusted language or terminal evidence is realized with
  an explicit unchanged-execution-state boundary; a reported completion cannot
  be verbalized as verified execution or a verified result.
- dialogue management: typed hold-floor, greeting, gratitude, farewell, and
  backchannel events select shared response meanings. Korean and English then
  realize those meanings through distinct interjections, predicates,
  complements, particles, auxiliaries, and endings without consulting the
  previously drafted sentence.
- user feedback: the assessed response target, one of six typed quality
  judgments, and the requested correction strategy form one shared meaning
  graph. Korean and English feedback are distinct expression phenotypes of the
  same semantic payload, not stored replies.
- continuation gate: the typed task, required real benefit, and
  positive, negative, and unresolved branches become separate ordered discourse
  moves. Persisted pending-gate status and proxy-evidence follow-ups use the
  same typed boundary; proxy observations cannot silently become proof of the
  required benefit.
- discourse-group update: add-member, remove-member, and merge-groups are typed
  operation concepts. The affected group and post-update member count remain
  language-independent; language-specific expressions only realize the
  verified update state.

The expression store contains roots and constructional forms rather than any
of the resulting complete sentences. Remaining legacy response acts are
migrated by adding semantic construction knowledge, not sentence contracts.
