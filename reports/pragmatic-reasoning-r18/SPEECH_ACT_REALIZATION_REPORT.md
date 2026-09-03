# R18 Speech-Act and Evidence-State Realization

Status: `PASS`

R18 repairs response selection after semantic interpretation. A user statement, a complaint about the answer, an explicit request, and a question about an earlier result are no longer sent through the same generic planning path.

The new `UserFeedbackIR` records bounded feedback kinds (`Unhelpful`, `Misunderstood`, `MissedPoint`, `TooVerbose`, `TooBrief`, and `Incorrect`) with evidence clause IDs. Pure feedback is realized directly without inventing a task plan. Feedback followed by an explicit request preserves the request's authority and produces a plan only for that requested action. Quoted feedback remains reported content rather than becoming the user's own evaluation.

An `Inform` turn now records a proposition referent without promoting the report to established fact or fabricating a plan. A bound result reference checks the evidence state before generic question answering. If the core has a validated plan but no execution receipt, it says that no execution result is recorded and does not pretend that the plan ran.

## Final evidence

- Frozen diagnostic suite: initial `0/24`; repaired final `24/24`
- Held-out transfer and attack suite: first semantic execution `15/16`; repaired final `16/16`
- R18 fresh total: `40/40`
- R17 regression: `40/40`
- Prior sealed R1-R17 tasks: `667/667`
- Cumulative R1-R18 canaries: `707/707`
- Canary binaries: `27/27`
- Adapter unit tests: `253/253`
- Workspace tests: `777/777`
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- Final build-cache cleanup: 10,798 files / 8.9 GiB removed

The first diagnostic and first held-out failures are preserved under `failed_runs/`. The held-out predicate was not weakened. Its only failure was repaired by letting an exact typed `ResultReference` evidence-state check outrank generic presupposition question answering.

## Architectural conclusion

The tested chat defects still do not justify the claim that B_Core must contain a newly trained 3B-9B autoregressive model. They were concrete loss-of-information and response-routing defects: distinct speech acts collapsed into generic plans, and a question about a non-existent execution result bypassed the evidence-state boundary. The repaired pure-Rust path now preserves those distinctions.

This does not prove GPT-class open-domain fluency. Meaning nodes plus per-language expressions are not sufficient by themselves; natural dialogue additionally requires compositional grammar, discourse state, reference resolution, pragmatic inference, evidence-state tracking, and a realization policy. B_Core can continue building those mechanisms symbolically and compositionally. A learned decoder remains an optional future surface-realization component only if blind evaluation demonstrates a residual fluency ceiling; it must remain IR-conditioned and non-authoritative.

The canonical R18 path made zero external LLM, local teacher, network, Python, or recursive source-mutation calls. No commit or push was performed. The worktree remains intentionally dirty because the sealed but uncommitted R13-R18 increments coexist. The broader GPT-level language goal remains open; R18 is a bounded verified increment.
