# B_Core portable product-core package

Source base commit: `cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f`

Source state: uncommitted native Language Circuit, generative Language Cortex,
and typed semantic-planner boundary worktree. This package records the base
commit and worktree state separately so it does not misrepresent uncommitted
integration work as a sealed commit.

This directory is the portable, product-facing subset of `B_Core`.

## Included

- `crates/dockable-semantic-core`: language-independent GoalIR/ResultIR runtime,
  semantic state, sparse index, runtime provenance, deliberation, experience,
  planning, executable mechanism memory, swarm coordination, configuration,
  and ABI.
- `crates/semantic-core-adapters`: bounded language, cognitive API, lexical and
  document knowledge, professional document, mechanism induction, long-term
  repair, and generic capability adapters kept outside the semantic payload.
  The current Language Center preserves immutable contributions and conflicts
  instead of letting analyzers overwrite one another. Goal projection now
  records every module proposal and binds all input/output hashes before one
  central materialization; direct public mutation reducers are not exposed.
  A single source selector now decides whether the Native circuit consumes the
  original utterance or an unambiguously resolved cross-turn surface. Local
  antecedents and generic focus guesses cannot rewrite the parser input. Once
  the Native circuit has selected a complete live-goal set, peer modules may
  enrich matching events but cannot append another selected plan; contextual
  goal restoration replaces same-intent placeholders rather than duplicating
  them.
  The core planner consumes `SemanticPlanGoalIR` rather than the scalar/string
  `LanguageUnderstandingIR` compatibility view. Every selected live event is
  bound to its own checked plan in `SemanticPlanBundleIR`; sequence,
  coordination, conditional, reported, and prohibited events remain typed at
  the handoff. The first `PlanIR` is exposed only for older consumers and
  cannot replace the multi-event bundle.
  Natural realization consumes only that semantic goal and checked bundle.
  `NaturalRealizationCoverageIR` binds every response move, selected event,
  prohibited event, and selected relation to exactly one grounded generation
  trace, so both invented claims and semantic omissions fail validation.
  A Rust-only `NativeLanguageCircuit` now constructs typed entities, scoped
  events, discourse relations, reference bindings, live goals, and response
  goals from Korean or English construction knowledge. It is the input
  arbitration source; older pragmatic IR is retained only as a compatibility
  projection. Reported, possible, conditional, prohibited, ambiguous, and
  unrelated-alternative events cannot silently become executable goals.
  Fronted themes with an explicit resumptive deictic are retained as a typed
  prior-theme binding, allowing later same-operation ellipsis to inherit the
  predicate without storing either sentence.
  Plan previews, reported-content acknowledgements, affect support, and
  applied topic transitions use the
  Rust-only generative path `meaning -> speech intent -> discourse plan ->
  expression nodes -> syntax -> morphology -> semantic verification`. Korean
  and English expression nodes share one semantic payload; completed sentences
  are rejected as expression knowledge.
  The R43 response contract binds grammatical composition, discourse/topic
  state, deixis/ellipsis, pragmatic intent, plan/result separation, and
  evidence-grounded realization into one tamper-evident integration record.
  R44 adds conversational definition grounding: a new lexical label may bind
  to exactly one existing semantic operator, while rejected, ambiguous, or
  hypothetical definitions cannot create a capability or semantic authority.
  R45 adds productive cross-predicate argument sharing: Korean left-shared and
  English right-shared objects compile into one typed entity binding with
  explicit direction and clause-relation provenance. The binding remains
  discourse-local and cannot grant semantic or execution authority.
  R46 carries an ordered multi-action structure across turns as a hash-bound
  `DiscourseProgramIR`. An explicit new subject may instantiate the complete
  program through the normal grammatical and GoalIR path; bare repetition,
  quoted programs, withdrawn programs, and partial programs that share a
  blocked target fail closed. Korean and English may share the same typed
  program without storing a whole-sentence solution template.
  R47 extends that program with hash-bound conditional guards. Mixed requests
  keep their immediate action active while their conditional action remains a
  `ConditionPending` deferred commitment. Explicit Korean/English subject
  rebinding re-enters the ordinary parser and GoalIR path; verified evidence is
  still required before a guarded action can activate. Bare, quoted,
  counterfactual, missing, and cross-target workflows fail closed.
  R48 links every guarded program step to its exact deferred commitment inside
  the conversation-state hash. Condition hash, normalized antecedent, subject,
  predicate, source turn, and activated GoalIR are cross-validated. Natural
  language claims remain non-authoritative; only a separate trusted evidence
  receipt can activate the linked step, while wrong hashes, foreign IDs,
  contradiction, and replay fail closed without executing an external action.
  R49 replaces the prior single-predicate guard collapse with a bounded,
  hash-bound condition-expression IR. `ALL`, `ANY`, and atom-level `NOT`
  preserve AND/OR precedence and parentheses across Korean/English workflow
  rebinding. Condition-internal alternatives no longer masquerade as action
  alternatives; mixed-subject expressions fail closed, and neither language
  claims nor expression nodes acquire semantic or execution authority.
  R50 closes the product response boundary with
  `LanguageCortexResponseIntegrationIR`. The request, normalization,
  definition grounding, reference resolution, pragmatic interpretation,
  action-state analysis, discourse outputs, pragmatic/conversation state,
  claim realization, interaction provenance, six-axis receipt, and final
  output are bound into one recomputable response hash. Component or request
  substitution fails live validation; the receipt remains non-authoritative,
  performs no external action, and records zero LLM, teacher, network, or
  recursive source-mutation dependencies.
  R51 strengthens defeasible discourse decisions without changing that public
  response schema. Proxy gains are separated from direct benefits, conditional
  continuation cannot create an immediately authorized `CONTINUE` goal, and a
  typed current task survives intervening neutral turns and stop-branch
  language. Reported or quoted commands cannot displace the user's outer
  assessment/explanation request. Korean embedded-question complements and
  audit phrases remain intact, while unacceptable recurring failures produce
  only a non-authoritative repair goal. The deterministic adapter still makes
  no LLM, teacher, network, or recursive source-mutation calls.
  R52 makes continuation and result anaphora reference-safe without changing
  the public response schema, conversation-state schema, or core ABI.
  `continue it`, `keep doing that`, and `그 작업을 이어가` defer to the typed
  pragmatic task frame instead of a conditional or stop-branch distractor.
  Same-turn `result`, `output`, `outcome`, `결과`, `출력`, and `산출물`
  expressions bind to their local producing event, while a true cross-turn
  result question remains an evidence-absence query and never fabricates
  execution. Straight and curly reported quotations stay non-authoritative;
  a following independent assessment may still become the user's grounded
  request. Complete continuation gates outrank incidental candidate
  competition but never authorize an immediate `CONTINUE` action.
  R53 distinguishes predicate coordination from argument coordination.
  English active, passive, quantified, and prepositional coordinated members
  compile into distinct typed entity nodes just as Korean particle-marked
  members do. When coordinated predicates share a coordinated argument set,
  the complete set is shared through typed bindings rather than collapsing to
  one surface string or copying only the primary object. Explicitly different
  argument sets remain separate, per-member quantifiers stay attached to their
  original nodes, quoted structures remain non-authoritative, and clause
  connectors such as `and if` cannot be mistaken for noun coordination.
  R54 makes pragmatic task restoration follow the hash-bound discourse topic
  rather than global recency. Task frames and pending continuation gates carry
  a non-authoritative topic ID; named, indexed, long-horizon, and cross-language
  topic returns reactivate only the matching state. Multiple suspended topics
  retain independent gates, similar topic names remain distinct, and a topic
  without a known task asks for clarification instead of borrowing another
  topic's work. Language still cannot authorize continuation or execution.
  The public response schema, conversation-state schema, and core ABI remain
  unchanged, but the nested pragmatic-memory schema is now
  `B_CORE_PRAGMATIC_MEMORY_STATE_IR_2`. Consumers that inspect or persist that
  nested adapter state must accept the additive topic fields; no persistence
  migration loader is claimed by this package.
  R55 extends the same exact topic identity to result referents and questions
  under discussion. Explicit named, indexed, long-horizon, and cross-language
  returns resolve result anaphora only inside the restored topic, including
  bounded Korean/English bare-result ellipsis. Multiple suspended topics keep
  independent QUDs; answering one does not erase another, and an unseen topic
  cannot borrow a globally recent result or question. These additions advance
  the conversation-state schema to `B_CORE_CONVERSATION_STATE_26`. Consumers
  that persist conversation state must accept referent/question topic IDs and
  `topic_pending_questions`; this package does not claim a state migration
  loader. Response schema 12, pragmatic-memory IR_2, and core ABI 1 remain
  unchanged.
  R56 adds a hash-bound grammatical-scope graph for recursive quantifier,
  restriction, conjunction, disjunction, negation, and focus composition.
  Korean and English structures reach GoalIR as typed, inspectable constraints
  without granting language semantic or execution authority. `NONE` scope
  blocks the governed action, while unresolved negation/quantifier scope is
  preserved instead of guessed. Coordinated negative predicates retain their
  shared argument through a structural gap rather than treating auxiliary
  words as an object. The compositional-analysis schema advances to
  `B_CORE_COMPOSITIONAL_ANALYSIS_IR_6`; response schema 12, conversation state
  26, pragmatic-memory IR_2, and core ABI 1 remain unchanged. This is an
  additive response-IR change and does not claim a persisted-state migration
  loader.
  R57 consolidates each live discourse topic into a bounded, hash-bound
  `TopicContextGraphIR`. Suspending and resuming a topic now restores that
  topic's own discourse focus, pending question, and discourse referents
  instead of borrowing the globally most recent object. Named, indexed,
  cross-language, social-interruption, and three-topic returns all use the same
  typed transition path. Standalone acknowledgements and hold-floor phrases do
  not replace semantic focus. Distinct-target prohibited clauses remain outside
  a reusable discourse-program cohort, while same-target prohibitions and
  deferred workflows still fail closed. The conversation-state schema advances
  to `B_CORE_CONVERSATION_STATE_27`; persisted-state consumers must accept the
  additive topic-context graph. Language and topic state retain zero semantic
  or execution authority, and the public response schema and core ABI remain
  unchanged.
  R58 replaces first-match handling for compound reference expressions with a
  bounded, hash-bound `ReferenceResolutionGraphIR`. Repeated possessives,
  multiple demonstratives, typed-person plus discourse-focus references, and
  ordered `former/latter` or `전자/후자` anchors are represented as separate
  mention nodes and candidate edges. An ordered member becomes the local
  antecedent for its following possessive instead of leaking the global focus.
  Markers inside balanced quotations remain inert, while a live marker after
  the closing quote is resolved normally. Multiple missing antecedents remain
  explicitly unresolved and cannot gain a binding. Candidate competition,
  selected edges, and resolution hashes are exposed for inspection but carry
  no semantic or execution authority. This additive public API advances the
  conversation response schema to `B_CORE_CONVERSATION_TURN_RESPONSE_13` and
  the frontend schema to `B_CORE_CONVERSATION_FRONTEND_3`; conversation state
  27, pragmatic-memory IR_2, and core ABI 1 remain unchanged.
  R59 adds a bounded, hash-bound `UtteranceIntentGraphIR` between surface
  parsing and pragmatic goal projection. It separates literal clause form from
  communicative intent for Korean and English problem disclosures, evidence
  requests, recommendation requests, explanation/summary requests,
  conditional decisions, and response-goal corrections. Candidate selection
  is inspectable, prior-context references fail closed when no antecedent is
  available, and no candidate receives semantic or execution authority.
  Explicit compositional requests still outrank generic disclosure readings,
  while a failure report tied to an already active action remains an
  unverified action-state report rather than becoming a new diagnostic task.
  The public conversation response advances additively to
  `B_CORE_CONVERSATION_TURN_RESPONSE_14`; conversation state 27,
  pragmatic-memory IR_2, frontend 3, and core ABI 1 remain unchanged.
  R60 adds a typed, hash-bound lifecycle view that keeps plan state, user
  reports, host-verified execution, and terminal result availability on four
  independent axes. A lifecycle question or response-axis correction may
  select which axis to discuss, but it cannot create, supersede, withdraw, or
  execute a plan. Outcome language remains an unverified report unless the
  existing host-evidence API supplies a bound receipt. Explicit explanation
  goal corrections continue through the ordinary GoalIR path rather than
  being swallowed by status lookup, and existing result-reference responses
  retain their more specific discourse binding. The public conversation
  response advances additively to `B_CORE_CONVERSATION_TURN_RESPONSE_15`, and
  the full response receipt advances to
  `B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_2` so the lifecycle boundary
  is included in live component validation. Conversation state 27,
  pragmatic-memory IR_2, frontend 3, and core ABI 1 remain unchanged.
  R61 adds a typed, evidence-grounded natural-realization layer after semantic
  reasoning and before final response integration. Korean and English plan
  previews now state concrete intended work while explicitly distinguishing a
  plan from execution; result queries, user-provided facts, feedback, affect,
  unresolved references, and topic returns are realized from their typed
  sources instead of exposing internal IR labels. Every emitted sentence binds
  to request, turn, plan, lifecycle, action, referent, topic, or definition
  evidence as applicable. Internal IR leaks, unsupported claims, and empty
  promises fail live validation. The public conversation response advances
  additively to `B_CORE_CONVERSATION_TURN_RESPONSE_16`, and the full response
  receipt advances to `B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_3` so the
  natural-realization hash and source binding are checked at the package
  boundary. Conversation state 27, pragmatic-memory IR_2, frontend 3, and core
  ABI 1 remain unchanged; the language layer retains zero semantic or execution
  authority and makes no LLM, teacher, network, or recursive mutation calls.
  R62 closes the six-axis integration boundary across grammatical composition,
  discourse/topic state, reference and ellipsis, pragmatic intent, plan/result
  lifecycle, and evidence-grounded realization. Eight typed cross-axis links
  and ten live invariants now bind the selected language structure through the
  final natural response. Hold-floor turns preserve the active task, explicit
  result-axis corrections cannot manufacture an execution result, newly
  activated topics fail closed instead of borrowing a globally recent result,
  and quoted or rejected language remains non-authoritative. The public
  conversation response advances additively to
  `B_CORE_CONVERSATION_TURN_RESPONSE_17`; full response integration advances to
  `B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_4`; and six-axis integration
  advances to `B_CORE_SIX_AXIS_INTEGRATION_IR_2`. Conversation state 27,
  pragmatic-memory IR_2, frontend 3, and core ABI 1 remain unchanged. The
  product path remains Rust-only with zero LLM, teacher, network, Python, or
  recursive mutation calls.
  R63 hardens the public cognitive API against adversarial combinations without
  changing its schemas. A mixed immediate/conditional utterance now retains an
  executable immediate GoalIR while recording the guarded part only as a
  non-authoritative deferred commitment. Quoted commands remain inert while a
  following live clause is parsed normally; Korean hold-floor fillers preserve
  the current task; late prohibitions bind only to their governed step; and
  clarification outranks stale lifecycle prose. Singular result references
  across independently active actions fail closed, while plural result phrases
  bind to the exact persistent action group. Independently introduced people
  cannot be collapsed behind an ungrounded plural pronoun. The frozen R63
  diagnostic and transfer suites pass 18/18 and 12/12, and all historical
  language canaries remain green. The product path remains deterministic Rust
  with no LLM, teacher, network, Python, or recursive source mutation.
  R69 closes the remaining recognize-then-forget path for cross-turn dialogue
  constraints. `DialogueDirectiveLedgerIR` is the single bounded,
  hash-bound conversation-state owner for response length, response format,
  interaction policy, and general constraints. Language analyzers submit
  immutable typed candidates; only `ConversationMemory` commits or supersedes
  them. A Korean response-length correction is retained as a language-neutral
  directive and reaches an English follow-up's planning context, generation
  response plan, generation meaning graph, and sentence provenance. All 20
  response acts cross one policy point: concise mode removes optional affect
  and topic bridges while preserving the primary task and corrective feedback.
  It also produces a smaller typed plan-preview graph before expression
  selection while retaining the action and plan/not-executed boundary; no
  completed text is truncated. Source wording is stored only as an evidence
  hash, and authority tampering fails validation. Conversation state advances to
  `B_CORE_CONVERSATION_STATE_28`; persisted-state consumers must accept the
  additive directive ledger. Public response schema 18, frontend 3,
  pragmatic-memory IR_2, and core ABI 1 remain unchanged.
  Explicit response-length requests are now composed from existing lexical
  target, operator, and value atoms before entering that ledger. Quoted and
  descriptive wording does not promote policy, conflicting values fail closed,
  and the authoritative Native live-goal set is not re-read through lower
  candidate selectors. Directive-only turns create neither a fake plan nor a
  persisted fake goal; a simultaneous independent task remains intact.
  Response format uses the same typed path for plain text, bullets, numbered
  steps, and tables; the frozen composition suite passes 16/16 with layout
  validation. Conversation goal memory now consumes only the selected
  `SemanticPlanGoalIR`. Native and compatibility candidates may restore display
  evidence but cannot add, remove, or replace a selected event. Generic action
  state and plan-result analyzers remain candidates until more specific typed
  temporal or discourse QA has had a chance to answer, and a conditional guard
  owns the response only when the current turn produced a typed evaluation.
- `docs/DOCKABLE_CORE_INTEGRATION.md`: integration and boundary guidance.
- `bin/core-x0-canary.exe`: prebuilt Windows x86-64 runtime canary.

## Deliberately excluded

Research/evaluation crates, SEM campaign reports, blind suites, historical
evidence, sandboxes, `.git`, build caches, debug binaries, growth-supervisor
campaign tooling, and recursive source-mutation machinery are not product-core
runtime dependencies and are not included.

The research canary campaign is deliberately excluded. Only the four minimal
runtime boundary canaries listed below remain in this portable package.

## Boundary

The dependency direction is `semantic-core-adapters` ->
`dockable-semantic-core`. Raw language does not enter the core, adapters do not
own semantic state, and language reports cannot establish verified execution.
The default build is Rust-only. `python-paddle-ocr` is an optional compatibility
feature and is disabled by default.

## Validate

From this directory:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::manual_is_multiple_of
cargo test --workspace
cargo run -p dockable-semantic-core --bin core-x0-canary
cargo run -p semantic-core-adapters --bin language-adapter-canary
cargo run -p semantic-core-adapters --bin generic-capability-canary
cargo run -p semantic-core-adapters --bin cognitive-api-canary
```

The prebuilt canary is only a boundary/runtime check. A consuming product
should depend on `crates/dockable-semantic-core` as a Rust library and connect
through its own adapter or `semantic-core-adapters`; it should not treat the
canary as a product service.
