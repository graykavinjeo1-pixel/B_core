# GPT-reference final decision

Status: `FAIL`

Disposition: `FINAL_RUN_INVARIANT_PANIC`

The frozen final input and three independent `gpt-5.6-sol` reference runs were sealed before B_Core was executed. The first and only B_Core final run stopped at `GPTREF-FINAL-C10-EN-01-T1` (`Um, could ya chek the Knoll service for me?`).

The compositional parser selected a grammatical candidate, but neither the pragmatic-intent graph nor contextual continuation selected an intent. That made the active `GrammaticalCompositionToPragmaticIntent` cross-axis link unsatisfied and violated `ActiveCrossAxisLinksCoherent`. A debug assertion at `crates/semantic-core-adapters/src/cognitive.rs:2906` aborted the runner before a response batch or similarity report could be emitted.

This is a concrete blind failure in noisy/typo-tolerant intent grounding, not a low surface-similarity result. Per the frozen one-shot rule, no post-result repair or final rerun was performed. A repair and a new final campaign require explicit authorization.
