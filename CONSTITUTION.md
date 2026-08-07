# Semantic Reasoning Project Constitution

**Canonical version:** 1.0.0
**Status:** Frozen at Stage S0
**Authority:** Highest-authority project document

## 1. Purpose and scope

This independent project tests whether a computational system can store and
execute meaning independently of language; compose meanings through adaptive,
sparse, graph-structured reasoning; autonomously discover and validate
reusable higher-order concepts; preserve their provenance while compressing
them; and use them to solve progressively more complex problems without being
given the corresponding high-level answers.

This Constitution governs architecture, experiments, evidence, promotion, and
claims. It does not assert that the hypothesis is true. Failure is valid
research output.

## 2. Constitutional rules

### 2.1 Meaning is not language

Human-language strings are not the canonical storage substrate for concepts.

```text
language != concept
```

The system must be capable in principle of possessing and operating on a
concept for which it has no human-readable name. A concept identifier may be
opaque, such as `C000001`. Human labels and translations are optional
inspection metadata and never constitute or control the concept's semantics.

### 2.2 Meaning is not a description field

A definition, documentation string, name, prompt, or embedding alone cannot
constitute a concept. The provisional research hypothesis is that
computational meaning requires an executable combination drawn from:

- invariants;
- relations;
- transformations and transition semantics;
- predictions;
- affordances;
- counterfactual behavior;
- grounding;
- derivation and provenance.

This representation may evolve only through explicit, versioned experimental
evidence and the amendment process in Section 5.

### 2.3 Meaning must do work

A promoted concept must causally contribute to relevant reasoning. Promotion
requires ablation: removing or disabling the concept must produce a measurable
effect on preregistered relevant behavior. A plausible interpretation, a good
label, frequent retrieval, or correlation with success is insufficient.

### 2.4 No supplied high-level answers

An experiment claiming autonomous concept emergence must not supply:

- target formulas or algorithms;
- target abstraction names or implementations;
- solution templates;
- hidden lookup tables;
- expected-answer or solved-instance caches;
- benchmark-specific shortcuts.

Primitive semantics, task definitions, environmental rules, and formal
definitions may be supplied only when the experiment explicitly permits and
records them.

### 2.5 Definitions may be external; solutions may not

A future controlled foraging stage may retrieve definitions, interface
contracts, or primitive semantics. It must not retrieve a solution to the
active experimental problem. Web and network access are off for SEM-0 and may
be opened only by a later authorized stage with contamination controls.

### 2.6 Reasoning depth is adaptive

There is no architectural hard limit of depth five. A simple problem may end
after one operation; a difficult one may require tens or more dependent
operations. Depth five is permitted only as a test band. Resource limits are
budgets, not definitions of reasoning or intelligence. No implementation may
encode `MAX_REASONING_DEPTH = 5` as a constitutional assumption.

### 2.7 Reasoning is graph-structured

Reasoning is not assumed to be one linear chain. The canonical derivation
object must be graph/DAG capable and support, in principle, decomposition,
branching, alternative hypotheses, parallel subproblems, dependency edges,
recombination, contradiction, rollback, recursion, abstraction, and
compression. Cyclic search activity must be represented through versioned
states or events so the preserved derivation remains auditable.

### 2.8 Capability is multidimensional

Capability must not be reduced to node count or reasoning depth. At minimum,
measure separately:

- depth, width, and branch count;
- number of concepts composed;
- active working-set size;
- abstraction and compression effectiveness;
- search expansion cost;
- memory cost and execution time;
- successful fresh transfer;
- counterfactual correctness.

The intended research notion of capability is the complexity of a valid
reasoning graph the system can construct, verify, reuse, and compress under
bounded resources.

### 2.9 Epistemic depth and operational depth differ

A concept may depend historically on a deep derivation while operating as one
reusable unit after validated compression. Every evaluation must distinguish:

- epistemic or historical derivation complexity; and
- current operational reasoning complexity.

Compression must never erase derivation lineage or its verification basis.

### 2.10 Abstraction promotion requires evidence

Repeated subgraphs are candidates, not automatically concepts. Promotion must
eventually require evidence of:

- executable behavior;
- cross-instance reuse;
- fresh blind transfer;
- counterfactual consistency;
- compression or reasoning benefit;
- no unacceptable regression;
- causal contribution demonstrated by ablation.

Exact thresholds are experimental parameters and belong in preregistered
experiment specifications, not in this Constitution.

### 2.11 Cache is not concept

The project must distinguish and report separately:

1. a solved-instance cache;
2. a repeated structural macro;
3. an executable generalized concept;
4. a semantic concept satisfying the stronger validation gates.

Retrieval speed, repeated structure, or compression alone does not establish
semantic status.

### 2.12 Sparse activation is infrastructure, not evidence

Inherited SYNAPSE indexing, routing, and sparse activation may be used to avoid
global scans. Their presence or performance is not evidence of semantic
reasoning, autonomous abstraction, or concept emergence.

### 2.13 Recursive improvement is quarantined

Inherited recursive-improvement machinery must not silently repair, optimize,
rewrite, or mutate the semantic reasoner during the initial concept-emergence
experiment. The initial policy is:

```text
OBSERVE = allowed
MEASURE = allowed
PROPOSE = disabled
APPLY = disabled
AUTO_MUTATION = disabled
AUTO_MERGE = disabled
AUTO_COMMIT = disabled
AUTO_PUSH = disabled
EXTERNAL_PROVIDER_REPAIR = disabled
BENCHMARK_DRIVEN_MUTATION = disabled
```

Recursive self-application may be opened only after autonomous concept
emergence has independently passed its gates and a later stage explicitly
authorizes it.

### 2.14 Language models are non-authoritative

SEM-0 core experiments must not depend on an external language model for
reasoning, concept creation, target-abstraction suggestion, solution search,
or validation. If an LLM is introduced later, it is a bounded adapter or
hypothesis source unless an explicitly approved architecture amendment says
otherwise. Its outputs require independent executable verification.

### 2.15 Verification outranks plausibility

A representation that sounds intelligent but fails execution,
counterfactual testing, blind transfer, causal ablation, or provenance review
is not accepted as a semantic concept.

### 2.16 Failure is valid research output

The system and its evaluation harness must fail closed, preserve evidence, and
report failed gates. Thresholds, controls, or definitions must not be weakened
merely to obtain a passing result.

## 3. Evidence and claim discipline

Claims must state the tested system version, data split, seed policy, resource
budget, controls, success thresholds, failures, exclusions, and known
confounds. Successful task solving alone is never proof of concept emergence.
Post-hoc interpretations are hypotheses until validated by fresh tests.

Negative and null results must be retained. Selective reporting, silent seed
discarding, outcome-dependent threshold changes, and hidden human intervention
invalidate the affected claim.

## 4. Stage S0 and SEM-0 boundary

Stage S0 is documentation, inspection, isolation, and baseline freezing only.
It may add canonical documents, inventories, integrity verification, and the
minimum quarantine gate. It must not implement semantic learning, concept
mining, abstraction promotion, adaptive reasoning search, mathematics
reasoning, program synthesis, active inference, web foraging, language
grounding, or self-application.

SEM-0 may begin only after S0 validation passes. SEM-0 is limited to a small,
closed-world executable substrate and a controlled attempt to prove or falsify
at least one reusable concept beyond cache and macro baselines.

## 5. Authority and amendments

If a later prompt, comment, report, generated artifact, implementation detail,
or inherited SYNAPSE document conflicts with this Constitution, this
Constitution wins unless a human explicitly authorizes a constitutional
amendment.

Every amendment must:

1. record the old text;
2. record the new text;
3. record the rationale;
4. quote or identify the authorizing human instruction;
5. increment the canonical version;
6. update the canonical manifest;
7. identify and invalidate or review all affected experiments.

Silent semantic mutation, reinterpretation by implementation, and amendment by
generated report are prohibited.
