# R46 Typed Cross-Turn Discourse Program Report

Status: **PASS**

R46 adds a typed, hashed `DiscourseProgramIR` to conversation state. A turn
such as “캐시를 확인하고 수리해” can contribute an ordered two-step program;
an explicit later target such as “인덱스도 똑같이 해” instantiates the same
ordered semantic operations for the new target. The instantiated controlled
utterance is sent back through the ordinary grammar, compositional semantics,
and GoalIR path. It is not a whole-sentence source or answer template.

## Frozen evaluation

- Diagnostic preimplementation baseline: **0/12**
- Diagnostic final: **12/12**
- Held-out transfer first exposure: **7/8**
- Held-out transfer strict second exposure: **7/8**
- Held-out transfer final: **8/8**
- Fresh R46 tasks: **20/20**
- Oracle corrections: **0**
- Diagnostic output SHA-256:
  `4FAE3D4766B7B69E079008938CBB9CE59A6B140EC675FC35033220D3777FD906`
- Transfer output SHA-256:
  `D948EC75AA9BC7736F0648047AB6C9439DC521DE4E6BCB02A7939D02C8017336`

The first transfer failure showed that a shorter closed alias (`파일`) could
overwrite a fresh grammatical noun phrase (`설정 파일`). Open noun-phrase
grounding now wins when it provides the more specific target. A later strict
harness run exposed a same-target English negative frame whose surface theme
was unresolved even though R45's semantic-role graph had the correct shared
argument. R46 now uses that typed binding as target-cohort evidence. A
different-target negative frame remains outside the program cohort, preserving
R20's safe parallel ellipsis behavior.

Two harness-only repairs did not change expected answers: the missing-program
case recognizes the existing `ELLIPTICAL_GOAL` evidence marker, and either R46
canary exits nonzero if any row fails.

## Structural boundary

`B_CORE_CONVERSATION_STATE_22` stores `B_CORE_DISCOURSE_PROGRAM_IR_1` records.
Every program contains ordered steps, source and blocked frame counts, a shared
subject, replayability state, turn provenance, authority flags, and a SHA-256
integrity field. Validation rejects a tampered hash.

A program can be replayed only when it is complete, positive, unquoted,
unwithdrawn, and explicitly rebound to a new target. A bare multi-action
“그대로 해” remains ambiguous. Partial or same-target negated programs,
quoted programs, missing programs, and withdrawn programs fail closed. Topic
restoration selects a prior matching program without granting semantic or
external execution authority.

The language layer still cannot mutate semantic payloads. Program storage and
aliasing create no semantic concept generation, and program instantiation does
not authorize external execution.

## Regression evidence

- R46 diagnostic and held-out transfer: **20/20**
- Current R20 compatibility rows: **24/24**
- Adapter library tests: **387/387**
- Current workspace library tests: **910/910**
- `cargo test --workspace --lib`: **PASS**
- `cargo fmt --all -- --check`: **PASS**
- Current-impact R46/R20 Clippy with warnings denied: **PASS**
- New R46 unit invariants: **5**
- Cargo-metadata workspace canaries: **87**
- Previously sealed R1-R45 workspace canaries: **85/85**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`

The full current 87-canary set was not relinked and rerun. R45 already sealed
85/85; this stage reran both new R46 suites and the affected R20 compatibility
suite, plus all current library tests and the independent portable package.
An attempted root `--all-targets` Clippy run reached the historical canaries
without code diagnostics, then Windows denied removal of an invalidated
incremental `dep-graph.bin` (`os error 5`). The same current-impact targets
passed with `CARGO_INCREMENTAL=0`. This infrastructure event is not represented
as an all-target success.

## Portable package

The product-only `pakage` directory is synchronized as
`B_CORE_PORTABLE_PRODUCT_CORE_R46_WORKTREE_ABI1`:

- Adapter product sources: **43/43**, hash mismatches 0
- Dockable core files: **20/20**, hash mismatches 0
- R46 research canaries included: **0**
- Package workspace tests: **414/414**
- Minimal runtime canaries: **4/4**
- Package fmt and all-target Clippy: **PASS**
- Network/LLM Cargo dependency hits: **0**
- Prebuilt core canary manifest hash and byte count: **PASS**

The package contains only the dockable core and product language adapters.
Research campaigns, recursive source mutation, and semantic-reasoning workspace
machinery remain outside the portable boundary.

## Safety and cleanup

External LLM calls, local teacher calls, network calls, Python calls in the R46
language path, and recursive source mutations are all **0**. Sparse runtime
checks retain `FULL_CATALOG_SCANS=0` and `ROUTING_FALSE_NEGATIVES=0`. The
pre-existing user change in `growth_supervisor.rs` remains exactly preserved.

After validation, root cleanup removed 5,980 files (3,845,380,736 bytes), and
package cleanup removed 5,687 files (4,066,309,722 bytes). Both `target`
directories are absent.

R46 is complete. The broader GPT-level objective is not complete, and no
unrestricted GPT-level equivalence is claimed. Assuming each remaining stage
succeeds, four stages remain: R47, R48, R49, and R50 final integration.
