# R27 Clause-Aware Discourse Focus

Status: **PASS**

R27 connects the R26 `ClauseGraphIR` to a typed, bounded multi-turn discourse-focus state. The system now distinguishes primary, secondary, and background centers and uses that state to resolve a later generic reference when flat recency cannot identify one unique antecedent. This is a bounded Korean/English discourse repair; it is not a claim of GPT-level language understanding.

## Result

The frozen 24-case diagnostic improved from **0/24** to **24/24**. It covers coordinated and sequential goal centering, subordinate-nucleus centering, contrastive pivots, focus retention across social turns, and invariance to fronted versus postposed subordinate clauses.

The separately frozen 16-case transfer suite scored **16/16 on its first semantic execution**. It used new entities and new Korean/English surfaces for coordination, sequence, subordination, and social-turn retention. No held-out oracle or expected result was changed.

Together these are **40/40 fresh R27 tasks** and **1067/1067 cumulative R1-R27 tasks**.

## Added semantic mechanisms

- `DiscourseFocusStateIR` stores bounded, hash-bound `PRIMARY`, `SECONDARY`, and `BACKGROUND` centers and typed focus transitions.
- Each center records its source frame, source clause, clause function, governing relation, salience, and source order.
- Clause relations select a center from semantic structure rather than the last noun mentioned on the surface.
- A viable target of a contrast edge can become the conversational pivot without creating a false intent-competition clarification.
- Gratitude and acknowledgements preserve an existing multi-clause center. `알겠어` is treated as an acknowledgement rather than a new task subject.
- Generic `it` and `그것` use the clause-aware center only when the older recency state has no unique antecedent or the focus state is genuinely multi-centered. Simple one-referent cases retain their prior binding contract.
- Expletive `it`, multiple-pronoun sentences, explicit topic returns, and unresolved ambiguity remain protected from over-broad focus substitution.
- Focus state can never grant semantic or execution authority: every node and transition enforces `semantic_authority=false` and `external_execution_authorized=false`.

## Regression repair

The first full-canary run after the diagnostic fix scored **39/45**. It exposed an integration boundary: the new focus fallback was intercepting simple one-referent pronouns and explicit-topic social turns that already had sealed behavior. The override was narrowed to structurally multi-centered or non-uniquely-resolvable contexts. The existing binding and response contracts then returned to green while both R27 suites remained green.

## Relation to the six language capabilities

The dependency order is correct, but the six items are not all solved by this stage.

1. **Grammatical composition:** R26's bounded typed clause graph remains intact; unrestricted grammar is still open.
2. **Discourse/topic state:** R27 now drives bounded focus transitions from clause structure and preserves focus across tested social turns. Multi-speaker and unrestricted long-horizon discourse remain open.
3. **Deixis/ellipsis:** earlier bounded resolution remains intact and can now consult typed focus as a fallback. General bridging, implicit arguments, and ambiguity calibration are the next dependency.
4. **Speech intent/pragmatics:** prior authority and intent gates pass regression, but broad indirect speech-act inference remains open.
5. **Plan/result distinction:** prior typed boundaries still pass; arbitrary tool interaction histories are not yet covered.
6. **Evidence-grounded realization:** prior faithful realization still passes; open-domain claim-level provenance remains open.

The next program stage should therefore be **deixis and ellipsis over the typed discourse-focus state**, followed by pragmatics, plan/result interpretation, and evidence-grounded realization.

## Verification

- R27 diagnostic: **24/24**
- R27 held-out transfer, first execution: **16/16**
- All adapter canary binaries: **45/45**
- Adapter unit tests: **283/283**
- Workspace tests: **807/807**
- `cargo fmt --all --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Frozen diagnostic and transfer hashes: **UNCHANGED**
- External LLM, local teacher, network, Python language-path, external action, and recursive source-mutation calls: **0**

The implementation is pure Rust. No commit or push was performed. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` was preserved and not edited by R27.

## Remaining boundary

R27 does not establish general discourse understanding or GPT-level language ability. Remaining work includes competing long-horizon centers, multi-speaker state, bridging reference, implicit and ambiguous ellipsis, broad pragmatic intent, plan-versus-observed-result interpretation across tools, and evidence-backed open-domain realization.
