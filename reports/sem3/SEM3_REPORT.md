# SEM-3 Active Experiment Selection Report

Status: `PASS`

Disposition: `AUTONOMOUS_ACTIVE_EXPERIMENT_SELECTION_VERIFIED`

## Protocol

All predecessor stages and preserved failed runs verified before execution. A private closed-world environment accepted experiment queries and returned observations without exposing hidden rules. The independently frozen 100-task blind evaluator was unavailable to every selector during curriculum construction.

`LOCAL_ACTIVE_INFERENCE` remained separately measured from `EPISTEMIC_EXPERIMENT_SELECTION`; SEM-3 reports the latter.

## Equal-budget comparison

| Condition | Experiments | Blind solve rate | Uncertainties resolved |
|---|---:|---:|---:|
| Random A | 50 | 0.930000 | 8 |
| Novelty B | 50 | 0.650000 | 0 |
| Fixed C | 50 | 0.830000 | 6 |
| Uncertainty D | 50 | 1.000000 | 12 |
| Active E | 50 | 1.000000 | 12 |

Active-vs-random information-efficiency ratio: `1.266320`.

## Epistemic outcomes

- Autonomous experiments generated / executed: `14400` / `50`
- Hypotheses eliminated: `24`
- Semantic surprise events / model revisions: `8` / `20`
- New promoted concepts: `0`
- Capability frontier expanded: `true`
- Maximum solution / primitive-expanded depth: `69` / `555`

All nine primary gates passed. Network, web, external LLM, local teacher, recursive source mutation, full catalog scan, and routing false-negative counts were zero.

## Stage boundary

SEM-4 was not started. The next allowed stage is `SEM-4_MATHEMATICAL_FIRST_PRINCIPLES_DERIVATION`.
