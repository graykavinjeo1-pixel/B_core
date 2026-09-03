//! Frozen R26-RUN-0001 diagnostic for typed clause-graph composition.

use semantic_core_adapters::CompositionalSemanticAnalyzer;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
    pass: bool,
}

fn analysis(text: &str) -> Value {
    serde_json::to_value(CompositionalSemanticAnalyzer.analyze(text)).expect("analysis json")
}

fn frame_predicate(value: &Value, frame_id: &str) -> Option<String> {
    value["frames"].as_array()?.iter().find_map(|frame| {
        (frame["frame_id"].as_str()? == frame_id)
            .then(|| frame["canonical_predicate"].as_str().map(str::to_string))
            .flatten()
    })
}

fn clause_predicate(value: &Value, clause_id: &str) -> Option<String> {
    value["clause_graph"]["nodes"]
        .as_array()?
        .iter()
        .find_map(|node| {
            (node["clause_id"].as_str()? == clause_id)
                .then(|| node["canonical_predicate"].as_str().map(str::to_string))
                .flatten()
        })
}

fn edge_signatures(value: &Value) -> Vec<String> {
    let mut signatures = value["clause_graph"]["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|edge| {
            let source = clause_predicate(value, edge["source_clause_id"].as_str()?)?;
            let target = clause_predicate(value, edge["target_clause_id"].as_str()?)?;
            Some(format!("{source}>{target}:{}", edge["relation"].as_str()?))
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures
}

fn selected_predicates(value: &Value) -> Vec<String> {
    let selected_ids = value["selected_candidate_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut selected = value["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|candidate| {
            candidate["candidate_id"]
                .as_str()
                .is_some_and(|id| selected_ids.contains(&id))
        })
        .filter_map(|candidate| frame_predicate(value, candidate["source_frame_id"].as_str()?))
        .collect::<Vec<_>>();
    selected.sort();
    selected
}

fn candidate_is_nonviable(value: &Value, predicate: &str) -> bool {
    value["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|candidate| {
            frame_predicate(
                value,
                candidate["source_frame_id"].as_str().unwrap_or_default(),
            )
            .is_some_and(|candidate_predicate| candidate_predicate == predicate)
                && candidate["disposition"].as_str() != Some("VIABLE")
                && candidate["external_execution_authorized"].as_bool() == Some(false)
        })
}

fn has_shared_theme(value: &Value, predicates: &[&str], theme: &str) -> bool {
    let graph = &value["semantic_role_graph"];
    let mut required = BTreeMap::new();
    for predicate in predicates {
        *required.entry(*predicate).or_insert(0usize) += 1;
    }
    required.into_iter().all(|(predicate, required_count)| {
        let event_ids = graph["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|node| {
                node["kind"].as_str() == Some("EVENT")
                    && node["normalized_label"].as_str() == Some(predicate)
            })
            .filter_map(|node| node["node_id"].as_str())
            .collect::<Vec<_>>();
        event_ids
            .iter()
            .filter(|event_id| {
                graph["role_edges"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|edge| {
                        edge["event_node_id"].as_str() == Some(*event_id)
                            && matches!(edge["role"].as_str(), Some("THEME" | "PATIENT"))
                    })
                    .filter_map(|edge| edge["argument_node_id"].as_str())
                    .any(|argument_id| {
                        graph["nodes"].as_array().into_iter().flatten().any(|node| {
                            node["node_id"].as_str() == Some(argument_id)
                                && node["normalized_label"]
                                    .as_str()
                                    .is_some_and(|label| label.contains(theme))
                        })
                    })
            })
            .count()
            >= required_count
    })
}

fn relation_case(
    id: &str,
    category: &str,
    text: &str,
    expected_edge: &str,
    selected: &[&str],
    subordinate: Option<&str>,
) -> Row {
    let value = analysis(text);
    let edges = edge_signatures(&value);
    let selected_actual = selected_predicates(&value);
    let mut selected_expected = selected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    selected_expected.sort();
    Row {
        id: id.to_string(),
        category: category.to_string(),
        trace: vec![
            format!("edges={edges:?}"),
            format!("selected={selected_actual:?}"),
        ],
        pass: value["clause_graph"]["schema"].as_str() == Some("B_CORE_CLAUSE_GRAPH_IR_1")
            && edges.iter().any(|edge| edge == expected_edge)
            && selected_actual == selected_expected
            && subordinate.is_none_or(|predicate| candidate_is_nonviable(&value, predicate)),
    }
}

fn shared_case(id: &str, text: &str, expected_edge: &str, predicates: &[&str], theme: &str) -> Row {
    let value = analysis(text);
    let edges = edge_signatures(&value);
    Row {
        id: id.to_string(),
        category: "shared_argument_composition".to_string(),
        trace: vec![
            format!("edges={edges:?}"),
            serde_json::to_string(&value["semantic_role_graph"]).expect("role graph"),
        ],
        pass: edges.iter().any(|edge| edge == expected_edge)
            && has_shared_theme(&value, predicates, theme)
            && selected_predicates(&value).len() == 2,
    }
}

fn equivalence_case(
    id: &str,
    left: &str,
    right: &str,
    expected_edge: &str,
    selected: &[&str],
) -> Row {
    let left_value = analysis(left);
    let right_value = analysis(right);
    let left_edges = edge_signatures(&left_value);
    let right_edges = edge_signatures(&right_value);
    let mut expected_selected = selected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    expected_selected.sort();
    Row {
        id: id.to_string(),
        category: "surface_order_invariance".to_string(),
        trace: vec![
            format!("left={left_edges:?}"),
            format!("right={right_edges:?}"),
        ],
        pass: left_edges == right_edges
            && left_edges.iter().any(|edge| edge == expected_edge)
            && selected_predicates(&left_value) == expected_selected
            && selected_predicates(&right_value) == expected_selected,
    }
}

fn main() {
    let rows = vec![
        relation_case(
            "R26_FRONT_1",
            "fronted_relation_direction",
            "if you inspect the manifest, repair the parser",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_FRONT_2",
            "fronted_relation_direction",
            "because the worker reported the error, inspect the log",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_FRONT_3",
            "fronted_relation_direction",
            "before you deploy the service, inspect the manifest",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_FRONT_4",
            "fronted_relation_direction",
            "after you inspect the manifest, repair the parser",
            "INVESTIGATE>REPAIR:TEMPORAL_BEFORE",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_POST_1",
            "postposed_relation_direction",
            "repair the parser if you inspect the manifest",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_POST_2",
            "postposed_relation_direction",
            "inspect the log because the worker reported the error",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_POST_3",
            "postposed_relation_direction",
            "deploy the service after you inspect the manifest",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["DEPLOY"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_POST_4",
            "postposed_relation_direction",
            "inspect the manifest before you deploy the service",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_KO_1",
            "korean_subordinate_direction",
            "파일을 검사하면 파서를 수리해",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_KO_2",
            "korean_subordinate_direction",
            "오류를 보고했기 때문에 로그를 검사해",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_KO_3",
            "korean_subordinate_direction",
            "서비스를 배포하기 전에 매니페스트를 검사해",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_KO_4",
            "korean_subordinate_direction",
            "매니페스트를 검사한 뒤 파서를 수리해",
            "INVESTIGATE>REPAIR:TEMPORAL_BEFORE",
            &["INVESTIGATE", "REPAIR"],
            None,
        ),
        shared_case(
            "R26_SHARED_1",
            "read and save the archive",
            "EXECUTE>EXECUTE:COORDINATION",
            &["EXECUTE", "EXECUTE"],
            "archive",
        ),
        shared_case(
            "R26_SHARED_2",
            "inspect and repair every bundle",
            "INVESTIGATE>REPAIR:COORDINATION",
            &["INVESTIGATE", "REPAIR"],
            "bundle",
        ),
        shared_case(
            "R26_SHARED_3",
            "아카이브를 읽고 저장해",
            "EXECUTE>EXECUTE:SEQUENCE",
            &["EXECUTE", "EXECUTE"],
            "아카이브",
        ),
        shared_case(
            "R26_SHARED_4",
            "모든 묶음을 검사하고 수리해",
            "INVESTIGATE>REPAIR:SEQUENCE",
            &["INVESTIGATE", "REPAIR"],
            "묶음",
        ),
        relation_case(
            "R26_SCOPE_1",
            "subordinate_authority",
            "inspect the logs in order to repair the parser",
            "INVESTIGATE>REPAIR:PURPOSE",
            &["INVESTIGATE"],
            Some("REPAIR"),
        ),
        relation_case(
            "R26_SCOPE_2",
            "subordinate_authority",
            "파서를 수리하기 위해 로그를 검사해",
            "INVESTIGATE>REPAIR:PURPOSE",
            &["INVESTIGATE"],
            Some("REPAIR"),
        ),
        relation_case(
            "R26_SCOPE_3",
            "subordinate_authority",
            "although the worker repaired the parser, inspect the logs",
            "REPAIR>INVESTIGATE:CONTRAST",
            &["INVESTIGATE"],
            Some("REPAIR"),
        ),
        relation_case(
            "R26_SCOPE_4",
            "subordinate_authority",
            "워커가 파서를 수리했지만 로그를 검사해",
            "REPAIR>INVESTIGATE:CONTRAST",
            &["INVESTIGATE"],
            Some("REPAIR"),
        ),
        equivalence_case(
            "R26_EQ_1",
            "if you inspect the manifest, repair the parser",
            "repair the parser if you inspect the manifest",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
        ),
        equivalence_case(
            "R26_EQ_2",
            "because the worker reported the error, inspect the log",
            "inspect the log because the worker reported the error",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
        ),
        equivalence_case(
            "R26_EQ_3",
            "before you deploy the service, inspect the manifest",
            "inspect the manifest before you deploy the service",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
        ),
        equivalence_case(
            "R26_EQ_4",
            "파서를 수리하기 위해 로그를 검사해",
            "inspect the log in order to repair the parser",
            "INVESTIGATE>REPAIR:PURPOSE",
            &["INVESTIGATE"],
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R26-RUN-0001",
        "frozen_before_product_changes": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
    if passed != payload["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
