# R49 — Typed compound guard expressions

Status: `PASS`

R49 removes the last single-predicate collapse in guarded discourse programs.
Conditional antecedents now compile to a bounded language-independent tree of
`ATOM`, `ALL`, `ANY`, and atom-level `NOT` nodes. AND binds more tightly than
OR, parentheses are preserved, and compound negation is normalized with De
Morgan transformations. Every tree is size/depth bounded, authority-free, and
SHA-256 bound inside the guarded program.

## Product outcome

- English and Korean compound conditions retain the same typed tree.
- Cross-turn and cross-language workflow rebinding retains the same expression
  hash, including nested precedence.
- `or` inside a condition no longer becomes an unresolved action alternative.
  A real action alternative still fails closed.
- Explicit subjects in every condition atom must match the guarded action
  subject. Mixed-target conditions are recorded but cannot become replayable
  programs.
- Natural-language and reported condition claims remain unable to activate a
  deferred action. The separate trusted evidence lifecycle from R48 is
  unchanged.
- `INVALID` and `NOT(VALID)` remain distinct canonical forms even when a
  language can express them similarly; Korean uses `무효` for the former so a
  round trip cannot silently change the tree hash.

## Frozen evaluation

The 12-case diagnostic was frozen before implementation. Its baseline was
`0/12`; after implementation it passed `12/12`. Rustfmt was the only change to
the shared harness bytes after the baseline, and both pre-format and final
hashes are recorded in the JSON report.

The separately frozen eight-case transfer suite scored `7/8` on first
exposure. The one failure was the `INVALID` versus `NOT(VALID)` Korean
realization mismatch described above. After the general lexical distinction
was repaired, the same frozen suite passed `8/8`; no expected answer or case
was changed.

R48 evidence lifecycle regressions passed `12/12` diagnostic and `8/8`
transfer cases under the new schemas.

## Verification

- R49 fresh suites: `20/20`
- Adapter library tests: `397/397`
- Root workspace substantive tests: `921/921`
- Portable package tests: `424/424`
- Portable runtime canaries: `4/4`
- Root and package format checks: pass
- Root and package all-target Clippy with warnings denied: pass
- Canonical manifest: pass, 10 files,
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- External LLM, local teacher, network, recursive source mutation, full catalog
  scan, and routing false-negative counts: `0`

The portable package remains Rust-only by default. All 43 product adapter
files match their root counterparts; research canaries remain excluded.

## Deployment boundary

The conversation state, discourse program, and guard schemas advance to
versions 25, 4, and 3 respectively. Existing persisted v24 conversation state
must be migrated or drained before deployment; it must not be silently loaded
under the new adapter. The core ABI remains version 1.

No commit or push was performed. The cumulative R13–R49 worktree remains
intentionally uncommitted. Build caches were removed after verification.

R49 is the last capability stage in this sequence. One success-assumed stage
remains: `R50_FINAL_INTEGRATION_REGRESSION_AND_SEALING`. R50 has not started.
