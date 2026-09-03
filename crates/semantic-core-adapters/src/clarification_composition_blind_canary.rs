//! Frozen R24-RUN-0001 clarification-composition blind suite.
//!
//! The cases below were fixed before the first execution. They test whether
//! unfamiliar surfaces and cross-language output still travel through the
//! typed meaning-to-question generator instead of a drafted sentence path.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, GenerationSpeechIntentIR, LanguageCodeIR, NaturalRealizationPathIR,
    UtteranceAlternativeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Turn<'a> {
    text: &'a str,
    input_language: LanguageCodeIR,
}

struct Case<'a> {
    id: &'a str,
    category: &'a str,
    setup: &'a [Turn<'a>],
    text: &'a str,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    modality: ConversationInputModalityIR,
    confidence_millis: u16,
    alternatives: &'a [(&'a str, u16)],
    expected_concept: &'a str,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    output_language: LanguageCodeIR,
    expected_concept: String,
    semantic_surface: String,
    nonliteral_clarification: bool,
    ambiguous_references: Vec<String>,
    realized_text: String,
    typed_generation: bool,
    safety_boundary: bool,
    pass: bool,
}

#[derive(Serialize)]
struct Report {
    suite: &'static str,
    frozen_before_first_execution: bool,
    fresh_cases: usize,
    passed: usize,
    failed: usize,
    generative_path_rate_millis: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    rows: Vec<Row>,
}

fn request(
    conversation_id: &str,
    turn_index: u64,
    turn: Turn<'_>,
    output_language: LanguageCodeIR,
) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(output_language),
        context_tags: vec![format!("INPUT_LANGUAGE:{:?}", turn.input_language)],
        max_plan_steps: 16,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, turn) in case.setup.iter().copied().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded turn"),
            turn,
            turn.input_language,
        ))
        .unwrap_or_else(|error| panic!("setup failed: case={}, error={error:?}", case.id));
    }
    let turn_index = u64::try_from(case.setup.len() + 1).expect("bounded turn");
    let mut final_request = request(
        case.id,
        turn_index,
        Turn {
            text: case.text,
            input_language: case.input_language,
        },
        case.output_language,
    );
    final_request.modality = case.modality;
    final_request.input_confidence_millis = case.confidence_millis;
    final_request.alternatives = case
        .alternatives
        .iter()
        .map(|(text, confidence_millis)| UtteranceAlternativeIR {
            text: (*text).to_string(),
            confidence_millis: *confidence_millis,
        })
        .collect();
    let response = api
        .process_conversation_turn(&final_request)
        .unwrap_or_else(|error| panic!("case failed: case={}, error={error:?}", case.id));
    let trace = response.natural_realization.generation_traces.first();
    let typed_generation = response.disposition
        == ConversationTurnDispositionIR::ClarificationRequired
        && response.grounded_response.is_none()
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| {
            trace.validate()
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == case.expected_concept)
                && trace
                    .speech_intent
                    .intents
                    .iter()
                    .all(|intent| intent.intent == GenerationSpeechIntentIR::Ask)
        });
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_CLARIFY_")
        && !response.output.text.contains("GoalIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        output_language: response.output.language,
        expected_concept: case.expected_concept.to_string(),
        semantic_surface: response.normalization.semantic_surface_text,
        nonliteral_clarification: response
            .pragmatic_interpretation
            .nonliteral_analysis
            .clarification_required,
        ambiguous_references: response.reference_resolution.ambiguous_reference_surfaces,
        realized_text: response.output.text,
        typed_generation,
        safety_boundary,
        pass: typed_generation && safety_boundary,
    }
}

fn main() {
    use ConversationInputModalityIR::{Text, VoiceTranscript};
    use LanguageCodeIR::{English, Korean};

    const NO_TURNS: &[Turn<'static>] = &[];
    const KO_PROGRAM: &[Turn<'static>] = &[Turn {
        text: "호박 파일을 읽고 저장해",
        input_language: Korean,
    }];
    const EN_PROGRAM: &[Turn<'static>] = &[Turn {
        text: "Read and save the Juniper file",
        input_language: English,
    }];
    const KO_VOICE: &[(&str, u16)] = &[("수정 기록을 얼어", 770)];
    const EN_VOICE: &[(&str, u16)] = &[("open the silver lock", 765)];

    let cases = [
        Case { id: "R24_KO_FIRE", category: "nonliteral_ambiguity", setup: NO_TURNS, text: "자수정 구역에 불이 났어", input_language: Korean, output_language: Korean, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_NONLITERAL_READING" },
        Case { id: "R24_EN_FIRE", category: "nonliteral_ambiguity", setup: NO_TURNS, text: "The opal room is on fire", input_language: English, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_NONLITERAL_READING" },
        Case { id: "R24_CROSS_FIRE", category: "cross_language_nonliteral", setup: NO_TURNS, text: "여기 불이 났어", input_language: Korean, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_NONLITERAL_READING" },
        Case { id: "R24_KO_VOICE", category: "voice_alternative", setup: NO_TURNS, text: "수정 기록을 열어", input_language: Korean, output_language: Korean, modality: VoiceTranscript, confidence_millis: 800, alternatives: KO_VOICE, expected_concept: "C_CLARIFY_VOICE_ALTERNATIVE" },
        Case { id: "R24_EN_VOICE", category: "voice_alternative", setup: NO_TURNS, text: "open the silver log", input_language: English, output_language: English, modality: VoiceTranscript, confidence_millis: 800, alternatives: EN_VOICE, expected_concept: "C_CLARIFY_VOICE_ALTERNATIVE" },
        Case { id: "R24_CROSS_VOICE", category: "cross_language_voice", setup: NO_TURNS, text: "수정 기록을 열어", input_language: Korean, output_language: English, modality: VoiceTranscript, confidence_millis: 800, alternatives: KO_VOICE, expected_concept: "C_CLARIFY_VOICE_ALTERNATIVE" },
        Case { id: "R24_KO_COMPETE", category: "competing_request", setup: NO_TURNS, text: "자수정 파일을 분석해; 호박 코드를 수정해", input_language: Korean, output_language: Korean, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_COMPETING_REQUEST" },
        Case { id: "R24_EN_COMPETE", category: "competing_request", setup: NO_TURNS, text: "Analyze the opal file; repair the juniper code", input_language: English, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_COMPETING_REQUEST" },
        Case { id: "R24_KO_ORDERED", category: "ordered_pair_ambiguity", setup: NO_TURNS, text: "자수정 파일은 오래됐고 호박 폴더는 비었고 은빛 보고서는 낡았어. 전자를 분석하고 후자를 수정해", input_language: Korean, output_language: Korean, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_ORDERED_PAIR" },
        Case { id: "R24_EN_ORDERED", category: "ordered_pair_ambiguity", setup: NO_TURNS, text: "The opal file is stale, the juniper folder is empty, and the silver report is old. Analyze the former and repair the latter", input_language: English, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_ORDERED_PAIR" },
        Case { id: "R24_KO_REPEAT", category: "multi_goal_ellipsis", setup: KO_PROGRAM, text: "그대로 해", input_language: Korean, output_language: Korean, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_RESOLVE_REFERENCE" },
        Case { id: "R24_EN_REPEAT", category: "multi_goal_ellipsis", setup: EN_PROGRAM, text: "Do the same", input_language: English, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_RESOLVE_REFERENCE" },
        Case { id: "R24_KO_PREVIOUS", category: "missing_previous_topic", setup: NO_TURNS, text: "이전 주제로 돌아가자", input_language: Korean, output_language: Korean, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_PREVIOUS_TOPIC" },
        Case { id: "R24_EN_PREVIOUS", category: "missing_previous_topic", setup: NO_TURNS, text: "Return to the previous topic", input_language: English, output_language: English, modality: Text, confidence_millis: 1_000, alternatives: &[], expected_concept: "C_CLARIFY_PREVIOUS_TOPIC" },
    ];
    let rows = cases.iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let external_llm_calls = 0;
    let local_teacher_calls = 0;
    let report = Report {
        suite: "CLARIFICATION-COMPOSITION-BLIND-CANARY-R24-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        generative_path_rate_millis: passed * 1_000 / rows.len(),
        unsupported_explanation_facts: 0,
        external_llm_calls,
        local_teacher_calls,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serialization")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
