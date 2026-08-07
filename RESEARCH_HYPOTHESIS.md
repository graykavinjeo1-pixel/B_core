# Falsifiable Research Hypotheses

**Canonical version:** 1.0.0
**Stage:** S0 foundation; no hypothesis is yet confirmed

## Evaluation convention

Each hypothesis is evaluated against preregistered tasks, controls, held-out
splits, resource budgets, and thresholds. A supporting observation raises
confidence but does not prove universal truth. A falsifying result rejects the
tested version and conditions; it must not be hidden by post-hoc relabeling.

## H1 — Executable Meaning Hypothesis

**Claim.** A useful computational concept can be represented as more than
stored samples, text, a description field, or an embedding and can possess
executable semantic structure.

**Supporting observation required.** A language-independent ConceptIR object
executes predictions or transformations across multiple instances, passes
counterfactual probes, and remains usable without its lexical aliases.

**Falsifying result.** Under matched resources, removal of textual labels and
instance lookup destroys all utility, or the candidate cannot make correct
novel executable predictions beyond stored examples.

**Confounds.** Hidden answer keys, benchmark identifiers, overexpressive
primitive operators, evaluator leakage, embeddings that encode labels, or a
human-written executable implementation masquerading as discovery.

**Required controls.** Label-scrambled and label-free runs, cache baseline,
primitive-only baseline, fresh instances, executable verification, and
candidate ablation.

## H2 — Autonomous Abstraction Hypothesis

**Claim.** Repeated successful derivations can yield reusable concepts not
explicitly supplied by a teacher.

**Supporting observation required.** From primitive semantics and training
episodes alone, the system constructs a candidate whose generalized behavior
was not supplied, promotes it under fixed gates, and improves fresh blind
transfer.

**Falsifying result.** Candidates merely reproduce supplied templates or seen
solutions, fail fresh transfer, or require a human/LLM to name, code, select,
or validate the target abstraction.

**Confounds.** Target-bearing filenames, fixtures, prompts, DSL operators,
tests, hard-coded branches, answer caches, seed selection, or evaluator
feedback revealing the abstraction.

**Required controls.** Source and fixture scan, primitive-only and cache
baselines, sealed test split, clean-process reconstruction, provenance audit,
and no-network/no-LLM execution.

## H3 — Semantic-vs-Macro Hypothesis

**Claim.** A representation validated through prediction, counterfactual
behavior, transfer, and ablation generalizes better than raw cached solutions
or unvalidated repeated macros.

**Supporting observation required.** The semantic-evolution system exceeds
matched cache and structural-macro baselines on preregistered fresh transfer
and counterfactual accuracy, with positive causal ablation and no unacceptable
regression.

**Falsifying result.** A macro or cache baseline matches the candidate on fresh
transfer and counterfactual tests, or candidate ablation has no relevant
effect.

**Confounds.** Unequal compute, storage, training examples, routing quality,
operator expressiveness, tuning effort, or semantic system access to extra
metadata.

**Required controls.** Four-arm matched experiment, identical primitives and
budgets, blinded scoring, representation inspection, ablation, and confidence
intervals across deterministic declared seeds.

## H4 — Recursive Concept Ladder Hypothesis

**Claim.** A promoted concept can become an input primitive for later reasoning
and participate in discovery of higher-order concepts.

**Supporting observation required.** A second-generation promoted concept has
an auditable dependency on at least one autonomously promoted earlier concept,
passes its own fresh transfer and counterfactual gates, and reduces operational
cost relative to primitive reconstruction.

**Falsifying result.** Later candidates do not use earlier concepts causally,
lineage is fabricated or lost, or every generation must be reconstructed or
supplied independently.

**Confounds.** Renaming a flat macro hierarchy, manually authored generation
links, cached composite answers, or hidden primitives equivalent to the later
concept.

**Required controls.** Lineage audit, ancestor ablation, primitive-only
reconstruction, generation holdouts, and operational-versus-epistemic cost
measurement.

## H5 — Adaptive Complexity Hypothesis

**Claim.** The reasoning engine can dynamically allocate varying depth, width,
branching, and composition according to task demands rather than using a fixed
reasoning depth.

**Supporting observation required.** Across tasks with independently measured
graph demands, successful traces show materially different depths, widths,
branch counts, and working-set sizes while respecting resource budgets.

**Falsifying result.** Search shape is effectively fixed, success collapses
outside a narrow depth band such as five, or reported adaptivity is only
padding/truncation without causal task dependence.

**Confounds.** Task order correlated with budget, hidden per-benchmark limits,
precomputed plans, or a router selecting fixed solvers.

**Required controls.** Mixed-order task suite, shape-specific ablations,
budget sweeps, trace validation, and comparison with fixed-depth/fixed-width
baselines.

## H6 — Compression-with-Provenance Hypothesis

**Claim.** Deep historical derivations can be compressed into low
operational-cost concepts without losing provenance or verification basis.

**Supporting observation required.** A promoted concept executes with lower
operational graph cost while its full derivation lineage, evidence hashes, and
verification results remain recoverable and replayable.

**Falsifying result.** Compression discards necessary lineage, cannot reproduce
verification, changes semantics silently, or yields no operational benefit.

**Confounds.** Moving cost to an unmeasured cache, excluding retrieval cost,
retaining opaque non-auditable state, or comparing unequal implementations.

**Required controls.** End-to-end cost accounting, lineage replay, generation
hash verification, decompression/reconstruction checks, and semantic
equivalence tests.

## H7 — Sparse Semantic Scaling Hypothesis

**Claim.** As total concept count grows, reasoning cost can remain primarily
dependent on the active relevant working set rather than total stored concept
count.

**Supporting observation required.** Holding active relevant content and task
difficulty approximately constant while increasing irrelevant concept count
causes sublinear or bounded degradation within a preregistered scaling model,
without lost correctness.

**Falsifying result.** Search, memory, or latency scales primarily with total
concept count; routing misses require full scans; or sparse scaling preserves
speed by dropping needed concepts.

**Confounds.** Warm caches, duplicated easy distractors, index-build cost
omission, hardware effects, or task difficulty changing with corpus size.

**Required controls.** Cold and warm runs, index-build and resident-memory
accounting, relevant-set oracle comparison, adversarial distractors, recall
measurement, and multiple concept-count scales.

## H8 — Self-Application Hypothesis (future; excluded from SEM-0)

**Claim.** A concept autonomously discovered in external problem solving may
later be safely applied to improve the reasoner's own mechanisms and produce
measurable improvement.

**Supporting observation required.** In the SEM-9 sandbox or later, an
autonomously discovered concept produces a proposed mechanism change that
passes causal comparison, regression, rollback, provenance, and independent
approval gates before any promotion.

**Falsifying result.** Self-application yields no reproducible improvement,
depends on benchmark leakage, cannot pass rollback/regression tests, or evades
the mutation boundary.

**Confounds.** Human-authored patches, external provider repair, benchmark
overfitting, unequal tuning, hidden state, or selection among many failed
attempts without correction.

**Required controls.** H8 is not tested in S0 or SEM-0. Future testing requires
a sealed sandbox, immutable baseline, proposal/apply separation, independent
verifier, held-out regressions, causal ablation, rollback proof, and explicit
human authorization.

## Dependency and claim limits

H1–H3 form the first concept-emergence proof target. H4 requires a prior
promotion. H5–H7 are scaling and architecture hypotheses and cannot rescue a
failed semantic-emergence result. H8 is constitutionally unavailable until
the later self-application stage. Passing one hypothesis does not imply that
any other hypothesis passed.
