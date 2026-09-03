//! Frozen R62 full-axis integration evaluator support.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
// This evaluator is compiled into several independent canary binaries. Each
// frozen suite intentionally exercises only a subset of the shared check DSL.
#[allow(dead_code)]
pub enum Check {
    Act {
        turn: usize,
        act: &'static str,
    },
    Text {
        turn: usize,
        required: &'static [&'static str],
        forbidden: &'static [&'static str],
    },
    Plan {
        turn: usize,
        intent: &'static str,
        target: &'static str,
        rejected: &'static str,
    },
    MultiGoal {
        turn: usize,
        predicates: &'static [&'static str],
        min_blocked: usize,
    },
    Reference {
        turn: usize,
        target: &'static str,
        rejected: &'static str,
    },
    Clarification {
        turn: usize,
    },
    ReportUnverified {
        turn: usize,
    },
    ResultUnavailable {
        turn: usize,
        target: &'static str,
    },
    Links {
        turn: usize,
        active: &'static [&'static str],
    },
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub checks: &'static [Check],
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Serialize)]
struct Suite {
    suite: String,
    frozen_before_product_changes: bool,
    held_out_until_diagnostic_passes: bool,
    passed: usize,
    failed: usize,
    total: usize,
    rows: Vec<Row>,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
}

fn request(case_id: &str, turn: usize, input: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case_id.to_string(),
        turn_index: u64::try_from(turn).expect("bounded turn"),
        request_id: format!("{case_id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: input.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(input.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn response_value(response: &ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn contains_ci(text: &str, fragment: &str) -> bool {
    text.to_lowercase().contains(&fragment.to_lowercase())
}

fn selected_predicates(response: &ConversationTurnResponseIR) -> Vec<String> {
    response
        .pragmatic_interpretation
        .pragmatic_intent_graph
        .composition
        .as_ref()
        .map(|composition| {
            composition
                .nodes
                .iter()
                .filter(|node| composition.selected_node_ids.contains(&node.node_id))
                .map(|node| node.canonical_predicate.to_uppercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn integration_contract(
    response: &ConversationTurnResponseIR,
    request: &ConversationTurnRequestIR,
) -> bool {
    let value = response_value(response);
    let Some(links) = value
        .pointer("/six_axis_integration/cross_axis_links")
        .and_then(Value::as_array)
    else {
        return false;
    };
    let natural_hash = value
        .pointer("/natural_realization/realization_sha256")
        .and_then(Value::as_str);
    let six_natural_hash = value
        .pointer("/six_axis_integration/natural_realization_sha256")
        .and_then(Value::as_str);
    let six_hash = value
        .pointer("/six_axis_integration/integration_sha256")
        .and_then(Value::as_str);
    let receipt_six_hash = value
        .pointer("/language_cortex_integration/six_axis_integration_sha256")
        .and_then(Value::as_str);
    response.schema == "B_CORE_CONVERSATION_TURN_RESPONSE_18"
        && value.pointer("/six_axis_integration/schema")
            == Some(&Value::String(
                "B_CORE_SIX_AXIS_INTEGRATION_IR_2".to_string(),
            ))
        && value.pointer("/language_cortex_integration/schema")
            == Some(&Value::String(
                "B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_5".to_string(),
            ))
        && links.len() == 8
        && links.iter().all(|link| {
            link["satisfied"] == true
                && link["evidence_refs"]
                    .as_array()
                    .is_some_and(|refs| !refs.is_empty())
        })
        && natural_hash.is_some_and(|hash| Some(hash) == six_natural_hash)
        && six_hash.is_some_and(|hash| Some(hash) == receipt_six_hash)
        && value.pointer("/six_axis_integration/complete") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/language_can_execute") == Some(&Value::Bool(false))
        && response.natural_realization.validate()
        && response.grounded_realization.validate()
        && response.natural_realization.realized_text == response.output.text
        && response.grounded_realization.realized_text == response.output.text
        && response.output.unsupported_freeform_claims == 0
        && response.validate_against(request)
}

fn active_links(response: &ConversationTurnResponseIR) -> Vec<String> {
    response_value(response)
        .pointer("/six_axis_integration/cross_axis_links")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|link| link["active"] == true)
        .filter_map(|link| link["kind"].as_str().map(str::to_string))
        .collect()
}

fn check(case_check: Check, responses: &[ConversationTurnResponseIR]) -> (bool, String) {
    let turn = match case_check {
        Check::Act { turn, .. }
        | Check::Text { turn, .. }
        | Check::Plan { turn, .. }
        | Check::MultiGoal { turn, .. }
        | Check::Reference { turn, .. }
        | Check::Clarification { turn }
        | Check::ReportUnverified { turn }
        | Check::ResultUnavailable { turn, .. }
        | Check::Links { turn, .. } => turn,
    };
    let response = &responses[turn - 1];
    let value = response_value(response);
    match case_check {
        Check::Act { act, .. } => {
            let actual = value
                .pointer("/natural_realization/response_act")
                .and_then(Value::as_str)
                .unwrap_or_default();
            (actual == act, format!("turn={turn};act={actual}"))
        }
        Check::Text {
            required,
            forbidden,
            ..
        } => {
            let required_ok = required
                .iter()
                .all(|fragment| contains_ci(&response.output.text, fragment));
            let forbidden_ok = forbidden
                .iter()
                .all(|fragment| !contains_ci(&response.output.text, fragment));
            (
                required_ok && forbidden_ok,
                format!(
                    "turn={turn};required={required_ok};forbidden={forbidden_ok};text={}",
                    response.output.text
                ),
            )
        }
        Check::Plan {
            intent,
            target,
            rejected,
            ..
        } => {
            let actual = response.grounded_response.as_deref();
            let subject = actual
                .map(|item| item.understanding.subject.as_str())
                .unwrap_or_default();
            let actual_intent = actual
                .map(|item| format!("{:?}", item.understanding.intent).to_uppercase())
                .unwrap_or_default();
            let combined = format!(
                "{} {} {}",
                subject, response.reference_resolution.resolved_semantic_text, response.output.text
            );
            let pass = actual.is_some()
                && actual_intent == intent
                && contains_ci(&combined, target)
                && (rejected.is_empty() || !contains_ci(subject, rejected));
            (
                pass,
                format!(
                    "turn={turn};intent={actual_intent};subject={subject};resolved={}",
                    response.reference_resolution.resolved_semantic_text
                ),
            )
        }
        Check::MultiGoal {
            predicates,
            min_blocked,
            ..
        } => {
            let actual = selected_predicates(response);
            let pass = predicates.iter().all(|predicate| {
                actual
                    .iter()
                    .any(|item| item.eq_ignore_ascii_case(predicate))
            }) && response
                .pragmatic_interpretation
                .compositional_analysis
                .blocked_execution_count()
                >= min_blocked;
            (
                pass,
                format!(
                    "turn={turn};predicates={actual:?};blocked={}",
                    response
                        .pragmatic_interpretation
                        .compositional_analysis
                        .blocked_execution_count()
                ),
            )
        }
        Check::Reference {
            target, rejected, ..
        } => {
            let resolved = &response.reference_resolution.resolved_semantic_text;
            let pass = contains_ci(resolved, target)
                && (rejected.is_empty() || !contains_ci(resolved, rejected))
                && (!response.reference_resolution.used_referent_ids.is_empty()
                    || !response.reference_resolution.discourse_bindings.is_empty());
            (pass, format!("turn={turn};resolved={resolved}"))
        }
        Check::Clarification { .. } => {
            let pass = response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && response.grounded_response.is_none()
                && value.pointer("/natural_realization/response_act")
                    == Some(&Value::String("CLARIFICATION_REQUEST".to_string()));
            (
                pass,
                format!(
                    "turn={turn};disposition={:?};ambiguities={:?}",
                    response.disposition,
                    response.reference_resolution.ambiguous_reference_surfaces
                ),
            )
        }
        Check::ReportUnverified { .. } => {
            let reports = value
                .pointer("/conversation_state/action_state_ledger/language_report_history")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let verified_result = value
                .pointer("/interaction_provenance/nodes")
                .and_then(Value::as_array)
                .is_some_and(|nodes| nodes.iter().any(|node| node["kind"] == "VERIFIED_RESULT"));
            (
                reports > 0 && !verified_result,
                format!("turn={turn};reports={reports};verified_result={verified_result}"),
            )
        }
        Check::ResultUnavailable { target, .. } => {
            let unavailable = value
                .pointer("/plan_result_boundary/snapshots")
                .and_then(Value::as_array)
                .is_some_and(|snapshots| {
                    !snapshots.is_empty()
                        && snapshots
                            .iter()
                            .all(|snapshot| snapshot["result_availability"] == "UNAVAILABLE")
                });
            (
                unavailable && contains_ci(&response.output.text, target),
                format!(
                    "turn={turn};unavailable={unavailable};text={}",
                    response.output.text
                ),
            )
        }
        Check::Links { active, .. } => {
            let actual = active_links(response);
            let pass = active
                .iter()
                .all(|kind| actual.iter().any(|item| item == kind));
            (pass, format!("turn={turn};active_links={actual:?}"))
        }
    }
}

fn run(case: &Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut responses = Vec::new();
    let mut requests = Vec::new();
    for (offset, input) in case.turns.iter().copied().enumerate() {
        let request = request(case.id, offset + 1, input);
        let response = api
            .process_conversation_turn(&request)
            .expect("conversation turn");
        requests.push(request);
        responses.push(response);
    }
    let contracts = responses
        .iter()
        .zip(&requests)
        .map(|(response, request)| integration_contract(response, request))
        .collect::<Vec<_>>();
    let mut trace = vec![format!("contracts={contracts:?}")];
    let mut pass = contracts.iter().all(|item| *item);
    for expected in case.checks.iter().copied() {
        let (check_pass, detail) = check(expected, &responses);
        pass &= check_pass;
        trace.push(format!("pass={check_pass};{detail}"));
    }
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace,
    }
}

pub fn emit(suite: &str, held_out: bool, cases: &[Case]) {
    let rows = cases.iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Suite {
        suite: suite.to_string(),
        frozen_before_product_changes: true,
        held_out_until_diagnostic_passes: held_out,
        passed,
        failed: rows.len() - passed,
        total: rows.len(),
        rows,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if report.failed > 0 {
        std::process::exit(1);
    }
}
