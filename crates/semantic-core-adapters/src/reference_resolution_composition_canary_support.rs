//! Frozen evaluator support for the R58 reference-composition suites.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct Turn<'a> {
    pub text: &'a str,
    pub language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
pub enum Expectation<'a> {
    Resolved {
        surfaces: &'a [(&'a str, usize)],
        forbidden_markers: &'a [&'a str],
        selected_antecedents: &'a [(&'a str, usize)],
        minimum_mentions: usize,
        minimum_bindings: usize,
    },
    Unresolved {
        live_markers: &'a [&'a str],
        minimum_mentions: usize,
        minimum_unresolved: usize,
    },
}

#[derive(Clone, Copy)]
pub struct Case<'a> {
    pub id: &'a str,
    pub category: &'a str,
    pub setup: &'a [Turn<'a>],
    pub follow: Turn<'a>,
    pub expectation: Expectation<'a>,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    resolved_text: String,
    binding_count: usize,
    mention_count: usize,
    selected_edge_count: usize,
    unresolved_mention_count: usize,
    graph_schema: String,
    trace: Vec<String>,
    pass: bool,
}

fn request(conversation_id: &str, turn_index: u64, turn: Turn<'_>) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(turn.language),
        context_tags: Vec::new(),
        max_plan_steps: 20,
    }
}

fn occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(&needle.to_lowercase()).count()
}

fn array(value: &Value, field: &str) -> Vec<Value> {
    value[field].as_array().cloned().unwrap_or_default()
}

fn graph_is_safe(graph: &Value) -> bool {
    graph["schema"] == "B_CORE_REFERENCE_RESOLUTION_GRAPH_IR_1"
        && graph["semantic_authority"] == false
        && graph["external_execution_authorized"] == false
        && graph["graph_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && graph["source_text_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && graph["resolution_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
}

fn selected_antecedent_count(edges: &[Value], surface: &str) -> usize {
    let surface = surface.to_lowercase();
    edges
        .iter()
        .filter(|edge| edge["selected"] == true)
        .filter(|edge| {
            edge["antecedent_surface"]
                .as_str()
                .is_some_and(|candidate| candidate.to_lowercase().contains(&surface))
        })
        .count()
}

fn run_case(case: Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn_index = 1_u64;
    for setup in case.setup {
        api.process_conversation_turn(&request(case.id, turn_index, *setup))
            .expect("setup turn");
        turn_index += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn_index, case.follow))
        .expect("follow turn");
    let resolution = serde_json::to_value(&response.reference_resolution).expect("resolution json");
    let graph = &resolution["resolution_graph"];
    let mentions = array(graph, "mention_nodes");
    let edges = array(graph, "candidate_edges");
    let unresolved = array(graph, "unresolved_mention_ids");
    let selected_edge_count = edges.iter().filter(|edge| edge["selected"] == true).count();
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let safe = graph_is_safe(graph);
    let pass = match case.expectation {
        Expectation::Resolved {
            surfaces,
            forbidden_markers,
            selected_antecedents,
            minimum_mentions,
            minimum_bindings,
        } => {
            safe && mentions.len() >= minimum_mentions
                && response.reference_resolution.resolved_reference_count >= minimum_bindings
                && selected_edge_count >= minimum_bindings
                && unresolved.is_empty()
                && surfaces
                    .iter()
                    .all(|(surface, count)| occurrences(&resolved, surface) >= *count)
                && forbidden_markers
                    .iter()
                    .all(|marker| !resolved.contains(&marker.to_lowercase()))
                && selected_antecedents
                    .iter()
                    .all(|(surface, count)| selected_antecedent_count(&edges, surface) >= *count)
        }
        Expectation::Unresolved {
            live_markers,
            minimum_mentions,
            minimum_unresolved,
        } => {
            safe && mentions.len() >= minimum_mentions
                && unresolved.len() >= minimum_unresolved
                && selected_edge_count == 0
                && response.reference_resolution.resolved_reference_count == 0
                && live_markers
                    .iter()
                    .all(|marker| resolved.contains(&marker.to_lowercase()))
        }
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        resolved_text: resolved,
        binding_count: response.reference_resolution.resolved_reference_count,
        mention_count: mentions.len(),
        selected_edge_count,
        unresolved_mention_count: unresolved.len(),
        graph_schema: graph["schema"].as_str().unwrap_or_default().to_string(),
        trace: vec![
            format!(
                "ambiguous={:?}",
                response.reference_resolution.ambiguous_reference_surfaces
            ),
            format!("used={:?}", response.reference_resolution.used_referent_ids),
            format!("safe_graph={safe}"),
        ],
        pass,
    }
}

pub fn emit(suite: &str, cases: &[Case<'_>], held_out: bool) {
    let rows = cases.iter().copied().map(run_case).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "suite": suite,
            "frozen_before_product_changes": true,
            "held_out_until_diagnostic_passes": held_out,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_calls": 0,
            "recursive_source_mutations": 0,
            "rows": rows,
        }))
        .expect("suite json")
    );
    if passed != rows.len() {
        std::process::exit(1);
    }
}
