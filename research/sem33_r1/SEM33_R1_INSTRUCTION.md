# SEMANTIC REASONING PROJECT — SEM-33-R1
## VERIFIER TRANSPORT REPAIR + FRESH BLIND HIERARCHICAL PLANNING REGATE
## MEASURE THE PLANNER THAT SEM-33 FAILED TO MEASURE

Continue ONLY the independent B_Core / Semantic Reasoning Project lineage.

Historical SEM-33 campaign result is immutable:

SEM33_CAMPAIGN_STATUS=FAIL
SEM33_SCIENTIFIC_CAPABILITY_STATUS=UNRESOLVED_NOT_MEASURED
SEM33_DOMINANT_BOUNDARY=OTHER / VERIFIER_TRANSPORT_DESERIALIZATION_LIMIT

Historical sealed SEM-33 commit:

901ee1b01109e24b1f7b683b3f1e1b2e30b74e43

The valid capability predecessor remains the sealed SEM-32-R1 commit:

b23dcaf42365d202cbd03e0a8c7a11aa0a7e6c1b

Do NOT treat historical SEM-33 as planner-capability evidence.

Do NOT start SEM-34.

Do NOT push unless explicitly authorized.

============================================================
0. CENTRAL PURPOSE
============================================================

SEM-33 did NOT establish that the planner failed.

The canonical measurement failed because a verifier transport field with a numeric map key was serialized by JSON as a string key such as:

"100"

and the runner attempted to deserialize that map key directly as u16.

Therefore SEM-33-R1 has two strictly separated phases:

P0:
repair and prove the GENERIC verifier/runner transport contract only.

P1:
freeze infrastructure, return to the valid SEM-32-R1 capability predecessor, and execute the original SEM-33 planning experiment on entirely fresh unseen planning instances.

No planner semantic repair is authorized before fresh measurement.

============================================================
1. PRESERVE HISTORICAL SEM-33
============================================================

Preserve all historical artifacts and receipts.

Required:

HISTORICAL_SEM33_CAMPAIGN_STATUS=FAIL
HISTORICAL_SEM33_CAPABILITY_STATUS=UNRESOLVED_NOT_MEASURED
HISTORICAL_SEM33_RESULT_REWRITTEN=false

Do NOT convert historical SEM-33 to PASS after fixing infrastructure.

SEM-33-R1 is a new canonical measurement lineage.

============================================================
2. P0 — INFRASTRUCTURE-ONLY REPAIR
============================================================

P0 may modify only:

verifier transport schema
serialization/deserialization boundary
runner transport parsing
transport tests
acceptance plumbing required to consume verifier output

P0 must NOT modify:

planner
GoalIR / DesiredWorldPhenotype semantics
reachability
subgoal synthesis
causal world model
uncertainty reasoning
routing
planning policy
compiled procedural memory
world dynamics

Required:

P0_PLANNER_SEMANTIC_DIFF=0
P0_WORLD_MODEL_SEMANTIC_DIFF=0

============================================================
3. ROOT-CAUSE CONTRACT
============================================================

Record the exact historical failure:

JSON object keys are strings by specification.

A numeric semantic key transported as:

100

became:

"100"

and the runner expected a native u16 map key.

Do NOT special-case the literal key "100".

Repair the generic schema.

============================================================
4. GENERIC NUMERIC-KEY TRANSPORT
============================================================

Use one canonical generic representation for transport fields whose logical keys are bounded integers.

Preferred approaches include either:

A.
explicit key/value records:

[
  {"key": 100, "value": ...},
  ...
]

or

B.
JSON string-key maps with an explicit validated conversion layer:

"100" -> parsed bounded u16(100)

The implementation choice is autonomous.

Whichever representation is chosen must:

preserve semantic value
validate numeric syntax
validate u16 range
reject malformed keys
reject negative keys
reject overflow
reject ambiguous textual forms where applicable
round-trip deterministically

No task-specific transport branches.

Required:

TASK_SPECIFIC_TRANSPORT_BRANCHES=0

============================================================
5. TRANSPORT ROUND-TRIP CANARIES
============================================================

Before any planning task exposure, test representative key cases including:

0
1
100
255
256
32767
65535

and invalid cases including equivalents of:

negative
65536+
non-numeric
empty
fractional
malformed

Required:

VALID_U16_KEY_ROUNDTRIP_PASS=true
INVALID_U16_KEY_REJECTION_PASS=true
TRANSPORT_SEMANTIC_ROUNDTRIP_DIFF=0

============================================================
6. NESTED TRANSPORT CANARIES
============================================================

Test the repaired transport contract when the numeric-key structure appears:

top level
nested inside another object
inside arrays
with multiple map entries
with empty maps
with unrelated adjacent fields

Do not prove only the exact historical payload shape.

Required:

NESTED_TRANSPORT_CANARIES_PASS=true

============================================================
7. VERIFIER / RUNNER AGREEMENT
============================================================

Serialize verifier output.

Consume it through the exact production runner path.

Then compare canonical semantic content before serialization and after runner deserialization.

Required:

VERIFIER_RUNNER_TRANSPORT_EQUIVALENCE=true

Do not use an independent toy parser as the only proof.

============================================================
8. TRANSPORT NEGATIVE TEST
============================================================

Create a deliberately malformed verifier payload.

The runner must fail closed with a precise transport/schema error.

It must NOT:

silently coerce
drop the field
default the value
mark planning PASS
continue with partial verifier state

Required:

TRANSPORT_FAIL_OPEN_EVENTS=0
TRANSPORT_FIELD_DROP_EVENTS=0

============================================================
9. ACCEPTANCE HARNESS RECHECK
============================================================

Carry forward the raw-field acceptance discipline established in SEM-32-R1.

Required:

RAW_FIELD_ACCEPTANCE_AUTHORITY=true
PRIMARY_SECONDARY_ACCEPTANCE_DIFF=0
ACCEPTANCE_FALSE_PASS_EVENTS=0

A verifier transport failure must never become planner PASS.

============================================================
10. BUDGET CONTRACT
============================================================

SEM-33 requested:

MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096

Before canonical exposure require:

REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096
CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096
CAMPAIGN_BUDGET_CONTRACT_PASS=true

4096 is containment only.

It is not a target consumption.

============================================================
11. SEAL P0
============================================================

After transport and acceptance tests pass:

freeze:

transport schema hash
serializer hash
runner parser hash
verifier hash
acceptance hash
campaign config hash

Produce:

P0_INFRASTRUCTURE_REPAIR_SEALED=true

Only after this freeze may planning fixtures be selected/exposed.

============================================================
12. DO NOT USE THE HISTORICAL SEM-33 PLANNER DESCENDANT AS CAPABILITY AUTHORITY
============================================================

Canonical SEM-33-R1 planning capability must start from:

b23dcaf42365d202cbd03e0a8c7a11aa0a7e6c1b

plus the P0 infrastructure-only transport repair.

Do not import any planner behavior that might have been influenced by historical SEM-33 fresh exposure.

Required:

HISTORICAL_SEM33_PLANNING_STATE_REUSE_EVENTS=0
HISTORICAL_SEM33_FRESH_INSTANCE_REUSE_EVENTS=0

============================================================
13. FRESH HOLDOUT REQUIREMENT
============================================================

Every canonical planning instance used in historical SEM-33 is now EXPOSED.

Generate/select an entirely fresh sealed holdout.

Freeze before planner exposure:

world-family manifest
world semantics
action semantics
goal semantics
causal verifier
goal verifier
seed
task generator
fresh holdout hashes

Required:

FRESH_PLANNING_HOLDOUT=true
HISTORICAL_HOLDOUT_INSTANCE_OVERLAP=0

============================================================
14. DO NOT PRE-REPAIR THE PLANNER
============================================================

This is critical.

The purpose of SEM-33-R1 is first to measure the planner that SEM-33 failed to measure.

Before the first fresh planning result:

PLANNER_CAPABILITY_REPAIR_EVENTS=0
HUMAN_PLANNER_REPAIR_EVENTS=0
AUTONOMOUS_PLANNER_REPAIR_EVENTS=0

Run the ordinary planning baseline first.

============================================================
15. ORIGINAL SEM-33 SCIENTIFIC CONTRACT REMAINS
============================================================

Re-run the original SEM-33 requirements without weakening them.

The planner must demonstrate:

semantic DesiredWorldPhenotype grounding
causal reachability
finite-budget reachability integrity
autonomous subgoal synthesis where useful
hierarchical long-horizon planning
belief/uncertainty-aware information gathering
closed-loop execution
world-model residual handling
replanning
novel topology planning
entity-cardinality generalization
novel goal composition
sparse planning
no brute-force full action tree
mechanical goal verification
required causal ablations

Do not replace these with transport success.

============================================================
16. INITIAL CAPABILITY MEASUREMENT
============================================================

Run the sealed SEM-32-R1 + repaired P0 infrastructure on the fresh SEM-33-R1 planning holdout.

Record:

INITIAL_PLANNER_MEASUREMENT_COMPLETED
INITIAL_GOAL_TASKS_TOTAL
INITIAL_GOAL_TASKS_SOLVED
INITIAL_LONG_HORIZON_TASKS_SOLVED
INITIAL_REACHABILITY_RESULT
INITIAL_HIERARCHICAL_PLANNING_RESULT
INITIAL_UNCERTAINTY_PLANNING_RESULT
INITIAL_CLOSED_LOOP_RESULT
INITIAL_GENERALIZATION_RESULT

If the planner passes all original SEM-33 gates immediately:

PASS SEM-33-R1.

Do NOT invent extra planner self-research.

============================================================
17. IF THE PLANNER GENUINELY FAILS
============================================================

Only after a valid measured planner failure may autonomous research begin.

Then allow the existing autonomous research loop to diagnose the actual measured boundary.

Possible observed classes may include:

GOAL_GROUNDING_LIMIT
REACHABILITY_PLANNING_LIMIT
SUBGOAL_SYNTHESIS_LIMIT
LONG_HORIZON_PLANNING_LIMIT
BRANCH_ROUTING_LIMIT
UNCERTAINTY_PLANNING_LIMIT
INFORMATION_GATHERING_LIMIT
CLOSED_LOOP_REPLANNING_LIMIT
PLAN_TRANSFER_LIMIT
PROCEDURAL_COMPRESSION_LIMIT
OTHER

Do NOT preselect one.

============================================================
18. AUTONOMOUS RESEARCH AFTER MEASURED FAILURE
============================================================

If required:

observe failure
→ generate competing diagnoses
→ run discriminating experiments
→ synthesize generic planner repair
→ freeze repaired planner
→ regate on a SECOND fresh planning holdout

The holdout used to diagnose a measured planner failure becomes exposed.

Do NOT repair and then claim success on the same exposed holdout.

============================================================
19. TWO-STAGE FRESHNESS IF REPAIR OCCURS
============================================================

If no planner repair is needed:

Fresh Set A is sufficient.

If planner repair occurs:

Fresh Set A = measurement/diagnosis only
Fresh Set B = final blind repair regate

Required in repair case:

REPAIR_REGATE_HOLDOUT_DISTINCT=true
SET_A_SET_B_INSTANCE_OVERLAP=0

============================================================
20. GOAL SUCCESS AUTHORITY
============================================================

Required:

PLANNER_IS_GOAL_SUCCESS_AUTHORITY=false

Only realized mechanically verified world state may satisfy a goal.

============================================================
21. NO GOLD PLAN / ACTION ACCESS
============================================================

Required:

GOLD_ACTION_READS=0
GOLD_PLAN_READS=0
EXPECTED_GOAL_STATE_LOOKUPS=0
FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=0

============================================================
22. REACHABILITY
============================================================

Required:

UNREACHABLE_PLAN_ACCEPTS=0
SEMANTIC_NEAR_UNREACHABLE_SHORTCUT_ACCEPTS=0

Semantic similarity is not reachability.

============================================================
23. AUTONOMOUS SUBGOALS
============================================================

For long-horizon tasks:

HUMAN_SUBGOAL_SELECTION_EVENTS=0

Track:

AUTONOMOUS_SUBGOALS_CREATED
HIERARCHICAL_PLAN_EVENTS
MAX_SUBGOAL_DEPTH

Do not require hierarchy on trivial tasks.

============================================================
24. PARTIAL OBSERVABILITY / INFORMATION ACTIONS
============================================================

Where current belief is insufficient:

allow autonomous information gathering.

Required:

UNSUPPORTED_PLAN_CONFIDENT_EXECUTIONS=0

Track:

INFORMATION_GATHERING_ACTIONS
EPISTEMIC_UNCERTAINTY_PLANNING_EVENTS

============================================================
25. CLOSED-LOOP PLANNING
============================================================

Canonical loop:

belief
→ plan
→ bounded execution
→ observation
→ prediction residual
→ world/belief update
→ replan where needed

Track:

REPLAN_EVENTS
REPLAN_CAUSED_BY_MODEL_RESIDUAL
GOALS_SATISFIED_AFTER_REPLAN

============================================================
26. STRUCTURAL GENERALIZATION
============================================================

Required:

NOVEL_RELATION_TOPOLOGY_PLANNING_PASS=true
ENTITY_CARDINALITY_PLANNING_GENERALIZATION_PASS=true
NOVEL_GOAL_COMPOSITION_PASS=true

Carry forward SEM-32-R1 local relational mechanism composition.

Do not use graph-instance lookup.

============================================================
27. NO MEMORIZED PLAN AUTHORITY
============================================================

Required:

TASK_ID_TO_PLAN_LOOKUP_AUTHORITY=false
WORLD_HASH_TO_PLAN_LOOKUP_AUTHORITY=false
GOAL_HASH_TO_PLAN_LOOKUP_AUTHORITY=false

============================================================
28. SPARSE PLANNING
============================================================

Required:

WORLD_MEMORY_FULL_SCANS=0
CAUSAL_MECHANISM_FULL_SCANS=0
FULL_ACTION_TREE_ENUMERATION_EVENTS=0

Track:

ACTIVE_ENTITIES_PER_PLAN_P50/P95
ACTIVE_RELATIONS_PER_PLAN_P50/P95
ACTIVE_CAUSAL_MECHANISMS_PER_PLAN_P50/P95

RAW_ACTION_BRANCHING_FACTOR_SEQUENCE
SEMANTICALLY_ROUTED_CANDIDATES_SEQUENCE
ACTUALLY_ROLLED_OUT_CANDIDATES_SEQUENCE

============================================================
29. 100K ENTITY CANARY
============================================================

Preserve the large persistent-world routing test.

A bounded local planning problem in a 100K-entity persistent world must remain local.

Do not claim 100K richly simulated entities.

This is a routing/scaling canary only.

============================================================
30. REQUIRED ABLATIONS
============================================================

If the corresponding mechanism is claimed, require:

REACHABILITY_PLANNING_ABLATION_PASS=true
HIERARCHICAL_PLANNING_ABLATION_PASS=true
CAUSAL_MODEL_PLANNING_ABLATION_PASS=true
UNCERTAINTY_PLANNING_ABLATION_PASS=true
CLOSED_LOOP_REPLANNING_ABLATION_PASS=true
SPARSE_PLANNING_ABLATION_PASS=true

If compiled procedural memory does not naturally emerge:

PROCEDURAL_MEMORY_ABLATION_PASS=N/A_NO_NATURAL_PROMOTION

Do not force promotion.

============================================================
31. ACCEPTANCE LEVELS
============================================================

Retain original SEM-33 Levels A–H.

Level A:
Semantic Goal Grounding

Level B:
Causal Reachability Planning

Level C:
Autonomous Hierarchical Decomposition

Level D:
Belief / Uncertainty-Aware Planning

Level E:
Closed-Loop Execution

Level F:
Structural Generalization

Level G:
Sparse Scalable Planning

Level H:
Causal Mechanism Validation

SEM-33-R1 PASS requires all A–H.

============================================================
32. INFRASTRUCTURE FAILURE MUST REMAIN DISTINCT
============================================================

Introduce explicit scientific disposition:

MEASURED_PASS
MEASURED_CAPABILITY_FAIL
UNRESOLVED_INFRASTRUCTURE_FAILURE

A transport/schema/runner failure after this repair must never be classified as planner capability failure.

Required:

CAPABILITY_FAILURE_FROM_INFRASTRUCTURE_ONLY_EVENTS=0

============================================================
33. CLEAN RECONSTRUCTION
============================================================

Final result must survive:

workspace/all-target tests
campaign-specific tests
Clippy diff
independent offline clean reconstruction
verifier/runner transport canary
primary/secondary acceptance recomputation

============================================================
34. WARM CACHE POLICY
============================================================

Preserve the currently validated single warm cache if compatible.

Do not delete it before the new cache is validated.

After a newer cache is mechanically validated and the older cache is obsolete, reclaim the obsolete cache within the existing disk budget.

Warm cache is never semantic authority.

============================================================
35. REQUIRED ARTIFACTS
============================================================

Create at minimum:

reports/sem33_r1/
    historical_sem33_unresolved_receipt.json

    transport_root_cause.json
    transport_schema_repair.json
    transport_roundtrip_canaries.json
    malformed_transport_negative_tests.json
    verifier_runner_equivalence.json

    acceptance_recheck.json
    budget_contract.json
    p0_infrastructure_freeze.json

    fresh_set_a_manifest.json
    initial_planner_measurement.json

    measured_failure_diagnosis.jsonl        # only if required
    planner_repair_lineage.jsonl            # only if required
    repair_freeze.json                      # only if required
    fresh_set_b_manifest.json               # only if required
    repair_blind_regate.json                # only if required

    reachability_results.json
    hierarchy_results.json
    uncertainty_planning_results.json
    closed_loop_results.json
    structural_generalization.json
    sparse_planning.json
    planning_ablations.json

    raw_level_inputs.json
    primary_acceptance.json
    secondary_acceptance.json

    final_regression.json
    clean_reconstruction.json

    sem33_r1_final_report.json
    SEM33_R1_REPORT.md
    artifact_manifest.json

============================================================
36. FINAL RESPONSE
============================================================

Return:

SEM33_R1_STATUS=PASS|FAIL
SCIENTIFIC_DISPOSITION=
MEASURED_PASS
|
MEASURED_CAPABILITY_FAIL
|
UNRESOLVED_INFRASTRUCTURE_FAILURE

CAMPAIGN_ID=

BRANCH=
COMMIT=
WORKTREE_CLEAN=
PUSH_PERFORMED=

HISTORICAL_SEM33_CAMPAIGN_STATUS=FAIL
HISTORICAL_SEM33_CAPABILITY_STATUS=UNRESOLVED_NOT_MEASURED
HISTORICAL_SEM33_COMMIT=901ee1b01109e24b1f7b683b3f1e1b2e30b74e43

SEALED_CAPABILITY_PREDECESSOR_COMMIT=
b23dcaf42365d202cbd03e0a8c7a11aa0a7e6c1b

P0_INFRASTRUCTURE_REPAIR_SEALED=
P0_PLANNER_SEMANTIC_DIFF=
P0_WORLD_MODEL_SEMANTIC_DIFF=

TRANSPORT_SCHEMA_REPAIRED=
VALID_U16_KEY_ROUNDTRIP_PASS=
INVALID_U16_KEY_REJECTION_PASS=
NESTED_TRANSPORT_CANARIES_PASS=
TRANSPORT_SEMANTIC_ROUNDTRIP_DIFF=
VERIFIER_RUNNER_TRANSPORT_EQUIVALENCE=
TRANSPORT_FAIL_OPEN_EVENTS=
TRANSPORT_FIELD_DROP_EVENTS=

REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=
CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=
CAMPAIGN_BUDGET_CONTRACT_PASS=

FRESH_PLANNING_HOLDOUT=
HISTORICAL_HOLDOUT_INSTANCE_OVERLAP=

INITIAL_PLANNER_MEASUREMENT_COMPLETED=
INITIAL_GOAL_TASKS_TOTAL=
INITIAL_GOAL_TASKS_SOLVED=

PLANNER_CAPABILITY_REPAIR_REQUIRED=
AUTONOMOUS_PLANNER_REPAIR_EVENTS=
AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED=

REPAIR_REGATE_HOLDOUT_DISTINCT=
SET_A_SET_B_INSTANCE_OVERLAP=

GOAL_DIRECTED_SEMANTIC_PLANNER_PRESENT=

GOAL_TASKS_TOTAL=
GOAL_TASKS_SOLVED=

LONG_HORIZON_TASKS=
LONG_HORIZON_TASKS_SOLVED=

REACHABILITY_QUERIES=
UNREACHABLE_PLAN_CASES=
UNREACHABLE_PLAN_ACCEPTS=
SEMANTIC_NEAR_UNREACHABLE_SHORTCUT_ACCEPTS=

AUTONOMOUS_SUBGOALS_CREATED=
HUMAN_SUBGOAL_SELECTION_EVENTS=
HIERARCHICAL_PLAN_EVENTS=
MAX_SUBGOAL_DEPTH=

INFORMATION_GATHERING_ACTIONS=
UNSUPPORTED_PLAN_CONFIDENT_EXECUTIONS=

PLAN_EXECUTION_ACTIONS=
REPLAN_EVENTS=
REPLAN_CAUSED_BY_MODEL_RESIDUAL=
GOALS_SATISFIED_AFTER_REPLAN=

NOVEL_RELATION_TOPOLOGY_PLANNING_PASS=
ENTITY_CARDINALITY_PLANNING_GENERALIZATION_PASS=
NOVEL_GOAL_COMPOSITION_PASS=

FULL_ACTION_TREE_ENUMERATION_EVENTS=
WORLD_MEMORY_FULL_SCANS=
CAUSAL_MECHANISM_FULL_SCANS=

REACHABILITY_PLANNING_ABLATION_PASS=
HIERARCHICAL_PLANNING_ABLATION_PASS=
CAUSAL_MODEL_PLANNING_ABLATION_PASS=
UNCERTAINTY_PLANNING_ABLATION_PASS=
CLOSED_LOOP_REPLANNING_ABLATION_PASS=
SPARSE_PLANNING_ABLATION_PASS=
PROCEDURAL_MEMORY_ABLATION_PASS=

TASK_ID_TO_PLAN_LOOKUP_AUTHORITY=
WORLD_HASH_TO_PLAN_LOOKUP_AUTHORITY=
GOAL_HASH_TO_PLAN_LOOKUP_AUTHORITY=

PLANNER_IS_GOAL_SUCCESS_AUTHORITY=

GOLD_ACTION_READS=
GOLD_PLAN_READS=
EXPECTED_GOAL_STATE_LOOKUPS=
FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=

RAW_FIELD_ACCEPTANCE_AUTHORITY=
PRIMARY_SECONDARY_ACCEPTANCE_DIFF=
ACCEPTANCE_FALSE_PASS_EVENTS=

CAPABILITY_FAILURE_FROM_INFRASTRUCTURE_ONLY_EVENTS=

SEM33_R1_LEVEL_A_PASS=
SEM33_R1_LEVEL_B_PASS=
SEM33_R1_LEVEL_C_PASS=
SEM33_R1_LEVEL_D_PASS=
SEM33_R1_LEVEL_E_PASS=
SEM33_R1_LEVEL_F_PASS=
SEM33_R1_LEVEL_G_PASS=
SEM33_R1_LEVEL_H_PASS=

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

SEM34_STARTED=false
NEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY

============================================================
37. INTERPRETATION RULE
============================================================

If transport now works and planner fails a real Level:

that is the first scientifically measured SEM-33 capability failure.

If transport works and Levels A–H all pass:

SEM-33-R1 is PASS.

If infrastructure prevents valid measurement again:

report UNRESOLVED_INFRASTRUCTURE_FAILURE.

Never infer planner capability from an unmeasured run.

Suggested commit:

Repair SEM-33 verifier transport and perform fresh hierarchical planning regate

Start SEM-33-R1 now.
