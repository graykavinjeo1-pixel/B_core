# SEM-2 Adaptive Reasoning Complexity Report

Status: `PASS`

Disposition: `ADAPTIVE_REASONING_COMPLEXITY_CONTROL_VERIFIED`

## Integrity and protocol

Canonical and predecessor integrity passed. The failed `SEM1-RUN-0001` and sealed successful `SEM1-RUN-0002` were verified before implementation. Four promoted concepts remained immutable. The blind matrix was frozen before evaluation and no post-blind tuning occurred. Network, external LLM, local teacher, and recursive source mutation counts were all zero.

## Metric semantics audit

SEM-1's `MAX_REASONING_WIDTH=28540` and corresponding live-branch field represented cumulative candidate-plan generation, not instantaneous concurrency. SEM-2 reports instantaneous frontier width, simultaneous live branches, cumulative branches, and cumulative expansions separately. SEM-1 depth 56 counted dynamic execution work, whereas primitive-expanded depth 17 counted static derivation nodes.

## Equal-resource result

| Metric | Baseline B | Adaptive D |
|---|---:|---:|
| Strict solve rate | 1.000000 | 1.000000 |
| Median hard WIDTH/MIXED expansions | 1848.000 | 10.500 |
| Peak simultaneous live branches | 236 | 79 |

Expansion reduction: `0.994318`. Live-branch reduction: `0.665254`.

## Adaptive reasoning evidence

- Maximum solution graph depth: `55`
- Maximum primitive-expanded depth: `496`
- Maximum search trajectory depth: `55`
- Maximum concepts composed: `4`
- Maximum simultaneous subproblems: `4`
- Information probes executed: `0`
- Hypotheses eliminated: `0`
- Semantic prunes / false prunes: `14167` / `0`
- Semantic state merges / false merges: `4018` / `0`

Deep reasoning, dynamic allocation, decomposition, recombination, semantic pruning, frontier control, sparse routing, and all eight primary gates passed.

## Stage boundary

SEM-3 was not started. The next allowed stage is `SEM-3_ACTIVE_EXPERIMENT_SELECTION`.
