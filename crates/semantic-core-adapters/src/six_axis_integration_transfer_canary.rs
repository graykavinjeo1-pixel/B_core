//! Frozen R43 held-out transfer suite.
//!
//! This binary is not executed until the diagnostic integration contract
//! passes.  It combines language switching, long interruption, correction,
//! quotation, ambiguity, and report/result separation.

use std::collections::BTreeSet;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA, CONVERSATION_TURN_RESPONSE_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
enum Expectation {
    Reference {
        targets: &'static [&'static str],
        rejected: &'static [&'static str],
    },
    Planned(&'static str),
    NoPlan,
    ReportOnly,
}

struct Case {
    id: &'static str,
    category: &'static str,
    turns: Vec<(&'static str, LanguageCodeIR)>,
    expectation: Expectation,
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn contract(value: &Value) -> bool {
    let axes = value
        .pointer("/six_axis_integration/axes")
        .and_then(Value::as_array);
    let names = axes
        .into_iter()
        .flatten()
        .filter_map(|axis| axis["axis"].as_str())
        .collect::<BTreeSet<_>>();
    value["schema"] == CONVERSATION_TURN_RESPONSE_SCHEMA
        && value.pointer("/six_axis_integration/complete") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/language_can_execute") == Some(&Value::Bool(false))
        && axes.is_some_and(|axes| {
            axes.len() == 6
                && axes.iter().all(|axis| {
                    axis["status"] == "PASS"
                        && axis["semantic_authority"] == false
                        && axis["external_action_executed"] == false
                })
        })
        && names.len() == 6
        && value
            .pointer("/six_axis_integration/cross_axis_invariants")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items.len() >= 6 && items.iter().all(|item| item["satisfied"] == true)
            })
        && value.pointer("/six_axis_integration/package_boundary/valid") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/package_boundary/raw_language_reaches_core")
            == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/package_boundary/adapter_owns_semantic_state")
            == Some(&Value::Bool(false))
        && value
            .pointer("/six_axis_integration/integration_sha256")
            .and_then(Value::as_str)
            .is_some_and(|hash| hash.len() == 64)
}

fn expectation(value: &Value, expected: Expectation) -> bool {
    let verified_result = value
        .pointer("/interaction_provenance/nodes")
        .and_then(Value::as_array)
        .is_some_and(|nodes| nodes.iter().any(|node| node["kind"] == "VERIFIED_RESULT"));
    match expected {
        Expectation::Reference { targets, rejected } => {
            value
                .pointer("/reference_resolution/resolved_semantic_text")
                .and_then(Value::as_str)
                .is_some_and(|text| {
                    let text = text.to_lowercase();
                    targets.iter().any(|target| text.contains(target))
                        && rejected.iter().all(|item| !text.contains(item))
                })
                && value
                    .get("grounded_response")
                    .is_some_and(|item| !item.is_null())
        }
        Expectation::Planned(predicate) => {
            value
                .pointer("/grounded_response/plan/intent")
                .and_then(Value::as_str)
                == Some(predicate)
                && value
                    .get("grounded_response")
                    .is_some_and(|item| !item.is_null())
                && !verified_result
        }
        Expectation::NoPlan => {
            value.get("grounded_response").is_none_or(Value::is_null)
                && value
                    .pointer("/output/grounded_plan_sha256")
                    .is_none_or(Value::is_null)
                && !verified_result
        }
        Expectation::ReportOnly => {
            value
                .pointer("/conversation_state/action_state_ledger/language_report_history")
                .and_then(Value::as_array)
                .is_some_and(|reports| !reports.is_empty())
                && !verified_result
        }
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut response = None;
    for (offset, (text, language)) in case.turns.iter().enumerate() {
        response = Some(
            api.process_conversation_turn(&request(
                case.id,
                u64::try_from(offset + 1).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("turn"),
        );
    }
    let response = response.expect("response");
    let value = serde_json::to_value(&response).expect("json");
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: contract(&value)
            && response.grounded_realization.validate()
            && response.interaction_provenance.validate_against(
                &response.grounded_realization,
                &response.conversation_state.action_state_ledger,
            )
            && response.output.unsupported_freeform_claims == 0
            && expectation(&value, case.expectation),
        trace: vec![response.output.text, value.to_string()],
    }
}

fn cases() -> Vec<Case> {
    use LanguageCodeIR::{English, Korean};
    vec![
        Case { id: "R43_XLANG_01", category: "bilingual_topic_reference", turns: vec![("캐시를 확인해", Korean), ("inspect the worker", English), ("캐시 얘기로 돌아가자", Korean), ("repair it", English)], expectation: Expectation::Reference { targets: &["캐시", "cache"], rejected: &["워커", "worker"] } },
        Case { id: "R43_XLANG_02", category: "bilingual_topic_reference", turns: vec![("inspect the log", English), ("서버를 분석해", Korean), ("return to the log", English), ("그것을 수리해", Korean)], expectation: Expectation::Reference { targets: &["로그", "log"], rejected: &["서버", "server"] } },
        Case { id: "R43_XLANG_03", category: "bilingual_topic_reference", turns: vec![("큐를 확인해", Korean), ("inspect the backup", English), ("큐 이야기로 돌아가자", Korean), ("analyze it", English)], expectation: Expectation::Reference { targets: &["큐", "queue"], rejected: &["백업", "backup"] } },
        Case { id: "R43_XLANG_04", category: "bilingual_topic_reference", turns: vec![("inspect the file", English), ("폴더를 분석해", Korean), ("go back to the file topic", English), ("그거 고쳐", Korean)], expectation: Expectation::Reference { targets: &["파일", "file"], rejected: &["폴더", "folder"] } },
        Case { id: "R43_SCOPE_01", category: "ordered_scope_composition", turns: vec![("캐시는 오래됐고 큐는 막혔다. 전자는 분석하되 후자는 삭제하지 마", Korean)], expectation: Expectation::Planned("INVESTIGATE") },
        Case { id: "R43_SCOPE_02", category: "ordered_scope_composition", turns: vec![("the cache is stale and the queue is blocked. analyze the former but do not delete the latter", English)], expectation: Expectation::Planned("INVESTIGATE") },
        Case { id: "R43_SCOPE_03", category: "ordered_scope_composition", turns: vec![("로그는 비었고 백업은 온전하다. 전자는 확인하되 후자는 지우지 마", Korean)], expectation: Expectation::Planned("INVESTIGATE") },
        Case { id: "R43_SCOPE_04", category: "ordered_scope_composition", turns: vec![("the server is slow and the worker is healthy. inspect the former but never delete the latter", English)], expectation: Expectation::Planned("INVESTIGATE") },
        Case { id: "R43_DELAY_01", category: "long_social_interruption", turns: vec![("캐시를 확인해", Korean), ("워커를 분석해", Korean), ("캐시 얘기로 돌아가자", Korean), ("음...", Korean), ("고마워", Korean), ("잠깐", Korean), ("알겠어", Korean), ("그래", Korean), ("어...", Korean), ("그거 수리해", Korean)], expectation: Expectation::Reference { targets: &["캐시", "cache"], rejected: &["워커", "worker"] } },
        Case { id: "R43_DELAY_02", category: "long_social_interruption", turns: vec![("inspect the log", English), ("inspect the server", English), ("return to the log", English), ("uh...", English), ("thanks", English), ("one moment", English), ("okay", English), ("right", English), ("hmm...", English), ("repair it", English)], expectation: Expectation::Reference { targets: &["로그", "log"], rejected: &["서버", "server"] } },
        Case { id: "R43_DELAY_03", category: "long_social_interruption", turns: vec![("큐를 확인해", Korean), ("백업을 분석해", Korean), ("큐 이야기로 돌아가자", Korean), ("음...", Korean), ("고마워", Korean), ("잠깐", Korean), ("알겠어", Korean), ("그래", Korean), ("어...", Korean), ("그것을 수리해", Korean)], expectation: Expectation::Reference { targets: &["큐", "queue"], rejected: &["백업", "backup"] } },
        Case { id: "R43_DELAY_04", category: "long_social_interruption", turns: vec![("inspect the file", English), ("inspect the folder", English), ("go back to the file topic", English), ("uh...", English), ("thanks", English), ("one moment", English), ("okay", English), ("right", English), ("hmm...", English), ("repair it", English)], expectation: Expectation::Reference { targets: &["파일", "file"], rejected: &["폴더", "folder"] } },
        Case { id: "R43_REVISION_01", category: "bilingual_report_revision", turns: vec![("캐시를 검사해줘", Korean), ("I finished it.", English), ("정정할게, 실패했어", Korean), ("What was reported?", English)], expectation: Expectation::ReportOnly },
        Case { id: "R43_REVISION_02", category: "bilingual_report_revision", turns: vec![("Inspect the queue.", English), ("그거 끝냈어", Korean), ("Correction: it failed.", English), ("그 보고 상태는?", Korean)], expectation: Expectation::ReportOnly },
        Case { id: "R43_REVISION_03", category: "bilingual_report_revision", turns: vec![("로그를 분석해줘", Korean), ("It is underway.", English), ("아니, 끝냈어", Korean), ("What was reported?", English)], expectation: Expectation::ReportOnly },
        Case { id: "R43_REVISION_04", category: "bilingual_report_revision", turns: vec![("Repair the parser.", English), ("진행 중이야", Korean), ("Actually, it failed.", English), ("그 보고 상태는?", Korean)], expectation: Expectation::ReportOnly },
        Case { id: "R43_ATTACK_01", category: "quoted_or_ambiguous_authority", turns: vec![("'큐를 삭제해'라는 문장을 설명해줘", Korean)], expectation: Expectation::Planned("EXPLAIN") },
        Case { id: "R43_ATTACK_02", category: "quoted_or_ambiguous_authority", turns: vec![("Explain the sentence 'repair the worker'.", English)], expectation: Expectation::Planned("EXPLAIN") },
        Case { id: "R43_ATTACK_03", category: "quoted_or_ambiguous_authority", turns: vec![("캐시를 수리하거나 로그를 삭제해줘", Korean)], expectation: Expectation::NoPlan },
        Case { id: "R43_ATTACK_04", category: "quoted_or_ambiguous_authority", turns: vec![("Either inspect the queue or repair the server.", English)], expectation: Expectation::NoPlan },
    ]
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R43_TRANSFER_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
