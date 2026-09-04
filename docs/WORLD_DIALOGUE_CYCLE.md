# World-grounded dialogue cycle

Current follow-up: [WORLD_CONVERSATION_PLANNING.md](WORLD_CONVERSATION_PLANNING.md)
adds context-bound fragments, clarification and typed utterance planning. The
original receipt below remains historical rather than a current deployment claim.

Engineering scope: connect language to the existing core deliberation engine.
This is a functional analogy, not a claim to reproduce human cognition.
Canonical concepts, autonomous promotion, source-mutation quarantine and prior
dirty-tree changes are unchanged.

## Executable work and acceptance

1. Understand a bounded copular-state/conditional grammar in Korean and English.
   Compile to entity identities, state predicates, polarity and implications.
   Reject partial parses. Natural-language claims are premises, not verified
   observations of reality. Preserve raw episode/source separately from meaning.
2. Remember bounded, conversation-local premises and conditional mechanisms.
   Keep contradictory reports; explicit correction supersedes earlier reports.
   Hypothetical interventions are local to a query and cannot alter memory.
3. Think using the existing core: retrieve a goal-relevant dependency closure,
   compare support and opposition, compose inference mechanisms, expose hypotheses,
   counterfactuals and search receipts. Do not write a second rule-answer engine.
4. Judge and decide before realization: conclusion, conflict, information request
   or resource-limited deferral. No closed-world assumption or execution grant.
   Bind a yes/no reply only to the missing premise actually queried by the core,
   retain its question/decision provenance, and re-deliberate the original goal.
   Replies in hypothetical worlds cannot become actual-memory observations.
   When no discriminating premise is available, report the gap without returning
   the user's original question as a question to them.
5. Realize decision and proof through shared atomic expression nodes and a small
   clause grammar. Do not add one complete answer per topic or sentence.

The production PROCESS_CONVERSATION_TURN path must use the decision, not merely
attach an unused report. Unsupported language remains on the existing path.
Recognized state and implication acknowledgements use the same atomic clause
grammar. Their old execution-guard response route no longer owns these turns.
Unsupported language and other response families retain legacy paths; this
migration does not remove every existing hard-coded response in one step.

Acceptance: unseen entity combinations and multi-step chains, conjunction,
positive/negative/opposing/unknown premises, correction, evidence ablation,
temporary counterfactuals, expression ablation, Korean/English decision parity,
source/decision tampering rejection and existing Rust regression tests. Passes
measure this declared slice, not general conversation or autonomous emergence.

## Implemented boundary and current limits

The sole conversation-state owner stores `DialogueWorldIR`. Episodes contain raw
speaker statements and source hashes separately from typed entity/property/value
and prerequisite/effect relations. They are supplied premises, not sensor evidence,
verified reality, or autonomously promoted concepts. No vision/sensor acquisition
or new law-discovery mechanism is claimed by this change.

The initial bounded parser grounded eight shared state concepts (active, ready,
open, available, safe, valid, connected, powered), Korean copular expressions and
English equivalents, novel single-identifier entities, up to four conjunctive
antecedents, explicit negation, correction and one query-local assumption. It
rejects quotation, partial parsing, unresolved pronouns and mixed commands.
The subsequent registered-vocabulary extension adds supplied unary predicates
and ordered binary relations through the same pipeline; see
`WORLD_PREDICATE_REGISTRATION.md`. This is not unrestricted Korean/English
understanding: rich event roles, flexible paraphrases, autonomous vocabulary
acquisition and general conversational style remain incomplete.

Example full cycle, using the public conversation API:

1. `alpha가 가동 상태이면 beta는 안전 상태다.` — store a conditional premise.
2. `beta는 안전 상태인가?` — cannot infer without alpha; ask its state.
3. `응` — record an answer about **alpha**, not beta; derive beta via the core.
4. `왜?` — realize the recorded premise and actual inference path.

The existing core deliberator receives language-independent signed support atoms.
Support and refutation are searched separately, plus a core search for joint
support/refutation of any relevant proposition. This avoids letting a direct
observation hide a contradictory derived result. Goal-dependent working sets
filter conversation-local mechanisms; no promoted concept catalog is scanned.
Diagnostic selection is a separate core request so it cannot hide a truncated
proof search. Proof search depth is 16, beam width 32; memory is capped at 64
premises and 64 implications, with a 64-atom working-set bound. Capacity failures
are explicit and do not evict evidence or partially commit a turn.

`WorldDecisionIR` is fixed before expression generation. `WorldReasoningIR`
records requests, hypotheses, counterfactuals, selected proofs, bounded-search
dispositions and an independently replayable decision. No external execution or
model invocation is authorized. Hypothetical interventions do not modify stored
premises. Korean/English output is assembled from entity/property/polarity and
epistemic-force nodes; expression ablation does not remove core reasoning.

Serialized dialogue state now uses `B_CORE_CONVERSATION_STATE_31`, public responses
use `B_CORE_CONVERSATION_TURN_RESPONSE_21`. State persists in the existing session
snapshot; no new automatic disk persistence is introduced. Existing saved states
need migration or a new session. The public DiscourseAnswer carrier now includes
either a world reasoning receipt or a typed memory update; downstream consumers
must not interpret a memory acknowledgement as a verified worldly fact.

No deployment, commit or push is authorized. Mirror verified runtime sources to
pakage, preserving unrelated package-only edits. State schema changes require
explicit migration or fresh sessions; no silent persistent-state migration.

## Initial milestone verification (2026-09-04; historical receipt)

- 14 new Rust test functions, including 48 structural combinations / 96 language
  realizations. These are development tests, not a blinded general-NLU benchmark.
- Root workspace library tests: 1,166 passed; portable workspace: 651 passed.
- Root/portable format checks and adapter-library/CLI Clippy checks: passed.
- Rebuilt CLI: 14 turns across Korean information acquisition/explanation/revision,
  English conflict and hypothetical conversations; all expected verdicts matched,
  with no generated action records. Unsupported inference reports a gap, not a
  copy of the original question.
- Canonical manifest: 10 files unchanged; runtime mirror: 18 files identical.
- Existing optional Python/pytest research hooks were not exercised because
  pytest is unavailable. The new runtime and tests are Rust.

Evidence: `reports/language-cortex-completion/world_dialogue_cycle_2026-09-04.json`
in the source repository. No commit, push or service deployment was performed.
