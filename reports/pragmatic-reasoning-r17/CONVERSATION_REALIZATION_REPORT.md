# R17 Grounded Conversational Realization and Focus Fidelity

Status: `PASS`

R17 repairs the boundary between the already validated semantic plan and the final chat surface. The prior conversation renderer only asserted that a plan existed and then emitted a generic promise. It now selects three representative operations from the validated `PlanIR`, states that they are planned rather than completed, and preserves `unsupported_freeform_claims=0`.

User-grounded acronyms are restored without a fixed allowlist. Only bounded ASCII alphanumeric tokens that occur in the current user input with at least two uppercase letters and no lowercase letters may restore casing. Their spelling can persist through the existing result/event referent, but cannot modify semantic payloads or introduce an unmentioned acronym.

Pure gratitude and backchannels already preserved the underlying referent. The remaining focus bug was that a later request such as “그 결과를 설명해” installed a new explanation goal and replaced the underlying active task. Result, event, and proposition references now reuse the existing active goal while the explanation receives its own grounded plan. Korean and English affect surfaces are realized directly, while quoted affect is excluded and affect alone creates no mutation authority.

## Final evidence

- Frozen diagnostic suite: initial `1/24`, effective `0/24` because the only pass was a weak echoed-token assertion; repaired final `24/24`
- Held-out transfer and attack suite: harness compile failure before any case ran; first semantic execution `12/16`; repaired final `16/16`
- R17 fresh total: `40/40`
- Prior sealed R1-R16 tasks: `627/627`
- Cumulative R1-R17 canaries: `667/667`
- Canary binaries: `25/25`
- Adapter unit tests: `249/249`
- Workspace tests: `773/773`
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- Final build-cache cleanup: 9,442 files / 8.6 GiB removed

The initial diagnostic weakness, held-out harness compile defect, and first held-out semantic failures are preserved under `failed_runs/`. The held-out failures all retained their typed reference and acronym surface; they failed because the explanation request replaced the active goal IDs. No held-out predicate was weakened to obtain the final result.

## Architectural conclusion

The observed R12 chat defects do not establish that a new 3B–9B language model is required. In the inspected path, B_Core had already produced an 11–14 step semantic plan, but the final renderer discarded it. The repaired deterministic path now carries that plan, focus, acronym provenance, and affect boundary into the response.

This result also does not prove that a symbolic renderer alone can match GPT-class open-domain fluency. Meaning nodes plus language-specific forms require compositional grammar, discourse tracking, and realization mechanisms; nodes alone are insufficient. If future blind evaluation shows a residual surface-diversity ceiling, a learned decoder may be added only as an IR-conditioned, non-authoritative realizer. That future decision is not justified by the defects repaired here.

The canonical R17 path is pure Rust and made zero external LLM, local teacher, network, Python, or recursive source-mutation calls. No commit or push was performed. The worktree remains intentionally dirty because the sealed but uncommitted R13-R17 increments coexist. The broader GPT-level language goal remains open; R17 is a bounded verified increment.
