# SEM-10 Predecessor Gate Report

`SEM10_STATUS=FAIL`

`DISPOSITION=PREDECESSOR_INTEGRITY_FAILURE:ARTIFACT_TREE_DRIFT_AND_FRESH_RECONSTRUCTION_DRIFT`

SEM-10 recursive execution did not start. The independent commit lineage,
canonical manifest, preserved SEM-9 failure, SEM9-R1 pass, CORE-X0 reports, and
portable package were found and verified in the original worktree. A fresh
worktree from CORE-X0 commit `2961a975fd57e3ad0f5cabe29a2058fb0ca4fcba`
did not reproduce the sealed predecessor bytes.

## Blocking evidence

- The canonical manifest verifier passed all 8 canonical files.
- `cargo fmt --check` passed.
- current Clippy 1.96 failed with 22 toolchain-drift lint errors.
- `cargo test --workspace --locked --offline` executed 140 tests before the
  mandatory stop: 126 passed and 14 predecessor-integrity tests failed with
  `ARTIFACT_TREE_DRIFT`.
- Five authoritative core JSON/config/ABI files changed byte length and SHA-256
  after fresh checkout while Git still reported a clean worktree.
- A fresh core-only release build and runtime canary succeeded semantically,
  but the binary SHA-256 and size did not match the sealed CORE-X0 binary.

The evidence supports checkout line-ending drift under global
`core.autocrlf=true` plus incomplete `.gitattributes`, together with an
unrecorded exact Rust toolchain identity. No repair was applied because the
SEM-10 protocol requires stopping on predecessor-integrity failure.

## Safety and contamination

- Recursive attempts executed: 0
- Candidate patches generated: 0
- Production source mutations: 0
- Protected-core mutations accepted: 0
- External/local teacher calls: 0
- Network reads/writes: 0/0
- Later SYNAPSE graft imports: 0

The next permissible action is a separately authorized predecessor
reproducibility repair and fresh regate. SEM-11 was not started.
