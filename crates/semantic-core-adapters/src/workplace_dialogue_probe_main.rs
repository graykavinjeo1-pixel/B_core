use std::{env, fs, path::PathBuf};

use semantic_core_adapters::{
    compositional_response_sha256, realize_workplace_dialogue_response, ApprovedClauseUnitIR,
    ApprovedCompositionalClaimIR, ApprovedCompositionalResponseIR, ApprovedDiscourseRelationIR,
    ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR,
    ApprovedRelationTypeIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR, ApprovedSpeechActIR,
    ApprovedVerbosityIR, DiscourseFocusCandidateIR, DiscourseFocusStateIR, LanguageRegisterIR,
    WorkplaceAudienceIR, WorkplaceCommunicationContextIR, WorkplaceCommunicationFormatIR,
    WorkplaceDialogueContextIR, WorkplaceDialogueDispositionIR,
    COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: &'static str,
    expected_omission: bool,
    disposition: WorkplaceDialogueDispositionIR,
    full_form: String,
    rendered: String,
    recovered_subject_node_id: Option<String>,
    contextual_inverse_pass: bool,
    output_valid: bool,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    scenarios: usize,
    contextual_omission: usize,
    explicit_fallback: usize,
    contextual_inverse_pass: usize,
    expected_disposition_match: usize,
    rows: Vec<Row>,
}

fn response(
    subject_id: &str,
    subject_label: &str,
    relation: ApprovedRelationTypeIR,
    value: ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
) -> ApprovedCompositionalResponseIR {
    let claim = ApprovedCompositionalClaimIR {
        proposition_id: format!("P_{}", subject_id.to_uppercase()),
        subject: ApprovedLexicalNodeIR {
            node_id: subject_id.into(),
            semantic_type: ApprovedSemanticTypeIR::Event,
            canonical_lexical_label: subject_label.into(),
        },
        relation,
        value,
        polarity: true,
        modality: ApprovedModalityIR::Asserted,
    status_frame: None,
};
    let operation = match speech_act {
        ApprovedSpeechActIR::Acknowledge => ApprovedOperationIR::Confirm,
        _ => ApprovedOperationIR::Assert,
    };
    let mut response = ApprovedCompositionalResponseIR {
        schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
        speech_act,
        operation,
        claims: vec![claim],
        event_realizations: Vec::new(),
        discourse_relation,
        clause_plan: vec![ApprovedClauseUnitIR {
            unit_index: 0,
            role: "ASSERT".into(),
            proposition_ids: vec![format!("P_{}", subject_id.to_uppercase())],
            predecessor_indices: Vec::new(),
        }],
        style: ApprovedResponseStyleIR {
            register: LanguageRegisterIR::Neutral,
            verbosity: ApprovedVerbosityIR::Short,
        },
        source_world_state_sha256: "a".repeat(64),
        source_deliberation_sha256: "b".repeat(64),
        approval_replay_verified: true,
        unsupported_claims: 0,
        semantic_sha256: String::new(),
    };
    response.semantic_sha256 = compositional_response_sha256(&response);
    assert!(response.validate());
    response
}

fn lexical(id: &str, label: &str, semantic_type: ApprovedSemanticTypeIR) -> ApprovedOpenValueIR {
    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
        node_id: id.into(),
        semantic_type,
        canonical_lexical_label: label.into(),
    })
}

fn context(
    topic: &str,
    focus_surface: &str,
    focus_concept: &str,
    completed_turns: u64,
) -> WorkplaceDialogueContextIR {
    let mut discourse_focus = DiscourseFocusStateIR::default();
    discourse_focus.apply_turn(
        1,
        &[DiscourseFocusCandidateIR::explicit_topic(
            focus_surface,
            Some(focus_concept),
        )],
    );
    WorkplaceDialogueContextIR {
        workplace: WorkplaceCommunicationContextIR {
            format: WorkplaceCommunicationFormatIR::InstantMessage,
            audience: WorkplaceAudienceIR::Peer,
            topic_label: topic.into(),
        },
        discourse_focus,
        completed_turns,
    }
}

fn main() {
    let cases = vec![
        (
            "EXACT_STATUS",
            true,
            response(
                "release",
                "배포",
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("배포", "배포", "release", 2),
        ),
        (
            "EXACT_DATE_ACKNOWLEDGE",
            true,
            response(
                "remote_work",
                "재택근무",
                ApprovedRelationTypeIR::Date,
                ApprovedOpenValueIR::Date {
                    year: 2026,
                    month: 9,
                    day: 23,
                },
                ApprovedSpeechActIR::Acknowledge,
                ApprovedDiscourseRelationIR::Confirmation,
            ),
            context("재택근무", "재택근무", "remote_work", 2),
        ),
        (
            "EXACT_COUNT",
            false,
            response(
                "unassigned_ticket",
                "미배정 문의",
                ApprovedRelationTypeIR::Count,
                ApprovedOpenValueIR::Quantity {
                    amount: 23,
                    unit: ApprovedLexicalNodeIR {
                        node_id: "case_unit".into(),
                        semantic_type: ApprovedSemanticTypeIR::Unit,
                        canonical_lexical_label: "건".into(),
                    },
                },
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("미배정 문의", "미배정 문의", "unassigned_ticket", 2),
        ),
        (
            "EXACT_APPROVAL",
            true,
            response(
                "training_budget",
                "교육 예산",
                ApprovedRelationTypeIR::Approved,
                ApprovedOpenValueIR::Boolean(true),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("교육 예산", "교육 예산", "training_budget", 2),
        ),
        (
            "MISMATCHED_FOCUS",
            false,
            response(
                "release",
                "배포",
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("배포", "계약", "contract", 2),
        ),
        (
            "SURFACE_ONLY_COLLISION",
            false,
            response(
                "release",
                "배포",
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("배포", "배포", "different_release", 2),
        ),
        (
            "CONCEPT_ONLY_COLLISION",
            false,
            response(
                "release",
                "배포",
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("배포", "출시", "release", 2),
        ),
        (
            "STALE_FOCUS",
            false,
            response(
                "release",
                "배포",
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
            ),
            context("배포", "배포", "release", 18),
        ),
    ];

    let mut rows = Vec::new();
    for (id, expected_omission, response, context) in cases {
        let output = realize_workplace_dialogue_response(&response, context)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        rows.push(Row {
            id,
            expected_omission,
            disposition: output.disposition,
            full_form: output.full_form.rendered.clone(),
            rendered: output.rendered.clone(),
            recovered_subject_node_id: output.recovered_subject_node_id.clone(),
            contextual_inverse_pass: output.contextual_inverse_pass,
            output_valid: output.validate(&response),
        });
    }
    let report = Report {
        schema: "B_CORE_WORKPLACE_DIALOGUE_CONTEXT_PROBE_1",
        scenarios: rows.len(),
        contextual_omission: rows
            .iter()
            .filter(|row| {
                row.disposition == WorkplaceDialogueDispositionIR::ContextualSubjectOmission
            })
            .count(),
        explicit_fallback: rows
            .iter()
            .filter(|row| {
                row.disposition == WorkplaceDialogueDispositionIR::ExplicitSubjectFallback
            })
            .count(),
        contextual_inverse_pass: rows
            .iter()
            .filter(|row| row.contextual_inverse_pass && row.output_valid)
            .count(),
        expected_disposition_match: rows
            .iter()
            .filter(|row| {
                row.expected_omission
                    == (row.disposition
                        == WorkplaceDialogueDispositionIR::ContextualSubjectOmission)
            })
            .count(),
        rows,
    };
    let output_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("D:/B_Core_validation/workplace_dialogue_context_probe_v1.json")
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("output directory");
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report).unwrap()).expect("write report");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
