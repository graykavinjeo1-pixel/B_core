# SEM-4 Mathematical First-Principles Derivation Report

Status: `PASS`

Disposition: `MATHEMATICAL_FIRST_PRINCIPLES_DERIVATION_VERIFIED`

## Protocol

The reasoner received exact mathematical primitives, formal definitions, and a transformation-rule catalog, but no target formulas, target proof scripts, named solution templates, CAS results, or teacher answers. The independent kernel checked proposed transformations without performing solution search.

The 100-task blind manifest, its 20 definition-only subset, and adversarial subset were frozen before evaluation. Equation candidates were checked by substitution; recurrence candidates were checked by symbolic base and successor-difference obligations.

## Equal-task comparison

| Condition | Blind solve rate | Search expansions |
|---|---:|---:|
| Primitive A | 0.600000 | 22296 |
| Structural macro B | 0.840000 | 7822 |
| Semantic no-promotion C | 1.000000 | 5628 |
| First-principles D | 1.000000 | 2150 |

Definition-only zero-shot solve rate: `1.000000`.

## Derived mathematical substrate

- Autonomous candidates / promoted concepts: `4` / `2`
- Formally proved new relations: `4`
- Best opaque concept: `C000007`
- Primitive-expanded / compressed steps: `72` / `2`
- Compression ratio: `36.000000`
- Verified induction proofs: `44`
- Target-formula solver leaks: `0`
- Invalid transformations accepted: `0`

All nine primary gates passed. Network, external LLM, local teacher, CAS, SMT, recursive source mutation, full catalog scan, and routing false-negative counts were zero.

## Stage boundary

SEM-5 was not started. The next allowed stage is `SEM-5_PROGRAMMING_FIRST_PRINCIPLES_EXPANSION`.
