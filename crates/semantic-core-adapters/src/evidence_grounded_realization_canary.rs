//! Frozen R31 diagnostic suite.
//!
//! The suite observes only public API JSON and was frozen before the
//! claim-level evidence-grounded realization boundary existed.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ACTION_EVIDENCE_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";

#[derive(Debug, Clone, Copy)]
enum Scenario {
    Plan,
    Report,
    Verified,
    Attribution,
    Temporal,
    DialogueRelation,
    EvidenceAbsence,
    Interaction,
}

impl Scenario {
    const fn category(self) -> &'static str {
        match self {
            Self::Plan => "plan_claim",
            Self::Report => "language_report_claim",
            Self::Verified => "verified_execution_claim",
            Self::Attribution => "attributed_dialogue_claim",
            Self::Temporal => "temporal_relation_claim",
            Self::DialogueRelation => "dialogue_relation_claim",
            Self::EvidenceAbsence => "evidence_absence_claim",
            Self::Interaction => "nonfactual_interaction_claim",
        }
    }

    const fn expected(self) -> (&'static str, &'static str, &'static str, bool) {
        match self {
            Self::Plan => ("PLAN_STATUS", "STRUCTURALLY_GROUNDED", "PLANNED", false),
            Self::Report => ("LANGUAGE_REPORT", "REPORTED_ONLY", "REPORTED", false),
            Self::Verified => (
                "VERIFIED_EXECUTION",
                "VERIFIED_EVIDENCE",
                "VERIFIED_OBSERVED",
                true,
            ),
            Self::Attribution => (
                "ATTRIBUTED_DIALOGUE_RECORD",
                "DERIVED_FROM_DIALOGUE_RECORDS",
                "DERIVED",
                false,
            ),
            Self::Temporal => (
                "TEMPORAL_RELATION",
                "DERIVED_FROM_DIALOGUE_RECORDS",
                "DERIVED",
                false,
            ),
            Self::DialogueRelation => (
                "DIALOGUE_RELATION",
                "DERIVED_FROM_DIALOGUE_RECORDS",
                "DERIVED",
                false,
            ),
            Self::EvidenceAbsence => ("EVIDENCE_ABSENCE", "EVIDENCE_ABSENT", "UNKNOWN", false),
            Self::Interaction => ("INTERACTION_STATE", "NON_FACTUAL", "INTERACTION", false),
        }
    }
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

fn receipt_hash(
    receipt_id: &str,
    conversation_id: &str,
    action_id: &str,
    execution_id: &str,
    status: &str,
    evidence_digest: &str,
) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_EVIDENCE_SCHEMA,
        receipt_id,
        conversation_id,
        action_id,
        execution_id,
        status,
        evidence_digest,
    ))
    .expect("receipt hash payload");
    format!("{:x}", Sha256::digest(bytes))
}

fn submit(api: &mut CognitiveApi, id: &str, action_id: &str, suffix: &str, status: &str) -> bool {
    let receipt_id = format!("{id}-R31-{suffix}");
    let execution_id = format!("{id}-EXECUTION");
    let evidence_digest = format!("{:064x}", status.len() + suffix.len());
    let command = json!({
        "operation": "SUBMIT_ACTION_EVIDENCE",
        "request": {
            "schema": ACTION_EVIDENCE_SCHEMA,
            "receipt_id": receipt_id,
            "conversation_id": id,
            "action_id": action_id,
            "execution_id": execution_id,
            "status": status,
            "evidence_digest": evidence_digest,
            "verifier_receipt_sha256": receipt_hash(
                &receipt_id, id, action_id, &execution_id, status, &evidence_digest
            )
        }
    });
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn process(api: &mut CognitiveApi, id: &str, turn: u64, text: &str, lang: LanguageCodeIR) -> Value {
    let response = api
        .process_conversation_turn(&request(id, turn, text, lang))
        .expect("conversation turn");
    serde_json::to_value(response).expect("response json")
}

fn action_id(value: &Value) -> String {
    value
        .pointer("/conversation_state/action_state_ledger/records/0/action_id")
        .and_then(Value::as_str)
        .unwrap_or("MISSING-ACTION")
        .to_string()
}

fn run_scenario(id: &str, scenario: Scenario, variant: usize) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let language = if variant.is_multiple_of(2) {
        LanguageCodeIR::English
    } else {
        LanguageCodeIR::Korean
    };
    let value = match scenario {
        Scenario::Plan => process(
            &mut api,
            id,
            1,
            if language == LanguageCodeIR::English {
                "repair the cache"
            } else {
                "캐시를 수리해"
            },
            language,
        ),
        Scenario::Report => {
            process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "repair the parser"
                } else {
                    "파서를 수리해"
                },
                language,
            );
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "I completed it"
                } else {
                    "그 작업은 끝났어"
                },
                language,
            )
        }
        Scenario::Verified => {
            let first = process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "inspect the archive"
                } else {
                    "아카이브를 검사해"
                },
                language,
            );
            let action = action_id(&first);
            let _ = submit(&mut api, id, &action, "START", "EXECUTION_STARTED");
            let _ = submit(&mut api, id, &action, "END", "SUCCEEDED");
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "What is its execution result?"
                } else {
                    "그 실행 결과가 어떻게 됐어?"
                },
                language,
            )
        }
        Scenario::Attribution => {
            process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "Alice says that the server is down."
                } else {
                    "민수는 서버가 느리다고 말했다."
                },
                language,
            );
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "What did Alice say?"
                } else {
                    "민수는 뭐라고 말했어?"
                },
                language,
            )
        }
        Scenario::Temporal => {
            process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "The backup completed before the deploy started."
                } else {
                    "배포가 시작되기 전에 백업이 완료됐다."
                },
                language,
            );
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "What happened before the deploy started?"
                } else {
                    "배포가 시작되기 전에 무슨 일이 있었어?"
                },
                language,
            )
        }
        Scenario::DialogueRelation => {
            process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "Atlas cache integrity failed"
                } else {
                    "가온 캐시 무결성 실패"
                },
                language,
            );
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "Because of that, Atlas service latency is high"
                } else {
                    "그 때문에, 가온 서비스 지연 발생"
                },
                language,
            );
            process(
                &mut api,
                id,
                3,
                if language == LanguageCodeIR::English {
                    "Why is Atlas service latency high?"
                } else {
                    "왜 가온 서비스 지연 발생?"
                },
                language,
            )
        }
        Scenario::EvidenceAbsence => {
            process(
                &mut api,
                id,
                1,
                if language == LanguageCodeIR::English {
                    "repair the worker"
                } else {
                    "워커를 수리해"
                },
                language,
            );
            process(
                &mut api,
                id,
                2,
                if language == LanguageCodeIR::English {
                    "What is its result?"
                } else {
                    "그 결과는 어떻게 됐어?"
                },
                language,
            )
        }
        Scenario::Interaction => process(
            &mut api,
            id,
            1,
            if language == LanguageCodeIR::English {
                "Thanks"
            } else {
                "고마워"
            },
            language,
        ),
    };
    let (kind, support, epistemic, verified) = scenario.expected();
    let realization = value.pointer("/grounded_realization");
    let claims = realization
        .and_then(|item| item.get("claims"))
        .and_then(Value::as_array);
    let expected_claim = claims.is_some_and(|claims| {
        claims.iter().any(|claim| {
            claim["kind"] == kind
                && claim["support_status"] == support
                && claim["epistemic_status"] == epistemic
                && claim["verified"] == verified
                && claim["semantic_authority"] == false
                && claim["external_action_executed"] == false
                && (scenario.category() == "nonfactual_interaction_claim"
                    || claim["evidence_refs"]
                        .as_array()
                        .is_some_and(|refs| !refs.is_empty()))
        })
    });
    let pass = realization.is_some_and(|item| {
        item["schema"] == "B_CORE_EVIDENCE_GROUNDED_REALIZATION_IR_1"
            && item["realized_text"] == value["output"]["text"]
            && item["faithful"] == true
            && item["unsupported_claims"] == 0
            && item["semantic_authority"] == false
            && item["external_action_executed"] == false
            && item["realization_sha256"]
                .as_str()
                .is_some_and(|hash| hash.len() == 64)
    }) && expected_claim;
    Row {
        id: id.to_string(),
        category: scenario.category().to_string(),
        pass,
        trace: vec![value.to_string()],
    }
}

fn main() {
    let scenarios = [
        Scenario::Plan,
        Scenario::Report,
        Scenario::Verified,
        Scenario::Attribution,
        Scenario::Temporal,
        Scenario::DialogueRelation,
        Scenario::EvidenceAbsence,
        Scenario::Interaction,
    ];
    let mut rows = Vec::new();
    for scenario in scenarios {
        for variant in 0..4 {
            rows.push(run_scenario(
                &format!("R31_DIAG_{}_{variant}", scenario.category()),
                scenario,
                variant,
            ));
        }
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R31-RUN-0001",
            "frozen_before_first_suite_execution": true,
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
