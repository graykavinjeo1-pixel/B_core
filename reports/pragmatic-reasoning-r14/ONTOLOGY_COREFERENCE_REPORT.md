# Pragmatic Reasoning R14 — Ontology-Mediated Coreference

Status: **PASS** for the bounded R14 increment.

R14 adds a small executable ontology to the pure-Rust language cortex. Surface words ground to adapter-local entity or action concepts, and cross-turn reference selection operates on those concept paths rather than sentence identity. The resolver can therefore bind pairs such as `service → application`, `report → document`, `repository → codebase`, `repair → fix`, and `deploy → rollout` across Korean and English.

The ontology is not semantic authority. It reads dialogue-local, hashed referent memory; every mention remains `semantic_authority=false`; and a binding is emitted only when exactly one bounded candidate satisfies the concept and optional role path. The binding records its `ONTOLOGY_PATH` evidence. No promoted concept payload is modified.

## Evidence

| Validation | Result |
|---|---:|
| Frozen RUN1 baseline | 5 / 44 |
| RUN1 after initial pipeline | 30 / 44 |
| RUN1 after root repairs | 44 / 44 |
| Fresh transfer RUN2, first attempt | 25 / 26 |
| Fresh transfer RUN2 after root repair | 26 / 26 |
| Sealed R14 suite | 70 / 70 |
| Frozen R1–R13 canaries | 456 / 456 |
| Frozen R1–R14 canaries | 526 / 526 |
| Canary binaries | 19 / 19 |
| Workspace tests | 760 / 760 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Canonical manifest | PASS, 10 files |

RUN1 exposed three structural failures: generic filler nouns competed with individuated mentions, Korean predicate forms were misread as event nominals, and an entity resolver consumed the role inside a longer event phrase. The repairs select a uniquely individuated compatible candidate, require Korean event-nominal case marking, and give the complete event span ownership over its embedded role.

Fresh RUN2 then exposed one separate gap: `저장 계층` was not retained as a multiword entity span. The supplemental mention path is deliberately limited to multiword ontology forms; ordinary single-token mentions remain owned by the existing semantic-role graph. A later full regression exposed English `that` complement clauses such as `Alice says that the deployment ...`; those are now rejected as deictic references. All affected epistemic and dialogue-QA tests recovered.

Korean reference replacement also re-realizes object, subject, and topic particles from the resolved referent, avoiding outputs such as `서비스을` and `작업를`.

The final workspace run also exposed a validation-infrastructure defect outside the language path. A content-addressed intrinsic-curiosity seed legitimately emitted one `failure_*.json` diagnostic tombstone before two successful receipts, while the test counted all three directory entries as receipts. The assertion now counts successful receipts separately from failure diagnostics. The isolated test passed 5/5 after repair, followed by a clean 760/760 workspace run.

## Boundary

This is not GPT-level language understanding. The ontology is a compact controlled software/document vocabulary with one-parent entity paths and six event action families. Open-world entity linking, arbitrary bridging reference, broad commonsense, and causal/concessive discourse inference remain incomplete.

No external LLM, local teacher, network lookup, promoted semantic mutation, recursive source mutation, commit, or push was used. SEM-10 was not started. The regenerated Cargo build cache was removed after final validation.
