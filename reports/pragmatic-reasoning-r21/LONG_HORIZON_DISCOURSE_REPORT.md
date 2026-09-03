# R21 Long-Horizon Discourse and Ordered Reference

Status: `PASS`

R21 repairs two concrete gaps left after R20: references to two entities introduced in the same sentence, and navigation back to the immediately previous discourse topic. It also rechecks long-delay focus and evidence preservation. This is a bounded advance toward broad language understanding, not a claim of GPT-class coverage.

## What changed

`전자/후자` and `former/latter` now bind through a typed local ordered-reference path. The resolver extracts the ordered same-turn nominal set before consulting global recency, preserves Korean case/topic particles, and attaches non-authoritative binding evidence. Exactly two candidates are required. Three or more candidates produce clarification and no plan; the internal ambiguity code is not exposed in the user-facing sentence.

`이전 주제`, `아까 주제`, `previous topic`, and `prior topic` are now typed as a topic-stack operation rather than stored as literal topic names. The operation selects the second hash-bound active topic and rotates it to the front. Known Korean/English aliases share the same concept identity. Open-vocabulary topics remain surface-bound discourse state and do not create or mutate semantic concepts.

Explicit topic focus survived six to eight social turns, including hesitation and gratitude, before later pronominal action. Source-attributed conflicting claims remained available after five social turns, but were not promoted to dialogue truth. Topic-return output was also changed to a Korean particle-neutral form.

## Frozen blind evidence

- Diagnostic source hash: `69bef7cfbd46bb43bc45c738b68cff383ac9f545cad89160d83ef7d6d0f9f590`
- Initial diagnostic baseline: `12/24`
- Initial failures: same-turn local order `4/4`, previous-topic stack `4/4`, scoped local order `4/4`
- Final diagnostic: `24/24`
- Held-out source hash: `3442050269a5ce9be67e4cc5ed04557f76bb8f7b3da18fa82f8a219d5f5e831a`
- First semantic execution of held-out suite: `16/16`
- Fresh R21 tasks: `40/40`
- Cumulative R1-R21 tasks: `827/827`

The held-out suite covered cross-language previous-topic navigation, contrastive ordered references, eight-turn open-vocabulary focus, and three-way ambiguity attacks. Neither frozen file changed after its recorded hash.

## Verification

- Canary binaries: `33/33`
- Adapter unit tests: `269/269`
- Workspace tests: `793/793`
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- External LLM, local teacher, network, canonical-language Python, and recursive source-mutation calls: `0`
- Unsupported explanation facts: `0`

The full workspace test command exercised pre-existing optional Python-environment probes. Their missing-`pytest` observation was handled by Rust fail-closed tests; Python did not supply language inference or answers.

## Relation to the six language goals

R21 directly advances discourse/topic state, deixis/ellipsis resolution, and evidence-grounded realization. Grammatical composition, pragmatic intent, and plan-versus-result typing were inherited from R20 and passed all regression canaries. None of the six is declared generally complete: the implementation remains inspectable and bounded.

The next residual is broader compositional realization and reference structure: more than two coordinated referents, nested clause attachment, non-immediate topic history, and natural rendering of multi-goal plans without exposing internal IR labels. These should be attacked together because a fluent sentence must be generated from verified semantic structure, not from raw input or an ungrounded language model.

No commit or push was performed. The worktree remains intentionally dirty with the uncommitted R13-R21 increments and the pre-existing user change in `growth_supervisor.rs`.
