# Metrics and Structural Repair Strategy

## Why the prior loop did not demonstrate completion

The historical regression total measures retention of previously encoded behaviors. It does not measure productive transfer to new clause structures. A growing regression count can therefore coexist with weak generalization. R64 exposed that distinction:

- initial surface result: 4/20;
- initial strict semantic result: 2/20;
- current result after one structural repair batch: 6/20 surface, 4/20 strict;
- current verified final-capability score: 4/32 = 12.5%, with 12 transfer cases still sealed.

The current 20% diagnostic score is not presented as two days of effective target progress. It is evidence that the earlier local-capability loop used the wrong primary metric.

## Fixed dashboard

Five values are reported after every repair batch:

1. `FINAL_CAPABILITY_SCORE`: strict accepted cases / 32. Unopened cases are unverified, never assumed to pass.
2. `DIAGNOSTIC_EXACT_SCORE`: strict typed-semantic matches / 20.
3. `METAMORPHIC_INVARIANCE`: semantics-preserving variants passed / 100.
4. `REGRESSION_RETENTION`: unchanged historical language tests passed / historical total.
5. `READINESS_SCORE`: a fixed weighted gate score:

| Component | Weight |
| --- | ---: |
| Strict diagnostic exactness | 30 |
| Metamorphic invariance | 25 |
| Historical regression retention | 15 |
| Rust format/clippy/check/test quality | 10 |
| Semantic safety invariants | 10 |
| Canonical integrity and package parity | 10 |
| **Total** | **100** |

Unexecuted or stale gates score zero. Safety is all-or-nothing. Integrity/package parity is all-or-nothing. The sealed transfer may open only at readiness 100/100.

Current readiness is 21.25/100:

- diagnostic: 20% of 30 = 6;
- metamorphic: 21% of 25 = 5.25;
- historical regression: stale after current edits = 0;
- full Rust quality: not yet rerun after current edits = 0;
- semantic safety: 10/10;
- integrity plus package parity: current root/package parity not yet restored = 0.

## Leading indicators

These indicators prevent a high score produced by sentence memorization:

- `STRUCTURAL_REPAIR_YIELD`: new strict diagnostic passes / product repair batch;
- `CROSS_CASE_YIELD`: distinct frozen cases improved by one shared code change;
- `PARAPHRASE_YIELD`: unseen lexical variants improved without adding their whole sentence;
- `ENTITY_RENAME_INVARIANCE`: target replacement leaves intent/scope structure unchanged;
- `CLAUSE_ORDER_INVARIANCE`: semantics-preserving reordering leaves live versus conditional goals unchanged;
- `FALSE_POSITIVE_RATE`: response-text pass but wrong typed target; required value 0;
- `SENTENCE_TEMPLATE_COUNT`: whole-sentence solution dispatches; required value 0.

Current repair yield is +2 strict cases from one structural batch. This is acceptable evidence for the indirect-request predicate-family repair, but it does not validate the remaining architecture.

The frozen 100-probe metamorphic baseline is 21/100:

| Dimension | Result |
| --- | ---: |
| Entity rename | 4/20 |
| Lexical paraphrase | 7/20 |
| Clause order | 2/20 |
| Irrelevant distractor | 4/20 |
| Korean/English mirror | 4/20 |

| Axis | Result |
| --- | ---: |
| Grammar/scope | 11/30 |
| Discourse/topic | 1/20 |
| Reference/ellipsis | 5/20 |
| Pragmatic intent | 0/10 |
| Plan/result | 4/10 |
| Grounded realization | 0/10 |

## Structural cause map

The remaining failures are not treated as fourteen unrelated sentences.

| Cause | Frozen symptoms | Required shared repair |
| --- | --- | --- |
| A. Conditional/concessive action scope is overwritten during goal projection | D03-D06 | preserve conditionality per predicate and project only live main-clause goals |
| B. Intra-turn discourse entities do not feed later references and causal targets | D07-D10, D13-D14 | bind same-turn entity mentions before discourse-memory fallback |
| C. Operation ellipsis and ordinal topic restoration use incomplete English/Korean constructions | D11-D12, D17-D18 | typed operation inheritance plus ordered topic selection |
| D. Claim rejection can outrank the requested verified-result lookup | D16 | lifecycle-aware reference selection with result-query priority |
| E. Coordinated role sets are lost before realization | D19-D20 | preserve CoTheme ordering and realize from typed nodes without result claims |

D01-D02 and D15 are currently closed. D13-D14 remain failures under strict target identity even though the broad evaluator text check passes.

## Repair-batch admission rule

A product repair batch is admitted only when all of the following are true:

1. it targets one structural cause in the table, not one literal sentence;
2. a direct typed-IR unit test detects the cause without relying on response substring checks;
3. five metamorphic variants per affected diagnostic case are defined before the repair;
4. the change improves at least two frozen or metamorphic cases through shared logic;
5. no historical regression, safety violation, public API change, or package-boundary violation occurs.

If a proposed change only makes one exact sentence pass, it is rejected before merge into the repair stream.

## Fixed execution order

1. build the 100-case metamorphic readiness harness and record its pre-repair baseline;
2. repair cause A and rerun its variants plus the full diagnostic;
3. repair causes B and C, then rerun all prior variants to measure retention;
4. repair causes D and E;
5. run all historical suites, full Rust quality, safety, integrity, and package parity;
6. open the 12-case sealed transfer once at readiness 100.

There is no R65 fallback. A final transfer failure is a plan failure and stops the run.
