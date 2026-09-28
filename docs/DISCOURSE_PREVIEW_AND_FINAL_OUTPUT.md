# Discourse meaning preview and final output

Discourse answer schema 24 separates the diagnostic view of a response batch
from the actual reply. This change belongs to the language adapter; semantic
concept payloads and source records are unchanged.

## Consumer contract

- Display `ConversationTurnResponseIR.output.text` to the user.
- `DiscourseAnswerIR.realized_text` is a legacy diagnostic field, not a final
  reply. For ordinary response batches it is now a deterministic projection
  of selected role values, claim values, or disposition. It may not be a
  grammatical sentence.
- Reason from the typed query, projections, claims and source evidence, not
  from that diagnostic field. Unspoken source details remain in evidence IR.
- Standalone recaps, information-target gaps, decision inquiries and world
  responses use a typed-variant marker and source query as their diagnostic
  preview. This marker is neither a sentence draft nor semantic authority.
- Conversation state schema 96 and response schema 87 identify this version.
  No persisted state migration or deployment is performed by this change.

## Runtime path

The discourse engine selects answers and their allowed supporting details.
Batch validation checks the typed meaning, query ownership, evidence, response
constraints and canonical diagnostic projection. It does not realize ordinary
batch answers into sentences.

The natural-realization owner applies the current discourse and affective
policy and constructs each answer once. The completed reply receives one final
sentence check. Two independent answer obligations may therefore require two
generation calls; that is composition, not regeneration of the same answer.

No persistent sentence cache, input/answer lookup, trusted caller flag, or
post-generation string repair is introduced. Serialized diagnostic previews
are recomputed from their typed fields during validation; editing a leaf and
the parent preview together does not make invented prose valid.

## Structured response boundary

Standalone recaps, information-target gaps, decision inquiries, world-reference
clarifications, memory updates and world decisions no longer construct sentences
during answer assembly or validation. Typed source/proof checks remain active.
World-clause selection is shared by non-generative lexical/bounds preflight and
final realization; absent expressions still reject unsupported language output.
Event recaps retain their finite-form/language availability check.

The existing answer-focus record also stores recent clarification-act meaning
and bounded abstention evidence. It is committed with world state in one validated
update. Later reference selects this act before generation; it never reads prior
output as evidence or regenerates competing candidate sentences.

This is a latency and ownership improvement, not proof that broad natural
conversation or GPT-level understanding is complete.

## Morphology before response-goal selection

Utterance-intent version 2 consults the shared indexed lexical morphology before
classifying a Korean WH + deliberative ending as advice-seeking. Lookup version 3
composes supplied past principal forms with recollective/interrogative endings
(-지, -죠, -을까, -을까요), including vowel contractions. Past evidence excludes
the recommendation route, letting the existing event-role query read memory.
The same lexical forms are available to content parsing; no sentence answer,
new world fact or action-effect definition is added. Prior explicit unresolved
tense exclusions remain in place for predicates lacking indexed evidence.

This corrects a route error, not free-dialogue completion: open probes also
exposed unresolved preference-vs-condition, personal-cause and social-intent
handling. Their outputs are retained in the repair report rather than hidden
by the passing recollection regression.

## Multiple roles, one source event

Before generation, a validated multi-role projection may select the existing
event-clause realizer. The clause includes only requested bindings and exact
given arguments in the parsed question. Unrequested adjuncts are omitted before
lexical-node construction. If this would omit a core argument, cross a source
event or perspective boundary, or require unsupported language/finite morphology,
the original role-projection path remains available. No generated strings are
joined, repaired, or reused as evidence.

Subject/object gaps therefore share one source-preserved predicate and one
attribution phrase. Negation stays with that predicate. The original event and
answer claims are unchanged; this is not a request for elaboration or a promoted
world concept. The English role parser also treats a terminal bare object WH
after a subject WH as a typed object gap, rather than an object named `what`.
This bounded production does not claim general embedded or multi-clause WH parsing.

## Wish-complement scope before response selection

Modal graph schema 2 interprets a bounded Korean past-connective plus desiderative
matrix head as Desire/Wish, not an actual fact or execution permission. Conditional
analysis excludes only that complement's token span; a separate outer condition
remains hypothetical. The same grammatical match supplies an ephemeral connective
form to existing interaction-mode lookup (for example, contracted past to
connective). Original source evidence is unchanged. This replaces action-specific
wish exceptions and does not add an output owner, sentence cache or repair loop.

This handles controlled terminal wish constructions, not unrestricted quotation,
reported speech, indirect requests or personal causal recall. Final generation and
sentence validation remain single-pass; typed evidence checks are still required.

## Source-attributed property reasons

Lexical lookup version 4 indexes stative quotative morphology and connective
cause-or-sequence morphology from the existing dictionary's principal forms.
No dictionary sense, promoted concept, or world fact is added by these forms.
For a bounded state-to-state Korean -아/어서 construction, source-backed adjective
readings allow the proposition compiler to retain the speaker's causal relation.
Verbal/ambiguous lexical readings do not license this interpretation. This is
attributed language interpretation, not independent proof of causality.

A first-person recollective reporting question separates reporting source from
the quoted property. Indexed lexical identity links the quotative and finite
forms before retrieval. Retrieval selects the latest matching source record,
not the latest record with a convenient cause. Missing causes remain missing;
unrelated interaction preferences and other properties cannot supply them.

World-memory acknowledgements previously activated the common question-answer
ownership signal and thereby suppressed attributed observation. A validated,
assertion-only world update now permits the normal source-report ingestion path.
This distinguishes output ownership from observation: world memory and discourse
history retain the same source utterance, with no extra reply or truth grant.

The existing shared-recall framing can express this source-bound answer without
repeating its role label. It preserves the same evidence and is selected before
generation. There is no generated-text repair or additional sentence check.
The first implementation is a bounded grammatical interpretation, not unrestricted
reported speech, arbitrary causal inference, or a claim of natural-dialogue completion.

## Lexical nomination and affect are not response authority

Native circuit version 4 checks indexed nominal morphology before nominating a
Korean action label. An occurrence inside a subject/topic/genitive noun or a
dictionary compound is not itself an imperative. Bare nominal requests remain
available, and independently parsed governing requests retain their own force.
This prevents a noun such as explanation in a state report from creating a live
explanation goal and suppressing observation.

Generation version 15 / natural realization version 30 no longer construct a
repeated situation, recent failure, or investigation invitation from an affect
kind alone. The affect graph contains only the current-situation referent and
the expressive quality. A diagnostic action needs its own task/evidence source;
it cannot be smuggled in by an empathy response. The old full-sentence golden
test that required the invented failure was corrected and replaced with checks
across all five supported affect kinds. No generated-string repair was added.

These changes preserve a single final sentence check and do not claim complete
emotion-scope understanding or broad natural conversation.

Routing, primary affect selection and optional affect realization now consume
one expressed-affect selector. It masks quoted spans and distinguishes local
English/Korean negation from an affirmative cue, including focus particles on
Korean -지. Negation in a different causal argument does not erase the affect
predicate. Continuous field weights remain non-authoritative tone estimates;
they do not license an affirmative empathy claim about a denied emotion.

## Problem content does not manufacture a request

Native circuit version 5 removes the problem-word detector that populated an
unresolved request merely because a statement contained a negative state.
An absent action is not a missing action request. Actual unresolved references
and requested-operation bindings still require clarification. This removes an
output-policy owner rather than adding another state-specific suppression rule.
The single final sentence check is unchanged.

The current acknowledgment is still minimal. Open execution probes show that
subsequent queries for a state bearer or a previously mentioned cache can fail
retrieval. This change does not establish state-property understanding, rich
acknowledgment, or natural-conversation completion.

## Source-attributed state relations

Conversation state 97 / response 88 / discourse answer 25 distinguish event
descriptions from state descriptions. Both use the existing attributed
proposition store and evidence-checked projection path. A state contains a
dictionary-backed predicate identity, a Theme bearer and explicit polarity.
It grants neither an agent/action nor independently established world truth.
Replaying the source verifies the kind and role values; event perspective
transformations cannot convert a state into an action.

The initial full-consumption grammar covers Korean explicit subject/topic
noun phrases (including genitives), finite adjective forms and local negation,
and English explicit copula plus a dictionary-backed adjective. Querying the
bearer uses the same lexical identity, including Korean quotative/recollective
forms. No property-name list, input-answer mapping or new output owner is added.
Existing role-value realization constructs the answer once, followed by the
existing single final sentence check.

Missing bearers, embedded reports, unconsumed scope and unsupported forms do
not receive a state interpretation. Person-selecting WH questions require
entity-type evidence not supplied by this initial relation; they cannot use
an untyped bearer as a person answer. State-property questions in the reverse
direction, general anaphora/animacy and rich acknowledgment remain unfinished.
This is product-path progress, not completion of natural conversation.
No persisted-state migration, service deployment, commit or push accompanies
this schema change.

## State meaning selects response content and grammatical form

Conversation state 98 / response 89 / discourse answer 26 add a Property
projection to the existing state description. A bearer query selects Theme;
a property query constrains the bearer and selects the source predicate.
The property keeps its finite form, copula, tense and local negation. A question
about the property does not presuppose positive polarity. Property projection
uses source-validated state clauses, not the nominal value-plus-copula path.

Generation 16 / natural realization 31 share the existing clause generator
between attributed state answers and current-turn acknowledgments. State
bearers receive subject case rather than action-object case. The dialogue
layer selects Inform versus Acknowledge before generation. Korean receipt
endings are assembled from indexed present/past stems and preserve negation;
English acknowledgments retain the source finite clause. No answer-string
lookup, second draft or additional final sentence check is introduced.

Current-turn acknowledgment selection accounts for the terminal assertion
punctuation omitted by clause observation. It requires the same complete
utterance, current observation turn, user attribution and active actual-world
report. Matching a fragment, stale topic or reported quotation is insufficient.
The state remains an attributed report, never independently established truth.

Fresh recombination probes exercise inverse property questions, past forms
and negation in Korean and English. They are not independent blind evaluation
or proof of unrestricted conversation. English determiner casing after an
introductory phrase remains rough; general anaphora, person typing, cross-language
state-clause translation and broad pragmatic response choice remain incomplete.

## State reference and discourse-center ownership

Conversation state 99 / response 90 / discourse answer 27 center a state report
on its typed Theme bearer instead of a legacy subject-signature fallback. The
fallback previously selected a negative auxiliary as the topic. Referential
bearers are not treated as literal topic names: source-bound binding resolves
them before applying the named-topic constraint.

State reference retrieval reads the existing discourse center across social
turns. Newer centers outrank stale answer focus. A proposition center is tied
to its observation turn; it cannot select an older available state merely
because a new unavailable report has the same subject. Simultaneous state
introductions retain ambiguity instead of letting salience ranking select one.
Explicit topic changes cannot borrow an unrelated old state.

Known reference words share a productive Korean topic-contraction rule. It
uses the existing controlled reference lexicon, not an assumed general-vocabulary
entry or an input-answer table. Clarification alternatives reconstruct source
questions with the appropriate topic particle and replay their role bindings.
Displayed options are selectable verbatim; relaxed nominal matching normalizes
both sides and rejects collisions rather than choosing the first option.

Fresh probes cover social continuity and ambiguity followed by user selection.
Place/person typing is still incomplete: a state bearer alone does not prove
that it is a place, and a place-selecting reference can still produce an
inappropriate generic source clarification. This remains a known failure,
not natural-conversation completion. The final sentence check remains single.

## Definition-backed referent constraints before generation

Conversation state 100 / response 91 / discourse answer 28 let a State Theme
participate in person/place reference when the existing immutable dictionary
supplies unambiguous nominal-definition evidence. The dictionary's terminal
genus (person, place/space) constrains the referent; topic categories are not
types. Korean lemmas and English aliases use the same source sense IDs. Unknown
or incompatible nominal senses abstain, and arbitrary compound heads are not
guessed. No noun-specific type list or new semantic concept is introduced.

The role binding carries the dictionary hash and source-sense/head evidence.
Source replay checks it along with the selected bearer before realization.
Known controlled referential noun phrases are now admitted by the State parser;
this reuses reference knowledge rather than creating a second pronoun owner.
The same type constraint serves source-context compilation and question binding.

Meaning is selected before a single composition pass and one final sentence
check. The realizer is not retried to compensate for missing type knowledge.
This bounded definition grammar does not establish general taxonomy induction,
unrestricted ellipsis or natural conversation. In particular, a failed type
binding can still fall back to an overly generic clarification. No deployment
or persisted-state migration accompanies this change.

## Attributed state meaning outranks affect cues

Conversation state 101 / response 92 / discourse answer 29 preserve a complete
State description as response content. The shared affect-response selector
does not replace that attributed bearer/predicate with an emotion sentence.
The continuous tone field remains available for delivery policy. English
fatigue words no longer nominate frustration. The cognitive pipeline reuses
one affect selection for its routing and response candidates.

English copular interpretation reads exact source English equivalents as well
as adjective entries: a Korean verb may have a `be + complement` English
expression. This is language-specific syntax, not a new semantic event effect.
A verbal participle alone does not license the new copular-alias path, so
passive event interpretation is not manufactured from the prefix `be`.

Generation 17 / natural realization 32 preserve source memory and apply the
speaker-to-addressee perspective during State clause composition. First-person
singular user reports become second-person expressions with corresponding
English copula agreement; source tense and negation remain. The generation
graph records the deictic grounding path. English determiner casing is decided
before sentence generation, without lowercasing arbitrary names.

Existing source-validated world-memory acknowledgments retain ownership where
they already apply, including licensed Korean subject omission. No extra draft,
regeneration or final sentence check is introduced. These changes do not resolve
arbitrary implicit experiencers, embedded reports, general pronoun-sense
disambiguation or unrestricted natural dialogue. No deployment or persisted
state migration is performed.

## Attribution spans and typed question variables

Attribution graph 2 retains source declarative endings when removing Korean
quotative -고. The source substring now remains a finite proposition rather
than an unparseable predicate stem. Conversation state 102 / response 93 /
discourse answer 30 pass those propositions into the existing attributed
state store and projection path. A single, non-nested, named-speaker report
can be expressed with its source via the existing clause generator. Denial,
doubt, unresolved embedded deixis and other unsupported stances do not license
an affirmative report acknowledgment.

The affect-response selector masks existing attributed proposition spans,
preserving independent matrix affect outside them. Reported emotion is not
the current speaker's affect authority. This uses the existing attribution
analyzer, not a second report-verb inventory or a sentence-specific route.

State WH questions carry a bearer-kind constraint. Compatible existing
dictionary senses may satisfy an explicitly person-selecting question or
reference; absent senses cannot. This distinction allows contextual selection
of the teacher sense of a polysemous noun without making an unconditioned type
assertion. The original pupil/person probe remains a product failure: this
dictionary contains only pupil's eye-anatomy sense. Its abstention regression
protects against invented knowledge and is not counted as naturalness success.

Native planning and world atoms share the existing WH-placeholder vocabulary.
Typed open state bearers also remain query variables before Korean case
stripping; they cannot become entities named who or its malformed inflection.
Denied/doubted proposition records cannot supply affirmative state-role
answers. Questions about who reported something remain owned by the source
ledger, distinct from questions about its state bearer.

Generation 18 / natural realization 33 retain one composition path and one
final sentence check. No dictionary content, promoted concept, deployment or
persisted-state migration is changed. Nested reporting chains, missing lexical
senses and unrestricted conversation remain unproven.

## Proposition-query ownership of anaphora

Source queries reserve their arguments before generic entity replacement and
task ellipsis. The existing event-query and source-query operators determine
this ownership, not a stored whole-question response. The cognition path
consumes the original query and reuses its selected answer rather than letting
a later entity substitution choose a different proposition.

Under the source-query operator, lexical anaphors such as that/it/so and
그것/그거/그렇게 select a proposition argument. The existing contextual-record
selector binds that argument to live answer focus or a unique current record.
The same selector preserves stale, retracted and simultaneous-candidate gaps;
it cannot pick a referent simply because one candidate has a convenient answer.
An explicit content argument does not inherit the previous question's topic.

The existing contextual-target receipt records the selected belief ID and
Source slot. Reported, believed, negative or modal content can have a recorded
source without its content becoming true. Source lookup therefore uses record
availability rather than the affirmative-content filter. Grammatical function
words remain intact before noun-case stripping: a WH form cannot be shortened
into an invented topic keyword.

No response template, regeneration loop, additional final sentence check,
dictionary alteration or new semantic concept is added. This fixes reference
ownership, not unrestricted discourse understanding or the remaining verbose
source-ledger realization. Existing serialized shapes are retained; there is no
deployment or state migration in this change.

## Question focus before sentence composition

A proposition-source query with matching evidence for one shared proposition
selects source-actor values in the discourse layer. Its answer no longer
automatically repeats the embedded proposition and a not-fact disclaimer.
The selected meaning is source identity within the dialogue, not truth of the
reported content. The existing evidence and claims remain unchanged in IR.

Content-role and source-identity answers share the same grounded value-set
generator, noun-phrase coordination, Korean copular endings and English phrase
realization. Each actor value retains its belief IDs in the generation graph.
The dialogue user is expressed as the current addressee without changing the
stored actor. No finished reply is rewritten or cached.

Different propositions, inactive records, denied/doubted attributions, missing
evidence or mismatched claims do not license this reduction. Their qualification
or source/content association stays on the existing full-information path.
Content questions and truth-status questions keep their distinct response
meanings. One generation pass and one final sentence check are preserved.

## Shared proposition focus and source membership

Conversation state 103 / response 94 / discourse answer 31 separate a typed
proposition under discussion from the number of its attribution records.
`SharedPropositionFocusIR` holds one `PropositionMeaningIR` and the supporting
belief IDs. The descriptor comes from the existing full-consumption description
parser and source-validated content: description kind, predicate sense IDs,
finite predicate form, roles, negation, modal world and proposition polarity.
Source event IDs and sentence punctuation do not define the shared descriptor.
This is dialogue reference state, not a newly promoted semantic concept.

The finite predicate form remains a conservative tense/aspect guard; this is
not unrestricted paraphrase or cross-language equivalence. Context-dependent
participants and partial/compound descriptions without a complete resolved
description do not acquire a shared identity by string resemblance. Existing
speaker/addressee vocabulary and reference analyzers protect this boundary.

Source questions bind the current proposition before querying its available
source records. Membership is refreshed from the bounded live ledger, so a
retracted source is excluded without erasing the remaining sources. A social
backchannel preserves the existing bounded focus lifetime; new content and
stale focus do not revive old topics. The legacy target belief ID is only an
anchor for compatibility, never a preferred source or truth winner.

Shared-reference receipts replay every membership against the typed source
description. Response validation checks the eligible source set, preventing
silent omission of a source. Neither reference identity nor repeated reports
establish truth or authorize execution. Sentence generation and final checking
remain single-pass. No deployment or persisted-state migration is performed.

## Omitted speech content and attributed clause realization

The same quotative-content grammar supplies question force and reserves the
proposition argument before generic entity/task ellipsis. Korean WH nouns with
their quotative allomorph and optional polite marker, and bounded English
report-verb/WH fragments, select the Summary slot of the current proposition.
Punctuation is not required to turn these grammatical requests into questions.
The grammar consumes the whole fragment; it does not match arbitrary embedded
quotes or dispatch a stored answer sentence. Missing or ambiguous memory remains
unresolved. Shared-proposition membership and qualification checks still apply.

An answered speech-content query can expose its selected evidence as attributed
event clauses. This requires exact claim/evidence agreement, active actual-world
reported content, a fully parsed description and no unbound participant. The
existing event-summary generator composes those clauses from their roles and
predicate forms. Denied, doubted, modal or incompletely parsed content retains
the full-information path rather than losing its qualification. Reported content
does not become established truth or an execution instruction.

This adds no answer rewrite, generation retry or extra final sentence check.
The API regression instruments eleven Korean/English turns before independent
validation replay. Serialized shapes and dictionary payloads are unchanged.
The bounded grammar is not unrestricted ellipsis or general conversational
naturalness; those remain unestablished.

## Observed expression versus normalized identity

Input normalization keeps the existing lowercase semantic token stream and a
separate observed-spelling surface. Filler deletion, accepted token repairs,
width normalization and explicit self-repair apply to both aligned views.
Unchanged tokens keep their observed spelling; a repaired token uses the
accepted replacement. A failed alignment never invents the discarded spelling.

Composition may consume the observed token view only when it is case-equivalent
to the semantic input. It cannot undo reference resolution or overwrite a
different semantic stream to recover typography. Attribution grammar still
operates on normalized coordinates, but carries the selected actor start span.
A Unicode-aware boundary map recovers its original spelling while actor identity
remains normalized. Interior boundaries of a lowercase expansion are rejected.
Attribution records and reference receipts retain their observed source surface.

Same-source event context and source-value grouping compare normalized identity,
so a casing change does not create a new source or break antecedent memory.
Receipt replay still compares the exact source snapshot against its record.
The noun-phrase realization step handles sentence-position determiner casing
without erasing internal name capitalization. It does not rewrite a completed
reply. Generation and final sentence checking remain single-pass.

This protects the attribution-source path, not every participant expression in
every existing parser. Unrestricted lexical interpretation and conversation
remain unproven. No serialized field, canonical concept, dictionary payload,
deployment or persisted-state migration changes in this repair.

## Nominal sound evidence belongs to expressions

Attribution graph 3 / conversation state 104 / response 95 / discourse answer
32 / generation 19 / natural realization 34 preserve observed Korean nominal
forms as adapter-local lexical evidence. They are not promoted concepts or
changes to the meaning graph. Hangul spelling supplies known final-consonant
status. Non-Hangul spelling alone supplies no such status.

The attribution parser can retain the complete selected actor token with its
subject/topic allomorph. Bounded typed-entity memory keeps one witness for each
observed coda class. Duplicate supporting forms do not inflate evidence; opposite
classes coexist instead of the newest one silently replacing the old one.
This infers only the final-consonant distinction needed for paired suffixes,
not a full pronunciation or a consonant identity such as rieul.

Source-identity answers carry matching lexical witnesses into their expression
nodes. Response replay checks the complete available witness set, including
conflicting evidence, so dropping an inconvenient form cannot restore false
certainty. Noun coordination and informal copular answers consult the expression
evidence. With unknown/conflicting coda, coordination uses the invariant 하고
and an informal identity answer can remain a nominal fragment. A name need not
be mispronounced or guessed merely to answer who it identifies. Formal 입니다
does not require this coda distinction.

No full answer sentence is learned, rewritten or regenerated. Removing the
phonological evidence preserves the meaning hash while changing only eligible
inflection. Promoted payloads and the immutable dictionary remain unchanged.
Other legacy string-only particle helpers and automatic acquisition from every
possible entity role are not covered by this repair. General conversational
naturalness and full pronunciation knowledge remain unproven.

No deployment or stored-state migration is performed. Before deploying these
versioned schemas, persisted older dialogue state needs an explicit migration
or replay decision; the schema change does not authorize discarding that state.

## Reference interpretation cannot teach observed morphology

Reference resolution now consults typed-entity nominal-form memory when choosing
Korean subject/object case forms in its legacy semantic-parser bridge. Composition
also receives a separate original observation surface. Attribution can acquire a
nominal witness only when its complete span lies in an unchanged prefix or suffix
of that observation, not in the rewritten middle. The linear envelope is computed
once; it adds neither a second parse nor another generation or final output check.

This prevents an inferred name-plus-particle from becoming evidence that the user
pronounced the name that way. Actual later observations can still teach the form.
Unknown coda still uses the legacy vowel-form case marker inside the parser bridge;
that convention is not pronunciation evidence and cannot authorize final output
inflection. Multiple separated rewrites can conservatively exclude otherwise
unchanged middle spans. Full edit-lineage and a fully typed parser bridge remain
outside this repair.

Fifteen instrumented API turns preserve the original witness set with exactly one
generation and one final sentence check per turn. No serialized schema changes,
deployment, persisted-state cleanup or migration are performed. Previously polluted
stored witnesses cannot be silently removed without a separate provenance decision.

## Relation questions bind a proposition before reasoning

A nominal relation question is decomposed into an open relation slot and its
complement clause. Korean interrogative-pronoun/copular endings and an English
reason-head complement construction supply the syntax; neither stores a reply.
The world path accepts the cause slot only, grounds the complete complement with
the current lexical revision and sets the existing WorldQueryIR explanation bit.
It then uses the same core deliberator and utterance plan as a direct why-question.

Attributive morphology is an explicit parsing mode, not a global rewrite of a
fragment into an assertion. Current Korean state 한 / 하지 않은, relational 하는 /
하지 않는 and copular 인 / 아닌 forms are interpreted through the registered
lexeme's grammatical type. Finite premises retain their existing grammar.
The contracted subject form 누가 retains its open-variable identity instead of
becoming a fabricated entity 누 after case stripping.

Questions add no premises or causal mechanisms. A known state without a mechanism
licenses the existing cause-unknown speech move; removing evidence or a required
mechanism cannot leave a supported answer. Quoted, partially consumed, historical,
hypothetical and unbound complements remain outside this bounded world path.
The same construction is tested with newly registered opaque roots, not just
preinstalled state words. No output sentence construction or retry is added.

The speech planner distinguishes supported targets from refuted targets before
selecting that cause-unknown move. A refuted explanation presupposition produces
the supported conclusion/correction rather than silently changing the question
to ask why the opposite state holds. This rule applies equally to direct and
nominal why-questions; it is independent of Korean or English output strings.

This is not general subordinate-clause understanding or unrestricted state recall.
The diagnostics still expose unsupported open property questions and statements
containing an embedded why-clause. They remain failures, not passing exclusions.
No schema change, canonical mutation, deployment or stored-state migration occurs.

## Open property questions separate information heads from their owners

Nominal state questions compile directly to the existing State description with
an open Property slot and a bound Theme/owner. A state/condition head denotes the
requested information category, not an entity name or a search token. Korean
topic/subject constructions and genitive owners, and English state/condition-of
constructions, share this result with the existing how/어떻다 query path.

The category heads 상태/state/condition and copular morphology are supplied
language knowledge, not autonomously discovered concepts or answer sentences.
The query grammar leaves other categories, coordination, negation of the head,
and additional clauses unconsumed. It does not use an answer rewrite or reparse
a replacement question. A typed open-property description establishes question
ownership even without a final question mark; a finite state assertion does not.

The existing attributed-memory matcher chooses the source record and property,
and the existing generator realizes it. No new query kind, memory format, semantic
predicate, promoted payload, dictionary entry, sentence template, or final check
is introduced. Regression compares exact property bindings and source belief IDs
across the expressions, checks that query turns add no belief records, and removes
the memory to verify that language alone cannot supply the answer.

This fixes the bounded open-property failure noted above. Embedded-why statements,
arbitrary possessive/relative-clause parsing, and spurious UNKNOWN referents from
other parser paths are still unresolved. Whole-conversation naturalness remains
unestablished. Package synchronization is not deployment or state migration.

## Missing evidence does not erase the understood question

Discourse answer 33 retains an optional `described_query` alongside the result.
It carries the parsed question, not an assertion or an observation. Receipt
validation replays it against the original question; a missing-record answer
cannot omit or substitute a different parsed question. Successful retrieval can
retain the same structure while supplying a source-bound property value.

A bound open-property question remains owned by that path even in empty memory.
It does not become a presupposition failure merely because no descriptive records
exist. For an unavailable property, the meaning builder selects an epistemic
unknown act with separate owner and property-category nodes. Korean genitive/topic
morphology and an English owner relation then compose the reply. Search terms do
not participate in that branch; changing them cannot change the expressed target.
Generation consumes the retained query rather than parsing a replacement sentence.

This introduces an atomic expression for the unknown act, not a table of complete
answers. It adds no observation, action, plan, generated retry or final sentence
check. The unavailable branch stops being selected once a source supplies the
property. Unresolved references and other missing-information categories still
need their own structurally faithful handling; this is not general conversation
completion.

Conversation state 105 / turn response 96 / generation 20 / natural realization
35 accompany the answer schema. No deployment, restart or persisted-state
migration is performed. Older persisted sessions require an explicit migration
or replay decision before deployment; they have not been discarded or modified.
