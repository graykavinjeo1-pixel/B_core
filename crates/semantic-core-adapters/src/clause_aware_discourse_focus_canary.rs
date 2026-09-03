//! Frozen R27-RUN-0001 clause-aware discourse focus diagnostic.
//!
//! The public conversation API is the only system under test. Expectations
//! concern structural focus continuity, never a memorized whole sentence.

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

fn current_focus(value: &Value) -> Option<(&str, bool)> {
    let focus = &value["discourse_focus"];
    if focus["schema"].as_str()? != "B_CORE_DISCOURSE_FOCUS_STATE_IR_1" {
        return None;
    }
    let current = focus["current_focus_id"].as_str()?;
    let nodes = focus["nodes"].as_array()?;
    let node = nodes
        .iter()
        .find(|node| node["focus_id"].as_str() == Some(current))?;
    let safe = node["status"].as_str() == Some("PRIMARY")
        && node["semantic_authority"].as_bool() == Some(false)
        && node["external_execution_authorized"].as_bool() == Some(false)
        && node["salience_millis"]
            .as_u64()
            .is_some_and(|score| score <= 1_000)
        && nodes
            .iter()
            .filter(|node| node["status"].as_str() == Some("PRIMARY"))
            .count()
            == 1;
    Some((node["surface"].as_str()?, safe))
}

fn selected_subject(response: &semantic_core_adapters::ConversationTurnResponseIR) -> String {
    response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .map(|candidate| candidate.subject.to_lowercase())
        .unwrap_or_default()
}

fn has_focus_binding(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .iter()
        .flat_map(|binding| binding.evidence.iter())
        .any(|evidence| evidence.starts_with("DISCOURSE_CENTER:"))
}

fn run_case(case: Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut prior = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("setup turn");
    let mut turn = 2;
    for bridge in case.bridges {
        prior = api
            .process_conversation_turn(&request(case.id, turn, bridge, case.language))
            .expect("bridge turn");
        turn += 1;
    }
    let prior_state = serde_json::to_value(&prior.conversation_state).expect("state JSON");
    let focus = current_focus(&prior_state);
    let follow = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("follow turn");
    let resolved = follow
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let subject = selected_subject(&follow);
    let expected = case.expected.to_lowercase();
    let rejected = case.rejected.to_lowercase();
    let focus_surface = focus.map_or("", |(surface, _)| surface).to_lowercase();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        trace: vec![
            format!("focus={focus_surface:?}"),
            format!("resolved={resolved:?}"),
            format!("subject={subject:?}"),
        ],
        pass: prior.disposition == ConversationTurnDispositionIR::Grounded
            && focus.is_some_and(|(_, safe)| safe)
            && focus_surface.contains(&expected)
            && !focus_surface.contains(&rejected)
            && follow.disposition == ConversationTurnDispositionIR::Grounded
            && has_focus_binding(&follow)
            && resolved.contains(&expected)
            && !resolved.contains(&rejected)
            && subject.contains(&expected)
            && follow.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let cases = [
        Case {
            id: "R27_COORD_1",
            category: "coordinated_goal_centering",
            setup: "analyze the cache and repair the queue",
            bridges: &[],
            follow: "inspect it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_COORD_2",
            category: "coordinated_goal_centering",
            setup: "캐시를 분석하고 큐를 수리해",
            bridges: &[],
            follow: "그것을 검사해",
            expected: "큐",
            rejected: "캐시",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_COORD_3",
            category: "coordinated_goal_centering",
            setup: "inspect the server and analyze the log",
            bridges: &[],
            follow: "repair it",
            expected: "log",
            rejected: "server",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_COORD_4",
            category: "coordinated_goal_centering",
            setup: "파일을 검사하고 폴더를 분석해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "폴더",
            rejected: "파일",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_SEQ_1",
            category: "sequence_target_centering",
            setup: "inspect the cache, then repair the queue",
            bridges: &[],
            follow: "analyze it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_SEQ_2",
            category: "sequence_target_centering",
            setup: "서버를 검사한 뒤 로그를 분석해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "로그",
            rejected: "서버",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_SEQ_3",
            category: "sequence_target_centering",
            setup: "analyze the file, then repair the report",
            bridges: &[],
            follow: "inspect it",
            expected: "report",
            rejected: "file",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_SEQ_4",
            category: "sequence_target_centering",
            setup: "문서를 분석한 뒤 백업을 수리해",
            bridges: &[],
            follow: "그것을 검사해",
            expected: "백업",
            rejected: "문서",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_SUB_1",
            category: "subordinate_nucleus_centering",
            setup: "if the cache is stale, inspect the queue",
            bridges: &[],
            follow: "repair it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_SUB_2",
            category: "subordinate_nucleus_centering",
            setup: "inspect the queue because the cache failed",
            bridges: &[],
            follow: "repair it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_SUB_3",
            category: "subordinate_nucleus_centering",
            setup: "캐시가 실패했기 때문에 큐를 검사해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "큐",
            rejected: "캐시",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_SUB_4",
            category: "subordinate_nucleus_centering",
            setup: "서비스를 배포하기 전에 매니페스트를 검사해",
            bridges: &[],
            follow: "그것을 분석해",
            expected: "매니페스트",
            rejected: "서비스",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_CONTRAST_1",
            category: "contrastive_centering",
            setup: "analyze the cache but now repair the queue",
            bridges: &[],
            follow: "inspect it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_CONTRAST_2",
            category: "contrastive_centering",
            setup: "캐시를 분석하지만 이제 큐를 수리해",
            bridges: &[],
            follow: "그것을 검사해",
            expected: "큐",
            rejected: "캐시",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_CONTRAST_3",
            category: "contrastive_centering",
            setup: "Alice said delete the file, but now inspect the logs",
            bridges: &[],
            follow: "repair it",
            expected: "logs",
            rejected: "file",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_CONTRAST_4",
            category: "contrastive_centering",
            setup: "민수는 파일을 삭제하라고 말했지만 이제 로그를 검사해",
            bridges: &[],
            follow: "그것을 수리해",
            expected: "로그",
            rejected: "파일",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_RETAIN_1",
            category: "social_turn_focus_retention",
            setup: "analyze the cache and repair the queue",
            bridges: &["thanks"],
            follow: "inspect it",
            expected: "queue",
            rejected: "cache",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_RETAIN_2",
            category: "social_turn_focus_retention",
            setup: "캐시를 분석하고 큐를 수리해",
            bridges: &["음, 고마워"],
            follow: "그것을 검사해",
            expected: "큐",
            rejected: "캐시",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_RETAIN_3",
            category: "social_turn_focus_retention",
            setup: "inspect the server and analyze the log",
            bridges: &["okay", "thanks"],
            follow: "repair it",
            expected: "log",
            rejected: "server",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_RETAIN_4",
            category: "social_turn_focus_retention",
            setup: "파일을 검사하고 폴더를 분석해",
            bridges: &["음", "알겠어"],
            follow: "그것을 수리해",
            expected: "폴더",
            rejected: "파일",
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R27_ORDER_1",
            category: "surface_order_focus_invariance",
            setup: "before you deploy the service, inspect the manifest",
            bridges: &[],
            follow: "analyze it",
            expected: "manifest",
            rejected: "service",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_ORDER_2",
            category: "surface_order_focus_invariance",
            setup: "inspect the manifest before you deploy the service",
            bridges: &[],
            follow: "analyze it",
            expected: "manifest",
            rejected: "service",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_ORDER_3",
            category: "surface_order_focus_invariance",
            setup: "because the worker reported the error, inspect the log",
            bridges: &[],
            follow: "repair it",
            expected: "log",
            rejected: "error",
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R27_ORDER_4",
            category: "surface_order_focus_invariance",
            setup: "inspect the log because the worker reported the error",
            bridges: &[],
            follow: "repair it",
            expected: "log",
            rejected: "error",
            language: LanguageCodeIR::English,
        },
    ];
    let rows = cases.into_iter().map(run_case).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R27-RUN-0001",
        "frozen_before_product_changes": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("JSON"));
    if passed != 24 {
        std::process::exit(1);
    }
}
