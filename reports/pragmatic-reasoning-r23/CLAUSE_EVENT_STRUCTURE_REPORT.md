# R23 Relative-Clause and Event-Sequence Structure

Status: `PASS`

R23 advances the bounded Rust Language Cortex along three structural gaps left by R22: relative-clause attachment, quantifier scope inside relative noun phrases, and cross-turn reference to an ordered event sequence. It also closes an authority flaw in which a Korean action embedded inside a relative clause could be projected as a second executable goal. This is an inspectable typed-IR extension, not a claim that open-ended language understanding or GPT-class generation is complete.

## What changed

The semantic role graph now carries `RelativeClauseAttachmentIR`. A relative predicate is linked to a typed head node, dependent nodes, an optional embedded event node, negation, and source evidence. Korean and English relative noun phrases therefore preserve the head, embedded relation, and independently scoped quantifiers instead of flattening the whole phrase into one string.

Actions embedded in a relative clause are marked `RelativeClause` and `Descriptive`. They cannot become viable goals or gain external execution authority. The outer requested action remains the only authorized goal. Korean temporal adnominals such as `수리한 뒤` are distinguished from noun-modifying relative clauses, so ordered action chains are not accidentally suppressed.

Conversation memory now records a distinct normalized clause for each member of a multi-event plan. A later Korean or English reference to the first, second, third, fourth, or last action selects exactly one event from the latest event batch. Cross-language selection is realized from the shared typed goal. An ordinal outside the recorded range fails closed with a natural clarification.

For a single event, memory continues to retain the original semantic clause. This preserves established plural, former/latter, repetition, realization, and event-ontology contracts. For multiple events, per-event memory clauses are unquoted and non-authoritative; user-facing realization remains separate from stored event identity.

## Frozen blind evidence

- Diagnostic source hash: `5c98b34a87af9f3d085eae3d9de1b32cff679fc57a55966416b48248894ebd4d`
- Initial diagnostic baseline: `3/24`
- Initial category results: relative attachment `0/4`, nested quantifier scope `1/4`, cross-turn event ordinal `0/4`, ordinal range failure `0/4`, relative-action authority `0/4`, grounded relative realization `2/4`
- Final diagnostic: `24/24`
- Held-out transfer source hash: `7be8f1b428f974ccf974e5e40f65769e837f8b08c2a61648b40550cc20527dfd`
- First semantic execution of held-out transfer: `16/16`
- Fresh R23 tasks: `40/40`
- Cumulative R1-R23 tasks: `907/907`

The held-out suite remained byte-identical from its pre-product freeze through first execution. It tested unseen Korean and English nouns, cross-language event selection, relative-action authority attacks, and out-of-range event ordinals.

## Regression finding and repair

The first full post-R23 canary pass exposed eight failing binaries out of 37. The cause was one boundary error: event memory stored a quoted user-facing realization for every goal. That representation discarded parts of a single compound entity mention and changed the surface expected by repetition and event-ontology resolution.

The repair separated memory representation from user-facing realization. Single goals retain their original semantic clause, while multi-goal plans receive one unquoted normalized clause per event. No older test or oracle was changed. All eight failed canaries then passed, followed by a clean `37/37` full rerun.

## Verification

- Canary binaries: `37/37`
- Adapter unit tests: `278/278`
- Workspace tests: `802/802`
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Canonical manifest: pass, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: pass
- Diagnostic and held-out source hashes after all product changes: unchanged
- External LLM, local teacher, network, canonical-language Python, and recursive source-mutation calls: `0`
- Unsupported explanation facts: `0`

The workspace test command exercised pre-existing optional Python-environment probes. Python supplied no language inference, plan, event binding, or answer.

## Relation to the six language goals

R23 directly advances grammatical composition through typed relative attachment and nested scope. It advances discourse/topic state and deixis/ellipsis through an ordered cross-turn event batch. It preserves plan-versus-result separation by making stored references non-authoritative, and it advances grounded realization by deriving relative descriptions from typed attachments.

Speech-act and pragmatic inference remain regression-verified but were not broadly expanded in this run. None of the six axes is declared generally complete. Remaining gaps include multiple nested relative clauses, open-ended predicate morphology, event/result chains with richer temporal and causal relations, distant or implicit event reference beyond bounded ordinals, and generative fluency over open vocabulary.

No commit or push was performed. The worktree remains intentionally dirty with the uncommitted R13-R23 increments and the pre-existing user change in `growth_supervisor.rs`.
