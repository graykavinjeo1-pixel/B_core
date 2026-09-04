# Registered world predicates and lexical revisions

Current context/ellipsis and utterance-planning extension, schema changes and
limits: [WORLD_CONVERSATION_PLANNING.md](WORLD_CONVERSATION_PLANNING.md).
The receipt below describes the earlier registration-only revision.

Engineering extension of the existing world-dialogue cycle. No SEM stage
advancement, autonomous concept discovery, promoted-concept mutation, external
model invocation, source self-mutation, commit, push or deployment is implied.

## Scope and acceptance

- Register fresh unary boolean states and directed binary boolean relations
  through a structured host API, without adding topic/sentence branches.
- Keep opaque predicate identity/arity separate from Korean/English lexical
  roots, morphology and surface aliases. Only supplied premises and implications
  give these predicates inferential content. A word registration is not learning
  a real-world law, adding verified evidence or promoting a semantic concept.
- Use the existing core deliberator for signed support/opposition, conflict,
  missing-premise selection, hypothetical evaluation and proof explanation.
- Rename/remove aliases without changing old memory or core decisions. Retain
  the lexical revision that grounded each episode; never reinterpret old text
  using today's aliases. Preserve yes/no question bindings under lexical changes.
- Demonstrate new roots, ordered arguments, KO/EN equivalence, negative/conflict/
  unknown routes, semantic/lexical ablation, source replay and atomic failures
  through the real conversation API, plus workspace regression checks.

`world_vocabulary.rs` is a bounded data/grammar layer, not another response router
or a second inference engine. The sole conversation-memory owner commits updates.
Grammar selects expression nodes; the core decision is fixed before realization.

## Host API

`UPDATE_WORLD_VOCABULARY` accepts `conversation_id` and an `update` object:

```json
{
  "operation": "UPDATE_WORLD_VOCABULARY",
  "conversation_id": "example",
  "update": {
    "predicates": [{"predicate_id":"W_USER_42","arity":"BINARY"}],
    "aliases": [
      {"alias_id":"dep.en","predicate_id":"W_USER_42","language":"ENGLISH","root":"depend on","grammar":"ENGLISH_REGULAR_VERB"},
      {"alias_id":"dep.ko","predicate_id":"W_USER_42","language":"KOREAN","root":"의존","grammar":"KOREAN_HADA_LOCATIVE"}
    ],
    "remove_alias_ids": []
  }
}
```

The result is `WORLD_VOCABULARY_UPDATED` carrying the rehashed conversation state.
Registration creates an empty session if necessary, but does not advance a turn.
Subsequent `PROCESS_CONVERSATION_TURN` requests use ordinary turn indices:

1. `alpha는 beta에 의존하면 gamma는 안전 상태다.`
2. `gamma는 안전 상태인가?` — ask whether alpha depends on beta.
3. `응` — bind the reply to that ordered relation and infer gamma's state.
4. `왜?` — show the supplied relation and applied implication.

English equivalents include `alpha depends on beta`, `alpha does not depend on
beta`, `Does alpha depend on beta?`. Korean roots use locative `에` or accusative
`을/를` and the bounded 하다 morphology. Unary aliases use `COPULAR`. Supported
grammar classes describe lexical behavior, never semantic inference laws.

To rename an alias, remove its ID and add the replacement in one update. Predicate
contracts cannot be changed/reused for a different arity. Multiple different
aliases may share a predicate. Ambiguous roots, malformed entries, unknown alias
removals and capacity overflow fail atomically. No implicit pruning of evidence.

## Limits and deployment

- Maximum 128 local predicate contracts, 128 aliases per revision, 32 lexical
  snapshots (including the initial empty snapshot). These are bounded local
  lookups, not scans of the promoted semantic catalog.
- Existing limits remain: 64 premises, 64 implications, four conjunctive
  antecedents, single-identifier entities and the existing core search bounds.
- Binary arguments are ordered; symmetry/transitivity is never inferred from
  the verb. Relations are grounded instances, not quantified variable rules.
- Unresolved pronouns/partial parses are rejected by this path. Unsupported
  language can still enter legacy routes; this is not universal NLU replacement.
- A concept with no current lexical alias can still be reasoned about directly.
  If a required output alias is missing, realization fails and the conversation
  turn rolls back; it must not invent a translation or lose the prior memory.
- Grammar coverage: present copular states, English regular lexical verbs
  (including a bounded preposition slot), Korean nominal 하다 relations. No
  claim of unrestricted morphology, automatic natural-language definitions,
  broad vocabulary, sensor perception or human-level conversational quality.
- Runtime and tests are Rust. Schema 31 state / schema 21 responses require a
  fresh session or explicit migration; no silent saved-state migration. New
  expression morphology enum variants must be accepted by downstream consumers.
- Runtime sources are mirrored to `pakage`; unrelated package-only work is
  preserved. Existing service binaries are not automatically deployed.

## Verification receipt (2026-09-04)

- World-dialogue tests: 21 functions passed (7 added in this extension).
- Extension matrices: 16 relation/evidence combinations, each realized in both
  languages; 12 generated opaque roots with positive/negative Korean morphology.
  These are development structural tests, not a blinded general-language score.
- Root workspace library tests: 1,173 passed. Portable workspace: 658 passed.
  Both format checks and adapter library/CLI Clippy checks passed.
- Rebuilt CLI: 20 commands, including 16 conversation turns, two registrations
  and two alias renames. Both languages completed two-step inference, missing
  premise acquisition, explanation, lexical rename and corrected-premise
  retraction. No action records were generated; semantic contracts were unchanged
  by rename. Missing evidence was reported as unknown, not falsely refuted.
- Canonical manifest: 10 unchanged files. Runtime mirror: 19 identical files.
  Unrelated package-only changes were preserved.
- Existing optional Python research hooks were not exercised (pytest absent).
  The new runtime and all new tests are Rust; no LLM dependency was introduced.

Source-repository evidence:
`reports/language-cortex-completion/registered_world_predicates_2026-09-04.json`.
No commit, push or service deployment was performed.
