# Event roles own conversational referents

The native path could answer an explicit actor question while failing the next
personal-pronoun question. Inspection found three separate boundaries:

- A second passive `by` extractor consumed the rest of the clause after the
  ordinary preposition parser had already separated the agent and location.
- Bare interrogative arguments were represented as named entities, so `who`
  entered the person cache.
- Event questions deliberately bypass generic text coreference, but their
  event-context resolver lacked ordinary third-person personal pronouns.

The fix retains that intentional event-question boundary. It does not send
event questions through a nearest-noun rewrite. Personal-pronoun expressions
use the existing event-role reference kind, source context, ambiguity handling
and independently replayable binding proof. No gender is inferred from a name.

The preposition parser is now the only owner of passive by-agents. Interrogative
frames represent bare who/whom/what and supported Korean forms as QueryVariable
nodes. Their role edges remain available; entity memory and concrete proposition
bindings exclude them. This is grammatical knowledge, not a sentence blacklist.
The same surface in a declarative named-entity position is not globally banned.

Existing descriptive events also project their participant/location roles into
the common reference store. This avoids requiring a planning predicate for every
ordinary observed verb. Only active actual-world, non-denied/non-doubted event
records updated in the current turn and aligned to its original utterance
supply this projection. Embedded information statements, state descriptions and
derived source fragments cannot pass as outer event-role introductions. Alignment
uses the source hash or exact text after terminal statement punctuation/whitespace
removal, because the observation ledger removes that punctuation. It does not promote reported
content into verified truth, or authorize an action. Multiple projections of one
entity in one turn count as one mention, not repeated supporting evidence.

## Paired verification

Checks cover PP order, coordinated agents, bare-agent passives, query variables,
explicit names, passive/active observations followed by actor and personal-
pronoun questions, and unchanged evidence/action boundaries. Existing ambiguity,
quotation, event, information-gap and dialogue-continuity checks remain required.
One generation and one final realization check are asserted before independent
test replay. No finished-output rewrite or additional generation loop is added.

The action-oriented role graph does not itself cover every descriptive verb.
The cleaned-event case tests the descriptive-memory bridge; the coordinated-PP
unit test uses a predicate supported by that graph. These are different layers,
not interchangeable claims of grammatical coverage.

Schemas: role graph 5, conversation state 111, response 107, discourse answer 38.
No automatic stored-state migration, deployment, commit or push. The original
semantic catalog, bilingual dictionary and recursive policy remain unchanged.
General natural conversation remains unestablished by these bounded checks.
