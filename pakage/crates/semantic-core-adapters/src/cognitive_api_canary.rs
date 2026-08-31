use dockable_semantic_core::{ExperienceIR, ExperienceOutcomeIR, EXPERIENCE_SCHEMA};
use semantic_core_adapters::{
    CognitiveApi, LanguageCodeIR, NaturalLanguageRequestIR, NATURAL_LANGUAGE_REQUEST_SCHEMA,
};

fn main() {
    let mut api = CognitiveApi::new_embedded().expect("cognitive API");
    api.inject_experience(ExperienceIR {
        schema: EXPERIENCE_SCHEMA.to_string(),
        experience_id: "CANARY-SUCCESS-1".to_string(),
        situation: "경로 처리 실패".to_string(),
        action: "정확한 literal path 사용".to_string(),
        outcome: ExperienceOutcomeIR::Successful,
        outcome_description: "경로 검증 통과".to_string(),
        semantic_tags: vec!["path".to_string(), "repair".to_string()],
        evidence: vec!["canary=pass".to_string()],
        confidence_millis: 900,
        source_language: Some("ko".to_string()),
    })
    .expect("experience injection");
    let response = api
        .process(&NaturalLanguageRequestIR {
            schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
            request_id: "COGNITIVE-CANARY".to_string(),
            text: "경로 결함을 점검하고 수리 계획 세워줘. ㄱㄱ".to_string(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: vec!["path".to_string()],
            max_plan_steps: 12,
        })
        .expect("natural-language plan");
    println!(
        "{}",
        serde_json::to_string(&response).expect("response JSON")
    );
}
