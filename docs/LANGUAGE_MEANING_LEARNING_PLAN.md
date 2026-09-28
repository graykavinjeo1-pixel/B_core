# Native language: meaning preservation and learning plan

Status: bounded native-path and supplied-lexical campaign completed with known
limitations on 2026-09-09. Earlier checkpoints below are historical, not current
status. See the final checkpoint and integrated report; general natural
conversation remains incomplete.
Owner: Astra supervises/reviews; one Luna agent implements and tests.
Authority: the immutable North Star and canonical documents remain unchanged.

## Product outcome and scope

Advance the North Star's observation -> understanding -> memory -> reasoning ->
decision -> expression path: a native conversation must retain what the user
said, distinguish assertions from questions/hypotheses/reports, and use those
source-bound meanings to answer or request missing information. This is a
bounded engineering and supervised language-knowledge campaign, not a claim of
reproducing a biological brain or attaining GPT-level understanding.

The current blocker is incomplete semantic grounding, not a demonstrated lack
of compute. Existing raw proposition storage, causal/conditional bindings and
other parser routes must be checked before claiming information is lost.
Adding an unconsumed field, receipt, counter or generic acknowledgement does
not satisfy this plan.

No whole-sentence dispatch, stored target answers, external teacher/runtime
LLM, autonomous source mutation, canonical amendment, package synchronization,
deployment, commit or push is authorized by this plan. Existing unrelated
working-tree changes are preserved. The in-progress response arbitration patch
is frozen and remains unapproved pending its outstanding runtime checks.

## Five gates, in order

| Gate | Deliverable on the real native path | Exit evidence |
| --- | --- | --- |
| G0: identity and baseline | Record source/executable hashes, native entry point, state path and any provider adapter; distinguish original from copied deployments. | Reproducible inputs, outputs and intermediate meanings; unknown copy paths explicitly unverified. |
| G1: meaning preservation | Reuse the existing typed event/role machinery to preserve supported clause composition and scope; retain unsupported source as unresolved evidence without asserting its truth. | Previously lost supported relations reach memory and change a follow-up answer; independent names/arguments transfer; negation, quote and conditional controls remain safe. |
| G2: memory and judgment | Source-bound episode/proposition identity, correction lineage and one coherent state snapshot from interpretation through decision. | Follow-up retrieval, correction replacement, ambiguity and memory-ablation checks pass on the native API; plans are never presented as executed results. |
| G3: language-knowledge acquisition | Register bounded lexical/grammatical knowledge through existing validated machinery; share semantic identities between Korean/English expression forms. | A taught expression works with unseen arguments/constructions; alias changes preserve semantic payload; ungrounded definitions remain unresolved. |
| G4: integrated evaluation | Run meaning -> memory -> decision -> realization without provider assistance or answer repair loops. | Frozen structural suite, native regression suite, source/evidence fidelity, and time/memory measurements reported separately. |

The first implementation unit in G1 must be chosen from a verified runtime
loss, not inferred merely from a function returning None. Prefer composition
of already-grounded clauses over guessing the meaning of an unknown predicate.

## Meaning contract

Reuse existing IR types before introducing another pipeline/module. Preserve:

- predicate identity and participant roles, with source provenance;
- polarity and its scope (not a sentence-wide negative keyword flag);
- temporal evidence, conditions, modality, causal relation and attribution;
- correction target and surviving/retracted claims;
- reference bindings, unresolved alternatives and unsupported residual content.

Grammatical or lexical evidence is not an execution grant. Unknown vocabulary
must not acquire an executable predicate merely because its spelling occupies
a verb position. Quoted, conditional and hypothetical clauses are not current
facts. Acknowledgement is not proof of successful understanding. Raw retention
and actual semantic coverage are measured separately.

## Learning protocol (G3, after path validation)

1. Inspect existing candidate/lexical/grammar import machinery and use its
   validation and promotion boundary; do not create a competing answer store.
2. Freeze a small first batch: at most 16 lexical-expression records and four
   construction records, each with provenance, semantic identity/role signature,
   language and applicability constraints. Reduce the batch if fewer records
   have defensible semantic grounding; never pad counts with synonyms.
3. Label human/agent-authored supervision as supplied language knowledge, not
   autonomous concept emergence. Grammar records describe reusable composition,
   not complete test sentences. Dictionary ingestion alone is not learning.
4. Measure before/after on identical retained and held-out inputs. Separate
   lexical generalization (new arguments) from construction generalization
   (unseen compositions). Lexical registration success alone fails the gate.
5. Record acquisition duration, peak memory where available, storage delta,
   native turn latency, unsupported-claim rate and structural transfer. A causal
   graph's learning efficiency is an empirical question, not a premise of PASS.

## Fixed evaluation contract

Before implementation of each unit, freeze its input family, expected semantic
relations/response act and safety controls. Keep examples only in test/audit
artifacts; production may not read them. Supervisor-created cases must be marked
review cases, not independent blind data if the implementer has seen them.

Final G4 suite: 120 utterances (10 structural families x 2 languages x 6
argument/surface variations), plus 24 short memory sessions (12 per language).
Families: assertion, role question, polarity, time, condition, cause, correction,
report/quotation, reference/ellipsis, and request versus result. Each family
must contain both supported and deliberately ambiguous/unsupported controls.
Freeze exact counts of those subsets and expected responses before the run.
Split by lexical/structural combination, not random copies of sentences.

Acceptance for this bounded suite:

- 100% of supported expected relations preserved, with source evidence;
- 100% correct expected memory/response act on the declared supported subset;
- 100% controls preserve uncertainty/scope without invented facts or actions;
- zero unsupported explanation facts and zero execution from quoted/hypothetical
  content; no external/local teacher calls in the native execution path;
- no failures in retained tests; fmt, clippy and library tests pass;
- one realization validation, no iterative sentence repair loop;
- report p50/p95 latency on the same machine/profile, with no unexplained >20%
  regression against the matched baseline. Missing measurements remain unknown.

Report absolute numerators/denominators and each family's results. A supported
subset passing is not 100% general language ability. Naturalness needs a separate
human judgment and cannot be inferred from serialization/validator tests.

## Bounded iteration, release and rollback

At most two implementation/review rounds per unit. On continued failure,
preserve the evidence, name the failed boundary and propose a revised unit;
do not silently add exception sentences, expand scope, lower expectations or
declare success. Remaining gates stay NOT_STARTED/BLOCKED, not retrospectively
relabelled as completed. No promised calendar estimate precedes G0 measurement.

Test Rust with one build job and incremental compilation off. If Windows commit
pressure requires debug symbols off, record the profile; do not change stack,
safety checks, unrelated processes or global machine configuration.

All work remains local until the full integration gate and separate release
authorization. Record pre/post file hashes and schema changes. Old serialized
state must either validate explicitly or fail closed; do not silently mutate it.
Rollback is a reviewed patch of this task's changes only, never reset/clean/stash
of the dirty worktree. No package copy or running service is implicitly updated.

## Initial status

- Canonical manifest verified: 10 files; manifest SHA256
  `56363e77b86f59bf067ca3a559197092708c475993e412be5b60b05d993146bd`.
- The six original sentences from the other deployment are unavailable; their
  reported outcomes have not been reproduced against this original workspace.
- G0: IN_PROGRESS. G1-G4: NOT_STARTED.
- Earlier arbitration checks (918 library tests) are a separate, incomplete
  patch review, not evidence that these five language gates passed.

### First verified unit (supervisor native baseline)

`reports/language-cortex-completion/meaning_path_supervisor_baseline_2026-09-09.json`
records five conversations / 11 turns on the existing DAC21A5F... native binary.
It predates the frozen arbitration patch. English single-antecedent recall
already works. Korean same-turn object reference is incorrectly rewritten to
the actor before proposition storage. Multiple English object antecedents are
silently resolved by recency. This is a reference/provenance defect, not proof
that every ordinary statement fails to enter memory.

G1 unit scope: retain original assertion clauses, bind unambiguous event roles
through source-bound contextual composition, and admit earlier same-turn
sources only with strict chronological/order validation. Reject future/self/
cyclic bindings, ambiguous referents, modal/attribution leakage. No predicate
guessing, new sentence templates or arbitration expansion. The existing
same-turn rewriting path must not override stored semantic source evidence.

The initial earlier suspicion that every compound statement is lost was
rejected: existing clause extraction can store independent statements as
separate records. Implementation must address the verified reference boundary.

### Identified dependency before G3 training

`CognitiveApi::inject_compositional_predicate` validates and stores runtime
`PredicateLexemeIR` entries and supports snapshot export/import. In contrast,
`proposition_content::predicate_entries` reads only `builtin_pack()`. Teaching
the former therefore does not establish that an expression reaches attributed
event memory or follow-up QA. No G3 learning run has been conducted.

The implementation prerequisite is a shared, bounded, read-only lexical view
with explicit semantic/role identity and versioned provenance, used consistently
by ingestion and its source validator. Revalidation of an old record must not
silently use a later mutable dictionary. A planner intent hint is not sufficient
evidence of a new predicate's causal meaning or executability. Do not bypass
promotion or turn an alias into semantic authority to close this gap.

## Checkpoint: 2026-09-09, not approved

The first local reference/provenance patch improves the real native memory/QA
path in 11 supervisor diagnostic conversations (23 turns). Final source and
binary identity, outputs, commands and limitations are recorded in
`reports/language-cortex-completion/meaning_path_supervisor_review_2026-09-09.json`.
Reproduce the diagnostic inputs with PowerShell 7 and
`scripts/probe_meaning_memory_supervisor.ps1`; its successful process exit alone
is not a semantic PASS.

Final supervisor verification: fmt PASS, clippy PASS, build PASS; library tests
918/919, with `feedback_correction_preserves_contextual_explanation_goal` failing
because target identity becomes `helix parser` instead of the existing `helix`.
Luna's temporary disabled/enabled A/B passes with the new paths disabled and
fails after exact restoration. This is a newly induced interaction, not an
established pre-existing failure. The earlier agent report claiming otherwise
is rejected. Expected test results have not been changed.

Bounded implementation rounds have ended. Preserve the unapproved local patch;
do not call G1/G2 complete, start training, or release it. The revised next unit
is diagnostic isolation of the assertion-source branch versus same-turn source
admission, followed by a canonical referent identity fix if justified. Preserve
raw observations and typed identity separately; no example-specific noun
truncation. Explicit source-order tamper tests and performance/retained native
checks also remain outstanding. G3 and G4 have not started.

## Authorized continuation and frozen evaluation

The user subsequently directed continuation of this plan and an integration
briefing on completion. The prior failed checkpoint remains intact. The next
G1 unit isolates assertion-source reanalysis from same-turn source admission;
it must preserve the existing canonical referent identity without changing
test expectations or truncating specific names/nouns.

Before evaluation, the supervisor froze:

- `reports/language-cortex-completion/meaning-learning-2026-09-09/integration_manifest.json`:
  120 target utterances (60 supported, 60 scope/uncertainty controls), 138 setup
  turns counted separately, and 24 memory sessions. SHA256
  `B432F9A3B0F1B11B368A8EDDAF4EBF334E467AEDD9E62F18B5A518CC50D28517`.
- `supervised_lexical_batch.json` in the same directory: 16 Korean/English
  expression records for eight explicitly supplied boolean/ordered-relation
  contracts; no whole input/answer sentences. SHA256
  `36572D97F2483D9E8D22B37F7F37E48FB0181D55854607E8099B7E7595197BBF`.
- `scripts/evaluate_meaning_integration.ps1`: typed memory, reference, scope,
  decision/proof and execution-boundary checks, plus measured latency/memory.
  This is a supervisor structural review, not an independent blind benchmark.

Further inspection found an existing shared versioned lexical view in the
world-model path: `WorldVocabularyIR.lexical_history`, consumed by world memory,
reasoning and realization through `UPDATE_WORLD_VOCABULARY`. G3 should first
reuse and test that path rather than build a competing registry. This does not
close the separate compositional-predicate-to-ordinary-event-memory gap noted
above. Any G3 success is scoped to the registered boolean/relation vocabulary,
not general linguistic learning or autonomous semantic concept emergence.

The first batch adds zero grammar constructions and reuses five existing
language/argument grammar variants. Registration and before/after transfer
evaluation remain pending G1/G2 verification. Do not present a frozen data file
as acquired runtime knowledge, or reuse a world-path metric as whole-language
accuracy.

## Resumed checkpoint: modal-content output boundary

After the host OOM pause the user authorized resumption. The native executable
`1C2C649CCD4E450770E1B410E3E249CF6BC36789DDF7A86269F92EE72E942576`
contains the source-preserving G1 repair and the modal source-content gate.
Reporting what a user wanted or conditionally asserted no longer requires the
embedded proposition to be an actual-world event. Attributed evidence, active
record identity, grounded bindings and non-execution boundaries remain required;
multipart answers reuse this criterion. Source-order tamper tests now construct
independently source-valid content before testing future/self/cyclic rejection.

The supervisor reran retained native suites: 432/432 turns PASS (continuity
98/98, formerly 93/98). Luna verified 924/924 library tests and the native build;
the supervisor separately verified Clippy and reviewed the source/test changes.
The new negative test's initial query-enum assumption was incorrect; it now
asserts semantic abstention without requiring that internal enum. No production
parser or frozen expectation was changed to force the enum.

The unchanged integration manifest still yields 133/144. Eight Korean
correction/retraction checks and three Korean hypothetical-question checks
remain failures. Therefore G2's complete memory/judgment scope and G4 remain
open, and G3 registration is still NOT_STARTED. The next bounded unit unifies
existing correction-prefix interpretation and repairs typed hypothetical-query
ownership; it must preserve supported world counterfactual reasoning and must
not convert internal errors into generic acknowledgements. No release,
deployment, package synchronization, commit, push or schema migration occurred.

## Structural path checkpoint (G3 follows separately)

The bounded correction/hypothetical unit ended after two production rounds.
Shared correction-prefix interpretation now reaches both proposition and world
memory; correction retracts the matching premise and invalidates dependent
answers. Non-actual outcome questions retain their hypothetical scope while
activating a consistent typed question and pragmatic information-request path.
No frozen integration input or expected semantic result was changed.

Supervisor verification on native binary
`64966A48CB944FABA6BECE1DF151CADE90DD12EC3D1980960A775DC275D55AD5`:
927/927 library tests, Clippy with warnings denied, fmt, diff whitespace and
canonical integrity PASS. All 11 retained suites pass (432/432 turns). The
unchanged structural integration suite improves from 133/144 to 144/144 over
342 turns. Its p50/p95 is 32.50/42.45 ms, versus matched baseline 35.16/56.02 ms;
these are debug native JSON round trips, not a production latency SLA.

The separate source-memory probe retains five supported projections and six
abstention controls (23 turns); nine hypothetical-boundary probes also pass.
These are diagnostic/structural reviews, not blind naturalness evaluations.
Unresolved hypothetical output can still expose awkward topic-word lists, and
low-confidence voice clarification is not established. General free conversation
is not complete. G3's eight supplied contracts/16 aliases are now authorized for
explicit isolated-session registration and transfer testing. The frozen batch
remains unchanged; acquiring it is not autonomous concept discovery.

## Final bounded campaign checkpoint

G0-G2 pass for the declared local native/source-preserving path, G3 passes for
the supplied boolean/relation contracts on supported grammar, and G4 passes
the frozen structural suite. This is not a general-language or naturalness PASS.
Final evidence is in
`reports/language-cortex-completion/meaning-learning-2026-09-09/integrated_final_report.json`
and `INTEGRATION_BRIEFING.md` alongside it.

The supervisor independently reproduced 16 exact before/after premise-query
pairs (eight supplied predicate IDs in Korean and English): no target-contract
answer before registration, 16/16 supported answers with exact participant roles
after explicit registration. Separate 39-turn checks cover new arguments,
conditional composition, polarity, role reversal, alias lifecycle and cold
process replay. The fresh-batch Rust ablation proves direct reasoning remains
available without current aliases and fails when the semantic contract is
removed while aliases remain. Native label-without-registration is not itself
claimed as causal semantic ablation. The durable 4,378-byte replay artifact
contains contracts and lexical roots/grammar, not complete input/answer pairs.

`bora는 분주한가?` remains an observed native morphology failure; the supported
`분주하나요?` form did not fix it. The failed surface and awkward output are
preserved in `lexical_matched_transfer_final.json`, outside the supported-form
pass numerator. These provided labels add no full real-world definitions or
autonomous semantic discoveries. They are explicitly replayed per conversation;
neither the default vocabulary nor a deployed service was changed.

Final supervisor checks: 928/928 library tests, Clippy all-targets with warnings
denied, fmt, diff whitespace and canonical verification PASS. All-targets Clippy
initially found one test-only `get().is_none()` lint; the equivalent `contains_key`
absence assertion fixes it without changing the expectation. Runtime executable
remains `64966A48CB944FABA6BECE1DF151CADE90DD12EC3D1980960A775DC275D55AD5`.
No release, commit, push, package sync, source self-mutation or default-state
migration occurred. The next language work needs a separately bounded grammar
and realization/general-event coverage unit, not more claim inflation from the
current structural pass rate.
