SEMANTIC REASONING PROJECT — SEM-36
AUTONOMOUS EPISTEMIC FRONTIER DISCOVERY
SELF-GENERATED SCIENTIFIC QUESTIONS FROM WORLD-MODEL GAPS

Continue ONLY the independent B_Core / Semantic Reasoning Project lineage.

Start from the exact sealed SEM-35-R1 predecessor:

SEALED_PREDECESSOR_COMMIT=
2b2d7b6ecc48b6b677a2fc8ac3277c41353b968f

Verify exact predecessor integrity.

Do NOT introduce quantum-inspired representation mechanisms.

Do NOT introduce vision/audio/sensor grounding.

Do NOT push unless explicitly authorized.

Do NOT start SEM-37 automatically.

0. CENTRAL SCIENTIFIC QUESTION

SEM-35-R1 established a persistent semantic world model with:

persistent semantic state

relational causal dynamics

partial-observation belief

uncertainty

counterfactual prediction

goal-directed hierarchical planning

sparse scalable planning

variable-duration temporal processes

dynamic semantic long-term memory

SEM-36 asks:

Can B_Core autonomously identify where its own world model is incomplete, uncertain, contradictory, poorly predictive, or unnecessarily complex — without a human specifying the research question — and then select useful scientific questions and experiments that improve the model?

The target transition is:

Human:
"Investigate mechanism X"

            ↓ remove this dependency

B_Core:
World Model
    ↓
"Something here is not explained."
    ↓
"What do I need to discover?"
1. NO HUMAN RESEARCH QUESTION

The canonical campaign must NOT provide:

the missing causal law

the missing property

the unknown relation

the correct hypothesis family

the experiment to run

the variable to intervene on

Human provides only a safe closed world and scientific acceptance contract.

Required:

HUMAN_RESEARCH_QUESTION_SELECTION_EVENTS=0
HUMAN_HYPOTHESIS_SELECTION_EVENTS=0
HUMAN_EXPERIMENT_SELECTION_EVENTS=0
2. EPISTEMIC FRONTIER

Derive or implement a concept equivalent in purpose to:

EpistemicFrontier

representing locations where current world knowledge is insufficient.

Potential evidence MAY include:

persistent prediction residual

high epistemic uncertainty

competing causal hypotheses

unexplained correlation

context-dependent inconsistency

failed counterfactual prediction

unexplained temporal boundary

new entity/property residual

failure of an existing semantic law

excessive exceptions to one law

poor compression / repeated unexplained structure

Do NOT hard-code this list as the only frontier types.

3. UNKNOWN IS NOT ERROR

Distinguish:

MODEL_ERROR

Genuine world stochasticity

Insufficient evidence

Currently unknown mechanism

Measurement noise

Out-of-model phenomenon

Do not automatically treat every prediction mismatch as a new law.

4. NOISE MUST NOT BECOME A SCIENCE TARGET

Repeated irreducible stochasticity must not attract unlimited research effort merely because prediction error remains high.

Required:

IRREDUCIBLE_NOISE_RESEARCH_LOOPS=0

where canonical ground truth supports the distinction.

5. EPISTEMIC VALUE

B_Core must determine which unknowns are worth investigating.

A useful frontier may be important because resolving it would improve:

prediction

counterfactual accuracy

causal explanation

planning

compression

transfer

uncertainty reduction

ability to explain several residuals at once

Do NOT impose a fixed human-weighted scalar formula.

6. SELF-GENERATED SCIENTIFIC QUESTION

For a selected frontier, generate a semantic research question equivalent to:

"What unknown mechanism explains residuals R1,R2,R3?"

"Which of hypotheses H1,H2,H3 governs this transition?"

"What hidden state is required for existing laws to remain consistent?"

"Is this apparent new phenomenon actually a composition of existing laws?"

Natural language formulation is report-only.

Canonical scientific question must remain semantic.

Required:

NATURAL_LANGUAGE_IS_RESEARCH_QUESTION_AUTHORITY=false
7. EXPLANATION BEFORE PRIMITIVE GENESIS

Before inventing new semantic knowledge:

attempt explanation using:

existing semantic atoms

existing causal laws

existing temporal processes

new composition

new relation topology

context refinement

applicability refinement

Only irreducible residuals justify new primitive/mechanism genesis.

8. COMPETING HYPOTHESES

For an unexplained frontier:

generate multiple distinct explanatory hypotheses where evidence permits.

Track:

HYPOTHESES_GENERATED
HYPOTHESES_REJECTED
HYPOTHESES_RETAINED

Do NOT reward large hypothesis count by itself.

9. HYPOTHESIS TYPES ARE OPEN

The system may autonomously hypothesize equivalents of:

missing property

missing relation

hidden state

missing causal mechanism

law applicability condition

new temporal process

interaction between existing mechanisms

new semantic primitive

existing law is wrong

Do NOT preselect which kind is correct.

10. EXPLANATORY COMPRESSION

A strong hypothesis should preferably explain multiple observations/residuals with less independent semantic structure.

Measure:

RESIDUALS_EXPLAINED

SEMANTIC_STRUCTURE_ADDED

EXCEPTIONS_REQUIRED

Do NOT simply optimize for smaller models if predictive distinctions are lost.

11. EXPERIMENT AS HYPOTHESIS DISCRIMINATION

Experiment selection should answer:

Which safe intervention or observation would best distinguish the currently plausible explanations?

Not merely:

Which action is novel?

Use existing planning/intervention machinery.

12. EXPERIMENT PREDICTIONS MUST BE FROZEN FIRST

Before executing an experiment:

each relevant hypothesis must make an explicit predicted semantic outcome or uncertainty prediction.

Required ordering:

hypotheses
↓
predictions frozen
↓
experiment selected
↓
experiment executed
↓
observation revealed
↓
hypotheses updated

Required:

EXPERIMENT_OUTCOME_READS_BEFORE_PREDICTION=0
13. INFORMATION GAIN IS NOT THE ONLY VALUE

An experiment that distinguishes hypotheses but has enormous cost may be inferior to a cheaper sufficiently discriminative experiment.

Allow bounded tradeoffs among:

hypothesis discrimination

uncertainty reduction

experiment cost

world disturbance

future scientific utility

Do NOT impose a universal formula.

14. ACTIVE INTERVENTION

Where observational evidence cannot identify causal structure:

B_Core may autonomously intervene on the safe closed world.

Carry forward:

observation != intervention

Interventional data can reveal causal structure unavailable from passive correlation; this is a mechanism-level principle supported by causal representation learning research.

15. NEGATIVE EXPERIMENTS ARE KNOWLEDGE

If an experiment disproves every current hypothesis:

do NOT call the experiment useless.

Store:

hypothesis family failed

predicted outcomes failed

new residual structure

constraints on future theories

Negative evidence must narrow future research.

16. THEORY REVISION

Allow:

LAW_REFINEMENT

LAW_SPLIT

LAW_MERGE

LAW_COMPOSITION

LAW_DEMOTION

NEW_LAW_GENESIS

NEW_PROPERTY_GENESIS

NEW_RELATION_GENESIS

NEW_TEMPORAL_PROCESS_GENESIS

only when evidence requires it.

Do not require all event types.

17. MECHANISM-CENTRIC KNOWLEDGE

Prefer reusable explanatory mechanism over episode-specific prediction rule.

Forbidden scientific success:

exact state hash
→ expected next state

A valid discovered mechanism must expose semantic applicability.

18. DISCOVERY MUST PREDICT SOMETHING NEW

A new mechanism/law is not validated merely because it explains past observations.

Before promotion:

derive at least one fresh consequence not used to create the mechanism.

Freeze prediction.

Then expose fresh test.

Required:

NOVEL_PREDICTION_BEFORE_VALIDATION=true
19. FRESH PREDICTION GATE

Strong discovery evidence requires:

old model:
cannot correctly predict fresh phenomenon

new hypothesis:
predicts fresh phenomenon

independent verifier:
confirms prediction

Track:

NOVEL_PREDICTIONS
NOVEL_PREDICTIONS_VERIFIED
NOVEL_PREDICTION_ERRORS
20. EXPLANATION VS FITTING

Construct cases where two models fit the current observed evidence equally well but imply different unseen outcomes.

The discovered model must survive fresh discrimination.

Do not count retrospective fit as discovery.

21. COUNTERFACTUAL CONSEQUENCES

New causal mechanisms should generate counterfactual consequences where meaningful.

Validate selected cases through independent branch realization.

This tests whether the mechanism carries causal meaning rather than statistical description.

22. TRANSFER

A discovered scientific mechanism must be tested on:

new entity identity

new parameter/state binding

new relation topology

new temporal context

where its semantics should apply.

Do not require transfer where applicability conditions are false.

23. SCIENTIFIC ANTI-OVERGENERALIZATION

Create superficially similar fresh cases where one semantic condition invalidates the law.

Required:

SCIENTIFIC_OVERGENERALIZATION_EVENTS=0
24. WORLD-MODEL GAP MEMORY

Store epistemic-frontier research history semantically:

what was unknown

why it was selected

hypotheses considered

experiments attempted

negative results

discovered mechanism

remaining exceptions

Do not store only natural-language lab notes.

25. RESEARCH EXPERIENCE COMPRESSION

Reuse SEM-29/30 mechanisms.

Repeated useful patterns in:

frontier diagnosis

hypothesis generation

experiment design

theory revision

may become reusable research motifs/laws/compressed semantic memory.

Do NOT force promotion.

26. RESEARCH METHOD MUST NOT OVERRIDE EVIDENCE

A learned research heuristic may route attention.

It cannot declare a scientific claim true.

Independent world evidence remains authority.

27. OPEN FRONTIER SELECTION

The world fixture must contain several possible unknowns simultaneously.

B_Core must choose what to investigate first.

Human must not rank them.

Track:

AVAILABLE_EPISTEMIC_FRONTIERS

EPISTEMIC_FRONTIERS_SELECTED

FRONTIER_SELECTION_SEQUENCE
28. FRONTIER VALUE TEST

Include:

easy but scientifically low-value unknown

harder but high-explanatory-value unknown

irreducible noisy phenomenon

redundant already-explained phenomenon

B_Core must not simply chase largest raw prediction error.

29. AUTONOMOUS STOP

A research question may terminate as:

DISCOVERED

CURRENTLY_UNIDENTIFIABLE

INSUFFICIENT_EVIDENCE

IRREDUCIBLE_STOCHASTICITY

RESOURCE_LIMIT

MODEL_CLASS_LIMIT

Do not fabricate conclusions to close every frontier.

30. SELF-GENERATED NEXT QUESTION

After one discovery:

update the world model.

Then allow the updated model to expose a new epistemic frontier.

A strong campaign should demonstrate:

discovery D1
↓
world model improves
↓
new question Q2 becomes visible

without human curriculum design.

31. DISCOVERY CHAIN

Do NOT require a fixed number.

But observe whether the system can produce a chain such as:

Q1
→ experiment
→ law L1

L1
→ exposes residual R2
→ Q2
→ experiment
→ law L2

L1 + L2
→ novel prediction

This is the beginning of autonomous scientific exploration.

32. NO SYNTHETIC DIFFICULTY INFLATION

SEM-36 research value is not measured by making problems arbitrarily harder.

The scientific target is:

world-model knowledge gain

not difficulty labels.

33. INFORMATION EFFICIENCY

Track per discovery:

observations consumed

interventions executed

hypotheses generated

experiments executed

semantic bytes added

prediction improvement

Do not optimize any single metric alone.

34. SCIENTIFIC COMPRESSION PRODUCTIVITY

Track:

previous unexplained residual count

residual count after discovery

new independent semantic structure

future predictions enabled

A powerful law may deserve promotion even if it required substantial research cost.

35. MECHANISTIC DISCOVERY PRINCIPLE

Prediction accuracy alone is insufficient.

A strong discovery should organize knowledge into a reusable mechanism capable of supporting:

prediction

intervention

counterfactual

transfer

compression

This mechanism-centric distinction aligns with recent work arguing that autonomous discovery requires explanatory mechanisms rather than only predictive mappings.

36. EXISTING AUTONOMOUS CURRICULUM IS NOT THE SCIENTIFIC TARGET

Do NOT turn the world into another artificial difficulty generator.

SEM-28/29 curriculum mechanisms may assist research routing.

But SEM-36 questions must originate from:

world-model epistemic state

not from a requirement to create harder tasks.

Required:

SCIENTIFIC_QUESTION_FROM_DIFFICULTY_GENERATOR_EVENTS=0
37. CLOSED WORLD FOR NOW

SEM-36 remains:

offline

local

synthetic

mechanically verifiable

safe

No internet.

No external lab.

No physical actuator.

This isolates scientific cognition from perception/embodiment.

38. MULTIPLE WORLD FAMILIES

Use multiple structurally distinct safe world families.

Some mechanisms should transfer.

Some should not.

This prevents scientific reasoning from collapsing into one fixture family.

39. HIDDEN MECHANISMS

Canonical world generators may contain ground-truth mechanisms unknown to B_Core.

The independent verifier may access them.

B_Core may not.

Required:

WORLD_GROUND_TRUTH_MECHANISM_READS=0
40. NO GOLD HYPOTHESIS

Required:

GOLD_HYPOTHESIS_READS=0
GOLD_EXPERIMENT_READS=0
EXPECTED_DISCOVERY_LOOKUPS=0
41. DISCOVERY FRESHNESS

Freeze before canonical research:

world generator

hidden mechanism family

verifier

fresh world seeds

scientific acceptance

research engine

No manual repair after final fresh exposure.

42. BASELINE

Before autonomous epistemic-frontier research:

run the sealed SEM-35-R1 system with no new SEM-36 mechanism.

Measure:

passive prediction capability

existing causal discovery capability

ability to notice unknowns

ability to select own research question

Do not assume SEM-36 mechanisms are needed.

43. AUTONOMOUS RESEARCH ONLY AFTER MEASURED GAP

If baseline already meets all gates:

PASS without inventing new machinery.

Otherwise:

observe measured epistemic limitation
↓
autonomous diagnosis
↓
candidate research-method repairs
↓
causal experiment
↓
generic repair

No human architecture selection.

44. REQUIRED ABLATION — FRONTIER SELECTION

Remove explicit epistemic-frontier selection while preserving world-model knowledge.

Compare discovery efficiency / useful discoveries.

Required for frontier-selection benefit claim:

EPISTEMIC_FRONTIER_SELECTION_ABLATION_PASS=true
45. REQUIRED ABLATION — INTERVENTION

For a confounded causal problem compare:

observation-only
vs
autonomous intervention

Required:

SCIENTIFIC_INTERVENTION_ABLATION_PASS=true
46. REQUIRED ABLATION — COMPETING HYPOTHESES

Force premature single-hypothesis commitment.

Compare against maintained competing hypotheses.

Required for benefit claim:

COMPETING_HYPOTHESIS_ABLATION_PASS=true
47. REQUIRED ABLATION — MECHANISTIC MEMORY

Disable promoted discovered mechanism while retaining raw episode history.

Fresh transfer/prediction should worsen if the mechanism is causal.

Required:

DISCOVERED_MECHANISM_MEMORY_ABLATION_PASS=true
48. REQUIRED ABLATION — NEGATIVE KNOWLEDGE

Remove failed-hypothesis/negative-experiment memory.

Measure whether repeated wasted research increases.

Required for negative-memory benefit claim:

NEGATIVE_SCIENTIFIC_MEMORY_ABLATION_PASS=true
49. SCIENTIFIC FAILURE IS VALID

If the hidden mechanism is not identifiable under the available interventions:

B_Core should say so.

Correct:

CURRENTLY_UNIDENTIFIABLE

Incorrect:

invent a convenient law
50. TRANSPORT / NUMERIC AUTHORITY

Carry forward SEM-35-R1 exact numeric discipline:

NUMERIC_AUTHORITY_MANIFEST_PRESENT=true

DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY=false

GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE=false

VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE=true

DETERMINISTIC_RECOMPUTATION_DIFF=0
51. SPARSE RESEARCH

World model may be large.

Scientific investigation must activate only relevant semantic areas.

Required:

WORLD_MEMORY_FULL_SCANS=0
CAUSAL_MECHANISM_FULL_SCANS=0
TEMPORAL_MEMORY_FULL_SCANS=0

Track active semantic field per research question.

52. HARD CEILING

Use exactly:

MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096

as containment only.

Required:

REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096
CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096
CAMPAIGN_BUDGET_CONTRACT_PASS=true

Stop early on complete evidence or valid autonomous stop.

53. NO QUANTUM-INSPIRED WORK

Required:

QIS0_EXECUTED=false
QUANTUM_INSPIRED_CORE_CHANGES=0

The quantum-inspired backlog remains deferred until the core world model is substantially mature and stable.

54. LEVEL A — SELF-DETECTED EPISTEMIC FRONTIER

B_Core identifies genuine world-model gaps without a human specifying what is missing.

55. LEVEL B — AUTONOMOUS SCIENTIFIC QUESTION

B_Core converts an epistemic gap into a meaningful semantic research question and selects among several available frontiers.

56. LEVEL C — HYPOTHESIS GENERATION

B_Core generates competing explanatory hypotheses that imply discriminable outcomes.

57. LEVEL D — AUTONOMOUS EXPERIMENT DESIGN

B_Core selects and executes useful safe interventions/observations without human experiment choice.

58. LEVEL E — MECHANISM DISCOVERY

Evidence causes a reusable semantic causal mechanism/law to be refined, composed, or created.

59. LEVEL F — NOVEL PREDICTION

The discovered mechanism correctly predicts fresh unseen consequences that were not used to construct it.

60. LEVEL G — TRANSFER / ANTI-OVERGENERALIZATION

The mechanism transfers where semantic applicability holds and is rejected where it does not.

61. LEVEL H — CAUSAL SCIENTIFIC VALIDATION

Relevant ablations show that frontier selection, intervention, hypothesis maintenance, and discovered semantic memory causally contribute to the observed scientific performance.

Core SEM-36 PASS requires Levels A–H.

62. STRONGER OPTIONAL OBSERVATION

Report:

AUTONOMOUS_SCIENTIFIC_DISCOVERY_LOOP_OBSERVED=true

only if the full cycle occurs:

self-detected unknown
→ self-generated question
→ hypotheses
→ experiment
→ new mechanism
→ novel verified prediction
→ updated world model
→ new epistemic frontier

without human research steering.

63. DO NOT CLAIM NOVEL REAL-WORLD SCIENCE

SEM-36 operates in controlled synthetic worlds.

PASS demonstrates an autonomous scientific-discovery mechanism in that environment.

It does NOT establish discovery of previously unknown real-world physics/science.

64. REQUIRED OUTPUT

Return at minimum:

SEM36_STATUS=PASS|FAIL
DISPOSITION=

CAMPAIGN_ID=

BRANCH=
COMMIT=
WORKTREE_CLEAN=
PUSH_PERFORMED=

SEALED_PREDECESSOR_COMMIT=
PREDECESSOR_INTEGRITY=

SELF_DETECTED_EPISTEMIC_FRONTIERS=

AVAILABLE_EPISTEMIC_FRONTIERS=
EPISTEMIC_FRONTIERS_SELECTED=

HUMAN_RESEARCH_QUESTION_SELECTION_EVENTS=
HUMAN_HYPOTHESIS_SELECTION_EVENTS=
HUMAN_EXPERIMENT_SELECTION_EVENTS=

AUTONOMOUS_SCIENTIFIC_QUESTIONS=

HYPOTHESES_GENERATED=
HYPOTHESES_REJECTED=
HYPOTHESES_RETAINED=

EXPERIMENTS_PROPOSED=
EXPERIMENTS_EXECUTED=
INTERVENTIONS_EXECUTED=

EXPERIMENT_OUTCOME_READS_BEFORE_PREDICTION=

IRREDUCIBLE_NOISE_RESEARCH_LOOPS=

LAW_REFINEMENT_EVENTS=
LAW_SPLIT_EVENTS=
LAW_MERGE_EVENTS=
LAW_COMPOSITION_EVENTS=
NEW_CAUSAL_LAW_GENESIS_EVENTS=

NEW_PROPERTY_GENESIS_EVENTS=
NEW_RELATION_GENESIS_EVENTS=
NEW_TEMPORAL_PROCESS_GENESIS_EVENTS=

NOVEL_PREDICTIONS=
NOVEL_PREDICTIONS_VERIFIED=
NOVEL_PREDICTION_ERRORS=

COUNTERFACTUAL_DISCOVERY_VALIDATIONS=

DISCOVERED_MECHANISM_TRANSFER_EVENTS=
SCIENTIFIC_OVERGENERALIZATION_EVENTS=

RESIDUALS_BEFORE_DISCOVERY=
RESIDUALS_AFTER_DISCOVERY=

SEMANTIC_BYTES_ADDED_BY_DISCOVERY=
FUTURE_PREDICTIONS_ENABLED=

RESEARCH_QUESTIONS_TERMINATED_DISCOVERED=
RESEARCH_QUESTIONS_TERMINATED_UNIDENTIFIABLE=
RESEARCH_QUESTIONS_TERMINATED_NOISE=
RESEARCH_QUESTIONS_TERMINATED_RESOURCE_LIMIT=

AUTONOMOUS_SCIENTIFIC_DISCOVERY_LOOP_OBSERVED=

EPISTEMIC_FRONTIER_SELECTION_ABLATION_PASS=
SCIENTIFIC_INTERVENTION_ABLATION_PASS=
COMPETING_HYPOTHESIS_ABLATION_PASS=
DISCOVERED_MECHANISM_MEMORY_ABLATION_PASS=
NEGATIVE_SCIENTIFIC_MEMORY_ABLATION_PASS=

SCIENTIFIC_QUESTION_FROM_DIFFICULTY_GENERATOR_EVENTS=

WORLD_GROUND_TRUTH_MECHANISM_READS=
GOLD_HYPOTHESIS_READS=
GOLD_EXPERIMENT_READS=
EXPECTED_DISCOVERY_LOOKUPS=

WORLD_MEMORY_FULL_SCANS=
CAUSAL_MECHANISM_FULL_SCANS=
TEMPORAL_MEMORY_FULL_SCANS=

QIS0_EXECUTED=
QUANTUM_INSPIRED_CORE_CHANGES=

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

SEM36_LEVEL_A_PASS=
SEM36_LEVEL_B_PASS=
SEM36_LEVEL_C_PASS=
SEM36_LEVEL_D_PASS=
SEM36_LEVEL_E_PASS=
SEM36_LEVEL_F_PASS=
SEM36_LEVEL_G_PASS=
SEM36_LEVEL_H_PASS=

SEM37_STARTED=false
NEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY

Suggested commit:

Establish autonomous epistemic frontier discovery from semantic world-model gaps

Start SEM-36 now.

