# B_Core Pragmatic Reasoning R1

Status: `PASS`

This revision adds a reusable discourse/pragmatic reasoning circuit rather than a handler for one sentence. The implementation is deterministic Rust and makes no LLM or network calls.

## Reasoning path

```text
utterance
  -> conservative normalization
  -> discourse clauses
  -> clause roles and relations
  -> speech act
  -> inferred goal / decision policy
  -> authority-aware planning projection
  -> existing semantic planner
```

The clause graph separates observations, causes, contrasts, conditions, current actions, problems, knowledge gaps, proposals, costs, benefits, evaluations, decisions, negation, and uncertainty. The resulting IR is included in both standalone natural-language and conversational API responses.

## User utterance

The supplied utterance now produces:

```text
speech_act       = CONDITIONAL_CONTINUATION
current_task     = 통합
required_benefit = 커버리지를 확장하는 효과가 있다
planner_intent   = INVESTIGATE

SUPPORTED  -> CONTINUE_CURRENT_WORK
REJECTED   -> REPORT_NEGATIVE_AND_ASK_WHETHER_TO_STOP
UNRESOLVED -> REPORT_UNCERTAINTY_AND_ASK_HOW_TO_PROCEED
```

The former `실제 -> 실행` fuzzy-normalization corruption is blocked. The paragraph itself is no longer used as an opaque executable subject. Proxy score evidence is separated from direct outcome evidence, so a high score alone cannot satisfy the continuation gate.

## Generalization checks

The same mechanism is exercised with:

- a Korean refactoring/cost/failure-reduction paraphrase;
- an English migration/cost/failure-reduction paraphrase;
- an unpunctuated speech-style transcript;
- an unacceptable problem state implying a repair goal;
- curiosity implying investigation rather than repair;
- a desirable feature interpreted as a suggestion rather than authorization;
- a causal fact that must not authorize continuation;
- an explicit negative override that blocks positive continuation inference.

## Authority boundary

Indirect intent may select a planning goal, but it does not independently grant external mutation authority. A conditional continuation claim is first projected as an evidence-gathering `INVESTIGATE` goal. Negative and unresolved results retain the user's stop/continue decision.

## Verification

- semantic-core-adapters: `101 passed, 0 failed`
- workspace: `562 passed, 0 failed`
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- pragmatic reasoning canary: `8/8` pass

## Limits

This is a materially stronger, inspectable reasoning circuit, but not GPT-scale unrestricted language understanding. Novel metaphors, sarcasm, culturally implicit reference, and very long-context ellipsis still require broader language knowledge and discourse memory. Those limits are reported rather than hidden behind a benchmark score.
