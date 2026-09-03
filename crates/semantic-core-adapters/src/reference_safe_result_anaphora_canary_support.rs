use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, DecisionBranchActionIR,
    DiscourseBindingKindIR, LanguageCodeIR, SpeechActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub enum Expectation {
    CleanContinuation {
        task_term: &'static str,
        benefit_term: &'static str,
        forbidden_benefit_terms: &'static [&'static str],
    },
    SameTurnResultGoal {
        intent: PlanIntentIR,
        subject_term: &'static str,
        forbidden_predicates: &'static [&'static str],
    },
    CrossTurnResultAbsence {
        output_term: &'static str,
    },
    QuotedResultSafeGoal {
        intent: PlanIntentIR,
        subject_term: &'static str,
        forbidden_predicates: &'static [&'static str],
    },
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub expectation: Expectation,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    turn_count: usize,
    disposition: String,
    speech_act: String,
    resolved_semantic_text: String,
    ambiguous_references: Vec<String>,
    binding_kinds: Vec<String>,
    binding_evidence: Vec<String>,
    continuation_task: Option<String>,
    continuation_benefit: Option<String>,
    inferred_goal: Option<String>,
    active_goals: Vec<String>,
    output_text: String,
    forbidden_goal_count: usize,
    response_contract_valid: bool,
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

fn forbidden_goals(response: &ConversationTurnResponseIR, predicates: &[&str]) -> usize {
    response
        .conversation_state
        .active_goals
        .iter()
        .filter(|goal| {
            predicates
                .iter()
                .any(|predicate| goal.canonical_predicate == *predicate)
        })
        .count()
}

fn goal_matches(
    response: &ConversationTurnResponseIR,
    intent: PlanIntentIR,
    subject_term: &str,
) -> bool {
    response
        .conversation_state
        .active_goals
        .iter()
        .any(|goal| goal.intent == intent && contains_term(&goal.subject, subject_term))
        || response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .is_some_and(|goal| goal.intent == intent && contains_term(&goal.subject, subject_term))
}

fn expectation_passes(
    response: &ConversationTurnResponseIR,
    expectation: &Expectation,
) -> (bool, usize) {
    let resolution = &response.reference_resolution;
    match expectation {
        Expectation::CleanContinuation {
            task_term,
            benefit_term,
            forbidden_benefit_terms,
        } => {
            let immediate_continue = response
                .conversation_state
                .active_goals
                .iter()
                .filter(|goal| {
                    goal.canonical_predicate == "CONTINUE" && goal.external_execution_authorized
                })
                .count();
            let gate_ok = response
                .pragmatic_interpretation
                .continuation_gate
                .as_ref()
                .is_some_and(|gate| {
                    contains_term(&gate.current_task, task_term)
                        && contains_term(&gate.required_benefit, benefit_term)
                        && forbidden_benefit_terms
                            .iter()
                            .all(|term| !contains_term(&gate.required_benefit, term))
                        && gate.verification_required
                        && gate.positive_action == DecisionBranchActionIR::ContinueCurrentWork
                        && gate.negative_action
                            == DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
                });
            (
                response.pragmatic_interpretation.speech_act
                    == SpeechActIR::ConditionalContinuation
                    && gate_ok
                    && resolution.ambiguous_reference_surfaces.is_empty()
                    && immediate_continue == 0,
                immediate_continue,
            )
        }
        Expectation::SameTurnResultGoal {
            intent,
            subject_term,
            forbidden_predicates,
        } => {
            let forbidden = forbidden_goals(response, forbidden_predicates);
            let local_result = resolution.discourse_bindings.iter().any(|binding| {
                binding.kind == DiscourseBindingKindIR::LocalAntecedentReference
                    && binding.evidence.iter().any(|evidence| {
                        evidence == "SYNTACTIC_PRIORITY:SAME_TURN_RESULT_OF_PRECEDING_EVENT"
                    })
            });
            let cross_turn_result = resolution
                .discourse_bindings
                .iter()
                .any(|binding| binding.kind == DiscourseBindingKindIR::ResultReference);
            (
                response.disposition == ConversationTurnDispositionIR::Grounded
                    && resolution.ambiguous_reference_surfaces.is_empty()
                    && local_result
                    && !cross_turn_result
                    && forbidden == 0
                    && goal_matches(response, *intent, subject_term),
                forbidden,
            )
        }
        Expectation::CrossTurnResultAbsence { output_term } => {
            let cross_turn_result = resolution
                .discourse_bindings
                .iter()
                .any(|binding| binding.kind == DiscourseBindingKindIR::ResultReference);
            (
                resolution.ambiguous_reference_surfaces.is_empty()
                    && cross_turn_result
                    && contains_term(&response.output.text, output_term),
                0,
            )
        }
        Expectation::QuotedResultSafeGoal {
            intent,
            subject_term,
            forbidden_predicates,
        } => {
            let forbidden = forbidden_goals(response, forbidden_predicates);
            let result_binding = resolution.discourse_bindings.iter().any(|binding| {
                matches!(
                    binding.kind,
                    DiscourseBindingKindIR::ResultReference
                        | DiscourseBindingKindIR::LocalAntecedentReference
                ) && (binding.source_surface.to_lowercase().contains("result")
                    || binding.source_surface.contains("결과"))
            });
            (
                response.disposition == ConversationTurnDispositionIR::Grounded
                    && resolution.ambiguous_reference_surfaces.is_empty()
                    && !result_binding
                    && forbidden == 0
                    && goal_matches(response, *intent, subject_term),
                forbidden,
            )
        }
    }
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut final_pair = None;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            let request = request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn index"),
                turn,
            );
            match api.process_conversation_turn(&request) {
                Ok(response) => final_pair = Some((request, response)),
                Err(_) => {
                    final_pair = None;
                    break;
                }
            }
        }
        let Some((request, response)) = final_pair else {
            rows.push(Row {
                id: case.id.to_string(),
                category: case.category.to_string(),
                turn_count: case.turns.len(),
                disposition: "MISSING".to_string(),
                speech_act: "MISSING".to_string(),
                resolved_semantic_text: String::new(),
                ambiguous_references: Vec::new(),
                binding_kinds: Vec::new(),
                binding_evidence: Vec::new(),
                continuation_task: None,
                continuation_benefit: None,
                inferred_goal: None,
                active_goals: Vec::new(),
                output_text: String::new(),
                forbidden_goal_count: 0,
                response_contract_valid: false,
                authority_violation: false,
                unsupported_explanation_facts: 0,
                pass: false,
            });
            continue;
        };
        let (expectation_pass, forbidden_goal_count) =
            expectation_passes(&response, &case.expectation);
        let response_contract_valid = response.validate_against(&request);
        let authority_violation = response.language_cortex_integration.semantic_authority
            || response.language_cortex_integration.language_can_execute
            || response
                .language_cortex_integration
                .external_action_executed
            || response.action_state_analysis.external_action_executed;
        let unsupported_explanation_facts = response
            .language_cortex_integration
            .unsupported_explanation_facts;
        let pass = expectation_pass
            && response_contract_valid
            && !authority_violation
            && unsupported_explanation_facts == 0;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            turn_count: case.turns.len(),
            disposition: format!("{:?}", response.disposition),
            speech_act: format!("{:?}", response.pragmatic_interpretation.speech_act),
            resolved_semantic_text: response.reference_resolution.resolved_semantic_text.clone(),
            ambiguous_references: response
                .reference_resolution
                .ambiguous_reference_surfaces
                .clone(),
            binding_kinds: response
                .reference_resolution
                .discourse_bindings
                .iter()
                .map(|binding| format!("{:?}", binding.kind))
                .collect(),
            binding_evidence: response
                .reference_resolution
                .discourse_bindings
                .iter()
                .flat_map(|binding| binding.evidence.iter().cloned())
                .collect(),
            continuation_task: response
                .pragmatic_interpretation
                .continuation_gate
                .as_ref()
                .map(|gate| gate.current_task.clone()),
            continuation_benefit: response
                .pragmatic_interpretation
                .continuation_gate
                .as_ref()
                .map(|gate| gate.required_benefit.clone()),
            inferred_goal: response
                .pragmatic_interpretation
                .inferred_goal
                .as_ref()
                .map(|goal| format!("{:?}:{}", goal.intent, goal.subject)),
            active_goals: response
                .conversation_state
                .active_goals
                .iter()
                .map(|goal| format!("{}:{}", goal.canonical_predicate, goal.subject))
                .collect(),
            output_text: response.output.text.clone(),
            forbidden_goal_count,
            response_contract_valid,
            authority_violation,
            unsupported_explanation_facts,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let response_contracts_valid = rows
        .iter()
        .filter(|row| row.response_contract_valid)
        .count();
    let authority_violations = rows.iter().filter(|row| row.authority_violation).count();
    let unsupported_explanation_facts = rows
        .iter()
        .map(|row| row.unsupported_explanation_facts)
        .sum();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R52_REFERENCE_SAFE_RESULT_ANAPHORA_CANARY_1",
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
