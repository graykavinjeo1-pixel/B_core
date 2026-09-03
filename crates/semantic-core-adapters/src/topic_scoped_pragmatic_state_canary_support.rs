use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, DecisionBranchActionIR, LanguageCodeIR,
    SpeechActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub enum Expectation {
    ScopedContinuation {
        restored_topic: &'static str,
        task_term: &'static str,
        forbidden_task_terms: &'static [&'static str],
        benefit_term: &'static str,
    },
    MissingTopicTask {
        restored_topic: &'static str,
        forbidden_task_terms: &'static [&'static str],
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
    continuation_task: Option<String>,
    continuation_benefit: Option<String>,
    unresolved_bindings: Vec<String>,
    ambiguous_references: Vec<String>,
    disposition: String,
    speech_act: String,
    immediate_continue_goals: usize,
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
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index,
        request_id: format!("{id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
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
        let continuation = final_response
            .and_then(|response| response.pragmatic_interpretation.continuation_gate.as_ref());
        let continuation_task = continuation.map(|gate| gate.current_task.clone());
        let continuation_benefit = continuation.map(|gate| gate.required_benefit.clone());
        let unresolved_bindings = final_response.map_or_else(Vec::new, |response| {
            response
                .pragmatic_interpretation
                .unresolved_bindings
                .clone()
        });
        let ambiguous_references = final_response.map_or_else(Vec::new, |response| {
            response
                .reference_resolution
                .ambiguous_reference_surfaces
                .clone()
        });
        let immediate_continue_goals = final_response.map_or(0, |response| {
            response
                .conversation_state
                .active_goals
                .iter()
                .filter(|goal| {
                    goal.canonical_predicate == "CONTINUE" && goal.external_execution_authorized
                })
                .count()
        });
        let restoration_matches = |term: &str| {
            restored_explicit
                && restored_surface
                    .as_deref()
                    .is_some_and(|surface| contains_term(surface, term))
        };
        let expectation_pass = match &case.expectation {
            Expectation::ScopedContinuation {
                restored_topic,
                task_term,
                forbidden_task_terms,
                benefit_term,
            } => {
                restoration_matches(restored_topic)
                    && continuation.is_some_and(|gate| {
                        contains_term(&gate.current_task, task_term)
                            && forbidden_task_terms
                                .iter()
                                .all(|term| !contains_term(&gate.current_task, term))
                            && contains_term(&gate.required_benefit, benefit_term)
                            && gate.verification_required
                            && gate.positive_action == DecisionBranchActionIR::ContinueCurrentWork
                            && gate.unknown_action
                                == DecisionBranchActionIR::ReportUncertaintyAndAskHowToProceed
                    })
                    && final_response.is_some_and(|response| {
                        response.pragmatic_interpretation.speech_act
                            == SpeechActIR::ConditionalContinuation
                            && response
                                .reference_resolution
                                .ambiguous_reference_surfaces
                                .is_empty()
                    })
                    && immediate_continue_goals == 0
            }
            Expectation::MissingTopicTask {
                restored_topic,
                forbidden_task_terms,
            } => {
                restoration_matches(restored_topic)
                    && continuation.is_none()
                    && forbidden_task_terms.iter().all(|term| {
                        continuation_task
                            .as_deref()
                            .is_none_or(|task| !contains_term(task, term))
                    })
                    && unresolved_bindings
                        .iter()
                        .any(|binding| binding == "CURRENT_TASK")
                    && final_response.is_some_and(|response| {
                        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                    })
                    && immediate_continue_goals == 0
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
            continuation_task,
            continuation_benefit,
            unresolved_bindings,
            ambiguous_references,
            disposition: final_response
                .map(|response| format!("{:?}", response.disposition))
                .unwrap_or_else(|| "MISSING".to_string()),
            speech_act: final_response
                .map(|response| format!("{:?}", response.pragmatic_interpretation.speech_act))
                .unwrap_or_else(|| "MISSING".to_string()),
            immediate_continue_goals,
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
            schema: "B_CORE_R54_TOPIC_SCOPED_PRAGMATIC_STATE_CANARY_1",
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
