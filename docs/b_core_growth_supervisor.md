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

High-value learning requires an explicit local PASS or a structural code-plus-regression cohort. Documentation-only edits, failed work, generated output, unverified test-only work, and large unexplained churn are not promoted.

## Plateau behavior

After the configured number of scans without high-value evidence, the state becomes `WAITING_PLATEAU`. The supervisor continues observing at the fixed polling interval. It does not create empty generations or raise difficulty. New high-value evidence returns it to the normal campaign cycle.

## Bounds

The frozen configuration limits lifetime campaigns, accepted generations, active processing time, state bytes, total observed bytes, bytes per scan, files per scan, bytes per file, pending observations, observations per campaign, retained lessons, and consecutive failures. Reaching a hard bound produces `SAFE_STOPPED`; it does not silently reset a budget.

Only operator stop can be resumed in place. A hard resource stop requires a new explicitly frozen configuration and state line.

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

# Always-on foreground mode.
.\bin\b-core-growth-supervisor.exe run .\config\growth.json

# Stop at a safe cycle boundary.
.\bin\b-core-growth-supervisor.exe stop .\config\growth.json
```

Use `tools\install-growth-autostart.ps1` to register a limited-privilege `ONLOGON` scheduled task. Registration is never performed automatically by the package.

## Work-event integration

`tools\record-growth-work-event.ps1` records bounded provenance for work performed by a user, Codex, or a local tool. It stores actor, kind, outcome, scoped paths, a short summary, and optional evidence hashes—not command transcripts, chats, or source text.

An explicit `PASS` is evidence, not automatic authority. The independent verifier still decides whether the frozen structural lesson is promotable.
