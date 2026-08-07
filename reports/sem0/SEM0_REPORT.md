# SEM-0 Report

Status: `PASS`
Disposition: `MINIMAL_AUTONOMOUS_CONCEPT_EMERGENCE`
Canonical pre-run self-hash: `3c116e2e0fc228360c4247a9d4069e2b0be07a4be2448726d2f45b9678f1adc7`

## Result

One opaque candidate (`C000001`) was mined by typed anti-unification of 6 independently solved primitive derivations. No lexical alias was available to the engine. It passed all eight gates and was promoted only after frozen blind evaluation and causal ablation.

The structural-macro control had the same typed execution power and matched D on blind solve rate and expansions. Therefore this experiment does **not** claim a performance advantage of D over C. D differs through executable predictions, counterfactual validation, immutable provenance, promotion gates, and causal ablation.

## Frozen controls

| Condition | Solved / attempted | Strict rate | Expansions | Max depth | Macro uses | Concept uses |
|---|---:|---:|---:|---:|---:|---:|
| A | 0 / 6 | 0.000 | 48 | 0 | 0 | 0 |
| B | 0 / 6 | 0.000 | 48 | 0 | 0 | 0 |
| C | 6 / 6 | 1.000 | 24 | 1 | 6 | 0 |
| D | 6 / 6 | 1.000 | 24 | 1 | 0 | 6 |

## Semantic evidence

- Counterfactual pass rate: `1.000` (10 / 10)
- Compression ratio: `8.000`
- Ablation solve-rate delta: `1.000`
- Ablation expansion delta (disabled minus enabled): `24`
- Blind task manifest SHA-256: `b5b29580d21f79fdbd4b860d9761ad3ce8c3a14386cac9c72836278006b1a8f5`
- Candidate semantics SHA-256 before blind: `53bd5409cb782180fa5091d2c8ad212339394bfef80d6a43236989e3f5e268ba`
- Full catalog scans: `0`

## Contamination controls

Network, external LLM, local teacher, solution retrieval, expected-query lookup during solving, and recursive source mutation counts were all zero. The inherited recursive stack remained observe/measure-only. Blind expected outputs and hidden generator metadata were absent from the reasoner-visible manifest.

## Scope

This is a single-generation, closed-world SEM-0 result. It does not establish general intelligence, does not validate later hypotheses, and does not start SEM-1. Human lexical interpretation is intentionally absent from the sealed canonical metrics and may be attached only afterward as forensic metadata.

## Post-hoc forensic interpretation

After the canonical metrics and gates were sealed, `C000001` was interpreted as functionally resembling element-wise mapping with a parameterized checked scalar operator. This alias was never available to the generator, reasoner, miner, candidate, router, verifier, or promotion gate and is not promotion evidence.
