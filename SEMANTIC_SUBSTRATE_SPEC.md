# Semantic Substrate v0 — Conceptual Specification

**Canonical version:** 1.0.0
**Implementation status:** Not started
**Specification status:** Provisional research hypothesis

## 1. Purpose and non-goals

Semantic Substrate v0 specifies a language-neutral, executable representation
for primitives, episodes, candidate abstractions, and promoted concepts. It is
a schema-level hypothesis to be tested in SEM-0, not an implementation and not
evidence that computational meaning has been achieved.

This specification does not define a learning algorithm, a concept miner, a
reasoning search procedure, promotion thresholds, a mathematics solver, a
program synthesizer, language grounding, or self-modification.

## 2. ConceptIR v0

A ConceptIR record is a typed, versioned envelope. Fields may reference
content-addressed artifacts rather than embed them. Primitive records need not
populate every field, but absence must be explicit rather than silently
invented.

| Field | Required meaning |
|---|---|
| `concept_id` | Opaque stable identity; never a natural-language semantic authority. |
| `kind` | One of the record kinds in Section 3. |
| `signature` | Typed inputs, outputs, state variables, and admissible domains. |
| `preconditions` | Executable or formally checkable applicability conditions. |
| `invariants` | Properties expected to remain true across valid executions. |
| `relations` | Typed links to entities, concepts, states, operators, evidence, or constraints. |
| `transition_semantics` | State transformation or relation update behavior. |
| `predictions` | Testable outputs or state consequences with declared scope. |
| `affordances` | Actions or compositions made possible when applicable. |
| `counterfactuals` | Rules or generators for interventions and expected changed behavior. |
| `grounding_references` | Links to primitive observations, environment state, sensors, or formal objects. |
| `derivation_graph` | DAG-capable lineage of premises, operations, dependencies, alternatives, verification, and compression. |
| `executable_semantics_ref` | Versioned reference to the interpreter/operator implementation or declarative executable body. |
| `evidence` | Confidence plus structured evidence items, failures, scope, and calibration data. |
| `promotion_status` | Lifecycle state such as primitive, observed, candidate, rejected, promoted, superseded, or revoked. |
| `version` | Schema version and semantic generation. |
| `provenance` | Origin, run, split, seed, parents, authorship class, timestamps, hashes, and contamination attestations. |
| `lexical_aliases` | Optional human labels, languages, and display notes; non-authoritative metadata. |

### 2.1 Illustrative shape, not implementation

```text
ConceptIR {
  concept_id,
  kind,
  signature: { inputs, outputs, state },
  preconditions,
  invariants,
  relations,
  transition_semantics,
  predictions,
  affordances,
  counterfactuals,
  grounding_references,
  derivation_graph,
  executable_semantics_ref,
  evidence: { observations, failures, confidence, scope },
  promotion_status,
  version: { schema, generation },
  provenance,
  lexical_aliases?
}
```

The concrete serialization, type system, interpreter, and canonical byte
encoding are SEM-0 design decisions. A text serialization is allowed as a
transport format; text values must not become the meaning substrate merely
because JSON, TOML, or source code is used for storage.

## 3. Record-kind separation

### 3.1 Primitive

A supplied atomic operator, relation, state element, observation contract, or
environment rule explicitly permitted by the experiment. Its supplied status
and trusted semantic boundary must be recorded. A primitive is not claimed as
autonomously discovered.

### 3.2 Observation/Episode

An immutable record of inputs, state, actions, outputs, verification results,
and provenance from an execution. An episode is evidence, not a generalized
concept. Replaying one episode is solved-instance recall.

### 3.3 Candidate Abstraction

A proposed generalization derived from observations or derivations. It may
contain executable behavior, but it remains untrusted until all applicable
promotion gates pass. Repeated occurrence is nomination evidence only.

### 3.4 Promoted Concept

An immutable semantic generation that passed the experiment's executable,
reuse, blind-transfer, counterfactual, benefit, regression, provenance, and
causal-ablation gates. Promotion is scoped; it does not imply correctness
outside the validated domain.

### 3.5 Cached Solution

A stored mapping or trace for a solved instance. It may be useful as an
explicit baseline or operational cache, but cannot be promoted by relabeling.
Cache hits must remain observable in metrics and provenance.

### 3.6 Structural Macro

A repeated or compressed subgraph that can replay structure. It lacks, or has
not yet passed, the stronger semantic validation. It is the critical baseline
against which a semantic-evolution claim must be distinguished.

### 3.7 Human Lexical Alias

An optional mapping between a human-language form and an opaque record ID. An
alias may support inspection or later language grounding. It may not alter
execution, identity, promotion, routing truth, or validation outcomes.

## 4. Executability and verification

Executable semantics may be declarative, interpreted, or safely referenced,
provided the evaluator can reproduce them from a clean process. Every
execution must identify the semantic generation, interpreter/operator version,
inputs, outputs, resource usage, and verification result.

Predictions and counterfactuals must declare their domains and failure modes.
Unknown or out-of-scope is preferable to an unjustified answer. Confidence is
evidence metadata, not a substitute for correctness.

## 5. Derivation graph requirements

The derivation graph must support typed nodes for premises, observations,
operators, intermediate states, hypotheses, contradictions, verification
events, rejected alternatives, compression events, and conclusions. Edges
must record dependency roles. The graph must retain:

- every parent generation;
- the distinction between supplied and discovered content;
- failed and negative evidence relevant to promotion;
- counterfactual and ablation results;
- compression mapping from historical subgraph to operational unit;
- experiment, split, seed, and evaluator identity.

Operational reuse may reference a compressed node, but audit and replay must
recover its epistemic history.

## 6. Identity and version rules

1. Use content-addressed or deterministic IDs where practical. Random IDs are
   permitted only with a deterministic content hash recorded alongside them.
2. Canonicalization must exclude non-semantic display aliases from semantic
   identity unless an experiment explicitly tests lexical behavior.
3. A promoted generation is immutable. New evidence or changed behavior
   creates a new version or successor generation.
4. Replacement and supersession are explicit edges; they never overwrite or
   delete prior semantics.
5. Full derivation lineage and validation evidence are retained.
6. No silent semantic mutation is allowed. A changed executable reference,
   invariant, precondition, transition, prediction, or counterfactual rule is a
   semantic change requiring a new generation and revalidation.
7. Equivalent content deduplication may share storage, but distinct evidence
   histories must remain recoverable.

## 7. Promotion-state discipline

A lifecycle may include `PRIMITIVE`, `OBSERVED`, `CANDIDATE`, `REJECTED`,
`PROMOTED`, `SUPERSEDED`, and `REVOKED`. State changes are append-only events
with reasons and evidence hashes. A failed gate cannot be converted to pass by
changing the label. Exact thresholds are defined by the experiment protocol,
and promotion must fail closed when evidence is missing or contaminated.

## 8. Open questions reserved for SEM-0

SEM-0 may choose and test the minimal type system, canonical encoding,
interpreter boundary, counterfactual generator contract, evidence aggregation,
and promotion thresholds. Those choices must remain small and closed-world.
They may revise this provisional representation only through versioned
evidence and, if constitutional meaning changes, the amendment process.
