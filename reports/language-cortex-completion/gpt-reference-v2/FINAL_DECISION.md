# GPT-reference V2 final decision

Status: `FAIL`

Disposition: `GPT_REFERENCE_SIMILARITY_GATES_FAILED`

The V2 input suite was frozen before B_Core execution and contained 160 previously unseen turns across 40 Korean and English dialogues. Three independent `gpt-5.6-sol` runs supplied 480 reference surfaces. B_Core was then executed exactly once. It emitted all 160 responses without the V1 six-axis invariant panic, and the source tree hash remained unchanged during the run.

| Gate | Required | Actual |
| --- | ---: | ---: |
| Mean composite similarity | 85.00% | 39.37% |
| Median composite similarity | 88.00% | 24.66% |
| 10th-percentile composite similarity | 75.00% | 2.56% |
| Responses scoring at least 80% | 90.00% | 12.50% |
| Intent and context exact match | 95.00% | 11.25% |
| Response-act exact match | 95.00% | 35.63% |
| Mean GPT-relative surface similarity | 85.00% | 44.29% |
| 10th-percentile GPT-relative surface similarity | 70.00% | 17.25% |
| Silent ambiguity guesses | 0 | 16 |

The strongest category was affect/social backchannel at 52.92%; the weakest was explicit request handling at 22.32%. The evaluator recorded 87 critical response-boundary mismatches and seven failures to clarify ambiguity. This shows that the dominant deficit is not merely prose style: B_Core frequently selects the wrong speech act, epistemic boundary, or conversational action before realization.

The safety and architectural boundaries held. Unsupported reference propositions, semantic-authority violations, external execution authorizations, false execution/result claims, B_Core external LLM calls, local teacher calls, network calls, and recursive source mutations were all zero.

This frozen V2 suite will not be repaired against and rerun as a final test. Any subsequent development may use this failure report as evidence, but the next final decision requires a newly authored and independently sealed suite.
