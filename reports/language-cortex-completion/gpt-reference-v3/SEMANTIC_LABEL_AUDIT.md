# V3 Semantic Label Audit

Status: `FROZEN_RESULT_PRESERVED`

The V3 result remains unchanged. This audit records a defect in four pre-authored semantic labels; it does not relabel, rescore, discard, or rerun the final benchmark.

The following response IDs are labelled `CLARIFICATION_REQUEST / ASK_CLARIFICATION` with `ambiguity_requires_clarification=true`:

- `GPTREF-V3-FINAL-C02-EN-01-T1`
- `GPTREF-V3-FINAL-C02-KO-01-T1`
- `GPTREF-V3-FINAL-C02-EN-02-T1`
- `GPTREF-V3-FINAL-C02-KO-02-T1`

Across all three independently authored GPT surfaces for these cases, the actual behavior is affect acknowledgement plus an offer or proposed starting point. The surfaces do not ask the user a clarification question. The semantic labels therefore conflict with the GPT reference behavior.

Consequences:

- V3 remains the latest official frozen score: `4803` basis points.
- The four conflicts are retained as benchmark-label defects, not silently converted into B_Core defects or successes.
- No post-result tuning is evaluated against V3 prompts.
- Future final suites must mechanically audit that response-act labels agree with all calibrated reference surfaces before sealing.
