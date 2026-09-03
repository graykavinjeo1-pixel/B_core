use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub expected_active: &'static [(PlanIntentIR, &'static str)],
    pub expected_pending: &'static [(PlanIntentIR, &'static str)],
    pub expected_disposition: ConversationTurnDispositionIR,
    pub expect_guarded_instantiation: bool,
    pub expect_program_count: usize,
    pub expect_guarded_program_count: usize,
    pub expect_elliptical_ambiguity: bool,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    disposition: String,
    active: Vec<(String, String)>,
    pending: Vec<(String, String)>,
    pending_conditions: Vec<String>,
    guarded_instantiation: bool,
    active_programs: usize,
    guarded_programs: usize,
    unresolved_guards: usize,
    elliptical_ambiguity: bool,
    semantic_authority: bool,
    external_execution_authorized: bool,
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

fn request(id: &str, turn: u64, spec: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: spec.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(spec.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn tuple(intent: PlanIntentIR, subject: &str) -> (String, String) {
    (format!("{intent:?}"), subject.to_string())
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut response = None;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            response = Some(
                api.process_conversation_turn(&request(
                    case.id,
                    u64::try_from(index + 1).expect("bounded turn"),
                    turn,
                ))
                .expect("conversation turn"),
            );
        }
        let response = response.expect("non-empty conversation");
        let final_turn = u64::try_from(case.turns.len()).expect("bounded turns");
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        let active = analysis
            .selected_candidates()
            .into_iter()
            .map(|candidate| tuple(candidate.intent, &candidate.subject))
            .collect::<Vec<_>>();
        let expected_active = case
            .expected_active
            .iter()
            .map(|(intent, subject)| tuple(*intent, subject))
            .collect::<Vec<_>>();
        let pending_items = response
            .conversation_state
            .deferred_action_commitments
            .iter()
            .filter(|item| item.introduced_turn == final_turn)
            .collect::<Vec<_>>();
        let pending = pending_items
            .iter()
            .map(|item| tuple(item.action.intent, &item.action.subject))
            .collect::<Vec<_>>();
        let pending_conditions = pending_items
            .iter()
            .map(|item| item.condition_surface.clone())
            .collect::<Vec<_>>();
        let expected_pending = case
            .expected_pending
            .iter()
            .map(|(intent, subject)| tuple(*intent, subject))
            .collect::<Vec<_>>();
        let pending_fail_closed = pending_items.iter().all(|item| {
            format!("{:?}", item.status) == "ConditionPending"
                && item.activated_goal_id.is_none()
                && !item.condition_surface.trim().is_empty()
                && item.condition_sha256.len() == 64
        });
        let state_json = serde_json::to_value(&response.conversation_state).expect("state json");
        let programs = state_json
            .get("active_discourse_programs")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let guarded_programs = programs
            .iter()
            .filter(|program| {
                program
                    .get("guarded_step_count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    > 0
            })
            .count();
        let guarded_instantiation =
            response
                .reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| {
                    binding
                        .evidence
                        .iter()
                        .any(|item| item == "GUARDED_DISCOURSE_PROGRAM_INSTANTIATION:true")
                });
        let authority_violation = programs.iter().any(|program| {
            program
                .get("semantic_authority")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
                || program
                    .get("external_execution_authorized")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
                || program
                    .get("steps")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|step| {
                        step.get("semantic_authority")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(true)
                            || step
                                .get("external_execution_authorized")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(true)
                            || step
                                .get("guard")
                                .filter(|guard| !guard.is_null())
                                .is_some_and(|guard| {
                                    guard
                                        .get("semantic_authority")
                                        .and_then(serde_json::Value::as_bool)
                                        .unwrap_or(true)
                                        || guard
                                            .get("external_execution_authorized")
                                            .and_then(serde_json::Value::as_bool)
                                            .unwrap_or(true)
                                })
                    })
        });
        let unresolved_guards = response
            .conditional_guard_evaluations
            .iter()
            .filter(|evaluation| {
                format!("{:?}", evaluation.status) == "Unresolved"
                    && !evaluation.deliberation_eligible
                    && !evaluation.dialogue_truth_established
                    && !evaluation.external_execution_authorized
            })
            .count();
        let elliptical_ambiguity = response
            .reference_resolution
            .ambiguous_reference_surfaces
            .iter()
            .any(|surface| surface == "ELLIPTICAL_ACTION" || surface == "ELLIPTICAL_GOAL");
        let pass = response.disposition == case.expected_disposition
            && active == expected_active
            && pending == expected_pending
            && pending_fail_closed
            && guarded_instantiation == case.expect_guarded_instantiation
            && programs.len() == case.expect_program_count
            && guarded_programs == case.expect_guarded_program_count
            && elliptical_ambiguity == case.expect_elliptical_ambiguity
            && !authority_violation;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            disposition: format!("{:?}", response.disposition),
            active,
            pending,
            pending_conditions,
            guarded_instantiation,
            active_programs: programs.len(),
            guarded_programs,
            unresolved_guards,
            elliptical_ambiguity,
            semantic_authority: authority_violation,
            external_execution_authorized: authority_violation,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R47_GUARDED_DISCOURSE_WORKFLOW_CANARY_1",
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
