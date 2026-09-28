# Proposal roles and dictionary sense frames

Status: implemented and regression-tested, 2026-09-07. This is not completion
of natural conversation or action-benefit reasoning.

## Product change

The existing decision-request route now carries the proposed action's typed
event roles and polarity, not only an opaque phrase and a bag of entry IDs.
`proposition_content` shares one role parser across reports, questions and an
explicitly non-finite proposal mode. The public report entry point retains its
old mode: an infinitive or prohibition is not newly accepted as an observation.
The proposal wrapper never inserts a default actor or a fictitious executed
event into conversational beliefs or the action ledger.

`ProposedActionIR.frame_candidates` joins the predicate's indexed dictionary
entries to their source sense IDs and syntactic patterns. A small grammar of
numbered arguments and case particles compiles required roles. Missing and
incompatible roles remain explicit. Partially understood patterns remain
unparsed; empty role lists on those candidates do not mean unconstrained valency.
English aliases are sense-local, so an entry hit does not import unrelated
translations. Korean headword senses remain alternatives. The result is bounded
to 128 candidates and does not scan the dictionary or world concept catalogue.

The same source sense (NIKL 71280/1) has the same frame on Korean and English
paths. Its intransitive pattern is incompatible with an explicit Theme; the
separate transitive senses are not erased. Frame fit alone does not select a
world meaning or prove an effect. In particular, an omitted Agent stays missing.

The final generator identifies a proposal from predicate alternatives, polarity
and role bindings, with source/role grounding references. The surface is still
a source-language mention of the user's action, not a generated event assertion
or an independently translated phrase. It preserves case, articles, prepositions
and focus markers that the current event roles do not fully represent. Complete
action-phrase grammar generation and cross-language disambiguation remain open.
No finished output is repaired or regenerated.

## Circular-regression acceptance

- Old assertions are unchanged, including world-cause versus inquiry-reason
  ownership, reference expiry, quotation, observation and execution boundaries.
- New tests check role/case preservation through a reason follow-up, Korean
  argument order, local negation, missing actor, sense/frame alternatives,
  source replay against forged roles and non-finite/report separation.
- Instrumentation checks one generation and one final realization check before
  the test independently revalidates the response.
- Native cases use different names from the Rust tests. They are developer
  regressions, not an independent blind benchmark or a naturalness score.
- The predecessor binary fails the new typed-frame contract because it lacks
  the fields; that count is not twelve newly fixed conversational answers.
  Its lowercasing of English argument names is an observable output defect.

## Verification and remaining work

`cargo fmt --all --check`, adapter-library Clippy with warnings denied, 895
adapter-library tests and the native API build pass. The native proposal suite
passes 6 conversations / 12 turns; the existing information-gap, continuity and
event-reference suites pass another 217 turns. These results protect tested
paths, not every possible utterance.

Action-benefit judgement is still incomplete: the system continues to ask for
expected benefit. Next work must bind the actor and a justified lexical sense
to explicit semantic action-effect knowledge and use core deliberation. It must
not replace that step with a canned rest recommendation, infer execution, or
silently turn a dictionary definition into a guaranteed causal fact.

## Compatibility and delivery

Schemas: utterance intent 5, conversation state 117, conversation response 113,
generative language 26. Old serialized proposal receipts are not automatically
migrated; no deployed service or user memory was restarted/migrated here.
Canonical documents, promoted concepts and the source dictionary are unchanged.
The tested scoped source files, this document and the native test script are
mirrored individually into `pakage`, guarding predecessor hashes. Unrelated
package edits are preserved. No commit, push, deployment or recursive source
mutation is performed by the product. Rollback must reverse only this scoped
delta; a repository-wide reset would discard unrelated work.

Evidence: `reports/language-cortex-completion/proposal_role_and_sense_grounding_2026-09-07.json`.
