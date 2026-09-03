//! Frozen R21-RUN-0002 held-out transfer and ambiguity attacks.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
    pass: bool,
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

fn has_local_order_binding(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .iter()
        .any(|binding| {
            binding
                .evidence
                .iter()
                .any(|item| item == "SYNTACTIC_PRIORITY:LOCAL_ORDERED_ANTECEDENTS")
        })
}

struct CrossPreviousCase<'a> {
    id: &'a str,
    first: (&'a str, LanguageCodeIR),
    second: (&'a str, LanguageCodeIR),
    first_shift: (&'a str, LanguageCodeIR),
    second_shift: (&'a str, LanguageCodeIR),
    previous: (&'a str, LanguageCodeIR),
    action: (&'a str, LanguageCodeIR),
    target: &'a str,
    rejected: &'a str,
}

fn cross_previous_case(case: CrossPreviousCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, (text, language)) in [
        case.first,
        case.second,
        case.first_shift,
        case.second_shift,
        case.previous,
    ]
    .into_iter()
    .enumerate()
    {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            language,
        ))
        .expect("cross-language topic turn");
    }
    let action = api
        .process_conversation_turn(&request(case.id, 6, case.action.0, case.action.1))
        .expect("cross-language resumed action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "cross_language_previous_topic".to_string(),
        trace: vec![resolved.clone(), action.output.text.clone()],
        pass: action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected)
            && action.output.unsupported_freeform_claims == 0,
    }
}

struct ContrastOrderCase<'a> {
    id: &'a str,
    text: &'a str,
    first: &'a str,
    second: &'a str,
    language: LanguageCodeIR,
}

fn contrast_order_case(case: ContrastOrderCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("contrast order");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let subjects = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.subject.to_lowercase())
        .collect::<Vec<_>>();
    Row {
        id: case.id.to_string(),
        category: "contrastive_local_order_transfer".to_string(),
        trace: vec![resolved, format!("subjects={subjects:?}")],
        pass: has_local_order_binding(&response)
            && subjects.len() == 2
            && subjects.iter().any(|subject| subject.contains(case.first))
            && subjects.iter().any(|subject| subject.contains(case.second)),
    }
}

struct LongOpenCase<'a> {
    id: &'a str,
    first: &'a str,
    distractor: &'a str,
    shift: &'a str,
    action: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn long_open_case(case: LongOpenCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("open first");
    api.process_conversation_turn(&request(case.id, 2, case.distractor, case.language))
        .expect("open distractor");
    api.process_conversation_turn(&request(case.id, 3, case.shift, case.language))
        .expect("open shift");
    let social = if case.language == LanguageCodeIR::Korean {
        [
            "음...",
            "고마워",
            "잠깐",
            "그래",
            "알겠어",
            "어...",
            "좋아",
            "응",
        ]
    } else {
        [
            "uh...",
            "thanks",
            "one moment",
            "right",
            "okay",
            "hmm...",
            "good",
            "yes",
        ]
    };
    for (offset, text) in social.into_iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(offset + 4).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("long open delay");
    }
    let action = api
        .process_conversation_turn(&request(case.id, 12, case.action, case.language))
        .expect("long open action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "long_horizon_open_vocabulary_focus".to_string(),
        trace: vec![resolved.clone(), action.output.text.clone()],
        pass: action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected),
    }
}

struct ThreeWayAmbiguityCase<'a> {
    id: &'a str,
    text: &'a str,
    language: LanguageCodeIR,
}

fn three_way_ambiguity_case(case: ThreeWayAmbiguityCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("three-way ambiguity");
    Row {
        id: case.id.to_string(),
        category: "three_local_candidates_fail_closed".to_string(),
        trace: vec![
            response.reference_resolution.resolved_semantic_text.clone(),
            format!(
                "ambiguity={:?}",
                response.reference_resolution.ambiguous_reference_surfaces
            ),
            response.output.text.clone(),
        ],
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.grounded_response.is_none()
            && response.output.grounded_plan_sha256.is_none()
            && !response
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty(),
    }
}

fn main() {
    let rows = vec![
        cross_previous_case(CrossPreviousCase { id: "R21_TRANSFER_PREVIOUS_1", first: ("캐시를 확인해", LanguageCodeIR::Korean), second: ("inspect the log", LanguageCodeIR::English), first_shift: ("return to the cache", LanguageCodeIR::English), second_shift: ("로그 얘기로 돌아가자", LanguageCodeIR::Korean), previous: ("go back to the previous topic", LanguageCodeIR::English), action: ("그거 수리해", LanguageCodeIR::Korean), target: "캐시", rejected: "로그" }),
        cross_previous_case(CrossPreviousCase { id: "R21_TRANSFER_PREVIOUS_2", first: ("inspect the file", LanguageCodeIR::English), second: ("폴더를 확인해", LanguageCodeIR::Korean), first_shift: ("파일 이야기로 돌아가자", LanguageCodeIR::Korean), second_shift: ("return to the folder", LanguageCodeIR::English), previous: ("이전 주제로 돌아가자", LanguageCodeIR::Korean), action: ("repair it", LanguageCodeIR::English), target: "file", rejected: "folder" }),
        cross_previous_case(CrossPreviousCase { id: "R21_TRANSFER_PREVIOUS_3", first: ("큐를 확인해", LanguageCodeIR::Korean), second: ("inspect the backup", LanguageCodeIR::English), first_shift: ("go back to the queue topic", LanguageCodeIR::English), second_shift: ("백업 이야기로 돌아가자", LanguageCodeIR::Korean), previous: ("return to the prior topic", LanguageCodeIR::English), action: ("그것을 분석해", LanguageCodeIR::Korean), target: "큐", rejected: "백업" }),
        cross_previous_case(CrossPreviousCase { id: "R21_TRANSFER_PREVIOUS_4", first: ("inspect the server", LanguageCodeIR::English), second: ("워커를 확인해", LanguageCodeIR::Korean), first_shift: ("서버 얘기로 돌아가자", LanguageCodeIR::Korean), second_shift: ("return to the worker", LanguageCodeIR::English), previous: ("아까 주제로 돌아가자", LanguageCodeIR::Korean), action: ("repair it", LanguageCodeIR::English), target: "server", rejected: "worker" }),
        contrast_order_case(ContrastOrderCase { id: "R21_TRANSFER_ORDER_1", text: "서버는 정상인 반면 캐시는 오래됐다. 전자를 확인하고 후자를 수리해", first: "서버", second: "캐시", language: LanguageCodeIR::Korean }),
        contrast_order_case(ContrastOrderCase { id: "R21_TRANSFER_ORDER_2", text: "although the worker is healthy, the queue is stale; inspect the former and repair the latter", first: "worker", second: "queue", language: LanguageCodeIR::English }),
        contrast_order_case(ContrastOrderCase { id: "R21_TRANSFER_ORDER_3", text: "백업은 온전하지만 로그는 비었다. 전자를 분석하고 후자를 확인해", first: "백업", second: "로그", language: LanguageCodeIR::Korean }),
        contrast_order_case(ContrastOrderCase { id: "R21_TRANSFER_ORDER_4", text: "the folder is intact whereas the file is stale; analyze the former and repair the latter", first: "folder", second: "file", language: LanguageCodeIR::English }),
        long_open_case(LongOpenCase { id: "R21_TRANSFER_OPEN_1", first: "인덱서를 확인해", distractor: "캐시를 분석해", shift: "인덱서 얘기로 돌아가자", action: "그거 수리해", target: "인덱서", rejected: "캐시", language: LanguageCodeIR::Korean }),
        long_open_case(LongOpenCase { id: "R21_TRANSFER_OPEN_2", first: "inspect the orchestrator", distractor: "inspect the log", shift: "return to the orchestrator", action: "repair it", target: "orchestrator", rejected: "log", language: LanguageCodeIR::English }),
        long_open_case(LongOpenCase { id: "R21_TRANSFER_OPEN_3", first: "트랜스포터를 확인해", distractor: "큐를 분석해", shift: "트랜스포터 이야기로 돌아가자", action: "그것을 수리해", target: "트랜스포터", rejected: "큐", language: LanguageCodeIR::Korean }),
        long_open_case(LongOpenCase { id: "R21_TRANSFER_OPEN_4", first: "inspect the dispatcher", distractor: "inspect the server", shift: "go back to the dispatcher topic", action: "repair it", target: "dispatcher", rejected: "server", language: LanguageCodeIR::English }),
        three_way_ambiguity_case(ThreeWayAmbiguityCase { id: "R21_TRANSFER_AMBIG_1", text: "파일은 오래됐고 폴더는 비었고 보고서는 낡았다. 전자를 분석하고 후자를 수리해", language: LanguageCodeIR::Korean }),
        three_way_ambiguity_case(ThreeWayAmbiguityCase { id: "R21_TRANSFER_AMBIG_2", text: "the file is stale, the folder is empty, and the report is old. analyze the former and repair the latter", language: LanguageCodeIR::English }),
        three_way_ambiguity_case(ThreeWayAmbiguityCase { id: "R21_TRANSFER_AMBIG_3", text: "캐시는 오래됐고 큐는 막혔고 로그는 비었다. 전자를 수리하고 후자를 분석해", language: LanguageCodeIR::Korean }),
        three_way_ambiguity_case(ThreeWayAmbiguityCase { id: "R21_TRANSFER_AMBIG_4", text: "the server is slow, the worker is blocked, and the log is incomplete. repair the former and analyze the latter", language: LanguageCodeIR::English }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R21-RUN-0002",
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
