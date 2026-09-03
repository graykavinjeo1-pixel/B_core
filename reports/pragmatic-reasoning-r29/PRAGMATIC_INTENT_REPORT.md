# R29 Pragmatic Intent Inference

Status: **PASS**

R29 adds a bounded Korean/English speech-intent layer above the typed clause, discourse-focus, and deixis mechanisms from R26-R28. It does not claim general pragmatics or GPT-level language understanding.

## Result

The frozen 32-case diagnostic improved from **0/32** to **32/32**. It covers eight classes with four cases each: conventional indirect requests, preference requests, advisory suggestions, rhetorical evaluations, information questions, self offers, metalinguistic mentions, and active-goal corrections.

The separately frozen 16-case transfer suite scored **16/16 on its first semantic execution**. Its oracle did not change and it triggered no product repair. Together these are **48/48 fresh R29 tasks** and **1155/1155 cumulative R1-R29 tasks**.

After held-out evaluation, the full historical canary run initially scored **47/49**. The failures were earlier noun-only argument corrections, not R29 held-out cases. The repair distinguishes an action correction from an argument replacement and prevents an English noun such as `the report` from being promoted to a communication action. A second boundary defect was found in `No, inspect it instead of deleting it`: the sentence-initial `No` had crossed the comma and incorrectly acted as a determiner for `inspect`. Determiner scope now stops at the clause boundary. The final cumulative run is **49/49**.

## Added semantic mechanisms

- `PragmaticIntentGraphIR` represents pragmatic force, goal projection, subject binding, confidence, and evidence independently of semantic authority.
- Request-shaped forms can project a candidate goal. Advice remains advisory. Rhetorical, informational, self-directed, and metalinguistic forms do not become execution goals.
- Goal correction requires one unambiguous active goal and a different explicit action predicate. It inherits the typed active subject and fails closed when the binding is absent or competing.
- The question router distinguishes genuine information questions from question-shaped requests, suggestions, preferences, and rhetorical evaluations.
- Korean and English action morphology is shared across constructions; no whole-sentence solution dispatch was added.
- Every typed pragmatic graph sets `semantic_authority=false` and `external_action_execution_authorized=false`. The language layer proposes interpretation; it does not certify truth or claim an external action happened.

## Relation to the six language capabilities

1. **Grammatical composition:** R26 remains green for its bounded typed clause graph. Unrestricted syntax and attachment remain open.
2. **Discourse/topic state:** R27 remains green for clause-aware typed focus. General multi-speaker discourse remains open.
3. **Deixis/ellipsis:** R28 remains green for the tested Korean/English focus-binding cases. Bridging and plural reference remain open.
4. **Speech intent/pragmatics:** R29 now passes the eight tested intent families. General implicature, irony, social relations, multi-party commitments, and unrestricted vocabulary remain open.
5. **Plan/result distinction:** this is the next bounded stage. Existing authority boundaries remain green, but arbitrary tool observations and claimed outcomes are not yet comprehensively separated.
6. **Evidence-grounded realization:** prior realization checks remain green. Open-domain claim-level provenance is still open and follows item 5.

This means the first four roadmap items now have bounded, regression-tested implementations; it does **not** mean that any of them are solved at unrestricted GPT scope. The next stage should address execution result versus plan, followed by evidence-grounded sentence realization.

## Verification

- R29 diagnostic: **0/32 baseline**, **32/32 final**
- R29 held-out transfer: **16/16 first execution**, **16/16 final**
- All adapter canary binaries: **47/49 initial cumulative run**, **49/49 final**
- Adapter unit tests: **291/291**
- Workspace library tests: **814/814**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments`: **PASS**
- Canonical manifest: **PASS**, 10 files, self-hash `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`
- `git diff --check`: **PASS**
- Frozen hashes: diagnostic `45a79914296a22266b7c73971bdcea3abc69d765d53f273f0d7e2bd4b87d9719`; transfer `4400021a184197842ff0ad5c1ad1ae5ae829a05a9b313fd0a4da91383a535415`; both unchanged
- Temporary R29 debug markers: **0**
- External LLM, local teacher, network, Python language-path, external action, and recursive source-mutation calls: **0**

The one explicit `clippy` exception applies only to the already-frozen diagnostic harness's table-shaped constructor. Product code was refactored to meet the normal argument limit. Editing the harness after first execution would have invalidated its blind hash, so all other warnings remain denied while that single harness lint is allowed. Cargo's hardlink messages are filesystem cache warnings rather than Rust lint failures.

No commit or push was performed. The pre-existing user change in `crates/semantic-reasoning/src/growth_supervisor.rs` was preserved and not edited by R29.

## Remaining boundary

R29 does not establish unrestricted intent recognition. Indirect complaints, sarcasm, socially conditioned politeness, multi-party commitments, and truly open-vocabulary pragmatic inference still need broader mechanisms and fresh tests. Those limits must not be hidden behind the PASS label.
