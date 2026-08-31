use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, PendingGateStatusIR, ReadingSelectionIR,
    SpeechActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct R2Receipt {
    case_id: &'static str,
    observed: String,
    pass: bool,
}

fn request(conversation_id: &str, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::Korean),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn main() {
    let mut receipts = Vec::new();
    let mut api = CognitiveApi::new_embedded().expect("cognitive API");
    let first = api
        .process_conversation_turn(&request(
            "R2-ELLIPSIS",
            1,
            "마이그레이션은 힘들다. 마이그레이션을 하면 장애 빈도가 감소한다. 그 정도 이득이면 계속 진행할 만하다.",
        ))
        .expect("first gate");
    assert!(first.pragmatic_state.pending_continuation_gate.is_some());
    let second = api
        .process_conversation_turn(&request("R2-ELLIPSIS", 2, "그 정도면 계속할 만하지"))
        .expect("ellipsis");
    let restored = second
        .pragmatic_interpretation
        .continuation_gate
        .as_ref()
        .expect("restored gate");
    assert_eq!(restored.current_task, "마이그레이션");
    assert!(restored.required_benefit.contains("장애 빈도"));
    receipts.push(R2Receipt {
        case_id: "MULTI_TURN_ELLIPSIS",
        observed: format!("{} -> {}", restored.current_task, restored.required_benefit),
        pass: true,
    });

    let mut suspend_api = CognitiveApi::new_embedded().expect("cognitive API");
    suspend_api
        .process_conversation_turn(&request(
            "R2-SUSPEND",
            1,
            "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어든다. 이러면 계속할 만하다.",
        ))
        .expect("gate");
    let suspended = suspend_api
        .process_conversation_turn(&request("R2-SUSPEND", 2, "그래도 계속하지 마"))
        .expect("suspend");
    assert_eq!(
        suspended
            .pragmatic_state
            .pending_continuation_gate
            .as_ref()
            .expect("pending gate")
            .status,
        PendingGateStatusIR::SuspendedByUser
    );
    let after = suspend_api
        .process_conversation_turn(&request("R2-SUSPEND", 3, "그 정도면 계속할 만하지"))
        .expect("post-suspension ellipsis");
    assert!(after.pragmatic_interpretation.continuation_gate.is_none());
    receipts.push(R2Receipt {
        case_id: "NEGATIVE_STANCE_PERSISTS",
        observed: "SUSPENDED_BY_USER".to_string(),
        pass: true,
    });

    let mut sarcasm_api = CognitiveApi::new_embedded().expect("cognitive API");
    let sarcasm = sarcasm_api
        .process_conversation_turn(&request(
            "R2-SARCASM",
            1,
            "테스트가 전부 깨졌네. 아주 잘했어.",
        ))
        .expect("sarcasm");
    assert_eq!(
        sarcasm.pragmatic_interpretation.speech_act,
        SpeechActIR::NegativeEvaluation
    );
    receipts.push(R2Receipt {
        case_id: "SARCASM_INCONGRUITY",
        observed: "NEGATIVE_EVALUATION_NOT_APPROVAL".to_string(),
        pass: true,
    });

    let mut ambiguous_api = CognitiveApi::new_embedded().expect("cognitive API");
    let ambiguous = ambiguous_api
        .process_conversation_turn(&request("R2-AMBIGUOUS-FIRE", 1, "여기 불이 났어"))
        .expect("ambiguous metaphor");
    assert_eq!(
        ambiguous.disposition,
        ConversationTurnDispositionIR::ClarificationRequired
    );
    assert!(ambiguous.grounded_response.is_none());
    receipts.push(R2Receipt {
        case_id: "LITERAL_FIGURATIVE_AMBIGUITY",
        observed: "CLARIFICATION_REQUIRED".to_string(),
        pass: true,
    });

    let mut metaphor_api = CognitiveApi::new_embedded().expect("cognitive API");
    let metaphor = metaphor_api
        .process_conversation_turn(&request(
            "R2-SOFTWARE-FIRE",
            1,
            "배포 뒤 프로젝트에 불이 났어",
        ))
        .expect("software metaphor");
    assert_eq!(
        metaphor
            .pragmatic_interpretation
            .nonliteral_analysis
            .expressions[0]
            .selected_reading,
        ReadingSelectionIR::Figurative
    );
    receipts.push(R2Receipt {
        case_id: "CONTEXTUAL_METAPHOR",
        observed: "C_CRITICAL_INCIDENT".to_string(),
        pass: true,
    });

    println!("{}", serde_json::to_string(&receipts).expect("R2 receipts"));
}
