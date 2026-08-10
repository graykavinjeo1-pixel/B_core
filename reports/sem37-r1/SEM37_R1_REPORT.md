# SEM-37-R1 final report

`SEM37_R1_STATUS=FAIL`

`DISPOSITION=EXTERNAL_CAUSAL_STRUCTURE_PRECISION_LIMIT`

SEM-37-R1 preserved the historical SEM-37 failure and started from the accepted SEM-36 capability commit `b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4`. The failed SEM-37 adaptive state was not promoted.

## What worked

The system separated a reusable intervention-response family from target-specific numeric realization. On both development partitions it learned to promote bounded rebinding only in the independently repeated two-variable context and to abstain elsewhere.

On fresh R1-FINAL-C Lane B:

| Arm | SSE | Adaptation work |
|---|---:|---:|
| Shift-aware transfer | 8.619500392175587 | 71 |
| No-change | 8.63237192020683 | 0 |
| Naive transfer | 10.196399402769668 | 27 |
| Scratch | 18.734659829368738 | 364 |

One target mechanism was partially transferred and rebound. Thirteen unsupported attempts were rejected before promotion. `NEGATIVE_TRANSFER_ACCEPTED=0` and `PROMOTED_TARGET_TRANSFER_WORSE_THAN_NO_CHANGE_EVENTS=0`. Transfer used 293 fewer work units than scratch under equal target evidence.

This supports a scoped `MODULAR_CAUSAL_TRANSFER_OBSERVED=true` result for Lane B. It does not establish universal invariance.

## Why the campaign failed

Lane A's historical overpermissive-structure failure was not repaired. A generic residual/direction grid of 143 candidate policies found no non-trivial policy that improved exact F1 over naive transfer on both DEV-A and DEV-B. The engine therefore refused to claim a false invariant and retained the unresolved complete candidate set.

Fresh final Lane A was:

```text
TP=38
FP=104
FN=0
DIRECTION_ERRORS=8
LAG_ERRORS=0
```

Shift-aware and naive transfer were identical on these structural fields, while scratch had `TP=17, FP=19, FN=21`. Because the required fresh structural-precision repair was not demonstrated, Level F failed and the overall campaign is FAIL.

## Integrity

- Primary and secondary acceptance paths agree: diff 0.
- Deterministic recomputation diff: 0.
- Internal SEM-36 capability regressions: 0.
- Full workspace tests and all-target Clippy passed with no new warning signatures.
- Offline clean reconstruction from commit `860d9976cd19bece99f40349f701260421a48047` passed without warm-cache authority.
- Network reads/writes during canonical execution: 0/0.
- Ground-truth graph/equation reads and expected-result lookups: 0.
- SEM-38, perception grounding, and QIS-0 were not started.

The next allowed action is operator review only.
