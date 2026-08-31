# Pragmatic Reasoning R12 — Evidence-Bounded Conditional Guards

Status: **PASS**
Long-term target: GPT-grade general language understanding
Current assessment: **NOT YET GPT-grade**

## Outcome

R12 adds a pure-Rust conditional-guard layer over the existing modal graph and epistemic ledger. A conditional is stored as an inactive rule. Later active, actual-world dialogue records can support or contradict its antecedent, but support only makes the consequent available for deliberation.

```text
conditional utterance
  -> typed inactive guard
  -> active Actual-world dialogue evidence
  -> UNRESOLVED | SUPPORTED | CONTRADICTED | CONTESTED
  -> deliberation eligibility only
```

It does not create a current action, infer the antecedent from the consequent, establish external truth, or authorize execution.

## What changed

- Added persistent `ConditionalGuardStoreIR` and validated `ConditionalGuardEvaluationIR`.
- Kept the original modal conditional immutable and unsatisfied; runtime evidence is tracked separately.
- Added explicit unresolved, supported, contradicted, contested, and counterfactual-ineligible states.
- Matched only active `Actual`-world ledger records by typed subject, state axis/value, and compatible temporal anchor.
- Added correct negated-antecedent behavior for English `unless` and Korean negative conditional forms.
- Excluded possible, probable, predicted, desired, hypothetical, and counterfactual records from actual guard support.
- Preserved cross-source conflict and followed same-source correction/supersession.
- Prohibited consequent-to-antecedent reverse inference structurally.
- Blocked hypothetical Korean consequents from leaking into active goals.
- Added bounded sentence-local binding for `If the backup is available, restore it`.
- Added Korean `-이면` recognition and repaired `Actually, Alice ...` actor extraction.
- Added bilingual status realization that states the no-auto-execution boundary.

## Safety boundary

- `SUPPORTED` means supported by dialogue records, not verified world truth.
- A supported consequent is only available for deliberation.
- No guard, evidence row, or evaluation can authorize execution.
- Conditional declarations and activations create no automatic `GoalIR`.
- Observing the consequent never supports the antecedent.
- Non-actual modal records never satisfy an actual guard.
- Counterfactuals never activate current consequents.
- Conflicting sources remain contested.
- Superseded and retracted records are excluded.
- Unsupported realization claims remain zero.

## Validation

| Check | Result |
|---|---:|
| Adapter unit/integration tests | 224 passed, 0 failed |
| Workspace tests | 685 passed, 0 failed |
| R12 conditional-guard canary | 56 passed, 0 failed |
| Frozen R1–R12 canaries | 381 passed, 0 failed |
| `cargo fmt --all --check` | PASS |
| Adapter clippy, all targets, warnings denied | PASS |
| Workspace clippy, all targets, warnings denied | PASS |
| External LLM calls | 0 |
| Local teacher calls | 0 |
| Network calls | 0 |

The R12 canary contains 8 English conditional surfaces, 8 Korean surfaces, 14 evidence-evaluation sequences, 8 modal/reverse-inference attacks, 6 conflict/revision sequences, and 12 direct tamper attacks.

The initial frozen run was 49/56. It exposed sentence-local `it` binding, Korean hypothetical-goal leakage, missing `-이면`, and correction-actor identity defects. The underlying boundaries were repaired, and the unchanged semantic expectations then passed 56/56. All R1–R11 canaries were rerun afterward.

## Honest capability assessment

This closes a meaningful reasoning gap but remains far below GPT-grade general understanding. Guard matching still relies on a bounded state lexicon and one-condition signatures. General conjunction/disjunction, quantified and numeric conditions, rich temporal guards, broad paraphrase entailment, source reliability, and long-distance mixed-language coreference remain incomplete. Dialogue evidence is also deliberately not external verification.

The next high-value increment is long-distance typed coreference across actors, entities, events, and belief holders, followed by ontology-mediated paraphrase matching and causal/concessive discourse relations with explicit evidence paths.

## Repository state

- Branch: `main`
- Pre-commit HEAD: `603eb2a`
- Relative to `origin/main`: ahead 4, behind 0
- This report was sealed before the requested commit and push.
- The final commit hash and push result are recorded in the task response, avoiding a self-referential commit hash in this report.
- Worktree clean at report seal: no; the accumulated verified R1-R12, SWE, Synapse-adoption, and portable-package snapshot was awaiting commit.

Machine-readable details: [conditional_guard_report.json](./conditional_guard_report.json)
