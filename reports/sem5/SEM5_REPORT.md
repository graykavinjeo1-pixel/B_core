# SEM-5 Programming First-Principles Expansion

Run `SEM5-RUN-0002` is **PASS**: `PROGRAMMING_FIRST_PRINCIPLES_EXPANSION_VERIFIED`. The frozen evaluator used 130 fresh blind tasks, including 20 definition-only opaque-API tasks and 20 adversarial programs. Expected outputs, evaluator families, and reference source were absent from solver-visible manifests.

## Execution evidence

- ProgramIR valid rate: `1.000000`
- offline Rust-Min compile rate: `1.000000`
- bounded runtime-valid rate: `1.000000`
- hidden property pass rate: `1.000000`
- definition-only zero-shot rate: `1.000000`
- containment violations: `0`

ProgramIR remained authoritative; Rust source was generated only as a deterministic execution adapter. All canonical programs were source-audited, compiled with the local `rustc` and no external crates, run in isolated temporary directories, and deleted after capture. Windows does not expose a portable standard-library-only address-space limiter, so timeout, output, path, process, dependency, and filesystem containment were enforced while the memory-limit field records that platform limitation.

## Controlled comparison

- primitive A solve rate: `0.423077`
- structural B solve rate: `0.692308`
- semantic no-promotion C solve rate: `0.769231`
- full first-principles D solve rate: `1.000000`
- D-minus-C solve delta: `0.230769`
- D search-cost reduction versus C: `36.769231`

All conditions used the same frozen tasks and expansion budget. Outcomes arise from typed IR construction and resource-bounded search; no opened blind task was rewritten.

## Autonomous concepts

3 candidates were proposed from recurring IR dependency structures and 3 passed semantic consistency, compression, calibration, fresh reuse, cross-instance, language-separation, lineage, and causal-ablation gates. Generation-3 concepts depend on immutable Generation-2 ancestors; the Generation-4 concept recombines the promoted Generation-3 abstractions. Human interpretations were attached only after sealing.

Best concept: `C000010` — type-compatible staged composition abstraction. Compression: `43` primitive-expanded nodes to `6` operational nodes (`7.166667x`). Cross-domain transfers: `3`; predecessor-concept reuse: `90`.

## Gates

- `AUTONOMOUS_PROGRAM_ABSTRACTION_PASS`: PASS
- `BASIC_PROGRAM_SYNTHESIS_PASS`: PASS
- `CAUSAL_UTILITY_PASS`: PASS
- `CROSS_INSTANCE_REUSE_PASS`: PASS
- `DEFINITION_ONLY_API_PASS`: PASS
- `FRESH_GENERALIZATION_PASS`: PASS
- `LANGUAGE_SEPARATION_PASS`: PASS
- `NO_CONTAMINATION_PASS`: PASS
- `PROGRAM_CONCEPT_PROMOTION_PASS`: PASS
- `REAL_RUST_EXECUTION_PASS`: PASS
- `SPARSE_OPERATION_PASS`: PASS
- `TARGET_PROGRAM_LEAKAGE_AUDIT_PASS`: PASS

## Quarantine and next stage

Network, external LLM, local teacher, recursive mutation, full-catalog scan, and routing-false-negative counts are zero. Recursive improvement remains observe/measure-only. SEM-6 was not started; the next allowed stage is `SEM-6_DEFINITION_ONLY_KNOWLEDGE_FORAGING`.
