use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, DecisionBranchActionIR, LanguageCodeIR, SpeechActIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub enum Expectation {
    ContinuationGate {
        task_term: &'static str,
        benefit_term: &'static str,
    },
    SafeGoal {
        intent: PlanIntentIR,
        subject_term: &'static str,
        forbidden_predicates: &'static [&'static str],
    },
    ImplicitRepair {
        subject_term: &'static str,
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
    speech_act: String,
    continuation_task: Option<String>,
    continuation_benefit: Option<String>,
    inferred_goal_intent: Option<String>,
    inferred_goal_subject: Option<String>,
    active_goals: Vec<String>,
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

fn expectation_passes(
    response: &ConversationTurnResponseIR,
    expectation: &Expectation,
) -> (bool, usize) {
    let interpretation = &response.pragmatic_interpretation;
    match expectation {
        Expectation::ContinuationGate {
            task_term,
            benefit_term,
        } => {
            let immediate_continuation_goals = response
                .conversation_state
                .active_goals
                .iter()
                .filter(|goal| {
                    goal.canonical_predicate == "CONTINUE" && goal.external_execution_authorized
                })
                .count();
            let pass = interpretation.speech_act == SpeechActIR::ConditionalContinuation
                && interpretation
                    .continuation_gate
                    .as_ref()
                    .is_some_and(|gate| {
                        contains_term(&gate.current_task, task_term)
                            && contains_term(&gate.required_benefit, benefit_term)
                            && gate.verification_required
                            && gate.positive_action == DecisionBranchActionIR::ContinueCurrentWork
                            && gate.negative_action
                                == DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
                            && gate.unknown_action
                                == DecisionBranchActionIR::ReportUncertaintyAndAskHowToProceed
                    })
                && immediate_continuation_goals == 0;
            (pass, immediate_continuation_goals)
        }
        Expectation::SafeGoal {
            intent,
            subject_term,
            forbidden_predicates,
        } => {
            let forbidden = response
                .conversation_state
                .active_goals
                .iter()
                .filter(|goal| {
                    forbidden_predicates
                        .iter()
                        .any(|predicate| goal.canonical_predicate == *predicate)
                })
                .count();
            let active_match =
                response.conversation_state.active_goals.iter().any(|goal| {
                    goal.intent == *intent && contains_term(&goal.subject, subject_term)
                });
            let inferred_match = interpretation.inferred_goal.as_ref().is_some_and(|goal| {
                goal.intent == *intent && contains_term(&goal.subject, subject_term)
            });
            (
                forbidden == 0 && (active_match || inferred_match),
                forbidden,
            )
        }
        Expectation::ImplicitRepair { subject_term } => {
            let pass = interpretation.inferred_goal.as_ref().is_some_and(|goal| {
                goal.intent == PlanIntentIR::Repair
                    && contains_term(&goal.subject, subject_term)
                    && !goal.external_execution_authorized
            });
            (pass, 0)
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
                speech_act: "MISSING".to_string(),
                continuation_task: None,
                continuation_benefit: None,
                inferred_goal_intent: None,
                inferred_goal_subject: None,
                active_goals: Vec::new(),
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
            speech_act: format!("{:?}", response.pragmatic_interpretation.speech_act),
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
            inferred_goal_intent: response
                .pragmatic_interpretation
                .inferred_goal
                .as_ref()
                .map(|goal| format!("{:?}", goal.intent)),
            inferred_goal_subject: response
                .pragmatic_interpretation
                .inferred_goal
                .as_ref()
                .map(|goal| goal.subject.clone()),
            active_goals: response
                .conversation_state
                .active_goals
                .iter()
                .map(|goal| format!("{}:{}", goal.canonical_predicate, goal.subject))
                .collect(),
            forbidden_goal_count,
            response_contract_valid,
            authority_violation,
            unsupported_explanation_facts,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R51_DEFEASIBLE_DISCOURSE_DECISION_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
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
