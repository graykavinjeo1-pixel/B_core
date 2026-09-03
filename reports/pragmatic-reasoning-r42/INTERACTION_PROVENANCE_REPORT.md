# R42 — Interaction Provenance Graph

Status: **PASS**

R42 connects request, semantic goal, plan, language report, trusted execution
observation, verifier receipt, verified result, and realized claim through one
typed, hash-sealed provenance graph. Language remains a non-authoritative
interface: neither a report nor fluent output can create execution evidence or
advance an action state.

## Outcome

- Diagnostic blind suite: **28/28** (baseline **0/28**)
- Held-out transfer suite: **20/20** on first exposure
  (preimplementation baseline **0/20**)
- Fresh R42 tasks: **48/48**
- Metadata-discovered workspace canary binaries: **79/79**
  (adapter canaries: **75/75**)
- Cumulative row-based R1–R42 cases: **1,771/1,771**
- Direct-response special cases: **7/7**
- Aggregate adapter cases: **1,778/1,778**

Frozen suite hashes stayed unchanged after product implementation:

- Diagnostic: `94c991b288545e5f3f34583efa8bd55345b3702063afb17869852def0c723755`
- Transfer: `1c099e570c776e76221b112d6cf9aed442608531e30b82b3416d252e5686fc38`

No transfer oracle or fixture was changed after first exposure.

## Implemented boundary

`B_CORE_CONVERSATION_TURN_RESPONSE_10` now returns a
`B_CORE_INTERACTION_PROVENANCE_GRAPH_IR_1`. Its typed nodes cover language
input, semantic goal, planned action, language report, execution observation,
verification receipt, verified result, and realized claim. Typed edges cover
grounding, projection, report revision, execution start, verification, result
establishment, and claim support.

Every node, edge, and complete graph has an independent SHA-256 seal. The graph
validator rejects invalid typed edge pairs, ungrounded claims, a verified claim
grounded only by language, an observation without a plan, and a result without
a terminal verification chain. The graph is bounded to 1,024 nodes and 2,048
edges.

The action ledger now retains bounded, hash-sealed language-report revision
history and full trusted-evidence audit records. Reports remain descriptive:
`semantic_authority=false` and `external_action_executed=false`. Only typed,
validated evidence can establish an observed execution transition or result.

## Regression defects exposed and repaired

The integrated graph caught two predecessor defects during full regression:

1. Unsupported-presupposition answers could be labeled as derived from
   dialogue records despite having no evidence reference or source turn. They
   are now explicit `EvidenceAbsent` / `Unknown` claims grounded by the current
   query turn.
2. Topic-anchored claims could carry an action identity while their fallback
   source node did not. Action-bearing, non-report, non-verified claims now bind
   to the matching typed plan node.

These were product regressions found by the cumulative suite. They did not
modify the frozen R42 transfer oracle.

## Verification

- Adapter library tests: **353/353**
- Workspace library tests: **876/876**
- `cargo test --workspace`: **PASS**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::manual_is_multiple_of`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- New unit invariants: **7**
- Build cache cleanup: **28,764 files / 30.6 GiB removed**; `target` absent

## Safety and repository state

- External LLM calls: **0**
- Local teacher calls: **0**
- Network calls: **0**
- Python calls in the Language Cortex path: **0**
- Recursive source mutations: **0**
- Unsupported realization claims: **0**
- Graph semantic authority: **false**
- Language execution authority: **false**
- Branch: `main`
- HEAD: `cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f`
- Commit created: **no**
- Push performed: **no**
- Worktree clean: **no**, because the cumulative R13–R42 changes remain
  intentionally uncommitted

The pre-existing user change in
`crates/semantic-reasoning/src/growth_supervisor.rs` remains exactly unchanged.

## Limitation and next stage

The SHA chain is internal tamper evidence, not an external executor signature
or trust root. Tool-specific payload truth, multi-executor reconciliation, and
partial-progress/cancel/rollback semantics are not claimed as solved here.

Assuming one-pass success, **one major stage remains**:

1. **R43** — integrate all six language axes, run the final adversarial
   long-context and package-boundary regression, and seal the distributable
   Language Cortex boundary.

R43 is the final integration stage. Repair rounds may still be required if its
blind suite exposes a cross-axis failure.
