# Language Cortex Completion Contract

Status: FROZEN AFTER THE FIRST R64 STRUCTURAL REPAIR BATCH
Implementation substrate: Rust only
Baseline date: 2026-09-02

## 1. Terminal objective

Complete the bounded Korean/English Language Cortex so that it converts utterances into typed semantic goals, preserves discourse state and truth boundaries, and realizes only supported claims. This contract replaces open-ended `R+1` stage creation. R64 is the final development cycle for this objective.

The six terminal axes are fixed:

1. grammatical composition and scope;
2. discourse and topic state;
3. deixis, reference, and ellipsis resolution;
4. speech-act intent and pragmatic inference;
5. plan, execution, report, and verified-result separation;
6. evidence-grounded Korean/English realization.

## 2. Fixed acceptance denominator

The acceptance denominator is 32 cases and must not grow after this freeze:

| Axis | Diagnostic | Sealed transfer | Total |
| --- | ---: | ---: | ---: |
| Grammatical composition and scope | 6 | 2 | 8 |
| Discourse and topic state | 4 | 2 | 6 |
| Deixis/reference/ellipsis | 4 | 2 | 6 |
| Speech-act and pragmatic intent | 2 | 2 | 4 |
| Plan/result truth boundary | 2 | 2 | 4 |
| Evidence-grounded realization | 2 | 2 | 4 |
| **Total** | **20** | **12** | **32** |

Diagnostic cases are `R64_D01..R64_D20`. The sealed transfer suite is `R64_T01..R64_T12`. The transfer executable may not be run until every readiness gate in section 5 passes.

Case-to-axis assignment is fixed:

- grammar/scope: D01-D06;
- discourse/topic: D13, D14, D17, D18;
- reference/ellipsis: D09-D12;
- pragmatic intent: D07-D08;
- plan/result: D15-D16;
- realization: D19-D20.

Passing requires the typed semantic target, not merely a response string containing the expected entity. D13 and D14 therefore count as failures at baseline even though the original evaluator's broad text check reported them as passes.

## 3. Baseline and progress calculation

The first diagnostic exposure produced 4/20 surface passes. Strict semantic review removed two false positives, giving a true initial baseline of 2/20. The first structural repair batch added productive indirect-inspection forms and raised the current result to 6/20 surface passes and 4/20 strict passes.

Current strict axis scores:

| Axis | Strict baseline |
| --- | ---: |
| Grammar/scope | 2/6 = 33.3% |
| Discourse/topic | 0/4 = 0% |
| Reference/ellipsis | 1/4 = 25% |
| Pragmatic intent | 0/2 = 0% |
| Plan/result | 1/2 = 50% |
| Realization | 0/2 = 0% |
| **Diagnostic total** | **4/20 = 20%** |

Two progress numbers must always be reported:

- `DIAGNOSTIC_COMPLETION = strict diagnostic passes / 20`;
- `SEALED_ACCEPTANCE_COMPLETION = strict passes across all opened acceptance cases / 32`.

Before the transfer suite is legally opened, sealed-acceptance completion is reported as `4/32 = 12.5% verified; 12 sealed`, not guessed. Initial baseline and current score must remain separate so that repair yield is measurable.

## 4. Fixed repair backlog

No new stage is created. All work must close one of these bounded structural causes:

1. predicate families and productive argument extraction;
2. conditional, concessive, negation, and coordination scope graphs;
3. same-turn and cross-turn topic/reference binding;
4. operation ellipsis and ordinal topic restoration;
5. problem-disclosure to causal-goal binding;
6. report/claim/result lifecycle selection;
7. structured-plan realization with entity preservation and unsupported-claim exclusion.

A failing sentence may add a unit regression, but it may not add a new completion axis, denominator, or follow-on R stage.

## 5. Readiness gates before the sealed transfer opens

Every gate is binary and requires 100%:

1. strict diagnostic: 20/20 with direct typed-IR assertions;
2. metamorphic readiness: 100/100 generated probes, five per diagnostic case:
   entity renaming, lexical paraphrase, semantics-preserving clause-order variation,
   irrelevant distractor insertion, and Korean/English structural mirroring;
3. all historical Language Cortex canaries and transfer canaries pass unchanged;
4. full Rust workspace tests, formatting, clippy, and check pass under documented allowances;
5. semantic safety: zero external LLM/teacher/network calls, zero recursive source mutation,
   zero unsupported explanation facts, and no execution authority inferred from reports or conditions;
6. public API seal, canonical hashes, protected user line, and portable package source parity pass.

If any readiness gate is below 100%, the transfer suite remains sealed and the dashboard identifies the exact remaining fixed backlog item.

## 6. Final acceptance policy

The sealed 12-case transfer suite is executed once after all readiness gates pass.

- PASS: 12/12 transfer, 32/32 combined, every safety gate at 100%; then integrate, package, and seal the final report.
- FAIL: declare `LANGUAGE_CORTEX_COMPLETION=FAIL` and `DISPOSITION=PLAN_FAILURE`; preserve the evidence and stop. Do not create R65 or silently expand the plan.

This process cannot logically guarantee that an unseen evaluation will pass. It does guarantee that final acceptance is not attempted on an underprepared build and that a rejection is treated as a failed plan rather than hidden by an endless extension.

## 7. Prohibited shortcuts

- Python in the canonical language path;
- external or local LLM reasoning;
- whole-sentence answer templates;
- raw text as semantic authority;
- weakening, editing, or selectively skipping a frozen evaluator;
- opening the sealed transfer early;
- changing the denominator after seeing failures.
