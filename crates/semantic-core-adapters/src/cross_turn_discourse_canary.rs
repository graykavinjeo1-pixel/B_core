use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseBindingKindIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Expected {
    disposition: ConversationTurnDispositionIR,
    resolved: &'static str,
    intent: Option<PlanIntentIR>,
    subject: Option<&'static str>,
    binding: Option<DiscourseBindingKindIR>,
}

struct Case {
    case_id: &'static str,
    turns: &'static [&'static str],
    expected: Expected,
}

#[derive(Serialize)]
struct Row {
    case_id: String,
    turn_count: usize,
    disposition: ConversationTurnDispositionIR,
    resolved_semantic_text: String,
    selected_intent: Option<PlanIntentIR>,
    selected_subject: Option<String>,
    binding: Option<DiscourseBindingKindIR>,
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
        output_language: Some(if text.is_ascii() {
            LanguageCodeIR::English
        } else {
            LanguageCodeIR::Korean
        }),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn main() {
    let grounded = ConversationTurnDispositionIR::Grounded;
    let clarify = ConversationTurnDispositionIR::ClarificationRequired;
    let cases = [
        Case {
            case_id: "KO_SINGULAR_PRONOUN",
            turns: &["파일을 열어", "그걸 수정해"],
            expected: Expected {
                disposition: grounded,
                resolved: "파일을 수정해",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("파일"),
                binding: Some(DiscourseBindingKindIR::PronominalReference),
            },
        },
        Case {
            case_id: "KO_PLURAL_PRONOUN",
            turns: &["파일과 폴더를 확인해", "그것들을 저장해"],
            expected: Expected {
                disposition: grounded,
                resolved: "파일과 폴더를 저장해",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("폴더"),
                binding: Some(DiscourseBindingKindIR::PluralReference),
            },
        },
        Case {
            case_id: "KO_FORMER_REFERENCE",
            turns: &["파일과 폴더를 확인해", "전자를 수정해"],
            expected: Expected {
                disposition: grounded,
                resolved: "파일을 수정해",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("파일"),
                binding: Some(DiscourseBindingKindIR::OrderedReference),
            },
        },
        Case {
            case_id: "KO_LATTER_REFERENCE",
            turns: &["파일과 폴더를 확인해", "후자를 저장해"],
            expected: Expected {
                disposition: grounded,
                resolved: "폴더를 저장해",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("폴더"),
                binding: Some(DiscourseBindingKindIR::OrderedReference),
            },
        },
        Case {
            case_id: "KO_PARALLEL_ELLIPSIS",
            turns: &["파일을 확인해", "문서도"],
            expected: Expected {
                disposition: grounded,
                resolved: "문서를 확인해",
                intent: Some(PlanIntentIR::Investigate),
                subject: Some("문서"),
                binding: Some(DiscourseBindingKindIR::EllipticalAction),
            },
        },
        Case {
            case_id: "KO_REPEAT_GOAL",
            turns: &["보고서를 저장해", "그대로 해"],
            expected: Expected {
                disposition: grounded,
                resolved: "보고서를 저장해",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("보고서"),
                binding: Some(DiscourseBindingKindIR::RepeatedGoal),
            },
        },
        Case {
            case_id: "KO_CORRECT_ARGUMENT",
            turns: &["파일을 열어", "그거 말고 폴더로"],
            expected: Expected {
                disposition: grounded,
                resolved: "폴더를 열어",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("폴더"),
                binding: Some(DiscourseBindingKindIR::CorrectedArgument),
            },
        },
        Case {
            case_id: "KO_AMBIGUOUS_PROGRAM_REPEAT",
            turns: &["파일을 읽고 저장해", "그대로 해"],
            expected: Expected {
                disposition: clarify,
                resolved: "그대로 해",
                intent: None,
                subject: None,
                binding: None,
            },
        },
        Case {
            case_id: "KO_NOISY_PARALLEL_ELLIPSIS",
            turns: &["음... 파일을 고처줘", "문서도"],
            expected: Expected {
                disposition: grounded,
                resolved: "문서를 고쳐",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("문서"),
                binding: Some(DiscourseBindingKindIR::EllipticalAction),
            },
        },
        Case {
            case_id: "KO_TO_EN_REFERENCE",
            turns: &["파일을 열어", "fix it"],
            expected: Expected {
                disposition: grounded,
                resolved: "fix file",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("file"),
                binding: Some(DiscourseBindingKindIR::PronominalReference),
            },
        },
        Case {
            case_id: "EN_SINGULAR_PRONOUN",
            turns: &["open file", "fix it"],
            expected: Expected {
                disposition: grounded,
                resolved: "fix file",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("file"),
                binding: Some(DiscourseBindingKindIR::PronominalReference),
            },
        },
        Case {
            case_id: "EN_PLURAL_PRONOUN",
            turns: &["check file and folder", "save them"],
            expected: Expected {
                disposition: grounded,
                resolved: "save file and folder",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("file and folder"),
                binding: Some(DiscourseBindingKindIR::PluralReference),
            },
        },
        Case {
            case_id: "EN_FORMER_REFERENCE",
            turns: &["check file and folder", "fix former"],
            expected: Expected {
                disposition: grounded,
                resolved: "fix file",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("file"),
                binding: Some(DiscourseBindingKindIR::OrderedReference),
            },
        },
        Case {
            case_id: "EN_LATTER_REFERENCE",
            turns: &["check file and folder", "save latter"],
            expected: Expected {
                disposition: grounded,
                resolved: "save folder",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("folder"),
                binding: Some(DiscourseBindingKindIR::OrderedReference),
            },
        },
        Case {
            case_id: "EN_PARALLEL_ELLIPSIS",
            turns: &["inspect file", "same for report"],
            expected: Expected {
                disposition: grounded,
                resolved: "inspect report",
                intent: Some(PlanIntentIR::Investigate),
                subject: Some("report"),
                binding: Some(DiscourseBindingKindIR::EllipticalAction),
            },
        },
        Case {
            case_id: "EN_REPEAT_GOAL",
            turns: &["save report", "do the same"],
            expected: Expected {
                disposition: grounded,
                resolved: "save report",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("report"),
                binding: Some(DiscourseBindingKindIR::RepeatedGoal),
            },
        },
        Case {
            case_id: "EN_CORRECT_ARGUMENT",
            turns: &["open file", "not that, folder instead"],
            expected: Expected {
                disposition: grounded,
                resolved: "open folder",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("folder"),
                binding: Some(DiscourseBindingKindIR::CorrectedArgument),
            },
        },
        Case {
            case_id: "EN_AMBIGUOUS_PROGRAM_REPEAT",
            turns: &["read file and then save it", "do the same"],
            expected: Expected {
                disposition: clarify,
                resolved: "do the same",
                intent: None,
                subject: None,
                binding: None,
            },
        },
        Case {
            case_id: "EN_TO_KO_REFERENCE",
            turns: &["create report", "그걸 저장해"],
            expected: Expected {
                disposition: grounded,
                resolved: "보고서를 저장해",
                intent: Some(PlanIntentIR::Execute),
                subject: Some("보고서"),
                binding: Some(DiscourseBindingKindIR::PronominalReference),
            },
        },
        Case {
            case_id: "EN_PUNCTUATED_REFERENCE",
            turns: &["open file", "fix it, then save it"],
            expected: Expected {
                disposition: grounded,
                resolved: "fix file, then save file",
                intent: Some(PlanIntentIR::Repair),
                subject: Some("file"),
                binding: Some(DiscourseBindingKindIR::PronominalReference),
            },
        },
    ];

    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut final_response = None;
        for (index, text) in case.turns.iter().enumerate() {
            final_response = Some(
                api.process_conversation_turn(&request(
                    case.case_id,
                    u64::try_from(index + 1).expect("turn index"),
                    text,
                ))
                .expect("conversation turn"),
            );
        }
        let response = final_response.expect("at least one turn");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate();
        let selected_intent = selected.map(|candidate| candidate.intent);
        let selected_subject = selected.map(|candidate| candidate.subject.clone());
        let binding = response
            .reference_resolution
            .discourse_bindings
            .last()
            .map(|binding| binding.kind);
        let pass = response.disposition == case.expected.disposition
            && response.reference_resolution.resolved_semantic_text == case.expected.resolved
            && selected_intent == case.expected.intent
            && selected_subject.as_deref() == case.expected.subject
            && binding == case.expected.binding;
        rows.push(Row {
            case_id: case.case_id.to_string(),
            turn_count: case.turns.len(),
            disposition: response.disposition,
            resolved_semantic_text: response.reference_resolution.resolved_semantic_text,
            selected_intent,
            selected_subject,
            binding,
            pass,
        });
    }
    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
