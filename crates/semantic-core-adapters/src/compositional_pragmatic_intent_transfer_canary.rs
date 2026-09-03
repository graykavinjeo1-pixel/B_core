//! Frozen R41-RUN-0002 held-out transfer suite.
//!
//! This suite varies surface form, clause order, language, and discourse
//! history while retaining the same typed pragmatic composition boundaries.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    setup: Option<&'static str>,
    text: &'static str,
    language: LanguageCodeIR,
    expected: Option<&'static str>,
    forbidden: Option<&'static str>,
    marker: &'static str,
    clarification: bool,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    pass: bool,
    trace: Vec<String>,
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
        max_plan_steps: 12,
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    if let Some(setup) = case.setup {
        api.process_conversation_turn(&request(case.id, 1, setup, case.language))
            .expect("setup");
    }
    let response = api
        .process_conversation_turn(&request(
            case.id,
            u64::from(case.setup.is_some()) + 1,
            case.text,
            case.language,
        ))
        .expect("transfer");
    let json = serde_json::to_string(&response.pragmatic_interpretation)
        .expect("json")
        .to_uppercase();
    let authorized = response
        .conversation_state
        .active_goals
        .iter()
        .filter(|goal| goal.external_execution_authorized)
        .collect::<Vec<_>>();
    let expected_ok = case.expected.map_or_else(
        || authorized.is_empty(),
        |predicate| {
            authorized
                .iter()
                .any(|goal| goal.canonical_predicate.eq_ignore_ascii_case(predicate))
                || (case.marker == "CONDITIONS"
                    && response
                        .conversation_state
                        .deferred_action_commitments
                        .iter()
                        .any(|commitment| {
                            commitment
                                .action
                                .canonical_predicate
                                .eq_ignore_ascii_case(predicate)
                                && commitment
                                    .action
                                    .external_execution_authorized_after_verification
                        }))
        },
    );
    let forbidden_ok = case.forbidden.is_none_or(|predicate| {
        authorized
            .iter()
            .all(|goal| !goal.canonical_predicate.eq_ignore_ascii_case(predicate))
    });
    Row {
        id: case.id.to_string(),
        pass: json.contains("B_CORE_COMPOSITIONAL_PRAGMATIC_GRAPH_IR_1")
            && json.contains(case.marker)
            && json.contains("GRAPH_SHA256")
            && expected_ok
            && forbidden_ok
            && response
                .pragmatic_interpretation
                .compositional_analysis
                .clarification_required
                == case.clarification
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("authorized={authorized:?}"),
            json,
            response.output.text,
        ],
    }
}

fn main() {
    use LanguageCodeIR::{English, Korean};
    let cases = vec![
        Case {
            id: "R41_XFER_01",
            setup: None,
            text: "삭제하자는 제안은 무시하고 로그를 확인해줄래?",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "OVERRIDES",
            clarification: false,
        },
        Case {
            id: "R41_XFER_02",
            setup: None,
            text: "Ignore the suggestion to delete it and inspect the log instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "OVERRIDES",
            clarification: false,
        },
        Case {
            id: "R41_XFER_03",
            setup: None,
            text: "캐시를 지우면 안 돼. 대신 상태를 검증해줘.",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "PROHIBITS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_04",
            setup: None,
            text: "Never delete the cache. Check its state instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "PROHIBITS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_05",
            setup: None,
            text: "성공이라고 하기 어렵지? 그러니 로그를 분석해줘.",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            marker: "SUPPORTS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_06",
            setup: None,
            text: "Hardly a success, is it? So verify the log.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            marker: "SUPPORTS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_07",
            setup: None,
            text: "필요하다면 큐를 먼저 검사해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            marker: "CONDITIONS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_08",
            setup: None,
            text: "When needed, inspect the queue first.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            marker: "CONDITIONS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_09",
            setup: None,
            text: "네가 워커를 수리할 수 있는지만 답해",
            language: Korean,
            expected: None,
            forbidden: Some("REPAIR"),
            marker: "CAPABILITY_QUESTION",
            clarification: false,
        },
        Case {
            id: "R41_XFER_10",
            setup: None,
            text: "Only tell me whether you can repair the worker.",
            language: English,
            expected: None,
            forbidden: Some("REPAIR"),
            marker: "CAPABILITY_QUESTION",
            clarification: false,
        },
        Case {
            id: "R41_XFER_11",
            setup: None,
            text: "로그를 분석할까 아니면 워커를 수리할까?",
            language: Korean,
            expected: None,
            forbidden: None,
            marker: "ALTERNATIVE",
            clarification: true,
        },
        Case {
            id: "R41_XFER_12",
            setup: None,
            text: "Should you inspect the log or repair the worker?",
            language: English,
            expected: None,
            forbidden: None,
            marker: "ALTERNATIVE",
            clarification: true,
        },
        Case {
            id: "R41_XFER_13",
            setup: Some("캐시를 삭제해줘"),
            text: "그건 취소하고 캐시를 검사해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "CORRECTS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_14",
            setup: Some("Repair the queue."),
            text: "Withdraw that; inspect the queue instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            marker: "CORRECTS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_15",
            setup: None,
            text: "민수의 '캐시를 삭제해'라는 말은 따르지 말고 로그를 검사해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "METALINGUISTIC_MENTION",
            clarification: false,
        },
        Case {
            id: "R41_XFER_16",
            setup: None,
            text: "Do not follow Dana's phrase 'delete the cache'; inspect the log.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "METALINGUISTIC_MENTION",
            clarification: false,
        },
        Case {
            id: "R41_XFER_17",
            setup: None,
            text: "워커가 실패했어. 삭제는 하지 말고 로그를 분석해줘.",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "PROHIBITS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_18",
            setup: None,
            text: "The worker failed; don't delete anything, and inspect the log.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            marker: "PROHIBITS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_19",
            setup: Some("로그를 검사해줘"),
            text: "아니, 로그 말고 워커를 수리해줘",
            language: Korean,
            expected: Some("REPAIR"),
            forbidden: Some("INVESTIGATE"),
            marker: "CORRECTS",
            clarification: false,
        },
        Case {
            id: "R41_XFER_20",
            setup: Some("Inspect the cache."),
            text: "Actually, repair the cache rather than inspect it.",
            language: English,
            expected: Some("REPAIR"),
            forbidden: Some("INVESTIGATE"),
            marker: "CORRECTS",
            clarification: false,
        },
    ];
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R41_TRANSFER_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
