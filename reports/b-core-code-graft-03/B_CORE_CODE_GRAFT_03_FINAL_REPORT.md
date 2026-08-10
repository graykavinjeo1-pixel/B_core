# B_CORE-CODE-GRAFT-03 Final Report

## Canonical verdict

`B_CORE_CODE_GRAFT_03_STATUS=FAIL`

`DISPOSITION=REPOSITORY_CONTRACT_INFERENCE_LIMIT`

The frozen candidate empirically restored all three independent historical real-world defects, while GRAFT01 restored one and the ungrafted arm restored none. Nevertheless, the frozen final acceptance aggregate counted five visible-pass/hidden-fail comparator rows as false accepts even though none submitted a repair. Because post-FINAL acceptance changes are forbidden, the aggregate was not repaired and the canonical campaign remains FAIL.

## Frozen lineage and partitions

- Authoritative predecessor: `09fe306e96711b6194eefa5b379ce775a1fe4079`
- Historical GRAFT02 comparator: `88eb6aea188af384be5098c7b92d5cc1cd6f1d8d` (`FAIL`, not promoted)
- GRAFT03 pre-FINAL freeze: `ebf093c7dcb41845fb77555398c7b36fa7210780`
- DEV_C tasks: 3
- FINAL_D tasks: 3
- DEV_C / FINAL_D overlap: 0
- GRAFT02 FINAL / FINAL_D overlap: 0
- FINAL repositories: thiserror, gjson, p-queue
- FINAL languages: Rust, Go, TypeScript

## FINAL_D evidence

| Historical defect | Ungrafted | GRAFT01 | GRAFT03 | GRAFT03 semantic delta |
|---|---:|---:|---:|---:|
| thiserror parser progress / termination | FAIL | FAIL | PASS | 2 changed lines |
| gjson empty quoted-query boundary | FAIL | PASS | PASS | 2 changed lines |
| p-queue per-call timeout invariant | FAIL | FAIL | PASS | 5 changed lines |

Measured totals:

- `UNGRAFTED_TASKS_SOLVED=0`
- `GRAFT01_TASKS_SOLVED=1`
- `GRAFT03_TASKS_SOLVED=3`
- `GRAFT03_UNIQUE_REAL_WORLD_SOLVES=2`
- `REPOSITORY_REPAIR_PRODUCTIVITY_GAIN=true`
- `REAL_REPAIR_NOVEL_RECOMBINATION_SOLVED=3`
- `CONTRACT_RECONSTRUCTION_SUCCESS_LANGUAGES=3`
- `GRAFT_PACKAGE_EXTERNAL_ABLATION_PASS=true`
- `REPAIR_CONTRACT_ABLATION_PASS=true`
- `UNRELATED_SEMANTIC_CHANGE_EVENTS=0`
- `GRAFT03_VISIBLE_TEST_ONLY_FALSE_ACCEPTS=0`
- `CANONICAL_VISIBLE_TEST_ONLY_FALSE_ACCEPTS=5`

All five canonical false-accept rows came from non-submitting comparator arms. The frozen acceptance scope still treats them as authoritative failures; this report does not reinterpret or change that rule after exposure.

## Leakage, routing, and sparsity

- Gold patch reads by repair arms: 0
- Fix commit reads by repair arms: 0
- Repair-revealing issue reads: 0
- Hidden-test reads before submission: 0
- Task-ID routing events: 0
- Patch-hash routing events: 0
- Repository-ID routing events: 0
- Full repository scan events: 0
- Full coding-knowledge scans: 0
- Maximum active coding objects: 3
- Post-FINAL engine / knowledge / routing / verifier / acceptance changes: `0 / 0 / 0 / 0 / 0`

The TypeScript fixture initially failed before public manifest creation because the frozen evaluator expected a missing runtime `ROOT` binding. The incomplete tree was removed, `ROOT=VAULT` was supplied at process startup, and the exact frozen evaluator file hash remained `2859e8b026c257ada4f91f032246276973f0cf23d4e03e8a5c3897d5b22dc7bf` before and after. No FINAL content or outcome informed the binding.

## Quality and reconstruction

- `cargo test --workspace -j 1`: PASS (430,985 ms)
- `cargo clippy --workspace --all-targets -j 1`: PASS
- New Clippy warning signatures: 0
- Exact-freeze offline clean release reconstruction: PASS (573,722 ms)
- Controlled coding regressions: 0
- First-principles reasoning regressions: 0
- Autonomous scientific-loop regressions: 0
- World-model regressions: 0
- Planning regressions: 0
- Temporal-abstraction regressions: 0

Levels A through H all passed individually. The canonical overall failure is solely the frozen `no_visible_test_only_false_accepts` aggregate.

## Disposition

GRAFT03 is not promoted. GRAFT01 remains the authoritative predecessor, warm infrastructure remains non-authoritative, and the next allowed action is `OPERATOR_REVIEW_ONLY`. No subsequent campaign is started automatically.
