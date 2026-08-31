# B_Core Pragmatic Reasoning R3

Status: **PASS**
GPT-grade status: **NOT YET**

R3 replaces sentence-wide action cue selection with an inspectable compositional interpretation lattice. Predicate mentions are represented as frames, their scope is recorded, losing interpretations remain visible, and only the winning structurally authoritative reading is projected into GoalIR.

## Material capability increase

- “서비스를 수정하지 말고 장애 원인만 설명해줘” selects `EXPLAIN`; `REPAIR` remains present but blocked by negation.
- A destructive command inside quotation is represented but cannot authorize execution.
- A manager's reported command remains reported speech rather than becoming the user's command.
- Counterfactual actions remain hypothetical; conditional questions become investigations.
- `X 말고 Y` / `not X but Y` preserves the rejected alternative and selects the corrected target.
- Two equally supported conflicting requests produce clarification instead of arbitrary routing.
- A definition-grounded new predicate such as `다듬` can be attached to `C_REFINE_DOCUMENT` and immediately reuses the same negation, quotation, modality, and authority rules.

The implemented path is:

```text
surface language
  -> predicate frames + semantic scopes
  -> competing interpretation candidates
  -> selected pragmatic goal
  -> GoalIR
  -> existing semantic reasoner
```

No external LLM, network inference, direct text-to-solution path, or semantic payload mutation was introduced.

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 127 passed, 0 failed |
| Workspace tests | 588 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R3 canary contains ten Korean and ten English cases across negation, quotation, reported speech, counterfactuals, conditional questions, target corrections, passive metalinguistic clauses, and opaque targets. During development it first exposed four rule-level counterexamples; the scope and authority rules were repaired, then the full old and new suites were rerun.

Rust printed hard-link fallback notices because the I: filesystem cannot hard-link incremental-cache artifacts. Some repository capability tests also deliberately probed an unavailable `pytest` module and handled the absence; the workspace test command still completed successfully.

## Honest boundary

This stage is not GPT-grade natural-language understanding. It adds a reusable symbolic structure that removes several severe false-intent errors, but it does not contain the pretrained linguistic/world knowledge, broad morphology, long-distance syntax, cultural implicature coverage, or large-scale calibration of a frontier language model.

The next capability frontier is persistent learned grammar/predicate memory, nested dependency and semantic-role graphs, multi-goal ordering, quantifier/temporal/coreference scope, and a much larger family-held-out adversarial suite. The long-term goal remains active; R3 is one verified increment, not a declaration of completion.

No commit or push was performed. The worktree still contains current, prior-stage, and pre-existing changes.
