# Korean state-question grammar unit

2026-09-09. Astra supervises; one Luna implements. Canonical documents verified
and unchanged. Advances observation -> understanding -> memory/query -> decision
-> expression. Product outcome: a supplied unary HADA state/experiencer lexeme
works with current-state interrogative morphology without storing questions as
facts or coercing binary action grammar into current-state meaning.

## Scope and acceptance fixed before production result

- Reuse lexical grammar classes and typed world queries. No sentence-specific
  branches, dictionary expansion, new semantic definitions or provider assistance.
- Support ROOT한가, ROOT한가요, ROOT하지 않은가, ROOT하지 않은가요, with and
  without question marks, for unary state/experiencer grammar only.
- Registered opaque/new roots must work without source edits for each root.
- Preserve original source/provenance, polarity and subject. Questions do not add
  world premises or execution authority. Existing binary, quote, conditional,
  unknown-root and incomplete-attributive boundaries remain unchanged.
- Two production rounds maximum; no package/deployment/commit/push/default-state
  change. Preserve unrelated dirty edits. Reviewed patch-only rollback.

Frozen supervisor matrix (not blind): 106 two-turn sessions, 88 intended queries
and 18 controls. Source manifest SHA256:
`338E9C06054DB20C330386C6886AF2878393FE7A9C43FD0A5895917E7D5B6AA5`.
Files: `scripts/evaluate_hada_question_matrix.ps1` and
`reports/language-cortex-completion/hada-question-2026-09-09/manifest.json`.

Baseline binary 64966A48...: 38/106 pass. All 64 newly requested ending variants
fail typed-query creation; 24 retained queries and 14 boundary controls pass.
Four embedded curiosity utterances already create an unwanted generic
INVESTIGATE plan and return clarification. Supervisor inspected the actual IR:
execution NOT_OBSERVED, external_execution_authorized=false,
external_action_execution_observed=false, semantic_authority=false.
The combined evaluator label EXECUTION_OR_PLAN_LEAK is an unwanted-plan failure
here, not evidence that external execution occurred.

Grammar-unit acceptance is all 64 new mappings plus all 38 baseline passes
retained, existing 144-case integration and retained native suites unregressed,
and Rust test/format/static checks passing. The four pre-existing indirect-intent
failures are not repaired or relabelled by this unit: if they remain, the full
106-case matrix must still be reported FAIL (102/106), even if this narrower
grammar repair passes. A separate indirect-question ownership unit would need
correct semantic expectations: an implicit information request may legitimately
become a query, whereas a quotation must not. No frozen expectation is changed
to force an aggregate PASS.

Report before/after counts, actual outputs, binary/source hashes, latency and
memory. Do not infer free-dialogue naturalness or autonomous learning from the
result. Use one Cargo job, incremental off, dev/test debug symbols off.
