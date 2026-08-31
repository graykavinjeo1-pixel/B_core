# Pragmatic Reasoning R10 — Evidence-Bounded Discourse Q&A

Status: **PASS**
Long-term target: GPT-grade general language understanding
Current assessment: **NOT YET GPT-grade**

## Outcome

R10 adds a pure-Rust question-answering boundary over the existing attribution, modality, and epistemic-revision state. It can answer a controlled family of English and Korean dialogue questions about who said or believed what, whether a record is possible/predicted/counterfactual rather than actual, whether sources conflict, and whether a question is trying to smuggle in an unverified premise.

The answer path is:

```text
normalized question
  -> DiscourseQueryIR
  -> bounded epistemic-ledger filtering
  -> cited evidence + typed answer claims
  -> Korean/English realization
```

It does not route through the planner. A recognized question creates no new goal, plan hash, task frame, belief, truth claim, or execution authority.

## What changed

- Added `DiscourseQueryIR` for source content, proposition sources, actuality, modality, conflict, and presupposition checks.
- Added `DiscourseAnswerIR` with bounded evidence and claim-to-belief-record citations.
- Added bilingual source and attitude questions covering saying, reporting, believing, thinking, knowing, wanting, and expecting.
- Kept `knows that P` as `PRESENTED_AS_KNOWN`; it does not turn `P` into dialogue truth.
- Added explicit actual-world abstention: source reports can be returned, but the adapter cannot claim they are facts.
- Added modal realization for possibility, prediction, hypothetical, and counterfactual records.
- Added current-versus-historical retrieval over superseded and corrected records.
- Added cross-source conflict answers without source ranking.
- Added unverified-presupposition rejection for why/when/how and factive-style questions.
- Fixed bounded syntactic uses of English `it` and `that` without weakening general ambiguous-reference rejection.
- Fixed actor prefix capture (`Ann` no longer answers for `Annabelle`).
- Projected `want` and `expect` complements into desired and predicted worlds.
- Extended local state opposition for valid/invalid, ready/not-ready, and 정상/비정상.

## Safety boundary

Every answer validates these invariants:

- no answer or evidence row may claim dialogue-grounded truth;
- no answer or evidence row may authorize external execution;
- every evidence-bearing claim must cite an evidence belief ID;
- unknown sources and missing propositions are not fabricated;
- conflicting reports remain conflicting;
- modal classification never implies actuality;
- an unverified question premise is not silently accepted;
- unsupported free-form explanation claims remain zero.

## Validation

| Check | Result |
|---|---:|
| Adapter unit/integration tests | 203 passed, 0 failed |
| Workspace tests | 664 passed, 0 failed |
| R10 discourse-Q&A canary | 55 passed, 0 failed |
| Frozen R1–R10 canaries | 269 passed, 0 failed |
| `cargo fmt --all --check` | PASS |
| Adapter clippy, all targets, warnings denied | PASS |
| Workspace clippy, all targets, warnings denied | PASS |
| External LLM calls | 0 |
| Local teacher calls | 0 |
| Network calls | 0 |

The R10 canary contains 32 English questions, 16 Korean questions, and seven non-linguistic answer-tamper checks. Its families cover source/attitude, actuality/modality, conflict/revision, presupposition/abstention, unknown-source handling, actor-boundary attacks, and IR tampering.

Development counterexample progression was 42/55, 50/55, 54/55, then 55/55 after repairing the underlying semantics. Tests were not weakened to hide failures.

## Honest capability assessment

This is a meaningful discourse-reasoning increment, but it is not GPT-grade general language understanding. The remaining gap is large: the query grammar is bounded, topic retrieval is still mostly lexical, temporal/event semantics are coarse, long-distance coreference is limited, multi-hop questions are incomplete, and there has been no broad corpus or human evaluation.

The next high-value increment is a typed event and temporal-relation graph, followed by verified conditional-guard activation, long-distance entity/event coreference, and compositional paraphrase matching with explicit proof paths.

## Repository state

- Branch: `main`
- HEAD: `603eb2a`
- Relative to `origin/main`: ahead 4, behind 0
- Commit created in R10: no
- Push performed: no
- Worktree clean: no; prior uncommitted R1–R9 and unrelated user changes were preserved

Machine-readable details: [discourse_qa_report.json](./discourse_qa_report.json)
