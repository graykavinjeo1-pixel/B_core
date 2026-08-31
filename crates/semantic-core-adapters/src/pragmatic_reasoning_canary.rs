use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    PragmaticContextIR, PragmaticReasoner, SpeechActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct CanaryReceipt {
    case_id: &'static str,
    speech_act: SpeechActIR,
    inferred_task: Option<String>,
    required_benefit: Option<String>,
    inferred_goal: Option<String>,
    external_execution_authorized: Option<bool>,
    pass: bool,
}

fn main() {
    let mut api = CognitiveApi::new_embedded().expect("cognitive API");
    let request = ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: "PRAGMATIC-CANARY".to_string(),
        turn_index: 1,
        request_id: "PRAGMATIC-CANARY-1".to_string(),
        modality: ConversationInputModalityIR::Text,
        raw_text: "유일하게 고통을 참고 진행하려면 기존에는 점수만 높았지 실제 코딩능력은 한참 낮았다. 왜냐? capability와 routing이 결합되어 나온 거품점수라서 실제 커버리지는 낮았다. 그래서 통합을 하면 커버리지를 확장하는 효과가 있다. 이러면 할만하지".to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::Korean),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    };
    let response = api
        .process_conversation_turn(&request)
        .expect("indirect continuation request");
    let gate = response
        .pragmatic_interpretation
        .continuation_gate
        .as_ref()
        .expect("continuation gate");
    assert_eq!(gate.current_task, "통합");
    assert_eq!(gate.required_benefit, "커버리지를 확장하는 효과가 있다");
    assert!(response
        .grounded_response
        .as_ref()
        .is_some_and(|grounded| grounded.understanding.intent
            == dockable_semantic_core::PlanIntentIR::Investigate));

    let reasoner = PragmaticReasoner;
    let cases = [
        (
            "KOREAN_PARAPHRASE",
            "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어드는 효과가 있다. 그 정도 이득이면 계속 진행할 만하다.",
            SpeechActIR::ConditionalContinuation,
        ),
        (
            "ENGLISH_PARAPHRASE",
            "The migration is difficult. If migration reduces failures, it is worth continuing despite the cost.",
            SpeechActIR::ConditionalContinuation,
        ),
        (
            "IMPLICIT_REPAIR",
            "배포 후 오류가 늘었네. 이 상태로 둘 수는 없지.",
            SpeechActIR::RequestAction,
        ),
        (
            "KNOWLEDGE_GAP",
            "로그가 왜 비어 있는지 궁금하네.",
            SpeechActIR::Ask,
        ),
        (
            "FEATURE_SUGGESTION",
            "검색 기능이 있으면 반복 작업이 줄어서 편하겠다.",
            SpeechActIR::Suggest,
        ),
        (
            "NON_AUTHORIZING_FACT",
            "통합을 하면 커버리지가 확장된다.",
            SpeechActIR::Inform,
        ),
        (
            "NEGATIVE_OVERRIDE",
            "통합을 하면 커버리지가 늘 수 있다. 그렇다고 바로 계속하지 마.",
            SpeechActIR::Reject,
        ),
    ];
    let mut receipts = vec![CanaryReceipt {
        case_id: "USER_UTTERANCE",
        speech_act: response.pragmatic_interpretation.speech_act,
        inferred_task: Some(gate.current_task.clone()),
        required_benefit: Some(gate.required_benefit.clone()),
        inferred_goal: response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .map(|goal| format!("{:?}", goal.intent)),
        external_execution_authorized: response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .map(|goal| goal.external_execution_authorized),
        pass: true,
    }];
    for (case_id, text, expected) in cases {
        let interpretation = reasoner.interpret(text, &PragmaticContextIR::default());
        assert_eq!(interpretation.speech_act, expected, "{case_id}");
        receipts.push(CanaryReceipt {
            case_id,
            speech_act: interpretation.speech_act,
            inferred_task: interpretation.inferred_current_task.clone(),
            required_benefit: interpretation
                .continuation_gate
                .as_ref()
                .map(|gate| gate.required_benefit.clone()),
            inferred_goal: interpretation
                .inferred_goal
                .as_ref()
                .map(|goal| format!("{:?}", goal.intent)),
            external_execution_authorized: interpretation
                .inferred_goal
                .as_ref()
                .map(|goal| goal.external_execution_authorized),
            pass: true,
        });
    }
    println!(
        "{}",
        serde_json::to_string(&receipts).expect("canary receipts")
    );
}
