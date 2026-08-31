use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CompositionalSemanticAnalyzer, LanguageCodeIR, PredicateLexemeIR, PredicateLexiconSnapshotIR,
    PREDICATE_LEXEME_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
struct GraphCase {
    case_id: &'static str,
    text: &'static str,
    expected_nodes: usize,
    expected_edges: usize,
    expected_conditions: usize,
    expected_prohibitions: usize,
    clarification_required: bool,
}

#[derive(Debug, Serialize)]
struct ResultRow {
    case_id: String,
    nodes: usize,
    edges: usize,
    conditions: usize,
    prohibitions: usize,
    clarification_required: bool,
    pass: bool,
}

fn main() {
    let cases = [
        graph_case(
            "KO_READ_TRANSFORM_SAVE",
            "파일을 읽고 각 줄을 변환한 뒤 저장해. 원본은 지우지 마",
            3,
            2,
            0,
            1,
            false,
        ),
        graph_case(
            "KO_INSPECT_THEN_REPAIR",
            "로그를 확인하고 설정을 수정해",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "KO_ANALYZE_REPAIR_SAVE",
            "코드를 분석하고 수정한 뒤 저장해",
            3,
            2,
            0,
            0,
            false,
        ),
        graph_case(
            "KO_SAVE_AND_REPORT",
            "결과를 저장하고 사용자에게 보고해",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "KO_CREATE_SAVE_DEPLOY_PROHIBITED",
            "초안을 작성하고 저장해. 배포하지 마",
            2,
            1,
            0,
            1,
            false,
        ),
        graph_case(
            "KO_CONDITIONAL_SAVE",
            "파일을 읽고 오류가 없으면 저장해",
            2,
            1,
            1,
            0,
            false,
        ),
        graph_case(
            "KO_CREATE_AND_EXPLAIN",
            "보고서를 작성하고 핵심을 설명해",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "KO_UNCONNECTED_CONFLICT",
            "파일을 분석해; 코드를 수정해",
            0,
            0,
            0,
            0,
            true,
        ),
        graph_case(
            "EN_READ_TRANSFORM_SAVE",
            "Read the file, transform each line, then save it. Do not delete the original.",
            3,
            2,
            0,
            1,
            false,
        ),
        graph_case(
            "EN_INSPECT_AND_REPAIR",
            "Inspect the logs and repair the service.",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "EN_ANALYZE_REPAIR_SAVE",
            "Analyze the code, then fix it, then save it.",
            3,
            2,
            0,
            0,
            false,
        ),
        graph_case(
            "EN_SAVE_AND_REPORT",
            "Save the result and report it.",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "EN_CREATE_SAVE_DEPLOY_PROHIBITED",
            "Create the draft and save it. Do not deploy it.",
            2,
            1,
            0,
            1,
            false,
        ),
        graph_case(
            "EN_CONDITIONAL_SAVE",
            "Read the file, and if it is valid, save it.",
            2,
            1,
            1,
            0,
            false,
        ),
        graph_case(
            "EN_CREATE_AND_EXPLAIN",
            "Create the report and explain its assumptions.",
            2,
            1,
            0,
            0,
            false,
        ),
        graph_case(
            "EN_UNCONNECTED_CONFLICT",
            "Analyze the file; repair the service.",
            0,
            0,
            0,
            0,
            true,
        ),
    ];

    let analyzer = CompositionalSemanticAnalyzer;
    let mut rows = cases
        .into_iter()
        .map(|case| {
            let analysis = analyzer.analyze(case.text);
            let (nodes, edges, conditions, prohibitions) =
                analysis.goal_graph.as_ref().map_or((0, 0, 0, 0), |graph| {
                    (
                        graph.nodes.len(),
                        graph.edges.len(),
                        graph.conditions.len(),
                        graph.prohibitions.len(),
                    )
                });
            ResultRow {
                case_id: case.case_id.to_string(),
                nodes,
                edges,
                conditions,
                prohibitions,
                clarification_required: analysis.clarification_required,
                pass: nodes == case.expected_nodes
                    && edges == case.expected_edges
                    && conditions == case.expected_conditions
                    && prohibitions == case.expected_prohibitions
                    && analysis.clarification_required == case.clarification_required,
            }
        })
        .collect::<Vec<_>>();

    let predicate = PredicateLexemeIR {
        schema: PREDICATE_LEXEME_SCHEMA.to_string(),
        predicate_id: "P-R4-PERSIST-CANARY".to_string(),
        language: LanguageCodeIR::Korean,
        surface_forms: vec!["다듬".to_string()],
        canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
        intent_hint: PlanIntentIR::Create,
        definition: "revise a document into a clearer finished form".to_string(),
        confidence_millis: 930,
    };
    let snapshot = PredicateLexiconSnapshotIR::build(vec![predicate]).expect("snapshot");
    rows.push(ResultRow {
        case_id: "PREDICATE_SNAPSHOT_ROUND_TRIP".to_string(),
        nodes: snapshot.entries.len(),
        edges: 0,
        conditions: 0,
        prohibitions: 0,
        clarification_required: false,
        pass: snapshot.validate().is_ok() && snapshot.snapshot_sha256.len() == 64,
    });
    let mut tampered = snapshot;
    tampered.entries[0].intent_hint = PlanIntentIR::Execute;
    rows.push(ResultRow {
        case_id: "PREDICATE_SNAPSHOT_TAMPER_REJECTED".to_string(),
        nodes: tampered.entries.len(),
        edges: 0,
        conditions: 0,
        prohibitions: 0,
        clarification_required: false,
        pass: tampered.validate().is_err(),
    });

    println!(
        "{}",
        serde_json::to_string(&rows).expect("serialize discourse-program canary")
    );
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}

const fn graph_case(
    case_id: &'static str,
    text: &'static str,
    expected_nodes: usize,
    expected_edges: usize,
    expected_conditions: usize,
    expected_prohibitions: usize,
    clarification_required: bool,
) -> GraphCase {
    GraphCase {
        case_id,
        text,
        expected_nodes,
        expected_edges,
        expected_conditions,
        expected_prohibitions,
        clarification_required,
    }
}
