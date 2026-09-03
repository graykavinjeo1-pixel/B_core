# GPT Similarity Stop Rule

Status: `FROZEN_STOP_RULE`

This rule turns the phrase "GPT-level conversation" into a finite acceptance decision. The denominator and thresholds must not be expanded after the final reference set is sealed.

## 1. What similarity means

Exact wording is not the target. The same dialogue must lead to a sufficiently similar:

- user intent and response act;
- discourse topic, reference, and ellipsis binding;
- required and prohibited claims;
- epistemic boundary between plan, execution, report, and verified result;
- Korean or English realization quality.

## 2. GPT variation calibration

Each final response has three independently authored GPT reference surfaces. Their pairwise deterministic surface similarities establish the `GPT_SELF_SIMILARITY` baseline for that response.

```text
BCORE_RELATIVE_SURFACE_SIMILARITY =
    min(1,
        max_similarity(B_Core, GPT references)
        / median_pairwise_similarity(GPT references))
```

This avoids requiring B_Core to copy one arbitrary GPT wording while still rejecting repetitive or mechanical realization.

## 3. Frozen PASS thresholds

- sealed final denominator: 40 dialogues, 160 assistant responses;
- semantic/pragmatic composite mean: at least `0.85`;
- intent/context exact agreement: at least `0.95`;
- response-act exact agreement: at least `0.95`;
- mean `BCORE_RELATIVE_SURFACE_SIMILARITY`: at least `0.85`;
- 10th-percentile `BCORE_RELATIVE_SURFACE_SIMILARITY`: at least `0.70`;
- fixed structural/metamorphic transfer: `100/100`;
- unsupported claims, false results, silent ambiguity guesses, semantic-authority violations: `0`;
- B_Core runtime external LLM/teacher/network calls and recursive source mutations: `0`.

The final set is evaluated once. PASS stops the current Language Cortex campaign. FAIL preserves the result and stops; a new benchmark or further campaign requires explicit user authorization.

## 4. Current measured state

Current source-bound development evaluation (`probe20`, 240 responses):

- semantic/pragmatic composite mean: `0.9205` — PASS;
- median composite: `0.9308` — PASS;
- intent/context exact: `1.0000` — PASS;
- response-act exact: `1.0000` — PASS;
- fixed structural/metamorphic transfer: `100/100` — PASS;
- all critical safety counters: `0` — PASS;
- single-reference absolute normalized surface mean: `0.2990` — diagnostic only;
- calibrated three-reference final surface score: `NOT_MEASURED_RUNNER_ABORTED`;
- sealed final result: `FAIL_FINAL_RUN_INVARIANT_PANIC`.

The development-distribution score did not transfer to the independently authored V2 final distribution. It must not be used as evidence of GPT-level completion.

The first and only final B_Core run stopped at `GPTREF-FINAL-C10-EN-01-T1` before emitting a response batch. The typo-bearing request `Um, could ya chek the Knoll service for me?` selected a compositional candidate but no pragmatic or contextual intent, leaving the active `GrammaticalCompositionToPragmaticIntent` link unsatisfied. The frozen campaign therefore ends in FAIL without a similarity score or post-result repair.

After a structural repair, a separately authored V2 campaign was sealed and executed once. It completed all 160 responses without the V1 invariant panic, but failed the similarity gates:

- semantic/pragmatic composite mean: `0.3937`;
- median composite: `0.2466`;
- 10th-percentile composite: `0.0256`;
- intent/context exact: `0.1125`;
- response-act exact: `0.3563`;
- mean GPT-relative surface similarity: `0.4429`;
- 10th-percentile GPT-relative surface similarity: `0.1725`;
- responses at or above `0.80`: `0.1250`;
- silent ambiguity guesses: `16`;
- unsupported claims, semantic-authority violations, false execution/result claims, B_Core external calls, and recursive source mutations: `0`.

The V2 final result is `FAIL_GPT_REFERENCE_SIMILARITY_GATES`. Its primary measured deficit is incorrect intent/context and response-boundary selection, not only surface realization.

## 5. Implemented evaluation boundary

The Rust evaluator now enforces the stop rule directly:

- exactly three reference surfaces per final response;
- three distinct generation run IDs, consistent across the whole final suite;
- identical model, date, system-prompt hash, and generation-configuration hash across runs;
- each run is bound to the frozen final-input hash and all 160 response IDs;
- any run marked as having consulted B_Core output is rejected;
- every surface, run, reference suite, candidate batch, and report is SHA-256 sealed;
- final reports include GPT self-similarity and B_Core relative surface similarity;
- semantic success with mechanical realization still fails the final surface gates.

`gpt-reference-final-sealer` is the only canonical merge path for the three independent runs and the semantic annotation draft. It will not emit a sealed final reference outside the workspace `reports` directory.

## 6. Frozen final input

The final input was authored and sealed before any B_Core evaluation:

- suite: `B_CORE_GPT_REFERENCE_V1_FINAL`;
- final-input SHA-256: `c31162100ef2257a538f409fe4cd41a359b42f1244387b7fc1a3f88914f41960`;
- development-input SHA-256: `ef2a003c6a7b4aeb1ae3143e1e8c4f0401aa3cce894d7eb953474643425f3f3e`;
- 40 dialogues and 160 responses;
- Korean responses: 80; English responses: 80;
- 10 diagnostic categories with 16 responses each;
- duplicate final prompts: 0;
- exact development-prompt reuse: 0;
- mean nearest-development surface similarity: `0.3891`;
- 95th-percentile nearest-development surface similarity: `0.6250`;
- maximum nearest-development surface similarity: `0.8000`, caused by the deliberately short deictic pair `Fix that one.` / `Fix that.`;
- B_Core evaluations before reference sealing: 0;
- external LLM calls by the authoring tool: 0;
- input-audit SHA-256: `8c8d3bb7c4ae93067e4e347fef51de842f8372936a5ab1036b73d9ae162e5067`.

The canonical runner refuses a FINAL input until a matching three-run sealed reference suite is supplied. This prevents candidate output from influencing the reference wording.

## 7. V2 frozen final input and decision

- suite: `B_CORE_GPT_REFERENCE_V2_FINAL`;
- final-input SHA-256: `ff80de6025b7ef07627642367d098da835f920b75ba34915979be609e0189b5d`;
- final-reference SHA-256: `86fc8bbb5cf68343be4110f890b0aa07a6ab5999987f00af3fba54bfea5f81c4`;
- B_Core response-batch SHA-256: `5dc342514acc4d1bb79f23349c88c528b1ccf2b1293fc6ab74983ce329bae10a`;
- evaluation-report SHA-256: `dfff2f45936f20615df509bb0ffd62bb8fef6427750f5b0a4176efac7ec129d9`;
- 40 dialogues and 160 responses;
- Korean responses: 80; English responses: 80;
- exact development- or V1-final-prompt reuse: 0;
- three independent `gpt-5.6-sol` reference runs and 480 reference surfaces;
- B_Core final run attempts: 1;
- post-result V2 repairs or reruns: 0.

## 8. V3 frozen final input and decision

After the source-bound development gate reached a `0.9205` composite mean with exact intent/context and response-act agreement, a new V3 campaign was authored without exact prompt reuse from development, V1, or V2. Three independent `gpt-5.6-sol` runs were sealed before the candidate run.

- suite: `B_CORE_GPT_REFERENCE_V3_FINAL`;
- final-input SHA-256: `77e5e6f8836bf02b972227a02a6204f8d6a220af2c65930ecdb1675ba5b6f5aa`;
- final-reference SHA-256: `dde457451f9ce63d685a331d4b257182b36d7b7524aa9dbff09fa947e316cbcc`;
- B_Core response-batch SHA-256: `f8ff3fdef2499e57215989803e6a2895c9ab34c7f390eed82550abda912af63a`;
- evaluation-report SHA-256: `6c0c41148f11ffd921fb8ae32d916e4daa9e764a217cfd3f46f09b79c1ec3682`;
- 40 dialogues and 160 responses;
- semantic/pragmatic composite mean: `0.4803`;
- intent/context exact: `0.1875`;
- response-act exact: `0.5063`;
- mean GPT-relative surface similarity: `0.4798`;
- 10th-percentile GPT-relative surface similarity: `0.1389`;
- silent ambiguity guesses: `25`;
- unsupported claims, semantic-authority violations, false execution/result claims, B_Core external calls, and recursive source mutations: `0`;
- B_Core final run attempts: 1;
- post-result V3 repairs or reruns: 0;
- decision: `FAIL_GPT_REFERENCE_SIMILARITY_GATES`.

## 9. Post-V3 structural repair boundary

The V3 result is frozen and was not rerun. A semantic-label audit found four pre-authored `CLARIFICATION_REQUEST` labels whose three GPT surfaces consistently perform affect acknowledgement plus an offer rather than asking a clarification question. The score was not changed; the conflicts are recorded in `gpt-reference-v3/SEMANTIC_LABEL_AUDIT.md` so benchmark defects and B_Core defects remain separate.

Repairs were derived at the structural boundary rather than by memorizing V3 sentences. A new suite used different surfaces and different entity names across six defect families:

- event nominals and light verbs;
- Korean embedded actions;
- discontinuous explanation constructions;
- discourse-revision prefaces;
- ordinal target binding;
- typed plan/result queries.

The fresh suite improved from `18/24` (`0.7500`) to `24/24` (`1.0000`). The existing fixed structural/metamorphic suite remained `100/100`, and all safety counters remained zero.

This is evidence that the identified structural defects were repaired. It is not a new GPT-relative completion score. Until a new independently authored and sealed GPT-reference final suite is run, the latest official completion measurement remains V3:

- semantic/pragmatic composite mean: `0.4803`;
- intent/context exact: `0.1875`;
- response-act exact: `0.5063`;
- mean GPT-relative surface similarity: `0.4798`;
- decision: `INCOMPLETE`.

## 10. Productive response-boundary repair

The frozen V3 residual was classified before further repair. Excluding the four recorded annotation conflicts without recomputing the score, 59 responses had a critical response-boundary mismatch. The dominant errors were plan requests collapsing to acknowledgement and verified-result questions collapsing to acknowledgement or plan preview.

A separate 36-case development suite was then authored with no V3 prompt reuse. It covers productive requests, inherited constraints, operation ellipsis, retarget corrections, verified-result queries, and affect/request contrast, with six fresh cases per family.

- baseline: `6/36` (`0.1666`);
- repaired: `36/36` (`1.0000`);
- existing post-V3 structural transfer: `24/24`;
- fixed metamorphic transfer: `100/100`;
- adapter library tests: `567/567`;
- safety, semantic-authority, external-call, and recursive-mutation violations: `0`.

The repair also separates historical result queryability from operation replay: a withdrawn task may remain in auditable history, but an ellipsed command cannot reactivate it.

These results close a measured development bottleneck; they are not a replacement for GPT-relative scoring. The official completion score remains V3 `0.4803` until a new independent three-reference suite is sealed before one candidate run.
