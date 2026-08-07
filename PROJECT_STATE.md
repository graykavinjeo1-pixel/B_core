# Project State

**Canonical version:** 1.0.0
**State frozen for:** Stage S0 baseline

This is an operational snapshot. It does not replace or override
`CONSTITUTION.md`.

| Field | Value |
|---|---|
| Current stage | `S0` |
| Stage S0 status | `PASS` |
| SEM-0 implementation | `NOT STARTED` |
| Recursive self-mutation | `DISABLED` |
| Recursive-improvement mode | `OBSERVE_MEASURE_ONLY` |
| Proposal generation | `DISABLED` |
| Source patch/apply | `DISABLED` |
| Auto apply/merge/commit/push | `DISABLED` |
| External provider repair | `DISABLED` |
| Recursive benchmark-driven mutation | `DISABLED` |
| LLM reasoning dependency | `DISABLED` |
| Web/network learning | `DISABLED` |
| Current branch | `main` |
| Current commit | `SELF` — the commit containing this frozen state; resolve with `git rev-parse HEAD` |
| Worktree at committed freeze | `CLEAN` |
| Next allowed stage | `SEM-0_MINIMAL_AUTONOMOUS_CONCEPT_EMERGENCE` |

## Canonical document status

The following version 1.0.0 documents are complete and frozen:

- `CONSTITUTION.md`
- `RESEARCH_HYPOTHESIS.md`
- `SEMANTIC_SUBSTRATE_SPEC.md`
- `REASONING_ARCHITECTURE.md`
- `EXPERIMENT_PROTOCOL.md`
- `ROADMAP.md`
- `PROJECT_STATE.md`
- `docs/CANONICAL_READING_ORDER.md`

Their exact byte lengths and SHA-256 values are recorded in
`docs/CANONICAL_MANIFEST.json`. The manifest verifier must pass before a run is
eligible to claim canonical compliance.

## Inherited SYNAPSE status

- Sparse activation, indexing, and routing infrastructure is retained and
  available only as infrastructure; it is not semantic evidence.
- The inherited graph, language-oriented concept, hot-cache, cognition,
  embryo, and product-specific abstractions are retained for audit but are not
  authorized SEM-0 semantic primitives without an explicit SEM-0 design and
  contamination review.
- All inherited recursive-improvement implementation modules are retained as
  frozen source but excluded from the compiled public crate boundary.
- The compiled recursive crate exposes only the immutable S0 quarantine status
  plus the non-recursive core re-export.
- The inherited `patch_sandbox` source contains the only filesystem-write path
  found in the recursive stack; it can write sandbox copies but is disabled by
  the crate-boundary quarantine.
- No process execution, Git commit/push/merge implementation, network client or
  server, external LLM, or external repair-provider dependency was found.

The detailed classification and contamination decisions are in
`reports/stage_s0/inherited_component_inventory.json`.

## Advancement constraint

Starting SEM-0 requires an explicit subsequent task. It may implement only the
minimal closed-world concept-emergence experiment governed by the canonical
documents. S0 completion does not authorize SEM-1 or recursive self-application.
