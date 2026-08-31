# Benchmark-shaped capability canary R3

- Status: `PASS_CONTROLLED_CANARY_OFFICIAL_BENCHMARK_UNMEASURED`
- Long-horizon trace: 1200 files indexed, depth 160, 161 files selected, 0 rescans
- Validation impact: 1 changed file selected 161 affected files; 1/2 prior proofs reused and 1 invalidated
- Validation safety: 1 structural change escalated to full workspace; 2 replay validations passed
- Long requirements: 91 clauses; 2 implicit constraints; 1 conflicts; 1 ambiguous references rejected
- Source synthesis: 8/8 tasks across 3 languages passed natively; 32 examples executed
- TypeScript compiler: `Version 7.0.2`
- TypeScript compiler boundary: 4 source strict typecheck passes; 1 type-error execution rejection; 1 API-migration strict typecheck pass
- Advanced TypeScript: 1 async/Promise pass; 1 nested-sequence composition pass
- Sequence mechanism transfer: 3 languages
- Compiler-guided TypeScript repair: 2/2 candidates bound and 2 verified; 1 unsupported-diagnostic abstention
- API migration: 3/3 language migrations passed natively; 3 compatibility shims
- Environment diagnosis: 8/8 failure families
- Nondeterminism diagnosis: 5/5 cause families
- External LLM calls: 0
- Network reads: 0
- Official benchmark harness executed: false
- Official score claimed: false

This controlled canary closes the named engineering gaps at the tested boundary. It is not an official SWE-bench/DeepSWE score.
