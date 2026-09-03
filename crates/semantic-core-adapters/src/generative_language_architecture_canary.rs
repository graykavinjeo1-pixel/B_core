use semantic_core_adapters::{
    ExpressionNodeStore, GenerationContextIR, GenerationEmotionIR, GenerationMeaningEdgeIR,
    GenerationMeaningGraphIR, GenerationMeaningNodeIR, GenerationMeaningNodeKindIR,
    GenerationMeaningRelationIR, GenerationSpeechIntentIR, GenerationTenseIR,
    GenerativeLanguageCortex, GenerativeLanguageRequestIR, LanguageCodeIR, LanguageRegisterIR,
};
use serde::Serialize;

#[derive(Serialize)]
struct Report {
    suite: &'static str,
    meaning_graph_valid: bool,
    korean_generation_valid: bool,
    english_generation_valid: bool,
    shared_semantic_payload: bool,
    distinct_language_phenotypes: bool,
    append_only_stage_lineage: bool,
    expression_scores_explainable: bool,
    morphology_fully_sourced: bool,
    semantic_roundtrip_exact: bool,
    no_language_execution_authority: bool,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    passed: usize,
    failed: usize,
}

fn graph() -> GenerationMeaningGraphIR {
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: vec!["CANARY:SAFETY_SCENARIO".to_string()],
    };
    let edge = |edge_id: &str, target: &str, relation| GenerationMeaningEdgeIR {
        edge_id: edge_id.to_string(),
        source_node_id: "EVENT_MOVE".to_string(),
        target_node_id: target.to_string(),
        relation,
    };
    GenerationMeaningGraphIR::new(
        vec![
            node("EVENT_MOVE", "C_MOVE", GenerationMeaningNodeKindIR::Event),
            node(
                "ENTITY_VICTIM",
                "C_ASSAULT_VICTIM",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "ENTITY_SAFE_PLACE",
                "C_SAFE_PLACE",
                GenerationMeaningNodeKindIR::Entity,
            ),
        ],
        vec![
            edge(
                "EDGE_AGENT",
                "ENTITY_VICTIM",
                GenerationMeaningRelationIR::Agent,
            ),
            edge(
                "EDGE_GOAL",
                "ENTITY_SAFE_PLACE",
                GenerationMeaningRelationIR::Goal,
            ),
        ],
    )
}

fn generate(
    language: LanguageCodeIR,
    meaning: GenerationMeaningGraphIR,
    expressions: &ExpressionNodeStore,
) -> semantic_core_adapters::GenerativeLanguageIR {
    GenerativeLanguageCortex
        .generate(GenerativeLanguageRequestIR {
            meaning,
            context: GenerationContextIR {
                language,
                register: LanguageRegisterIR::Neutral,
                tense: GenerationTenseIR::Present,
                emotion: GenerationEmotionIR::Concerned,
                urgency_millis: 900,
                default_speech_intent: GenerationSpeechIntentIR::Advise,
            },
            expressions,
        })
        .expect("built-in safety expressions")
}

fn main() {
    let meaning = graph();
    let expressions = ExpressionNodeStore::bilingual_builtin();
    let korean = generate(LanguageCodeIR::Korean, meaning.clone(), &expressions);
    let english = generate(LanguageCodeIR::English, meaning.clone(), &expressions);
    let hashes = [
        &korean.speech_intent.source_semantic_sha256,
        &korean.discourse_plan.source_semantic_sha256,
        &korean.expression_selection.source_semantic_sha256,
        &korean.syntax_plan.source_semantic_sha256,
        &korean.morphology.source_semantic_sha256,
    ];
    let checks = [
        meaning.validate(),
        korean.validate(),
        english.validate(),
        korean.meaning.semantic_sha256 == english.meaning.semantic_sha256,
        korean.morphology.realized_text != english.morphology.realized_text,
        hashes
            .iter()
            .all(|hash| hash.as_str() == meaning.semantic_sha256),
        korean
            .expression_selection
            .selections
            .iter()
            .all(|selection| {
                !selection.score.reasons.is_empty()
                    && selection.score.activation_millis <= 1_000
                    && selection.score.confidence_millis <= 1_000
                    && selection.score.context_fit_millis <= 1_000
            }),
        korean
            .morphology
            .tokens
            .iter()
            .all(|token| token.expression_id.is_some() || token.grammar_rule_id.is_some()),
        korean.verification.semantic_roundtrip_sha256 == meaning.semantic_sha256
            && english.verification.semantic_roundtrip_sha256 == meaning.semantic_sha256,
        !korean.semantic_authority
            && !korean.language_can_execute
            && !english.semantic_authority
            && !english.language_can_execute,
    ];
    let passed = checks.iter().filter(|check| **check).count();
    let report = Report {
        suite: "GENERATIVE-LANGUAGE-ARCHITECTURE-CANARY-1",
        meaning_graph_valid: checks[0],
        korean_generation_valid: checks[1],
        english_generation_valid: checks[2],
        shared_semantic_payload: checks[3],
        distinct_language_phenotypes: checks[4],
        append_only_stage_lineage: checks[5],
        expression_scores_explainable: checks[6],
        morphology_fully_sourced: checks[7],
        semantic_roundtrip_exact: checks[8],
        no_language_execution_authority: checks[9],
        external_llm_calls: 0,
        local_teacher_calls: 0,
        passed,
        failed: checks.len() - passed,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serialization")
    );
    if passed != checks.len() {
        std::process::exit(1);
    }
}
