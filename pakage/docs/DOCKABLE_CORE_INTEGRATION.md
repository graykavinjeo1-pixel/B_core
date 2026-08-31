# Dockable Semantic Core Integration

## Boundary

`dockable-semantic-core` is the single product-agnostic semantic runtime. It
owns the language-independent GoalIR/ResultIR interface, semantic substrate,
adaptive reasoner, authoritative deployable semantic-state receipts, sparse
index, and capability contract ABI.

`semantic-core-adapters` owns input/output and product-facing translation.
`semantic-reasoning` owns research runners, blind suites, evaluation logic,
historical evidence, and stage reports. None of those packages is a dependency
of the core-only build.

The long-term architecture is one core with many adapters. Product forks such
as `core_robot`, `core_coding`, `core_character`, or `core_video` are forbidden.

```text
Product
  -> Adapter
  -> CapabilityContract / GoalIR
  -> Dockable Core
  -> ResultIR / CapabilityResult
  -> Adapter
  -> Product
```

## Versions

- `CORE_ABI_VERSION=1`
- `SEMANTIC_STATE_VERSION=SEMANTIC-STATE-SEM8-1`
- `CAPABILITY_CONTRACT_VERSION=1`

An adapter must declare the core ABI it accepts. An external capability must
expose its input/output types, preconditions, postconditions, effects, state
mutation, resource limits, failure modes, permissions, and semantic relations.
The core rejects ABI, contract, capability-ID, and type mismatches before
execution.

## Core-only build and operation

```powershell
cargo build -p dockable-semantic-core --release --bin core-x0-canary
./target/release/core-x0-canary.exe
```

This dependency graph contains only `dockable-semantic-core` and ordinary Rust
serialization/hash libraries. It does not contain the language adapter,
research crate, reports, blind data, sandbox sources, network clients, or
product implementations.

The canary loads the embedded semantic state and sparse index, accepts direct
GoalIR, runs the extracted reasoner, returns ResultIR, and exits. Korean and
English lexical data are not loaded.

## Adapter examples

### AI character or chat application

The language adapter compiles bounded text into GoalIR. Raw text and lexical
aliases remain outside the core; the core receives only semantic fields.

### Coding tool

Expose compiler, filesystem, or test execution through separate capability
contracts with explicit effects and permissions. Do not add Rust/compiler or
filesystem behavior to the core.

### Robot

Sensor and actuator adapters should expose typed observations and bounded
action capabilities. Device permissions and failure modes belong in each
contract; semantic reasoning remains in the same core.

### Image/video tool

Buffer, decoder, encoder, camera, and rendering operations are adapter
capabilities. General concepts must not be copied into image/video packages.

### Offline embedded device

Ship the release core binary/library, semantic state, sparse index, compact
runtime provenance receipt, configuration, and ABI manifest. Network and
research history are not required.

## Portable package

The portable subset is described by `PACKAGE_MANIFEST.json` at the package
root. It includes the independently buildable core and adapter source, the
release core canary, and the files under
`crates/dockable-semantic-core/{state,config,abi}`. It excludes `.git`, `target`
caches, reports, blind data, historical runs, sandboxes, research/evaluation
crates, recursive self-improvement sources, and debug binaries.
