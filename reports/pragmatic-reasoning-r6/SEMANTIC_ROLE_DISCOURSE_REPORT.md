# B_Core Pragmatic Reasoning R6

Status: **PASS**
GPT-grade status: **NOT YET**

R6 adds an inspectable semantic-role and discourse-reference layer to the pure-Rust Language Cortex. Korean and English utterances can now produce typed event/entity nodes, role edges, quantifier scopes, event relations, and bounded event/result/proposition references before the existing GoalIR boundary.

This is a verified capability increment, not a claim of GPT-level general language understanding.

## Material capability increase

- Korean case particles and English active/passive/prepositional constructions produce explicit agent, theme/patient, recipient, source, destination, instrument, location, result, and comparison roles.
- Controlled universal, distributive, existential, negative, and cardinal quantifiers are represented as scope objects rather than flattened into noun strings.
- Coordination, condition, cause, purpose, contrast, temporal order, and prior-result relations are represented between event frames.
- Unknown noun phrases remain local discourse entities and do not receive invented canonical concept IDs.
- Cross-turn references can address a prior event, result, or proposition, not only a prior object.
- Reintroduced prior commands are quoted and metalinguistically wrapped. Referring to an event or its result therefore does not silently authorize that command again.
- Competing same-kind proposition referents fail closed and request clarification.
- Validated role and quantifier facts are projected into GoalIR constraints while promoted semantic payloads remain unchanged.

The implemented path is:

```text
Korean / English utterance
  -> bounded compositional frames
  -> semantic role, quantifier, and event-relation graph
  -> pragmatic authority and ambiguity checks
  -> GoalIR constraints
  -> existing semantic reasoner

prior event / result / proposition
  -> typed, bounded discourse referent
  -> quoted metalinguistic reconstruction
  -> same non-authorizing analysis path
```

## Evidence

| Check | Result |
|---|---:|
| Adapter tests | 154 passed, 0 failed |
| Workspace tests | 615 passed, 0 failed |
| R1 pragmatic canary | 8/8 |
| R2 context canary | 5/5 |
| R3 compositional canary | 20/20 |
| R4 discourse-program canary | 18/18 |
| R5 cross-turn discourse canary | 20/20 |
| R6 semantic-role/discourse canary | 24/24 |
| R1-R6 frozen canaries | 95/95 |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `git diff --check` | PASS |

The R6 canary contains ten Korean role/scope/relation cases, ten English cases, and four cross-turn event/result/proposition cases. Older R1-R5 canaries were rerun after every structural repair.

Development exposed four useful counterexamples. The English noun “report” was initially mistaken for an action; negated “any” retained existential scope; an argument inside a quoted command leaked into the outer passive clause; and coordinated “file and folder” lost its conjunction during normalization. These were repaired respectively with nominal-context classification, explicit negative quantifier scope, quote-aware frame bounds, and conjunction-preserving argument normalization.

The known independent `semantic-reasoning --all-features` module-identity issue remains outside this language change. The default workspace, standard deny-warnings Clippy gate, and all 615 workspace tests pass.

## Safety boundary

Semantic-role nodes are discourse-local IR, not promoted concepts. Lexical surfaces cannot mutate semantic payloads. Unknown noun phrases do not acquire semantic authority. Result and proposition references are prohibited from carrying execution authority even if conversation state is tampered with and rehashed. Ambiguous references fail closed, and the adapter performs no external action, network request, LLM call, source mutation, or direct text-to-solution conversion.

## Honest boundary

This remains a deterministic bilingual parser, not GPT-grade language understanding. Arbitrary morphology, deeply embedded clauses, general semantic-role labeling, nested modal/negation/quantifier scope, belief attribution, broad world knowledge, bridging inference, and open-domain calibration are incomplete. A 95-case frozen regression suite demonstrates protected structural gains; it does not establish frontier-model breadth.

The next frontier is belief and attribution graphs, richer temporal/modal/logical scope, explicit goal cancellation/reordering history, broader ontology-grounded entity typing without authority mutation, transactional conversation snapshots, and a larger family-held-out adversarial dialogue suite. The long-term goal remains active.

No commit or push was performed. The worktree contains current, prior-stage, and pre-existing changes.
