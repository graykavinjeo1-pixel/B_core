use super::*;

#[test]
fn affect_policy_changes_the_public_response_plan_not_semantic_obligations() {
    use crate::affective_field::AffectiveFieldIR;
    let mut neutral_api = CognitiveApi::new_embedded().unwrap();
    let mut urgent_api = CognitiveApi::new_embedded().unwrap();
    let input = request(
        "BREVITY",
        1,
        "I'm upset. Investigate the cache.",
        LanguageCodeIR::English,
    );
    urgent_api.affective_memory.insert(
        "BREVITY".into(),
        AffectiveFieldIR::observe(None, "urgent urgent urgent", None),
    );
    let neutral = neutral_api.process_conversation_turn(&input).unwrap();
    let urgent = urgent_api.process_conversation_turn(&input).unwrap();
    assert_eq!(
        neutral.natural_realization.response_act,
        urgent.natural_realization.response_act
    );
    assert!(urgent.affective_policy.brevity_millis > 150);
    assert!(
        neutral.natural_realization.response_plan.moves.len()
            > urgent.natural_realization.response_plan.moves.len(),
        "neutral={} urgent={}",
        neutral.output.text,
        urgent.output.text
    );
    assert_eq!(neutral.request_semantics, urgent.request_semantics);
    assert!(urgent.validate_against(&input));
    assert!(urgent.natural_realization.coverage.orphan_generation_traces == 0);
    let neutral_traces = &neutral.natural_realization.generation_traces;
    let urgent_traces = &urgent.natural_realization.generation_traces;
    let removed_auxiliary = neutral_traces.len() - urgent_traces.len();
    assert_eq!(
        neutral_traces[removed_auxiliary..]
            .iter()
            .map(|trace| &trace.meaning)
            .collect::<Vec<_>>(),
        urgent_traces
            .iter()
            .map(|trace| &trace.meaning)
            .collect::<Vec<_>>()
    );
    println!(
        "BREVITY_NEUTRAL={}\nBREVITY_URGENT={}",
        neutral.output.text, urgent.output.text
    );
}

#[test]
fn playful_social_input_reaches_public_output_with_no_authority() {
    for (text, language, marker) in [
        ("ㅋㅋ 고마워", LanguageCodeIR::Korean, "ㅎㅎ"),
        ("haha thanks", LanguageCodeIR::English, "Heh,"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("PLAYFUL-API", 1, text, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.output.text.contains(marker),
            "{}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        println!(
            "PLAYFUL_INPUT={text}\nPLAYFUL_OUTPUT={}",
            response.output.text
        );
    }
}

#[test]
fn reformulation_requeries_evidence_and_preserves_answer_slot() {
    for (id, language, report, question, followup) in [
        (
            "REFORM-KO",
            LanguageCodeIR::Korean,
            "민수가 보고서를 수정했어.",
            "누가 수정했어?",
            "핵심만 다시 설명해.",
        ),
        (
            "REFORM-EN",
            LanguageCodeIR::English,
            "Mina said that Lumen failed because DeltaWorker stopped.",
            "Why did Lumen fail?",
            "Explain that again briefly.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(id, 1, report, language))
            .unwrap();
        let first = api
            .process_conversation_turn(&request(id, 2, question, language))
            .unwrap();
        let prior = first.discourse_answer.as_ref().unwrap();
        assert!(prior.content_projection.is_some());
        let next_request = request(id, 3, followup, language);
        let next = api.process_conversation_turn(&next_request).unwrap();
        let answer = next.discourse_answer.as_ref().unwrap();
        assert_eq!(
            next.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer,
            "{}",
            next.output.text
        );
        assert!(answer.reformulated_request.is_some(), "{answer:#?}");
        assert_eq!(prior.content_projection, answer.content_projection);
        assert_eq!(prior.claims, answer.claims);
        assert!(next.validate_against(&next_request));
        assert!(next
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        println!(
            "REFORMULATION_INPUT={followup}\nREFORMULATION_OUTPUT={}",
            next.output.text
        );

        // Adversarial state ablation: removing current semantic evidence must
        // remove the answer even though the question focus still exists.
        let mut state = next.conversation_state.clone();
        state.epistemic_ledger.records.clear();
        let missing = crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, Some(&state), language)
            .unwrap();
        assert!(missing.content_projection.is_none());
        assert!(!missing
            .claims
            .iter()
            .any(|claim| claim.value == prior.claims[0].value));
        let mut expired = next.conversation_state.clone();
        expired.completed_turns += 4;
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, Some(&expired), language)
            .is_none());
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, None, language)
            .is_none());
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(
                "Explain Beryl again.",
                Some(&next.conversation_state),
                language
            )
            .is_none());

        api.process_conversation_turn(&request(id, 4, "Investigate Beryl queue.", language))
            .unwrap();
        assert!(api
            .conversation_memory
            .state(id)
            .unwrap()
            .answer_focus
            .is_none());
    }
}

#[test]
fn reformulation_grammar_rejects_new_topics_quotes_negation_and_mixed_actions() {
    use crate::discourse_qa::is_answer_reformulation as accepts;
    for text in [
        "다시 설명해.",
        "그걸 짧게 다시 말해.",
        "Please rephrase your previous answer.",
        "Explain it again in detail.",
    ] {
        assert!(accepts(text), "{text}");
    }
    for text in [
        "Explain Beryl again.",
        "Do not repeat that.",
        "다시 설명하지 마.",
        "Explain it again and delete the file.",
        "\"Explain it again\"",
        "누가 수정했어?",
    ] {
        assert!(!accepts(text), "{text}");
    }
}

#[test]
fn playful_morphology_is_social_only_reversible_and_meaning_preserving() {
    use crate::affective_field::AffectiveRealizationPolicyIR;
    use crate::generative_language::{
        generate_dialogue_response_from_knowledge, GenerationDialogueResponseKindIR,
    };
    for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
        for kind in [
            GenerationDialogueResponseKindIR::Greeting,
            GenerationDialogueResponseKindIR::Gratitude,
        ] {
            let neutral = generate_dialogue_response_from_knowledge(language, kind).unwrap();
            let mut playful = neutral.clone();
            let policy = AffectiveRealizationPolicyIR {
                playfulness_millis: 800,
                ..Default::default()
            };
            playful.condition_realization(&policy);
            assert!(playful.validate());
            assert_eq!(neutral.meaning, playful.meaning);
            assert_eq!(neutral.speech_intent, playful.speech_intent);
            assert_eq!(neutral.syntax_plan, playful.syntax_plan);
            assert_ne!(
                neutral.morphology.realized_text,
                playful.morphology.realized_text
            );
            assert_eq!(
                playful
                    .morphology
                    .tokens
                    .iter()
                    .filter(|token| token.grammar_rule_id.as_deref()
                        == Some("GRAMMAR_SOCIAL_PLAYFUL_MARKER"))
                    .count(),
                1
            );
            playful.condition_realization(&AffectiveRealizationPolicyIR {
                urgency_millis: 800,
                ..policy
            });
            assert!(!playful
                .morphology
                .tokens
                .iter()
                .any(|token| token.grammar_rule_id.as_deref()
                    == Some("GRAMMAR_SOCIAL_PLAYFUL_MARKER")));
            playful.condition_realization(&Default::default());
            assert_eq!(neutral.morphology, playful.morphology);
        }
    }
}

/// Exhaust the declared finite route dimensions, not all possible sentences.
/// Each row supplies competing module signals, including a misleading native
/// plan candidate. This tests the actual central projection/arbitration code.
#[test]
fn finite_category_route_matrix_544_cells() {
    let categories = [
        "question",
        "explanation",
        "command",
        "correction",
        "cancellation",
        "disagreement",
        "agreement",
        "condition",
        "hypothesis",
        "past_result",
        "followup",
        "topic_transition",
        "affect",
        "social",
        "ambiguous_reference",
        "multiple_goals",
        "fragment",
    ];
    let mut cells = 0;
    for category in categories {
        for flags in 0..32 {
            let multi_turn = flags & 1 != 0;
            let explicit = flags & 2 != 0;
            let negative = flags & 4 != 0;
            let emotional = flags & 8 != 0;
            let known_reference = flags & 16 != 0;
            let information = matches!(
                category,
                "question" | "explanation" | "past_result" | "followup"
            );
            let non_action = matches!(
                category,
                "disagreement" | "agreement" | "hypothesis" | "affect" | "social"
            );
            let ambiguous = category == "ambiguous_reference" || (!explicit && !known_reference);
            use LanguagePipelineSignalIR as S;
            let routing = LanguagePipelineRoutingIR::from_candidates([
                Some(S::NormalizedGrounded),
                Some(S::GroundedDisposition),
                Some(S::SemanticGoalAvailable),
                Some(S::NativeGoalOwnsTurn),
                information.then_some(S::InformationRequest),
                non_action.then_some(S::AssertionOnly),
                explicit.then_some(S::ExplicitSelectedRequest),
                (multi_turn && category == "past_result").then_some(S::PlanResultOwnsTurn),
                negative.then_some(S::InteractionBoundaryOwnsTurn),
                emotional.then_some(S::AffectOnly),
                ambiguous.then_some(S::AmbiguousInput),
                (!ambiguous).then_some(S::ReferencesFullyResolved),
            ]);
            let plan = PlanProjectionDecisionIR::from_routing(&routing);
            if information || non_action || negative || ambiguous {
                assert!(
                    !plan.allows_plan(),
                    "category={category} flags={flags} blockers={:?}",
                    plan.blockers
                );
            }
            let mut candidates = vec![NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::NativePlan,
                NaturalResponseActIR::PlanPreview,
                "competing plan",
            )];
            if information {
                candidates.push(NaturalResponseCandidateIR::new(
                    NaturalResponseSourceIR::InformationAnswer,
                    NaturalResponseActIR::DiscourseAnswer,
                    "answer obligation",
                ));
            }
            if ambiguous {
                candidates.push(NaturalResponseCandidateIR::new(
                    NaturalResponseSourceIR::Clarification,
                    NaturalResponseActIR::ClarificationRequest,
                    "unresolved reference",
                ));
            }
            let arbitration = arbitrate_natural_response(candidates.clone());
            candidates.reverse();
            assert_eq!(
                arbitration.selected_act,
                arbitrate_natural_response(candidates).selected_act
            );
            if ambiguous {
                assert_eq!(
                    arbitration.selected_act,
                    NaturalResponseActIR::ClarificationRequest
                );
            } else if information {
                assert_eq!(
                    arbitration.selected_act,
                    NaturalResponseActIR::DiscourseAnswer
                );
            }
            cells += 1;
        }
    }
    assert_eq!(cells, 544);
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: crate::conversation::CONVERSATION_TURN_REQUEST_SCHEMA.into(),
        conversation_id: id.into(),
        request_id: format!("{id}-{turn}"),
        turn_index: turn,
        raw_text: text.into(),
        modality: crate::conversation::ConversationInputModalityIR::Text,
        input_confidence_millis: 1000,
        alternatives: vec![],
        output_language: Some(language),
        context_tags: vec![],
        max_plan_steps: 16,
    }
}

#[test]
fn dialogue_role_slots_are_answered_without_reissuing_the_reported_action() {
    for (index, (actor, object)) in [("민수", "보고서"), ("유나", "파일"), ("지민", "설정")]
        .iter()
        .enumerate()
    {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let id = format!("ROLE-{index}");
        let first = api
            .process_conversation_turn(&request(
                &id,
                1,
                &format!("{actor}가 {object}를 수정했어."),
                LanguageCodeIR::Korean,
            ))
            .unwrap();
        assert!(
            first.grounded_response.is_none(),
            "report became a plan: {}",
            first.output.text
        );
        for (turn, text, expected) in [(2, "누가 수정했어?", *actor), (3, "뭘 수정했어?", *object)]
        {
            let response = api
                .process_conversation_turn(&request(&id, turn, text, LanguageCodeIR::Korean))
                .unwrap();
            assert!(response.grounded_response.is_none());
            let projection = response
                .discourse_answer
                .as_ref()
                .and_then(|answer| answer.content_projection.as_ref());
            assert_eq!(
                projection.map(|projection| projection.binding.value.as_str()),
                Some(expected),
                "{}",
                response.output.text
            );
            assert!(response.output.text.contains(expected));
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
        }
    }
}

#[test]
fn causal_slots_are_retrieved_from_reported_content_without_truth_promotion() {
    for (index, (statement, question, expected, language)) in [
        (
            "Mina says the Lumen cache is stale because the worker stopped.",
            "Why is the Lumen cache stale?",
            "the worker stopped",
            LanguageCodeIR::English,
        ),
        (
            "Joon says the Orion task failed because the disk filled.",
            "Why did the Orion task fail?",
            "the disk filled",
            LanguageCodeIR::English,
        ),
        (
            "서버 중단 때문에 요청이 실패했어.",
            "왜 실패했어?",
            "서버 중단 때문에",
            LanguageCodeIR::Korean,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let id = format!("CAUSE-{index}");
        api.process_conversation_turn(&request(&id, 1, statement, language))
            .unwrap();
        let response = api
            .process_conversation_turn(&request(&id, 2, question, language))
            .unwrap();
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("typed content answer");
        assert_eq!(
            answer
                .content_projection
                .as_ref()
                .map(|p| p.binding.value.as_str()),
            Some(expected),
            "{}",
            response.output.text
        );
        assert!(!answer.dialogue_truth_established);
        assert!(!response.action_state_analysis.has_language_reports());
    }
}

#[test]
fn unsupported_explanations_are_gaps_not_promises_and_queries_do_not_create_actions() {
    for (id, language, turns) in [
        (
            "GAP-KO",
            LanguageCodeIR::Korean,
            vec![
                "캐시가 뭔지 설명해.",
                "왜 필요한데?",
                "계획 말고 지금 설명해.",
            ],
        ),
        (
            "GAP-EN",
            LanguageCodeIR::English,
            vec![
                "Explain what a cache is.",
                "Why is it useful?",
                "Do not tell me your plan. Answer the question.",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.into_iter().enumerate() {
            let response = api
                .process_conversation_turn(&request(id, index as u64 + 1, text, language))
                .unwrap();
            assert!(response.grounded_response.is_none());
            assert_ne!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanPreview
            );
            assert!(!response
                .reference_resolution
                .resolved_semantic_text
                .contains("why is why"));
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
        }
    }
}

#[test]
fn affect_changes_realization_but_cannot_change_meaning_or_authority() {
    use crate::affective_field::AffectiveFieldIR;
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "AFFECT-BOUNDARY",
        1,
        "민수가 보고서를 수정했어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let query = request(
        "AFFECT-BOUNDARY",
        2,
        "누가 수정했어?",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&query).unwrap();
    let neutral = response
        .natural_realization
        .generation_traces
        .first()
        .unwrap();
    let mut formal = neutral.clone();
    let field = AffectiveFieldIR::observe(None, "알려 주세요.", None);
    formal.condition_realization(&field.policy());
    assert!(neutral.validate() && formal.validate());
    assert_ne!(
        neutral.morphology.realized_text,
        formal.morphology.realized_text
    );
    assert_eq!(neutral.meaning, formal.meaning);
    assert_eq!(neutral.speech_intent, formal.speech_intent);
    assert_eq!(neutral.syntax_plan, formal.syntax_plan);
    assert_eq!(neutral.expression_selection, formal.expression_selection);
    assert!(!formal.language_can_execute && !formal.semantic_authority);
    let mut playful_fact = neutral.clone();
    playful_fact.condition_realization(&crate::affective_field::AffectiveRealizationPolicyIR {
        playfulness_millis: 1000,
        ..Default::default()
    });
    assert_eq!(neutral.morphology, playful_fact.morphology);
    let mut forged_morphology = neutral.clone();
    forged_morphology.morphology.tokens[0].surface = "unsupported grammar claim".into();
    forged_morphology.generation_sha256 =
        crate::generative_language::generative_language_sha256(&forged_morphology);
    assert!(!forged_morphology.validate());
    let mut forged = response.clone();
    forged.conversation_contract.independent_action_requested = true;
    assert!(!forged.validate_against(&query));
    let mut forged_answer = response.discourse_answer.clone().unwrap();
    forged_answer.claims[0].value = "a fabricated actor".into();
    assert!(!forged_answer.validate());
}

#[test]
fn content_compilation_keeps_case_and_does_not_guess_ambiguous_causality() {
    use crate::proposition_content::{ContentSlotIR, PropositionContentIR};
    let text = "The Q17 cache failed because DeltaWorker stopped.";
    let content = PropositionContentIR::compile(text);
    assert!(content.validate_source(text));
    assert!(content
        .bindings
        .iter()
        .any(|binding| binding.slot == ContentSlotIR::Cause
            && binding.value == "DeltaWorker stopped."));
    let mut forged = content;
    forged.bindings[0].value = "invented".into();
    assert!(!forged.validate_source(text));
    let ambiguous = PropositionContentIR::compile("파일을 열어서 내용을 수정했어.");
    assert!(!ambiguous
        .bindings
        .iter()
        .any(|binding| binding.slot == ContentSlotIR::Cause));
}

#[test]
fn affect_evidence_has_bounds_decay_negation_and_no_invented_timing() {
    use crate::affective_field::{AffectAxisIR as A, AffectiveFieldIR as F};
    let positive = F::observe(None, "happy", None);
    let negative = F::observe(None, "not happy", None);
    assert!(positive.value(A::Valence) > 0 && negative.value(A::Valence) < 0);
    assert!(F::observe(None, "\"urgent fuck!!!\"", None)
        .observations
        .is_empty());
    let timed = F::observe(None, "", Some(4000));
    assert!(timed.axes.is_empty());
    assert_eq!(timed.response_interval_ms, Some(4000));
    let punctuation = F::observe(None, "!!!", None);
    assert!(punctuation.value(A::Arousal) > 0);
    assert_eq!(punctuation.value(A::Confrontation), 0);
    let mut accumulated = F::observe(None, "urgent!", None);
    for _ in 0..50 {
        accumulated = F::observe(Some(&accumulated), "urgent urgent!!!", None);
    }
    assert!(accumulated.validate());
    assert!(
        F::observe(Some(&accumulated), "ordinary input", None).value(A::Urgency)
            < accumulated.value(A::Urgency)
    );
}

/// End-to-end category smoke coverage is separate from the 544 typed-signal
/// combinations. Passing these checks says nothing about arbitrary paraphrases.
#[test]
fn seventeen_categories_reach_a_valid_committed_response() {
    let cases: [(&str, &[&str]); 17] = [
        ("question", &["민수가 보고서를 수정했어.", "누가 수정했어?"]),
        (
            "explanation",
            &[
                "Mina says the cache failed because the disk filled.",
                "Why did the cache fail?",
            ],
        ),
        ("command", &["Inspect the Aster cache."]),
        (
            "correction",
            &[
                "Inspect the Aster cache.",
                "No, do not inspect it; explain why it failed.",
            ],
        ),
        ("cancellation", &["Run the build.", "Cancel it."]),
        (
            "disagreement",
            &["Mina says the cache is stale.", "I disagree."],
        ),
        ("agreement", &["Mina says the cache is stale.", "I agree."]),
        ("condition", &["If the tests pass, deploy the bundle."]),
        ("hypothesis", &["Suppose the cache failed."]),
        (
            "past_result",
            &["Inspect the Aster cache.", "Has it been executed?"],
        ),
        (
            "followup",
            &[
                "민수가 보고서를 수정했어.",
                "누가 수정했어?",
                "뭘 수정했어?",
            ],
        ),
        (
            "topic_transition",
            &[
                "Inspect the Aster cache.",
                "Let's talk about the Beryl queue.",
            ],
        ),
        ("affect", &["답답해."]),
        ("social", &["고마워."]),
        ("ambiguous_reference", &["Inspect it."]),
        (
            "multiple_goals",
            &["Inspect the Aster cache, then repair the Beryl queue."],
        ),
        ("fragment", &["음... 저기..."]),
    ];
    for (category, turns) in cases {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.iter().enumerate() {
            let input = request(category, index as u64 + 1, text, LanguageCodeIR::English);
            let response = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|error| panic!("category={category} turn={index}: {error:?}"));
            assert!(response.validate_against(&input));
            if matches!(
                category,
                "hypothesis" | "affect" | "social" | "ambiguous_reference" | "fragment"
            ) {
                assert!(
                    response.grounded_response.is_none(),
                    "category={category}: {}",
                    response.output.text
                );
                assert!(response
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
            }
            assert_eq!(response.natural_realization.stage_overwrite_count, 0);
            if response.conversation_contract.answer_only() {
                assert!(response.grounded_response.is_none());
                assert_ne!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::PlanPreview
                );
            }
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert_eq!(response.language_cortex_integration.external_llm_calls, 0);
        }
    }
}

#[test]
fn cause_questions_without_causal_knowledge_do_not_become_execution_status_queries() {
    for (id, statement, question) in [
        (
            "CAUSE-GAP-KO",
            "서버가 멈춰서 요청이 실패했어.",
            "왜 실패했어?",
        ),
        (
            "CAUSE-GAP-EN",
            "The cache failed.",
            "Why did the cache fail?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let report = api
            .process_conversation_turn(&request(id, 1, statement, LanguageCodeIR::English))
            .unwrap();
        assert!(report.grounded_response.is_none());
        let response = api
            .process_conversation_turn(&request(id, 2, question, LanguageCodeIR::English))
            .unwrap();
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer
        );
        assert_eq!(
            response.discourse_answer.as_ref().unwrap().disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord
        );
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
    }
}
