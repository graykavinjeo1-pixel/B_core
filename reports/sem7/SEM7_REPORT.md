# SEM-7 Language Cortex Adapter Report

Status: `PASS` — `LANGUAGE_CORTEX_ATTACHED_AND_SEMANTIC_BOUNDARY_VERIFIED`

The failed frozen `SEM7-RUN-0001` is preserved under `reports/sem7/failed_runs/SEM7-RUN-0001/`. Its repair was limited to Korean Language-to-GoalIR morphology and negation scope; the passing results below are from fresh frozen `SEM7-RUN-0002`.

The bounded deterministic adapter compiled 100 frozen Korean/English requests into GoalIR. The semantic reasoner received no raw language strings. Direct GoalIR and language-derived GoalIR agreed on every hidden execution.

The regression includes 20 Korean grounding tasks, 20 English grounding tasks, 20 language-to-program tasks, 20 language-to-math tasks, and 10 definition-only foraging replays. Program tasks passed 80 offline Rust-Min checks through ProgramIR. Math tasks produced typed derivation certificates; language strings were never accepted as proofs.

Lexical aliases are held in a separate store. Korean and English share 6 semantic concepts. Alias attachment, rename, second-language attachment, removal, unnamed operation, opaque relexicalization, language ablation, and semantic ablation all passed without semantic payload mutation. Unsupported explanation facts, lexical-token-dependent promoted concepts, LLM calls, teacher calls, recursive source mutations, full-catalog scans, and routing false negatives were all zero.

All 13 gates passed. SEM-8 was not started. The next allowed stage is `SEM-8_CROSS_DOMAIN_STRUCTURAL_MECHANISM_TRANSFER`.
