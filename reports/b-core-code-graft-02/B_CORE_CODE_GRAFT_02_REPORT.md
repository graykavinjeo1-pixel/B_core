# B_CORE-CODE-GRAFT-02 — External Real-World Coding Transfer Validity

## Verdict

```text
B_CORE_CODE_GRAFT_02_STATUS=FAIL
DISPOSITION=EXTERNAL_VALIDITY_NOT_ESTABLISHED
NEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY
```

The accepted 27-object GRAFT-01 state outsolved the exact ungrafted authority on upstream test contracts (2/3 versus 0/3), with sparse activation and causal package ablations. The campaign nevertheless fails its frozen primary gate because neither successful candidate passed the independent exact semantic-restoration check, the Go task remained unsolved, and there was no task solved by both arms from which to establish the required productivity gain.

## Frozen authorities

- Predecessor: `09fe306e96711b6194eefa5b379ce775a1fe4079`
- Ungrafted authority: `b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4`
- Final evaluation freeze: `978b8750b8aa810dbcfded61e3ea40a52b9e6adc`
- Final repositories: byteorder (Rust), go-cmp (Go), p-limit (JavaScript)
- Exact final task-memory overlap: 0
- Exact final patch-memory overlap: 0
- Post-final knowledge/routing/acceptance changes: 0 / 0 / 0

## Paired FINAL-B results

| Task | Ungrafted | Grafted upstream contract | Independent verification | Patch attempts U/G | Repair work U/G |
|---|---:|---:|---:|---:|---:|
| byteorder return-type defect | FAIL | PASS | FAIL — not exact minimal restoration | 0 / 1 | 2 / 6 |
| go-cmp relational guard defect | FAIL | FAIL | FAIL | 1 / 2 | 5 / 8 |
| p-limit async release defect | FAIL | PASS | FAIL — not exact minimal restoration | 0 / 1 | 2 / 8 |

Aggregate raw acceptance counts are ungrafted 0/3 and grafted 2/3. These two graft-enabled upstream-test passes are preserved as observations, but they are not sufficient for campaign PASS because external independent verification is authoritative.

## Gate results

- Grafted outsolves ungrafted: PASS
- Multiple external defect families solved by upstream contracts: PASS
- Unique graft-enabled solves: PASS (2)
- Package causal ablation: PASS (3/3 degradations)
- Sparse activation: PASS (P50/P95/max = 2/2/2; full scans = 0)
- Independent verification of every grafted success: FAIL
- Productivity gain among tasks solved by both arms: FAIL (no shared solved task)
- B_Core regression/quality gates: PASS

## Integrity and quality

- Task/repository/patch-hash routing events: 0 / 0 / 0
- Gold-patch reads: 0
- Hidden-test reads before submission: 0
- Test weakening and verifier bypass solutions: 0 / 0
- Canonical network reads/writes: 0 / 0
- External LLM and local teacher calls: 0 / 0
- Workspace test: PASS with `cargo test --workspace -j 1` (semantic-reasoning 270 tests)
- Clippy: PASS; new warning signatures = 0
- Offline clean release reconstruction: PASS from the sealed freeze commit without warm cache
- Controlled coding, first-principles, autonomous-loop, world-model, planning, and temporal-abstraction regressions: all 0

## Failure boundary

The supported conclusion is narrower than GRAFT-01's controlled-fixture result: the graft can produce repairs that satisfy real third-party toolchains, but this campaign did not establish externally valid, exact, minimal semantic repair across the frozen real-world set. No post-FINAL repair was made. The recorded next dominant growth limit is `UNSEEDED_HISTORICAL_BUG_REPAIR_EXTERNALITY`; no follow-on campaign is started automatically.
