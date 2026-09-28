# Explain a retained decision question

The existing answer-focus owner now retains the semantic decision inquiry it
actually emitted. It stores the missing input and optional proposed-action
description, not generated response text. An explanation carries a bounded
origin question/turn receipt. Repeated follow-ups keep that original receipt;
they do not build recursive memory or renew the three-turn origin lifetime.

Selection occurs before world preparation. A question about why the assistant
asked must not accidentally prepare a query about an old world property. The
response meaning expresses a dependency: evaluating the proposal/decision
requires the missing input. The existing final generation path realizes that
dependency with the configured register. It neither repeats the information
request nor invents a beneficial outcome.

Bare causal WH is not always about the latest assistant question. A generic
decision-input request that has no proposed action retains the existing world
question under discussion. Explicit references to the assistant's asking or
thinking act can select the retained inquiry. This boundary was established
after an unchanged old world-explanation test caught an overbroad priority
change; the failed run is preserved in the report.

Same-context social backchannels may bridge the existing short lifetime. Topic
changes and other output owners clear focus; expired, quoted, third-party,
explicit-world and compound-action reason requests cannot reuse it. Input
and origin replay, response/focus equality, source-memory equality, output
tamper checks, one generation and one final check remain enforced.

Tests cover Korean/English, formal/informal expression, generic and action-
specific inquiries, social bridging, expiry and topic change, while preserving
the previous world's causal follow-up behavior. This adds conversational
continuity, not common-sense action-benefit knowledge. Whether resting helps a
tired person, sense/argument disambiguation and broad natural conversation
remain unestablished requirements.

Schemas: utterance intent 4, discourse answer 41, natural realization 40,
conversation state 116, conversation response 112. No runtime deployment,
state migration, service restart, canonical mutation, commit or push.
