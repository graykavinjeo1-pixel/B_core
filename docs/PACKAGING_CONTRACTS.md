# B_Core Packaging Contracts

These contracts keep product deployment separate from research continuation.

## Deploy package

The deploy package contains only the dockable runtime product surface:

- `bin/core-x0-canary.exe`
- `state/semantic_state.json`
- `state/sparse_index.json`
- `state/runtime_provenance.json`
- `config/core-config.json`
- `abi/core-abi.json`

It excludes research reports, evaluators, historical runs, source-control history,
build caches, language adapters, and recursive campaign tooling. Its historical
CORE-X0 binary receipt remains
`57a81bbe59dd9e524d7aea5f17ba7ecf471cec8e90da922d2f65de73f53b4ae2`.

## Research continuation seed

The research seed is a Git bundle of the independent B_Core lineage at the
sealed SEM10-P0 repair commit. It contains the full tracked source, test and
evaluator harness, retained research evidence, failure receipts, portability
metadata, and history needed to reconstruct the campaign on a clean host.

It excludes all `target/` directories, compiler caches, temporary worktrees,
diagnostic outputs, and host-local Git configuration. A reconstructed checkout
must honor `.gitattributes`, install the toolchain recorded in
`rust-toolchain.toml`, and follow `research_continuation_manifest.json`.

The two packages are not interchangeable. A deploy package is sufficient to
dock and run the compact core; only a research continuation seed is sufficient
to resume the SEM-series research lineage.
