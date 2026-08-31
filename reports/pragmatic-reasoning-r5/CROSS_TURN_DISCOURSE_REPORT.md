# B_Core Pragmatic Reasoning R5

Status: **PASS**
GPT-grade status: **NOT YET**

R5 adds an inspectable cross-turn discourse state. The language cortex can now distinguish a reference to a prior object from an ellipsis that inherits a prior action, reconstruct a complete semantic request, and feed that request through the same compositional parser and GoalIR path as a fully stated utterance.

## Material capability increase

- Singular, plural, former, and latter references bind to typed dynamic referents.
- A Korean referent can be named through its English alias on the next turn, and vice versa, using one canonical concept ID.
- “문서도” after “파일을 확인해” becomes “문서를 확인해” through an explicit inherited-goal binding.
- “same for report” reuses the prior typed action rather than matching a memorized full sentence.
- “그거 말고 폴더로” / “not that, folder instead” revises the argument of one active goal.
- “그대로 해” / “do the same” repeats one unambiguous active goal, while the same phrase after a multi-goal program requires clarification.
- Goal ellipsis expires after a bounded turn distance, preventing old execution authority from being silently revived.
- Filler and typo normalization now preserves sentence and quotation boundaries for downstream scope reasoning.

The implemented path is:

```text
noisy current turn
  -> punctuation-preserving semantic surface
  -> typed referent / inherited-goal binding
  -> complete resolved semantic request
  -> compositional frames and scopes
  -> pragmatic goal or goal graph
  -> GoalIR
  -> existing semantic reasoner
```

Conversation aliases and active goals remain adapter state. They do not mutate promoted semantic concepts, and the adapter performs no external action.

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 144 passed, 0 failed |
| Workspace tests | 605 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| R4 discourse-program canary | 18/18 |
| R5 cross-turn discourse canary | 20/20 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R5 canary covers twenty Korean/English multi-turn cases: singular and plural pronouns, former/latter selection, parallel ellipsis, same-goal repetition, argument correction, multi-goal ambiguity, noisy speech normalization, punctuation, and two cross-language bindings.

Development exposed four important counterexamples. Punctuation loss collapsed clause structure; `열어` passed only through the older keyword path and never entered typed goal memory; inherited `고쳐` was over-inflected to `고쳐해`; and unbounded ellipsis could revive stale authority. Each was repaired at its structural boundary and all older canaries were rerun.

The known independent `semantic-reasoning --all-features` module-identity issue remains recorded from R4. The default workspace, standard deny-warnings Clippy gate, and all 605 tests pass.

## Honest boundary

This is still not GPT-grade language understanding. It has a useful symbolic circuit for a bounded class of multi-turn references and action ellipses, but it does not yet have broad entity knowledge, arbitrary semantic roles, event/proposition reference, nested belief state, general ellipsis, deep discourse coherence, or large-scale calibration.

The next frontier is typed dependency and semantic-role structure, event and proposition referents, explicit goal-revision history, logical temporal/modal/quantifier scope, durable rollback-capable conversation snapshots, and a much larger family-held-out dialogue benchmark. The long-term goal remains active; R5 is a verified increment, not completion.

No commit or push was performed. The worktree still contains current, prior-stage, and pre-existing changes.
