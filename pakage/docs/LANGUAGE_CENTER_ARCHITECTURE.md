# B_Core Language Center and Cortex Architecture

## Boundary

The Language Center is a language-neutral thought graph. Korean and English are phenotype Cortex adapters around that graph. Surface strings are evidence and realization material; they are never semantic authority.

```text
Korean Cortex ─┐
               ├─> immutable typed contributions ─> Language Center IR
English Cortex ┘                                  ├─> GoalIR projection
                                                  ├─> discourse/world update
                                                  └─> Result/Derivation evidence
Korean Cortex <──────────────────────────────────────────────┤
English Cortex <─────────────────────────────────────────────┘
```

## Native input circuit

`NativeLanguageCircuit` is the bounded live-goal arbitration layer for input.
It stores lexical and construction knowledge rather than completed utterances
and emits an inspectable `NativeTurnIR` containing entities, events, event
scope, discourse relations, reference bindings, selected live goals, and a
response goal. The circuit handles Korean and English through the same typed
event and goal structures.

Event scope is explicit: `LIVE`, `CONDITIONAL`, `PROHIBITED`, `REPORTED`, or
`POSSIBLE`. Only fully bound live events may become active goals. Reported or
merely possible commands therefore remain content, and unrelated or
alternative multi-goal readings fail closed instead of being guessed. Typed
bindings cover local anaphora, explicit prior-theme resumption, contrastive
retargeting, causal targets, coordinated-set members, operation ellipsis,
event ordinals, and verified result targets. Explicit prior-theme resumption
preserves a fronted target such as `The Kestrel worker—inspect it` as a typed
goal before a later turn inherits its operation; it is not a stored sentence
pattern.

The native turn is the decision source. Existing pragmatic and compositional
IRs remain compatibility projections for current consumers; they may be
rebound only inside the owned Language Center materialization and cannot
overwrite the native record. Every native input/output record is hash-bound,
has `semantic_authority = false`, and has `language_can_execute = false`.

Before the native circuit is built, one source selector decides whether it may
consume the original utterance or an unambiguously resolved cross-turn surface.
Local antecedents, generic focus guesses, and unresolved references do not gain
rewrite authority. All native refinements and validation reuse this exact
choice; there is no second parser-source switch later in the turn.

Native reconciliation happens once in
`PragmaticInterpretationIR::reconcile_native_projection`. It rebuilds the
Language Center and projection receipt together. Native events never borrow a
compositional frame ID; predicate and nearest source position align the two
graphs with one-to-one frame consumption.

Cross-turn entity input has one ownership order:

```text
typed active entities
  -> current discourse focus (when the typed set is empty)
  -> raw native mention memory (fallback only)
```

Raw mention memory cannot union incidental nouns back into a resolved typed
set. It may restore phenotype spelling for an identity already selected by the
typed layer, but cannot change the entity set, order, salience, or referent ID.

## No-overwrite rule

Clause, role, scope, modality, discourse, pragmatic, and action-truth modules do not mutate a shared semantic object. Each module emits an immutable contribution. The Language Center retains every contribution, resolves compatible restrictions with an explicit lattice, and preserves incompatible claims as conflicts or unresolved state.

Resolution is fail-closed:

```text
PROHIBITED
  > CONDITIONAL
  > REPORTED / SUPPRESSED
  > LIVE_REQUEST
  > ADVISORY
  > INQUIRY
  > DESCRIPTIVE
```

This ordering does not erase lower contributions. It determines only the final projection while all contributing source claims remain inspectable.

The raw contribution lattice is not allowed to overwrite the one-shot central
materializer's final selected-candidate set. At planner handoff, a frame that
survived central materialization is projected as a live semantic event while
the earlier incompatible proposals remain present in the Language Center
conflict record. This separates retained disagreement from final goal
selection instead of letting the last module win.

The same rule applies at response time. `NaturalResponsePlanIR` composes
relational support, discourse bridges, and one primary task instead of selecting
one surface response class that overwrites the others. Auxiliary moves cannot
become semantic authority or execution permission, and the compatibility
response-act field must equal the plan's primary act.

Utterance-level response goals use the same ownership model. The analyzer
retains one deterministic primary intent plus typed supporting intents instead
of returning from the first matching module branch. Only the primary may choose
the GoalIR-facing response goal. Supporting demands remain active constraints
and appear in the central projection receipt, so a request to decide and also
show evidence cannot lose its evidence requirement or create a second plan.

The conversation pipeline creates one monotonic `LanguagePipelineRoutingIR`
immediately after normalization. Every specialty module contributes a typed
`LanguagePipelineSignalIR`; no module can delete or rewrite a peer's signal.
QA, temporal analysis, semantic-plan projection, memory commit, and response
arbitration all read that same order-independent set. Final response producers
then submit candidates to `NaturalResponseArbitrationIR`, so module source
order is provenance rather than precedence.

Persistent dialogue constraints use the same single-owner rule.
`DialogueDirectiveLedgerIR` accepts typed candidate values for response
length, response format, interaction policy, and general constraints through
`ConversationMemory`; analyzers cannot mutate it directly. One value is active
per `(kind, target)` axis, prior values remain auditable as `SUPERSEDED`, and
the ledger is bound into the conversation-state hash. Active values are read
once into the next turn's planning context and the single response-plan policy
point shared by every response act. A concise value suppresses optional affect
and topic-bridge moves, but never the primary task or current-turn corrective
feedback. For a plan preview, the generator also constructs a smaller typed
meaning graph that preserves the requested action plus the plan/not-executed
truth boundary; it does not shorten already generated text. The directive
evidence remains bound through final realization provenance. The source
sentence is retained only as an evidence hash, so a Korean or English
expression cannot become the semantic directive payload.

Explicit response-length instructions enter that ledger through composition,
not sentence recognition. `LanguageKnowledgeBase` supplies typed target,
operator, and value atoms; all three must agree before one phenotype-neutral
directive frame is emitted. Quoted spans and descriptive clauses cannot
promote a frame, while two incompatible values leave the axis unresolved.
Once `NativeLanguageCircuit` publishes its authoritative live-goal set, that
set alone determines whether a real task accompanies the directive. No later
consumer may re-read a lower parser candidate and revive a response-shaped
goal. A directive-only turn can therefore change response policy without
creating a plan or persistent task, while a directive-plus-task turn keeps the
independent task intact. Native event binding additionally applies an immediate
pre-predicate conjunction as a left clause boundary, preventing an entity in
the directive clause from leaking into the following action Theme.

Response-format instructions use the same composition boundary. The lexical
target, operator, and one of `PLAIN`, `BULLETS`, `NUMBERED`, or `TABLE` produce
a typed directive; realization changes layout only after the semantic response
plan is fixed. Format selection cannot add task events or alter remembered
goals.

Signals are inserted before their first possible consumer. In particular,
response-goal correction is recorded before lifecycle-query detection and
native answer candidate creation. Both paths must honor the same receipt, so a
surface-level result cue inside “do not inspect it; explain why it failed”
cannot take the turn back from the corrected explanation goal.

Before response arbitration, `PlanProjectionDecisionIR` is the sole owner of
whether a semantic plan may be materialized. Definition, action, QA, topic,
feedback, affect, and interaction analyzers only contribute typed blocker
evidence. They cannot draft or overwrite surface text. All blockers survive in
the decision receipt, and `NaturalRealizationIR` remains the only live output
writer.

State-dependent guard evaluation is a two-phase adoption, not a late
overwrite. The pre-commit phase may retain a plan candidate so conversation
state can be updated; it does not publish that plan. After guard evidence has
added its routing signal, the central projection decision adopts the retained
candidate once or suppresses it with a typed blocker. No specialty module can
clear an adopted response or plan hash.

The presence of a stored guard is only a plan-blocking evidence candidate. It
becomes `ConditionalGuardOwnsTurn` only when the current turn produced a typed
evaluation. Generic action-state and plan-result detections follow the same
candidate-first rule: temporal, dialogue-relation, and discourse QA receive the
turn first, and a generic candidate is promoted to the single owner only when
no specific typed answer exists.

## One-shot Goal projection

`LanguageCenterGoalProjectionIR` is the sole production boundary that creates
the GoalIR-facing compositional view. The base composition, pragmatic-intent
graph, utterance-intent graph, illocution graph, and continuation gate are all
borrowed immutably. Each contributes a typed decision with source, evidence,
effect, and precedence. The central materializer then constructs one owned
output exactly once.

The projection record binds SHA-256 hashes for every input and the materialized
output. It also retains all Language Center conflict IDs. An effect that loses
the precedence comparison is therefore still observable; no later language
module can erase the earlier proposal or call the materializer a second time.

The selected semantic event set also has a single owner. A complete native goal
set is matched one-to-one to Language Center events and prevents peer modules
from appending extra selected plans. In the absence of that set, a
context-restored pragmatic goal displaces a same-intent surface placeholder.

Conversation goal memory consumes that materialized `SemanticPlanGoalIR`
directly. Compatibility and Native analyses may provide predicate display
evidence, but they cannot be re-read to append, remove, or retarget a remembered
goal. Deferred commitments and guarded discourse programs normalize their
shared target to the same selected semantic subject.
All displaced and suppressed events remain available as provenance; only their
selection authority is removed.

Discourse markers and predicate arguments also have a single handoff rule.
Clause-initial conversational particles are removed before role extraction and
can never be promoted to an event Theme. A real argument gap on a prohibited
event may inherit the sole discourse-grounded native subject at the Language
Center boundary; if more than one distinct subject is available, it remains
unresolved. No planner or realization module performs a second surface-text
guess.

```text
immutable module graphs
        ↓ typed decisions (all retained)
explicit precedence/conflict lattice
        ↓
central_materialization_count = 1
        ↓
GoalIR-facing compositional analysis
```

Language evidence can identify a live user request, but the projection IR
itself has neither semantic authority nor execution capability. The resulting
planner handoff is `SemanticPlanGoalIR`, which preserves every typed event,
argument role, scope, and event relation. `SemanticPlanBundleIR` binds one
validated plan to every selected live event in selection order. Its first
`PlanIR` is a compatibility view only; it is no longer the authoritative
multi-event representation.

Deictic forms such as `it` and `그것` remain explicit discourse-reference
concepts until a reference layer supplies a stronger binding. They are not
dropped as lexical stopwords. Goal-graph sequence and coordination edges are
merged with clause-graph relations before planner handoff, so either parser
view cannot erase the other.

The output boundary consumes only `SemanticPlanGoalIR` and
`SemanticPlanBundleIR`. It cannot inspect parser candidates to select a
different plan. Explicit coverage obligations prove that every selected event,
prohibited event, selected relation, and response move is represented exactly
once by a grounded generation trace.

## Semantic convergence

The semantic hash excludes:

- Korean/English surface strings;
- token positions;
- source-module names;
- realization style.

It includes:

- canonical event predicates and intents;
- phenotype-neutral argument concept keys;
- semantic roles;
- event relations;
- resolved live/conditional/prohibited/report status.

Equivalent Korean and English inputs must converge on the same semantic hash. Different surface forms may retain different provenance hashes.

## Migration order

1. keep the native circuit authoritative for input arbitration and preserve its
   full trace at the Language Center boundary;
2. **completed for semantic planning:** migrate the core planner from scalar
   compatibility fields to Language Center event, argument, relation, and goal
   records;
3. make discourse/world updates consume Language Center referents and relations;
4. remove compatibility reducers after every remaining caller consumes the typed projection;
5. make Korean/English realization consume only Language Center plus verified result evidence.

The public conversation ABI remains stable during the internal migration. No external LLM, Python language path, raw-text solution dispatch, or language-owned execution authority is introduced.
