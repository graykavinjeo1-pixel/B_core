# SEMANTIC REASONING PROJECT — SEM-31

## PERSISTENT SEMANTIC WORLD STATE

## LANGUAGE-FREE ENTITY / PROPERTY / RELATION / STATE GROUNDING

Continue ONLY the independent B_Core / Semantic Reasoning Project lineage.

Start from the exact sealed SEM-30 predecessor:

```text
SEALED_PREDECESSOR_COMMIT=
e7b3539a89e4870fd7461bc9ca6d65fbc93abd9c
```

Verify exact predecessor integrity.

Do NOT push unless explicitly authorized.

---

# 0. SCIENTIFIC PURPOSE

SEM-30 established causally verified reversible compiled semantic memory.

SEM-31 begins the persistent world-model line.

The central question is:

> Can B_Core maintain a persistent representation of a changing world using semantic entities, shared properties, typed relations, states, observations, and residual updates — without natural language becoming canonical memory?

This is NOT yet:

* vision;
* audio;
* robotics;
* open-world internet knowledge;
* full physical simulation.

First establish the internal semantics of persistent world state.

---

# 1. WORLD MODEL CONSTITUTION

The canonical world representation must follow:

```text
meaning ≠ language
meaning ≠ node ID

meaning =
semantic structure
+ typed relations
+ topology
+ transformations
+ causal/evidential context
```

Required:

```text
NODE_ID_IS_SEMANTIC_PAYLOAD=false

NATURAL_LANGUAGE_IS_CANONICAL_WORLD_MEMORY=false
NATURAL_LANGUAGE_IS_WORLD_REASONING_AUTHORITY=false
```

---

# 2. THIN ENTITY REPRESENTATION

Concrete entities must remain thin bindings.

Conceptually:

```text
Entity
=
identity
+ schema/type bindings
+ distinctive property edges
+ current state
+ unique relations
+ exceptions/residuals
```

Do NOT duplicate all inherited/shared semantics into every entity.

---

# 3. SHARED PROPERTY NODES

When multiple entities possess the same semantic property:

use one shared property representation where semantically appropriate.

Example:

```text
E1 ─COLOR→ BROWN
E2 ─COLOR→ BROWN
E3 ─COLOR→ BROWN
```

Do NOT store three independent copies of the meaning `BROWN`.

Required measurement:

```text
DUPLICATED_SHARED_SEMANTIC_PAYLOAD_EVENTS
```

Target:

```text
DUPLICATED_SHARED_SEMANTIC_PAYLOAD_EVENTS=0
```

---

# 4. NEW PROPERTY DISCOVERY

When an already-known entity gains a newly observed property:

default update should be equivalent to:

```text
existing entity
+
new semantic edge
```

not:

```text
duplicate/rewrite entire entity record
```

Track:

```text
INCREMENTAL_ENTITY_UPDATE_EVENTS
FULL_ENTITY_REWRITE_EVENTS
```

Canonical semantic updates should avoid full rewrites unless causally necessary.

---

# 5. MISSING PROPERTY SEMANTIC

If a newly observed property has no existing semantic node:

do NOT immediately create a new primitive.

Perform:

```text
new observation
      ↓
existing semantic representation available?
      │
   YES ─→ reuse
      │
      NO
      ↓
composition of existing semantics sufficient?
      │
   YES ─→ construct composition
      │
      NO
      ↓
irreducible new semantic property candidate
      ↓
verify
      ↓
promote if justified
```

Track separately:

```text
EXISTING_PROPERTY_REUSE_EVENTS
PROPERTY_COMPOSITION_EVENTS
NEW_PROPERTY_PRIMITIVE_GENESIS_EVENTS
```

---

# 6. COMPOSITION BEFORE PRIMITIVE GENESIS

Prevent semantic vocabulary explosion.

Forbidden pattern:

```text
SMALL_RED_ROUND_OBJECT
```

becoming a primitive merely because:

```text
SMALL
RED
ROUND
```

co-occur.

If the composition later becomes repeatedly useful/predictive, SEM-30 compressed-memory machinery may promote it as a motif/schema.

---

# 7. PERSISTENT VS TRANSIENT

Distinguish persistent properties from transient states.

Examples:

```text
material
structural type
stable identity
```

may be persistent.

Examples:

```text
temperature now
position now
door currently open
velocity now
```

must be represented as state or temporally scoped bindings.

Required:

```text
PERSISTENT_PROPERTY_TRANSIENT_STATE_CONFUSION_EVENTS=0
```

---

# 8. EVIDENCE STATUS MUST BE EXPLICIT

The world model must distinguish at least equivalent epistemic roles:

```text
OBSERVED
INFERRED
PREDICTED
HYPOTHESIZED
```

Do not silently convert perception/inference/prediction into fact.

Each nontrivial world-state assertion must preserve enough provenance to determine how it became believed.

---

# 9. UNCERTAINTY

Allow uncertainty/confidence where evidence does not justify categorical truth.

Do NOT require probabilistic neural representations.

Use whatever compact semantic/evidential representation fits B_Core.

Required:

```text
UNCERTAIN_ASSERTIONS_COLLAPSED_TO_CERTAIN=0
```

where uncertainty was part of the canonical test fixture.

---

# 10. IDENTITY PERSISTENCE

The same world entity observed at different times must remain the same semantic entity when evidence supports identity continuity.

Do NOT create:

```text
entity_t0
entity_t1
entity_t2
```

as unrelated entities merely because observations occurred at different times.

Track:

```text
IDENTITY_CONTINUITY_EVENTS
FALSE_ENTITY_DUPLICATION_EVENTS
FALSE_ENTITY_MERGE_EVENTS
```

---

# 11. RELATIONS ARE FIRST-CLASS

World meaning must support typed relationships such as abstract equivalents of:

```text
PART_OF
CONNECTED_TO
SUPPORTED_BY
CONTAINS
NEAR
CAUSES
DEPENDS_ON
LOCATED_AT
```

Do NOT hard-code this exact vocabulary as the only possible relations.

Relations may themselves carry:

```text
context
time
confidence
direction
provenance
```

where necessary.

---

# 12. RELATION CHANGE

If a relation changes:

record the semantic delta.

Example:

```text
E1 ─SUPPORTED_BY→ E2
```

becomes false after an event.

Do not duplicate the whole world snapshot.

---

# 13. WORLD HISTORY COMPRESSION

Forbidden canonical representation:

```text
WorldSnapshot_1
WorldSnapshot_2
WorldSnapshot_3
WorldSnapshot_4
...
```

with full independent copies.

Prefer equivalent structure:

```text
initial / anchor state
+
meaningful deltas
+
persistent dynamics
+
unexpected residuals
```

Track:

```text
FULL_WORLD_SNAPSHOT_COPIES
WORLD_DELTA_EVENTS
```

Required:

```text
FULL_WORLD_SNAPSHOT_COPIES=0
```

for canonical history storage, aside from explicitly declared checkpoint/serialization infrastructure.

---

# 14. HISTORY RECONSTRUCTION

Even with compressed history, it must be possible to reconstruct mechanically required past states for test cases.

Required:

```text
HISTORICAL_STATE_RECONSTRUCTION_PASS=true
```

---

# 15. RESIDUAL-FIRST LEARNING

For each new observation:

attempt to explain it using current semantics.

Conceptually:

```text
Observation
     ↓
Current World Model prediction/explanation
     ↓
explained?
   /       \
 yes        no
  ↓          ↓
minimal      residual
update       ↓
        semantic investigation
```

Store primarily what existing knowledge cannot explain.

Track:

```text
EXPLAINED_OBSERVATION_EVENTS
IRREDUCIBLE_RESIDUAL_EVENTS
```

---

# 16. DO NOT OVERSTORE OBSERVATIONS

A repeated observation that provides no new semantic information should not require a full new semantic record.

It may update:

```text
confidence
frequency evidence
temporal confirmation
```

if useful.

But semantic payload duplication must remain bounded.

---

# 17. REPEATED WORLD STRUCTURE MAY COMPRESS

Reuse SEM-30 compiled semantic memory.

If a repeated world-state pattern or transition structure proves:

```text
reusable
transferable
predictive
causally useful
compression-positive
```

it may become a compressed semantic node.

Do NOT force any promotion for SEM-31 PASS.

---

# 18. COMPRESSED NODE REMAINS DECOMPOSABLE

Any SEM-31 world-memory compression must preserve:

```text
underlying semantic DAG
applicability
provenance
exceptions
verification evidence
```

and retain decompression.

---

# 19. CLOSED-WORLD FIXTURE

Use a frozen, fully local, language-free synthetic world fixture.

Its canonical input should be typed structured events, not natural-language descriptions.

Example abstract domains may involve:

```text
objects
containers
supports
locations
movements
state changes
property observations
relation changes
```

Do NOT manually encode the expected semantic graph as the solver answer.

The fixture defines world events and independently checkable truth.

---

# 20. WORLD FIXTURE MUST SUPPORT NOVEL DISCOVERY

Include situations where:

```text
known entity gains known property

known entity gains compositional property

known entity exposes genuinely new irreducible property

relation appears

relation disappears

transient state changes

identity persists across observation gaps

observation is uncertain

inference differs from observation

repeated experience contains no new semantic novelty
```

---

# 21. INDEPENDENT WORLD VERIFIER

The world-state generator is not success authority.

Required:

```text
WORLD_GENERATOR_IS_SUCCESS_AUTHORITY=false
```

The verifier must mechanically check:

```text
entity identity
property correctness
relation correctness
state correctness
history reconstruction
epistemic status
semantic duplication
residual accounting
```

Freeze verifier semantics before canonical fresh world episodes.

---

# 22. NO GOLD WORLD GRAPH LEAKAGE

Canonical B_Core path must not access the verifier's hidden expected semantic graph.

Required:

```text
WORLD_GOLD_GRAPH_READS=0
EXPECTED_WORLD_STATE_LOOKUPS=0
FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=0
```

---

# 23. ACTIVE WORLD GRAPH MUST REMAIN SPARSE

Persistent world knowledge may grow.

Current reasoning must not scan it all.

Track:

```text
TOTAL_WORLD_ENTITIES
TOTAL_WORLD_SEMANTIC_NODES
TOTAL_WORLD_RELATIONS

ACTIVE_ENTITIES_P50
ACTIVE_ENTITIES_P95

ACTIVE_SEMANTIC_NODES_P50
ACTIVE_SEMANTIC_NODES_P95

WORLD_MEMORY_FULL_SCANS
```

Required:

```text
WORLD_MEMORY_FULL_SCANS=0
```

---

# 24. WORLD SIZE SCALING CANARY

Run the same local reasoning operation at increasing persistent world sizes.

Example observation points:

```text
10^2
10^3
10^4
10^5
```

if feasible within the existing hardware/time envelope.

Do NOT require all sizes if mechanically inappropriate.

The target is:

> current reasoning cost should track relevant active semantic field much more closely than total persistent world size.

---

# 25. STORAGE SCALING CANARY

Measure two separately:

```text
RAW_EXPERIENCE_COUNT
SEMANTIC_MEMORY_BYTES
```

and:

```text
NEW_SEMANTIC_BYTES_PER_EXPERIENCE
```

SEM-31 does NOT require asymptotic proof.

It should determine whether repeated already-explained experiences consume substantially less new semantic storage than genuinely novel ones.

---

# 26. CRITICAL STORAGE TEST

Create two equal-length episode groups.

### Group A — Redundant experience

Mostly already explained by existing semantic structures.

### Group B — Novel experience

Contains genuine new semantic distinctions.

Required expected phenomenon:

```text
NEW_SEMANTIC_BYTES_PER_EVENT(Group A)
<
NEW_SEMANTIC_BYTES_PER_EVENT(Group B)
```

with the difference attributable to semantic reuse, not dropped required information.

---

# 27. ENTITY STORAGE TEST

When introducing many entities sharing one schema/properties:

measure whether storage behaves approximately like:

```text
shared semantics once
+
thin bindings per entity
```

rather than:

```text
full semantic description × entity count
```

---

# 28. INSTANCE EXCEPTIONS

If an entity differs from its schema:

store the exception/residual.

Do not fork an entire schema unless evidence requires a new class/schema.

Track:

```text
INSTANCE_EXCEPTION_EVENTS
UNNECESSARY_SCHEMA_FORK_EVENTS
```

Target:

```text
UNNECESSARY_SCHEMA_FORK_EVENTS=0
```

---

# 29. MEMORY CONSISTENCY

Contradictory observations must not silently coexist as equally authoritative fact.

The system may:

```text
retain uncertainty
preserve conflicting evidence
revise belief
mark temporal change
request/infer missing context
```

depending on semantics.

Track:

```text
UNRESOLVED_SILENT_WORLD_CONTRADICTIONS
```

Required:

```text
UNRESOLVED_SILENT_WORLD_CONTRADICTIONS=0
```

---

# 30. NO LANGUAGE SHORTCUT

Do NOT serialize canonical meaning as human-language descriptions and parse them back as memory.

Natural-language reports are allowed only outside canonical reasoning/storage.

Required:

```text
WORLD_MEMORY_NATURAL_LANGUAGE_BYTES_ON_HOT_PATH=0
```

where technically measurable.

---

# 31. NO WORLD ONTOLOGY DUMP

Do not manually provide an enormous predefined ontology.

SEM-31 should begin with the minimum substrate necessary for world-state representation and allow semantic structures to be reused/generated through existing B_Core mechanisms.

---

# 32. NO FULL WORLD MODEL CLAIM

SEM-31 success means:

> persistent semantic world state exists.

It does NOT yet establish:

```text
general physical world understanding
human-level common sense
visual grounding
scientific discovery
robot embodiment
```

Do not make those claims.

---

# 33. PRIMARY EVENT-BOUNDED SUCCESS PATH

A strong canonical sequence is:

```text
fresh world starts
↓
entities grounded
↓
shared properties reused
↓
state changes represented as deltas
↓
known entity gains new property by edge addition
↓
missing semantic property is composed or generated
↓
identity persists over time
↓
observed/inferred/predicted states remain distinct
↓
history reconstructs
↓
repeated familiar events add little semantic payload
↓
novel residual causes meaningful semantic growth
↓
sparse reasoning remains bounded
```

---

# 34. HARD CEILING

Use:

```text
MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096
```

as containment only.

Stop early when all required persistent-world-state evidence is complete.

Budget must not influence semantic decisions.

---

# 35. CHECKPOINTING

Checkpoint every 64 autonomous research epochs and at:

```text
first persistent entity
first identity continuation
first property-edge update
first new semantic-property genesis
first state delta
first uncertainty-preserving update
first residual-driven semantic growth
first compressed world-memory promotion, if any
```

---

# 36. REQUIRED CAUSAL ABLATIONS

At minimum:

### A — Shared semantic reuse ablation

Force duplicated per-entity property representation under equal semantic requirements.

Confirm memory cost increases without capability benefit.

### B — Residual-learning ablation

Store every observation as fully novel.

Confirm semantic memory growth worsens.

### C — Sparse world-memory routing ablation

Use a bounded comparison demonstrating that avoiding full scans causes the claimed scaling benefit.

Do not compromise canonical architecture merely for the ablation.

---

# 37. REQUIRED LEVELS

## Level A — Persistent Entity Identity

Entities persist correctly across time.

## Level B — Semantic Property/Relation Grounding

Shared properties and typed relations represent world meaning without language authority.

## Level C — Incremental State Update

New information changes only necessary semantic bindings/deltas.

## Level D — Epistemic/Temporal Integrity

Persistent properties, transient state, observations, inference, prediction, hypothesis, and uncertainty remain distinguishable where required.

## Level E — Residual Semantic Learning

Familiar experience is largely reused while irreducible novelty causes semantic growth.

## Level F — Sparse Persistent World Memory

World memory grows while local reasoning remains sparse with zero full scans.

## Level G — Causal Storage Advantage

Shared-semantics and residual-learning ablations establish the storage/computation benefits causally.

Core SEM-31 PASS requires A–G.

---

# 38. PRIMARY RAW OUTPUTS

Return at minimum:

```text
SEM31_STATUS=PASS|FAIL
DISPOSITION=

CAMPAIGN_ID=

BRANCH=
COMMIT=
WORKTREE_CLEAN=
PUSH_PERFORMED=

SEALED_PREDECESSOR_COMMIT=
PREDECESSOR_INTEGRITY=

PERSISTENT_WORLD_STATE_PRESENT=

WORLD_ENTITIES_TOTAL=
WORLD_PROPERTY_NODES_TOTAL=
WORLD_RELATION_TYPES_TOTAL=
WORLD_RELATIONS_TOTAL=
WORLD_STATE_EVENTS_TOTAL=

IDENTITY_CONTINUITY_EVENTS=
FALSE_ENTITY_DUPLICATION_EVENTS=
FALSE_ENTITY_MERGE_EVENTS=

EXISTING_PROPERTY_REUSE_EVENTS=
PROPERTY_COMPOSITION_EVENTS=
NEW_PROPERTY_PRIMITIVE_GENESIS_EVENTS=

INCREMENTAL_ENTITY_UPDATE_EVENTS=
FULL_ENTITY_REWRITE_EVENTS=

PERSISTENT_PROPERTY_TRANSIENT_STATE_CONFUSION_EVENTS=

OBSERVED_ASSERTIONS=
INFERRED_ASSERTIONS=
PREDICTED_ASSERTIONS=
HYPOTHESIZED_ASSERTIONS=

UNCERTAIN_ASSERTIONS_TOTAL=
UNCERTAIN_ASSERTIONS_COLLAPSED_TO_CERTAIN=

WORLD_DELTA_EVENTS=
FULL_WORLD_SNAPSHOT_COPIES=
HISTORICAL_STATE_RECONSTRUCTION_PASS=

EXPLAINED_OBSERVATION_EVENTS=
IRREDUCIBLE_RESIDUAL_EVENTS=

TOTAL_EXPERIENCE_EVENTS=
TOTAL_SEMANTIC_MEMORY_BYTES=
NEW_SEMANTIC_BYTES_PER_EXPERIENCE_SEQUENCE=

REDUNDANT_EXPERIENCE_BYTES_PER_EVENT=
NOVEL_EXPERIENCE_BYTES_PER_EVENT=

DUPLICATED_SHARED_SEMANTIC_PAYLOAD_EVENTS=
INSTANCE_EXCEPTION_EVENTS=
UNNECESSARY_SCHEMA_FORK_EVENTS=

UNRESOLVED_SILENT_WORLD_CONTRADICTIONS=

TOTAL_WORLD_SEMANTIC_NODES=
ACTIVE_SEMANTIC_NODES_P50=
ACTIVE_SEMANTIC_NODES_P95=

ACTIVE_ENTITIES_P50=
ACTIVE_ENTITIES_P95=

WORLD_MEMORY_FULL_SCANS=

WORLD_GENERATOR_IS_SUCCESS_AUTHORITY=
WORLD_GOLD_GRAPH_READS=
EXPECTED_WORLD_STATE_LOOKUPS=
FUTURE_WORLD_EVENT_LEAKAGE_EVENTS=

NODE_ID_IS_SEMANTIC_PAYLOAD=
NATURAL_LANGUAGE_IS_CANONICAL_WORLD_MEMORY=
NATURAL_LANGUAGE_IS_WORLD_REASONING_AUTHORITY=
WORLD_MEMORY_NATURAL_LANGUAGE_BYTES_ON_HOT_PATH=

SHARED_SEMANTIC_REUSE_ABLATION_PASS=
RESIDUAL_LEARNING_ABLATION_PASS=
SPARSE_WORLD_MEMORY_ROUTING_ABLATION_PASS=

COMPRESSED_WORLD_MEMORY_NODES_PROMOTED=
COMPRESSED_NODE_DECOMPRESSION_AVAILABLE=
SEMANTIC_INFORMATION_LOSS_EVENTS=

GLOBAL_REASONING_REGRESSIONS=
META_QUALITY_REGRESSIONS=
GAIN_ERASURE_EVENTS=
CAPABILITY_NEGATIVE_TRANSFER_EVENTS=

EXTERNAL_LLM_CALLS=
LOCAL_TEACHER_CALLS=
NETWORK_READS=
NETWORK_WRITES=
REMOTE_EXECUTIONS=

NEW_CLIPPY_WARNING_SIGNATURES_TOTAL=
CORE_DOCKABILITY_PRESERVED=

NEXT_DOMINANT_GROWTH_LIMIT=

SEM31_LEVEL_A_PASS=
SEM31_LEVEL_B_PASS=
SEM31_LEVEL_C_PASS=
SEM31_LEVEL_D_PASS=
SEM31_LEVEL_E_PASS=
SEM31_LEVEL_F_PASS=
SEM31_LEVEL_G_PASS=

SEM32_STARTED=false
NEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY
```

---

# 39. SCIENTIFIC SUCCESS INTERPRETATION

SEM-31 PASS means:

> B_Core can maintain a persistent changing world as semantic structure rather than language or duplicated snapshots; existing entities acquire new knowledge primarily through incremental semantic edges; shared meanings are reused; genuinely new meaning is generated only when existing compositions are insufficient; state/history are represented through bounded deltas; epistemic provenance is preserved; and large persistent world memory remains sparsely accessed.

---

# 40. MOST IMPORTANT STORAGE PRINCIPLE

Do NOT optimize for:

```text
store fewer bytes at any cost
```

Optimize for:

```text
store each independent meaning once
+
reuse it everywhere possible
+
retain only irreducible differences
```

No semantically necessary distinction may be discarded merely to improve compression metrics.

---

# 41. AFTER SEM-31

Do NOT automatically start SEM-32.

If SEM-31 passes, the likely next frontier is:

```text
persistent semantic world state
        ↓
temporal causal dynamics
        ↓
predict next state
        ↓
prediction residual
        ↓
counterfactual simulation
```

That is the point at which the persistent semantic memory becomes an actual predictive world model.

Suggested commit:

`Establish persistent language-free semantic world state`

Start SEM-31 now.
