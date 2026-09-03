# Structural Generalization Rebase

## Decision

Language Cortex development is rebased from utterance-specific repair to typed,
compositional generalization. A test case may reveal a missing semantic
operation, relation, state, or invariant, but it must not authorize a stored
sentence reply or a whole-utterance dispatch rule.

## Measured baseline

Audit snapshot after the response-plan rebase:

- product Rust: 50 files, 100,883 lines;
- canary/support Rust: 155 files, 57,648 lines;
- `contains_any` occurrences across the crate: 254;
- narrow predicate/argument/discourse matrix: 216/216;
- response-plan act/signal cross-product: 152/152;
- semantic-core-adapters library regression: 573/573;
- independent held-out conversation diagnostic: 12.5833/100;
- frozen V3 mean: 4,803 basis points.

The high structural-canary scores and low held-out conversation score are not
contradictory. They show that local closed operations generalize inside the
represented state space while ordinary conversation still requires semantic
dimensions that the IR does not yet preserve.

## Root causes

1. Lexical detection and semantic decision are mixed in several legacy paths.
   Surface evidence is legitimate for activating a construction, but a raw
   substring must not itself select the final intent, scope, or response.
2. `UtteranceIntentGraphIR` retains multiple candidates but projects one
   `selected_candidate_id`. Some downstream consumers therefore still see a
   winner rather than a compatible set of discourse contributions.
3. Until this rebase, natural realization also projected one response act.
   Feedback, affect, topic motion, and the actual task could overwrite one
   another.
4. Requested in-turn artifacts, preferences, constraints, and propositions are
   not yet carried through one shared language-independent discourse ledger.
   The system can recognize pieces and still lose them before realization.
5. Many canaries protect previously discovered examples. They are useful
   regressions, but their count is not evidence of broad conversational
   capability.

## Implemented architectural correction

`NaturalResponsePlanIR` now retains an ordered set of response moves:

```text
relational support* -> discourse bridge* -> exactly one primary task
```

Each move owns an act, role, source, and generation trace. The primary task
cannot be replaced by feedback, affect, or topic-transition signals. Auxiliary
moves cannot grant semantic authority or execution. A compatibility scalar act
is checked against the primary move rather than independently selected.

This removes one winner-takes-all bottleneck and is covered by a full 152-case
act/signal cross-product, integration tests for feedback plus correction and
affect plus request, and the 573-test library regression.

## Generalization acceptance rule

A language change is accepted only when all of the following hold:

1. It adds or repairs a typed operation, relation, state transition, or
   invariant rather than a completed-sentence rule.
2. The same semantic structure is shared by Korean and English expression
   phenotypes.
3. It passes a construction cross-product or metamorphic family, not only the
   motivating example.
4. Surface forms remain non-authoritative and cannot grant execution.
5. Existing product regression remains green.
6. Held-out conversation quality improves on a separately frozen evaluation;
   canary volume alone is not counted as capability.

## Next structural work

The next implementation boundary is a language-independent discourse
proposition/requested-output ledger. It must preserve user constraints,
preferences, selected referents, and requested conversational artifacts from
parsing through response planning. Only after that boundary is closed should
the system add more expression knowledge or rerun a fresh official benchmark.

Current status is not GPT-level. The rebase identifies why progress previously
looked repetitive and changes the acceptance mechanism so sentence-level
patching cannot masquerade as generalization.
