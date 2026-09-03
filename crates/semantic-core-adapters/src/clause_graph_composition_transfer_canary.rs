//! Frozen R26-RUN-0002 held-out transfer for typed clause-graph composition.

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
        graph["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|node| {
                node["kind"].as_str() == Some("EVENT")
                    && node["normalized_label"].as_str() == Some(predicate)
            })
            .filter_map(|node| node["node_id"].as_str())
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
        category: "held_out_shared_argument".to_string(),
        trace: vec![
            format!("edges={edges:?}"),
            serde_json::to_string(&value["semantic_role_graph"]).expect("role graph"),
        ],
        pass: edges.iter().any(|edge| edge == expected_edge)
            && has_shared_theme(&value, predicates, theme)
            && selected_predicates(&value).len() == predicates.len(),
    }
}

fn main() {
    let rows = vec![
        relation_case(
            "R26_TRANSFER_1",
            "held_out_fronted_relation",
            "if you analyze the checksum, restore the worker",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_TRANSFER_2",
            "held_out_fronted_relation",
            "because the agent recorded the anomaly, review the archive",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_TRANSFER_3",
            "held_out_fronted_relation",
            "before you publish the bundle, examine the signature",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_TRANSFER_4",
            "held_out_fronted_relation",
            "after you review the snapshot, restore the queue",
            "INVESTIGATE>REPAIR:TEMPORAL_BEFORE",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_TRANSFER_5",
            "held_out_postposed_relation",
            "restore the worker if you analyze the checksum",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_TRANSFER_6",
            "held_out_postposed_relation",
            "review the archive because the agent recorded the anomaly",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_TRANSFER_7",
            "held_out_postposed_relation",
            "publish the bundle after you examine the signature",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["DEPLOY"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_TRANSFER_8",
            "held_out_postposed_relation",
            "examine the signature before you publish the bundle",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_TRANSFER_9",
            "held_out_korean_relation",
            "체크섬을 분석하면 워커를 복구해",
            "INVESTIGATE>REPAIR:CONDITION",
            &["REPAIR"],
            Some("INVESTIGATE"),
        ),
        relation_case(
            "R26_TRANSFER_10",
            "held_out_korean_relation",
            "이상을 기록했기 때문에 아카이브를 검토해",
            "COMMUNICATE>INVESTIGATE:CAUSE",
            &["INVESTIGATE"],
            Some("COMMUNICATE"),
        ),
        relation_case(
            "R26_TRANSFER_11",
            "held_out_korean_relation",
            "번들을 게시하기 전에 서명을 확인해",
            "INVESTIGATE>DEPLOY:TEMPORAL_BEFORE",
            &["INVESTIGATE"],
            Some("DEPLOY"),
        ),
        relation_case(
            "R26_TRANSFER_12",
            "held_out_korean_relation",
            "스냅샷을 검토한 뒤 큐를 복구해",
            "INVESTIGATE>REPAIR:TEMPORAL_BEFORE",
            &["INVESTIGATE", "REPAIR"],
            None,
        ),
        shared_case(
            "R26_TRANSFER_13",
            "analyze and restore every archive",
            "INVESTIGATE>REPAIR:COORDINATION",
            &["INVESTIGATE", "REPAIR"],
            "archive",
        ),
        shared_case(
            "R26_TRANSFER_14",
            "문서를 읽고 보고해",
            "EXECUTE>COMMUNICATE:SEQUENCE",
            &["EXECUTE", "COMMUNICATE"],
            "문서",
        ),
        relation_case(
            "R26_TRANSFER_15",
            "held_out_subordinate_authority",
            "review the records in order to document the findings",
            "INVESTIGATE>CREATE:PURPOSE",
            &["INVESTIGATE"],
            Some("CREATE"),
        ),
        relation_case(
            "R26_TRANSFER_16",
            "held_out_subordinate_authority",
            "작업자가 큐를 복구했지만 스냅샷을 검토해",
            "REPAIR>INVESTIGATE:CONTRAST",
            &["INVESTIGATE"],
            Some("REPAIR"),
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R26-RUN-0002",
        "held_out_until_after_diagnostic_repairs": true,
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
