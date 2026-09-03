# R43 — Six-Axis Integration and Package Gate

Status: **PASS**

R43 completes the planned six-axis engineering program. Each conversation
response now carries one tamper-evident contract that binds grammatical
composition, discourse/topic state, deixis and ellipsis, pragmatic intent,
plan/result separation, and evidence-grounded realization to the live IR that
produced the response.

## Outcome

- Final diagnostic suite: **28/28**
- Final held-out transfer suite: **20/20**
- Fresh R43 tasks: **48/48**
- Metadata-discovered workspace canary binaries: **81/81**
  (adapter canaries: **77/77**)
- Cumulative row-based R1-R43 cases: **1,819/1,819**
- Direct-response special cases: **7/7**
- Aggregate adapter cases: **1,826/1,826**

Final sealed suite hashes:

- Diagnostic: `34F8881CD0B4678DC05C2D1402DA565DC4A6F353DA9CED51D72C3BFBCD1B570C`
- Transfer: `195F67D5EDAB04A794DF7F911E72D831692E343EE3D1CEFED9A7DFC959FBB512`

The blind accounting is intentionally conservative. The first product run was
**24/28**, because four indirect console/receipt strings were rejected rather
than retained as language reports. That was the safer product behavior, so the
oracle was tightened to require untrusted-evidence rejection. No product repair
followed that correction.

The transfer suite scored **12/20** on first exposure. Eight expectations were
then corrected: four cross-language references now accept either lexical alias
of the same semantic target; two scoped-composition checks use authoritative
PlanIR rather than a non-authoritative surface graph; and two quoted-command
explanations require the outer `EXPLAIN` plan while denying authority to the
quoted inner command. Product repairs after transfer exposure: **0**.

## Integrated contract

`B_CORE_CONVERSATION_TURN_RESPONSE_11` includes a
`B_CORE_SIX_AXIS_INTEGRATION_IR_1`. Every axis records its component schema,
live component hash, typed evidence references, and the invariant values
`semantic_authority=false` and `external_action_executed=false`.

The contract enforces eight cross-axis invariants:

1. turn and state alignment;
2. ambiguity fails closed;
3. GoalIR/PlanIR provenance is complete;
4. a language report cannot verify a result;
5. realization claims have typed sources;
6. output equals the grounded realization;
7. language has no semantic authority;
8. package dependencies point from adapter to core.

Validation recomputes all six component hashes from the live response. A
component cannot be substituted and merely rehashed to pass. The package
boundary also binds the actual Cargo manifest hashes at compile time.

## Portable package

The independent `pakage` workspace is synchronized to base commit
`cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f` plus the explicitly uncommitted
R13-R43 Language Cortex worktree. Its manifest does not misstate that work as a
sealed commit.

- Adapter product sources: **42/42**, hash mismatches **0**
- Dockable semantic-core sources: **20/20**, hash mismatches **0**
- Research canaries copied into the package: **0**
- Minimal runtime boundary canaries: **4/4**
- Independent package unit tests: **386/386**
- Independent package format and Clippy checks: **PASS**
- Cargo network/LLM dependency hits: **0**
- Adapter-owned semantic state: **false**
- Research/recursive crates in the package workspace: **false**

The dependency direction is
`semantic-core-adapters -> dockable-semantic-core`. The default runtime is
Rust-only. `python-paddle-ocr` remains an optional, disabled-by-default
compatibility feature and is not used by the Language Cortex.

## Verification

- Adapter library tests: **359/359**
- Workspace library tests: **882/882**
- Workspace binary tests: **1/1**
- `cargo test --workspace`: **PASS**
- `cargo fmt --all -- --check`: **PASS**
- Strict Clippy with the two bounded historical harness exceptions: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Temporary R43 debug markers: **0**
- New R43 unit invariants: **6**
- Build cache cleanup: root **17,610 files / 28.6 GiB** and package
  **5,467 files / 3.4 GiB** removed; both `target` directories absent

## Safety and repository state

- External LLM calls: **0**
- Local teacher calls: **0**
- Network calls: **0**
- Python calls in the Language Cortex path: **0**
- Recursive source mutations: **0**
- Unsupported realization claims: **0**
- Language semantic authority: **false**
- Language execution authority: **false**
- Branch: `main`
- HEAD: `cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f`
- Commit created: **no**
- Push performed: **no**
- Worktree clean: **no**, because cumulative R13-R43 work remains intentionally
  uncommitted

The pre-existing user change in
`crates/semantic-reasoning/src/growth_supervisor.rs` remains exactly unchanged.

## Completion boundary

The planned six-axis engineering sequence has **zero major stages remaining**:
R43 was the final integration and package gate.

This is not a claim of unrestricted GPT-level equivalence. The validated system
is a deterministic, inspectable Korean/English Language Cortex with strong
typed composition, bounded long-context discourse, fail-closed ambiguity, and
evidence-grounded output. Open-domain knowledge breadth, generative linguistic
coverage, and human-level robustness remain unproven research objectives.
