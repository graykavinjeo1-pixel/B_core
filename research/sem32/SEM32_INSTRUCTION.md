SEMANTIC REASONING PROJECT — SEM-32
LITERATURE-INFORMED TEMPORAL CAUSAL WORLD MODEL
OBJECT-RELATIONAL DYNAMICS · BELIEF STATE · INTERVENTION · UNCERTAINTY · COUNTERFACTUAL · REACHABILITY

Continue ONLY the independent B_Core / Semantic Reasoning Project lineage.

Start from the exact sealed SEM-31 predecessor:

SEALED_PREDECESSOR_COMMIT=
106616f9920fe8c6de7abe884486c4aa8588d77f

Verify predecessor integrity before any campaign mutation.

Do NOT push unless explicitly authorized.

SEM-32 has NOT previously started.

Do NOT import a neural world-model architecture wholesale.

Do NOT add an external LLM, pretrained world model, video model, or teacher.

The literature audit is DESIGN INPUT only.

The canonical campaign remains fully local and offline.

0. LITERATURE MECHANISM AUDIT FREEZE

Before implementation, create:

docs/research/SEM32_WORLD_MODEL_LITERATURE_AUDIT.md

Record the following primary research lines as mechanism references only:

World Models                         arXiv:1803.10122
PlaNet                               arXiv:1811.04551
Dreamer                              arXiv:1912.01603
DreamerV3                            arXiv:2301.04104
Dreamer4                             arXiv:2509.24527

MuZero                               arXiv:1911.08265
TD-MPC2                              arXiv:2310.16828

DINO-WM                              arXiv:2411.04983
V-JEPA 2                             arXiv:2506.09985
V-JEPA 2.1                           arXiv:2603.14482
MuDreamer                            arXiv:2405.15083
Dreamer-CDP                          arXiv:2603.07083

Slot Attention                       arXiv:2006.15055
Interaction Networks                 arXiv:1612.00222
Graph Network Simulator              arXiv:2002.09405
Recurrent Independent Mechanisms     arXiv:1909.10893
Slot Structured World Models         arXiv:2402.03326
Causal-JEPA                          arXiv:2602.11389

Towards Causal Representation
Learning                              arXiv:2102.11107
Interventional Causal Representation
Learning                              arXiv:2209.11924
Meta-Causal World                     arXiv:2506.23068
Relational Structural Causal Models   arXiv:2606.14892

PETS                                  arXiv:1805.12114
MOPO                                  arXiv:2005.13239
Plan2Explore                          arXiv:2005.05960

Director                              arXiv:2206.04114
FF-JEPA                               arXiv:2606.09311
RC-aux                                arXiv:2605.07278

Genie                                 arXiv:2402.15391
GameNGen                              arXiv:2408.14837
Cosmos                                arXiv:2501.03575

For each mechanism classify:

ADOPT
ADAPT_TO_B_CORE
PERCEPTION_ONLY
SIMULATOR_ONLY
DEFER
REJECT_AS_CANONICAL

Do NOT fetch these papers during the canonical run.

Network remains disabled.

1. ADOPTION RULE

A literature mechanism may enter B_Core only if it passes:

MECHANISM_VALUE
SEMANTIC_COMPATIBILITY
CAUSAL_VALUE
RESOURCE_VALUE

Paper reputation, benchmark rank, citation count, or model scale is not authority.

Required:

WHOLE_ARCHITECTURE_TRANSPLANTS=0
2. CANONICAL REJECTIONS

Do NOT make any of the following canonical B_Core world-state authority:

raw pixel reconstruction

video generation

single opaque dense latent state

natural-language description

reward/value-only task latent

full world snapshots

one-step prediction accuracy alone

correlation-only causal claims

generator self-verification

unbounded full-memory rollout

These may appear only as adapters, observations, auxiliary tools, or comparison arms where explicitly permitted.

3. CENTRAL SCIENTIFIC QUESTION

SEM-31 established:

persistent semantic World(t)

SEM-32 must establish:

Belief(World_t)
+
Action / Intervention / Exogenous Event
+
Context
+
Causal Mechanisms
        ↓
Distribution / Set of plausible semantic ΔWorld
        ↓
Belief(World_t+1)

and:

same anchor state
+
alternative intervention
        ↓
isolated counterfactual branch

The model must explain and predict how meaning changes over time.

4. THREE-LAYER WORLD STATE

Separate three functions.

A. Persistent Semantic World

Task-independent reusable world knowledge:

entities
properties
relations
states
laws
history
provenance
B. Belief World State

What B_Core currently believes given partial evidence:

observed facts
inferred state
hidden-state hypotheses
uncertainty
competing explanations
C. Active World Slice

Sparse task/context-relevant projection:

currently relevant entities
relations
mechanisms
hypotheses

Required:

ACTIVE_PROJECTION_CAN_MUTATE_CANONICAL_WORLD_SEMANTICS=false

This preserves task-relevant efficiency without reducing the world itself to a task-specific representation.

5. PARTIAL OBSERVABILITY IS FIRST-CLASS

Do NOT assume the entire world state is directly observed.

The canonical fixture must include hidden state.

B_Core must distinguish:

KNOWN_TRUE
KNOWN_FALSE
UNKNOWN
BELIEVED
COMPETING_HYPOTHESES

or semantically equivalent forms.

Do NOT fabricate missing state simply to create a complete snapshot.

6. TEMPORAL BELIEF UPDATE

Support:

prior belief
+
new observation
+
known transition mechanisms
        ↓
posterior semantic belief

Exact Bayesian implementation is NOT required.

But evidence must update belief consistently.

Required:

UNOBSERVED_STATE_HALLUCINATED_AS_FACT=0
7. EPISTEMIC VS ALEATORIC UNCERTAINTY

Distinguish at least:

EPISTEMIC_UNCERTAINTY
= B_Core does not yet know enough

ALEATORIC_OR_WORLD_STOCHASTICITY
= the world genuinely permits multiple outcomes

Do not collapse these into one uncertainty scalar.

Example:

"I do not know whether the door is locked"

is not equivalent to:

"this mechanism randomly opens with probability p"

Track both separately.

8. OBJECT / RELATION FACTORED DYNAMICS

World transitions should preferentially decompose into:

entity-local dynamics

relation-mediated interaction

context-dependent mechanism

global invariant where genuinely necessary

Do NOT use one monolithic transition rule if causal factorization is available.

9. SPARSE INDEPENDENT MECHANISMS

At a given step:

activate only causal mechanisms relevant to the active semantic neighborhood.

Conceptual form:

M1 inactive
M2 active
M3 inactive
M4 active
...

Mechanisms communicate only through explicit semantic dependencies.

Required:

ALL_CAUSAL_MECHANISMS_ACTIVE_PER_STEP=false
10. ACTIONS ARE NOT OBJECT PROPERTIES

Represent actions/interventions as first-class semantic events.

Do NOT concatenate action meaning into entity properties.

Distinguish:

ACTION
INTERVENTION
EXOGENOUS_EVENT
PASSIVE_OBSERVATION
11. OBSERVATION IS NOT INTERVENTION

Mandatory distinction:

observe X
then observe Y

does NOT establish:

do(X) causes Y

Track:

OBSERVATIONAL_TRANSITIONS
INTERVENTIONAL_TRANSITIONS

Required:

FALSE_CAUSAL_PROMOTIONS=0
12. CAUSAL HYPOTHESIS SETS

When observations support multiple explanations:

preserve them.

Example:

H1: A → C
H2: B → C
H3: hidden H → A and C

Do NOT collapse early.

Track hypothesis support and contradiction evidence.

13. HIDDEN CONTEXT / CAUSAL REGIMES

The fixture must include at least one case where:

same apparent action
+
same visible objects

but hidden/contextual state differs
        ↓
different effect

B_Core must discover or infer the missing context rather than creating contradictory universal laws.

Example abstractly:

PUSH + UNLOCKED → OPEN
PUSH + LOCKED   → NO_OPEN

Do NOT hard-code lock semantics.

14. RELATIONAL CAUSAL GENERALIZATION

A causal law must be capable of applying across:

different entity IDs
different numbers of entities
novel combinations of known entities
novel relation topology

when its semantic applicability conditions remain true.

Identity names are never causal authority.

15. CAUSAL LAW REPRESENTATION

Derive or implement something equivalent in purpose to:

CausalMechanismIR {
    inputs,
    required_relations,
    context,
    preconditions,
    intervention_or_event,
    transformation,
    predicted_delta,
    uncertainty,
    applicability,
    exceptions,
    provenance,
    verification,
}

Exact Rust structure is not prescribed.

16. SEMANTIC DELTA PREDICTION

Canonical prediction target is:

ΔWorld

not a reconstructed full world.

Predict only changes and necessary continuities.

Do NOT repeatedly rewrite unchanged state.

Required:

FULL_PREDICTED_WORLD_SNAPSHOT_COPIES=0
UNCHANGED_SEMANTIC_REWRITE_EVENTS=0
17. REPRESENTATION-SPACE PREDICTION PRINCIPLE

Predict future semantic structure directly.

Do NOT generate:

pixels
frames
text

to discover what happened unless explicitly running a non-canonical adapter comparison.

SEM-32 has no visual decoder requirement.

18. TASK-IRRELEVANT DISTRACTOR TEST

Include semantic observations that change but have no causal relevance to the target transition.

The Active World Slice should avoid activating them when unnecessary.

However the persistent world may still retain them if they are valid world facts.

Measure:

IRRELEVANT_ACTIVE_SEMANTIC_LOAD

Do not gain efficiency by deleting valid world knowledge.

19. ONE-STEP PREDICTION GATE

Before observing the fresh next state:

freeze current belief
freeze predicted semantic delta
then reveal transition

Required:

FUTURE_STATE_READS_BEFORE_PREDICTION=0

Measure structural prediction errors separately for:

entity state
properties
relations
creation/destruction
epistemic state
20. MULTI-HORIZON PREDICTION

Short-horizon correctness is insufficient.

Evaluate open-loop prediction at several horizons.

At minimum where supported:

H1
H2
H4
H8

Do NOT expose hidden intermediate truth during open-loop rollout.

Track raw error by horizon.

21. RECURSIVE VS MULTI-HORIZON FAILURE

Measure whether errors arise from:

wrong local mechanism
compounding rollout error
missing context
uncertainty collapse
wrong entity identity
relation error

Do not report only one aggregate prediction score.

22. STOCHASTIC FUTURE

Where the fixture genuinely supports multiple valid outcomes:

prediction must represent multiple plausible semantic futures or an equivalent uncertainty representation.

Forbidden:

choose one arbitrary future and mark CERTAIN

Required:

STOCHASTIC_FUTURE_COLLAPSE_EVENTS=0
23. UNCERTAINTY PROPAGATION

Uncertain premises must not automatically generate certain descendants.

Track:

PREDICTIVE_UNCERTAINTY_COLLAPSE_EVENTS

Target:

0

unless independent evidence resolves the uncertainty.

24. MODEL-BOUNDARY AWARENESS

When rollout enters a region poorly supported by experience:

epistemic uncertainty should increase or an explicit UNKNOWN boundary should appear.

Required:

UNSUPPORTED_ROLLOUT_CONFIDENT_HALLUCINATIONS=0

A valid response may be:

INSUFFICIENT_MODEL_SUPPORT
25. ACTIVE CAUSAL EXPERIMENT SELECTION

Reuse autonomous experiment machinery.

When hypotheses compete, B_Core must select an intervention based on expected causal information value.

Conceptually prefer:

expected hypothesis reduction
+
expected uncertainty reduction
+
causal discriminability
-
experiment cost

over raw novelty.

Do NOT prescribe a fixed formula.

26. DISAGREEMENT IS A SIGNAL, NOT A GOAL

Model/hypothesis disagreement may indicate useful exploration.

But noisy or inherently random regions must not attract exploration forever.

Distinguish:

reducible disagreement
vs
irreducible stochasticity

Track wasted exploration on irreducible noise.

27. NO HUMAN EXPERIMENT SELECTION

Required:

HUMAN_CAUSAL_EXPERIMENT_SELECTION_EVENTS=0
HUMAN_CAUSAL_HYPOTHESIS_SELECTION_EVENTS=0
28. COUNTERFACTUAL WORLD BRANCH

From an anchor world state:

Actual:
S0 + action A → S1

Counterfactual:
S0 + action B → S1'

Counterfactual state must remain logically isolated.

Use copy-on-write/delta sharing where useful.

Do NOT duplicate complete world state merely to create a branch.

29. ACTUAL / COUNTERFACTUAL SEPARATION

Required:

COUNTERFACTUAL_TO_ACTUAL_MUTATION_EVENTS=0
ACTUAL_HIDDEN_FUTURE_TO_COUNTERFACTUAL_LEAKAGE_EVENTS=0
30. COUNTERFACTUAL VERIFICATION

Where fixture semantics permit:

instantiate an independent equivalent anchor world and realize the alternative intervention.

Compare:

predicted counterfactual
vs
realized counterfactual

Do not let the verifier provide the answer before prediction.

31. REACHABILITY IS NOT SEMANTIC SIMILARITY

Introduce or derive an equivalent of:

ReachabilityIR

A state being semantically similar or "close" to a goal does not imply it is reachable within a finite action budget.

Track:

REACHABLE_WITHIN_BUDGET
REACHABLE_EVENTUALLY
UNREACHABLE
UNKNOWN_REACHABILITY

where appropriate.

32. LATENT/SEMANTIC SHORTCUT TRAP

Create cases where a superficially attractive future state cannot be produced through valid transition mechanisms within the available horizon.

The model must not accept:

"looks close"

as:

"causally reachable"

Required:

UNREACHABLE_SHORTCUT_ACCEPTS=0

This is only a planning-readiness canary.

SEM-32 does NOT build the full planner.

33. CAUSAL PATH CERTIFICATE

A predicted reachable future should be able to expose:

anchor state
→ mechanism/event
→ delta
→ intermediate state
→ ...
→ predicted state

or a compressed equivalent with decompression available.

This becomes the future planning substrate.

34. RESIDUAL-DRIVEN DYNAMICS LEARNING

When prediction fails:

classify the residual.

Possible discovered classes may include:

missing property
missing relation
missing hidden state
wrong causal direction
wrong applicability
missing interaction
new mechanism
incorrect stochastic assumption
identity error

Do NOT preselect the answer.

35. COMPOSITION BEFORE NEW PRIMITIVE

Before creating:

new semantic primitive
new causal mechanism

test whether existing semantic atoms/mechanisms in a new relation topology already explain the residual.

Track:

CAUSAL_COMPOSITION_EVENTS
NEW_CAUSAL_PRIMITIVE_EVENTS
36. SEMANTIC VOCABULARY PRESSURE

SEM-31 showed substantial cold-start primitive genesis.

Begin longitudinal measurement:

NEW_PRIMITIVES_PER_100_NOVEL_EVENTS

EXISTING_SEMANTIC_REUSE_RATE

SEMANTIC_COMPOSITION_RATE

NEW_CAUSAL_LAWS_PER_100_RESIDUALS

No arbitrary threshold for SEM-32 PASS.

But preserve the sequence for later world-model maturation.

37. DESIRED LONG-TERM TREND

Do NOT enforce this numerically in SEM-32.

Observe whether growth begins moving toward:

primitive genesis ↓

reuse ↑

composition ↑

compressed reusable laws ↑

as world experience increases.

38. RELATION-LOCAL INTERACTION TEST

Include worlds where only a small subset of entities actually interact.

Prediction must activate that neighborhood rather than the whole world.

Example abstractly:

100,000 persistent entities

current event involves:
E17
E31
E99

Active reasoning should remain bounded around relevant relations.

39. 100K WORLD SCALING CANARY

Reuse the SEM-31 100,000-entity scale canary.

Extend it to causal prediction.

Track:

TOTAL_WORLD_ENTITIES

ACTIVE_ENTITIES_P50
ACTIVE_ENTITIES_P95

TOTAL_CAUSAL_MECHANISMS

ACTIVE_CAUSAL_MECHANISMS_P50
ACTIVE_CAUSAL_MECHANISMS_P95

WORLD_MEMORY_FULL_SCANS
CAUSAL_MECHANISM_FULL_SCANS

Required:

WORLD_MEMORY_FULL_SCANS=0
CAUSAL_MECHANISM_FULL_SCANS=0
40. ENTITY-COUNT GENERALIZATION

A mechanism learned with one number of entities must be tested with different cardinalities when semantically valid.

Do not bind causal law applicability to fixture size.

41. NOVEL COMPOSITION GENERALIZATION

Test:

known entities
known property meanings
known mechanisms

but

fresh relation configuration

B_Core should compose existing mechanisms where sufficient.

This is a stronger test than memorized state transition replay.

42. REPEATED DYNAMICS → COMPILED LONG-TERM MEMORY

Reuse the causally verified SEM-30 mechanism.

If a transition reasoning DAG becomes:

repeated
verified
transferable
predictive
compression-positive

it may be promoted into a reversible compiled semantic node.

No forced promotion.

43. FAST CAUSAL PATH

When a compiled causal node is valid:

current active semantic state
→ compiled mechanism
→ predicted delta

may replace a longer derivation.

Measure:

depth before/after
active nodes before/after
cost before/after
44. SURPRISE FORCES DECOMPRESSION

If:

applicability mismatch
unexpected residual
contradictory evidence
unknown context

occurs:

the compiled node must either reject fast-path use or decompress.

Required:

UNSAFE_CAUSAL_SHORTCUT_ACCEPTS=0
45. WORLD LAW IS NOT CACHE

Forbidden:

state hash → expected next state

as evidence of causal learning.

A law must expose reusable semantic applicability independent of exact task instance.

Required:

TASK_INSTANCE_TRANSITION_CACHE_AUTHORITY=false
46. INVARIANT / CONSTRAINT PRESERVATION

Where the fixture defines a mechanically verified invariant:

predicted states must respect it.

If a rollout violates an invariant:

reject
reduce confidence
or
diagnose model error

Do not accept an impossible state merely because local prediction score is high.

47. TEMPORAL SCALE

Represent temporal ordering explicitly.

Where useful also preserve:

duration
delay
persistence

Do not assume all causes have one-tick effects.

Include at least one delayed-effect canary.

48. DELAYED CAUSALITY

A cause may affect a state after intermediate events.

Do not require adjacent-step correlation for causal attribution.

Test at least one:

intervention at t
→ no immediate visible effect
→ effect at t+k

with independent verification.

49. CREATION / DESTRUCTION / MERGE / SPLIT

Where supported by fixture semantics:

world dynamics may change entity structure itself.

Handle:

CREATE
DESTROY
MERGE
SPLIT

without false identity persistence.

50. PREDICTIVE SUFFICIENCY WITHOUT WORLD AMNESIA

The Active World Slice may discard irrelevant detail for a current prediction.

But the Persistent Semantic World must not erase valid knowledge simply because it is currently task-irrelevant.

Required distinction:

ACTIVE_PREDICTION_SUFFICIENCY
!=
CANONICAL_WORLD_MEMORY_CONTENT
51. STRUCTURED CLOSED-WORLD FIXTURE

SEM-32 remains language-free.

Use typed structured events.

The canonical fixture must include at least:

persistent entities

shared properties

relations

hidden state

deterministic mechanism

stochastic mechanism

confounded observation

direct intervention

context-dependent mechanism

delayed effect

novel entity combination

unreachable shortcut

counterfactual branch

Do NOT encode expected causal laws into B_Core input.

52. MULTIPLE WORLD FAMILIES

Use multiple structurally distinct world families.

A causal law or semantic mechanism transfer claim must involve fresh structures, not renamed clones.

Measure structural distance.

53. GENERATOR / VERIFIER SEPARATION

Required:

WORLD_GENERATOR_IS_SUCCESS_AUTHORITY=false

Freeze verifier before fresh canonical episodes.

The verifier may know ground truth.

B_Core may not.

54. NO GOLD LEAKAGE

Required:

CAUSAL_GOLD_LAW_READS=0

EXPECTED_NEXT_STATE_LOOKUPS=0

FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=0

COUNTERFACTUAL_GOLD_BRANCH_READS=0
55. CAUSAL ABLATION — OBSERVATION VS INTERVENTION

Construct a confounded case.

Arm A:

observation only

Arm B:

observation + discriminating intervention

Strong causal claims should improve only with sufficient evidence.

Required:

INTERVENTIONAL_CAUSALITY_ABLATION_PASS=true
56. CAUSAL LAW MEMORY ABLATION

Arm A:

learned causal mechanisms retained

Arm B:

raw prior experience available
but promoted causal-law memory disabled

Measure transfer/prediction cost.

Required:

CAUSAL_LAW_MEMORY_ABLATION_PASS=true
57. FACTORIZATION ABLATION

Compare:

object/relation-local mechanism routing

against a controlled non-factored transition representation with equal available evidence.

Claim advantage only if:

generalization
sparsity
cost
or
sample efficiency

actually improves.

Required for factorization superiority claims:

FACTORED_DYNAMICS_ABLATION_PASS=true
58. UNCERTAINTY ABLATION

Remove epistemic uncertainty tracking while keeping observations constant.

Test OOD/unseen transitions.

Required for uncertainty benefit claims:

EPISTEMIC_UNCERTAINTY_ABLATION_PASS=true
59. COUNTERFACTUAL MODEL ABLATION

Disable causal transition structure while preserving superficial observational association where feasible.

Counterfactual performance must materially degrade.

Required:

COUNTERFACTUAL_CAUSAL_MODEL_ABLATION_PASS=true
60. SPARSE ROUTING ABLATION

Use equal semantic task.

Compare sparse local activation to bounded full/global routing comparison.

Required:

SPARSE_CAUSAL_ROUTING_ABLATION_PASS=true
61. COMPILED MEMORY ABLATION

If compiled causal nodes naturally emerge:

compare:

compiled fast path
vs
forced decompressed path

Claim performance benefit only if ablation supports it.

Otherwise return:

N/A_NO_NATURAL_PROMOTION

Do not force a node for the metric.

62. PREDICTION ≠ PLANNING

SEM-32 must explicitly report:

PREDICTION_CAPABILITY_ESTABLISHED
PLANNING_CAPABILITY_ESTABLISHED

Expected SEM-32 target:

PREDICTION_CAPABILITY_ESTABLISHED=true
PLANNING_CAPABILITY_ESTABLISHED=false

unless an already-existing mechanism independently satisfies a stronger test.

Do not overclaim.

63. NO POLICY TRAINING REQUIREMENT

Do NOT implement:

Dreamer actor/critic
MuZero MCTS
TD-MPC CEM planner
FF-JEPA planner

as SEM-32 requirements.

Take their useful principles, not their whole architectures.

Full autonomous goal-directed planning is reserved for a later stage.

64. NO VISION REQUIREMENT

Do NOT add:

DINO
V-JEPA
video encoder
image tokenizer
diffusion model

to canonical B_Core.

Their literature contribution is the principle of predictive abstraction.

Perceptual grounding comes later through adapters.

65. NO GENERATIVE VIDEO CORE

Genie/GameNGen/Cosmos-style simulation may later be useful as:

perception training source
synthetic observation generator
external simulator

but NOT canonical semantic memory.

Required:

GENERATIVE_VIDEO_MODEL_CORE_DEPENDENCY=false
66. RESOURCE DISCIPLINE

Preserve:

CORE_MANDATORY_VRAM=0
CORE_DEPENDS_ON_GPU_RUNTIME=false

Canonical SEM-32 must remain CPU-capable.

Do not trade semantic efficiency for large neural inference.

67. EVENT-BOUNDED CAMPAIGN

Primary success chain:

persistent semantic world
        ↓
partial observation
        ↓
multiple causal hypotheses
        ↓
autonomous discriminating intervention
        ↓
verified relational causal mechanism
        ↓
fresh next-state prediction
        ↓
multi-horizon rollout
        ↓
uncertainty preserved
        ↓
fresh entity/topology transfer
        ↓
counterfactual prediction
        ↓
counterfactual verification
        ↓
unreachable shortcut rejected
        ↓
prediction residual
        ↓
causal mechanism refinement/composition
        ↓
future prediction improves

Stop when complete evidence is obtained.

68. HARD CEILING

Use:

MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096

as containment only.

Budget must not be semantic input.

Stop early on PASS or valid autonomous stop.

69. CHECKPOINTING

Operational checkpoint every:

64 epochs

and immediately at:

first causal hypothesis set

first intervention

first verified causal mechanism

first stochastic future

first hidden-context discovery

first counterfactual verification

first delayed-effect law

first cross-entity causal transfer

first reachability rejection

first compiled causal-memory promotion

major ablation
70. LEVEL A — TEMPORAL BELIEF WORLD

Pass only if:

persistent semantic state
+
partial observability
+
belief update
+
temporal deltas

work without language authority.

71. LEVEL B — FACTORED RELATIONAL DYNAMICS

Pass only if:

entity/relation/context mechanisms

predict fresh transitions and transfer to novel entity identity/configuration.

72. LEVEL C — UNCERTAINTY-AWARE PREDICTION

Pass only if:

epistemic vs stochastic uncertainty

remain distinguishable and confidence does not collapse improperly.

73. LEVEL D — INTERVENTIONAL CAUSALITY

Pass only if:

observation
!=
intervention

and confounded causal structure is resolved by actual discriminating evidence.

74. LEVEL E — LONGER-HORIZON DYNAMICS

Pass only if:

multi-step open-loop prediction

is mechanically evaluated and failure sources are decomposed rather than hidden in an aggregate score.

75. LEVEL F — COUNTERFACTUAL WORLD MODEL

Pass only if alternative actions generate isolated semantic branches that survive independent realization tests.

76. LEVEL G — REACHABILITY INTEGRITY

Pass only if the system rejects causally impossible finite-horizon shortcuts.

77. LEVEL H — RESIDUAL-DRIVEN CAUSAL LEARNING

Pass only if prediction errors autonomously refine, compose, split, or create mechanisms based on evidence.

No answer lookup.

78. LEVEL I — SPARSE SCALABLE WORLD DYNAMICS

Pass only if causal prediction over the large-world canary remains sparse with zero full scans.

79. LEVEL J — CAUSAL MECHANISM VALIDATION

Pass only if required causal/factorization/uncertainty/counterfactual/sparse ablations support the claimed mechanisms.

80. CORE PASS

SEM-32 PASS requires:

LEVEL_A=true
LEVEL_B=true
LEVEL_C=true
LEVEL_D=true
LEVEL_E=true
LEVEL_F=true
LEVEL_G=true
LEVEL_H=true
LEVEL_I=true
LEVEL_J=true

Do NOT weaken a failed level.

81. REQUIRED RAW MEASUREMENTS

Return raw sequences, not only aggregate scores:

ONE_STEP_PREDICTION_RESULTS

HORIZON_ERROR_SEQUENCE

STRUCTURAL_DELTA_ERROR_SEQUENCE

EPISTEMIC_UNCERTAINTY_SEQUENCE

ALEATORIC_OR_STOCHASTIC_BRANCH_COUNTS

CAUSAL_HYPOTHESIS_COUNT_SEQUENCE

INTERVENTION_SEQUENCE

CAUSAL_LAW_GENESIS_SEQUENCE

CAUSAL_LAW_REUSE_SEQUENCE

CAUSAL_LAW_TRANSFER_SEQUENCE

COUNTERFACTUAL_RESULTS

REACHABILITY_RESULTS

PREDICTION_RESIDUAL_SEQUENCE

SEMANTIC_REUSE_SEQUENCE

SEMANTIC_COMPOSITION_SEQUENCE

NEW_PRIMITIVE_SEQUENCE

ACTIVE_ENTITY_SEQUENCE

ACTIVE_CAUSAL_MECHANISM_SEQUENCE

WORLD_MEMORY_BYTES_SEQUENCE
82. REQUIRED FINAL RESPONSE

Return:

SEM32_STATUS=PASS|FAIL
DISPOSITION=

CAMPAIGN_ID=

BRANCH=
COMMIT=
WORKTREE_CLEAN=
PUSH_PERFORMED=

SEALED_PREDECESSOR_COMMIT=
PREDECESSOR_INTEGRITY=

LITERATURE_AUDIT_PRESENT=
LITERATURE_MECHANISMS_ADOPTED=
LITERATURE_MECHANISMS_ADAPTED=
LITERATURE_MECHANISMS_REJECTED_AS_CANONICAL=
WHOLE_ARCHITECTURE_TRANSPLANTS=

TEMPORAL_CAUSAL_WORLD_MODEL_PRESENT=

PERSISTENT_WORLD_LAYER_PRESENT=
BELIEF_WORLD_LAYER_PRESENT=
ACTIVE_WORLD_SLICE_PRESENT=

ACTIVE_PROJECTION_CAN_MUTATE_CANONICAL_WORLD_SEMANTICS=

PARTIAL_OBSERVABILITY_CASES=
HIDDEN_STATE_HYPOTHESES=

UNOBSERVED_STATE_HALLUCINATED_AS_FACT=

EPISTEMIC_UNCERTAINTY_EVENTS=
ALEATORIC_STOCHASTIC_EVENTS=
PREDICTIVE_UNCERTAINTY_COLLAPSE_EVENTS=
STOCHASTIC_FUTURE_COLLAPSE_EVENTS=

CAUSAL_MECHANISMS_TOTAL=
CAUSAL_MECHANISM_REUSE_EVENTS=
CAUSAL_MECHANISM_TRANSFER_EVENTS=

OBSERVATIONAL_TRANSITIONS=
INTERVENTIONAL_TRANSITIONS=
FALSE_CAUSAL_PROMOTIONS=

CAUSAL_HYPOTHESIS_COMPETITIONS=
AUTONOMOUS_DISCRIMINATING_INTERVENTIONS=
HYPOTHESES_RESOLVED=

HIDDEN_CONTEXT_DISCOVERY_EVENTS=
DELAYED_CAUSAL_EFFECT_EVENTS=

ONE_STEP_PREDICTIONS=
ONE_STEP_CORRECT=

MULTISTEP_PREDICTIONS=
HORIZON_ERROR_SEQUENCE=

FULL_PREDICTED_WORLD_SNAPSHOT_COPIES=
UNCHANGED_SEMANTIC_REWRITE_EVENTS=

COUNTERFACTUAL_PREDICTIONS=
COUNTERFACTUAL_VERIFIED=
COUNTERFACTUAL_ERRORS=

COUNTERFACTUAL_TO_ACTUAL_MUTATION_EVENTS=
ACTUAL_HIDDEN_FUTURE_TO_COUNTERFACTUAL_LEAKAGE_EVENTS=

REACHABILITY_QUERIES=
UNREACHABLE_SHORTCUT_CASES=
UNREACHABLE_SHORTCUT_ACCEPTS=

PREDICTION_RESIDUAL_EVENTS=
CAUSAL_COMPOSITION_EVENTS=
CAUSAL_LAW_REFINEMENT_EVENTS=
CAUSAL_LAW_SPLIT_EVENTS=
NEW_CAUSAL_LAW_GENESIS_EVENTS=
NEW_CAUSAL_PRIMITIVE_EVENTS=

NEW_PRIMITIVES_PER_100_NOVEL_EVENTS=
EXISTING_SEMANTIC_REUSE_RATE=
SEMANTIC_COMPOSITION_RATE=

COMPRESSED_CAUSAL_MEMORY_NODES_PROMOTED=
COMPRESSED_CAUSAL_MEMORY_DECOMPRESSION_AVAILABLE=
UNSAFE_CAUSAL_SHORTCUT_ACCEPTS=

ENTITY_ID_INVARIANT_CAUSAL_TRANSFER_PASS=
NOVEL_ENTITY_COUNT_TRANSFER_PASS=
NOVEL_RELATION_TOPOLOGY_TRANSFER_PASS=

TOTAL_WORLD_ENTITIES=
TOTAL_WORLD_SEMANTIC_NODES=
TOTAL_CAUSAL_MECHANISMS=

ACTIVE_ENTITIES_P50=
ACTIVE_ENTITIES_P95=
ACTIVE_CAUSAL_MECHANISMS_P50=
ACTIVE_CAUSAL_MECHANISMS_P95=

WORLD_MEMORY_FULL_SCANS=
CAUSAL_MECHANISM_FULL_SCANS=

TASK_INSTANCE_TRANSITION_CACHE_AUTHORITY=

INTERVENTIONAL_CAUSALITY_ABLATION_PASS=
CAUSAL_LAW_MEMORY_ABLATION_PASS=
FACTORED_DYNAMICS_ABLATION_PASS=
EPISTEMIC_UNCERTAINTY_ABLATION_PASS=
COUNTERFACTUAL_CAUSAL_MODEL_ABLATION_PASS=
SPARSE_CAUSAL_ROUTING_ABLATION_PASS=
COMPILED_CAUSAL_MEMORY_ABLATION_PASS=

PREDICTION_CAPABILITY_ESTABLISHED=
PLANNING_CAPABILITY_ESTABLISHED=

WORLD_GENERATOR_IS_SUCCESS_AUTHORITY=

CAUSAL_GOLD_LAW_READS=
EXPECTED_NEXT_STATE_LOOKUPS=
FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=
COUNTERFACTUAL_GOLD_BRANCH_READS=

NATURAL_LANGUAGE_IS_CANONICAL_WORLD_MEMORY=
NATURAL_LANGUAGE_IS_CAUSAL_REASONING_AUTHORITY=
WORLD_MEMORY_NATURAL_LANGUAGE_BYTES_ON_HOT_PATH=

GENERATIVE_VIDEO_MODEL_CORE_DEPENDENCY=

CORE_MANDATORY_VRAM=
CORE_DEPENDS_ON_GPU_RUNTIME=

HUMAN_CAUSAL_EXPERIMENT_SELECTION_EVENTS=
HUMAN_CAUSAL_HYPOTHESIS_SELECTION_EVENTS=

GLOBAL_REASONING_REGRESSIONS=
META_QUALITY_REGRESSIONS=
GAIN_ERASURE_EVENTS=
CAPABILITY_NEGATIVE_TRANSFER_EVENTS=

EXTERNAL_LLM_CALLS=
LOCAL_TEACHER_CALLS=
NETWORK_READS=
NETWORK_WRITES=
REMOTE_EXECUTIONS=

NEW_CLIPPY_WARNING_SIGNATURES_TOTAL=
CORE_DOCKABILITY_PRESERVED=

NEXT_DOMINANT_GROWTH_LIMIT=

SEM32_LEVEL_A_PASS=
SEM32_LEVEL_B_PASS=
SEM32_LEVEL_C_PASS=
SEM32_LEVEL_D_PASS=
SEM32_LEVEL_E_PASS=
SEM32_LEVEL_F_PASS=
SEM32_LEVEL_G_PASS=
SEM32_LEVEL_H_PASS=
SEM32_LEVEL_I_PASS=
SEM32_LEVEL_J_PASS=

SEM33_STARTED=false
NEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY
83. SUCCESS INTERPRETATION

SEM-32 PASS means:

B_Core does not merely remember a changing world.

It maintains uncertain beliefs about partially observed state,
discovers reusable object/relation causal mechanisms,
distinguishes observation from intervention,
predicts fresh semantic state changes,
propagates multiple possible futures,
tests causal hypotheses through autonomous interventions,
simulates and verifies counterfactual worlds,
rejects causally unreachable shortcuts,
and improves its causal model from prediction residuals,
while keeping persistent world reasoning sparse.
84. FAILURE INTERPRETATION

Do not collapse FAIL into one category.

Return the dominant boundary, such as:

BELIEF_STATE_LIMIT

PARTIAL_OBSERVABILITY_LIMIT

RELATIONAL_DYNAMICS_LIMIT

CAUSAL_IDENTIFICATION_LIMIT

INTERVENTION_SELECTION_LIMIT

UNCERTAINTY_REPRESENTATION_LIMIT

LONG_HORIZON_PREDICTION_LIMIT

COUNTERFACTUAL_LIMIT

REACHABILITY_LIMIT

CAUSAL_COMPOSITION_LIMIT

SPARSE_DYNAMICS_ROUTING_LIMIT

OTHER

Do NOT manually rescue the failed mechanism after canonical exposure.

85. WHAT IS DELIBERATELY DEFERRED

SEM-32 does NOT attempt:

full goal-directed planner

actor/critic policy learning

MCTS

robot embodiment

camera/video grounding

audio grounding

natural-language world memory

generative video world model

large neural latent simulator

multi-agent social world model

Those require later independent gates.

86. EXPECTED NEXT FRONTIER

If SEM-32 passes cleanly, the natural next stage is likely:

SEM-33

GOAL-DIRECTED HIERARCHICAL SEMANTIC PLANNING

current causal world belief
        ↓
desired world phenotype
        ↓
reachability
        ↓
subgoal synthesis
        ↓
bounded semantic imagination
        ↓
action selection
        ↓
real observation
        ↓
world-model correction

Do NOT start it automatically.

Suggested commit:

Establish literature-informed causal predictive semantic world model

Start SEM-32 now from the sealed SEM-31 predecessor.