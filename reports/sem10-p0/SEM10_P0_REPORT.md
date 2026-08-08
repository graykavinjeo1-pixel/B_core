# SEM10-P0 Portable Predecessor Reproducibility Repair

Status: `PASS`

The failed SEM-10 receipt at commit
`4b097f6fda09453d970051a73dd3e64375059eef` remains unchanged. SEM-10
recursion was not started, and no reasoning feature, semantic logic, promoted
concept, or self-improvement candidate was changed.

## Root cause and repair

The source Git object was intact. The host-wide `core.autocrlf=true` setting
converted 257 tracked predecessor evidence files under SEM-2 through SEM9-R1
to CRLF because the prior `.gitattributes` covered only canonical documents,
SEM-0, and SEM-1. Replacing CRLF with LF in memory reproduced every sealed
stage tree hash exactly. Repository-wide LF checkout policy and explicit binary
exclusions now make that invariant portable.

The 14 failed tests all stopped at the inherited SEM-3 gate, whose first
nonmatching path was `reports/sem2`. After the repair a completely new checkout
restored all nine sealed SEM-0 through SEM-8 tree hashes. The full workspace
regression passed 155/155 tests. The 315 tracked predecessor evidence files had
the same aggregate SHA-256 before and after the tests, so predecessor tree
mutations during validation were zero.

## Three identities

- Canonical source: CORE-X0 commit `2961a975fd57e3ad0f5cabe29a2058fb0ca4fcba`,
  tree `97b0a60dfd18c146f78053400105f158c70745af`.
- Semantic runtime state: semantic state `d1abd8de...d5d3c`, sparse index
  `77b17332...e6fc`, ABI 1, semantic state `SEMANTIC-STATE-SEM8-1`, capability
  contract 1. All 12 promoted payload hashes are exact.
- Portable build: rustc 1.96.0, target `x86_64-pc-windows-msvc`, MSVC linker
  14.44, `/Brepro`, fixed epoch, and host-path remapping. Two clean builds both
  produced `dad15e03eec28fed770bbf671e65559a68366b801b5c464f7dda560366273a8d`.

## Binary drift decision

The historical sealed hash remains
`57a81bbe59dd9e524d7aea5f17ba7ecf471cec8e90da922d2f65de73f53b4ae2`;
it was not replaced. Byte-exact cross-host reproduction was unavailable because
the sealed compiler version and flags were never recorded. PE inspection found
original `Administrator` paths, non-reproducible timestamp/PDB metadata, and a
different `.text` section consistent with toolchain code generation drift.

This drift is accepted as non-semantic only because independent evidence all
agrees: immutable source identity, exact state/index/ABI and promoted receipts,
155/155 predecessor tests, the 21-case CORE-X0 parity suite at 1.0, preserved
SEM9-R1 correctness/expansion/frontier behavior, and byte-identical sealed/local
canary JSON with zero full-catalog scans and zero routing false negatives.

## Continuation boundary

`research_continuation_manifest.json` records the portable environment and
`docs/PACKAGING_CONTRACTS.md` separates the deploy package from the research
continuation seed. Clippy remains a separately classified toolchain-lint drift:
rustc/clippy 1.96.0 reports 22 `manual_is_multiple_of` warnings against unchanged
predecessor source. No semantic source was edited to silence them.

SEM10-P0 is sealed. The next allowed stage is
`SEM-10_BOUNDED_RECURSIVE_IMPROVEMENT_LOOP_FRESH_RESTART`; it was not started
automatically.
