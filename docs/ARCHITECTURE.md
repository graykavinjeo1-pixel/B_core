# Architecture

Synapse Self Learning Core is organized around verified reasoning structures,
not answer memorization.

```text
Application
-> Swarm Layer
-> Core
-> Nested Kernel
-> Hierarchical Activation Index
-> Concept Nodes
-> Primitive Operators
-> Proof Engine
-> Verifier
-> Learning Engine
-> Verified Proof Library
```

## Runtime Layers

- `Core`: owns the bounded self-learning loop and policy transitions.
- `Swarm Layer`: runs bounded workers and sidecars.
- `Nested Kernel`: routes activation into a small relevant kernel set.
- `Proof Engine`: emits semantic witnesses, rule traces, independent proofs,
  and composite proof graphs.
- `Verifier`: rejects unverified outputs, leakage, and unsafe promotions.
- `Learning Engine`: promotes only verified proof structure.

## Knowledge Intelligence

The versioned M8 product path is:

```text
M6 lossless public evidence
-> M6R structural meaning / MechanismFrameV1
-> M8 source evidence and claim structure
-> source, contradiction, scope, and temporal audit
-> immutable epistemic knowledge generation
-> exact relation/mechanism index
-> bounded active knowledge
-> M7 context and M5.7R reasoning consumers
```

M8 never treats a document, repeated statement, teacher proposal, embedding
match, or latest timestamp as truth authority. Claims retain source identity,
validity, conditions, uncertainty, contradiction, supersession, retraction,
and history. New generations are content addressed; activation is an
append-only atomic manifest so rollback does not overwrite prior evidence.

Runtime queries read one typed index row and at most 32 referenced records.
They do not decode the full generation or scan the record store. Mechanism
retrieval uses canonical role and causal topology, while world truth and
planning remain outside M8.

## Repository Layout

The current executable layout is:

```text
src/       Python package MVP
crates/    Rust workspace crates
tools/     Sidecars and guarded learning runners
tests/     Test suite
docs/      Design documents
examples/  Answer-free examples and schemas
```

Large local runtime artifacts are ignored and should not be committed.
