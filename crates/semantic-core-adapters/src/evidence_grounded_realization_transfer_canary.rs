//! Frozen R31 held-out transfer suite.
//!
//! This suite is intentionally not executed until the diagnostic product
//! repair passes.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
struct Case {
    id: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    language: LanguageCodeIR,
    kind: &'static str,
    support: &'static str,
    epistemic: &'static str,
    category: &'static str,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
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
    let mut turn = 1;
    if !case.setup.is_empty() {
        api.process_conversation_turn(&request(case.id, turn, case.setup, case.language))
            .expect("setup turn");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow_up, case.language))
        .expect("transfer turn");
    let value = serde_json::to_value(&response).expect("response json");
    let claims = value
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array);
    let expected = claims.is_some_and(|claims| {
        claims.iter().any(|claim| {
            claim["kind"] == case.kind
                && claim["support_status"] == case.support
                && claim["epistemic_status"] == case.epistemic
                && claim["semantic_authority"] == false
                && claim["external_action_executed"] == false
        })
    });
    let pass = expected
        && value.pointer("/grounded_realization/faithful") == Some(&Value::Bool(true))
        && value.pointer("/grounded_realization/unsupported_claims")
            == Some(&Value::Number(0.into()))
        && value.pointer("/grounded_realization/realized_text") == value.pointer("/output/text");
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace: vec![value.to_string()],
    }
}

fn main() {
    let cases = [
        Case {
            id: "R31_X_PLAN_EN",
            setup: "",
            follow_up: "inspect the migration queue",
            language: LanguageCodeIR::English,
            kind: "PLAN_STATUS",
            support: "STRUCTURALLY_GROUNDED",
            epistemic: "PLANNED",
            category: "plan_transfer",
        },
        Case {
            id: "R31_X_PLAN_KO",
            setup: "",
            follow_up: "마이그레이션 큐를 검사해",
            language: LanguageCodeIR::Korean,
            kind: "PLAN_STATUS",
            support: "STRUCTURALLY_GROUNDED",
            epistemic: "PLANNED",
            category: "plan_transfer",
        },
        Case {
            id: "R31_X_REPORT_EN",
            setup: "repair the decoder",
            follow_up: "I could not finish it",
            language: LanguageCodeIR::English,
            kind: "LANGUAGE_REPORT",
            support: "REPORTED_ONLY",
            epistemic: "REPORTED",
            category: "report_transfer",
        },
        Case {
            id: "R31_X_REPORT_KO",
            setup: "디코더를 수리해",
            follow_up: "끝내지 못했어",
            language: LanguageCodeIR::Korean,
            kind: "LANGUAGE_REPORT",
            support: "REPORTED_ONLY",
            epistemic: "REPORTED",
            category: "report_transfer",
        },
        Case {
            id: "R31_X_ATTR_EN",
            setup: "Nora believes that the cache is stale.",
            follow_up: "What does Nora believe?",
            language: LanguageCodeIR::English,
            kind: "ATTRIBUTED_DIALOGUE_RECORD",
            support: "DERIVED_FROM_DIALOGUE_RECORDS",
            epistemic: "DERIVED",
            category: "attribution_transfer",
        },
        Case {
            id: "R31_X_ATTR_KO",
            setup: "지수는 캐시가 오래됐다고 믿는다.",
            follow_up: "지수는 무엇을 믿어?",
            language: LanguageCodeIR::Korean,
            kind: "ATTRIBUTED_DIALOGUE_RECORD",
            support: "DERIVED_FROM_DIALOGUE_RECORDS",
            epistemic: "DERIVED",
            category: "attribution_transfer",
        },
        Case {
            id: "R31_X_TIME_EN",
            setup: "The monitor failed during the deploy run.",
            follow_up: "What happened during the deploy run?",
            language: LanguageCodeIR::English,
            kind: "TEMPORAL_RELATION",
            support: "DERIVED_FROM_DIALOGUE_RECORDS",
            epistemic: "DERIVED",
            category: "temporal_transfer",
        },
        Case {
            id: "R31_X_TIME_KO",
            setup: "배포가 실행되는 동안 모니터가 실패했다.",
            follow_up: "배포가 실행되는 동안 무슨 일이 있었어?",
            language: LanguageCodeIR::Korean,
            kind: "TEMPORAL_RELATION",
            support: "DERIVED_FROM_DIALOGUE_RECORDS",
            epistemic: "DERIVED",
            category: "temporal_transfer",
        },
        Case {
            id: "R31_X_REL_EN",
            setup: "Cinder cache failure",
            follow_up: "Therefore, Cinder service entered degraded mode",
            language: LanguageCodeIR::English,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "relation_setup_transfer",
        },
        Case {
            id: "R31_X_REL_KO",
            setup: "다온 캐시 장애",
            follow_up: "따라서, 다온 서비스 성능 저하",
            language: LanguageCodeIR::Korean,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "relation_setup_transfer",
        },
        Case {
            id: "R31_X_ABSENT_EN",
            setup: "repair the scheduler",
            follow_up: "What is its result?",
            language: LanguageCodeIR::English,
            kind: "EVIDENCE_ABSENCE",
            support: "EVIDENCE_ABSENT",
            epistemic: "UNKNOWN",
            category: "absence_transfer",
        },
        Case {
            id: "R31_X_ABSENT_KO",
            setup: "스케줄러를 수리해",
            follow_up: "그 결과는 어떻게 됐어?",
            language: LanguageCodeIR::Korean,
            kind: "EVIDENCE_ABSENCE",
            support: "EVIDENCE_ABSENT",
            epistemic: "UNKNOWN",
            category: "absence_transfer",
        },
        Case {
            id: "R31_X_SOCIAL_EN",
            setup: "",
            follow_up: "Got it, thanks",
            language: LanguageCodeIR::English,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "interaction_transfer",
        },
        Case {
            id: "R31_X_SOCIAL_KO",
            setup: "",
            follow_up: "알겠어 고마워",
            language: LanguageCodeIR::Korean,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "interaction_transfer",
        },
        Case {
            id: "R31_X_CLARIFY_EN",
            setup: "",
            follow_up: "fix it",
            language: LanguageCodeIR::English,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "clarification_transfer",
        },
        Case {
            id: "R31_X_CLARIFY_KO",
            setup: "",
            follow_up: "그거 고쳐",
            language: LanguageCodeIR::Korean,
            kind: "INTERACTION_STATE",
            support: "NON_FACTUAL",
            epistemic: "INTERACTION",
            category: "clarification_transfer",
        },
    ];
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R31-TRANSFER-0001",
            "frozen_before_first_suite_execution": true,
            "held_out_until_diagnostic_pass": true,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "recursive_source_mutations": 0,
            "total": total,
            "passed": passed,
            "failed": total - passed,
            "rows": rows
        }))
        .expect("suite json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
