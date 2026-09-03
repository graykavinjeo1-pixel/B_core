# R20 Integrated Dialogue Coherence

Status: `PASS`

R20 joins the six previously separated language concerns into one bounded conversation path: grammatical composition; discourse and topic state; deixis and ellipsis; speech intent and pragmatic inference; plan versus execution-result status; and evidence-grounded realization. The stage does not claim general or GPT-class language understanding.

An explicit topic return is now typed discourse management, not a work request. Korean and English surface forms that share one topic Concept ID collapse to one hash-bound topic state. An explicit return can override ordinary recency, while automatically observed goal topics remain weak and cannot steal the existing entity-reference path. Hesitation, gratitude, and other social turns preserve the explicit focus without creating a plan.

Within a turn, the nearest compatible nominal outranks an older global referent. Cross-turn argument correction and parallel ellipsis run before generic topic-pronoun fallback, so `그거 말고 폴더로` and `same for the backup` inherit only the authorized action. Korean `하되 ... 하지 마` composition keeps the requested action and preserves the prohibition as a blocked candidate.

Continuation decisions remain evidence gated. A score, benchmark, attributed report, uncertain statement, or proxy metric cannot stand in for the required actual benefit. Questions such as `그러면 계속해도 돼?` return the unresolved gate state without creating a continuation plan. Actuality questions cite competing dialogue records and explicitly abstain from real-world truth. Plans continue to be realized as plans; delayed result questions report that no execution receipt exists.

## Blind-suite history

- Diagnostic source before first execution: `3a01e3b712b03e356b28237d978e5bca9ccf57507cabb1e0ae598190726bc88a`
- Original diagnostic baseline: `1/24`
- First harness correction: the original competing-source cases incorrectly expected a singular recent proposition to be ambiguous. They were replaced with direct actuality questions over competing sources. Corrected pre-repair hash: `869e232b234d48ba7876fc16e7a3e280437737c68eff85466b9ec43e9a94a9dc`; corrected baseline remained `1/24`.
- Second diagnostic harness correction: English no-result matching was made case-insensitive, and the expected predicate was corrected from nonexistent `ANALYZE` to the established canonical `INVESTIGATE`. Inputs and product behavior did not change. Final diagnostic hash: `da8d38f63db0f37759789a4b1cd55ca5b2e5585f89a3143ab68d67fbbffabe51`.
- Diagnostic progression after repair: `14/24`, `23/24`, final `24/24`.
- Held-out source remained `9c48b97b16ebe1cb6a84f5c24c4421efac717fa0b4e1f4291660d7b89525ac3d` until first semantic execution.
- First held-out execution exposed a Korean/English topic-alias ID collision and stopped before producing rows. After that product repair, the unchanged suite produced `12/16`.
- Two of the four remaining failures were the same case-sensitive English no-result assertion. The held-out harness alone was corrected to `df7b9be37c91a6333cb6b8ae0dd63f61fcd322de287af5ddc09ec5b965519be2`; the other two failures required product repair for condition-subject task recovery. Final held-out result: `16/16`.

## Final evidence

- R20 fresh integrated tasks: `40/40`
- Cumulative R1–R20 tasks: `787/787`
- Canary binaries: `31/31`
- Adapter unit tests: `267/267`
- Workspace tests: `791/791`
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- Canonical language path: zero external LLM, teacher, network, Python, or recursive source-mutation calls
- Unsupported explanation facts: `0`

The full workspace test command exercised a pre-existing optional Python-environment probe, which reported that `pytest` was unavailable and then passed its Rust-side fail-closed test. No R20 language inference or expected answer was obtained from Python.

## Remaining boundary

The current resolver is still bounded: topic aliases are deliberately small, same-turn reference priority handles controlled structures, and realization remains more templated than a generative language model. Long-context interference, nested multi-reference sentences, open-vocabulary grounding, broader ambiguity, and freer evidence-faithful expression remain the next research boundary.

No commit or push was performed. The worktree remains intentionally dirty with the uncommitted R13–R20 increments.
