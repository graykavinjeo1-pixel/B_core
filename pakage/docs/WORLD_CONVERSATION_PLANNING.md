# Context-bound world conversation and utterance planning

This is a bounded Rust engineering extension, not completion of unrestricted
conversation or a model of the full human mind. It extends the existing owner
of world-dialogue turns; no new competing answer router or inference engine.

## Pipeline

`utterance + discourse focus -> source-bound propositions / reference gap ->
conversation memory -> core deliberation -> typed utterance moves -> lexical
selection -> syntax / morphology -> replay validation`

- Memory stores proposition identities, polarity, relations, supplied evidence
  and lexical revisions, not a cache of complete replies. Original utterances
  remain separately as source records so their interpretation can be checked.
- A frozen pre-turn discourse snapshot grounds omitted subjects and references.
  It is context, not evidence that a proposition is true. New topic boundaries
  clear it without deleting remembered premises and mechanisms.
- The existing core determines the conclusion or missing premise. Its actual
  diagnostic question becomes focus. Both yes/no and a matching semantic state
  reply can fill that slot and resume the original query.
- A pronoun with two candidates produces a typed reference gap, not a guessed
  fact. The short choice is tied to the original question, source hash, context
  and lexical revision. Clarification cannot create evidence or action records.
- `WorldUtterancePlanIR` chooses premise, inference, conclusion, uncertainty and
  question moves from the core receipt before expression. The final conclusion
  is not repeated if already stated by the selected proof path.
- A direct user report establishes what was reported, not its real-world cause.
  With no derivation, a why-question produces a cause-unknown move. Supplied
  implications explain derivations; they are not verified causal world laws.
- The existing expression/syntax/morphology pipeline realizes those moves.
  Korean zero-subject rules preserve the semantic subject node and trace while
  omitting a repeated surface subject only in licensed contexts. No output text
  is deleted or rewritten after generation to simulate this behavior.

## Concrete increment

Contextual predicate fragments, contrastive subjects (`What about gate?` /
`다른문은?`), why/then follow-ups, short answers to actual information requests,
explicit corrections, bounded filler prefixes, social focus preservation,
two-candidate reference clarification, and shared-subject explanation ellipsis.

Three supplied unary state predicates are initialized in new conversation worlds:

| Shared identity | English root | Korean root / grammar |
| --- | --- | --- |
| W_USER_900001 | tired | 피곤 / KOREAN_HADA_STATE |
| W_USER_900002 | free | 한가 / KOREAN_HADA_STATE |
| W_USER_900003 | frustrated | 답답 / KOREAN_HADA_STATE |

These are lexical/boolean primitives, not emotion diagnoses, autonomous concept
discovery, or a conversational knowledge corpus. `I am` / `나` / `저` bind the
speaker identity, which realizes as `you` / `너` from the assistant viewpoint.
Predicate identity remains separate from Korean/English words and inflection.

## Acceptance and measurement

Six added test functions cover:

1. Personal state reports, contextual questions, corrections and source-versus-
   cause separation in both languages; serialization and source-context tampering.
2. Real diagnostic-slot completion, resumed inference, thanks and contrastive
   retargeting without fake plans or action records.
3. Ambiguous reference -> clarify -> short choice -> original query -> evidence
   acquisition; tampered reference choices rejected.
4. Proof-driven move selection, duplicate-conclusion removal, Korean licensed
   subject ellipsis and tampered utterance plans rejected.
5. Eight builtin properties x two languages (16 structural cases), retargeting,
   context ablation and unrelated-topic boundaries.
6. Speaker/addressee lexical perspective in subject, object and clarification
   roles; first-person relation agreement and choice replies without IR leaks.

These are development regression tests, not a blinded conversational benchmark,
GPT similarity score or a percentage of human-level language understanding.
The execution receipt is
`reports/language-cortex-completion/world_conversation_planning_2026-09-04.json`.

## Limits and integration

- Parsing remains controlled and present-tense. No unrestricted paraphrase,
  nested natural clauses, arbitrary vocabulary, English contractions, or general
  Korean conjugation. Unsupported language may still use older routes.
- Focus-based ellipsis is deliberately narrow. It does not solve general
  pragmatic speaker/addressee inference: a fragment about the focused speaker
  is not proof of understanding a question addressed to the assistant.
  `Then?` / `그럼?` currently requery the focused proposition, not automatically
  request advice or a next step. The CLI transcript exposes this still-literal
  behavior; passing route checks must not be read as passing naturalness checks.
- At most two distinct referents from the focused proposition are clarified;
  arbitrary discourse salience and multi-entity reference negotiation remain out
  of scope. Invalid choices do not silently become a selected referent.
- Stored statements are attributed premises, not sensor observations or verified
  reality. Source replay checks interpretation consistency, not source authenticity
  or a complete independently reconstructed historical event journal.
- Morphological grammar and discourse markers still contain small hand-authored
  rules. Whole responses are composed on the new path, but this does not remove
  every old hard-coded sentence in the repository.
- Canonical concepts, research stages and self-improvement quarantine are
  unchanged. No LLM/teacher dependency or automatic source modification added.
- State schema is `B_CORE_CONVERSATION_STATE_32`; response schema is
  `B_CORE_CONVERSATION_TURN_RESPONSE_22`. Consumers must accept the new optional
  clarification payload and nested discourse/grounding/utterance-plan fields.
  Start a new session or explicitly migrate old saved state; no silent migration.
- Source is mirrored in `pakage`. Binaries/services are not automatically
  deployed, and this turn does not commit or push changes.
