# Personality and expression selection

The public Rust cognitive API accepts `SET_DIALOGUE_PERSONALITY` before or
between conversation turns:

```json
{
  "operation": "SET_DIALOGUE_PERSONALITY",
  "personality": {
    "warmth_millis": 600,
    "playfulness_millis": 0,
    "formality_millis": 0,
    "concision_millis": 0,
    "korean_dialect": "STANDARD"
  }
}
```

Each numeric value is in 0..1000. `korean_dialect` is `STANDARD`, `GYEONGSANG`
or `CHUNGCHEONG`. Invalid input returns `INVALID_REQUEST` and leaves
the previous configuration intact. The default is zero on every axis. This
configuration belongs to the API instance, persists across its turns and
conversations, and must be reapplied by the host after a restart. It is not
inferred from a user's sentence or stored as a world fact.

The path is:

1. Select the answer meaning and its evidence through the existing reasoner.
2. Combine the configured expression tendencies with the bounded observed
   affective field. Current urgency or frustration suppresses playfulness.
3. Select lexical expressions for the same concept IDs using language,
   register, confidence, optional expression-level affect affinity and optional
   Korean regional affinity.
4. Assemble syntax and morphology once, including register-sensitive endings.
5. Check the completed realization once. Do not rewrite and retry the answer.

Personality does not change truth, confidence in a claim, source attribution,
semantic concept payloads, action permissions or execution state. Explicit
response-length directives retain priority over an inferred brevity preference.
The response records the personality and the effective affective policy so
their relationship can be checked. Inferred user affect is not a claim that
the core experiences human emotions.

`ExpressionNodeIR.preferred_emotion` is optional lexical metadata. Selection
first restricts candidates to the same concept and language. Exact register
match outranks a neutral fallback, which outranks an incompatible register;
each gap exceeds the affect-affinity range. Unmarked expressions remain available. Ties are
deterministic, so identical meaning, lexical store and context reproduce the
same output. Different context or personality may yield different valid
expressions; randomness and whole-answer alternative lists are unnecessary.

Regional style is carried as explicit generation evidence. Gyeongsang and
Chungcheong social lexemes and finite endings are selected before the final
sentence is joined. Every token keeps its original meaning-node ownership;
regional style cannot select another concept, polarity, participant or speech
act. English generation always records `STANDARD` and ignores the Korean-only
setting.

The same regional morphology is recognized in the input direction. Common
finite endings such as `합니더/합니꺼`, `해유/하나유`, regional copulas and
social expressions normalize into the existing semantic grammar while raw
source text remains available. This is a bounded initial inventory, not a claim
that every regional subvariety or speaker uses one uniform form.

Korean source-attributed state/event replies re-inflect an indexed predicate
stem for formal register instead of blindly copying the source ending. Past
stems and local negation remain attached to the source meaning. Unknown
morphology keeps the observed source form rather than guessing a new form.

## Evidence and limits

Regression compares actual public output diversity as well as exact meaning,
answer evidence, speech intent and action boundaries. It also checks persistence,
invalid configuration, suppression under urgency, opaque lexical variants,
negative/past source predicates, and generation/final-check invocation counts.
These are development regressions, not a blind naturalness benchmark.

The built-in warm alternatives cover greetings, hold acknowledgements,
gratitude replies and farewells. Regional variation covers those social acts
and a bounded finite-ending inventory. Register variation also reaches
source-attributed Korean state/event answers. This does not establish varied
wording for every concept, complete regional grammar or unrestricted natural
conversation. Missing expressive vocabulary remains explicit.

The current generation, frontend and conversation response schema constants
identify this behavior. Conversation-state storage is unchanged. No deployment,
restart, automatic state migration, commit or push is performed by this change.

## Shared register across constituents

Clause-initial world-memory acknowledgements use the same acknowledgement stem
and speech-ending/conjugation functions as standalone acknowledgements. Remember,
conditional and compound-conditional modes no longer embed a fixed informal
receipt. This applies before token emission, not as a completed-text replacement.

Social interjections have explicit formal lexical alternatives under their
existing concept IDs. A current-user attribution can select a formal nominal
expression under the same source-bound node; a named third-party source is not
renamed. The help-offer predicate likewise selects the honorific `도와드리`
lexeme before shared future/question morphology is applied, keeping a formal
greeting and its following clause at the same register. The register selector,
not per-input dispatch, chooses these variants. Strong warmth or playfulness
cannot make an incompatible informal alias beat the selected formal register.

Response-boundary validation also binds each generation context to the effective
policy: a requested formal register must remain formal, and urgency must agree.
This checks typed fields only; it does not generate, rephrase or revalidate the
completed sentence a second time. A self-consistent personality/policy pair
cannot conceal an incompatible generation context.

This protects the tested social/receipt/source constituents. It is not proof
that every legacy expression or every first-/second-person reference has complete
register agreement. The system still has limited expressive vocabulary and
unrestricted conversation remains unestablished.
