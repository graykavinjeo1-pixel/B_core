# R22 Structural Realization and Indexed Discourse

Status: `PASS`

R22 advances the six requested language axes without adding a neural language model or allowing text to become semantic authority. It turns typed multi-goal structures into natural Korean or English, preserves blocked goals as explicit exclusions, navigates bounded topic history, and binds local ordinal references. This is a measured extension of the inspectable Rust Language Cortex, not a claim of general GPT-class language coverage.

## What changed

Coordinated and sequential goals are now realized from selected typed candidates and goal-graph relations. Two- and three-step plans name each action and subject without exposing labels such as `compositional_goal_graph`, `Investigate`, or `Repair`. A prohibited candidate is stated as excluded rather than silently omitted. Every such response distinguishes a proposed plan from an executed and verified result.

The discourse state now supports bounded references to two or three prior topics in Korean and English. Topic history is selected by stack position, while Korean and English names remain lexical aliases of one shared concept identity. A later pronoun is realized in the current output language.

Same-turn ordinal expressions such as `첫 번째`, `둘째`, `the first`, and `the third` bind to typed local antecedents before global recency is consulted. A requested ordinal outside the available set fails closed and produces a natural clarification rather than inventing an entity.

## Frozen blind evidence

- Diagnostic source hash: `26f45f5a2ae4442837fd9169acc8b28870cdc8f8ef4608a289434333723ffed1`
- Initial diagnostic baseline: `0/24`
- Initial failures: typed multi-goal realization `4/4`, prohibition-aware realization `4/4`, indexed topic history `4/4`, local ordinal binding `4/4`, nested composition realization `4/4`, composed plan/result fidelity `4/4`
- Final diagnostic: `24/24`
- Original held-out source hash: `b67b35f48ba8249eaeb44376dfdc715b1762524487c2df3c2dab66a666ff597e`
- First raw held-out harness result: `14/16`
- First-execution semantic behavior after trace inspection: `16/16`
- Repaired held-out oracle hash: `d1bcb8a390a92e6023084dd769312a84c40192db9a4c1a1a4ee4011436f06e15`
- Result after oracle repair: `16/16`
- Fresh R22 tasks with the valid oracle: `40/40`
- Cumulative R1-R22 tasks: `867/867`

The two first-run harness failures are retained rather than erased. In both, the selected action was already correct (`서버를 분석해` and `repair cache`) and all distractors were absent. The defective assertion compared the pre-action topic state's single lexical surface with the language of a future action turn. That was impossible for the state to predict and contradicted the shared-concept boundary. Only the oracle was repaired: topic state is now checked by `concept_id_hint`, while the later resolved action is still checked in its requested language. No product code changed in response to those two rows.

## Verification

- Canary binaries: `35/35`
- Adapter unit tests: `273/273`
- Workspace tests: `797/797`
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- External LLM, local teacher, network, canonical-language Python, and recursive source-mutation calls: `0`
- Unsupported explanation facts: `0`

The workspace test command exercised pre-existing optional Python-environment probes. Python supplied no language inference, plan, or answer.

## Relation to the six language goals

R22 directly advances grammatical composition, discourse/topic state, deixis and ellipsis resolution, plan-versus-result separation, and evidence-grounded realization. Speech intent and pragmatic inference were preserved through all prior canaries but were not broadened in this run.

None of the six axes is declared generally complete. Remaining gaps include relative-clause attachment, quantifier scope across nested discourse, longer anaphoric event chains, and open-ended lexical coverage and generative fluency. The next iteration should target those gaps with frozen structural tests rather than expanding a phrase catalog.

No commit or push was performed. The worktree remains intentionally dirty with the uncommitted R13-R22 increments and the pre-existing user change in `growth_supervisor.rs`.
