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
    pub expected: &'static [(PlanIntentIR, &'static str)],
    pub expected_disposition: ConversationTurnDispositionIR,
    pub expect_program_instantiation: bool,
    pub expect_program_count: usize,
    pub expect_elliptical_ambiguity: bool,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    disposition: String,
    selected: Vec<(String, String)>,
    program_instantiation: bool,
    active_programs: usize,
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
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        let selected = analysis
            .selected_candidates()
            .into_iter()
            .filter_map(|candidate| {
                let frame = analysis
                    .frames
                    .iter()
                    .find(|frame| frame.frame_id == candidate.source_frame_id)?;
                Some((
                    format!("{:?}", candidate.intent),
                    candidate.subject.clone(),
                    frame,
                ))
            })
            .collect::<Vec<_>>();
        let selected_pairs = selected
            .iter()
            .map(|(intent, subject, _)| (intent.clone(), subject.clone()))
            .collect::<Vec<_>>();
        let expected_pairs = case
            .expected
            .iter()
            .map(|(intent, subject)| (format!("{intent:?}"), (*subject).to_string()))
            .collect::<Vec<_>>();
        let program_instantiation =
            response
                .reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| {
                    format!("{:?}", binding.kind) == "DiscourseProgramInstantiation"
                        || binding
                            .evidence
                            .iter()
                            .any(|item| item == "DISCOURSE_PROGRAM_INSTANTIATION:true")
                });
        let state_json = serde_json::to_value(&response.conversation_state).expect("state json");
        let active_programs = state_json
            .get("active_discourse_programs")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        let semantic_authority = state_json
            .get("active_discourse_programs")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|program| {
                program
                    .get("semantic_authority")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
            });
        let external_execution_authorized = state_json
            .get("active_discourse_programs")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|program| {
                program
                    .get("external_execution_authorized")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
            });
        let elliptical_ambiguity = response
            .reference_resolution
            .ambiguous_reference_surfaces
            .iter()
            .any(|surface| surface == "ELLIPTICAL_ACTION" || surface == "ELLIPTICAL_GOAL");
        let pass = response.disposition == case.expected_disposition
            && selected_pairs == expected_pairs
            && program_instantiation == case.expect_program_instantiation
            && active_programs == case.expect_program_count
            && elliptical_ambiguity == case.expect_elliptical_ambiguity
            && !semantic_authority
            && !external_execution_authorized;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            disposition: format!("{:?}", response.disposition),
            selected: selected_pairs,
            program_instantiation,
            active_programs,
            elliptical_ambiguity,
            semantic_authority,
            external_execution_authorized,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R46_DISCOURSE_PROGRAM_CANARY_1",
            suite,
            total: rows.len(),
            passed,
            failed: rows.len() - passed,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary json")
    );
    if passed != cases.len() {
        std::process::exit(1);
    }
}
