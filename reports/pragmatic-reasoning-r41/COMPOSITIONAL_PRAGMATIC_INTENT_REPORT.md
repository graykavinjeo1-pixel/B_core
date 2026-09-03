# R41 — Compositional Pragmatic Intent Graph

Status: **PASS**

R41 adds a hash-sealed, non-authoritative clause-level pragmatic graph inside
the existing `B_CORE_PRAGMATIC_INTENT_GRAPH_IR_1` contract. It does not replace
the existing parser or grant language execution authority.

## Outcome

- Diagnostic blind suite: **28/28** (baseline **0/28**)
- Held-out transfer suite: **20/20** (first exposure **7/20**)
- Fresh R41 tasks: **48/48**
- Metadata-discovered canary binaries: **73/73**
- Cumulative row-based R1–R41 cases: **1,723/1,723**
- Direct-response special cases: **7/7**
- Aggregate adapter cases: **1,730/1,730**

## Implemented boundary

Each action-bearing clause can now carry a typed force, projection, source
frame, subject, confidence, and non-authority evidence. Relations represent
support, condition, contrast, override, correction, prohibition, alternative,
sequence, and coordination. A SHA-256 seal detects graph tampering.

The graph resolves these composition boundaries:

- reported or quoted actions versus the user's actual request;
- prohibited actions versus an authorized alternative;
- rhetorical/evaluative clauses followed by a real request;
- causal and conditional scope around a requested action;
- capability questions that mention an action without requesting it;
- unresolved `or/또는` alternatives, which fail closed;
- multi-turn correction of a prior active goal;
- `rather than/instead of` exclusion direction;
- conditional directives that remain deferred until verified.

R41 intervention is deliberately narrow. The graph is always inspectable, but
it overrides the older decision path only when a typed composition boundary is
present. Ordinary single commands, result references, action-state questions,
QUD answers, and existing deixis paths remain on their sealed predecessor
logic.

## Oracle safety correction

The first frozen condition fixtures incorrectly demanded an immediately
authorized active goal. That expectation conflicts with the existing safety
boundary: an unverified antecedent must produce a deferred commitment. The
fixture was corrected to require a matching deferred action whose execution
authority becomes available only after verification. No product path was
weakened to satisfy the invalid immediate-authority oracle.

- Initial diagnostic hash: `eff1b915cd3d58e0993a4133861908007e139317abf6e274df460413b47cb465`
- Final diagnostic hash: `8687a8f64e89ca6c60e0af0f0b491f7b53b17bd0ac967a6f7cb7ba649ee3e783`
- Initial transfer hash: `e5f4e9387da9eadf4861479548f2aaa7ca46ecd77e9f7c5e8dc54121e6e47acd`
- Final transfer hash: `2f5d7cefd5985e0cf8c4121ce7195613834bf919aecc9d2bef6b5beae3d0a7d4`

## Verification

- Adapter library tests: **346/346**
- Workspace library tests: **869/869**
- `cargo test --workspace`: **PASS**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::manual_is_multiple_of`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- New unit invariants: **7**
- Build cache cleanup: **20,020 files / 22.3 GiB removed**; `target` absent

The optional Python compatibility probes logged missing `pytest` during the
workspace run, as expected, but all Rust tests passed. No Python path was used
by the Language Cortex.

## Safety and repository state

- External LLM calls: **0**
- Local teacher calls: **0**
- Network calls: **0**
- Recursive source mutations: **0**
- Unsupported realization claims: **0**
- Graph semantic authority: **false**
- Graph external execution authority: **false**
- Branch: `main`
- HEAD: `cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f`
- Commit created: **no**
- Push performed: **no**
- Worktree clean: **no**, because R13–R41 changes remain intentionally uncommitted

The pre-existing user change in
`crates/semantic-reasoning/src/growth_supervisor.rs` remains byte-for-byte
unchanged by R41.

## Remaining major stages

Assuming one-pass success, **two major stages remain**:

1. **R42** — distinguish and hash provenance across request, plan,
   observation, execution report, and verified result.
2. **R43** — integrate all six language axes, run adversarial long-context
   regression, and seal the package boundary.

R43 is the final integration stage. Additional repair rounds may still be
required if new blind tests expose a boundary failure.
