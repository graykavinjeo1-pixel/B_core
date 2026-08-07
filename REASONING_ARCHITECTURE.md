# Adaptive Semantic Reasoning Graph Engine

**Canonical version:** 1.0.0
**Implementation status:** Not started
**Name:** Adaptive Semantic Reasoning Graph Engine (ASRGE)

## 1. Architectural intent

ASRGE is the conceptual reasoning architecture for future stages. It builds,
executes, verifies, and optionally compresses typed reasoning graphs over an
active subset of the semantic substrate. It is not a Depth-5 Reasoner and has
no fixed architectural reasoning depth.

The architecture separates routing from reasoning, execution from
verification, candidate discovery from promotion, and operational compression
from historical provenance. S0 defines these boundaries only; it does not
implement them.

## 2. Canonical derivation model

A run produces a versioned derivation graph. Nodes may be goals, subgoals,
concept generations, primitive operators, observations, state snapshots,
hypotheses, counterfactuals, contradictions, verification events, rollbacks,
or conclusions. Directed typed edges encode requirements, transformations,
support, conflict, decomposition, recombination, and provenance.

The persisted derivation is DAG-capable. Runtime recursion and revisitation are
captured through versioned state/event nodes rather than erased cycles. A
linear trace is a valid special case, not the default assumption.

## 3. Conceptual components

1. **Goal Encoder / Goal State** — converts an experiment-defined goal into a
   typed target state without smuggling a solution or target abstraction.
2. **Sparse Concept Router** — retrieves a bounded candidate set by typed and
   structural relevance; a routing miss must not silently trigger a global
   semantic scan.
3. **Active Working Set** — holds the currently relevant concepts, states,
   constraints, and evidence separately from the total store.
4. **Candidate Operator Generator** — proposes applicable primitive or
   validated concept operations without answer-key access.
5. **Reasoning Graph / Derivation Graph** — records all dependencies,
   executions, alternatives, failures, and verification events.
6. **Branch Manager** — creates, prioritizes, suspends, resumes, compares, and
   retires alternative hypotheses under budgets.
7. **Decomposer** — converts eligible goals into dependency-linked subgoals.
8. **Recombiner** — joins verified subresults and checks interface,
   consistency, and dependency conditions.
9. **Executor / Simulator** — applies executable semantics to explicit state
   and emits reproducible transitions and predictions.
10. **Verifier** — independently checks preconditions, invariants, outputs,
    contradictions, and experiment constraints.
11. **Counterfactual Probe Generator** — creates bounded interventions that
    discriminate candidate semantics from memorized correlation or macros.
12. **Stagnation Detector** — detects repeated states, non-progress, exhausted
    branches, and unproductive expansion without inventing success.
13. **Resource Controller** — allocates and records soft budgets for search,
    working state, time, and memory.
14. **Concept Miner** — in later authorized stages, identifies candidate
    reusable subgraphs or behaviors; repetition alone never promotes them.
15. **Consolidator** — constructs a versioned compressed operational form while
    retaining a reversible link to full lineage.
16. **Promotion Gate** — applies preregistered evidence thresholds and fails
    closed on missing, regressive, or contaminated evidence.
17. **Provenance Store** — preserves immutable inputs, versions, splits, seeds,
    derivations, validation, failures, ablations, and compression mappings.

## 4. Control flow

Conceptually, a run:

1. encodes a typed goal and initial state;
2. routes a bounded relevant working set;
3. generates applicable operators and decompositions;
4. expands one or more branches in the reasoning graph;
5. executes and verifies transitions;
6. generates counterfactual probes when evidence is insufficient;
7. recombines verified subresults or rolls back failed branches;
8. adapts resource allocation based on progress and uncertainty;
9. terminates with verified success, explicit exhaustion, budget exhaustion,
   contamination failure, or an unknown result;
10. records the complete derivation and metrics.

Concept mining, consolidation, and promotion are later-stage post-run paths,
not implicit side effects of successful solving.

## 5. Adaptive resource controls

The Resource Controller manages soft, explicit, per-run budgets for:

- dependent-operation depth;
- frontier width;
- number of live and total branches;
- active concept count;
- wall-clock time;
- resident and peak memory;
- search expansion count.

Initial values may be conservative test bands. The controller may increase,
decrease, or redistribute budgets within a preregistered envelope based on
verified progress, branching uncertainty, stagnation, and remaining resources.
Budgets must be recorded in both requested and consumed form.

A budget exhaustion result is not evidence that the problem is invalid, and a
large budget is not evidence of intelligence. No budget may be disguised as a
semantic or constitutional fixed depth. In particular, depth five is not an
architectural ceiling.

## 6. Branching, rollback, and contradiction

Branches have explicit parentage, assumptions, resource usage, and status.
Contradictory branches may coexist until evidence resolves them. Rejection must
retain the cause and verifier result. Rollback restores a prior explicit state
and records the discarded branch; it never deletes failed evidence.

Recombination requires verified interface compatibility and cannot average
away contradictions. Recursive decomposition must have progress or resource
guards and must remain representable in the derivation history.

## 7. Compression and epistemic history

A validated composite may later execute as one operational node. Its metrics
must report both the compressed operational path and the full epistemic
derivation. Compression is acceptable only if semantic equivalence, lineage
recovery, and verification replay pass. A cached solved instance or unvalidated
macro is recorded under its own kind and cannot take the promoted-concept path.

## 8. Required metrics

Every relevant run must make the following fields available, using `null` with
a reason when a metric does not apply:

| Metric | Definition |
|---|---|
| `successful_depth_max` | Maximum verified dependency depth among successful outputs. |
| `successful_depth_mean` | Mean verified dependency depth over successful outputs. |
| `reasoning_width_max` | Maximum number of nodes at a derivation depth/frontier. |
| `live_branches_max` | Peak concurrently live alternative branches. |
| `concepts_composed_max` | Maximum distinct semantic units composed in a verified derivation. |
| `peak_active_concepts` | Peak concept count in the active working set. |
| `search_expansions` | Candidate graph expansions attempted. |
| `rollback_count` | Explicit branch/state rollbacks. |
| `decomposition_count` | Accepted goal-to-subgoal decompositions. |
| `recombination_count` | Attempted and successful recombinations, reported separately. |
| `abstraction_count` | Candidate and promoted abstractions, reported separately. |
| `compression_ratio` | Historical derivation cost divided by operational reuse cost under a declared cost model. |
| `fresh_transfer_gain` | Preregistered held-out performance gain over the matched baseline. |
| `counterfactual_accuracy` | Correct intervention-sensitive predictions over eligible probes. |
| `ablation_delta` | Change in relevant outcome when the candidate is disabled, with direction and uncertainty. |
| `wall_time` | End-to-end elapsed time with environment metadata. |
| `memory` | Peak/resident memory and included/excluded stores. |

Additional mandatory context includes total concept count, active-set recall,
budget values, termination reason, cache hits, macro uses, concept generations,
seed, split, and system version.

## 9. Failure semantics

Valid terminal states include `VERIFIED_SUCCESS`, `UNVERIFIED`, `UNKNOWN`,
`EXHAUSTED`, `BUDGET_EXHAUSTED`, `CONTRADICTION_UNRESOLVED`, `CONTAMINATED`, and
`VERIFIER_FAILURE`. Only verified success may count as task success. The engine
must not broaden search through forbidden data sources, weaken verification,
or promote a candidate in order to escape failure.
