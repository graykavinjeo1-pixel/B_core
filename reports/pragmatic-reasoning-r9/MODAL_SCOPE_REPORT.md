# Pragmatic Reasoning R9 — Modal, Conditional, and Counterfactual Scope

Status: **PASS for the R9 increment; GPT-grade general language understanding is NOT YET achieved.**

Date: 2026-08-31

## Outcome

R9 replaces the previous sentence-level `Possible`/`Necessary` flags with an inspectable, compositional modal scope graph. English and Korean utterances can now preserve nested possibility, probability, certainty, obligation, permission, prohibition, desire, intention, ability, prediction, conditional, counterfactual, and operator-versus-proposition negation distinctions.

The graph is connected to pragmatic goal projection and the R8 epistemic ledger. A possible, desired, predicted, hypothetical, or counterfactual action is not converted into a current action. A conditional directive does not become executable while its antecedent is unestablished. Direct polite requests are recognized as requests, but the language adapter itself performs no external action. Modal propositions remain dialogue-local records and never become world truth.

## Architecture

- Implementation language: Rust
- Modal scope graph: `B_CORE_MODAL_SCOPE_GRAPH_IR_1`
- Compositional analysis: `B_CORE_COMPOSITIONAL_ANALYSIS_IR_4`
- Epistemic ledger: `B_CORE_EPISTEMIC_LEDGER_IR_2`
- Conversation state: `B_CORE_CONVERSATION_STATE_6`
- External LLM calls: 0
- Network calls: 0
- Runtime source mutation: 0
- Semantic payload mutation: 0
- Direct text-to-solution shortcuts: 0

The typed path is:

```text
surface clauses
  -> closed-class modal/conditional operators
  -> scoped open proposition
  -> possible-world and negation-scope assignment
  -> pragmatic goal/authority projection
  -> world-indexed epistemic signature
  -> bounded conversation state
```

## Implemented distinctions

- epistemic possibility, probability, and certainty;
- deontic obligation, permission, and prohibition;
- desire, intention, ability, and prediction;
- nested English prefix and Korean suffix modal scope;
- proposition negation (`must not`) versus operator negation (`does not have to`);
- indicative, hypothetical, counterfactual, and `unless` conditionals;
- no converse inference from a conditional;
- unsatisfied conditional guards cannot authorize current execution;
- English and Korean indirect polite requests;
- ambiguous `may`, `should`, and non-request `could` readings remain explicit;
- `ACTUAL` and `EPISTEMIC_POSSIBLE` propositions do not falsely contradict each other;
- same-world, cross-source opposite propositions remain contested;
- conversation-state validation binds a discourse referent's modal world to its ledger record;
- truth, authority, reverse-inference, and modal-scope-cycle tampering fail validation.

## Development counterexamples that changed the implementation

1. Korean `…하면 안 된다` was initially parsed as an ordinary conditional. It is now recognized as a deontic prohibition before conditional segmentation.
2. Korean `…해 줄 수 있어?` projected a polite request but lacked an explicit ability operator. The missing compositional marker family was added.
3. A Korean antecedent before the action (`테스트가 통과하면 … 배포해`) let the imperative candidate escape as an immediate goal. Global conditional scope now gates candidate projection regardless of predicate position.
4. Passive English `should be explained` triggered the active `should` ambiguity gate and regressed the frozen R3 suite. Passive scope is now distinguished from the active advice/expectation ambiguity.
5. `You do not have to delete …` could retain a positive obligation-shaped goal. Operator-negated obligation, permission, and prohibition now block positive goal projection.
6. A rehashed conversation state could otherwise diverge a proposition referent's modal world from its belief signature. Validation now checks all modal, polarity, source, attitude, status, and surface bindings.

## Validation

- `cargo fmt --all -- --check`: PASS
- `cargo clippy -p semantic-core-adapters --all-targets -- -D warnings`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo test --workspace --no-fail-fast --quiet`: 649 passed, 0 failed
- `semantic-core-adapters` library tests: 188 passed, 0 failed
- R9 modal-scope canary: 55 passed, 0 failed
  - English surface family: 18
  - Korean surface family: 18
  - pragmatic projection family: 13
  - safety and cross-turn family: 6
- frozen R1–R9 language canaries: 214 passed, 0 failed
- `git diff --check`: PASS

The first R9 run passed 49/52 and exposed three real Korean composition defects. After repair and additional adversarial cases, the frozen R9 suite passed 55/55. A frozen R3 regression was also found and repaired before final validation.

The workspace tests emit expected `ModuleNotFoundError: pytest` traces in existing negative dependency-probe cases; all such tests pass and Python remains absent from the canonical Rust language path.

## Safety invariants

- Modal content does not establish dialogue truth.
- A modal operator never carries execution authority.
- A conditional relation starts with `condition_satisfied=false`.
- Reverse conditional inference is never authorized.
- Possible and actual claims inhabit distinct ledger worlds.
- Cross-source conflicts are preserved rather than silently ranked.
- A polite request may project explicit user-request authority, but the adapter does not execute it.
- Attributed, quoted, hypothetical, counterfactual, desired, predicted, and negated content remains non-authoritative.
- All modal and epistemic conversation structures are bounded and covered by state validation/hash checks.

## Honest limits

- The parser uses a bounded closed-class operator lexicon and structural rules, not a general syntactic parser.
- Modal accessibility relations, quantified modal logic, and calibrated probabilities are not represented.
- `may`, `should`, `could`, offers, permissions, and indirect requests still need richer actor and discourse context.
- Conditional antecedents are represented but are not yet evaluated against verified events to activate a guarded branch.
- Tense, aspect, recurring events, event time versus report time, and modal time interaction remain coarse.
- Factive and implicative verbs, presupposition projection, generalized neg-raising, and attitude de re/de dicto distinctions are incomplete.
- Long-distance actor coreference and multi-sentence modal embedding remain limited.
- No broad open-domain corpus, human calibration study, or frontier-model comparison has been run.
- The system remains below GPT-grade breadth, robustness, knowledge, and calibration.

## Next frontier

The next highest-value increment is modality- and attribution-aware question answering with factivity/presupposition control: answering “who believes what,” “what is merely possible,” “what actually follows,” and “what remains unknown” without collapsing reports, assumptions, desires, or counterfactuals into facts. Exact event-time relations and guarded-condition activation should follow within that work.

## Repository state

- Branch: `main`
- HEAD before an R9 commit: `603eb2a`
- Ahead of upstream: 4
- Behind upstream: 0
- Commit created: false
- Push performed: false
- Worktree clean: false; prior R1–R8 and unrelated user work remain present and were preserved.
