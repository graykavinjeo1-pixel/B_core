# B_Core portable product-core package

Source commit: `26faea38d67e901b0230d2681ee22106113c3cdf`

This directory is the portable, product-facing subset of `B_Core`.

## Included

- `crates/dockable-semantic-core`: language-independent GoalIR/ResultIR runtime,
  semantic state, sparse index, runtime provenance, deliberation, experience,
  planning, executable mechanism memory, swarm coordination, configuration,
  and ABI.
- `crates/semantic-core-adapters`: bounded language, cognitive API, lexical and
  document knowledge, professional document, mechanism induction, long-term
  repair, and generic capability adapters kept outside the semantic payload.
- `bin/core-x0-canary.exe`: prebuilt Windows x86-64 runtime canary.
- `docs/DOCKABLE_CORE_INTEGRATION.md`: integration and boundary guidance.

## Deliberately excluded

Research/evaluation crates, SEM campaign reports, blind suites, historical
evidence, sandboxes, `.git`, build caches, debug binaries, growth-supervisor
campaign tooling, and recursive source-mutation machinery are not product-core
runtime dependencies and are not included.

## Validate

From this directory:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p dockable-semantic-core --bin core-x0-canary
cargo run -p semantic-core-adapters --bin language-adapter-canary
cargo run -p semantic-core-adapters --bin generic-capability-canary
cargo run -p semantic-core-adapters --bin cognitive-api-canary
```

The prebuilt canary is only a boundary/runtime check. A consuming product
should depend on `crates/dockable-semantic-core` as a Rust library and connect
through its own adapter or `semantic-core-adapters`; it should not treat the
canary as a product service.
