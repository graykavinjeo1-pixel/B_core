# GPT Reference V4 independent authoring contract

Status: `WAITING_FOR_INDEPENDENT_INPUT_AUTHOR_AND_THREE_REFERENCE_RUNS`

The purpose of V4 is to make the completion decision reproducible. The B_Core candidate is not allowed to influence benchmark prompts, semantic labels, or GPT reference surfaces.

## Isolation boundary

The independent input author receives the completion rule and data schemas, but no B_Core outputs, V1/V2/V3 reference responses, repair canaries, residual-error reports, or source-level heuristics. The author creates exactly 40 dialogues and 160 response points: two languages, ten categories, four turns per dialogue, with the distribution required by `BenchmarkInputSuiteIR::validate`.

The input author must provide:

- a raw `B_CORE_GPT_REFERENCE_V4_FINAL` input suite with `frozen=false` and an empty `suite_payload_sha256`;
- a matching semantic annotation draft with `frozen=false`, an empty `suite_payload_sha256`, and no calibrated surfaces;
- an author manifest using schema `B_CORE_INDEPENDENT_INPUT_AUTHOR_MANIFEST_1`;
- an explicit assertion that no B_Core output or candidate artifact was consulted.

The author manifest binds the exact raw file bytes with SHA-256. Its own hash is SHA-256 over the compact JSON serialization of the manifest after `manifest_sha256` is cleared. The authoring run ID must identify an isolated context.

## Mechanical gates

`gpt-reference-independent-input-sealer` performs all of these checks before B_Core is run:

- validates the frozen-suite shape, category balance, language balance, dialogue order, and ID namespace;
- binds the annotation draft to the sealed input hash;
- pins the exact development, V1, V2, and V3 comparison suite IDs and hashes;
- rejects exact prompt, response-ID, and dialogue-ID reuse;
- rejects repeated structural skeletons, including resource/entity-only renames;
- requires nearest-prior-prompt similarity mean at most 6000 bp, p95 at most 8500 bp, and maximum below 10000 bp;
- accepts no candidate-output path and records `b_core_evaluations=0`;
- writes only new files under `reports/` and refuses replacement.

The intended invocation is:

```text
cargo run -q -p gpt-reference-evaluator --bin gpt-reference-independent-input-sealer -- \
  <raw-input.json> <raw-annotation-draft.json> <author-manifest.json> \
  reports/language-cortex-completion/gpt-reference-v1/development_input.json \
  reports/language-cortex-completion/gpt-reference-v1/final_input.json \
  reports/language-cortex-completion/gpt-reference-v2/final_input.json \
  reports/language-cortex-completion/gpt-reference-v3/final_input.json \
  reports/language-cortex-completion/gpt-reference-v4/final_input.json \
  reports/language-cortex-completion/gpt-reference-v4/annotation_draft.json \
  reports/language-cortex-completion/gpt-reference-v4/final_input_audit.json
```

## GPT surface references

After input sealing, three isolated GPT generation runs receive only the sealed input plus the same system prompt and generation configuration. They must not receive B_Core output. Run IDs must be distinct. Each raw run is sealed with `gpt-reference-surface-run-sealer`; `gpt-reference-final-sealer` accepts exactly three configuration-matched runs and refuses to overwrite the final reference.

Only after the input and all three references are sealed may B_Core execute once. No repair or rerun is allowed on V4. The resulting evaluator report is the next official completion decision under `GPT_SIMILARITY_STOP_RULE.md`.

## Stop behavior

- `PASS`: all stop-rule gates pass in the single sealed run; language-cortex completion work stops.
- `FAIL`: preserve the run and exact failed gates; do not tune on V4 and do not silently extend the campaign.

The last official result remains V3 at 4803 bp until V4 is independently authored and sealed.
