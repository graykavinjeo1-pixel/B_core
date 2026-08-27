# B_Core Bounded Growth Supervisor

`b-core-growth-supervisor` is an always-on local coordinator for bounded observational learning. It does not continuously rewrite the authoritative repository. It learns generalized structural and composition lessons, requires a separate deterministic verifier process, and promotes only accepted memory generations.

## Trust boundary

- Watched roots must be explicitly configured and must exist.
- The state directory cannot be inside a watched root.
- Symlinks, secrets, `.env`, private keys, build outputs, dependency folders, generated files, binaries, oversized files, and non-UTF-8 files are not observed.
- Raw source fragments are read transiently for bounded structural feature extraction and are never stored in learned memory.
- Passive filesystem changes are attributed to `UNKNOWN_LOCAL_WRITER`. User, Codex, and local-tool attribution requires an explicit work-event receipt.
- Codex calls, external LLM calls, and network reads/writes remain zero inside the supervisor and verifier.
- The proposer cannot approve its own candidate. `b-core-growth-verifier` is a separate executable whose SHA-256 is frozen before candidate construction.

## Growth cycle

```text
scoped observation
→ learning-value classification
→ immutable campaign freeze
→ structural composition lesson candidate
→ separate deterministic verification
→ accepted generation promotion or failed-candidate deletion
→ predecessor-preserving next scan
```

The first scan only establishes a baseline. Existing files are not treated as newly learned work.

Subsequent scans reuse the frozen file index when size and modification identity are unchanged, and periodically perform a full-hash canary. A scan runs behind a watchdog: the lease heartbeat and operator-stop check remain responsive while filesystem I/O is in progress, and a scan exceeding the fixed runtime bound produces an auditable safe stop instead of an indefinitely stale `SCANNING` state.

High-value learning requires an explicit local PASS or a structural code-plus-regression cohort. Documentation-only edits, failed work, generated output, unverified test-only work, and large unexplained churn are not promoted.

Before campaign freeze, the supervisor applies the same PASS/code-plus-test evidence gate as the independent verifier. An unverifiable cohort is retained for reconsideration when new evidence arrives, but it does not consume a campaign or consecutive-failure budget merely to reproduce a known verifier rejection.

Evidence with the same work kinds, diagnostic signals, composition recipe, applicability, obligations, and before/after performance values is a semantic revalidation. It is consumed without starting a campaign or incrementing the generation. A measured performance frontier can still advance when a bound before/after value changes; merely attaching a different log to the same value cannot manufacture growth.

## Plateau behavior

After the configured number of scans without high-value evidence, the state becomes `WAITING_PLATEAU`. The supervisor continues observing at the fixed polling interval. It does not create empty generations or raise difficulty. New high-value evidence returns it to the normal campaign cycle.

## Bounds

The frozen configuration limits lifetime campaigns, accepted generations, active processing time, state bytes, total observed bytes, bytes per scan, files per scan, bytes per file, pending observations, observations per campaign, retained lessons, and consecutive failures. Reaching a hard bound produces `SAFE_STOPPED`; it does not silently reset a budget.

Only operator stop can be resumed in place. A hard resource stop requires a new explicitly frozen configuration and state line. `continue-lineage` creates that line without resetting accepted memory: it requires a clean `SAFE_STOPPED` predecessor, rejects policy drift, carries only the current and immediate predecessor memory plus bounded executable-knowledge/index stores, and excludes build products, old campaigns, mutable control files, and staging binaries. It performs no scan, repair, verifier run, or difficulty selection.

### Same-attempt source revision

An authoritative source repair that fails after exact rollback may trigger a
bounded counterexample-guided revision in the same Supervisor turn. The next
candidate must be different and its generalized change must consume the fresh
validation counterexample. Revision stops on a duplicate candidate, missing
counterexample consumption, transient workspace contention, uncertain
rollback, or after three executions. This shortens the repair feedback path
without turning retries into independent generations or an unbounded loop.

Validation commands stream stdout and stderr through bounded readers. Their
complete byte counts and SHA-256 identities remain in the receipt, while only
the first 4 MiB per stream and a bounded diagnostic tail are retained. A
timeout terminates the child process tree before the command is reaped so a
compiler or test runner cannot leave descendant processes running after the
source transaction rolls back.

## Crash and reboot recovery

State, index, journal, campaign freeze, candidate, verifier receipt, history, and memory generation records are written as immutable files. Startup loads the highest valid snapshot. A partially completed campaign is resumed deterministically; an already promoted generation is recognized from immutable history; a divergent generation is rejected. Promotion is idempotent.

Only the current and immediate predecessor full memory snapshots are retained. Small immutable campaign and verification receipts remain as the audit trail. Failed candidates are deleted and the accepted predecessor stays current.

## Commands

```powershell
# Create a bounded config. The verifier is discovered beside the supervisor.
.\bin\b-core-growth-supervisor.exe make-config `
  .\config\growth.json `
  D:\Projects\WatchedWorkspace `
  D:\B_Core_Growth_State

# Freeze/initialize, one deterministic cycle, status.
.\bin\b-core-growth-supervisor.exe init .\config\growth.json
.\bin\b-core-growth-supervisor.exe step .\config\growth.json
.\bin\b-core-growth-supervisor.exe status .\config\growth.json

# Continue an exact hard-stopped lineage under a separately frozen, larger
# resource envelope. The successor state directory must not already exist.
.\bin\b-core-growth-supervisor.exe continue-lineage `
  .\config\growth-r4.json `
  .\config\growth-r5.json

# Always-on foreground mode.
.\bin\b-core-growth-supervisor.exe run .\config\growth.json

# Stop at a safe cycle boundary.
.\bin\b-core-growth-supervisor.exe stop .\config\growth.json
```

### Compound reasoning integration

The normal `step` and `run` paths also consume bounded compound-growth inputs.
An input must contain typed mechanisms, execution traces, hypotheses,
counterexamples, source bindings, or operator outcomes together with hashed
evidence. The caller cannot provide the generation or replace the accumulated
operator repository; both are supplied by the Supervisor's local sealed state.

```powershell
# Queue typed evidence. The ordinary Supervisor loop commits it once.
.\bin\b-core-growth-supervisor.exe record-compound-input `
  .\config\growth.json `
  .\compound-input.json

# Inspect the canonical bounded repository and latest cycle.
.\bin\b-core-growth-supervisor.exe compound-status .\config\growth.json
```

Committed inputs and deterministic cycle results form an immutable hash chain
under the Supervisor state directory. Text-only inputs, missing evidence,
duplicate identities with different content, and caller-supplied repository
authority fail closed. Compound cycles perform no network or external-model
calls and do not install source changes without the existing atomic validation
and rollback path.

Use `tools\install-growth-autostart.ps1` to register a limited-privilege `ONLOGON` scheduled task. Registration is never performed automatically by the package.

## Work-event integration

`tools\record-growth-work-event.ps1` records bounded provenance for work performed by a user, Codex, or a local tool. It stores actor, kind, outcome, scoped paths, a short summary, and optional evidence hashes—not command transcripts, chats, or source text.

An explicit `PASS` is evidence, not automatic authority. The independent verifier still decides whether the frozen structural lesson is promotable.

### Repository issue intake and autonomous repair contracts

Repository problems may enter the same product path as structured public
evidence. The statement must contain both observed and expected behavior and
must bind existing files inside a configured watched root.

```powershell
.\bin\b-core-growth-supervisor.exe record-repository-issue `
  .\config\growth.json `
  .\repository-issue.json
```

The request schema is `B_CORE_REPOSITORY_ISSUE_INTAKE_REQUEST_1` and contains
`issue_id`, `problem_statement`, `paths`, optional `evidence_artifacts`, and
optional `occurred_at_ms`. Natural language is localization evidence only. It
cannot authorize a patch or become executable knowledge by itself.

For a failing native validation, the supervisor autonomously derives a
`RepositoryRepairContractIR` that binds the issue evidence, exact target
symbols, bounded composition budget, generic edit atoms, public behavioral
verification, and atomic install/rollback obligations. That contract controls
the common typed compiler path. A successful materialized repair is promoted
to `ImprovementOperatorIR` only after sandbox verification and, when mutation
is enabled, authoritative installation plus revalidation. The issue,
validation, contract, synthesis, candidate, verifier output, installation, and
operator promotion remain connected by a validated causal provenance graph.

For a `PERFORMANCE_OPTIMIZATION` event, `-PerformanceMetricsPath` may point to a JSON array. Each entry contains `metric`, integer `before`, integer `after`, `lower_is_better`, and `evidence_sha256`. The digest must be bound to one of the supplied evidence files. A non-improving measurement is retained as negative evidence and is not promoted as a performance gain.
