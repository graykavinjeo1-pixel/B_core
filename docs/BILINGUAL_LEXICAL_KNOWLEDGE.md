# Bilingual lexical knowledge and the planner boundary

## Product outcome

The Rust input path now has a source-backed 15,000-headword Korean dictionary
with paired English expressions, separate senses, POS, definitions, principal
forms, syntax frames and grammar metadata. An additional 2,157 grammar entries
are separate from the requested headword count. Data provenance, selection
limits and license are in `crates/semantic-core-adapters/data/lexical-knowledge/`.

This advances input grounding and inspectable memory. It does not claim
unrestricted conversation, automatic reasoning from every definition, or that
word count measures naturalness. The prior world-dialogue planning extension is
described in `WORLD_CONVERSATION_PLANNING.md`.

## One boundary, two kinds of knowledge

`text -> indexed lexical lookup -> source sense candidates / morphology ->
working LexemeIR facets -> language evidence`

The existing language circuit still selects semantic goals. Source definitions
are **not** executable predicates. `LEXICAL_DEFINITION_ONLY` activations remain
in the lexical receipt and matched-knowledge provenance, but do not enter the
planner's context tags or select action intent. The concept-promotion path and
world-predicate registration boundary are unchanged.

This distinction repaired five regression failures: dictionary polysemy had
expanded context beyond the core planner's 64-item boundary. The fix separates
evidence from planner authority; it does not enlarge that limit. A synthetic
128-candidate boundary test checks intent/context invariance independently of
particular dialogue examples.

The 64-activation output budget also reserves semantic routing candidates ahead
of definition-only candidates, regardless of dictionary familiarity scores.
Lexical crowding cannot evict an otherwise selected legacy routing candidate.

The shared static pack is indexed once (OnceLock), not copied per conversation.
Lookup probes forms rather than scanning the catalog. Bounds are 8,192 input
characters, 128 tokens, eight-token phrases, and 32 returned entry/form matches.
Truncation is explicit; these are not no-false-negative guarantees for arbitrary
input. Mutable lexical memory materializes touched source senses only, with
derived surface/sense indexes rather than a per-turn full-store scan.

## Paired English attachment and ambiguity

`C_LEX_NIKL_{entry}_{sense}` identifies one lexical meaning. Korean and English
facets share it; aliases and inflections do not create autonomous generations.
Injecting either NIKL facet prevalidates and attaches both atomically. A snapshot
with half of a source pair is rejected without mutation. Forged translations or
action hints under the source namespace are rejected.

This guarantee applies to **source-backed imported senses**. Legacy custom
`InjectLexeme` remains compatible; an arbitrary novel Korean term with no
verified English equivalent cannot be translated by pretending one exists.
Such terms need an explicit paired definition/alias input or a future verified
translation source. The pack is not a general-purpose machine translator.

Korean homographs retain all candidate senses. English semicolon-separated
equivalents narrow to their matching source senses. Frequency is not a proof of
which sense the user intended. Definitions remain source claims, not current
legal conclusions or world truths.

## Agglutinative morphology

Productive recognition uses source principal parts, not remembered sentences:
stem endings, connective contractions, polite endings, past tense, prospective
and intention endings, propositive forms, and nominal particles. Source parts
support irregular roots such as 듣다/들어 and 돕다/도와. The six supplied 먹다
forms and a cross-root matrix are regression checks.

Morphological receipts name the base, ending and rule. A grammar entry being
stored does not mean every construction is implemented in the parser or output
realizer. This is bounded recognition, not a complete Korean analyzer: arbitrary
typos, fused compounds, all irregular variants, English inflection, speaker
pragmatics and free sentence synthesis remain incomplete. A lexical match for
먹지 does not independently decide the scope of negation.

## API / compatibility / operation

JSON-line cognitive API additions:

```json
{"operation":"LEXICAL_KNOWLEDGE_PACK_STATISTICS"}
{"operation":"LOOKUP_LEXICAL_KNOWLEDGE","text":"계약서를 먹었어?"}
```

Conversation response schema is now `B_CORE_CONVERSATION_TURN_RESPONSE_23` and
includes a replay-validated `lexical_knowledge` receipt. This is evidence, not a
second response owner. `lexical_activations` in the existing language path now
includes matched source senses; those are not merely report-only inventory.

Conversation state remains schema 32 from the preceding extension. Consumers of
older conversation states need migration or new sessions. Lexical snapshot
schema stays 1; only touched facets/usage are serialized, with indexes rebuilt
on restore. Future source-pack revisions require an explicit migration of old
source-locked facets. There is no silent dictionary refresh at runtime.

Source and portable `pakage` contain the same lexical files, importer and license
notice. Rebuild the cognitive API to use them; the prebuilt canary is not a new
language server. Deployment is not automatic. Runtime has no Python, LLM,
teacher API, network lookup, autonomous source mutation or auto-commit path.

Rollback: revert the language/lexical release commit and rebuild; use compatible
prior snapshots or new sessions. Do not delete unrelated package modifications.
