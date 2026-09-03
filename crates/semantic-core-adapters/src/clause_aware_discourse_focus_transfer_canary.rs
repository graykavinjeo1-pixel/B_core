//! Frozen R27-RUN-0002 held-out clause-aware focus transfer suite.
//!
//! Keep this suite unexecuted until after the diagnostic implementation.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
    pass: bool,
}

#[derive(Clone, Copy)]
struct Case<'a> {
    id: &'a str,
    category: &'a str,
    setup: &'a str,
    bridges: &'a [&'a str],
    follow: &'a str,
    expected: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
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
        max_plan_steps: 20,
    }
}

fn focus_surface(value: &Value) -> Option<&str> {
    let focus = &value["discourse_focus"];
    let current = focus["current_focus_id"].as_str()?;
    focus["nodes"].as_array()?.iter().find_map(|node| {
        (node["focus_id"].as_str() == Some(current)
            && node["status"].as_str() == Some("PRIMARY")
            && node["semantic_authority"].as_bool() == Some(false)
            && node["external_execution_authorized"].as_bool() == Some(false))
        .then(|| node["surface"].as_str())
        .flatten()
    })
}

fn run_case(case: Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut prior = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("setup");
    let mut turn = 2;
    for bridge in case.bridges {
        prior = api
            .process_conversation_turn(&request(case.id, turn, bridge, case.language))
            .expect("bridge");
        turn += 1;
    }
    let state = serde_json::to_value(&prior.conversation_state).expect("state JSON");
    let focus = focus_surface(&state).unwrap_or_default().to_lowercase();
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("follow");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let subject = response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .map(|candidate| candidate.subject.to_lowercase())
        .unwrap_or_default();
    let binding = response
        .reference_resolution
        .discourse_bindings
        .iter()
        .flat_map(|binding| binding.evidence.iter())
        .any(|evidence| evidence.starts_with("DISCOURSE_CENTER:"));
    let expected = case.expected.to_lowercase();
    let rejected = case.rejected.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        trace: vec![focus.clone(), resolved.clone(), subject.clone()],
        pass: prior.disposition == ConversationTurnDispositionIR::Grounded
            && focus.contains(&expected)
            && !focus.contains(&rejected)
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && binding
            && resolved.contains(&expected)
            && !resolved.contains(&rejected)
            && subject.contains(&expected)
            && response.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let cases = [
        Case {
            id: "R27_TRANSFER_1",
            category: "held_out_coordination",
            setup: "review the archive and restore the bundle",
            bridges: &[],
            follow: "inspect it",
            expected: "bundle",
            rejected: "archive",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_2",
            category: "held_out_coordination",
            setup: "아카이브를 검토하고 묶음을 복구해",
            bridges: &[],
            follow: "그것을 검사해",
            expected: "묶음",
            rejected: "아카이브",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_3",
            category: "held_out_coordination",
            setup: "inspect the parser and analyze the snapshot",
            bridges: &[],
            follow: "repair it",
            expected: "snapshot",
            rejected: "parser",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_4",
            category: "held_out_coordination",
            setup: "매니페스트를 검사하고 스냅샷을 분석해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "스냅샷",
            rejected: "매니페스트",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_5",
            category: "held_out_sequence",
            setup: "review the manifest, then restore the parser",
            bridges: &[],
            follow: "analyze it",
            expected: "parser",
            rejected: "manifest",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_6",
            category: "held_out_sequence",
            setup: "설정을 검사한 뒤 워커를 복구해",
            bridges: &[],
            follow: "그것을 분석해",
            expected: "워커",
            rejected: "설정",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_7",
            category: "held_out_sequence",
            setup: "analyze the archive, then repair the worker",
            bridges: &[],
            follow: "inspect it",
            expected: "worker",
            rejected: "archive",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_8",
            category: "held_out_sequence",
            setup: "큐를 분석한 뒤 백업을 수리해",
            bridges: &[],
            follow: "그것을 검사해",
            expected: "백업",
            rejected: "큐",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_9",
            category: "held_out_subordinate",
            setup: "because the archive failed, inspect the worker",
            bridges: &[],
            follow: "repair it",
            expected: "worker",
            rejected: "archive",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_10",
            category: "held_out_subordinate",
            setup: "inspect the worker because the archive failed",
            bridges: &[],
            follow: "repair it",
            expected: "worker",
            rejected: "archive",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_11",
            category: "held_out_subordinate",
            setup: "백업이 실패했기 때문에 큐를 검사해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "큐",
            rejected: "백업",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_12",
            category: "held_out_subordinate",
            setup: "워커를 복구하기 위해 스냅샷을 검사해",
            bridges: &[],
            follow: "그것을 분석해",
            expected: "스냅샷",
            rejected: "워커",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_13",
            category: "held_out_retention",
            setup: "review the archive and restore the bundle",
            bridges: &["thanks", "okay"],
            follow: "inspect it",
            expected: "bundle",
            rejected: "archive",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_14",
            category: "held_out_retention",
            setup: "아카이브를 검토하고 묶음을 복구해",
            bridges: &["음", "고마워"],
            follow: "그것을 검사해",
            expected: "묶음",
            rejected: "아카이브",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_TRANSFER_15",
            category: "held_out_retention",
            setup: "inspect the parser and analyze the snapshot",
            bridges: &["right", "thanks"],
            follow: "repair it",
            expected: "snapshot",
            rejected: "parser",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_TRANSFER_16",
            category: "held_out_retention",
            setup: "설정을 검사한 뒤 워커를 복구해",
            bridges: &["음", "알겠어"],
            follow: "그것을 분석해",
            expected: "워커",
            rejected: "설정",
            language: LanguageCodeIR::Korean,
        },
    ];
    let rows = cases.into_iter().map(run_case).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R27-RUN-0002",
        "held_out_until_after_diagnostic_repairs": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("JSON"));
    if passed != 16 {
        std::process::exit(1);
    }
}
