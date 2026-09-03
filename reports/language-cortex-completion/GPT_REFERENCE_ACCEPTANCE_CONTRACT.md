# GPT-Reference Conversational Acceptance Contract

Status: `FROZEN_BEFORE_REFERENCE_GENERATION`

This contract replaces the open-ended phrase “GPT-level language ability” with a finite, measurable stopping condition. Passing this contract ends the current Language Cortex capability campaign. A later expansion requires a new benchmark version and cannot silently reopen this denominator.

## 1. Target capability

Given the same dialogue history, B_Core should make substantially the same communicative decision as a fixed GPT reference answer:

- infer the user's intended response goal;
- retain and restore relevant dialogue context;
- bind references and ellipsis;
- distinguish requests, reports, plans, results, corrections, affect, and clarification needs;
- express only supported claims in natural Korean or English.

Exact wording is not required. Semantic and conversational equivalence is required.

## 2. Fixed benchmark denominator

- Development set: **60 dialogues**.
- Sealed final set: **40 dialogues**.
- User turns per dialogue: **4**.
- Total scored assistant responses: **400**.
- Sealed final responses: **160**.
- Languages: **50 Korean dialogues, 50 English dialogues**.
- The sealed final set is not opened until the development gate passes.

Ten categories contain exactly ten dialogues each:

1. explicit requests and questions;
2. indirect intent and pragmatic implication;
3. correction, rejection, and feedback;
4. pronouns, deixis, and ellipsis;
5. topic shift, interruption, and topic return;
6. conflicting reports, uncertainty, and source attribution;
7. plan, execution, and verified-result separation;
8. affect, gratitude, frustration, and social backchannels;
9. genuine ambiguity requiring a clarifying question;
10. Korean/English mixing, typos, fillers, and speech-like fragments.

Prompt cases, dialogue histories, category assignments, and reference-generation settings are hash-sealed before B_Core evaluation.

## 3. GPT reference policy

- One fixed GPT model and fixed decoding configuration generate the reference answers.
- The exact model identifier, date, system prompt, decoding parameters, and raw response hashes are recorded.
- GPT is used only to produce the frozen evaluation references.
- GPT is not used as a runtime parser, reasoner, reranker, generator, or judge for B_Core.
- After reference generation, scoring is deterministic and implemented in Rust.
- Reference answers are converted once into a frozen annotation containing response act, intended goal, discourse bindings, required propositions, prohibited propositions, epistemic status, and natural-language surface.
- Neither the reference answer nor its annotation may be edited after the first B_Core final-suite run.

## 4. Deterministic similarity score

Each response receives a score from 0 to 1:

```text
score =
    0.35 × meaning_graph_f1
  + 0.20 × discourse_binding_f1
  + 0.20 × required_proposition_f1
  + 0.15 × response_act_and_epistemic_boundary
  + 0.10 × normalized_surface_similarity
```

Definitions:

- `meaning_graph_f1`: overlap of language-independent concepts, predicates, operators, polarity, and goal relations.
- `discourse_binding_f1`: overlap of topic, reference, ellipsis, source, and prior-turn bindings.
- `required_proposition_f1`: precision/recall over claims required by the GPT reference annotation.
- `response_act_and_epistemic_boundary`: exact agreement on answer, question, clarification, correction, plan, result, uncertainty, attribution, or social response; plan/result and fact/report errors score zero on this axis.
- `normalized_surface_similarity`: deterministic maximum of token F1, character-trigram F1, and order-preserving token similarity after bounded Korean/English morphology normalization.

Surface similarity cannot compensate for an incorrect intent, fabricated fact, wrong referent, or false execution claim.

## 5. Development gate

The sealed final set may be opened only when the 240 development responses satisfy all of the following:

- mean composite similarity: **≥ 0.85**;
- median composite similarity: **≥ 0.88**;
- responses scoring at least 0.80: **≥ 90%**;
- intent/context exact agreement: **≥ 95%**;
- every category mean: **≥ 0.80**;
- unsupported reference propositions: **0**;
- false execution/result claims: **0**;
- silent guesses on cases marked ambiguous: **0**.

Development repairs are limited to **three planned rounds**. Each round must name the failed structural dimension before changing product code. After the third round, the development result is reported as PASS or FAIL; the denominator is not enlarged to hide a miss.

## 6. Final acceptance gate

The 160 sealed final responses PASS only when all conditions hold:

- mean composite similarity: **≥ 0.85**;
- median composite similarity: **≥ 0.88**;
- 10th-percentile similarity: **≥ 0.75**;
- responses scoring at least 0.80: **≥ 90%**;
- intent/context exact agreement: **≥ 95%**;
- response-act exact agreement: **≥ 95%**;
- each of the ten category means: **≥ 0.80**;
- Korean category mean: **≥ 0.83**;
- English category mean: **≥ 0.83**;
- unsupported reference propositions: **0**;
- semantic-authority violations: **0**;
- false execution/result claims: **0**;
- silent ambiguity guesses: **0**;
- external LLM calls during B_Core execution: **0**;
- local teacher calls during B_Core execution: **0**;
- recursive source mutations: **0**.

## 7. Stop rule

On final PASS:

```text
LANGUAGE_CORTEX_GPT_REFERENCE_STATUS=PASS
DISPOSITION=CONTROLLED_GPT_REFERENCE_SIMILARITY_TARGET_REACHED
FURTHER_REPAIR_LOOP=STOPPED
```

On final FAIL:

```text
LANGUAGE_CORTEX_GPT_REFERENCE_STATUS=FAIL
DISPOSITION=SEALED_FINAL_THRESHOLD_NOT_REACHED
```

The failed final suite is preserved unchanged. It is not converted into a development suite, rewritten, or expanded. Further work requires explicit user authorization for a new benchmark version.

## 8. Claim boundary

PASS means B_Core is sufficiently close to the fixed GPT reference distribution for the defined Korean/English conversational suite. It does not mean universal GPT parity, unrestricted world knowledge, or equivalence on every possible conversation.
