# Source event assertion and observation ownership

An indexed predicate and a complete source-bound event can establish an
assertion without an executable planning frame. A legacy action cue from a
noun must not override that event meaning. The conversation contract records
SOURCE_BOUND_EVENT_REPORT only when its existing assertion/action/question
guards accept it.

Observation readiness is separate from planning readiness. Accepted assertions
at input confidence >= 900 may enter the existing attributed proposition
history even when the action planner is not ready. Existing ownership exclusions
for questions, definitions, action/result updates, quoted requests, explicit
topic transitions and continuation decisions still apply. Observations remain
user reports, not verified truth or execution authority.

The active event parser also enforces two shared syntax boundaries: finite
lexical-verb negation in a declarative needs do-support, and a determiner alone
cannot fill the agent phrase. This prevents prohibitions from becoming reports
and a failed passive parse from treating its theme noun as an active verb.
Questions consume their fronted auxiliary separately; passive parsing precedes
active parsing. No sentence dispatch or predicate-specific exception is added.

Paired regression checks retain commands, questions, unknown participles,
negative facts and an agent named Do. Positive multi-turn tests check passive
and active assertions, role retrieval, pronouns, unchanged source records, no
action execution, one generation and one final realization check. Original
tests remain unchanged; failed intermediate runs are retained in the report.

No dictionary payload, canonical document or output realization code changes.
Schemas: conversation state 113, conversation response 109. No persisted-state
migration, deployment, service restart, commit or push. Passing these bounded
developer-authored tests does not establish general natural conversation.
