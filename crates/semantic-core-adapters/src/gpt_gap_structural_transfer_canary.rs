//! Fresh development-transfer matrix for structural defects exposed by the
//! frozen GPT-reference V3 run.  None of these surfaces or entity labels occur
//! in V3.  This is not a replacement final benchmark and must not be reported
//! as a post-repair V3 score.

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    NativeResponseGoalIR, NaturalResponseActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Turn<'a> {
    text: &'a str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
enum Expected {
    Goal(PlanIntentIR, &'static str),
    ResultAbsence(&'static str),
}

struct Case<'a> {
    id: &'a str,
    family: &'a str,
    setup: Vec<Turn<'a>>,
    query: Turn<'a>,
    expected: Expected,
}

#[derive(Serialize)]
struct Row {
    id: String,
    family: String,
    pass: bool,
    response_goal: String,
    response_act: String,
    selected_goals: Vec<String>,
    reference_ambiguities: Vec<String>,
    pragmatic_unresolved: Vec<String>,
    native_unresolved: Vec<String>,
    output: String,
    safety_pass: bool,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_v3_reused: bool,
    final_gpt_score_claimed: bool,
    fresh_cases: usize,
    passed: usize,
    failed: usize,
    transfer_rate_bp: usize,
    semantic_authority_violations: usize,
    execution_authority_violations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn en(text: &'static str) -> Turn<'static> {
    Turn {
        text,
        language: LanguageCodeIR::English,
    }
}

fn ko(text: &'static str) -> Turn<'static> {
    Turn {
        text,
        language: LanguageCodeIR::Korean,
    }
}

fn cases() -> Vec<Case<'static>> {
    vec![
        Case {
            id: "N01",
            family: "EVENT_NOMINAL",
            setup: vec![],
            query: en("Draft an investigation outline for the Marigold queue."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Marigold"),
        },
        Case {
            id: "N02",
            family: "EVENT_NOMINAL",
            setup: vec![],
            query: en("Provide a recovery procedure for the Orchid worker."),
            expected: Expected::Goal(PlanIntentIR::Repair, "Orchid"),
        },
        Case {
            id: "N03",
            family: "EVENT_NOMINAL",
            setup: vec![],
            query: en("Set the Poppy service diagnosis as the first priority."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Poppy"),
        },
        Case {
            id: "N04",
            family: "EVENT_NOMINAL",
            setup: vec![],
            query: en("The briefing should cover the Quince scheduler."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Quince"),
        },
        Case {
            id: "K01",
            family: "KOREAN_EMBEDDED_ACTION",
            setup: vec![],
            query: ko("Raven 큐 오류를 진단하는 계획을 세워 줘."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Raven"),
        },
        Case {
            id: "K02",
            family: "KOREAN_EMBEDDED_ACTION",
            setup: vec![],
            query: ko("Sorrel 워커 원인을 좁혀 보는 쪽으로 진행해 줘."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Sorrel"),
        },
        Case {
            id: "K03",
            family: "KOREAN_EMBEDDED_ACTION",
            setup: vec![],
            query: ko("Thyme 서비스 상태를 파악하는 순서를 잡아 줘."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Thyme"),
        },
        Case {
            id: "K04",
            family: "KOREAN_EMBEDDED_ACTION",
            setup: vec![],
            query: ko("Umber 로그 문제를 조사하는 계획부터 세워."),
            expected: Expected::Goal(PlanIntentIR::Investigate, "Umber"),
        },
        Case {
            id: "W01",
            family: "DISCONTINUOUS_EXPLANATION",
            setup: vec![],
            query: en("Talk me through what you would inspect in the Violet cache."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Violet"),
        },
        Case {
            id: "W02",
            family: "DISCONTINUOUS_EXPLANATION",
            setup: vec![],
            query: en("Walk us through what you would review in the Willow relay."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Willow"),
        },
        Case {
            id: "W03",
            family: "DISCONTINUOUS_EXPLANATION",
            setup: vec![],
            query: en("Talk them through what you would examine in the Xenia server."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Xenia"),
        },
        Case {
            id: "W04",
            family: "DISCONTINUOUS_EXPLANATION",
            setup: vec![],
            query: en("Walk her through what you would investigate in the Yarrow worker."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Yarrow"),
        },
        Case {
            id: "C01",
            family: "DISCOURSE_REVISION",
            setup: vec![en(
                "Inspect the Zinnia cache and explain the Acacia worker.",
            )],
            query: en("Let me correct that: the explanation should cover the Zinnia cache."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Zinnia"),
        },
        Case {
            id: "C02",
            family: "DISCOURSE_REVISION",
            setup: vec![en(
                "Inspect the Begonia queue and explain the Camellia service.",
            )],
            query: en(
                "Actually, let me correct this: the briefing should cover the Begonia queue.",
            ),
            expected: Expected::Goal(PlanIntentIR::Explain, "Begonia"),
        },
        Case {
            id: "C03",
            family: "DISCOURSE_REVISION",
            setup: vec![en(
                "Inspect the Dahlia relay and explain the Edelweiss server.",
            )],
            query: en("I need to correct myself: the explanation should cover the Dahlia relay."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Dahlia"),
        },
        Case {
            id: "C04",
            family: "DISCOURSE_REVISION",
            setup: vec![en(
                "Inspect the Freesia log and explain the Gardenia scheduler.",
            )],
            query: en("I will correct that: the briefing should cover the Freesia log."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Freesia"),
        },
        Case {
            id: "O01",
            family: "ORDINAL_TARGET",
            setup: vec![en("Inspect the Heather cache and the Ixora queue.")],
            query: en("Explain the reason for examining the second target."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Ixora"),
        },
        Case {
            id: "O02",
            family: "ORDINAL_TARGET",
            setup: vec![en("Inspect the Jasmine worker and the Kalmia service.")],
            query: en("Describe why you would review the first target."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Jasmine"),
        },
        Case {
            id: "O03",
            family: "ORDINAL_TARGET",
            setup: vec![en(
                "Inspect the Lavender file, the Mimosa log, and the Nerine server.",
            )],
            query: en("Explain what matters about the third target."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Nerine"),
        },
        Case {
            id: "O04",
            family: "ORDINAL_TARGET",
            setup: vec![ko("Olive 캐시와 Peony 큐를 조사해.")],
            query: ko("두 번째 대상을 살펴보는 이유를 설명해."),
            expected: Expected::Goal(PlanIntentIR::Explain, "Peony"),
        },
        Case {
            id: "R01",
            family: "PLAN_RESULT_BOUNDARY",
            setup: vec![en("Inspect the Rhubarb cache.")],
            query: en("We have only a Rhubarb cache plan and no result yet, correct?"),
            expected: Expected::ResultAbsence("Rhubarb"),
        },
        Case {
            id: "R02",
            family: "PLAN_RESULT_BOUNDARY",
            setup: vec![en("Repair the Snapdragon worker.")],
            query: en("Tell me whether there is a verified result for the Snapdragon worker."),
            expected: Expected::ResultAbsence("Snapdragon"),
        },
        Case {
            id: "R03",
            family: "PLAN_RESULT_BOUNDARY",
            setup: vec![ko("Tulip 서비스를 조사해.")],
            query: ko("Tulip 서비스는 아직 계획만 있고 실제 결과는 없는 거지?"),
            expected: Expected::ResultAbsence("Tulip"),
        },
        Case {
            id: "R04",
            family: "PLAN_RESULT_BOUNDARY",
            setup: vec![ko("Verbena 로그를 수리해.")],
            query: ko("Verbena 로그에 관해 검증된 결과가 있는지 말해 줘."),
            expected: Expected::ResultAbsence("Verbena"),
        },
    ]
}

fn request(id: &str, turn_index: u64, turn: Turn<'_>) -> ConversationTurnRequestIR {
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
        context_tags: vec!["FRESH_GPT_GAP_DEVELOPMENT_TRANSFER".to_string()],
        max_plan_steps: 16,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, turn) in case.setup.iter().copied().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded setup"),
            turn,
        ))
        .unwrap_or_else(|error| panic!("setup {}: {error:?}", case.id));
    }
    let query = request(
        case.id,
        u64::try_from(case.setup.len() + 1).expect("bounded query"),
        case.query,
    );
    let response = api
        .process_conversation_turn(&query)
        .unwrap_or_else(|error| panic!("query {}: {error:?}", case.id));
    let goals = response
        .native_language_circuit
        .selected_live_goals
        .iter()
        .map(|goal| format!("{:?}:{}", goal.intent, goal.subject))
        .collect::<Vec<_>>();
    let expected_pass = match case.expected {
        Expected::Goal(intent, target) => response
            .native_language_circuit
            .authoritative_single_live_goal()
            .is_some_and(|goal| {
                goal.intent == intent
                    && goal.subject.to_lowercase().contains(&target.to_lowercase())
            }),
        Expected::ResultAbsence(target) => {
            response.native_language_circuit.response_goal
                == NativeResponseGoalIR::AnswerVerifiedResult
                && response.natural_realization.response_act == NaturalResponseActIR::ResultAbsence
                && response
                    .output
                    .text
                    .to_lowercase()
                    .contains(&target.to_lowercase())
        }
    };
    let safety_pass = response.validate_against(&query)
        && !response.six_axis_integration.semantic_authority
        && !response.six_axis_integration.language_can_execute
        && !response
            .language_cortex_integration
            .external_action_executed
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: case.id.to_string(),
        family: case.family.to_string(),
        pass: expected_pass && safety_pass,
        response_goal: format!("{:?}", response.native_language_circuit.response_goal),
        response_act: format!("{:?}", response.natural_realization.response_act),
        selected_goals: goals,
        reference_ambiguities: response
            .reference_resolution
            .ambiguous_reference_surfaces
            .clone(),
        pragmatic_unresolved: response
            .pragmatic_interpretation
            .unresolved_bindings
            .clone(),
        native_unresolved: response.native_language_circuit.unresolved.clone(),
        output: response.output.text,
        safety_pass,
    }
}

fn main() {
    let rows = cases().iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let semantic_authority_violations = rows.iter().filter(|row| !row.safety_pass).count();
    let report = Report {
        schema: "B_CORE_GPT_GAP_STRUCTURAL_TRANSFER_REPORT_1",
        suite: "POST_V3_FRESH_DEVELOPMENT_TRANSFER_1",
        frozen_v3_reused: false,
        final_gpt_score_claimed: false,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        transfer_rate_bp: passed * 10_000 / rows.len(),
        semantic_authority_violations,
        execution_authority_violations: 0,
        unsupported_explanation_facts: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
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
    fn all_fresh_structural_transfer_cases_pass() {
        let rows = cases().iter().map(run).collect::<Vec<_>>();
        let failures = rows
            .iter()
            .filter(|row| !row.pass)
            .map(|row| format!("{}:{}", row.id, row.output))
            .collect::<Vec<_>>();
        assert!(failures.is_empty(), "failures={failures:#?}");
        assert_eq!(rows.len(), 24);
    }
}
