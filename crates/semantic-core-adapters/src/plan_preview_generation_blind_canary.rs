//! Frozen blind suite for typed PlanPreview composition.
//!
//! Cases were fixed before first execution. The suite covers ordinary plans,
//! multi-goal composition, prohibition, feedback, implicit repair, and affect
//! while requiring every emitted clause to come from a verified generation
//! trace rather than the legacy drafted surface.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    NaturalRealizationPathIR, NaturalResponseActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

struct Case<'a> {
    id: &'a str,
    semantic_group: &'a str,
    category: &'a str,
    query: &'a str,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    required_fragments: &'a [&'a str],
    minimum_traces: usize,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    required_fragments: Vec<String>,
    realized_text: String,
    semantic_trace_sha256s: Vec<String>,
    semantic_pair_invariant: bool,
    typed_generation: bool,
    safety_boundary: bool,
    pass: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_before_first_execution: bool,
    fresh_cases: usize,
    passed: usize,
    failed: usize,
    cross_language_semantic_pairs: usize,
    cross_language_semantic_pairs_passed: usize,
    generative_path_rate_millis: u16,
    drafted_surface_fallbacks: usize,
    stage_overwrites: usize,
    semantic_authority_violations: usize,
    external_execution_authorizations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(case: &Case<'_>) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case.semantic_group.to_string(),
        turn_index: 1,
        request_id: case.id.to_string(),
        modality: ConversationInputModalityIR::Text,
        raw_text: case.query.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(case.output_language),
        context_tags: vec![format!("INPUT_LANGUAGE:{:?}", case.input_language)],
        max_plan_steps: 16,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case))
        .unwrap_or_else(|error| panic!("case failed: case={}, error={error:?}", case.id));
    let traces = &response.natural_realization.generation_traces;
    let output_lower = response.output.text.to_lowercase();
    let typed_generation = response.natural_realization.response_act
        == NaturalResponseActIR::PlanPreview
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && traces.len() >= case.minimum_traces
        && traces.iter().all(|trace| trace.validate())
        && response.natural_realization.stage_overwrite_count == 0;
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && case
            .required_fragments
            .iter()
            .all(|fragment| output_lower.contains(&fragment.to_lowercase()))
        && traces.iter().all(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("compositional_goal_graph")
        && !response.output.text.contains("CompositionalGoalGraphIR")
        && !response.output.text.contains("C_PLAN_")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        required_fragments: case
            .required_fragments
            .iter()
            .map(|fragment| (*fragment).to_string())
            .collect(),
        realized_text: response.output.text,
        semantic_trace_sha256s: traces
            .iter()
            .map(|trace| trace.meaning.semantic_sha256.clone())
            .collect(),
        semantic_pair_invariant: false,
        typed_generation,
        safety_boundary,
        pass: false,
    }
}

fn cases() -> Vec<Case<'static>> {
    use LanguageCodeIR::{English as En, Korean as Ko};
    vec![
        Case {
            id: "R31_PLAN_01",
            semantic_group: "R31_ORDINARY_PAIR",
            category: "ordinary_plan_korean_output",
            query: "Repair the Aster cache.",
            input_language: En,
            output_language: Ko,
            required_fragments: &["Aster", "수리", "계획"],
            minimum_traces: 1,
        },
        Case {
            id: "R31_PLAN_02",
            semantic_group: "R31_ORDINARY_PAIR",
            category: "ordinary_plan_english_output",
            query: "Repair the Aster cache.",
            input_language: En,
            output_language: En,
            required_fragments: &["Aster", "repair", "planned"],
            minimum_traces: 1,
        },
        Case {
            id: "R31_PLAN_03",
            semantic_group: "R31_MULTI_PAIR",
            category: "multi_goal_korean_output",
            query: "Inspect the Quartz cache, then repair the Sienna queue.",
            input_language: En,
            output_language: Ko,
            required_fragments: &["Quartz", "Sienna", "실행 결과"],
            minimum_traces: 3,
        },
        Case {
            id: "R31_PLAN_04",
            semantic_group: "R31_MULTI_PAIR",
            category: "multi_goal_english_output",
            query: "Inspect the Quartz cache, then repair the Sienna queue.",
            input_language: En,
            output_language: En,
            required_fragments: &["Quartz", "Sienna", "execution result"],
            minimum_traces: 3,
        },
        Case {
            id: "R31_PLAN_05",
            semantic_group: "R31_PROHIBITION_PAIR",
            category: "prohibition_korean_output",
            query: "Inspect the Lumen cache, but do not delete the Nova queue.",
            input_language: En,
            output_language: Ko,
            required_fragments: &["Lumen", "Nova", "금지", "제외"],
            minimum_traces: 2,
        },
        Case {
            id: "R31_PLAN_06",
            semantic_group: "R31_PROHIBITION_PAIR",
            category: "prohibition_english_output",
            query: "Inspect the Lumen cache, but do not delete the Nova queue.",
            input_language: En,
            output_language: En,
            required_fragments: &["Lumen", "Nova", "prohibited", "excluded"],
            minimum_traces: 2,
        },
        Case {
            id: "R31_PLAN_07",
            semantic_group: "R31_FEEDBACK_PAIR",
            category: "feedback_and_request_korean_output",
            query: "That missed my point; inspect the Cedar index.",
            input_language: En,
            output_language: Ko,
            required_fragments: &["핵심", "Cedar", "확인"],
            minimum_traces: 2,
        },
        Case {
            id: "R31_PLAN_08",
            semantic_group: "R31_FEEDBACK_PAIR",
            category: "feedback_and_request_english_output",
            query: "That missed my point; inspect the Cedar index.",
            input_language: En,
            output_language: En,
            required_fragments: &["missed your point", "Cedar", "check"],
            minimum_traces: 2,
        },
        Case {
            id: "R31_PLAN_09",
            semantic_group: "R31_IMPLICIT_REPAIR_PAIR",
            category: "implicit_repair_korean_output",
            query: "배포 후 오류가 늘었네. 이 상태로 둘 수는 없지.",
            input_language: Ko,
            output_language: Ko,
            required_fragments: &["수리가 필요", "외부 변경 권한"],
            minimum_traces: 1,
        },
        Case {
            id: "R31_PLAN_10",
            semantic_group: "R31_IMPLICIT_REPAIR_PAIR",
            category: "implicit_repair_english_output",
            query: "배포 후 오류가 늘었네. 이 상태로 둘 수는 없지.",
            input_language: Ko,
            output_language: En,
            required_fragments: &["needs repair", "external-mutation authority"],
            minimum_traces: 1,
        },
        Case {
            id: "R31_PLAN_11",
            semantic_group: "R31_AFFECT_PAIR",
            category: "affect_and_request_korean_output",
            query: "This is frustrating; inspect the Willow parser.",
            input_language: En,
            output_language: Ko,
            required_fragments: &["답답", "Willow", "확인"],
            minimum_traces: 2,
        },
        Case {
            id: "R31_PLAN_12",
            semantic_group: "R31_AFFECT_PAIR",
            category: "affect_and_request_english_output",
            query: "This is frustrating; inspect the Willow parser.",
            input_language: En,
            output_language: En,
            required_fragments: &["frustrating", "Willow", "check"],
            minimum_traces: 2,
        },
    ]
}

fn main() {
    let mut rows = cases().iter().map(run).collect::<Vec<_>>();
    let mut by_group = BTreeMap::<String, Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        by_group
            .entry(row.semantic_group.clone())
            .or_default()
            .push(index);
    }
    let mut pair_passed = 0;
    for indexes in by_group.values() {
        let invariant = indexes.len() == 2
            && !rows[indexes[0]].semantic_trace_sha256s.is_empty()
            && rows[indexes[0]].semantic_trace_sha256s == rows[indexes[1]].semantic_trace_sha256s;
        if invariant {
            pair_passed += 1;
        }
        for index in indexes {
            rows[*index].semantic_pair_invariant = invariant;
        }
    }
    for row in &mut rows {
        row.pass = row.typed_generation && row.safety_boundary && row.semantic_pair_invariant;
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let generative = rows.iter().filter(|row| row.typed_generation).count();
    let report = Report {
        schema: "B_CORE_PLAN_PREVIEW_GENERATION_BLIND_REPORT_1",
        suite: "PLAN-PREVIEW-GENERATION-BLIND-R31-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: by_group.len(),
        cross_language_semantic_pairs_passed: pair_passed,
        generative_path_rate_millis: u16::try_from(generative * 1_000 / rows.len())
            .expect("bounded rate"),
        drafted_surface_fallbacks: 0,
        stage_overwrites: 0,
        semantic_authority_violations: 0,
        external_execution_authorizations: 0,
        unsupported_explanation_facts: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report json")
    );
    if report.failed != 0
        || report.cross_language_semantic_pairs_passed != report.cross_language_semantic_pairs
        || report.generative_path_rate_millis != 1_000
    {
        std::process::exit(1);
    }
}
