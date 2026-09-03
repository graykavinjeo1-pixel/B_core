# R45 Productive Grammatical Composition Report

Status: **PASS**

R45 replaces unrecorded object copying with a typed, discourse-local
`SharedArgumentBindingIR`. Korean left-shared arguments propagate forward;
English right-shared arguments propagate backward. Two- and three-predicate
chains reuse one entity node, preserve quantifier scope, and retain the typed
clause relation and surface evidence that licensed the binding.

## Frozen evaluation

- Diagnostic preimplementation baseline: **0/28**
- Diagnostic first product execution: **17/28**
- Diagnostic final: **28/28**
- Held-out transfer first exposure: **19/20**
- Held-out transfer final: **20/20**
- Oracle corrections: **0**
- Diagnostic output SHA-256:
  `85B4EF7F2BC666EB3C3D05D6FF60E62B0192226643B38C3572BEB54D2CB54F10`
- Transfer output SHA-256:
  `7B826BF04EC85654E4D8683318CE39F0D445ACAA91E9BE0EC66464AF09849830`

The transfer suite remained unexecuted until the diagnostic suite passed. Its
single first-exposure failure exposed `열어보고`: the embedded `보고` was
mistaken for COMMUNICATE. The product now recognizes the Korean try-auxiliary
boundary and retains OPEN/EXECUTE. No expected result was changed.

## Structural boundary

Each shared binding records provider event, dependent event, entity node,
semantic role, forward/backward direction, typed clause relation, evidence,
and confidence. Validation requires both events to point to the same entity.
It also requires:

- `syntactically_licensed = true`
- `semantic_authority = false`
- `external_execution_authorized = false`

Explicitly different objects are never overwritten. Condition, cause, and
purpose relations do not license sharing. A true temporal prior result remains
a `PriorResult`; only the old empty-object fallback on a simple sequence may be
replaced. Quoted and negated clauses may retain their grammatical structure but
cannot acquire execution authority.

Supporting morphology repairs keep `원인만` as an explicit focused argument,
do not split `말했지만` as the particle `만`, treat `-지만` as a noun-phrase
boundary, and type a comma as coordination only when it is pure punctuation.
Therefore `, so` remains causal.

## Regression evidence

- Fresh R45 tasks: **48/48**
- R1-R45 frozen cases: **1,915/1,915**
- Aggregate adapter cases including seven direct special cases:
  **1,922/1,922**
- Cargo-metadata discovered canaries: **85/85**
- Metadata-discovered JSON rows: **1,917/1,917**
- Adapter library tests: **382/382**
- Workspace library tests: **905/905**
- Workspace binary tests: **1/1**
- `cargo test --workspace`: **PASS**
- `cargo fmt --all -- --check`: **PASS**
- Clippy with denied warnings: **PASS** using the bounded historical harness
  allowances recorded in the JSON report
- New R45 unit invariants: **11**
- Canonical manifest: **PASS**, 10 files, self-hash
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`

## Portable package

The product-only `pakage` directory is synchronized as
`B_CORE_PORTABLE_PRODUCT_CORE_R45_WORKTREE_ABI1`:

- Adapter product sources: **43/43**, hash mismatches 0
- Dockable core sources: **20/20**, hash mismatches 0
- R45 research canaries included: **0**
- Package workspace tests: **409/409**
- Minimal runtime canaries: **4/4**
- Package fmt and Clippy: **PASS**
- Network/LLM Cargo dependency hits: **0**

The package workspace still contains only `dockable-semantic-core` and
`semantic-core-adapters`. Semantic-reasoning research and recursive mutation
machinery remain outside the product boundary.

## Safety and cleanup

External LLM calls, local teacher calls, network calls, Python calls in the R45
language path, and recursive source mutations are all **0**. The shared
binding cannot become semantic or execution authority. Sparse runtime checks
retain `FULL_CATALOG_SCANS=0` and `ROUTING_FALSE_NEGATIVES=0`.

After validation, root build cache cleanup removed 25,744 files
(33,037,310,754 bytes), and package cleanup removed 5,434 files
(3,508,328,825 bytes). Both `target` directories are absent.

R45 is complete. The broader GPT-level objective is not complete and no such
equivalence is claimed. The next engineering stage is
`R46_DISCOURSE_TOPIC_AND_DEIXIS_ELLIPSIS_INTEGRATION`.
