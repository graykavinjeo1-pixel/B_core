use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    UtteranceAlternativeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};

fn request(turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: "CONVERSATION-CANARY".to_string(),
        turn_index,
        request_id: format!("CONVERSATION-CANARY-{turn_index}"),
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
    let mut api = CognitiveApi::new_embedded().expect("cognitive API");
    let mut turns = vec![
        request(1, "음... 파일을, 아니 폴더를 열어"),
        request(2, "어... 그걸 확인해"),
        request(3, "응"),
    ];
    let mut voice = request(4, "쿵 소리가 났어");
    voice.modality = ConversationInputModalityIR::VoiceTranscript;
    voice.input_confidence_millis = 810;
    voice.alternatives = vec![UtteranceAlternativeIR {
        text: "콩 소리가 났어".to_string(),
        confidence_millis: 790,
    }];
    turns.push(voice);

    for turn in turns {
        let response = api
            .process_conversation_turn(&turn)
            .expect("conversation turn");
        println!(
            "{}",
            serde_json::to_string(&response).expect("response JSON")
        );
    }
}
