# SEM-1 Recursive Concept Ladder Report

## Disposition

- Status: `FAIL`
- Disposition: `RECURSIVE_CONCEPT_LADDER_NOT_VERIFIED`
- Predecessor integrity: `true`
- Canonical integrity: `true`
- Blind set frozen before evaluation: `true`
- Post-blind tuning: `false`

## Recursive Ladder

`C000001` remained immutable and was executed as an actual ancestor during discovery. The miner produced 2 Generation-2 candidates and promoted 1.

- Maximum autonomous concept generation: `2`
- Best Generation-2 concept: `C000003`
- Post-hoc interpretation: Functionally resembles a checked parameterized transformation followed by stateful aggregation.
- Generation-2 ablation pass: `true`
- Generation-1 ancestor ablation pass: `false`
- Expanded derivations preserved: `true`

## Semantic Separation

Baseline C is a typed, parameterized structural graph-macro system with structural matching, macro composition, and macro-on-macro reuse. It was not intentionally weakened. Condition D adds explicit semantic preconditions, safe abstention, relation-based equivalence, and counterfactual applicability checks.

| Metric | Structural C | Semantic D | D minus C |
|---|---:|---:|---:|
| Strict solve rate | 0.650000 | 0.850000 | 0.200000 |
| Search expansions | 310 | 105764 | 105454 |
| False-transfer rate | 0.200000 | 0.000000 | -0.200000 |
| Invalid abstention rate | 0.000000 | 1.000000 | 1.000000 |

Semantic separation pass: `true`.

## Generalization And Counterfactuals

- Frozen fresh-blind tasks: `20`
- Counterfactual probes: `18` (17 passed)
- Valid counterfactual prediction accuracy: `1.000000`
- Invalid-case rejection accuracy: `0.928571`
- Adversarial transfer tests: `8`

## Adaptive Complexity

- Maximum successful reasoning depth: `132`
- Maximum primitive-expanded depth: `32`
- Maximum reasoning width: `27272`
- Maximum live branches: `27272`
- Maximum concepts composed: `2`
- Maximum graph nodes / edges: `32` / `31`
- Peak active concepts: `3`
- Best multi-generation compression ratio: `12.000000`

## Sparse Activation And Quarantine

- Full catalog scans: `0`
- Routing false negatives: `0`
- Network / external LLM / local teacher calls: `0 / 0 / 0`
- Recursive source mutations: `0`
- `SELF_OBSERVE=true`
- `SELF_MEASURE=true`
- `SELF_PROPOSE=false`
- `SELF_APPLY=false`
- `SOURCE_MUTATION=false`

## Lineage

The exact lineage DAG is serialized in `concept_lineage.json`; it contains 14 nodes and 23 edges. Primitive expansion is reconstructable.

## Next Stage

SEM-2 was not started. The next allowed stage is `SEM-2_ADAPTIVE_REASONING_COMPLEXITY`.
