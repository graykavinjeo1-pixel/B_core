# Event reference understanding (2026-09-05)

This describes the earlier question-reference checkpoint. The subsequent
conversational continuity repair also binds references in new reports, retains
correction provenance and answers multiple roles. See DIALOGUE_REPAIR_ACCEPTANCE.md
and reports/language-cortex-completion/dialogue_continuity_final_2026-09-05.json
for the current implementation and evaluation; the limits below are historical.

This change lets a role question refer back to an attributed event, then search a
different recorded event using that resolved entity. For example, after identifying
who lent a book, asking who read "it" binds the book and queries reading records.
The answer can then support a location follow-up.

The implementation extends the existing PropositionContent / DiscourseQa path.
It adds no response owner, new executable semantic concept, or runtime teacher.
The source report, belief/event identity, original reference mention, antecedent
role and resolved entity remain inspectable in ContentProjectionIR.

Supported references are object pronouns (it / 그것), named demonstratives
(that book / 그 책), place references in an explicit location role, and the neutral
person form (that person / 그 사람). A person reference needs a unique participant
or the participant role selected by the previous question. Gender is not inferred
from names. Context comes from a valid answer focus or an unambiguous immediately
preceding report. The existing three-turn focus limit remains in force.

After resolving the mention, all explicit query constraints must match one event.
A different predicate may therefore select a different record. Explicit new entities
override the old focus. Missing or ambiguous antecedents produce a gap rather than
using the latest arbitrary noun. Unresolved references in stored reports are excluded
from literal event answers. The source question and the original source report stay
intact; generated text is never used as memory evidence.

Reference bindings are replayed from their source proposition. The public response
validator also checks the antecedent record against the live ledger, including its
source actor, event identity, active status and actual-world classification.
This is consistency validation, not independent authentication of conversation history.

A related commit_answer_focus defect checked the old reference instead of the incoming
one. It now validates the reference actually being committed.

## Validation

Run the new controlled diagnostic against both predecessor and new executables:

```powershell
./scripts/verify_event_reference_understanding.ps1 -Executable <executable>
cargo test --locked --offline -p semantic-core-adapters --lib
cargo clippy --locked --offline -p semantic-core-adapters --lib --bin b-core-cognitive-api -- -D warnings
```

The matrix contains 12 cases / 42 turns / 26 questions: cross-event reference chains,
named/place references, ambiguous people, incompatible event objects, explicit topic
replacement and absent prior context. The baseline, including its failed answers,
is preserved in event_reference_baseline_2026-09-05.json. Regression scripts cover
the previous event roles, state/decision intent, world reasoning and lexical store.

Unit tests check cross-event joins, expired/retracted antecedents, person ambiguity,
source tampering through the public API and 18 generated reference substitutions.
These are developer-controlled checks, not a blind evaluation of general language.

## Limits

The report-side binding of a new assertion such as "she read it" remains unimplemented.
This increment resolves questions over already explicit event reports. It does not
add unrestricted subordinate clauses, passive voice, gender knowledge, arbitrary
ellipsis, ownership/return duties, causal inference or advice generation. Lexical
identity is not general word-sense disambiguation. Responses still use explicit
attribution and role descriptions, so conversational naturalness remains limited.

Response schema is B_CORE_CONVERSATION_TURN_RESPONSE_26; state schema is
B_CORE_CONVERSATION_STATE_35. Content projections have optional reference_context
and reference_bindings. New sessions or explicit migration are required.

## Delivery status

The tested snapshot patch was applied to I:\B_Core and its pakage mirror on
2026-09-05 after verifying the canonical manifest, patch hash and predecessor
compatibility. Existing unrelated uncommitted changes were preserved. Application
and original-repository verification are recorded in
reports/language-cortex-completion/event_reference_application_2026-09-05.json.
Historical snapshot reports retain their original permission-blocked status.
No service restart, saved-state migration, commit or push is included.
