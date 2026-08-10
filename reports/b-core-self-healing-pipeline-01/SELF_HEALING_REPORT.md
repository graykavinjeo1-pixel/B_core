# B_Core self-healing pipeline report

## Outcome

`PASS` for the compiled and registered project surface at code commit
`13c8b924dfdfd2414fab0fee82fb17e859badeb4`.

- Compiled Cargo targets: 82
- Quarantined source-only surfaces: 21
- Test suites: 82
- Tests passed: 314
- Tests failed in the final audit: 0
- `cargo fmt --check`: PASS
- clean compile canary (`incremental=0`, `jobs=1`): PASS
- workspace/all-target/all-feature tests: PASS
- doc tests: PASS
- workspace/all-target/all-feature Clippy with `-D warnings`: PASS

The claim is bounded: every registered compiled target and test passed. It does
not claim that no unknown latent defect can exist.

## Local-only execution policy

“Independent verification” means an independent local process and authority
boundary, not an external service.

```text
VERIFICATION_MODE=INDEPENDENT_LOCAL_DETERMINISTIC_PROCESS
CODEX_CALLS=0
EXTERNAL_LLM_CALLS=0
NETWORK_READS=0
NETWORK_WRITES=0
HUMAN_VERIFICATION_DECISIONS=0
```

The verification attestation is rejected if any of those dependency counters
is non-zero, if the verifier/command hashes are missing, or if the result was
not derived only from frozen deterministic checks. The proposer still cannot
approve its own candidate.

## Implemented repair loop

```text
module inventory
→ bounded local probe
→ frozen Observation / DefectContract / RepairSpec
→ core primitive composition
→ proposal-only sparse patch
→ independent local deterministic verification
→ isolated post-install regression
→ provisional lesson
→ fresh non-identical local transfer attempts
→ independent local deterministic transfer verification
→ promoted persistent lesson memory
```

Unknown defect classes stop with `CAPABILITY_GAP`. They are not routed to
Codex, an external LLM, or the network.

## Learned code-composition recipe

The first generalized lesson records how existing code capabilities must be
assembled, rather than retaining an exact patch:

```text
FrozenRepairContext
→ PREDICATE_LOCATOR
→ PredicateSpan
→ EXPRESSION_BOUNDARY
→ BoundedPredicate
→ DIVISIBILITY_REWRITE
→ RewrittenSource
→ SPARSE_DIFF
→ SparseDiff
→ RSI_PATCH_CANDIDATE
→ PatchCandidateIR
```

- Primitive count: 5
- Typed recombination edges: 4
- Required roles: localization, expression-boundary preservation,
  transformation, sparse packaging, consequence prediction
- Exact patch lookups: 0
- Task identity routing: 0
- Repository identity routing: 0
- Core self-approval events: 0
- Original-source direct writes by the core runner: 0

Promotion requires at least two independently verified fresh scenarios.
Different expression shapes and both equality/inequality forms are covered;
non-zero remainder comparisons are explicitly outside applicability and cause
an abstention.

## Defects found and repaired

### Repeated canonicalization defect class

Clippy found 22 `manual_is_multiple_of` defects across 11 files. They were
repaired without semantic test changes. A naive automatic fix also produced a
bad precedence candidate for `(... != 0).then(...)`; the compile gate rejected
it. Expression-boundary preservation was therefore made an explicit primitive
in the learned composition.

### SEM-26 nondeterministic diagnosis

The final audit reproduced the historical intermittent failure:

```text
sem26::engine::tests::full_synthesis_is_not_fixed_catalog_selection
assertion failed: repair.source_elements.len() >= 2
```

Root cause: the diagnostic selector ranked experiments by one absolute
`perturbed_time_ns` measurement. Scheduler noise could select the wrong cause,
empty the compatible element set, and leave a one-element genesis repair.

Repair: rank structural work-unit reduction first, observed time reduction
second, and stable experiment index third. A hostile regression fixture proves
that an unrealistically faster wrong experiment cannot override the real
structural effect. The previously flaky test then passed 20 consecutive local
runs, followed by the complete final audit.

### Toolchain transient separated from source defects

An earlier parallel incremental `--no-run` produced a Rust 1.96 compiler ICE,
while the following full build/test passed. Compiler ICE output is now
classified as a toolchain transient and rechecked with the local clean compile
canary instead of triggering a source patch.

## Remaining boundary

The old `synapse-recursive-core` self-development source remains quarantined.
It is inventoried but is not counted as compiled authority. Re-enabling it
would require a separate review rather than silently expanding the trusted
surface.

Canonical evidence is in `module-audit/audit_receipt.json` and its hashed probe
logs.
