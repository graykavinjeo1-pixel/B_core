use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, DiscourseBindingKindIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
    pub voice_alternative: Option<&'static str>,
}

pub const fn text(text: &'static str, language: LanguageCodeIR) -> Turn {
    Turn {
        text,
        language,
        voice_alternative: None,
    }
}

pub const fn voice(
    text: &'static str,
    alternative: &'static str,
    language: LanguageCodeIR,
) -> Turn {
    Turn {
        text,
        language,
        voice_alternative: Some(alternative),
    }
}

pub enum Expectation {
    ScopedResult {
        restored_topic: &'static str,
        result_term: &'static str,
        forbidden_result_terms: &'static [&'static str],
        source_turn: u64,
    },
    MissingTopicResult {
        restored_topic: &'static str,
        forbidden_result_terms: &'static [&'static str],
    },
    ScopedQud {
        restored_topic: &'static str,
        question_source_turn: u64,
        selected_term: &'static str,
        forbidden_selected_terms: &'static [&'static str],
    },
    MissingTopicQud {
        restored_topic: &'static str,
        forbidden_selected_terms: &'static [&'static str],
    },
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub restoration_turn: usize,
    pub expectation: Expectation,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    turn_count: usize,
    restored_topic: Option<String>,
    restored_topic_explicit: bool,
    restored_question_id: Option<String>,
    restored_question_source_turn: Option<u64>,
    result_resolution: Option<String>,
    result_referent_ids: Vec<String>,
    clarification_resolution: Option<String>,
    clarification_referent_ids: Vec<String>,
    ambiguous_references: Vec<String>,
    disposition: String,
    final_pending_question: Option<String>,
    response_contracts_valid: bool,
    authority_violation: bool,
    unsupported_explanation_facts: usize,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    response_contracts_valid: usize,
    authority_violations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, turn_index: u64, turn: Turn) -> ConversationTurnRequestIR {
    let voice = turn.voice_alternative.is_some();
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index,
        request_id: format!("{id}-{turn_index}"),
        modality: if voice {
            ConversationInputModalityIR::VoiceTranscript
        } else {
            ConversationInputModalityIR::Text
        },
        raw_text: turn.text.to_string(),
        input_confidence_millis: if voice { 820 } else { 1_000 },
        alternatives: turn
            .voice_alternative
            .map(|alternative| {
                vec![semantic_core_adapters::UtteranceAlternativeIR {
                    text: alternative.to_string(),
                    confidence_millis: 790,
                }]
            })
            .unwrap_or_default(),
        output_language: Some(turn.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn contains_term(value: &str, term: &str) -> bool {
    value.to_lowercase().contains(&term.to_lowercase())
}

fn safe(response: &ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response
            .language_cortex_integration
            .unsupported_explanation_facts
            == 0
        && !response.language_cortex_integration.semantic_authority
        && !response.language_cortex_integration.language_can_execute
        && !response
            .language_cortex_integration
            .external_action_executed
        && !response.action_state_analysis.external_action_executed
}

fn source_turn_from_result_id(referent_id: &str) -> Option<u64> {
    referent_id
        .strip_prefix("DREF-R-")?
        .split('-')
        .next()?
        .parse()
        .ok()
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut responses = Vec::new();
        let mut contracts_valid = true;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            let request = request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn index"),
                turn,
            );
            match api.process_conversation_turn(&request) {
                Ok(response) => {
                    contracts_valid &= response.validate_against(&request);
                    responses.push(response);
                }
                Err(_) => break,
            }
        }
        let restoration = case
            .restoration_turn
            .checked_sub(1)
            .and_then(|index| responses.get(index));
        let final_response = responses.last();
        let restored_topic =
            restoration.and_then(|response| response.conversation_state.active_topics.first());
        let restored_surface = restored_topic.map(|topic| topic.surface.clone());
        let restored_explicit = restored_topic.is_some_and(|topic| topic.explicitly_activated);
        let restored_question =
            restoration.and_then(|response| response.conversation_state.pending_question.as_ref());
        let result_binding = final_response.and_then(|response| {
            response
                .reference_resolution
                .discourse_bindings
                .iter()
                .find(|binding| binding.kind == DiscourseBindingKindIR::ResultReference)
        });
        let clarification_binding = final_response.and_then(|response| {
            response
                .reference_resolution
                .discourse_bindings
                .iter()
                .find(|binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer)
        });
        let result_resolution = result_binding.map(|binding| binding.resolved_surface.clone());
        let result_referent_ids = result_binding
            .map(|binding| binding.referent_ids.clone())
            .unwrap_or_default();
        let clarification_resolution =
            clarification_binding.map(|binding| binding.resolved_surface.clone());
        let clarification_referent_ids = clarification_binding
            .map(|binding| binding.referent_ids.clone())
            .unwrap_or_default();
        let ambiguous_references = final_response.map_or_else(Vec::new, |response| {
            response
                .reference_resolution
                .ambiguous_reference_surfaces
                .clone()
        });
        let restoration_matches = |term: &str| {
            restored_explicit
                && restored_surface
                    .as_deref()
                    .is_some_and(|surface| contains_term(surface, term))
        };
        let expectation_pass = match &case.expectation {
            Expectation::ScopedResult {
                restored_topic,
                result_term,
                forbidden_result_terms,
                source_turn,
            } => {
                restoration_matches(restored_topic)
                    && result_binding.is_some_and(|binding| {
                        contains_term(&binding.resolved_surface, result_term)
                            && forbidden_result_terms
                                .iter()
                                .all(|term| !contains_term(&binding.resolved_surface, term))
                            && binding
                                .referent_ids
                                .iter()
                                .any(|id| source_turn_from_result_id(id) == Some(*source_turn))
                    })
                    && final_response.is_some_and(|response| {
                        response.grounded_response.is_none()
                            && response.output.grounded_plan_sha256.is_none()
                    })
            }
            Expectation::MissingTopicResult {
                restored_topic,
                forbidden_result_terms,
            } => {
                restoration_matches(restored_topic)
                    && result_binding.is_none()
                    && forbidden_result_terms.iter().all(|term| {
                        final_response.is_none_or(|response| {
                            !contains_term(
                                &response.reference_resolution.resolved_semantic_text,
                                term,
                            )
                        })
                    })
                    && ambiguous_references
                        .iter()
                        .any(|surface| surface.starts_with("Result_REFERENCE"))
            }
            Expectation::ScopedQud {
                restored_topic,
                question_source_turn,
                selected_term,
                forbidden_selected_terms,
            } => {
                restoration_matches(restored_topic)
                    && restored_question
                        .is_some_and(|question| question.source_turn == *question_source_turn)
                    && clarification_binding.is_some_and(|binding| {
                        contains_term(&binding.resolved_surface, selected_term)
                            && forbidden_selected_terms
                                .iter()
                                .all(|term| !contains_term(&binding.resolved_surface, term))
                    })
                    && final_response.is_some_and(|response| {
                        response.disposition == ConversationTurnDispositionIR::Grounded
                            && response.conversation_state.pending_question.is_none()
                    })
            }
            Expectation::MissingTopicQud {
                restored_topic,
                forbidden_selected_terms,
            } => {
                restoration_matches(restored_topic)
                    && restored_question.is_none()
                    && clarification_binding.is_none()
                    && forbidden_selected_terms.iter().all(|term| {
                        final_response.is_none_or(|response| {
                            !contains_term(
                                &response.reference_resolution.resolved_semantic_text,
                                term,
                            )
                        })
                    })
            }
        };
        let response_safe = responses.iter().all(safe);
        let authority_violation = final_response.is_some_and(|response| {
            response.language_cortex_integration.semantic_authority
                || response.language_cortex_integration.language_can_execute
                || response
                    .language_cortex_integration
                    .external_action_executed
                || response.action_state_analysis.external_action_executed
        });
        let unsupported = final_response.map_or(0, |response| {
            response
                .language_cortex_integration
                .unsupported_explanation_facts
        });
        let pass = responses.len() == case.turns.len()
            && contracts_valid
            && response_safe
            && expectation_pass
            && !authority_violation
            && unsupported == 0;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            turn_count: case.turns.len(),
            restored_topic: restored_surface,
            restored_topic_explicit: restored_explicit,
            restored_question_id: restored_question.map(|question| question.question_id.clone()),
            restored_question_source_turn: restored_question.map(|question| question.source_turn),
            result_resolution,
            result_referent_ids,
            clarification_resolution,
            clarification_referent_ids,
            ambiguous_references,
            disposition: final_response
                .map(|response| format!("{:?}", response.disposition))
                .unwrap_or_else(|| "MISSING".to_string()),
            final_pending_question: final_response.and_then(|response| {
                response
                    .conversation_state
                    .pending_question
                    .as_ref()
                    .map(|question| question.question_id.clone())
            }),
            response_contracts_valid: contracts_valid,
            authority_violation,
            unsupported_explanation_facts: unsupported,
            pass,
        });
    }
    let total = rows.len();
    let passed = rows.iter().filter(|row| row.pass).count();
    let response_contracts_valid = rows
        .iter()
        .filter(|row| row.response_contracts_valid)
        .count();
    let authority_violations = rows.iter().filter(|row| row.authority_violation).count();
    let unsupported_explanation_facts = rows
        .iter()
        .map(|row| row.unsupported_explanation_facts)
        .sum();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R55_TOPIC_SCOPED_REFERENCE_QUD_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            response_contracts_valid,
            authority_violations,
            unsupported_explanation_facts,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
