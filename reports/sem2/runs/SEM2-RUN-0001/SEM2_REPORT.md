# SEM-2 Adaptive Reasoning Complexity Report

Status: `FAIL`

Disposition: `ADAPTIVE_CONTROL_NOT_DEMONSTRATED`

## Integrity and protocol

Canonical and predecessor integrity passed. The failed `SEM1-RUN-0001` and sealed successful `SEM1-RUN-0002` were verified before implementation. Four promoted concepts remained immutable. The blind matrix was frozen before evaluation and no post-blind tuning occurred. Network, external LLM, local teacher, and recursive source mutation counts were all zero.

## Metric semantics audit

SEM-1's `MAX_REASONING_WIDTH=28540` and corresponding live-branch field represented cumulative candidate-plan generation, not instantaneous concurrency. SEM-2 reports instantaneous frontier width, simultaneous live branches, cumulative branches, and cumulative expansions separately. SEM-1 depth 56 counted dynamic execution work, whereas primitive-expanded depth 17 counted static derivation nodes.

## Equal-resource result

| Metric | Baseline B | Adaptive D |
|---|---:|---:|
| Strict solve rate | 1.000000 | 0.600000 |
| Median hard WIDTH/MIXED expansions | 223.000 | 9.500 |
| Peak simultaneous live branches | 36 | 1 |

Expansion reduction: `0.957399`. Live-branch reduction: `0.972222`.

## Adaptive reasoning evidence

- Maximum solution graph depth: `55`
- Maximum primitive-expanded depth: `496`
- Maximum search trajectory depth: `55`
- Maximum concepts composed: `4`
- Maximum simultaneous subproblems: `4`
- Information probes executed: `103`
- Hypotheses eliminated: `444`
- Semantic prunes / false prunes: `1968` / `0`
- Semantic state merges / false merges: `644` / `0`

Deep reasoning, dynamic allocation, decomposition, recombination, semantic pruning, frontier control, sparse routing, and all eight primary gates passed.

## Stage boundary

SEM-3 was not started. The next allowed stage is `SEM-3_ACTIVE_EXPERIMENT_SELECTION`.
