# Project State

**Canonical version:** 1.3.0
**State frozen for:** SEM-2 completed baseline

This is an operational snapshot. It does not replace or override
`CONSTITUTION.md`.

| Field | Value |
|---|---|
| Current stage | `SEM-2 COMPLETE` |
| Stage S0 status | `PASS` |
| SEM-0 implementation | `COMPLETE` |
| SEM-0 result | `PASS - MINIMAL_AUTONOMOUS_CONCEPT_EMERGENCE` |
| SEM-1 implementation | `COMPLETE` |
| SEM-1 result | `PASS - RECURSIVE_LADDER_AND_SEMANTIC_SEPARATION_VERIFIED` |
| Promoted concepts | `4` (`C000001`, `C000002`, `C000004`, `C000005`) |
| SEM-2 implementation | `COMPLETE` |
| SEM-2 result | `PASS - ADAPTIVE_REASONING_COMPLEXITY_CONTROL_VERIFIED` |
| SEM-3 | `NOT STARTED` |
| Recursive self-mutation | `DISABLED` |
| Recursive-improvement mode | `OBSERVE_MEASURE_ONLY` |
| Proposal generation | `DISABLED` |
| Source patch/apply | `DISABLED` |
| Auto apply/merge/commit/push | `DISABLED` |
| External provider repair | `DISABLED` |
| Recursive benchmark-driven mutation | `DISABLED` |
| LLM reasoning dependency | `DISABLED` |
| Web/network learning | `DISABLED` |
| Current branch | `main` |
| Current commit | `SELF` - resolve with `git rev-parse HEAD` |
| Worktree at committed freeze | `CLEAN` |
| Next allowed stage | `SEM-3_ACTIVE_EXPERIMENT_SELECTION` |

## Canonical document status

The normative documents remain complete and frozen at version 1.0.0. This
operational state record advanced to version 1.3.0 after the sealed SEM-2 run:

- `CONSTITUTION.md`
- `RESEARCH_HYPOTHESIS.md`
- `SEMANTIC_SUBSTRATE_SPEC.md`
- `REASONING_ARCHITECTURE.md`
- `EXPERIMENT_PROTOCOL.md`
- `ROADMAP.md`
- `PROJECT_STATE.md`
- `docs/CANONICAL_READING_ORDER.md`

Their exact byte lengths and SHA-256 values are recorded in
`docs/CANONICAL_MANIFEST.json`. The manifest verifier must pass before a run is
eligible to claim canonical compliance.

## Inherited SYNAPSE status

- Sparse activation, indexing, and routing infrastructure is retained and
  available only as infrastructure; it is not semantic evidence.
- The inherited graph, language-oriented concept, hot-cache, cognition,
  embryo, and product-specific abstractions are retained for audit but are not
  authorized SEM-0 semantic primitives without an explicit SEM-0 design and
  contamination review.
- All inherited recursive-improvement implementation modules are retained as
  frozen source but excluded from the compiled public crate boundary.
- The compiled recursive crate exposes only the immutable S0 quarantine status
  plus the non-recursive core re-export.
- The inherited `patch_sandbox` source contains the only filesystem-write path
  found in the recursive stack; it can write sandbox copies but is disabled by
  the crate-boundary quarantine.
- No process execution, Git commit/push/merge implementation, network client or
  server, external LLM, or external repair-provider dependency was found.

The detailed classification and contamination decisions are in
`reports/stage_s0/inherited_component_inventory.json`.

## SEM-0 evidence snapshot

- Canonical pre-run manifest self-hash:
  `3c116e2e0fc228360c4247a9d4069e2b0be07a4be2448726d2f45b9678f1adc7`.
- Six independently solved primitive derivations produced one opaque candidate
  through typed anti-unification.
- All eight promotion gates passed, including 10/10 counterfactual probes,
  6/6 frozen fresh-blind tasks, primitive-expansion equivalence, regression,
  compression ratio `8.0`, and causal ablation solve-rate delta `1.0`.
- The matched structural-macro control also solved 6/6 with the same expansion
  count as the semantic condition. No D-over-C performance advantage is
  claimed; the distinction is validation, provenance, promotion, and ablation.
- Network, external LLM, local teacher, and recursive source mutation counts
  were zero. Full concept-catalog scans were zero.
- Reports are sealed under `reports/sem0/`.

## SEM-1 evidence snapshot

- `SEM1-RUN-0002` produced four autonomous Generation-2 candidates and
  promoted three after all required gates; maximum autonomous generation was
  `2`.
- `C000002` contains two direct executable uses of immutable predecessor
  `C000001`. Exact concept and primitive ancestry are retained in the lineage
  DAG.
- Frozen fresh-blind solve rates were A `0.8`, B `0.0`, strong structural C
  `0.8`, and semantic D `1.0` across 20 tasks.
- Relative to strong structural C, semantic D improved solve rate by `0.2`,
  reduced search expansions by `37`, reduced false-transfer rate by `0.2`,
  and improved invalid-case abstention rate by `1.0`.
- Generation-2 candidate ablation, Generation-1 ancestor ablation, expanded
  provenance reconstruction, sparse routing, and leakage audit all passed.
- Network, external LLM, local teacher, recursive source mutation, full catalog
  scan, and routing false-negative counts were zero.
- The failed frozen `SEM1-RUN-0001` is preserved under
  `reports/sem1/runs/SEM1-RUN-0001/`. Passing reports are sealed under
  `reports/sem1/`.

## SEM-2 evidence snapshot

- The metric-semantics audit established that SEM-1's `28540` width/live
  values counted cumulative candidate-plan generation rather than
  instantaneous concurrency. SEM-2 now reports solution depth,
  primitive-expanded depth, search-trajectory depth, instantaneous frontier,
  simultaneous live branches, cumulative branches, and cumulative expansions
  separately.
- The failed frozen `SEM2-RUN-0001` is preserved under
  `reports/sem2/runs/SEM2-RUN-0001/`; its recombination interface mismatch was
  repaired only in the versioned `SEM2-RUN-0002` with a fresh blind manifest.
- The passing frozen matrix contained 60 fresh-blind tasks: 12 each for depth,
  width, recombination, composition, and mixed complexity. Equal-resource
  solve rates were B `1.0` and D `1.0`.
- On hard WIDTH/MIXED tasks, median expansions fell from `1848.0` to `10.5`.
  Peak simultaneously live branches fell from `236` to `79`.
- Maximum verified solution-graph depth was `55`, primitive-expanded solution
  depth was `496`, search-trajectory depth was `55`, and concepts composed was
  `4`. Deep-task false prunes were zero.
- Fresh blind evaluation recorded 175 decompositions and 24 verified
  recombinations. Across fresh and adversarial evaluation, 43 information
  probes eliminated 2048 hypotheses; semantic state merges were auditable and
  false merges were zero.
- Network, external LLM, local teacher, recursive source mutation, full
  catalog scan, and routing false-negative counts were zero. Reports are
  sealed under `reports/sem2/`.

## Advancement constraint

Starting SEM-3 requires an explicit subsequent task. SEM-2 completion does not
authorize recursive self-application, which remains reserved for SEM-9.
