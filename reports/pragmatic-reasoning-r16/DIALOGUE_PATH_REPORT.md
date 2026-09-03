# R16 Bounded Multi-Hop Dialogue Paths and Truth Maintenance

Status: `PASS`

R16 extends the R15 conversation-local relation graph from direct edges to bounded explanation paths. Korean and English cause questions traverse backward, consequence questions traverse forward, and a path may contain at most six causal edges. At most eight alternative paths are returned. When a longer path exists, the answer exposes truncation instead of implying completeness.

The graph now binds every relation endpoint to its epistemic belief record. Retraction or supersession is synchronized into `ACTIVE`, `SOURCE_INACTIVE`, `TARGET_INACTIVE`, or `BOTH_INACTIVE` edge status. Inactive edges remain auditable but cannot answer a current relation question. Modal worlds, proposition polarity, and contested belief status remain visible in path evidence. A concession edge is a discourse contrast, not a causal bridge, and is never inserted into a causal path.

## Final evidence

- Frozen diagnostic suite: `30/30`
- Held-out transfer and attack suite: initial `19/20`, repaired final `20/20`
- R16 fresh total: `50/50`
- Prior sealed R1-R15 tasks: `577/577`
- Cumulative R1-R16 canaries: `627/627`
- Canary binaries: `23/23`
- Adapter unit tests: `241/241`
- Workspace tests: `765/765`
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Final build-cache cleanup: 9,871 files / 8.5 GiB removed; `target/` absent

The diagnostic harness failure and the first held-out transfer failure are preserved under `failed_runs/`. The diagnostic harness initially used a Korean surface label inside an ASCII-only conversation identifier; no Korean semantic case was changed. The held-out failure exposed double English stemming (`readiness → readines → readine`), which was repaired by canonicalizing each comparison side once.

## Boundary

All relation paths remain dialogue claims. They establish neither world-level causal truth nor semantic authority and cannot authorize external execution. The canonical R16 path is pure Rust and made zero external LLM, local teacher, network, or recursive source-mutation calls.

No commit or push was performed. The worktree remains intentionally dirty because the sealed but uncommitted R13-R16 increments coexist. The broader GPT-level language goal remains open; R16 is one verified increment, not a completion claim.
