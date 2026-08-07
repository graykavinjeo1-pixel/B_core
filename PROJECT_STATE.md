# Project State

**Canonical version:** 1.9.1
**State frozen for:** SEM-9 failed sandbox run preserved

This is an operational snapshot. It does not replace or override
`CONSTITUTION.md`.

| Field | Value |
|---|---|
| Current stage | `SEM-9 FAILED` |
| Stage S0 status | `PASS` |
| SEM-0 implementation | `COMPLETE` |
| SEM-0 result | `PASS - MINIMAL_AUTONOMOUS_CONCEPT_EMERGENCE` |
| SEM-1 implementation | `COMPLETE` |
| SEM-1 result | `PASS - RECURSIVE_LADDER_AND_SEMANTIC_SEPARATION_VERIFIED` |
| Promoted concepts | `12` (`C000001`, `C000002`, `C000004`, `C000005`, `C000006`, `C000007`, `C000008`, `C000009`, `C000010`, `C000011`, `C000012`, `C000013`) |
| SEM-2 implementation | `COMPLETE` |
| SEM-2 result | `PASS - ADAPTIVE_REASONING_COMPLEXITY_CONTROL_VERIFIED` |
| SEM-3 implementation | `COMPLETE` |
| SEM-3 result | `PASS - AUTONOMOUS_ACTIVE_EXPERIMENT_SELECTION_VERIFIED` |
| SEM-4 implementation | `COMPLETE` |
| SEM-4 result | `PASS - MATHEMATICAL_FIRST_PRINCIPLES_DERIVATION_VERIFIED` |
| SEM-5 implementation | `COMPLETE` |
| SEM-5 result | `PASS - PROGRAMMING_FIRST_PRINCIPLES_EXPANSION_VERIFIED` |
| SEM-6 implementation | `COMPLETE` |
| SEM-6 result | `PASS - DEFINITION_ONLY_KNOWLEDGE_FORAGING_AND_CONSOLIDATION_VERIFIED` |
| SEM-7 implementation | `COMPLETE` |
| SEM-7 result | `PASS - LANGUAGE_CORTEX_ATTACHED_AND_SEMANTIC_BOUNDARY_VERIFIED` |
| SEM-8 implementation | `COMPLETE` |
| SEM-8 result | `PASS - CROSS_DOMAIN_SEMANTIC_MECHANISM_TRANSFER_VERIFIED` |
| SEM-9 implementation | `ATTEMPTED - FAILED` |
| SEM-9 result | `FAIL - SELF_PATCH_BUILD_FAILURE:CANDIDATE_FMT_CHECK_FAILED` |
| SEM-10 | `NOT STARTED` |
| Recursive self-mutation | `DISABLED` |
| Recursive-improvement mode | `OBSERVE_MEASURE_ONLY` |
| Proposal generation | `SANDBOX-ONLY SINGLE GENERATION EXECUTED; NOW STOPPED` |
| Source patch/apply | `SANDBOX CANDIDATE ONLY; PRODUCTION APPLY DISABLED` |
| Auto apply/merge/commit/push | `DISABLED` |
| External provider repair | `DISABLED` |
| Recursive benchmark-driven mutation | `DISABLED` |
| LLM reasoning dependency | `DISABLED` |
| Web/network learning | `BOUNDED READ-ONLY DEFINITION FORAGING VERIFIED IN SEM-6; ZERO NETWORK CALLS IN SEM-7` |
| Current branch | `main` |
| Current commit | `SELF` - resolve with `git rev-parse HEAD` |
| Worktree at committed freeze | `CLEAN` |
| Next allowed stage | `SEM9-R1_RECURSIVE_SELF_APPLICATION_REPAIR` |

## Canonical document status

The normative documents remain complete and frozen at version 1.0.0. This
operational state record advanced to version 1.9.1 after preserving the failed
SEM-9 sandbox run:

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

## SEM-3 evidence snapshot

- Frozen `SEM3-RUN-0001` used 100 evaluator-only fresh-blind tasks and an equal
  50-experiment budget for random, novelty, fixed-curriculum,
  uncertainty-only, and active-semantic selection. Selector blind access and
  post-blind tuning were both false.
- Active-semantic selection resolved all 12 evidence-backed uncertainties,
  eliminated 24 hypotheses, handled 8 semantic surprises through 20
  append-only model revisions, and executed 50 of 14,400 generated candidate
  experiments. Random selection resolved 8 uncertainties.
- Realized information per experiment was `0.3803910001730775` for active
  selection and `0.30039100017307746` for random selection, an efficiency
  ratio of `1.2663195633487891`.
- Frozen blind solve rates were random `0.93`, novelty `0.65`, fixed curriculum
  `0.83`, uncertainty-only `1.0`, and active-semantic `1.0`. Active selection
  reduced false transfer from random's `0.15217391304347827` to `0.0` and
  median expansions from `25` to `22`.
- The capability frontier expanded with maximum solution depth `69`, primitive
  depth `555`, four composed concepts, five subproblems, and one verified
  recombination. No new concept passed the existing promotion gates; maximum
  autonomous concept generation therefore remained `2`.
- All nine primary gates and all four selector ablations passed. Network,
  external LLM, local teacher, recursive source mutation, full catalog scan,
  and routing false-negative counts were zero. Reports are sealed under
  `reports/sem3/`.

## SEM-4 evidence snapshot

- Frozen `SEM4-RUN-0001` used 100 fresh mathematical blind tasks, including 20
  randomized definition-only zero-shot tasks and 40 adversarial tasks. The
  manifests contained no expected answers, target formulas, proof scripts, or
  human formula names; post-blind tuning was false.
- The mathematical substrate contained eight exact executable primitives and
  20 explicit transformation rules. The independent non-searching proof kernel
  checked 99 certificates and 158 transformation steps, including 44 induction
  proofs, while accepting zero invalid transformations.
- Blind solve rates were primitive A `0.6`, structural macro B `0.84`, semantic
  no-promotion C `1.0`, and first-principles D `1.0`. D reduced total search
  expansions from C's `5628` to `2150` at equal solve rate. Definition-only
  zero-shot solve rate was `1.0`, and invalid-transfer rate was zero.
- Four target-formula-free recurrence relations were autonomously synthesized
  and formally proved. `C000006` and `C000007` passed formal proof, executable
  applicability, fresh reuse, causal ablation, compression, and full-lineage
  gates and were promoted.
- Best primitive-expanded proof cost was `72` steps versus `2` compressed
  operational steps, a compression ratio of `36.0`. Both promoted concepts
  passed causal ablation through lower search cost and reasoning depth at equal
  solve rate.
- Maximum solution-graph depth was `92`, primitive-expanded depth was `905`,
  and concepts composed was `5`. Target-formula solver leaks, network calls,
  external LLM calls, local teacher calls, CAS/SMT calls, recursive source
  mutations, full catalog scans, and routing false negatives were zero. Reports
  are sealed under `reports/sem4/`.

## SEM-5 evidence snapshot

- Frozen `SEM5-RUN-0002` used 130 fresh blind programming tasks: 20 scalar,
  30 sequence/stateful, 20 nested-sequence, 20 file/image, 20 randomized
  definition-only opaque-API, and 20 multi-stage adversarial tasks. Expected
  outputs, evaluator family metadata, and reference source were absent from
  solver-visible manifests; property cases were generated after synthesis.
- ProgramIR was the typed, effect-checked semantic authority. Rust-Min was only
  a deterministic adapter. All 130 canonical programs passed ProgramIR
  validation, compiled offline with local `rustc` and no external crates, and
  executed under timeout, output, path, process, dependency, and temporary
  workspace containment. Containment violations and invalid effects accepted
  were both zero.
- Equal-budget solve rates were primitive A `0.4230769230769231`, structural B
  `0.6923076923076923`, semantic no-promotion C `0.7692307692307693`, and full
  first-principles D `1.0`. D reduced mean bounded search cost versus C by
  `36.76923076923077`; hidden property and definition-only zero-shot rates
  were both `1.0`.
- Three programming concepts were proposed from recurring ProgramIR dependency
  structures. Generation-3 `C000008` and `C000009` depend nontrivially on
  immutable Generation-2 ancestors; Generation-4 `C000010` recombines both.
  All three passed consistency, calibration, fresh reuse, cross-domain
  transfer, language-separation, compression, lineage, and real equal-budget
  ablation gates.
- Best primitive-expanded program cost was `43` nodes versus `6` compressed
  operational nodes, a compression ratio of `7.166666666666667`. The run
  recorded three cross-domain transfers, 90 predecessor-concept reuses, a
  maximum of five composed concepts, four simultaneous subproblems, and one
  verified recombination.
- Target-program solver leaks, Rust-token-dependent promoted concepts, network
  calls, external LLM calls, local teacher calls, recursive source mutations,
  full catalog scans, and routing false negatives were zero. Reports are
  sealed under `reports/sem5/`.

## SEM-6 evidence snapshot

- The pre-network checkpoint verified the canonical manifest, all SEM-0 through
  SEM-5 report trees, nine immutable promoted concepts, prior blind manifests,
  and the recursive-improvement quarantine before any external retrieval.
- Frozen SEM-6A contained 100 blind tasks in the required 30 programming/API,
  20 mathematical/formal, 20 protocol/specification, 20 ambiguous/conflict,
  and 10 contamination categories. Full definition foraging solved all 100;
  the keyword and semantic-gap-only controls each solved 70.
- The separately frozen 50-task SEM-6B live set used ten read-only requests
  against seven predeclared official or institutional sources. The full system
  solved 45 tasks. Five floor-definition tasks were left unresolved because
  the frozen DLMF section returned logarithm definitions; no replacement answer
  was fabricated. The resulting live zero-shot rate was `0.9`.
- Aggregate equal-budget solve rates were A `0.0`, B `0.7666666666666667`, C
  `0.7666666666666667`, and D `0.9666666666666667`. The semantic compiler
  accepted 145 fact uses, rejected 35 inapplicable or quarantined candidates,
  and passed 400 hidden-case assertions in a locally synthesized Rust batch.
- All ten planted solution spans and ten implementation spans were quarantined.
  Twenty instruction-like spans were detected as data and none were executed.
  False semantic imports, contamination events, external solution dependencies,
  network writes, remote execution, recursive source mutations, full-catalog
  scans, and routing false negatives were zero.
- Three Generation-5 candidates were evaluated. `C000011` and `C000012` passed
  all promotion requirements and were consolidated with source, scope, version,
  and lineage provenance; the unresolved DLMF candidate was not promoted. One
  cross-domain transfer passed. Reports are sealed under `reports/sem6/`.

## SEM-7 evidence snapshot

- The bounded Language Cortex used deterministic Korean/English lexical
  grounding and parsing to produce GoalIR; raw language never entered the
  semantic reasoning hot path.
- The first frozen `SEM7-RUN-0001` failed on four Korean Language-to-GoalIR
  conversions and is preserved under
  `reports/sem7/failed_runs/SEM7-RUN-0001/`. The repair was limited to the
  irregular sum form `합해` and the negation scope in `N보다 큰 값은 제외해`.
- Fresh frozen `SEM7-RUN-0002` passed all 100 tasks: 20 Korean grounding, 20
  English grounding, 10 synonym/paraphrase, 10 ambiguity/reference, 10 opaque
  relexicalization, 10 definition-only foraging, 20 language-to-program, and
  20 language-to-math paths, with the overlapping domain counts balanced at
  ten tasks per language.
- Language-to-GoalIR accuracy and direct-GoalIR reasoning equivalence were both
  `1.0`. Korean and English realization faithfulness were both `1.0`, and
  unsupported explanation facts were zero.
- Six promoted concepts share Korean and English aliases without semantic
  duplication. Alias add, rename, second-language attachment, removal, unnamed
  operation, opaque relexicalization, and semantic ablation all passed while
  semantic payload hashes remained invariant.
- External LLM calls, local teacher calls, network calls, recursive source
  mutations, lexical-token-dependent promoted concepts, full-catalog scans,
  and routing false negatives were zero. All 13 gates passed. Reports are
  sealed under `reports/sem7/`.

## SEM-8 evidence snapshot

- Frozen `SEM8-RUN-0001` used 120 fresh blind transfer tasks: 20 each for
  math-to-program/state, program-to-math/state, cross-data-domain, opaque
  state-machine, structural-mimic/broken-assumption, and semantically
  equivalent but structurally different targets. Forty targets were zero-shot.
- Eight domain-light `MechanismIR` views were extracted from sealed SEM-4/5/6
  evidence and split into four development and four blind source mechanisms.
  Solver-visible manifests contained no source-target pairs, analogy labels,
  evaluator families, hidden cases, or target solutions.
- Equal-budget solve rates were target-only A `0.25`, structural B
  `0.24166666666666667`, semantic-role C `0.8333333333333334`, and full D
  `1.0`. Median expansions were `120.0` for A and `37.0` for D.
- D produced 100 valid transfers, 99 causally useful transfers, 76 direct and
  24 adapted transfers, and composed at most two source mechanisms. All eight
  available source mechanisms were selected autonomously.
- Zero-shot transfer, role mapping, relation preservation, broken-assumption
  detection, and structurally different semantic-equivalence transfer rates
  were all `1.0`. All 20 invalid analogies were rejected; invalid accepted
  transfers and full-D structural-mimic false transfers were zero.
- Domain-light Generation-6 `C000013` was promoted from mathematics,
  programming, external-definition, and synthetic state-machine evidence with
  parents `C000006`, `C000008`, and `C000011`. No predecessor semantic payload
  changed.
- Lexical similarity authority uses, external transfer-solution dependencies,
  network calls, external LLM or teacher calls, recursive source mutations,
  full-catalog scans, and routing false negatives were zero. All 12 gates
  passed. Reports are sealed under `reports/sem8/`.

## SEM-9 failed-run evidence snapshot

- Frozen `SEM9-RUN-0001` contained 140 fresh blind tasks, twenty for each of
  seven predecessor capability families, plus 20 adversarial state-identity
  tasks. The candidate generator could not read hidden states, expected
  outputs, or evaluator classifications.
- Three evidence-backed self weaknesses were identified. Sparse role routing
  autonomously selected external-definition concept `C000012` through source
  mechanism `M0006` for `SELF-CANDIDATE-ROUTER`; two other mappings were
  rejected before patch generation because required assumptions were unknown
  or violated.
- The single sandbox candidate compiled, passed Clippy, passed all three
  sandbox contract tests, preserved strict solve rate `1.0`, reduced median
  expansions from `120.0` to `68.0`, and reduced peak frontier from `67` to
  `37`. Self-application and source-concept causal ablations passed with zero
  regressed tasks.
- The candidate failed `cargo fmt --check`. Therefore build/test integrity
  Gates 4 and 5 failed, the candidate class is `PATCH_INVALID`, and no verified
  self-application candidate exists despite the measured behavioral gain.
- Production source mutations, accepted protected-core mutation attempts,
  benchmark-specific patch branches, catalog scans, routing false negatives,
  external LLM/teacher calls, network writes, remote executions, auto merges,
  and auto pushes were zero. No candidate was applied to the canonical runtime.
- The failed evidence and sandbox binary hashes are sealed under
  `reports/sem9/`. Repair was not started after blind evaluation.

## Advancement constraint

SEM-9 did not pass. `SEM9-RUN-0001` must remain preserved and no SEM-10 work is
authorized. Starting `SEM9-R1_RECURSIVE_SELF_APPLICATION_REPAIR` requires an
explicit subsequent task.
