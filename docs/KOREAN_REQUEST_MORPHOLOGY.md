# Shared Korean request morphology

This Rust increment advances interpretation -> attributed retrieval -> language
realization. It does not complete natural conversation or change semantic
generations, canonical constraints or recursive-improvement quarantine.

## Implemented product path

Benefactive auxiliary/request endings now have one bounded grammar shared by
directive recognition, modal question recognition and question-argument extraction.
The caller must retain and validate the prefix: locally it must be an allowed
connector; a full request needs a known predicate, with negation/report checks.
A bare sentence ending in `줘` is not sufficient. A cheap suffix check precedes
predicate lookup. No example-specific answer or whole-sentence dispatch was added.

Spacing between a verb/connector and the auxiliary no longer makes the argument
extractor reject the content clause. Register detection includes formal auxiliary
endings, and focused Korean source attribution uses formal wording when requested.
The query, claims and evidence stay identical across 20 ending/spacing/punctuation
variants. Quoted, negative, conditional and mixed-task requests remain excluded
from this positive question-envelope grammar.

## Evidence and failed boundaries

State53 / response44 / discourse answer5. Adapter tests 702 + core tests 29 = 731
PASS. Six CLI regressions, fmt, clippy and API build PASS. Canonical 10-file
manifest unchanged. Previous AA now has 7 full / 0 partial / 1 failed semantic
requests: `알려주세요` directly answers the actor in the requested register.
Original U and R output surfaces are unchanged.

Fresh AB: 8 requests / 16 turns; 4 full / 1 partial / 3 failed. Developer-authored
and scored; full-surface overlap search found none. Not independent blind testing
or a naturalness percentage. Source/executable hashes stayed frozen after scoring.

- AB04: a valid inner object question fell through to generic source retrieval,
  interpreting the event actor as requested_source; known object was not answered.
- AB06: correct missing-evidence topic, but the gap reply ignored polite register.
- AB07: the positive request parser correctly rejected a prohibition, yet the
  conversation contract treated its wh word as an information request and generic
  retrieval revealed the actor. This is a real user-instruction boundary failure,
  not a passing abstention. Parser-only negative tests did not cover this fallback.
- AB08: third-party communication still becomes a plan about the content rather
  than recipient-bound message preparation. No external delivery occurred.

The next priority is end-to-end response suppression and typed query ownership:
negative matrix intent must outrank embedded question words; failure of a typed
event query must not reinterpret the actor as an attribution source. Do not patch
these particular fixture strings. Gap register and recipient composition remain.

## Deployment

Eight changed Rust files and this/acceptance documents are mirrored into pakage;
hash parity is not an independent package build or release certification. Use a
fresh conversation session. No commit, push, service restart or state migration.
Preserve failed observations and unrelated package changes. Goal remains active.
