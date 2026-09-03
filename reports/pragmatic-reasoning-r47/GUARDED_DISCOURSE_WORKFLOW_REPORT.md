# R47 Guarded Cross-Turn Discourse Workflow Report

Status: **PASS**

R47 extends the typed cross-turn discourse program with hashed conditional
guards. A workflow such as “캐시를 검사하고 캐시에 문제가 있으면 수리해”
can now be rebound by a later explicit target such as “인덱스도 같은 절차로
해”. Only `Investigate(index)` becomes the current active goal;
`Repair(index)` remains `CONDITION_PENDING` until trusted evidence establishes
the guard. The rebound utterance returns through the ordinary deterministic
grammar, compositional semantics, and GoalIR path.

## Frozen evaluation

- Diagnostic preimplementation baseline: **0/12**
- Diagnostic progression: **0/12 → 4/12 → 12/12**
- Diagnostic final: **12/12**
- Held-out transfer first exposure: **5/8**
- Held-out transfer final: **8/8**
- Fresh R47 tasks: **20/20**
- Oracle corrections: **0**
- Diagnostic JSON SHA-256:
  `20727604F644663E9F3E622D99A544228291204ACB4EF55A46A94C0243C7FB72`
- Transfer JSON SHA-256:
  `33A1E80574E5E482ED5F55E6E092CB1BBCDB184711703FA47CB40CE96616DC3D`

The first held-out exposure found three product gaps: Korean `고쳐` was not
recognized as a conditional repair imperative, English `it` did not inherit
the preceding shared target, and a fresh “same guarded procedure” ellipsis was
not linked to the typed guarded program. The product parser and binder were
repaired without changing the frozen suite or expected answers.

## Structural boundary

`B_CORE_CONVERSATION_STATE_23` stores
`B_CORE_DISCOURSE_PROGRAM_IR_2` programs whose guarded steps contain
`B_CORE_DISCOURSE_PROGRAM_GUARD_IR_1`. A guard records its typed conditional
kind, antecedent surface and normalized form, antecedent hash, canonical
condition predicate, source subject, negation, evidence requirement, and both
authority flags.

The guard and its negation participate in the program integrity hash. A
guarded action has neither semantic nor execution authority, and it cannot
become active merely because the condition is mentioned. It requires verified
evidence. Quoted, counterfactual, targetless, missing-program, unresolved-guard,
and mixed-target cases fail closed. Korean/English rebinding, cross-language
relexicalization, pronominal or zero-argument binding, and open compound targets
use typed structure rather than whole-sentence solution dispatch.

## Regression evidence

- R47 diagnostic and held-out transfer: **20/20**
- R46 diagnostic and transfer: **20/20**
- Deferred lifecycle diagnostic and transfer: **16/16**
- Modal-scope tests: **55/55**
- Conditional-guard tests: **56/56**
- Adapter library tests: **393/393**
- Root workspace substantive tests: **917/917**
- Root `cargo test --workspace`: **PASS**
- Root fmt and all-target Clippy with warnings denied: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`

The workspace test run emitted two expected missing-`pytest` probe messages,
but their owning tests and the complete workspace run passed with exit status
zero. They are optional-environment probes, not R47 language-path failures.

## Portable package

The product-only `pakage` directory is synchronized as
`B_CORE_PORTABLE_PRODUCT_CORE_R47_WORKTREE_ABI1`:

- Adapter product sources: **43/43**, hash mismatches 0
- Dockable core files: **20/20**, hash mismatches 0
- R47 research canaries included: **0**
- Package workspace unit tests: **420/420**
- Minimal runtime canaries: **4/4**
- Package fmt and all-target Clippy: **PASS**
- Default language runtime: **Rust only**

An inherited truncated base-commit string in the package manifest and README
was corrected to the actual 40-character HEAD
`cb8b2debad3a0e23d5597a29db9c24af3c3c3c4f`. This is a traceability repair;
it does not change runtime behavior.

## Safety, cleanup, and boundary

External LLM calls, local teacher calls, network calls, Python calls in the R47
language path, and recursive source mutations are all **0**. Sparse runtime
checks retain `FULL_CATALOG_SCANS=0` and `ROUTING_FALSE_NEGATIVES=0`. The
pre-existing user edit in `growth_supervisor.rs` remains unchanged.

After validation, root cleanup removed 7,921 files (6,753,706,567 bytes), and
package cleanup removed 4,124 files (2,484,561,767 bytes). Both `target`
directories are absent.

R47 is complete. The broader language objective is not complete, and no
unrestricted GPT-level equivalence is claimed. Assuming each subsequent stage
succeeds, three stages remain: R48, R49, and R50. R50 is the final integration,
whole-system regression, and sealing stage. No commit or push was performed.
