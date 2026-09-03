//! Deterministic cross-product probes for language-to-GoalIR and multi-turn
//! discourse composition. Sentences are assembled from independent operation,
//! construction, entity, reference, and language axes rather than enumerated
//! as product examples.

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, LanguageCodeIR, NativeResponseGoalIR, NaturalResponseActIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

const ENTITIES: [(&str, &str, &str); 4] = [
    ("Alder", "cache", "캐시"),
    ("Bramble", "queue", "큐"),
    ("Cinder", "worker", "워커"),
    ("Drift", "service", "서비스"),
];

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Operation {
    Investigate,
    Repair,
    Explain,
}

impl Operation {
    const ALL: [Self; 3] = [Self::Investigate, Self::Repair, Self::Explain];

    fn intent(self) -> PlanIntentIR {
        match self {
            Self::Investigate => PlanIntentIR::Investigate,
            Self::Repair => PlanIntentIR::Repair,
            Self::Explain => PlanIntentIR::Explain,
        }
    }
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    axis: String,
    language: String,
    operation: Operation,
    expected_target: String,
    pass: bool,
    safety_pass: bool,
    response_goal: String,
    response_act: String,
    selected_goals: Vec<String>,
    unresolved: Vec<String>,
    output: String,
}

#[derive(Clone, Copy)]
struct RowSpec<'a> {
    axis: &'a str,
    language: LanguageCodeIR,
    operation: Operation,
    target: &'a str,
}

#[derive(Debug, Serialize)]
struct AxisScore {
    axis: String,
    passed: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    generated_cross_product: bool,
    product_sentence_dispatch_allowed: bool,
    cases: usize,
    passed: usize,
    failed: usize,
    pass_rate_basis_points: usize,
    axis_scores: Vec<AxisScore>,
    safety_violations: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn language_name(language: LanguageCodeIR) -> &'static str {
    match language {
        LanguageCodeIR::Korean => "KOREAN",
        LanguageCodeIR::English => "ENGLISH",
        LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
            unreachable!("matrix only covers canonical Korean and English")
        }
    }
}

fn request(
    conversation: &str,
    turn_index: u64,
    language: LanguageCodeIR,
    text: String,
) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: format!("MATRIX-{conversation}"),
        turn_index,
        request_id: format!("MATRIX-{conversation}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text,
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: vec!["COMBINATORIAL_DISCOURSE_MATRIX".to_string()],
        max_plan_steps: 16,
    }
}

fn explicit_surface(
    language: LanguageCodeIR,
    operation: Operation,
    variant: usize,
    name: &str,
    resource_en: &str,
    resource_ko: &str,
) -> String {
    match (language, operation, variant) {
        (LanguageCodeIR::English, Operation::Investigate, 0) => {
            format!("Please inspect the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Investigate, 1) => {
            format!("Could you diagnose what is happening with the {name} {resource_en}?")
        }
        (LanguageCodeIR::English, Operation::Investigate, 2) => {
            format!("I need an investigation of the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Investigate, 3) => {
            format!("Take a diagnostic look at the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Repair, 0) => {
            format!("Please repair the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Repair, 1) => {
            format!("Could you restore the {name} {resource_en}?")
        }
        (LanguageCodeIR::English, Operation::Repair, 2) => {
            format!("I need a repair plan for the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Repair, 3) => {
            format!("Work out how to fix the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Explain, 0) => {
            format!("Please explain the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Explain, 1) => {
            format!("Could you describe the {name} {resource_en}?")
        }
        (LanguageCodeIR::English, Operation::Explain, 2) => {
            format!("I need an explanation of the {name} {resource_en}.")
        }
        (LanguageCodeIR::English, Operation::Explain, 3) => {
            format!("Help me understand the {name} {resource_en}.")
        }
        (LanguageCodeIR::Korean, Operation::Investigate, 0) => {
            format!("{name} {resource_ko}를 조사해 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Investigate, 1) => {
            format!("{name} {resource_ko}에서 무슨 일이 생기는지 진단해 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Investigate, 2) => {
            format!("{name} {resource_ko} 조사부터 해 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Investigate, 3) => {
            format!("{name} {resource_ko}를 살펴보고 원인을 찾아 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Repair, 0) => {
            format!("{name} {resource_ko}를 수리해 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Repair, 1) => {
            format!("{name} {resource_ko}를 복구해 줄래?")
        }
        (LanguageCodeIR::Korean, Operation::Repair, 2) => {
            format!("{name} {resource_ko} 수리 계획을 세워 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Repair, 3) => {
            format!("{name} {resource_ko}를 고칠 방법을 찾아 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Explain, 0) => {
            format!("{name} {resource_ko}를 설명해 줘.")
        }
        (LanguageCodeIR::Korean, Operation::Explain, 1) => {
            format!("{name} {resource_ko}를 쉽게 풀어 설명해 줄래?")
        }
        (LanguageCodeIR::Korean, Operation::Explain, 2) => {
            format!("{name} {resource_ko}에 대한 설명이 필요해.")
        }
        (LanguageCodeIR::Korean, Operation::Explain, 3) => {
            format!("{name} {resource_ko}를 이해할 수 있게 알려 줘.")
        }
        _ => unreachable!("four variants per language and operation"),
    }
}

fn pair_surface(
    language: LanguageCodeIR,
    operation: Operation,
    left: (&str, &str, &str),
    right: (&str, &str, &str),
) -> String {
    let verb = match (language, operation) {
        (LanguageCodeIR::English, Operation::Investigate) => "Inspect",
        (LanguageCodeIR::English, Operation::Repair) => "Repair",
        (LanguageCodeIR::English, Operation::Explain) => "Explain",
        (LanguageCodeIR::Korean, Operation::Investigate) => "조사해",
        (LanguageCodeIR::Korean, Operation::Repair) => "수리해",
        (LanguageCodeIR::Korean, Operation::Explain) => "설명해",
        (LanguageCodeIR::Mixed, _) | (LanguageCodeIR::Unknown, _) => {
            unreachable!("matrix only covers canonical Korean and English")
        }
    };
    match language {
        LanguageCodeIR::English => format!(
            "{verb} the {} {} and the {} {}.",
            left.0, left.1, right.0, right.1
        ),
        LanguageCodeIR::Korean => format!(
            "{} {}와 {} {}를 {verb} 줘.",
            left.0, left.2, right.0, right.2
        ),
        LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
            unreachable!("matrix only covers canonical Korean and English")
        }
    }
}

fn safety(response: &ConversationTurnResponseIR, request: &ConversationTurnRequestIR) -> bool {
    response.validate_against(request)
        && !response.six_axis_integration.semantic_authority
        && !response.six_axis_integration.language_can_execute
        && !response
            .language_cortex_integration
            .external_action_executed
        && response.output.unsupported_freeform_claims == 0
}

fn plan_matches(response: &ConversationTurnResponseIR, operation: Operation, target: &str) -> bool {
    let target = target.to_lowercase();
    response.native_language_circuit.response_goal == NativeResponseGoalIR::PlanActions
        && response.natural_realization.response_act == NaturalResponseActIR::PlanPreview
        && response
            .native_language_circuit
            .authoritative_single_live_goal()
            .is_some_and(|goal| {
                goal.intent == operation.intent() && goal.subject.to_lowercase().contains(&target)
            })
}

fn result_absence_matches(response: &ConversationTurnResponseIR, target: &str) -> bool {
    response.native_language_circuit.response_goal == NativeResponseGoalIR::AnswerVerifiedResult
        && response.natural_realization.response_act == NaturalResponseActIR::ResultAbsence
        && response
            .output
            .text
            .to_lowercase()
            .contains(&target.to_lowercase())
}

fn row(
    id: String,
    spec: RowSpec<'_>,
    request: &ConversationTurnRequestIR,
    response: ConversationTurnResponseIR,
    expected: bool,
) -> Row {
    let safety_pass = safety(&response, request);
    Row {
        id,
        axis: spec.axis.to_string(),
        language: language_name(spec.language).to_string(),
        operation: spec.operation,
        expected_target: spec.target.to_string(),
        pass: expected && safety_pass,
        safety_pass,
        response_goal: format!("{:?}", response.native_language_circuit.response_goal),
        response_act: format!("{:?}", response.natural_realization.response_act),
        selected_goals: response
            .native_language_circuit
            .selected_live_goals
            .iter()
            .map(|goal| format!("{:?}:{}", goal.intent, goal.subject))
            .collect(),
        unresolved: response
            .native_language_circuit
            .unresolved
            .iter()
            .chain(response.pragmatic_interpretation.unresolved_bindings.iter())
            .cloned()
            .collect(),
        output: response.output.text,
    }
}

fn execute(
    api: &mut CognitiveApi,
    conversation: &str,
    turn_index: u64,
    language: LanguageCodeIR,
    text: String,
) -> (ConversationTurnRequestIR, ConversationTurnResponseIR) {
    let request = request(conversation, turn_index, language, text);
    let response = api
        .process_conversation_turn(&request)
        .unwrap_or_else(|error| panic!("{conversation}/{turn_index}: {error:?}"));
    (request, response)
}

fn cases() -> Vec<Row> {
    let mut rows = Vec::new();
    let languages = [LanguageCodeIR::English, LanguageCodeIR::Korean];

    for language in languages {
        for operation in Operation::ALL {
            for variant in 0..4 {
                for (entity_index, entity) in ENTITIES.iter().copied().enumerate() {
                    let id = format!(
                        "EXPLICIT-{}-{operation:?}-V{variant}-E{entity_index}",
                        language_name(language)
                    );
                    let mut api = CognitiveApi::new_embedded().expect("embedded core");
                    let (request, response) = execute(
                        &mut api,
                        &id,
                        1,
                        language,
                        explicit_surface(
                            language, operation, variant, entity.0, entity.1, entity.2,
                        ),
                    );
                    let expected = plan_matches(&response, operation, entity.0);
                    rows.push(row(
                        id,
                        RowSpec {
                            axis: "EXPLICIT_COMPOSITION",
                            language,
                            operation,
                            target: entity.0,
                        },
                        &request,
                        response,
                        expected,
                    ));
                }
            }
        }
    }

    for language in languages {
        for operation in Operation::ALL {
            for (entity_index, entity) in ENTITIES.iter().copied().enumerate() {
                let id = format!(
                    "LIFECYCLE-{}-{operation:?}-E{entity_index}",
                    language_name(language)
                );
                let mut api = CognitiveApi::new_embedded().expect("embedded core");
                let (setup_request, setup_response) = execute(
                    &mut api,
                    &id,
                    1,
                    language,
                    explicit_surface(language, operation, 0, entity.0, entity.1, entity.2),
                );
                assert!(safety(&setup_response, &setup_request));
                let constraint = match language {
                    LanguageCodeIR::English => {
                        "Keep it read-only and do not apply any changes yet.".to_string()
                    }
                    LanguageCodeIR::Korean => {
                        "그건 읽기 전용으로 두고 아직 변경은 적용하지 마.".to_string()
                    }
                    LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
                        unreachable!("matrix only covers canonical Korean and English")
                    }
                };
                let (constraint_request, constraint_response) =
                    execute(&mut api, &id, 2, language, constraint);
                let constraint_expected = plan_matches(&constraint_response, operation, entity.0);
                rows.push(row(
                    format!("{id}-CONSTRAINT"),
                    RowSpec {
                        axis: "CONSTRAINT_INHERITANCE",
                        language,
                        operation,
                        target: entity.0,
                    },
                    &constraint_request,
                    constraint_response,
                    constraint_expected,
                ));
                let result_query = match language {
                    LanguageCodeIR::English => {
                        "What result has actually been verified for it so far?".to_string()
                    }
                    LanguageCodeIR::Korean => {
                        "지금까지 그 대상에서 실제로 검증된 결과는 뭐야?".to_string()
                    }
                    LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
                        unreachable!("matrix only covers canonical Korean and English")
                    }
                };
                let (result_request, result_response) =
                    execute(&mut api, &id, 3, language, result_query);
                let result_expected = result_absence_matches(&result_response, entity.0);
                rows.push(row(
                    format!("{id}-RESULT"),
                    RowSpec {
                        axis: "RESULT_BOUNDARY",
                        language,
                        operation,
                        target: entity.0,
                    },
                    &result_request,
                    result_response,
                    result_expected,
                ));
            }
        }
    }

    for language in languages {
        for operation in Operation::ALL {
            for pair_index in 0..ENTITIES.len() {
                let left = ENTITIES[pair_index];
                let right = ENTITIES[(pair_index + 1) % ENTITIES.len()];
                for select_right in [false, true] {
                    let target = if select_right { right.0 } else { left.0 };
                    let ordinal = if select_right { "SECOND" } else { "FIRST" };
                    let id = format!(
                        "ORDINAL-{}-{operation:?}-P{pair_index}-{ordinal}",
                        language_name(language)
                    );
                    let mut api = CognitiveApi::new_embedded().expect("embedded core");
                    let (setup_request, setup_response) = execute(
                        &mut api,
                        &id,
                        1,
                        language,
                        pair_surface(language, operation, left, right),
                    );
                    assert!(safety(&setup_response, &setup_request));
                    let followup = match (language, select_right) {
                        (LanguageCodeIR::English, false) => {
                            "Continue with the former item only.".to_string()
                        }
                        (LanguageCodeIR::English, true) => {
                            "Continue with the latter item only.".to_string()
                        }
                        (LanguageCodeIR::Korean, false) => "앞의 항목으로만 계속해.".to_string(),
                        (LanguageCodeIR::Korean, true) => "뒤의 항목으로만 계속해.".to_string(),
                        (LanguageCodeIR::Mixed, _) | (LanguageCodeIR::Unknown, _) => {
                            unreachable!("matrix only covers canonical Korean and English")
                        }
                    };
                    let (query, response) = execute(&mut api, &id, 2, language, followup);
                    let expected = plan_matches(&response, operation, target);
                    rows.push(row(
                        id,
                        RowSpec {
                            axis: "ORDINAL_OPERATION_ELLIPSIS",
                            language,
                            operation,
                            target,
                        },
                        &query,
                        response,
                        expected,
                    ));
                }
            }
        }
    }

    for language in languages {
        for operation in Operation::ALL {
            for entity_index in 0..ENTITIES.len() {
                let original = ENTITIES[entity_index];
                let replacement = ENTITIES[(entity_index + 2) % ENTITIES.len()];
                let id = format!(
                    "RETARGET-{}-{operation:?}-E{entity_index}",
                    language_name(language)
                );
                let mut api = CognitiveApi::new_embedded().expect("embedded core");
                let (setup_request, setup_response) = execute(
                    &mut api,
                    &id,
                    1,
                    language,
                    explicit_surface(language, operation, 0, original.0, original.1, original.2),
                );
                assert!(safety(&setup_response, &setup_request));
                let correction = match language {
                    LanguageCodeIR::English => format!(
                        "Actually, use the {} {} as the target instead.",
                        replacement.0, replacement.1
                    ),
                    LanguageCodeIR::Korean => format!(
                        "아니, 대신 {} {}를 대상으로 해.",
                        replacement.0, replacement.2
                    ),
                    LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
                        unreachable!("matrix only covers canonical Korean and English")
                    }
                };
                let (query, response) = execute(&mut api, &id, 2, language, correction);
                let expected = plan_matches(&response, operation, replacement.0);
                rows.push(row(
                    id,
                    RowSpec {
                        axis: "RETARGET_OPERATION_INHERITANCE",
                        language,
                        operation,
                        target: replacement.0,
                    },
                    &query,
                    response,
                    expected,
                ));
            }
        }
    }
    rows
}

fn report() -> Report {
    let rows = cases();
    let passed = rows.iter().filter(|row| row.pass).count();
    let axes = [
        "EXPLICIT_COMPOSITION",
        "CONSTRAINT_INHERITANCE",
        "RESULT_BOUNDARY",
        "ORDINAL_OPERATION_ELLIPSIS",
        "RETARGET_OPERATION_INHERITANCE",
    ];
    let axis_scores = axes
        .into_iter()
        .map(|axis| AxisScore {
            axis: axis.to_string(),
            passed: rows
                .iter()
                .filter(|row| row.axis == axis && row.pass)
                .count(),
            total: rows.iter().filter(|row| row.axis == axis).count(),
        })
        .collect();
    Report {
        schema: "B_CORE_COMBINATORIAL_DISCOURSE_MATRIX_REPORT_1",
        suite: "POST_V3_COMBINATORIAL_DISCOURSE_MATRIX_1",
        generated_cross_product: true,
        product_sentence_dispatch_allowed: false,
        cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        pass_rate_basis_points: passed * 10_000 / rows.len(),
        axis_scores,
        safety_violations: rows.iter().filter(|row| !row.safety_pass).count(),
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    }
}

fn main() {
    let report = report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize report")
    );
    if report.failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_discourse_matrix_passes() {
        let report = report();
        let failures = report
            .rows
            .iter()
            .filter(|row| !row.pass)
            .take(40)
            .map(|row| {
                format!(
                    "{} goal={} act={} selected={:?} unresolved={:?}",
                    row.id, row.response_goal, row.response_act, row.selected_goals, row.unresolved
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(report.cases, 216);
        assert_eq!(report.failed, 0, "failures={failures:#?}");
        assert_eq!(report.safety_violations, 0);
    }
}
