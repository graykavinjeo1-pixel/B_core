# Synapse_Core bounded adoption report

Status: `PASS_BOUNDED_INDEPENDENT_REIMPLEMENTATION`

The supplied URL, `graykavinjeo1-pixel/Synapse_Core`, returned 404. The closest
identity match was the public `graykavinjeo/Synapse_Core` repository at commit
`c85290ce6f2142959cd8a8c241a53df7c24d119e` (2026-08-03).

Its license is proprietary and explicitly grants no copy or modification
license. No source file was copied. The useful architectural mechanism was
implemented independently in Rust against B_Core's existing repository causal
graph.

## Adopted

- Content-addressed validation proof receipts.
- Reverse-dependency impact selection after a bounded source change.
- Reuse only for proofs whose complete dependency snapshot is unchanged.
- Automatic full-workspace validation for file-set, causal-topology,
  manifest/configuration, file-role, duplicate-authority, unindexed-surface, or
  affected-budget changes.
- Self-hashed plans with deterministic replay validation.

## Not imported

- Python campaign orchestration and long-running experiment machinery.
- Raw knowledge packages, benchmark answers, fixtures, or evaluation results.
- Recursive source mutation, external LLM, and teacher paths.
- Source-grounded repair and project-cognition layers already represented in
  B_Core; duplicating them would create competing authorities.

## Controlled acceptance

The R3 canary indexed 1,200 files and traced depth 160 without a catalog
rescan. A one-file leaf change selected the exact 161-file reverse dependency
closure, invalidated one affected proof, and reused one unrelated proof. A
causal-topology change escalated to full-workspace validation. Both emitted
plans passed deterministic replay validation.

This is a controlled capability result, not an official SWE-bench or DeepSWE
score.
