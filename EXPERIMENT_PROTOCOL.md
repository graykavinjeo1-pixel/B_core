# Initial Concept-Emergence Experiment Protocol

**Canonical version:** 1.0.0
**Applies first to:** SEM-0
**Experiment status:** Not started

## 1. Purpose

This protocol prevents successful task solving, caching, repeated structure,
or leakage from being misreported as autonomous semantic concept emergence.
Every SEM-0 experiment must preregister its primitives, tasks, splits, budgets,
metrics, gates, and contamination checks before evaluating the blind split.

## 2. Required experimental arms

All four arms receive the same permitted primitive semantics, training tasks,
environment rules, verifier contract, task split, and resource-accounting
method. Differences must be limited to the capabilities being tested.

### A. Primitive-only baseline

Solves each task using supplied primitives without persistent solved-instance
answers, reusable mined macros, or promoted concepts. It establishes what the
base operators and search procedure can accomplish.

### B. Solution-cache baseline

May store and retrieve solved instances by declared non-secret observable
keys. It must report cache hits and may not generalize a cached entry into a
new semantic unit. It tests memorization and near-instance retrieval.

### C. Structural-macro baseline

May identify, store, and replay repeated derivation subgraphs under the same
training evidence available to D. It does not receive semantic validation or
promotion gates beyond syntactic/type validity. It tests whether structural
compression alone explains D's results.

### D. Semantic-evolution system

May form candidate abstractions and promote them only after executable,
cross-instance reuse, fresh blind transfer, counterfactual, compression or
reasoning benefit, regression, provenance, and causal-ablation requirements.
All candidates, failures, gate outcomes, and uses must be recorded.

The experiment distinguishes D from C through intervention-sensitive
behavior, fresh transfer, verification, and causal ablation—not labels,
metadata richness, or storage format.

## 3. Fairness and matching

The experiment specification must declare how compute, wall time, memory,
storage, examples, seeds, routing, primitive operators, implementation effort,
and hyperparameter tuning are matched. If an arm receives extra resources or a
necessary structural difference, report it and include a sensitivity analysis.

Routing/indexing may be shared infrastructure. Its performance is measured but
cannot count as evidence for D. The verifier must not expose target-specific
hints to any arm.

## 4. Leakage prohibitions

During a fresh blind concept-emergence test, all of the following are
prohibited:

- target abstraction names, synonyms, or encoded identifiers;
- target formulas;
- target programs, implementations, or algorithm descriptions;
- solution templates or teacher demonstrations containing the solution;
- answer-key lookup during solving;
- network or web access;
- external LLM calls;
- benchmark-specific source branches or solver dispatch;
- hidden runtime fixtures encoding expected answers or target structures;
- expected-answer, output, or solved-test caches not explicitly belonging to
  baseline B;
- automatic recursive-improvement proposal or mutation;
- external provider repair;
- human intervention after the blind evaluation begins.

The evaluator may possess sealed answers solely for scoring after the system
has irreversibly committed its output. Scoring feedback must not flow back into
the same blind run.

## 5. Split and seed policy

Training, candidate-validation, promotion-validation, and final blind-test
instances must be generated or selected before final evaluation. The blind
split must contain genuinely unseen instances and, where applicable, unseen
surface encodings and counterfactual interventions.

Use deterministic declared seeds for reproducibility. The seed list, generator
version, and split hashes must be frozen before blind evaluation. Determinism
does not permit seed shopping: all preregistered seeds are reported, including
failures. A separate hidden seed or generator holdout should test dependence on
the public seed family when feasible.

## 6. Contamination checks

Before and after each blind run, record and verify:

1. network interfaces and external LLM/provider adapters are absent, disabled,
   or blocked;
2. recursive-improvement proposal, apply, merge, commit, and push paths are
   disabled;
3. source and fixture scans find no target formula, program, abstraction name,
   answer table, benchmark ID dispatch, or hidden solution template;
4. runtime traces show no answer-key access, unexpected file read, process
   execution, package installation, or undeclared input;
5. all artifacts loaded by the solver are enumerated and hashed;
6. lexical aliases are removed or randomized when the experiment claims
   language independence;
7. test outputs are committed before sealed scoring;
8. the clean evaluation process starts from only the declared frozen artifacts;
9. cache and macro use are separately logged;
10. generated candidates retain complete provenance to permitted inputs.

A positive contamination finding sets the affected run to `CONTAMINATED`; it
cannot count as pass even if its answers are correct.

## 7. Fresh reconstruction and clean-process evaluation

Final evaluation runs in a fresh process, and preferably a fresh worktree or
sealed container, with an empty transient cache. Install/build artifacts must
be reproducible from the committed source and lockfile. The process receives
only the declared primitive substrate, the permitted promoted generations from
the training/promotion phase, and the blind tasks.

At least one audit run must reconstruct every promoted candidate from its
recorded lineage and evidence or fail closed. Artifact hashes, source commit,
canonical manifest hash, configuration, executable versions, environment, and
seed must be included in the run manifest.

## 8. Promotion gates

Exact numeric thresholds belong in the preregistration. At minimum a candidate
must pass all applicable gates:

1. **Executable semantics:** deterministic or bounded-stochastic behavior can
   be run from the declared reference.
2. **Cross-instance reuse:** the same generation participates successfully in
   multiple non-identical instances.
3. **Fresh blind transfer:** performance on held-out instances exceeds the
   preregistered comparison criterion.
4. **Counterfactual consistency:** predictions change correctly under relevant
   interventions and remain stable under irrelevant ones.
5. **Benefit:** there is a declared compression, reasoning, sample-efficiency,
   or resource benefit.
6. **Regression:** no unacceptable degradation occurs on retained controls.
7. **Provenance:** all inputs, derivations, versions, failures, and evidence are
   complete and uncontaminated.
8. **Causal ablation:** disabling the candidate has the predicted measurable
   effect.

Missing evidence, an indeterminate verifier, or a failed gate means no
promotion.

## 9. Causal ablation protocol

For each claimed semantic concept, run at least:

- **enabled condition:** normal use of the frozen candidate generation;
- **disabled condition:** the candidate is unavailable to routing and
  execution;
- **matched replacement condition:** when feasible, substitute the comparable
  structural macro or primitive reconstruction;
- **unrelated ablation control:** disable a similarly sized irrelevant item to
  estimate generic disruption.

Keep tasks, seeds, budgets, routing policy, and all unrelated state identical.
Report outcome accuracy, verification rate, graph shape, search expansions,
wall time, memory, and failures. `ablation_delta` must state direction,
magnitude, uncertainty, and sample size. A concept removed after its result was
already cached is not a valid ablation; derived caches and active state must be
cleared or reconstructed.

A zero or wrong-direction delta on preregistered relevant tasks blocks the
causal-contribution claim. A positive delta alone does not establish semantics
unless the other gates also pass.

## 10. Counterfactual protocol

Probes must include interventions expected to change behavior and controls
expected not to change it. Probe generation must not receive the answer key.
Score direction, value, invariants, and declared unknowns separately. Include
probes designed to distinguish generalized semantics from replay of the most
similar training subgraph.

## 11. Reporting

Every arm reports task outcomes and the architecture metrics defined in
`REASONING_ARCHITECTURE.md`, plus:

- candidate counts by lifecycle state;
- cache hits and macro uses;
- promotion gate results;
- contamination results;
- supplied versus discovered semantic elements;
- all excluded runs and reasons;
- failures and null results;
- epistemic versus operational cost.

Report individual seeds as well as aggregate statistics. Do not describe a
candidate as a concept before promotion, and do not describe a promoted
candidate as generally semantic beyond its validated scope.

## 12. Fail-closed rules

The final experiment status is `FAIL` or `INCONCLUSIVE`, never an inferred
pass, when any mandatory artifact is missing, hashes differ, leakage cannot be
excluded, the verifier is unavailable, a gate is indeterminate, a split was
seen, source changed during evaluation, or the recursive mutation quarantine
was not active.

Do not lower a gate after observing results. A revised threshold creates a new
preregistered experiment and leaves the original result intact.
