# Attributed interaction preferences

The product goal remains natural conversation, not merely passing a narrow
regression suite. This increment addresses positive/negative preference scope,
source-attributed memory and response selection. It is not a completion claim.

## Representation and ownership

InteractionPreferenceIR retains a language-independent desired mode, excluded
modes, desired surface evidence, raw source hash and clause-coverage flag.
Supported modes distinguish listening, conversation, concise response, explanation,
advice, summary and execution. They are not affect labels or concept generations.

The preference parser is shared by the conversation contract, proposition
compiler and realization. Only fully covered, non-conflicting conversational
preferences can own the response. A remaining task clause prevents exclusive
conversational ownership. Hearing an explanation is not treated as asking the
assistant to listen.

PropositionContentIR stores the typed preference and validates it by replay from
its raw source. When normal clause ingestion omits a containing utterance's
exclusions, the combined attributed report is retained as nonactual preference
data. It grants neither world-fact status nor permission to execute an action.

Equivalent wishes from one turn's clause and full utterance are deduplicated only
for answer selection; source records remain. The more complete exclusion-bearing
source is preferred. Distinct wishes still remain ambiguous.

Response generation uses existing grounded dialogue graphs. It does not look up
an entire test sentence or inject a solved response into semantic knowledge.
The vocabulary and grammatical constructions remain controlled and incomplete.

## Validation and remaining work

Run the retained 50-turn developer probe, 98-turn continuity regression, other
existing CLI suites and additional fixed preference probes. A previously unseen
fixture used for repair is development evidence thereafter. Never relabel its
later pass as blind generalization.

The later REQUEST_MOOD_AND_INFORMATION_GAPS.md increment adds bounded addressed
question-form requests and contrast/imperative scope. Unrestricted indirectness,
long-term style application, affective responses, some correction chains and
conditional reasoning are not established by these changes. The explicit current
dialogue goal therefore remains active until broad natural conversation is
supported and verified.

Current state schema: 40. Response schema: 31. Use a new session; no automatic
state migration, service restart, commit, push or recursive self-modification.
