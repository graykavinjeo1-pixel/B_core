use std::{env, fs, path::PathBuf};

use semantic_core_adapters::{
    compositional_response_sha256, opaque_action_claim_count, realize_workplace_response, ApprovedClauseUnitIR,
    ApprovedActionEventArgumentIR, ApprovedActionEventFrameIR, ApprovedActionProcedureDependencyIR,
    ApprovedActionProcedureIR,
    ApprovedCompositionalClaimIR, ApprovedCompositionalResponseIR, ApprovedDiscourseRelationIR,
    ApprovedEventFocusIR, ApprovedEventPerspectiveIR, ApprovedEventPhaseIR,
    ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedEventRealizationIR,
    ApprovedEventVoiceIR, ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR,
    ApprovedOperationIR, ApprovedRelationTypeIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR,
    ApprovedSpeechActIR, ApprovedStatusFrameIR, ApprovedConfigurationStateIR,
    ApprovedVerbosityIR, LanguageRegisterIR, WorkplaceAudienceIR,
    WorkplaceCommunicationContextIR, WorkplaceCommunicationFormatIR,
    COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
};
use serde::Serialize;

#[derive(Debug)]
struct Scenario {
    id: &'static str,
    family: &'static str,
    context: WorkplaceCommunicationContextIR,
    response: ApprovedCompositionalResponseIR,
}

#[derive(Debug, Serialize)]
struct ProbeRow {
    id: String,
    family: String,
    format: WorkplaceCommunicationFormatIR,
    audience: WorkplaceAudienceIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
    claim_count: usize,
    opaque_action_claims: usize,
    opaque_actions: Vec<OpaqueActionProbe>,
    typed_event_realizations: usize,
    planned_event_predicates: usize,
    planned_event_subject_ids: Vec<String>,
    baseline_body: String,
    rendered: String,
    semantic_inverse_pass: bool,
    unsupported_claims: usize,
    baseline_format_fit: bool,
    formatted_fit: bool,
    diagnostics: Vec<String>,
}

/// Diagnostic provenance for an Action that is deliberately still opaque.
/// This is not used by realization; it makes the missing source-level event
/// roles visible so repairs can target recurring grounding gaps instead of
/// reverse-engineering Korean text at runtime.
#[derive(Debug, Serialize)]
struct OpaqueActionProbe {
    proposition_id: String,
    subject_node_id: String,
    subject_label: String,
    action_value: String,
    modality: ApprovedModalityIR,
}

#[derive(Debug, Serialize)]
struct ProbeReport {
    schema: &'static str,
    scenarios: usize,
    formats: usize,
    semantic_inverse_pass: usize,
    unsupported_claims: usize,
    opaque_action_claims: usize,
    typed_event_realizations: usize,
    baseline_format_fit: usize,
    formatted_fit: usize,
    format_counts: std::collections::BTreeMap<String, usize>,
    speech_act_counts: std::collections::BTreeMap<String, usize>,
    diagnostic_counts: std::collections::BTreeMap<String, usize>,
    rows: Vec<ProbeRow>,
}

fn node(id: &str, label: &str, semantic_type: ApprovedSemanticTypeIR) -> ApprovedLexicalNodeIR {
    ApprovedLexicalNodeIR {
        node_id: id.into(),
        semantic_type,
        canonical_lexical_label: label.into(),
    }
}

fn claim(
    proposition_id: &str,
    subject_id: &str,
    subject_label: &str,
    relation: ApprovedRelationTypeIR,
    value: ApprovedOpenValueIR,
) -> ApprovedCompositionalClaimIR {
    claim_for_subject(
        proposition_id,
        subject_id,
        subject_label,
        ApprovedSemanticTypeIR::Event,
        relation,
        value,
    )
}

/// Probe fixtures keep the source subject type explicit.  Most scenario
/// subjects are events, but care/handoff records may describe a concrete
/// entity while their associated procedure is a separate Action event.
fn claim_for_subject(
    proposition_id: &str,
    subject_id: &str,
    subject_label: &str,
    subject_type: ApprovedSemanticTypeIR,
    relation: ApprovedRelationTypeIR,
    value: ApprovedOpenValueIR,
) -> ApprovedCompositionalClaimIR {
    ApprovedCompositionalClaimIR {
        proposition_id: proposition_id.into(),
        subject: node(subject_id, subject_label, subject_type),
        relation,
        value,
        polarity: true,
        modality: ApprovedModalityIR::Asserted,
        status_frame: None,
}
}

fn lexical(id: &str, label: &str, semantic_type: ApprovedSemanticTypeIR) -> ApprovedOpenValueIR {
    ApprovedOpenValueIR::Lexical(node(id, label, semantic_type))
}

fn response(
    claims: Vec<ApprovedCompositionalClaimIR>,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
    register: LanguageRegisterIR,
    verbosity: ApprovedVerbosityIR,
    event_realizations: Vec<ApprovedEventRealizationIR>,
) -> ApprovedCompositionalResponseIR {
    let operation = match discourse_relation {
        ApprovedDiscourseRelationIR::Correction => ApprovedOperationIR::Revise,
        ApprovedDiscourseRelationIR::Comparison => ApprovedOperationIR::Compare,
        ApprovedDiscourseRelationIR::Cause | ApprovedDiscourseRelationIR::Explanation => {
            ApprovedOperationIR::Explain
        }
        ApprovedDiscourseRelationIR::Condition => ApprovedOperationIR::Condition,
        ApprovedDiscourseRelationIR::Recall => ApprovedOperationIR::Recall,
        ApprovedDiscourseRelationIR::Confirmation => ApprovedOperationIR::Confirm,
        ApprovedDiscourseRelationIR::Inquiry => ApprovedOperationIR::Query,
        ApprovedDiscourseRelationIR::Directive => ApprovedOperationIR::Request,
        ApprovedDiscourseRelationIR::Commitment => ApprovedOperationIR::Promise,
        ApprovedDiscourseRelationIR::Reassurance => ApprovedOperationIR::Reassure,
        ApprovedDiscourseRelationIR::Negation => ApprovedOperationIR::Negate,
        _ => ApprovedOperationIR::Assert,
    };
    let clause_plan = claims
        .iter()
        .enumerate()
        .map(|(index, claim)| ApprovedClauseUnitIR {
            unit_index: index,
            role: "ASSERT".into(),
            proposition_ids: vec![claim.proposition_id.clone()],
            predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
        })
        .collect();
    let mut output = ApprovedCompositionalResponseIR {
        schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
        speech_act,
        operation,
        claims,
        event_realizations,
        discourse_relation,
        clause_plan,
        style: ApprovedResponseStyleIR {
            register,
            verbosity,
        },
        source_world_state_sha256: "a".repeat(64),
        source_deliberation_sha256: "b".repeat(64),
        approval_replay_verified: true,
        unsupported_claims: 0,
        semantic_sha256: String::new(),
    };
    output.semantic_sha256 = compositional_response_sha256(&output);
    assert!(output.validate());
    output
}

fn status_response(
    key: &str,
    label: &str,
    value_id: &str,
    value_label: &str,
    speech_act: ApprovedSpeechActIR,
    discourse: ApprovedDiscourseRelationIR,
    register: LanguageRegisterIR,
    verbosity: ApprovedVerbosityIR,
) -> ApprovedCompositionalResponseIR {
    response(
        vec![claim(
            &format!("P_{key}_STATUS"),
            key,
            label,
            ApprovedRelationTypeIR::Status,
            lexical(value_id, value_label, ApprovedSemanticTypeIR::State),
        )],
        speech_act,
        discourse,
        register,
        verbosity,
        Vec::new(),
    )
}

fn event_response(
    key: &str,
    event_label: &str,
    class: ApprovedEventRealizationClassIR,
    sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    roles: Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
    register: LanguageRegisterIR,
) -> ApprovedCompositionalResponseIR {
    let claims = roles
        .into_iter()
        .enumerate()
        .map(|(index, (relation, value))| {
            claim(
                &format!("P_{key}_{index}"),
                key,
                event_label,
                relation,
                value,
            )
        })
        .collect();
    response(
        claims,
        ApprovedSpeechActIR::Inform,
        ApprovedDiscourseRelationIR::Statement,
        register,
        ApprovedVerbosityIR::Short,
        vec![ApprovedEventRealizationIR {
            subject_node_id: key.into(),
            class,
            predicate_sense: Some(sense),
            phase: Some(phase),
            perspective: Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
            voice: None,
            information_structure: None,
        }],
    )
}

fn multi_event_document_response(
    events: Vec<(
        &'static str,
        &'static str,
        ApprovedEventRealizationClassIR,
        ApprovedEventPredicateSenseIR,
        ApprovedEventPhaseIR,
        Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
    )>,
    trailing_claims: Vec<ApprovedCompositionalClaimIR>,
) -> ApprovedCompositionalResponseIR {
    phased_multi_event_document_response(Vec::new(), events, trailing_claims)
}

/// Builds a long document from source-authoritative Action→Event frames.  This
/// deliberately has no lexical fallback: each event's class, predicate sense,
/// phase and roles must already be present in the source frame.  Reusing a
/// lexical node here is an explicit source identity assertion, never a label
/// based co-reference guess by the renderer.
fn action_event_document_response(
    claims: Vec<ApprovedCompositionalClaimIR>,
    action_frames: Vec<ApprovedActionEventFrameIR>,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
    register: LanguageRegisterIR,
    verbosity: ApprovedVerbosityIR,
) -> ApprovedCompositionalResponseIR {
    action_event_and_procedure_document_response(
        claims,
        Vec::new(),
        action_frames,
        speech_act,
        discourse_relation,
        register,
        verbosity,
    )
}

fn action_event_and_procedure_document_response(
    mut claims: Vec<ApprovedCompositionalClaimIR>,
    procedures: Vec<ApprovedActionProcedureIR>,
    action_frames: Vec<ApprovedActionEventFrameIR>,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
    register: LanguageRegisterIR,
    verbosity: ApprovedVerbosityIR,
) -> ApprovedCompositionalResponseIR {
    // A request or promise scopes the action frames it contains, while the
    // surrounding location/owner/deadline claims remain asserted context. The
    // Action→Event frame must retain this distinction so the renderer routes
    // the event through its predicate rather than treating its Action claim as
    // an ordinary relation.
    let scoped_action_modality = match speech_act {
        ApprovedSpeechActIR::Request => Some(ApprovedModalityIR::Directive),
        ApprovedSpeechActIR::Promise => Some(ApprovedModalityIR::Commissive),
        _ => None,
    };
    let procedure_step_count = procedures.iter().map(|procedure| procedure.steps.len()).sum::<usize>();
    let mut event_realizations = Vec::with_capacity(procedure_step_count + action_frames.len());
    for procedure in procedures {
        let procedure_id = procedure.action_subject.node_id.clone();
        let (mut procedure_claims, mut procedure_realizations) = procedure
            .materialize()
            .map_err(|error| format!("{procedure_id}:{error}"))
            .expect("probe procedure must contain complete source event steps");
        claims.append(&mut procedure_claims);
        event_realizations.append(&mut procedure_realizations);
    }
    for mut frame in action_frames {
        if frame.action_modality == ApprovedModalityIR::Asserted {
            if let Some(modality) = scoped_action_modality {
                frame.action_modality = modality;
            }
        }
        let action_proposition_id = frame.action_proposition_id.clone();
        let (mut frame_claims, realization) = frame
            .materialize()
            .map_err(|error| format!("{action_proposition_id}:{error}"))
            .expect("probe action frame must be a complete source event");
        claims.append(&mut frame_claims);
        event_realizations.push(realization);
    }
    response(
        claims,
        speech_act,
        discourse_relation,
        register,
        verbosity,
        event_realizations,
    )
}

fn action_frame(
    action_id: &str,
    action_label: &str,
    event_id: &str,
    event_label: &str,
    class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    voice: ApprovedEventVoiceIR,
    arguments: Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
) -> ApprovedActionEventFrameIR {
    action_frame_for_subject(
        &format!("P_{action_id}_ACTION"),
        action_id,
        action_label,
        event_id,
        event_label,
        class,
        predicate_sense,
        phase,
        voice,
        arguments,
    )
}

fn action_frame_for_subject(
    action_proposition_id: &str,
    action_subject_id: &str,
    action_subject_label: &str,
    event_id: &str,
    event_label: &str,
    class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    voice: ApprovedEventVoiceIR,
    arguments: Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
) -> ApprovedActionEventFrameIR {
    ApprovedActionEventFrameIR {
        action_proposition_id: action_proposition_id.into(),
        action_subject: node(action_subject_id, action_subject_label, ApprovedSemanticTypeIR::Event),
        event: node(event_id, event_label, ApprovedSemanticTypeIR::Event),
        class,
        predicate_sense,
        phase,
        perspective: ApprovedEventPerspectiveIR {
            voice,
            focus: match voice {
                ApprovedEventVoiceIR::Active => ApprovedEventFocusIR::Agent,
                ApprovedEventVoiceIR::Passive => ApprovedEventFocusIR::Theme,
            },
        },
        action_modality: ApprovedModalityIR::Asserted,
        arguments: arguments
            .into_iter()
            .enumerate()
            .map(|(index, (relation, value))| ApprovedActionEventArgumentIR {
                proposition_id: format!("P_{action_proposition_id}_ROLE_{index:02}"),
                relation,
                value,
            })
            .collect(),
    }
}

/// A procedural source may authorize an obligation without claiming that the
/// event has already happened.  The event frame remains fully typed; the
/// directive is carried by Canonical IR rather than inferred from Korean.
fn directive_action_frame(
    action_id: &str,
    action_label: &str,
    event_id: &str,
    event_label: &str,
    class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
    arguments: Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
) -> ApprovedActionEventFrameIR {
    let mut frame = action_frame(
        action_id,
        action_label,
        event_id,
        event_label,
        class,
        predicate_sense,
        ApprovedEventPhaseIR::Planned,
        ApprovedEventVoiceIR::Passive,
        arguments,
    );
    frame.action_modality = ApprovedModalityIR::Directive;
    frame
}

/// A procedure is a source-declared action with more than one ordered,
/// independently typed step.  Every step keeps its own Action proposition and
/// valency frame, while the shared action subject retains the procedure's
/// canonical identity.  This is structural source data, never a grouping
/// inferred from Korean surface text.
fn directive_procedure_step(
    procedure_id: &str,
    procedure_label: &str,
    step_id: &str,
    event_id: &str,
    event_label: &str,
    class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
    arguments: Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
) -> ApprovedActionEventFrameIR {
    let mut frame = directive_action_frame(
        step_id,
        procedure_label,
        event_id,
        event_label,
        class,
        predicate_sense,
        arguments,
    );
    frame.action_proposition_id = format!("P_{procedure_id}_STEP_{step_id}_ACTION");
    frame.action_subject = node(procedure_id, procedure_label, ApprovedSemanticTypeIR::Event);
    frame
}

fn phased_multi_event_document_response(
    mut leading_claims: Vec<ApprovedCompositionalClaimIR>,
    events: Vec<(
        &'static str,
        &'static str,
        ApprovedEventRealizationClassIR,
        ApprovedEventPredicateSenseIR,
        ApprovedEventPhaseIR,
        Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
    )>,
    mut trailing_claims: Vec<ApprovedCompositionalClaimIR>,
) -> ApprovedCompositionalResponseIR {
    let mut claims = Vec::new();
    claims.append(&mut leading_claims);
    let mut realizations = Vec::new();
    for (event_index, (key, label, class, sense, phase, roles)) in events.into_iter().enumerate() {
        for (role_index, (relation, value)) in roles.into_iter().enumerate() {
            claims.push(claim(
                &format!("P_LONG_EVENT_{event_index:02}_{role_index:02}"),
                key,
                label,
                relation,
                value,
            ));
        }
        realizations.push(ApprovedEventRealizationIR {
            subject_node_id: key.into(),
            class,
            predicate_sense: Some(sense),
            phase: Some(phase),
            perspective: Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
            voice: None,
            information_structure: None,
        });
    }
    claims.append(&mut trailing_claims);
    response(
        claims,
        ApprovedSpeechActIR::Inform,
        ApprovedDiscourseRelationIR::Statement,
        LanguageRegisterIR::Formal,
        ApprovedVerbosityIR::Explanatory,
        realizations,
    )
}

/// The phased document fixtures predate typed action frames.  Keep their
/// independently completed events intact while materializing every remaining
/// source Action through the same validated frame contract used elsewhere.
fn phased_action_event_document_response(
    leading_claims: Vec<ApprovedCompositionalClaimIR>,
    events: Vec<(
        &'static str,
        &'static str,
        ApprovedEventRealizationClassIR,
        ApprovedEventPredicateSenseIR,
        ApprovedEventPhaseIR,
        Vec<(ApprovedRelationTypeIR, ApprovedOpenValueIR)>,
    )>,
    mut trailing_claims: Vec<ApprovedCompositionalClaimIR>,
    action_frames: Vec<ApprovedActionEventFrameIR>,
) -> ApprovedCompositionalResponseIR {
    let mut materialized_actions = Vec::new();
    let mut action_realizations = Vec::new();
    for frame in action_frames {
        let action_proposition_id = frame.action_proposition_id.clone();
        let (mut claims, realization) = frame
            .materialize()
            .map_err(|error| format!("{action_proposition_id}:{error}"))
            .expect("phased probe action frame must be complete");
        materialized_actions.append(&mut claims);
        action_realizations.push(realization);
    }
    let phased = phased_multi_event_document_response(leading_claims, events, Vec::new());
    let mut claims = phased.claims;
    let mut event_realizations = phased.event_realizations;
    claims.append(&mut materialized_actions);
    claims.append(&mut trailing_claims);
    event_realizations.extend(action_realizations);
    response(
        claims,
        phased.speech_act,
        phased.discourse_relation,
        phased.style.register,
        phased.style.verbosity,
        event_realizations,
    )
}

fn context(
    format: WorkplaceCommunicationFormatIR,
    audience: WorkplaceAudienceIR,
    topic: &str,
) -> WorkplaceCommunicationContextIR {
    WorkplaceCommunicationContextIR {
        format,
        audience,
        topic_label: topic.into(),
    }
}

fn scenarios() -> Vec<Scenario> {
    use ApprovedEventPhaseIR::*;
    use ApprovedEventPredicateSenseIR::*;
    use ApprovedEventRealizationClassIR::*;
    use ApprovedRelationTypeIR::*;
    use WorkplaceAudienceIR::*;
    use WorkplaceCommunicationFormatIR::*;

    let person = |id: &str, label: &str| lexical(id, label, ApprovedSemanticTypeIR::Person);
    let concept = |id: &str, label: &str| lexical(id, label, ApprovedSemanticTypeIR::Concept);
    let place = |id: &str, label: &str| lexical(id, label, ApprovedSemanticTypeIR::Location);
    let mut rows = vec![
        Scenario {
            id: "CHAT_01",
            family: "peer_release_status",
            context: context(InstantMessage, Peer, "배포"),
            response: status_response(
                "release",
                "배포",
                "ready",
                "준비",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "CHAT_02",
            family: "team_room_availability",
            context: context(InstantMessage, Team, "회의실"),
            response: response(
                vec![claim(
                    "P_ROOM_AVAILABLE",
                    "meeting_room",
                    "회의실",
                    RoomAvailable,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "CHAT_03",
            family: "peer_file_send",
            context: context(InstantMessage, Peer, "수정본 전송"),
            response: event_response(
                "send_revision",
                "수정본 전송",
                ApprovedEventRealizationClassIR::Transfer,
                Send,
                Completed,
                vec![
                    (Agent, person("seoyeon", "서연")),
                    (Theme, concept("revision", "수정본")),
                    (Destination, place("team_chat", "팀 채팅방")),
                ],
                LanguageRegisterIR::Informal,
            ),
        },
        Scenario {
            id: "EMAIL_01",
            family: "external_contract_send",
            context: context(Email, ExternalPartner, "계약서 전송"),
            response: event_response(
                "contract_send",
                "계약서 전송",
                ApprovedEventRealizationClassIR::Transfer,
                Send,
                Scheduled,
                vec![
                    (Agent, person("sales_team", "영업팀")),
                    (Theme, concept("contract", "계약서")),
                    (
                        Destination,
                        place("client_email_address", "거래처 이메일 주소"),
                    ),
                    (
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 15,
                            minute: 0,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "EMAIL_02",
            family: "manager_monthly_report",
            context: context(Email, Manager, "월간 보고서 작성"),
            response: event_response(
                "monthly_report_write",
                "월간 보고서 작성",
                Creation,
                Write,
                Completed,
                vec![
                    (Agent, person("planning_team", "기획팀")),
                    (Theme, concept("monthly_report", "월간 보고서")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 19,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
    ];

    let meeting = response(
        vec![
            claim(
                "P_VENDOR_TIME",
                "vendor_meeting",
                "협력사 회의",
                Time,
                ApprovedOpenValueIR::Clock {
                    hour: 14,
                    minute: 30,
                },
            ),
            claim(
                "P_VENDOR_LOCATION",
                "vendor_meeting",
                "협력사 회의",
                Location,
                place("conference_c", "회의실 C"),
            ),
        ],
        ApprovedSpeechActIR::Inform,
        ApprovedDiscourseRelationIR::Statement,
        LanguageRegisterIR::Formal,
        ApprovedVerbosityIR::Short,
        Vec::new(),
    );
    rows.push(Scenario {
        id: "EMAIL_03",
        family: "external_meeting_schedule",
        context: context(Email, ExternalPartner, "협력사 회의"),
        response: meeting.clone(),
    });

    let more = vec![
        (
            "STATUS_01",
            "data_migration",
            StatusUpdate,
            Team,
            "데이터 이관",
            event_response(
                "migration",
                "데이터 이관",
                ApprovedEventRealizationClassIR::Transfer,
                ApprovedEventPredicateSenseIR::Transfer,
                Ongoing,
                vec![
                    (Agent, person("infra_team", "인프라팀")),
                    (Theme, concept("customer_data", "고객 데이터")),
                    (Destination, place("new_cluster", "신규 클러스터")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "STATUS_02",
            "settlement_upload",
            StatusUpdate,
            Manager,
            "정산 자료 업로드",
            event_response(
                "settlement_upload",
                "정산 자료 업로드",
                ApprovedEventRealizationClassIR::Transfer,
                Upload,
                Completed,
                vec![
                    (Agent, person("finance_team", "재무팀")),
                    (Theme, concept("settlement_file", "정산 자료")),
                    (Destination, place("shared_drive", "공유 드라이브")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "STATUS_03",
            "support_queue",
            StatusUpdate,
            Team,
            "고객 문의",
            response(
                vec![claim(
                    "P_SUPPORT_COUNT",
                    "support",
                    "고객 문의",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 14,
                        unit: node("support_case_unit", "건", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "INCIDENT_01",
            "payment_incident",
            IncidentAlert,
            Team,
            "결제 서비스",
            status_response(
                "payment_service",
                "결제 서비스",
                "inspection",
                "점검 중",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
            ),
        ),
        (
            "INCIDENT_02",
            "recovery_progress",
            IncidentAlert,
            Executive,
            "장애 대응",
            status_response(
                "incident_response",
                "장애 대응",
                "in_progress",
                "진행 중",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
            ),
        ),
        (
            "INCIDENT_03",
            "connection_reset",
            IncidentAlert,
            Team,
            "설정 초기화",
            event_response(
                "connection_reset",
                "설정 초기화",
                StateChange,
                Reset,
                Ongoing,
                vec![
                    (Agent, person("database_team", "데이터베이스팀")),
                    (Theme, concept("connection_settings", "접속 설정")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "MEETING_01",
            "vendor_meeting_summary",
            MeetingSummary,
            Team,
            "협력사 회의",
            meeting.clone(),
        ),
        (
            "MEETING_02",
            "design_confirmed",
            MeetingSummary,
            Team,
            "설계 검토",
            response(
                vec![claim(
                    "P_DESIGN_CONFIRMED",
                    "design_review",
                    "설계 검토",
                    Confirmed,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Confirmation,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "MEETING_03",
            "interview_duration",
            MeetingSummary,
            Manager,
            "채용 면접",
            response(
                vec![claim(
                    "P_INTERVIEW_DURATION",
                    "interview",
                    "채용 면접",
                    Duration,
                    ApprovedOpenValueIR::Quantity {
                        amount: 60,
                        unit: node("minute", "분", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "HANDOFF_01",
            "asset_move",
            HandoffNote,
            Peer,
            "원본 파일 이동",
            event_response(
                "asset_move",
                "원본 파일 이동",
                Motion,
                Move,
                Completed,
                vec![
                    (Agent, person("jihoon", "지훈")),
                    (Theme, concept("source_files", "원본 파일")),
                    (Destination, place("archive_server", "보관 서버")),
                ],
                LanguageRegisterIR::Neutral,
            ),
        ),
        (
            "HANDOFF_02",
            "client_delivery",
            HandoffNote,
            Peer,
            "고객 자료 전달",
            event_response(
                "client_delivery",
                "고객 자료 전달",
                ApprovedEventRealizationClassIR::Transfer,
                Deliver,
                Completed,
                vec![
                    (Agent, person("day_shift", "주간 담당자")),
                    (Patient, person("client_manager", "고객사 담당자")),
                    (Theme, concept("test_result", "시험 결과")),
                ],
                LanguageRegisterIR::Neutral,
            ),
        ),
        (
            "HANDOFF_03",
            "permission_change",
            HandoffNote,
            Team,
            "계정 권한 변경",
            event_response(
                "permission_change",
                "계정 권한 변경",
                StateChange,
                Change,
                Completed,
                vec![
                    (Agent, person("admin", "관리자")),
                    (Theme, concept("account_permission", "계정 권한")),
                    (InitialState, ApprovedOpenValueIR::Text("읽기 전용".into())),
                    (ResultState, ApprovedOpenValueIR::Text("편집 가능".into())),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "APPROVAL_01",
            "purchase_confirmation",
            ApprovalRequest,
            Manager,
            "구매 요청",
            response(
                vec![claim(
                    "P_PURCHASE_CONFIRM",
                    "purchase_request",
                    "구매 요청",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "APPROVAL_02",
            "leave_confirmation",
            ApprovalRequest,
            Manager,
            "휴가 일정",
            response(
                vec![claim(
                    "P_LEAVE_CONFIRM",
                    "leave_schedule",
                    "휴가 일정",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "APPROVAL_03",
            "release_time_query",
            ApprovalRequest,
            Manager,
            "배포 일정",
            response(
                vec![claim(
                    "P_RELEASE_CONFIRM",
                    "release_schedule",
                    "배포 일정",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CORRECTION_01",
            "training_time_fix",
            CorrectionNotice,
            Team,
            "보안 교육",
            response(
                vec![claim(
                    "P_TRAINING_TIME",
                    "security_training",
                    "보안 교육",
                    Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 0,
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CORRECTION_02",
            "inspection_place_fix",
            CorrectionNotice,
            Team,
            "시설 점검",
            response(
                vec![claim(
                    "P_INSPECTION_PLACE",
                    "facility_inspection",
                    "시설 점검",
                    Location,
                    place("east_building", "동관 2층"),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CORRECTION_03",
            "shipment_count_fix",
            CorrectionNotice,
            ExternalPartner,
            "출고 수량",
            response(
                vec![claim(
                    "P_SHIPMENT_COUNT",
                    "shipment",
                    "출고 수량",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 128,
                        unit: node("shipment_piece_unit", "개", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
    ];
    rows.extend(
        more.into_iter()
            .map(|(id, family, format, audience, topic, response)| Scenario {
                id,
                family,
                context: context(format, audience, topic),
                response,
            }),
    );

    // A second, deliberately non-overlapping workplace set exercises ordinary
    // coordination, expenses, leave, customer work, handoff, corrections and
    // management reporting.  These are new canonical meanings rather than
    // paraphrases of the first probe rows.
    let broad = vec![
        (
            "CHAT_04",
            "expense_evidence_query",
            InstantMessage,
            Peer,
            "출장비 증빙",
            response(
                vec![claim(
                    "P_EXPENSE_REGISTRATION",
                    "expense_evidence",
                    "출장비 증빙",
                    Registration,
                    lexical("registered", "완료", ApprovedSemanticTypeIR::State),
                )],
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CHAT_05",
            "meeting_minutes_request",
            InstantMessage,
            Peer,
            "회의록",
            status_response(
                "meeting_minutes",
                "회의록",
                "ready",
                "준비",
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Short,
            ),
        ),
        (
            "CHAT_06",
            "corporate_card_receipt_promise",
            InstantMessage,
            Peer,
            "법인카드 영수증",
            response(
                vec![claim(
                    "P_CARD_RECEIPT_REGISTRATION",
                    "card_receipt",
                    "법인카드 영수증",
                    Registration,
                    lexical("registered", "완료", ApprovedSemanticTypeIR::State),
                )],
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CHAT_07",
            "remote_work_acknowledgement",
            InstantMessage,
            Team,
            "재택근무",
            response(
                vec![claim(
                    "P_REMOTE_DATE",
                    "remote_schedule",
                    "재택근무",
                    Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 9,
                        day: 23,
                    },
                )],
                ApprovedSpeechActIR::Acknowledge,
                ApprovedDiscourseRelationIR::Confirmation,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "EMAIL_04",
            "vendor_quote_review",
            Email,
            Manager,
            "협력사 견적서",
            response(
                vec![claim(
                    "P_QUOTE_CONFIRM",
                    "vendor_quote",
                    "협력사 견적서",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "EMAIL_05",
            "tax_invoice_date_correction",
            Email,
            ExternalPartner,
            "세금계산서 발행",
            response(
                vec![claim(
                    "P_TAX_INVOICE_DATE",
                    "tax_invoice",
                    "세금계산서 발행",
                    Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 9,
                        day: 25,
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "EMAIL_06",
            "delivery_schedule_reassurance",
            Email,
            ExternalPartner,
            "납품 일정",
            response(
                vec![claim(
                    "P_DELIVERY_CONFIRMED",
                    "delivery_schedule",
                    "납품 일정",
                    Confirmed,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "STATUS_04",
            "unassigned_ticket_count",
            StatusUpdate,
            Team,
            "미배정 문의",
            response(
                vec![claim(
                    "P_UNASSIGNED_TICKET_COUNT",
                    "unassigned_ticket",
                    "미배정 문의",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 23,
                        unit: node("ticket_unit", "건", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "STATUS_05",
            "ux_review_progress",
            StatusUpdate,
            Team,
            "화면 설계 검토",
            status_response(
                "ux_review",
                "화면 설계 검토",
                "under_review",
                "검토 중",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
            ),
        ),
        (
            "STATUS_06",
            "onboarding_checklist_creation",
            StatusUpdate,
            Team,
            "입사자 점검표 작성",
            event_response(
                "onboarding_checklist",
                "입사자 점검표 작성",
                Creation,
                Write,
                Completed,
                vec![
                    (Agent, person("hr_team", "인사팀")),
                    (Theme, concept("onboarding_checklist", "입사자 점검표")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "INCIDENT_04",
            "server_room_unavailable",
            IncidentAlert,
            Team,
            "서버실",
            response(
                vec![claim(
                    "P_SERVER_ROOM_UNAVAILABLE",
                    "server_room",
                    "서버실",
                    RoomAvailable,
                    ApprovedOpenValueIR::Boolean(false),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "INCIDENT_05",
            "night_deployment_cancelled",
            IncidentAlert,
            Team,
            "야간 배포",
            response(
                vec![claim(
                    "P_NIGHT_DEPLOYMENT_CANCELLED",
                    "night_deployment",
                    "야간 배포",
                    ApprovedRelationTypeIR::Cancelled,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "INCIDENT_06",
            "failed_job_count",
            IncidentAlert,
            Executive,
            "실패 작업",
            response(
                vec![claim(
                    "P_FAILED_JOB_COUNT",
                    "failed_job",
                    "실패 작업",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 37,
                        unit: node("job_unit", "건", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "MEETING_04",
            "launch_review_schedule",
            MeetingSummary,
            Team,
            "출시 검토 회의",
            response(
                vec![
                    claim(
                        "P_LAUNCH_REVIEW_TIME",
                        "launch_review",
                        "출시 검토 회의",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 10,
                            minute: 30,
                        },
                    ),
                    claim(
                        "P_LAUNCH_REVIEW_LOCATION",
                        "launch_review",
                        "출시 검토 회의",
                        Location,
                        place("meeting_room_b", "회의실 B"),
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "MEETING_05",
            "recruitment_plan_confirmation",
            MeetingSummary,
            Manager,
            "채용 계획 회의",
            response(
                vec![claim(
                    "P_RECRUITMENT_CONFIRMED",
                    "recruitment_plan",
                    "채용 계획 회의",
                    Confirmed,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Confirmation,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "MEETING_06",
            "action_item_write",
            MeetingSummary,
            Team,
            "프로젝트 회의",
            event_response(
                "action_item_write",
                "프로젝트 회의",
                Creation,
                Write,
                Planned,
                vec![
                    (Agent, person("project_manager", "프로젝트 매니저")),
                    (Theme, concept("action_items", "후속 조치 목록")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "HANDOFF_04",
            "log_bundle_upload",
            HandoffNote,
            Peer,
            "로그 묶음 업로드",
            event_response(
                "log_bundle_upload",
                "로그 묶음 업로드",
                ApprovedEventRealizationClassIR::Transfer,
                Upload,
                Completed,
                vec![
                    (Agent, person("evening_shift", "오후 담당자")),
                    (Theme, concept("log_bundle", "로그 묶음")),
                    (Destination, place("incident_folder", "장애 공유 폴더")),
                ],
                LanguageRegisterIR::Neutral,
            ),
        ),
        (
            "HANDOFF_05",
            "shared_account_deactivation",
            HandoffNote,
            Team,
            "공용 계정 비활성화",
            event_response(
                "shared_account_deactivation",
                "공용 계정 비활성화",
                StateChange,
                Deactivate,
                Completed,
                vec![
                    (Agent, person("security_manager", "보안 담당자")),
                    (Theme, concept("shared_account", "공용 계정")),
                ],
                LanguageRegisterIR::Formal,
            ),
        ),
        (
            "HANDOFF_06",
            "access_card_delivery",
            HandoffNote,
            Peer,
            "출입카드 전달",
            event_response(
                "access_card_delivery",
                "출입카드 전달",
                ApprovedEventRealizationClassIR::Transfer,
                Deliver,
                Completed,
                vec![
                    (Agent, person("day_duty", "주간 당직자")),
                    (Patient, person("night_duty", "야간 당직자")),
                    (Theme, concept("access_card", "출입카드")),
                ],
                LanguageRegisterIR::Neutral,
            ),
        ),
        (
            "APPROVAL_04",
            "training_budget_approval",
            ApprovalRequest,
            Manager,
            "교육 예산",
            response(
                vec![claim(
                    "P_TRAINING_BUDGET_CONFIRM",
                    "training_budget",
                    "교육 예산",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "APPROVAL_05",
            "overtime_meal_approval",
            ApprovalRequest,
            Manager,
            "야근 식대",
            response(
                vec![claim(
                    "P_OVERTIME_MEAL_CONFIRM",
                    "overtime_meal",
                    "야근 식대",
                    Approved,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CORRECTION_04",
            "shipment_date_correction",
            CorrectionNotice,
            ExternalPartner,
            "부품 출고",
            response(
                vec![claim(
                    "P_COMPONENT_SHIPMENT_DATE",
                    "component_shipment",
                    "부품 출고",
                    Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 9,
                        day: 28,
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "CORRECTION_05",
            "fire_drill_correction",
            CorrectionNotice,
            Team,
            "소방 훈련",
            response(
                vec![
                    claim(
                        "P_FIRE_DRILL_TIME",
                        "fire_drill",
                        "소방 훈련",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 14,
                            minute: 0,
                        },
                    ),
                    claim(
                        "P_FIRE_DRILL_LOCATION",
                        "fire_drill",
                        "소방 훈련",
                        Location,
                        place("west_parking_lot", "서편 주차장"),
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        ),
        (
            "INCIDENT_07",
            "payment_deadlock_response",
            IncidentAlert,
            Executive,
            "결제 장애",
            action_event_document_response(
                vec![
                    claim(
                        "P_PAYMENT_CAUSE",
                        "payment_incident",
                        "결제 장애",
                        Cause,
                        concept("coupon_deadlock", "쿠폰 검증 교착 상태"),
                    ),
                    claim(
                        "P_PAYMENT_IMPACT",
                        "payment_incident",
                        "결제 장애",
                        Impact,
                        concept("payment_delay", "결제 승인 지연"),
                    ),
                    claim(
                        "P_PAYMENT_OWNER",
                        "payment_incident",
                        "결제 장애",
                        Owner,
                        person("platform_team", "플랫폼팀"),
                    ),
                    claim(
                        "P_PAYMENT_DEADLINE",
                        "payment_incident",
                        "결제 장애",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 13,
                            minute: 0,
                        },
                    ),
                ],
                vec![action_frame(
                    "payment_incident",
                    "결제 장애",
                    "coupon_deactivation",
                    "할인 쿠폰 배포 비활성화",
                    StateChange,
                    Deactivate,
                    Planned,
                    ApprovedEventVoiceIR::Active,
                    vec![
                        (Agent, person("platform_team", "플랫폼팀")),
                        (
                            Theme,
                            concept("coupon_distribution", "할인 쿠폰 배포"),
                        ),
                    ],
                )],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Explanation,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        ),
        (
            "STATUS_07",
            "supplier_delay_response",
            StatusUpdate,
            Manager,
            "원자재 입고 지연",
            action_event_document_response(
                vec![
                    claim(
                        "P_SUPPLIER_CAUSE",
                        "supplier_delay",
                        "원자재 입고 지연",
                        Cause,
                        concept("port_strike", "항만 파업"),
                    ),
                    claim(
                        "P_SUPPLIER_IMPACT",
                        "supplier_delay",
                        "원자재 입고 지연",
                        Impact,
                        concept("line_stop_risk", "조립 2라인 가동 중단 위험"),
                    ),
                    claim(
                        "P_SUPPLIER_OWNER",
                        "supplier_delay",
                        "원자재 입고 지연",
                        Owner,
                        person("procurement_team", "구매팀"),
                    ),
                    claim(
                        "P_SUPPLIER_DEADLINE",
                        "supplier_delay",
                        "원자재 입고 지연",
                        Deadline,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 21,
                        },
                    ),
                ],
                vec![action_frame_for_subject(
                    "P_SUPPLIER_ACTION",
                    "supplier_delay",
                    "원자재 입고 지연",
                    "supplier_emergency_order_event",
                    "국내 대체 벤더 긴급 발주",
                    ApprovedEventRealizationClassIR::Transfer,
                    Send,
                    Planned,
                    ApprovedEventVoiceIR::Active,
                    vec![
                        (Agent, person("procurement_team", "구매팀")),
                        (Theme, concept("emergency_order", "긴급 발주")),
                    (Destination, concept("domestic_alternate_vendor", "국내 대체 벤더")),
                    ],
                )],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        ),
    ];
    rows.extend(
        broad
            .into_iter()
            .map(|(id, family, format, audience, topic, response)| Scenario {
                id,
                family,
                context: context(format, audience, topic),
                response,
            }),
    );

    let executive =
        |id: &'static str, family: &'static str, topic: &'static str, values: [(&str, i64); 3]| {
            let claims = values
                .into_iter()
                .enumerate()
                .map(|(index, (label, amount))| {
                    claim(
                        &format!("P_EXEC_{id}_{index}"),
                        &format!("exec_{id}_{index}"),
                        label,
                        Count,
                        ApprovedOpenValueIR::Quantity {
                            amount,
                            unit: node(
                                &format!("case_unit_{id}_{index}"),
                                "건",
                                ApprovedSemanticTypeIR::Unit,
                            ),
                        },
                    )
                })
                .collect();
            Scenario {
                id,
                family,
                context: context(ExecutiveBrief, Executive, topic),
                response: response(
                    claims,
                    ApprovedSpeechActIR::Explain,
                    ApprovedDiscourseRelationIR::Comparison,
                    LanguageRegisterIR::Formal,
                    ApprovedVerbosityIR::Explanatory,
                    Vec::new(),
                ),
            }
        };
    rows.extend([
        executive(
            "EXEC_01",
            "regional_backlog",
            "서울 미처리 건",
            [
                ("서울 미처리 건", 18),
                ("부산 미처리 건", 11),
                ("대전 미처리 건", 7),
            ],
        ),
        executive(
            "EXEC_02",
            "channel_sales",
            "직판 계약 건",
            [
                ("직판 계약 건", 24),
                ("파트너 계약 건", 17),
                ("온라인 계약 건", 31),
            ],
        ),
        executive(
            "EXEC_03",
            "quality_defects",
            "조립 불량 건",
            [
                ("조립 불량 건", 9),
                ("포장 불량 건", 4),
                ("검수 불량 건", 6),
            ],
        ),
    ]);

    // Deliberately unrelated holdout families. These meanings avoid the
    // release/meeting/payment/supplier vocabulary used by the main probe so
    // productive Korean realization is exercised rather than remembered
    // workplace phrasing.
    rows.extend([
        Scenario {
            id: "NOVEL_01",
            family: "sea_turtle_quarantine_transfer",
            context: context(InstantMessage, Peer, "바다거북 이송"),
            response: event_response(
                "turtle_transfer",
                "바다거북 이송",
                Motion,
                Move,
                Completed,
                vec![
                    (Agent, person("aquarist", "사육사")),
                    (Theme, concept("sea_turtle", "바다거북")),
                    (Source, place("quarantine_tank", "검역 수조")),
                    (Destination, place("recovery_tank", "회복 수조")),
                    (Instrument, concept("padded_stretcher", "완충 들것")),
                ],
                LanguageRegisterIR::Informal,
            ),
        },
        Scenario {
            id: "NOVEL_02",
            family: "archive_map_upload",
            context: context(Email, ExternalPartner, "고지도 원본 등록"),
            response: event_response(
                "map_upload",
                "고지도 원본 등록",
                ApprovedEventRealizationClassIR::Transfer,
                Upload,
                Completed,
                vec![
                    (Agent, person("archivist", "기록연구사")),
                    (Theme, concept("old_map_scan", "고지도 초고해상도 스캔본")),
                    (
                        Destination,
                        place("preservation_repository", "디지털 보존 저장소"),
                    ),
                    (
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 16,
                            minute: 20,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_03",
            family: "telescope_mount_reset",
            context: context(StatusUpdate, Team, "적도의 영점 재설정"),
            response: event_response(
                "mount_reset",
                "적도의 영점 재설정",
                StateChange,
                Reset,
                Ongoing,
                vec![
                    (Agent, person("night_observer", "야간 관측자")),
                    (Theme, concept("equatorial_mount", "적도의 구동축")),
                    (Location, place("dome_three", "제3관측돔")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_04",
            family: "sourdough_culture_status",
            context: context(InstantMessage, Peer, "천연발효종"),
            response: status_response(
                "starter_culture",
                "천연발효종",
                "peak_fermentation",
                "최고 발효점 도달",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "NOVEL_05",
            family: "beehive_inspection_cause",
            context: context(IncidentAlert, Team, "벌통 내검 중단"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_HIVE_CAUSE",
                        "hive_inspection_stop",
                        "벌통 내검 중단",
                        Cause,
                        concept("queen_agitation", "여왕벌 군집의 과도한 흥분"),
                    ),
                    claim(
                        "P_HIVE_OWNER",
                        "hive_inspection_stop",
                        "벌통 내검 중단",
                        Owner,
                        person("apiary_lead", "양봉장 책임자"),
                    ),
                ],
                vec![
                    action_frame("smoker_deactivation", "훈연기 사용 중지", "smoker_deactivation_event", "훈연기 사용 중지", StateChange, Deactivate, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("smoker", "훈연기"))]),
                    action_frame("worker_withdrawal", "작업자 후퇴", "worker_withdrawal_event", "작업자 후퇴", Motion, Withdraw, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("apiary_workers", "작업자"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "NOVEL_06",
            family: "fresco_consolidant_application",
            context: context(HandoffNote, Team, "벽화 보존 처리"),
            response: event_response(
                "fresco_treatment",
                "벽화 보존 처리",
                ApprovedEventRealizationClassIR::Transfer,
                Apply,
                Completed,
                vec![
                    (Agent, person("conservator", "보존처리사")),
                    (Theme, concept("lime_consolidant", "석회계 강화제")),
                    (Target, concept("fresco_flaking_layer", "벽화 박락층")),
                    (Instrument, concept("micro_syringe", "미세 주사기")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_07",
            family: "greenhouse_shade_expansion",
            context: context(ApprovalRequest, Manager, "차광 범위 확대"),
            response: event_response(
                "shade_expansion",
                "차광 범위 확대",
                StateChange,
                Expand,
                Planned,
                vec![
                    (Agent, person("cultivation_team", "재배팀")),
                    (Theme, concept("shade_range", "북측 온실 차광 범위")),
                    (Target, place("remaining_bench", "잔여 재배대 상부")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_08",
            family: "theater_prop_count_correction",
            context: context(CorrectionNotice, Team, "종이등 소품"),
            response: response(
                vec![claim(
                    "P_PROP_COUNT",
                    "lantern_prop",
                    "종이등 소품",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 23,
                        unit: node("prop_piece", "개", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "NOVEL_09",
            family: "seed_bank_freezer_alarm",
            context: context(IncidentAlert, Team, "종자은행 냉동고 경보"),
            response: response(
                vec![
                    claim(
                        "P_FREEZER_CAUSE",
                        "freezer_alarm",
                        "종자은행 냉동고 경보",
                        Cause,
                        concept("door_gasket_gap", "문짝 패킹 틈새"),
                    ),
                    claim(
                        "P_FREEZER_IMPACT",
                        "freezer_alarm",
                        "종자은행 냉동고 경보",
                        Impact,
                        concept("humidity_rise", "내부 습도 상승"),
                    ),
                    claim(
                        "P_FREEZER_DEADLINE",
                        "freezer_alarm",
                        "종자은행 냉동고 경보",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 10,
                            minute: 40,
                        },
                    ),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Explanation,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "NOVEL_10",
            family: "buoy_data_send",
            context: context(Email, ExternalPartner, "관측값 전송"),
            response: event_response(
                "buoy_send",
                "관측값 전송",
                ApprovedEventRealizationClassIR::Transfer,
                Send,
                Scheduled,
                vec![
                    (Agent, person("coastal_lab", "연안관측소")),
                    (Theme, concept("salinity_series", "시간대별 염분 관측값")),
                    (Destination, place("ocean_archive", "국가해양자료센터")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 22,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_11",
            family: "puppet_joint_creation",
            context: context(StatusUpdate, Team, "인형 관절 제작"),
            response: event_response(
                "puppet_joint",
                "인형 관절 제작",
                Creation,
                Create,
                Ongoing,
                vec![
                    (Agent, person("prop_maker", "소품 제작자")),
                    (Theme, concept("maple_joint", "단풍나무 구형 관절")),
                    (Instrument, concept("miniature_lathe", "소형 목선반")),
                ],
                LanguageRegisterIR::Neutral,
            ),
        },
        Scenario {
            id: "NOVEL_12",
            family: "meteorite_case_close",
            context: context(HandoffNote, Team, "운석 진열장 밀폐"),
            response: event_response(
                "case_close",
                "운석 진열장 밀폐",
                StateChange,
                Close,
                Completed,
                vec![
                    (Agent, person("collection_manager", "수장고 담당자")),
                    (Theme, concept("meteorite_case", "철질 운석 진열장")),
                    (Manner, concept("argon_purge", "아르곤으로 치환한 뒤")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_13",
            family: "rehearsal_start_time",
            context: context(MeetingSummary, Team, "현악 합주 리허설"),
            response: response(
                vec![
                    claim(
                        "P_REHEARSAL_TIME",
                        "string_rehearsal",
                        "현악 합주 리허설",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 18,
                            minute: 35,
                        },
                    ),
                    claim(
                        "P_REHEARSAL_LOCATION",
                        "string_rehearsal",
                        "현악 합주 리허설",
                        Location,
                        place("rehearsal_room_b", "지하 제2연습실"),
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "NOVEL_14",
            family: "lichen_sample_delivery",
            context: context(InstantMessage, Peer, "지의류 표본 인계"),
            response: event_response(
                "lichen_delivery",
                "지의류 표본 인계",
                ApprovedEventRealizationClassIR::Transfer,
                Deliver,
                Completed,
                vec![
                    (Agent, person("field_ecologist", "현장 생태조사원")),
                    (Patient, person("lab_curator", "표본실 관리자")),
                    (Theme, concept("lichen_packet", "건조 지의류 표본 봉투")),
                    (Location, place("herbarium", "식물표본관")),
                ],
                LanguageRegisterIR::Informal,
            ),
        },
        Scenario {
            id: "NOVEL_15",
            family: "ceramic_kiln_completion",
            context: context(StatusUpdate, Team, "청자 재현 소성"),
            response: event_response(
                "celadon_firing",
                "청자 재현 소성",
                StateChange,
                Complete,
                Completed,
                vec![
                    (Agent, person("kiln_operator", "가마 운용자")),
                    (
                        Theme,
                        concept("celadon_test_batch", "청자 시험편 4점의 소성"),
                    ),
                    (Location, place("gas_kiln", "환원염 가스가마")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_16",
            family: "cave_sensor_activation",
            context: context(ApprovalRequest, Manager, "미기후 센서 활성화"),
            response: event_response(
                "cave_sensor",
                "미기후 센서 활성화",
                StateChange,
                Activate,
                Planned,
                vec![
                    (Agent, person("karst_team", "카르스트 조사팀")),
                    (Theme, concept("co2_logger", "이산화탄소 기록계")),
                    (Location, place("lower_chamber", "동굴 하부 공동")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_17",
            family: "bat_detector_readiness_query",
            context: context(InstantMessage, Team, "박쥐 초음파 기록기"),
            response: status_response(
                "bat_detector",
                "박쥐 초음파 기록기",
                "ready",
                "준비",
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "NOVEL_18",
            family: "falcon_carrier_request",
            context: context(InstantMessage, Peer, "새끼 매 이동장"),
            response: status_response(
                "falcon_carrier",
                "새끼 매 이동장",
                "ready",
                "준비",
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "NOVEL_19",
            family: "perfume_trial_registration_promise",
            context: context(Email, ExternalPartner, "시향 기록"),
            response: response(
                vec![claim(
                    "P_SCENT_REGISTRATION",
                    "scent_trial",
                    "시향 기록",
                    Registration,
                    lexical("complete", "완료", ApprovedSemanticTypeIR::State),
                )],
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "NOVEL_20",
            family: "cold_imaging_room_reassurance",
            context: context(InstantMessage, Team, "저온 촬영실"),
            response: response(
                vec![claim(
                    "P_COLD_ROOM_AVAILABLE",
                    "cold_imaging_room",
                    "저온 촬영실",
                    RoomAvailable,
                    ApprovedOpenValueIR::Boolean(true),
                )],
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "NOVEL_21",
            family: "book_damage_register_creation",
            context: context(HandoffNote, Team, "고서 훼손 목록 작성"),
            response: event_response(
                "damage_register",
                "고서 훼손 목록 작성",
                Creation,
                Write,
                Completed,
                vec![
                    (Agent, person("book_conservator", "서화 보존가")),
                    (
                        Theme,
                        concept("damage_register_sheet", "낙장과 충해 흔적 목록"),
                    ),
                    (Location, place("rare_book_room", "고서 정리실")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_22",
            family: "raptor_record_handover",
            context: context(InstantMessage, Peer, "황조롱이 진료 기록 인계"),
            response: event_response(
                "raptor_record_handover",
                "황조롱이 진료 기록 인계",
                ApprovedEventRealizationClassIR::Transfer,
                Give,
                Completed,
                vec![
                    (Agent, person("wildlife_rehabilitator", "야생동물 재활사")),
                    (Patient, person("avian_vet", "조류 수의사")),
                    (Theme, concept("raptor_chart", "황조롱이 진료 기록")),
                ],
                LanguageRegisterIR::Informal,
            ),
        },
        Scenario {
            id: "NOVEL_23",
            family: "perfume_concentrate_color_change",
            context: context(StatusUpdate, Team, "향수 농축액 색상 변화"),
            response: event_response(
                "perfume_color_change",
                "향수 농축액 색상 변화",
                StateChange,
                Change,
                Completed,
                vec![
                    (Agent, person("perfumer", "조향사")),
                    (Theme, concept("perfume_concentrate", "향수 농축액")),
                    (
                        InitialState,
                        lexical("clear_state", "무색", ApprovedSemanticTypeIR::State),
                    ),
                    (
                        ResultState,
                        lexical("amber_state", "호박색", ApprovedSemanticTypeIR::State),
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_24",
            family: "lighthouse_shutter_opening",
            context: context(StatusUpdate, Team, "등명기 차광 셔터 개방"),
            response: event_response(
                "lighthouse_shutter",
                "등명기 차광 셔터 개방",
                StateChange,
                Open,
                Completed,
                vec![
                    (Agent, person("lighthouse_keeper", "등대 관리원")),
                    (Theme, concept("lamp_shutter", "등명기 차광 셔터")),
                    (
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 19,
                            minute: 5,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "NOVEL_25",
            family: "ice_core_tube_count_correction",
            context: context(CorrectionNotice, Team, "빙핵 보관 튜브"),
            response: response(
                vec![claim(
                    "P_ICE_TUBE_COUNT",
                    "ice_core_tube",
                    "빙핵 보관 튜브",
                    Count,
                    ApprovedOpenValueIR::Quantity {
                        amount: 31,
                        unit: node("tube_unit", "개", ApprovedSemanticTypeIR::Unit),
                    },
                )],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "SEALED_01",
            family: "escapement_model_transfer",
            context: context(InstantMessage, Peer, "탈진기 모형 이동"),
            response: event_response(
                "escapement_move",
                "탈진기 모형 이동",
                Motion,
                Move,
                Completed,
                vec![
                    (Agent, person("clock_restorer", "시계 복원사")),
                    (Theme, concept("escapement_model", "탈진기 모형")),
                    (Source, place("microscope_bench", "현미경 작업대")),
                    (Destination, place("vibration_case", "방진 보관함")),
                    (Instrument, concept("soft_tweezers", "연질 핀셋")),
                ],
                LanguageRegisterIR::Informal,
            ),
        },
        Scenario {
            id: "SEALED_02",
            family: "radio_spectrogram_upload",
            context: context(Email, ExternalPartner, "전파 분광도 등록"),
            response: event_response(
                "spectrogram_upload",
                "전파 분광도 등록",
                ApprovedEventRealizationClassIR::Transfer,
                Upload,
                Completed,
                vec![
                    (Agent, person("radio_observatory", "전파관측소")),
                    (Theme, concept("hydrogen_spectrogram", "중성수소선 분광도")),
                    (Destination, place("astronomy_archive", "천문자료 보존소")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 24,
                        },
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "SEALED_03",
            family: "beetle_label_application",
            context: context(HandoffNote, Team, "딱정벌레 표본 라벨 부착"),
            response: event_response(
                "beetle_label",
                "딱정벌레 표본 라벨 부착",
                ApprovedEventRealizationClassIR::StateChange,
                Attach,
                Completed,
                vec![
                    (Agent, person("entomologist", "곤충분류사")),
                    (Theme, concept("locality_label", "채집지 라벨")),
                    (Target, concept("pinned_beetle", "건조 딱정벌레 표본")),
                    (Instrument, concept("fine_forceps", "정밀 핀셋")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "SEALED_04",
            family: "acoustic_panel_coverage_expansion",
            context: context(ApprovalRequest, Manager, "흡음 패널 범위 확대"),
            response: event_response(
                "panel_expansion",
                "흡음 패널 범위 확대",
                StateChange,
                Expand,
                Planned,
                vec![
                    (Agent, person("acoustic_team", "음향 설계팀")),
                    (Theme, concept("panel_coverage", "흡음 패널 설치 범위")),
                    (Target, place("rear_wall", "객석 후면 벽")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "SEALED_05",
            family: "olive_brine_maturation_status",
            context: context(InstantMessage, Team, "올리브 염수 숙성액"),
            response: status_response(
                "olive_brine",
                "올리브 염수 숙성액",
                "maturity_peak",
                "권장 숙성점 도달",
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "SEALED_06",
            family: "vivarium_humidity_incident",
            context: context(IncidentAlert, Team, "양서류 사육장 습도 경보"),
            response: response(
                vec![
                    claim(
                        "P_VIVARIUM_CAUSE",
                        "vivarium_alarm",
                        "양서류 사육장 습도 경보",
                        Cause,
                        concept("mist_nozzle_block", "분무 노즐 막힘"),
                    ),
                    claim(
                        "P_VIVARIUM_IMPACT",
                        "vivarium_alarm",
                        "양서류 사육장 습도 경보",
                        Impact,
                        concept("humidity_drop", "격리 칸 습도 저하"),
                    ),
                    claim(
                        "P_VIVARIUM_OWNER",
                        "vivarium_alarm",
                        "양서류 사육장 습도 경보",
                        Owner,
                        person("vivarium_keeper", "사육장 관리자"),
                    ),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "SEALED_07",
            family: "poetry_reading_rehearsal",
            context: context(MeetingSummary, Team, "시 낭독 리허설"),
            response: response(
                vec![
                    claim(
                        "P_READING_TIME",
                        "poetry_rehearsal",
                        "시 낭독 리허설",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 14,
                            minute: 15,
                        },
                    ),
                    claim(
                        "P_READING_LOCATION",
                        "poetry_rehearsal",
                        "시 낭독 리허설",
                        Location,
                        place("small_theater", "소극장 무대"),
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "SEALED_08",
            family: "falcon_telemetry_readiness_query",
            context: context(InstantMessage, Team, "매 위치추적 송신기"),
            response: status_response(
                "falcon_transmitter",
                "매 위치추적 송신기",
                "ready",
                "준비",
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "SEALED_09",
            family: "tea_aroma_vial_request",
            context: context(InstantMessage, Peer, "차 향기 표본병"),
            response: status_response(
                "tea_aroma_vial",
                "차 향기 표본병",
                "ready",
                "준비",
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Short,
            ),
        },
        Scenario {
            id: "SEALED_10",
            family: "puppet_costume_registration_promise",
            context: context(Email, ExternalPartner, "인형극 의상 대장"),
            response: response(
                vec![claim(
                    "P_COSTUME_REGISTER",
                    "puppet_costume_register",
                    "인형극 의상 대장",
                    Registration,
                    lexical("complete", "완료", ApprovedSemanticTypeIR::State),
                )],
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Short,
                Vec::new(),
            ),
        },
        Scenario {
            id: "SEALED_11",
            family: "indigo_vat_color_change",
            context: context(StatusUpdate, Team, "쪽빛 염료조 색상 변화"),
            response: event_response(
                "indigo_color_change",
                "쪽빛 염료조 색상 변화",
                StateChange,
                Change,
                Completed,
                vec![
                    (Agent, person("dyer", "염색 장인")),
                    (Theme, concept("indigo_vat", "쪽빛 염료액")),
                    (
                        InitialState,
                        lexical("yellow_green", "황록색", ApprovedSemanticTypeIR::State),
                    ),
                    (
                        ResultState,
                        lexical("deep_blue", "진청색", ApprovedSemanticTypeIR::State),
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "SEALED_12",
            family: "music_box_tuning_completion",
            context: context(StatusUpdate, Team, "자동연주 상자 조율"),
            response: event_response(
                "music_box_tuning",
                "자동연주 상자 조율",
                StateChange,
                Complete,
                Completed,
                vec![
                    (Agent, person("instrument_restorer", "악기 복원사")),
                    (
                        Theme,
                        concept("music_box_tuning_work", "자동연주 상자의 조율"),
                    ),
                    (Location, place("sound_lab", "음향 측정실")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "CONFIRM_01",
            family: "pressed_plant_accession_tag_attachment",
            context: context(HandoffNote, Team, "압착 식물표본 등록표 부착"),
            response: event_response(
                "plant_accession_tag",
                "압착 식물표본 등록표 부착",
                StateChange,
                Attach,
                Completed,
                vec![
                    (Agent, person("herbarium_curator", "식물표본 관리사")),
                    (Theme, concept("accession_tag", "소장 등록표")),
                    (Target, concept("pressed_fern", "압착 고사리 표본")),
                    (Instrument, concept("archival_adhesive", "무산성 접착제")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "CONFIRM_02",
            family: "ceramic_crack_resin_application",
            context: context(StatusUpdate, Team, "도자기 균열 보호 수지 도포"),
            response: event_response(
                "ceramic_resin",
                "도자기 균열 보호 수지 도포",
                ApprovedEventRealizationClassIR::Transfer,
                Apply,
                Completed,
                vec![
                    (Agent, person("ceramic_conservator", "도자기 보존가")),
                    (Theme, concept("protective_resin", "보호 수지")),
                    (Target, concept("crack_edge", "균열 가장자리")),
                    (Instrument, concept("detail_brush", "세필")),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        // Long-form calibration families deliberately use unrelated domains and
        // four or more approved semantic units. They probe discourse flow,
        // aggregation, reference stability, and format realization rather than
        // merely substituting new nouns into the short-form cases above.
        Scenario {
            id: "LONG_01",
            family: "coral_nursery_pump_failure_response",
            context: context(Email, Team, "산호 유생 수조 순환 장애"),
            response: action_event_and_procedure_document_response(
                vec![
                    claim(
                        "P_L01_CAUSE",
                        "coral_incident",
                        "산호 유생 수조 순환 장애",
                        Cause,
                        concept("intake_clog", "취수구의 해조류 막힘"),
                    ),
                    claim(
                        "P_L01_IMPACT",
                        "coral_incident",
                        "산호 유생 수조 순환 장애",
                        Impact,
                        concept("oxygen_drop", "용존산소 농도 저하"),
                    ),
                    claim(
                        "P_L01_OWNER",
                        "coral_incident",
                        "산호 유생 수조 순환 장애",
                        Owner,
                        person("aquaculture_team", "해양생물 관리팀"),
                    ),
                    claim(
                        "P_L01_DEADLINE",
                        "coral_incident",
                        "산호 유생 수조 순환 장애",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 18,
                            minute: 30,
                        },
                    ),
                ],
                vec![ApprovedActionProcedureIR {
                    action_subject: node(
                        "coral_incident",
                        "산호 유생 수조 순환 장애",
                        ApprovedSemanticTypeIR::Event,
                    ),
                    action_modality: ApprovedModalityIR::Directive,
                    steps: vec![
                        directive_procedure_step(
                            "coral_incident",
                            "산호 유생 수조 순환 장애",
                            "emergency_oxygen_activation",
                            "emergency_oxygen_activation_event",
                            "예비 산소 공급기 가동",
                            StateChange,
                            Activate,
                            vec![(Theme, concept("emergency_oxygen_supplier", "예비 산소 공급기"))],
                        ),
                        directive_procedure_step(
                            "coral_incident",
                            "산호 유생 수조 순환 장애",
                            "intake_algae_removal",
                            "intake_algae_removal_event",
                            "취수구 세척",
                            StateChange,
                            Remove,
                            vec![(Theme, concept("intake_clog", "취수구의 해조류 막힘"))],
                        ),
                    ],
                    dependencies: Vec::new(),
                }],
                Vec::new(),
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_02",
            family: "mountain_hut_water_shortage",
            context: context(StatusUpdate, Team, "고산 대피소 식수 부족"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L02_CAUSE",
                        "hut_water",
                        "고산 대피소 식수 부족",
                        Cause,
                        concept("frozen_pipe", "취수 배관의 결빙"),
                    ),
                    claim(
                        "P_L02_IMPACT",
                        "hut_water",
                        "고산 대피소 식수 부족",
                        Impact,
                        concept("rationing", "투숙객 식수 배급 제한"),
                    ),
                    claim(
                        "P_L02_OWNER",
                        "hut_water",
                        "고산 대피소 식수 부족",
                        Owner,
                        person("hut_keeper", "대피소 관리인"),
                    ),
                    claim(
                        "P_L02_DEADLINE",
                        "hut_water",
                        "고산 대피소 식수 부족",
                        Deadline,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 22,
                        },
                    ),
                ],
                vec![
                    action_frame("emergency_water_supply", "비상 비축수 사용", "emergency_water_supply_event", "비상 비축수 사용", StateChange, Supply, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("emergency_reserve_water", "비상 비축수")), (Target, concept("hut_drinking_water", "대피소 식수"))]),
                    action_frame("snow_water_heating", "제설수 끓이기", "snow_water_heating_event", "제설수 끓이기", StateChange, Heat, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("melted_snow_water", "제설수"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_03",
            family: "silent_film_screening_correction",
            context: context(CorrectionNotice, Team, "무성영화 복원본 상영회"),
            response: response(
                vec![
                    claim(
                        "P_L03_NAME",
                        "film_screening",
                        "무성영화 복원본 상영회",
                        Name,
                        ApprovedOpenValueIR::Text("달빛 아래의 항구 복원판".into()),
                    ),
                    claim(
                        "P_L03_DATE",
                        "film_screening",
                        "무성영화 복원본 상영회",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 10,
                            day: 3,
                        },
                    ),
                    claim(
                        "P_L03_TIME",
                        "film_screening",
                        "무성영화 복원본 상영회",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 19,
                            minute: 10,
                        },
                    ),
                    claim(
                        "P_L03_LOCATION",
                        "film_screening",
                        "무성영화 복원본 상영회",
                        Location,
                        place("cinema_two", "시네마테크 2관"),
                    ),
                    claim(
                        "P_L03_CAPACITY",
                        "film_screening",
                        "무성영화 복원본 상영회",
                        Capacity,
                        ApprovedOpenValueIR::Quantity {
                            amount: 84,
                            unit: node("seat_unit_l03", "석", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Correction,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_04",
            family: "rescued_hedgehog_foster_handoff",
            context: context(InstantMessage, Peer, "구조 고슴도치"),
            response: action_event_and_procedure_document_response(
                vec![
                    claim_for_subject(
                        "P_L04_STATUS",
                        "hedgehog",
                        "구조 고슴도치",
                        ApprovedSemanticTypeIR::Concept,
                        Status,
                        lexical(
                            "stable_appetite",
                            "식욕 안정",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                    claim_for_subject(
                        "P_L04_LOCATION",
                        "hedgehog",
                        "구조 고슴도치",
                        ApprovedSemanticTypeIR::Concept,
                        Location,
                        place("warm_cage", "보온 케이지"),
                    ),
                    claim_for_subject(
                        "P_L04_OWNER",
                        "hedgehog",
                        "구조 고슴도치",
                        ApprovedSemanticTypeIR::Concept,
                        Owner,
                        person("night_volunteer", "야간 봉사자"),
                    ),
                ],
                vec![ApprovedActionProcedureIR {
                    action_subject: node(
                        "hedgehog_handoff_plan",
                        "구조 고슴도치 인계 계획",
                        ApprovedSemanticTypeIR::Event,
                    ),
                    action_modality: ApprovedModalityIR::Directive,
                    steps: vec![
                        directive_procedure_step(
                            "hedgehog_handoff_plan",
                            "구조 고슴도치 인계 계획",
                            "wet_food_supply",
                            "wet_food_supply_event",
                            "습식 사료 급여",
                            StateChange,
                            Supply,
                            vec![
                                (Theme, concept("wet_food", "습식 사료")),
                                (Patient, concept("hedgehog_patient", "구조 고슴도치")),
                                (Time, ApprovedOpenValueIR::Clock { hour: 21, minute: 0 }),
                            ],
                        ),
                        directive_procedure_step(
                            "hedgehog_handoff_plan",
                            "구조 고슴도치 인계 계획",
                            "body_weight_record",
                            "body_weight_record_event",
                            "체중 기록",
                            Creation,
                            Record,
                            vec![(Theme, concept("hedgehog_body_weight", "구조 고슴도치 체중"))],
                        ),
                    ],
                    dependencies: Vec::new(),
                }],
                Vec::new(),
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_05",
            family: "herbarium_freezer_move_request",
            context: context(ApprovalRequest, Manager, "식물표본 냉동고 이전"),
            response: action_event_and_procedure_document_response(
                vec![
                    claim(
                        "P_L05_OWNER",
                        "freezer_move",
                        "식물표본 냉동고 이전",
                        Owner,
                        person("collection_team", "수장고 관리팀"),
                    ),
                    claim(
                        "P_L05_DEADLINE",
                        "freezer_move",
                        "식물표본 냉동고 이전",
                        Deadline,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 25,
                        },
                    ),
                ],
                vec![ApprovedActionProcedureIR {
                    action_subject: node(
                        "freezer_move",
                        "식물표본 냉동고 이전",
                        ApprovedSemanticTypeIR::Event,
                    ),
                    action_modality: ApprovedModalityIR::Directive,
                    steps: vec![
                        directive_procedure_step(
                            "freezer_move",
                            "식물표본 냉동고 이전",
                            "freezer_frost_removal",
                            "freezer_frost_removal_event",
                            "성에 제거",
                            StateChange,
                            Remove,
                            vec![(Theme, concept("freezer_frost", "냉동고 성에"))],
                        ),
                        directive_procedure_step(
                            "freezer_move",
                            "식물표본 냉동고 이전",
                            "herbarium_freezer_relocation",
                            "herbarium_freezer_relocation_event",
                            "식물표본 냉동고 이동",
                            Motion,
                            Move,
                            vec![
                                (Theme, concept("herbarium_freezer", "식물표본 냉동고")),
                                (Destination, place("quarantine_room", "표본 검역실")),
                            ],
                        ),
                    ],
                    dependencies: vec![ApprovedActionProcedureDependencyIR {
                        predecessor_event_node_id: "freezer_frost_removal_event".into(),
                        successor_event_node_id: "herbarium_freezer_relocation_event".into(),
                    }],
                }],
                Vec::new(),
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_06",
            family: "planetarium_projector_calibration_commitment",
            context: context(HandoffNote, Team, "천체투영기 보정"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L06_LOCATION",
                        "projector_calibration",
                        "천체투영기 보정",
                        Location,
                        place("main_dome", "주 돔 상영관"),
                    ),
                    claim(
                        "P_L06_OWNER",
                        "projector_calibration",
                        "천체투영기 보정",
                        Owner,
                        person("optics_engineer", "광학 담당자"),
                    ),
                    claim(
                        "P_L06_DEADLINE",
                        "projector_calibration",
                        "천체투영기 보정",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 15,
                            minute: 40,
                        },
                    ),
                ],
                vec![
                    action_frame_for_subject("P_L06_AXIS_ACTION", "projector_calibration", "천체투영기 보정", "projector_axis_alignment_event", "광축 정렬", StateChange, Align, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("projector_optical_axis", "천체투영기 광축"))]),
                    action_frame_for_subject("P_L06_COLOR_ACTION", "projector_calibration", "천체투영기 보정", "projector_color_calibration_event", "색온도 보정", StateChange, Calibrate, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("projector_color_temperature", "천체투영기 색온도"))]),
                ],
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_07",
            family: "community_kiln_order_reassurance",
            context: context(Email, Peer, "공동 가마 소성 일정"),
            response: response(
                vec![
                    claim(
                        "P_L07_CONFIRMED",
                        "kiln_schedule",
                        "공동 가마 소성 일정",
                        Confirmed,
                        ApprovedOpenValueIR::Boolean(true),
                    ),
                    claim(
                        "P_L07_DATE",
                        "kiln_schedule",
                        "공동 가마 소성 일정",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 27,
                        },
                    ),
                    claim(
                        "P_L07_TIME",
                        "kiln_schedule",
                        "공동 가마 소성 일정",
                        Time,
                        ApprovedOpenValueIR::Clock { hour: 8, minute: 0 },
                    ),
                    claim(
                        "P_L07_OWNER",
                        "kiln_schedule",
                        "공동 가마 소성 일정",
                        Owner,
                        person("kiln_master", "가마 담당자"),
                    ),
                ],
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_08",
            family: "migratory_bird_survey_comparison",
            context: context(ExecutiveBrief, Executive, "철새 도래지 조사"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L08_COUNT_A",
                        "north_mudflat",
                        "북측 갯벌 관측 개체",
                        Count,
                        ApprovedOpenValueIR::Quantity {
                            amount: 126,
                            unit: node("bird_unit_a", "마리", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                    claim(
                        "P_L08_COUNT_B",
                        "reed_marsh",
                        "갈대 습지 관측 개체",
                        Count,
                        ApprovedOpenValueIR::Quantity {
                            amount: 84,
                            unit: node("bird_unit_b", "마리", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                    claim(
                        "P_L08_COUNT_C",
                        "estuary_islet",
                        "하구 모래섬 관측 개체",
                        Count,
                        ApprovedOpenValueIR::Quantity {
                            amount: 57,
                            unit: node("bird_unit_c", "마리", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                ],
                vec![action_frame_for_subject("P_L08_ACTION", "bird_survey", "철새 도래지 조사", "bird_survey_reobservation_event", "철새 도래지 재관측", Inspection, Observe, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("migratory_bird_arrival_site", "철새 도래지")), (Time, ApprovedOpenValueIR::Text("다음 간조 시간대".into()))])],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Comparison,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_09",
            family: "orchid_pollination_observation_sequence",
            context: context(MeetingSummary, Team, "야생 난초 수분 관찰"),
            response: response(
                vec![
                    claim(
                        "P_L09_DATE",
                        "orchid_pollination",
                        "야생 난초 수분 관찰",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 19,
                        },
                    ),
                    claim(
                        "P_L09_TIME",
                        "orchid_pollination",
                        "야생 난초 수분 관찰",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 6,
                            minute: 35,
                        },
                    ),
                    claim(
                        "P_L09_LOCATION",
                        "orchid_pollination",
                        "야생 난초 수분 관찰",
                        Location,
                        place("east_slope", "동쪽 사면 군락지"),
                    ),
                    claim(
                        "P_L09_STATUS",
                        "orchid_pollination",
                        "야생 난초 수분 관찰",
                        Status,
                        lexical(
                            "pollinia_transfer",
                            "화분괴 이동 확인",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                    claim(
                        "P_L09_OWNER",
                        "orchid_pollination",
                        "야생 난초 수분 관찰",
                        Owner,
                        person("field_ecologist", "현장 생태 조사원"),
                    ),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Temporal,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_10",
            family: "floating_library_humidity_incident",
            context: context(IncidentAlert, Team, "수상 도서관 서고 습도 경보"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L10_CAUSE",
                        "library_humidity",
                        "수상 도서관 서고 습도 경보",
                        Cause,
                        concept("hatch_seal", "선미 점검구 패킹 이탈"),
                    ),
                    claim(
                        "P_L10_IMPACT",
                        "library_humidity",
                        "수상 도서관 서고 습도 경보",
                        Impact,
                        concept("map_cabinet_condensation", "고지도 서랍장 표면의 결로"),
                    ),
                    claim(
                        "P_L10_OWNER",
                        "library_humidity",
                        "수상 도서관 서고 습도 경보",
                        Owner,
                        person("preservation_officer", "자료 보존 담당자"),
                    ),
                    claim(
                        "P_L10_DEADLINE",
                        "library_humidity",
                        "수상 도서관 서고 습도 경보",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 23,
                            minute: 0,
                        },
                    ),
                ],
                vec![
                    action_frame_for_subject("P_L10_SEAL_ACTION", "library_humidity", "수상 도서관 서고 습도 경보", "inspection_port_sealing_event", "점검구 임시 밀폐", StateChange, Seal, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("library_inspection_port", "서고 점검구"))]),
                    action_frame_for_subject("P_L10_DEHUMIDIFY_ACTION", "library_humidity", "수상 도서관 서고 습도 경보", "mobile_dehumidifier_activation_event", "이동식 제습기 가동", StateChange, Activate, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("mobile_dehumidifier", "이동식 제습기"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_11",
            family: "underwater_tablet_archive_transfer",
            context: context(StatusUpdate, Team, "수중 점토판 보존 이송"),
            response: event_response(
                "tablet_transfer",
                "수중 점토판 보존 이송",
                Motion,
                Move,
                Completed,
                vec![
                    (Agent, person("underwater_archaeologist", "수중고고학자")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 20,
                        },
                    ),
                    (
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 11,
                            minute: 25,
                        },
                    ),
                    (Location, place("salinity_lab", "염분 안정화실")),
                    (Source, place("freshwater_tank", "담수 치환 수조")),
                    (Theme, concept("clay_tablet_cradle", "점토판 지지대")),
                    (Destination, place("drying_chamber", "저온 건조실")),
                    (Instrument, concept("vibration_cart", "방진 운반대")),
                    (
                        Manner,
                        ApprovedOpenValueIR::Text("표면 수막을 유지하면서".into()),
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "LONG_12",
            family: "moss_chamber_cycle_change",
            context: context(StatusUpdate, Team, "고산 이끼 생육실 주기 변경"),
            response: event_response(
                "moss_cycle_change",
                "고산 이끼 생육실 주기 변경",
                StateChange,
                Change,
                Completed,
                vec![
                    (Agent, person("bryophyte_researcher", "선태식물 연구원")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 20,
                        },
                    ),
                    (Location, place("growth_chamber_three", "제3생육실")),
                    (Theme, concept("light_cycle", "명암 주기")),
                    (
                        InitialState,
                        ApprovedOpenValueIR::Text("14시간 밝음·10시간 어두움".into()),
                    ),
                    (
                        ResultState,
                        ApprovedOpenValueIR::Text("12시간 밝음·12시간 어두움".into()),
                    ),
                    (Instrument, concept("environment_controller", "환경 제어기")),
                    (
                        Manner,
                        ApprovedOpenValueIR::Text("온도 설정은 유지한 채".into()),
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "LONG_13",
            family: "lunar_sample_viewing_query",
            context: context(Email, Peer, "월면 시료 공개 관람"),
            response: response(
                vec![
                    claim(
                        "P_L13_STATUS",
                        "lunar_viewing",
                        "월면 시료 공개 관람",
                        Status,
                        lexical(
                            "reservation_open",
                            "예약 가능",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                    claim(
                        "P_L13_DATE",
                        "lunar_viewing",
                        "월면 시료 공개 관람",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 10,
                            day: 11,
                        },
                    ),
                    claim(
                        "P_L13_TIME",
                        "lunar_viewing",
                        "월면 시료 공개 관람",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 13,
                            minute: 20,
                        },
                    ),
                    claim(
                        "P_L13_LOCATION",
                        "lunar_viewing",
                        "월면 시료 공개 관람",
                        Location,
                        place("clean_gallery", "청정 전시실"),
                    ),
                    claim(
                        "P_L13_CAPACITY",
                        "lunar_viewing",
                        "월면 시료 공개 관람",
                        Capacity,
                        ApprovedOpenValueIR::Quantity {
                            amount: 12,
                            unit: node("person_unit_l13", "명", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                ],
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_14",
            family: "paper_theatre_prop_request",
            context: context(InstantMessage, Team, "종이극 소품 점검"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L14_LOCATION",
                        "paper_prop_check",
                        "종이극 소품 점검",
                        Location,
                        place("prop_room", "소품 준비실"),
                    ),
                    claim(
                        "P_L14_OWNER",
                        "paper_prop_check",
                        "종이극 소품 점검",
                        Owner,
                        person("stage_assistant", "무대 보조자"),
                    ),
                    claim(
                        "P_L14_DEADLINE",
                        "paper_prop_check",
                        "종이극 소품 점검",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 17,
                            minute: 10,
                        },
                    ),
                ],
                vec![
                    action_frame_for_subject("P_L14_PRESS_ACTION", "paper_prop_check", "종이극 소품 점검", "backdrop_pressing_event", "휘어진 배경판 압착", StateChange, Press, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("warped_backdrop", "휘어진 배경판"))]),
                    action_frame_for_subject("P_L14_INSPECT_ACTION", "paper_prop_check", "종이극 소품 점검", "scene_number_inspection_event", "장면 번호 확인", Inspection, Inspect, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("scene_numbers", "장면 번호"))]),
                ],
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                LanguageRegisterIR::Informal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_15",
            family: "salt_marsh_boardwalk_recovery",
            context: context(ExecutiveBrief, Executive, "염습지 탐방로 복구"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_L15_CAUSE",
                        "boardwalk_recovery",
                        "염습지 탐방로 폐쇄",
                        Cause,
                        concept("pile_scour", "만조 때 발생한 기초 말뚝 세굴"),
                    ),
                    claim(
                        "P_L15_IMPACT",
                        "boardwalk_recovery",
                        "염습지 탐방로 폐쇄",
                        Impact,
                        concept("east_route_closed", "동측 관찰 구간 통행 제한"),
                    ),
                    claim(
                        "P_L15_OWNER",
                        "boardwalk_recovery",
                        "염습지 탐방로 복구",
                        Owner,
                        person("wetland_team", "습지 보전팀"),
                    ),
                    claim(
                        "P_L15_DEADLINE",
                        "boardwalk_recovery",
                        "염습지 탐방로 복구",
                        Deadline,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 28,
                        },
                    ),
                    claim(
                        "P_L15_STATUS",
                        "boardwalk_recovery",
                        "염습지 탐방로 복구",
                        Status,
                        lexical(
                            "materials_secured",
                            "교체 자재 확보",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                ],
                vec![
                    action_frame_for_subject("P_L15_SUPPORT_ACTION", "boardwalk_recovery", "염습지 탐방로 복구", "temporary_support_installation_event", "임시 지지대 설치", StateChange, Install, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("temporary_support", "임시 지지대")), (Target, concept("damaged_boardwalk", "손상 탐방로"))]),
                    action_frame_for_subject("P_L15_REPLACE_ACTION", "boardwalk_recovery", "염습지 탐방로 복구", "damaged_deck_replacement_event", "손상 데크 교체", StateChange, Replace, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("damaged_deck", "손상 데크"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_CONFIRM_01",
            family: "glacier_core_archive_transfer_holdout",
            context: context(StatusUpdate, Team, "빙하 코어 표본 이송"),
            response: event_response(
                "glacier_core_transfer",
                "빙하 코어 표본 이송",
                Motion,
                Move,
                Completed,
                vec![
                    (
                        Destination,
                        place("minus_twenty_archive", "영하 20도 보존고"),
                    ),
                    (Theme, concept("glacier_core_box", "빙하 코어 표본 상자")),
                    (Agent, person("polar_archive_curator", "극지 시료 관리원")),
                    (Source, place("cold_loading_bay", "저온 하역실")),
                    (
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 14,
                            minute: 45,
                        },
                    ),
                    (Instrument, concept("insulated_cart", "단열 운반 카트")),
                    (Location, place("polar_sample_center", "극지 시료 센터")),
                    (
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 21,
                        },
                    ),
                    (
                        Manner,
                        ApprovedOpenValueIR::Text("표본 방향을 유지한 채".into()),
                    ),
                ],
                LanguageRegisterIR::Formal,
            ),
        },
        Scenario {
            id: "LONG_CONFIRM_02",
            family: "cave_laser_scanner_incident_holdout",
            context: context(IncidentAlert, Team, "석회동굴 레이저 스캐너 중단"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_LC02_CAUSE",
                        "cave_scanner_stop",
                        "석회동굴 레이저 스캐너 중단",
                        Cause,
                        concept("lens_condensation", "렌즈 보호창 내부의 결로"),
                    ),
                    claim(
                        "P_LC02_IMPACT",
                        "cave_scanner_stop",
                        "석회동굴 레이저 스캐너 중단",
                        Impact,
                        concept("point_cloud_gap", "서쪽 통로 점군 데이터 누락"),
                    ),
                    claim(
                        "P_LC02_OWNER",
                        "cave_scanner_stop",
                        "석회동굴 레이저 스캐너 중단",
                        Owner,
                        person("survey_engineer", "공간정보 측량기사"),
                    ),
                    claim(
                        "P_LC02_DEADLINE",
                        "cave_scanner_stop",
                        "석회동굴 레이저 스캐너 중단",
                        Deadline,
                        ApprovedOpenValueIR::Clock {
                            hour: 20,
                            minute: 30,
                        },
                    ),
                ],
                vec![
                    action_frame_for_subject("P_LC02_DRY_ACTION", "cave_scanner_stop", "석회동굴 레이저 스캐너 중단", "scanner_drying_event", "장비 건조", StateChange, Dry, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("laser_scanner", "레이저 스캐너"))]),
                    action_frame_for_subject("P_LC02_SURVEY_ACTION", "cave_scanner_stop", "석회동굴 레이저 스캐너 중단", "west_passage_resurvey_event", "서쪽 통로 재측량", Inspection, Observe, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, place("west_passage", "서쪽 통로"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "LONG_CONFIRM_03",
            family: "mobile_darkroom_workshop_query_holdout",
            context: context(Email, ExternalPartner, "이동식 암실 인화 워크숍"),
            response: response(
                vec![
                    claim(
                        "P_LC03_STATUS",
                        "darkroom_workshop",
                        "이동식 암실 인화 워크숍",
                        Status,
                        lexical(
                            "application_open",
                            "신청 가능",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                    claim(
                        "P_LC03_DATE",
                        "darkroom_workshop",
                        "이동식 암실 인화 워크숍",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 10,
                            day: 17,
                        },
                    ),
                    claim(
                        "P_LC03_TIME",
                        "darkroom_workshop",
                        "이동식 암실 인화 워크숍",
                        Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 10,
                            minute: 15,
                        },
                    ),
                    claim(
                        "P_LC03_LOCATION",
                        "darkroom_workshop",
                        "이동식 암실 인화 워크숍",
                        Location,
                        place("photo_bus", "사진 아카이브 버스"),
                    ),
                    claim(
                        "P_LC03_CAPACITY",
                        "darkroom_workshop",
                        "이동식 암실 인화 워크숍",
                        Capacity,
                        ApprovedOpenValueIR::Quantity {
                            amount: 9,
                            unit: node("person_unit_lc03", "명", ApprovedSemanticTypeIR::Unit),
                        },
                    ),
                ],
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_CONFIRM_04",
            family: "rare_seed_drying_reassurance_holdout",
            context: context(Email, Peer, "희귀종 종자 건조 일정"),
            response: response(
                vec![
                    claim(
                        "P_LC04_CONFIRMED",
                        "seed_drying_schedule",
                        "희귀종 종자 건조 일정",
                        Confirmed,
                        ApprovedOpenValueIR::Boolean(true),
                    ),
                    claim(
                        "P_LC04_DATE",
                        "seed_drying_schedule",
                        "희귀종 종자 건조 일정",
                        Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 24,
                        },
                    ),
                    claim(
                        "P_LC04_LOCATION",
                        "seed_drying_schedule",
                        "희귀종 종자 건조 일정",
                        Location,
                        place("low_humidity_room", "저습도 건조실"),
                    ),
                    claim(
                        "P_LC04_OWNER",
                        "seed_drying_schedule",
                        "희귀종 종자 건조 일정",
                        Owner,
                        person("seed_bank_curator", "종자은행 관리원"),
                    ),
                ],
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                LanguageRegisterIR::Neutral,
                ApprovedVerbosityIR::Explanatory,
                Vec::new(),
            ),
        },
        Scenario {
            id: "LONG_CONFIRM_05",
            family: "tidal_clock_restoration_brief_holdout",
            context: context(ExecutiveBrief, Executive, "조석 시계 복원"),
            response: action_event_document_response(
                vec![
                    claim(
                        "P_LC05_CAUSE",
                        "tidal_clock_restoration",
                        "조석 시계 오차",
                        Cause,
                        concept("float_axis_salt", "부자 회전축의 염 결정 고착"),
                    ),
                    claim(
                        "P_LC05_IMPACT",
                        "tidal_clock_restoration",
                        "조석 시계 오차",
                        Impact,
                        concept("display_delay", "만조 표시 18분 지연"),
                    ),
                    claim(
                        "P_LC05_OWNER",
                        "tidal_clock_restoration",
                        "조석 시계 복원",
                        Owner,
                        person("maritime_clockmaker", "해양 계측 복원사"),
                    ),
                    claim(
                        "P_LC05_DEADLINE",
                        "tidal_clock_restoration",
                        "조석 시계 복원",
                        Deadline,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 9,
                            day: 30,
                        },
                    ),
                    claim(
                        "P_LC05_STATUS",
                        "tidal_clock_restoration",
                        "조석 시계 복원",
                        Status,
                        lexical(
                            "replacement_float_secured",
                            "교체 부자 확보",
                            ApprovedSemanticTypeIR::State,
                        ),
                    ),
                ],
                vec![
                    action_frame_for_subject("P_LC05_CLEAN_ACTION", "tidal_clock_restoration", "조석 시계 복원", "clock_axis_cleaning_event", "회전축 세척", StateChange, Clean, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("tidal_clock_rotation_axis", "조석 시계 회전축"))]),
                    action_frame_for_subject("P_LC05_CALIBRATE_ACTION", "tidal_clock_restoration", "조석 시계 복원", "clock_scale_recalibration_event", "눈금판 재보정", StateChange, Calibrate, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("tidal_clock_scale_plate", "조석 시계 눈금판"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
    ]);

    let long_claims = |prefix: &str,
                       specs: &[(&str, ApprovedRelationTypeIR, &str)]|
     -> Vec<ApprovedCompositionalClaimIR> {
        specs
            .iter()
            .enumerate()
            .map(|(index, (subject, relation, value))| {
                let value_type = match relation {
                    Owner => ApprovedSemanticTypeIR::Person,
                    Location | Source | Destination => ApprovedSemanticTypeIR::Location,
                    Status => ApprovedSemanticTypeIR::State,
                    _ => ApprovedSemanticTypeIR::Concept,
                };
                claim(
                    &format!("P_{prefix}_{index:02}"),
                    &format!("{prefix}_{index:02}"),
                    subject,
                    *relation,
                    lexical(&format!("{prefix}_value_{index:02}"), value, value_type),
                )
            })
            .collect()
    };

    // Fixtures may carry source-approved state frames when a bare `Status`
    // edge would otherwise erase the predicate structure.  This attaches the
    // frame by canonical node identity before realization; it never inspects
    // or rewrites Korean surface text.
    let annotate_status_frames = |
        claims: &mut [ApprovedCompositionalClaimIR],
        frames: &[(&str, ApprovedStatusFrameIR)],
    | {
        for (subject_node_id, frame) in frames {
            let matching = claims
                .iter()
                .filter(|claim| claim.subject.node_id == *subject_node_id)
                .count();
            assert_eq!(
                matching,
                1,
                "each source status frame must select exactly one canonical claim"
            );
            let claim = claims
                .iter_mut()
                .find(|claim| claim.subject.node_id == *subject_node_id)
                .expect("checked source status frame claim");
            assert_eq!(claim.relation, Status);
            assert!(claim.status_frame.is_none());
            claim.status_frame = Some(frame.clone());
        }
    };

    rows.extend([
        Scenario {
            id: "DOCUMENT_LONG_01",
            family: "floating_seed_vault_storm_recovery_document",
            context: context(ExecutiveBrief, Executive, "해상 종자보관소 폭풍 피해 복구"),
            response: action_event_document_response(
                {
                    let mut claims = long_claims(
                        "DL01",
                        &[
                            ("해상 종자보관소 폭풍 피해 복구", Cause, "방파문을 넘은 월파와 갑판 배수구 막힘"),
                            ("외벽 누수", Impact, "서측 저장동 단열재 젖음"),
                            ("비상 전원 전환", Status, "완료됨"),
                            ("저온 저장고 1구역", Status, "온도 안정"),
                            ("저온 저장고 2구역", Status, "온도 회복 중"),
                            ("염수 유입 구간", Location, "서측 서비스 통로"),
                            ("1차 점검 결과", Status, "직접 침수 없음"),
                            ("습도 상승 영향", Impact, "포장재 표면 결로 가능성"),
                            ("환기 설비 이상", Cause, "흡기 필터의 염분 포화"),
                            ("전기 패널 점검", Status, "절연 저항 확인"),
                            ("예비 발전기 연료", Status, "72시간 운전분 확보"),
                            ("부두 접근 제한", Impact, "외부 정비 인력 승선 지연"),
                            ("재가동 조건", Status, "저장고 온습도 12시간 안정"),
                            ("종자 출고 제한", Status, "복구 검증 전 일시 중지"),
                            ("다음 상황 보고", Deadline, "2026년 9월 22일 오전 9시"),
                        ],
                    );
                    claims.extend([
                        claim("P_DL01_OWNER_INITIAL", "initial_waterproofing_owner", "초기 차수 담당", Owner, person("facility_response_team", "시설 대응반")),
                        claim("P_DL01_OWNER_ISOLATION", "seed_isolation_owner", "격리 작업 담당", Owner, person("seed_conservation_team", "종자 보존팀")),
                        claim("P_DL01_OWNER_RECORD", "insurance_record_owner", "보험 기록 담당", Owner, person("operations_support", "운영지원 담당자")),
                        claim("P_DL01_OWNER_STRUCTURE", "structure_inspection_owner", "구조 점검 담당", Owner, person("naval_structure_engineer", "선박 구조 기술사")),
                        claim("P_DL01_OWNER_RECOVERY", "recovery_lead_owner", "복구 총괄", Owner, person("marine_preservation_lead", "해상 보존시설 책임자")),
                    ]);
                    claims
                },
                vec![
                    action_frame("initial_waterproofing", "초기 차수 조치", "tarp_installation", "방수포 설치", StateChange, Attach, Completed, ApprovedEventVoiceIR::Active, vec![(Agent, person("facility_response_team", "시설 대응반")), (Theme, concept("waterproof_tarp", "방수포")), (Target, place("saltwater_ingress_zone", "염수 유입 구간"))]),
                    action_frame("drainage_pump_activation", "배수 펌프 가동", "drainage_pump_start", "배수 펌프 가동", StateChange, Activate, Completed, ApprovedEventVoiceIR::Active, vec![(Agent, person("facility_response_team", "시설 대응반")), (Theme, concept("drainage_pump", "배수 펌프"))]),
                    action_frame("seed_box_inspection", "종자 상자 1차 점검", "seed_box_surface_inspection", "종자 상자 점검", Inspection, Inspect, Completed, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("seed_boxes", "종자 상자"))]),
                    action_frame("humidity_box_isolation", "고습도 상자 격리", "humidity_box_move", "고습도 상자 이동", Motion, Move, Ongoing, ApprovedEventVoiceIR::Active, vec![(Agent, person("seed_conservation_team", "종자 보존팀")), (Theme, concept("high_humidity_boxes", "고습도 상자")), (Destination, place("drying_anteroom", "건조 전실"))]),
                    action_frame("insurance_record", "보험 현장 기록", "damage_log_creation", "손상 구역 기록 작성", Creation, Create, Ongoing, ApprovedEventVoiceIR::Active, vec![(Agent, person("operations_support", "운영지원 담당자")), (Theme, concept("damage_photo_and_measurement_log", "손상 구역 사진과 계측 로그"))]),
                    action_frame("structure_inspection", "구조 안전 점검", "hull_mooring_inspection", "구조 안전 점검", Inspection, Inspect, Planned, ApprovedEventVoiceIR::Active, vec![(Agent, person("naval_structure_engineer", "선박 구조 기술사")), (Theme, concept("west_hull_and_moorings", "서측 외판과 계류 장치"))]),
                    action_frame("stakeholder_notification", "이해관계자 통지", "preservation_status_notice", "보존 상태 통지", ApprovedEventRealizationClassIR::Transfer, Send, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("delay_reason_and_preservation_status", "지연 사유와 보존 상태")), (Destination, lexical("depositing_institutions", "기탁 기관", ApprovedSemanticTypeIR::Organization))]),
                    action_frame("root_cause_review", "근본 원인 조사", "drainage_storm_procedure_review", "근본 원인 조사", Inspection, Inspect, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("drainage_design_and_storm_procedure", "배수 설계와 폭풍 운용 절차"))]),
                    action_frame("recurrence_prevention", "재발 방지안", "drain_warning_installation", "재발 방지 설비 설치", StateChange, Attach, Planned, ApprovedEventVoiceIR::Active, vec![(Agent, person("marine_preservation_lead", "해상 보존시설 책임자")), (Theme, concept("double_drain_strainer_and_water_alarm", "이중 거름망과 수위 경보")), (Target, place("drainage_outlets", "배수구"))]),
                    action_frame("intake_filter_replacement", "필터 교체", "intake_filter_replacement_event", "흡기 필터 교체", StateChange, Replace, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("salt_saturated_intake_filter", "염분 포화 흡기 필터"))]),
                    action_frame("duct_interior_drying", "덕트 내부 건조", "duct_interior_drying_event", "덕트 내부 건조", StateChange, Dry, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("ventilation_duct_interior", "환기 덕트 내부"))]),
                    action_frame("small_workboat_deployment", "소형 작업선 투입", "small_workboat_deployment_event", "소형 작업선 투입", Motion, Deploy, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("small_workboat", "소형 작업선"))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Cause,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },        Scenario {
            id: "DOCUMENT_LONG_02",
            family: "historic_station_conversion_meeting_document",
            context: context(MeetingSummary, Team, "폐선 철도역 복합문화공간 전환 회의"),
            response: action_event_document_response(
                long_claims(
                    "DL02",
                    &[
                        ("폐선 철도역 복합문화공간 전환 회의", Status, "기본 설계 검토 완료"),
                        ("역사 외벽", Status, "원형 벽돌 보존"),
                        ("승강장 캐노피", Status, "구조 보강 필요"),
                        ("캐노피 정밀진단", Owner, "구조안전 자문단"),
                        ("철도 신호기", Status, "현 위치 보존"),
                        ("소방 피난 동선", Impact, "공연장 수용 인원 상한 조정"),
                        ("무장애 접근로", Location, "남측 완만한 경사 구간"),
                        ("승강기 시야 영향", Impact, "원형 입면 가림 최소화"),
                        ("카페 배기 설비", Location, "기존 굴뚝 내부"),
                        ("카페 조리 범위", Status, "간단한 가열 음식으로 제한"),
                        ("상설 전시 자료 출처", Source, "지역 철도 노동자 구술 기록"),
                        ("기증 자료 정리", Owner, "지역사 연구모임"),
                        ("야간 소음 기준", Status, "주거지 경계 측정값 준수"),
                        ("야외 공연 종료", Deadline, "밤 9시 이전"),
                        ("주차 수요 영향", Impact, "주말 인근 골목 혼잡 가능성"),
                        ("임시 주차 협의", Owner, "교통행정과"),
                        ("문화재 자문", Status, "다음 설계안 제출 전 필요"),
                        ("주민 설명회", Deadline, "10월 둘째 목요일"),
                        ("수정 설계안", Owner, "공공건축 설계팀"),
                        ("다음 회의 안건", Status, "운영 주체와 대관 기준 확정"),
                    ],
                ),
                vec![
                    action_frame("ticket_office_conversion", "옛 매표소 전환", "ticket_office_conversion_event", "옛 매표소 전환", StateChange, Convert, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("old_ticket_office", "옛 매표소")), (ResultState, concept("regional_records_room", "지역 기록 열람실"))]),
                    action_frame("baggage_hall_conversion", "수하물 창고 전환", "baggage_hall_conversion_event", "수하물 창고 전환", StateChange, Convert, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("baggage_store", "수하물 창고")), (ResultState, concept("small_performance_hall", "소규모 공연장"))]),
                    action_frame("north_track_conversion", "북측 선로 구간 조성", "north_track_conversion_event", "북측 선로 구간 조성", StateChange, Convert, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("north_track_section", "북측 선로 구간")), (ResultState, concept("trail_and_exhibition_route", "야외 산책로와 전시 동선"))]),
                    action_frame("east_emergency_exit", "공연장 출입구 보강", "east_exit_installation_event", "동측 비상문 설치", StateChange, Install, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("east_emergency_exit_door", "동측 비상문")), (Target, place("performance_hall_entrance", "공연장 출입구"))]),
                    action_frame("elevator_construction", "승강기 설치", "independent_elevator_construction_event", "독립 승강기 구조 시공", Creation, Build, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("independent_elevator_structure", "독립 승강기 구조")), (Location, place("stairwell_rear", "기존 계단실 후면"))]),
                    action_frame("resident_exhibit_operation", "주민 전시 공간 운영", "resident_exhibit_operation_event", "주민 전시 공간 운영", StateChange, Operate, Scheduled, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("resident_exhibit_space", "주민 전시 공간"))]),
                    action_frame("window_frame_reinforcement", "목재 프레임 보강", "window_frame_reinforcement_event", "낡은 대합실 창호 목재 프레임 보강", StateChange, Reinforce, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("old_waiting_room_window_frame", "낡은 대합실 창호 목재 프레임"))]),
                    action_frame("shuttle_stop_connection", "셔틀 정류장 연계", "shuttle_stop_connection_event", "역전 광장 셔틀 정류장 연계", StateChange, Connect, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("public_transport_guidance", "대중교통 안내")), (Target, place("station_square_shuttle_stop", "역전 광장 셔틀 정류장"))]),
                ],
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Temporal,
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "DOCUMENT_LONG_03",
            family: "urban_apiary_summer_operations_guide",
            context: context(Email, ExternalPartner, "도심 양봉장 여름철 운영 안내"),
            response: action_event_and_procedure_document_response(
                {
                    let mut claims = long_claims(
                    "DL03",
                    &[
                        (
                            "도심 양봉장 여름철 운영 안내",
                            Cause,
                            "고온과 밀원 감소가 겹치는 계절 변화",
                        ),
                        (
                            "벌통 내부 과열",
                            Impact,
                            "유충 발달 저하와 일벌의 환기 부담 증가",
                        ),
                        ("물 교체 주기", Status, "매일 1회"),
                        ("벌통 환기구", Status, "말벌 침입을 막는 폭으로 개방"),
                        ("내검 시간", Status, "기온이 낮은 이른 오전"),
                        ("내검 지속시간", Status, "벌통당 10분 이내"),
                        ("설탕물 누출", Impact, "주변 봉군의 약탈 행동 유발"),
                        ("약탈 징후", Status, "출입구 몸싸움과 비정상 비행 증가"),
                        (
                            "방제 시행 조건",
                            Status,
                            "꿀 수확 일정과 약제 휴약기간 확인",
                        ),
                        ("말벌 포획기", Location, "봉군 비행을 방해하지 않는 외곽"),
                        ("살충제 살포 영향", Impact, "귀소 일벌의 대량 손실 가능성"),
                        ("보호 장비", Status, "면포와 장갑 여분 확보"),
                        ("쏘임 사고 기록", Owner, "현장 안전 담당자"),
                        ("폭염 중단 기준", Status, "작업자 열 스트레스 징후 발생"),
                        ("긴급 연락망", Status, "양봉 책임자와 건물 관리실 번호 확인"),
                    ],
                    );
                    for claim in &mut claims {
                        claim.status_frame = match claim.subject.canonical_lexical_label.as_str() {
                            "벌통 환기구" => Some(ApprovedStatusFrameIR::Configuration {
                                predicate: ApprovedConfigurationStateIR::Open,
                                modifier: Some(node(
                                    "hornet_entry_prevention_opening_width",
                                    "말벌 침입을 막는 폭",
                                    ApprovedSemanticTypeIR::Concept,
                                )),
                            }),
                            "약탈 징후" => Some(ApprovedStatusFrameIR::Observation {
                                evidence: vec![
                                    node(
                                        "entrance_fighting",
                                        "출입구 몸싸움",
                                        ApprovedSemanticTypeIR::Event,
                                    ),
                                    node(
                                        "abnormal_flight_increase",
                                        "비정상 비행 증가",
                                        ApprovedSemanticTypeIR::State,
                                    ),
                                ],
                            }),
                            "폭염 중단 기준" => Some(ApprovedStatusFrameIR::Threshold {
                                trigger: node(
                                    "worker_heat_stress_sign_onset",
                                    "작업자 열 스트레스 징후 발생",
                                    ApprovedSemanticTypeIR::Event,
                                ),
                            }),
                            _ => None,
                        };
                    }
                    claims
                },
                vec![
                    ApprovedActionProcedureIR {
                        action_subject: node("water_station_management", "급수대 관리", ApprovedSemanticTypeIR::Event),
                        action_modality: ApprovedModalityIR::Directive,
                        steps: vec![
                            directive_procedure_step("water_station_management", "급수대 관리", "floating_platform_install", "floating_platform_installation", "부유 발판 설치", StateChange, Install, vec![(Theme, concept("floating_platform", "부유 발판")), (Target, concept("shallow_vessel", "얕은 용기")), (Location, place("shaded_area_away_from_flight_path", "출입구 비행선과 떨어진 그늘"))]),
                            directive_procedure_step("water_station_management", "급수대 관리", "clean_water_supply", "clean_water_supply_event", "깨끗한 물 공급", StateChange, Supply, vec![(Theme, concept("clean_water", "깨끗한 물")), (Target, concept("shallow_vessel", "얕은 용기")), (Location, place("shaded_area_away_from_flight_path", "출입구 비행선과 떨어진 그늘"))]),
                        ],
                        dependencies: Vec::new(),
                    },
                    ApprovedActionProcedureIR {
                        action_subject: node("robbery_response", "약탈 대응", ApprovedSemanticTypeIR::Event),
                        action_modality: ApprovedModalityIR::Directive,
                        steps: vec![
                            directive_procedure_step("robbery_response", "약탈 대응", "entrance_reduction", "entrance_reduction", "출입구 축소", StateChange, Reduce, vec![(Theme, concept("hive_entrance", "출입구"))]),
                            directive_procedure_step("robbery_response", "약탈 대응", "exposed_food_removal", "exposed_food_removal", "노출 먹이 제거", StateChange, Remove, vec![(Theme, concept("exposed_food", "노출 먹이")), (Manner, ApprovedOpenValueIR::Text("즉시".into()))]),
                        ],
                        dependencies: Vec::new(),
                    },
                ],
                vec![
                    directive_procedure_step("shade_screen_installation", "차광막 설치", "install", "shade_screen_installation_event", "차광막 설치", StateChange, Install, vec![(Theme, concept("shade_screen", "차광막")), (Target, place("hive_top_clearance", "벌통 위 30센티미터 높이")), (Manner, ApprovedOpenValueIR::Text("오전 햇빛은 유지하고 오후 직사광선은 차단하도록".into()))]),
                    directive_action_frame("queen_brood_check", "여왕벌 산란 확인", "queen_brood_inspection", "여왕벌 산란 확인", Inspection, Inspect, vec![(Theme, concept("egg_and_young_larvae_distribution", "알과 어린 유충의 분포"))]),
                    directive_action_frame("colony_food_check", "봉군 먹이 상태", "colony_food_inspection", "봉군 먹이 상태", Inspection, Inspect, vec![(Theme, concept("sealed_honey_and_pollen_stores", "봉개꿀과 화분 저장량"))]),
                    directive_action_frame("mite_level_record", "응애 점검", "mite_level_recording", "응애 점검", Creation, Record, vec![(Theme, concept("mite_infestation_level", "응애 감염 수준")), (Instrument, concept("standard_sample_test", "표준 표본 검사"))]),
                    directive_action_frame("nearby_control_schedule_share", "인근 방제 일정", "nearby_control_schedule_sharing", "인근 방제 일정 공유", ApprovedEventRealizationClassIR::Transfer, Share, vec![(Patient, concept("management_authority", "관리 주체")), (Theme, concept("nearby_control_schedule", "인근 방제 일정")), (Manner, ApprovedOpenValueIR::Text("사전에".into()))]),
                    directive_action_frame("complaint_prevention_notice", "민원 예방 안내", "work_hours_notice_posting", "작업 시간 공지 게시", StateChange, Post, vec![(Theme, concept("work_hours_notice", "작업 시간 공지")), (Target, place("roof_door_and_ground_floor_board", "옥상 출입문과 1층 게시판"))]),
                    directive_action_frame("evening_small_food_supply", "저녁 소량 급여", "evening_small_food_supply_event", "저녁 소량 급여", StateChange, Supply, vec![(Theme, concept("small_food_portion", "소량 먹이")), (Patient, concept("food_shortage_colony", "먹이 부족 봉군")), (Time, ApprovedOpenValueIR::Text("저녁".into()))]),
                ],
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Explanation,
                // This is an external operational email.  The register is a
                // source-level interaction contract, not a surface repair.
                LanguageRegisterIR::Formal,
                ApprovedVerbosityIR::Explanatory,
            ),
        },
        Scenario {
            id: "DOCUMENT_LONG_CONFIRM_01",
            family: "mountain_observatory_winter_readiness_document",
            context: context(ExecutiveBrief, Executive, "고산 천문대 겨울 관측 준비"),
            response: multi_event_document_response(
                vec![
                    (
                        "coolant_move",
                        "냉각재 운반",
                        Motion,
                        Move,
                        Completed,
                        vec![
                            (Agent, person("logistics_team", "관측 지원팀")),
                            (Theme, concept("coolant_boxes", "냉각재 상자")),
                            (Source, place("receiving_room", "자재 인수실")),
                            (Instrument, concept("insulated_cart", "단열 운반 카트")),
                            (Destination, place("cold_store", "저온 자재고")),
                        ],
                    ),
                    (
                        "rail_lubrication",
                        "돔 레일 윤활",
                        ApprovedEventRealizationClassIR::Transfer,
                        Apply,
                        Completed,
                        vec![
                            (Agent, person("mechanical_team", "기계 설비팀")),
                            (Theme, concept("low_temp_lubricant", "저온용 윤활제")),
                            (Target, concept("dome_rail", "돔 회전 레일")),
                        ],
                    ),
                    (
                        "ice_sensor_attachment",
                        "결빙 센서 부착",
                        StateChange,
                        Attach,
                        Completed,
                        vec![
                            (Agent, person("safety_team", "안전 관리팀")),
                            (Theme, concept("ice_sensor", "결빙 감지 센서")),
                            (Target, place("north_stairs", "북측 외부 계단")),
                        ],
                    ),
                    (
                        "schedule_upload",
                        "겨울 관측표 업로드",
                        ApprovedEventRealizationClassIR::Transfer,
                        Upload,
                        Completed,
                        vec![
                            (Agent, person("observation_lead", "관측 책임자")),
                            (Theme, concept("winter_schedule", "겨울 관측 일정표")),
                            (Destination, place("operations_server", "운영 서버")),
                        ],
                    ),
                    (
                        "checklist_write",
                        "비상 정지 점검표 작성",
                        Creation,
                        Write,
                        Completed,
                        vec![
                            (Agent, person("control_engineer", "제어 엔지니어")),
                            (Theme, concept("shutdown_checklist", "비상 정지 점검표")),
                            (Location, place("control_room", "중앙 제어실")),
                        ],
                    ),
                    (
                        "generator_mode_change",
                        "예비 발전기 운전 모드 변경",
                        StateChange,
                        Change,
                        Completed,
                        vec![
                            (Agent, person("power_team", "전력 설비팀")),
                            (Theme, concept("generator_mode", "예비 발전기 운전 모드")),
                            (InitialState, concept("automatic_mode", "자동 대기")),
                            (ResultState, concept("manual_priority", "수동 우선")),
                        ],
                    ),
                    (
                        "trace_heater_activation",
                        "배관 열선 가동",
                        StateChange,
                        Activate,
                        Ongoing,
                        vec![
                            (Agent, person("facility_team", "시설 관리팀")),
                            (Theme, concept("trace_heater", "급수 배관 열선")),
                            (Location, place("machine_room", "기계실")),
                        ],
                    ),
                    (
                        "access_point_deactivation",
                        "시험용 접속 장치 비활성화",
                        StateChange,
                        Deactivate,
                        Completed,
                        vec![
                            (Agent, person("network_admin", "네트워크 관리자")),
                            (Theme, concept("test_access_point", "옥외 시험용 접속 장치")),
                            (Location, place("roof", "관측동 옥상")),
                        ],
                    ),
                    (
                        "gateway_reset",
                        "기상 센서 게이트웨이 초기화",
                        StateChange,
                        Reset,
                        Completed,
                        vec![
                            (Agent, person("systems_team", "시스템 운영팀")),
                            (Theme, concept("weather_gateway", "기상 센서 게이트웨이")),
                        ],
                    ),
                    (
                        "bedding_expansion",
                        "비상 침구 비축 확대",
                        StateChange,
                        Expand,
                        Planned,
                        vec![
                            (Agent, person("lodging_manager", "생활관 관리자")),
                            (Theme, concept("bedding_inventory", "비상 침구 비축량")),
                            (Target, concept("twenty_four_people", "24명분")),
                        ],
                    ),
                    (
                        "duty_room_open",
                        "겨울 당직실 개방",
                        StateChange,
                        Open,
                        Scheduled,
                        vec![
                            (Agent, person("operations_lead", "운영 책임자")),
                            (Theme, place("winter_duty_room", "겨울 당직실")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 11,
                                    day: 15,
                                },
                            ),
                        ],
                    ),
                    (
                        "deck_close",
                        "서측 관측 데크 폐쇄",
                        StateChange,
                        Close,
                        Scheduled,
                        vec![
                            (Agent, person("safety_manager", "안전 관리자")),
                            (Theme, place("west_deck", "서측 관측 데크")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 12,
                                    day: 1,
                                },
                            ),
                        ],
                    ),
                ],
                {
                    let mut claims = long_claims(
                        "DLCF",
                        &[
                        ("고산 천문대 겨울 관측 준비", Status, "진행 중"),
                        ("주 관측경 광학계", Status, "성에 제거 완료"),
                        ("돔 비상 구동 배터리", Status, "정격 용량 확보"),
                        ("제설 장비 보관 위치", Location, "동측 차고"),
                        ("야간 제설 담당", Owner, "시설 관리팀 당직조"),
                        ("산악 도로 통제 영향", Impact, "교대 인력 도착 지연 가능성"),
                        ("비축 식량", Status, "14일분 확보"),
                        ("위성 전화 시험", Status, "통화 품질 확인"),
                        ("의료 산소 용기", Status, "유효기간 점검 완료"),
                        ("겨울 관측 승인", Deadline, "첫 적설 전"),
                        ("다음 안전 점검", Deadline, "11월 셋째 주"),
                        ("기상 악화 보고 담당", Owner, "관측 당직 책임자"),
                        ("최종 준비 상태", Status, "조건부 운영 가능"),
                        ],
                    );
                    annotate_status_frames(
                        &mut claims,
                        &[
                            (
                                "DLCF_02",
                                ApprovedStatusFrameIR::CapacitySufficient {
                                    criterion: node(
                                        "rated_capacity",
                                        "정격 용량",
                                        ApprovedSemanticTypeIR::State,
                                    ),
                                },
                            ),
                            (
                                "DLCF_07",
                                ApprovedStatusFrameIR::Verification {
                                    finding: node(
                                        "call_quality",
                                        "통화 품질",
                                        ApprovedSemanticTypeIR::State,
                                    ),
                                },
                            ),
                        ],
                    );
                    claims
                },
            ),
        },
        Scenario {
            id: "DOCUMENT_LONG_CONFIRM_02",
            family: "subsea_cable_recovery_and_service_transition_document",
            context: context(StatusUpdate, Team, "해저 통신 케이블 장애"),
            response: phased_action_event_document_response(
                Vec::new(),
                vec![
                    (
                        "traffic_reroute",
                        "통신 우회",
                        Motion,
                        Move,
                        Completed,
                        vec![
                            (Agent, person("network_control_team", "망 관제팀")),
                            (Theme, concept("island_traffic", "도서 지역 통신 트래픽")),
                            (Source, place("damaged_route", "손상 구간")),
                            (Instrument, concept("backup_ring", "예비 광링")),
                            (Destination, place("east_landing_station", "동부 육양국")),
                        ],
                    ),
                    (
                        "joint_sealant_application",
                        "접속함 밀봉",
                        ApprovedEventRealizationClassIR::Transfer,
                        Apply,
                        Completed,
                        vec![
                            (Agent, person("splice_team", "광접속 작업팀")),
                            (Theme, concept("waterproof_sealant", "수중 경화형 밀봉재")),
                            (Target, concept("joint_enclosure", "해저 접속함")),
                        ],
                    ),
                    (
                        "marker_buoy_attachment",
                        "표지 부이 부착",
                        StateChange,
                        Attach,
                        Completed,
                        vec![
                            (Agent, person("dive_team", "잠수 작업팀")),
                            (Theme, concept("marker_buoy", "임시 표지 부이")),
                            (Target, concept("repair_point", "복구 지점 계류선")),
                        ],
                    ),
                    (
                        "route_map_upload",
                        "복구 항로도 업로드",
                        ApprovedEventRealizationClassIR::Transfer,
                        Upload,
                        Completed,
                        vec![
                            (Agent, person("survey_lead", "해양 조사 책임자")),
                            (Theme, concept("recovery_route_map", "복구 항로도")),
                            (Destination, place("navigation_server", "작업선 항해 서버")),
                        ],
                    ),
                    (
                        "repair_log_write",
                        "복구 이력 작성",
                        Creation,
                        Write,
                        Completed,
                        vec![
                            (Agent, person("quality_engineer", "품질 엔지니어")),
                            (Theme, concept("repair_log", "광섬유 접속 복구 이력")),
                            (
                                Location,
                                place("cable_ship_control_room", "케이블 작업선 제어실"),
                            ),
                        ],
                    ),
                    (
                        "power_feed_change",
                        "급전 경로 변경",
                        StateChange,
                        Change,
                        Completed,
                        vec![
                            (Agent, person("power_control_team", "급전 제어팀")),
                            (Theme, concept("repeater_power_route", "중계기 급전 경로")),
                            (InitialState, concept("west_feed", "서부 단독 급전")),
                            (ResultState, concept("dual_feed", "동서 양방향 급전")),
                        ],
                    ),
                    (
                        "attenuation_monitor_activation",
                        "감쇠 감시 가동",
                        StateChange,
                        Activate,
                        Ongoing,
                        vec![
                            (Agent, person("optical_monitoring_team", "광전송 감시팀")),
                            (Theme, concept("attenuation_monitor", "실시간 광감쇠 감시")),
                            (Location, place("west_landing_station", "서부 육양국")),
                        ],
                    ),
                    (
                        "temporary_amplifier_deactivation",
                        "임시 증폭기 비활성화",
                        StateChange,
                        Deactivate,
                        Completed,
                        vec![
                            (Agent, person("transmission_operator", "전송 운용팀")),
                            (Theme, concept("temporary_amplifier", "임시 광증폭기")),
                            (Location, place("backup_rack", "예비 전송 랙")),
                        ],
                    ),
                    (
                        "landing_controller_reset",
                        "육양국 제어기 초기화",
                        StateChange,
                        Reset,
                        Completed,
                        vec![
                            (Agent, person("control_system_team", "제어 시스템팀")),
                            (Theme, concept("landing_controller", "육양국 선로 제어기")),
                        ],
                    ),
                    (
                        "spare_cable_move",
                        "예비 케이블 운반",
                        Motion,
                        Move,
                        Completed,
                        vec![
                            (Agent, person("deck_team", "갑판 작업팀")),
                            (Theme, concept("spare_cable_drum", "예비 케이블 드럼")),
                            (Source, place("aft_deck", "선미 적재 구역")),
                            (Instrument, concept("cable_tensioner", "케이블 장력 조절기")),
                            (Destination, place("repair_bay", "중앙 복구 작업대")),
                        ],
                    ),
                    (
                        "patrol_range_expansion",
                        "선박 통제 순찰 범위 확대",
                        StateChange,
                        Expand,
                        Planned,
                        vec![
                            (Agent, person("marine_safety_team", "해상 안전팀")),
                            (Theme, concept("patrol_range", "작업 해역 순찰 범위")),
                            (Target, concept("five_nautical_miles", "반경 5해리")),
                        ],
                    ),
                    (
                        "customer_channel_open",
                        "복구 문의 채널 개방",
                        StateChange,
                        Open,
                        Scheduled,
                        vec![
                            (Agent, person("customer_response_team", "고객 대응팀")),
                            (Theme, concept("recovery_channel", "전용 복구 문의 채널")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 9,
                                    day: 22,
                                },
                            ),
                        ],
                    ),
                    (
                        "temporary_route_close",
                        "임시 우회 경로 종료",
                        StateChange,
                        Close,
                        Scheduled,
                        vec![
                            (Agent, person("network_control_lead", "망 관제 책임자")),
                            (Theme, concept("temporary_route", "임시 우회 경로")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 9,
                                    day: 24,
                                },
                            ),
                        ],
                    ),
                    (
                        "inspection_report_upload",
                        "매설 상태 보고서 업로드",
                        ApprovedEventRealizationClassIR::Transfer,
                        Upload,
                        Planned,
                        vec![
                            (Agent, person("burial_survey_team", "매설 조사팀")),
                            (Theme, concept("burial_report", "케이블 매설 상태 보고서")),
                            (
                                Destination,
                                place("asset_registry", "해저 자산 관리 시스템"),
                            ),
                        ],
                    ),
                ],
                {
                    let mut claims = long_claims(
                        "DLCG",
                        &[
                        (
                            "해저 통신 케이블 장애",
                            Cause,
                            "어선 닻에 의한 외피 손상과 광섬유 단선",
                        ),
                        ("도서 지역 통신 품질", Impact, "야간 음성 통화 지연 증가"),
                        ("핵심 서비스 복구", Status, "완료됨"),
                        ("일반 인터넷 복구", Status, "진행 중"),
                        ("복구 지점", Location, "북위 34도 인근 작업 해역"),
                        ("광감쇠 측정", Status, "허용 범위 확보"),
                        ("접속함 수밀 시험", Status, "통과"),
                        ("매설 심도 영향", Impact, "향후 어구 접촉 위험 감소 필요"),
                        ("항행 경보 유지", Status, "작업선 철수 전까지 필요"),
                        ("어선 접근 통제", Owner, "해경 상황실"),
                        ("고객 장애 공지", Owner, "서비스 운영팀"),
                        ("장비 회수", Deadline, "2026년 9월 25일 일몰 전"),
                        ("최종 품질 검증", Deadline, "연속 48시간 안정 운용 뒤"),
                        ("보험 증빙 보존", Owner, "해저 자산 관리팀"),
                        ("최종 서비스 상태", Status, "조건부 정상 운영"),
                        ],
                    );
                    annotate_status_frames(
                        &mut claims,
                        &[ (
                            "DLCG_05",
                            ApprovedStatusFrameIR::WithinRange {
                                range: node(
                                    "permitted_attenuation_range",
                                    "허용 범위",
                                    ApprovedSemanticTypeIR::State,
                                ),
                            },
                        ) ],
                    );
                    claims
                },
                vec![
                    action_frame("seafloor_boundary_inspection", "해저 지형 재조사", "seafloor_boundary_inspection_event", "노출 암반과 퇴적층 경계 확인", Inspection, Inspect, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("exposed_rock_and_sediment_boundary", "노출 암반과 퇴적층 경계"))]),
                    action_frame("carrier_usage_comparison", "손실 트래픽 정산", "carrier_usage_comparison_event", "통신사별 우회 사용량 대조", Inspection, Compare, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("carrier_rerouted_usage", "통신사별 우회 사용량")), (Target, concept("rerouted_usage_baseline", "기준 사용량"))]),
                    action_frame("burial_depth_improvement", "재발 방지 검토", "burial_depth_improvement_event", "매설 깊이 개선", StateChange, Change, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("cable_burial_depth", "케이블 매설 깊이")), (InitialState, concept("current_burial_depth", "현재 매설 깊이")), (ResultState, concept("improved_burial_depth", "개선 매설 깊이"))]),
                    action_frame("chart_marking_improvement", "재발 방지 검토", "chart_marking_improvement_event", "해도 표기 개선", StateChange, Change, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("navigation_chart_marking", "해도 표기")), (InitialState, concept("current_chart_marking", "현재 해도 표기")), (ResultState, concept("improved_chart_marking", "개선 해도 표기"))]),
                ],
            ),
        },
        Scenario {
            id: "DOCUMENT_LONG_CONFIRM_03",
            family: "international_print_exhibition_deinstallation_handoff_document",
            context: context(HandoffNote, Team, "국제 판화 특별전 철수"),
            response: phased_action_event_document_response(
                long_claims(
                    "DLCH_LEAD",
                    &[
                        ("국제 판화 특별전 철수", Cause, "전시 운영 기간 종료"),
                        ("관람 운영", Status, "완료됨"),
                        ("작품 포장 일정", Status, "진행 중"),
                        ("전시장 출입", Status, "철수 인력으로 제한"),
                        ("작품 임시 집결지", Location, "지하 1층 포장실"),
                        ("철수 동선 영향", Impact, "화물 승강기 일반 사용 제한"),
                        ("작품 상태 확인", Status, "필요"),
                    ],
                ),
                vec![
                    (
                        "print_crate_move",
                        "판화 작품 상자 운반",
                        Motion,
                        Move,
                        Completed,
                        vec![
                            (Agent, person("art_handler_team", "미술품 취급팀")),
                            (Theme, concept("print_crates", "판화 작품 상자")),
                            (Source, place("gallery_three", "제3전시실")),
                            (Instrument, concept("air_ride_cart", "진동 완충 카트")),
                            (Destination, place("packing_room", "지하 포장실")),
                        ],
                    ),
                    (
                        "frame_coating_application",
                        "액자 보호제 도포",
                        ApprovedEventRealizationClassIR::Transfer,
                        Apply,
                        Completed,
                        vec![
                            (Agent, person("conservation_team", "보존처리팀")),
                            (Theme, concept("frame_protectant", "중성 보호제")),
                            (Target, concept("metal_frame_joint", "금속 액자 접합부")),
                        ],
                    ),
                    (
                        "rfid_tag_attachment",
                        "RFID 인계표 부착",
                        StateChange,
                        Attach,
                        Completed,
                        vec![
                            (Agent, person("registration_team", "등록 관리팀")),
                            (Theme, concept("rfid_handover_tag", "RFID 인계표")),
                            (Target, concept("transport_case", "운송 케이스 손잡이")),
                        ],
                    ),
                    (
                        "inventory_upload",
                        "철수 재고표 업로드",
                        ApprovedEventRealizationClassIR::Transfer,
                        Upload,
                        Completed,
                        vec![
                            (Agent, person("collection_registrar", "소장품 등록 담당자")),
                            (Theme, concept("deinstallation_inventory", "철수 재고표")),
                            (
                                Destination,
                                place("collection_system", "소장품 관리 시스템"),
                            ),
                        ],
                    ),
                    (
                        "condition_report_write",
                        "상태 조사서 작성",
                        Creation,
                        Write,
                        Completed,
                        vec![
                            (Agent, person("paper_conservator", "지류 보존처리사")),
                            (Theme, concept("condition_report", "작품 상태 조사서")),
                            (Location, place("conservation_lab", "보존과학실")),
                        ],
                    ),
                    (
                        "climate_mode_change",
                        "수장고 공조 모드 변경",
                        StateChange,
                        Change,
                        Completed,
                        vec![
                            (Agent, person("facility_operator", "시설 운전팀")),
                            (
                                Theme,
                                concept("storage_climate_mode", "임시 수장고 공조 모드"),
                            ),
                            (InitialState, concept("exhibition_mode", "전시 유지")),
                            (ResultState, concept("packing_mode", "포장 작업 우선")),
                        ],
                    ),
                    (
                        "dehumidifier_activation",
                        "제습 설비 가동",
                        StateChange,
                        Activate,
                        Ongoing,
                        vec![
                            (Agent, person("environment_team", "환경 관리팀")),
                            (Theme, concept("mobile_dehumidifier", "이동식 제습 설비")),
                            (Location, place("packing_room", "지하 포장실")),
                        ],
                    ),
                    (
                        "guide_kiosk_deactivation",
                        "안내 키오스크 비활성화",
                        StateChange,
                        Deactivate,
                        Completed,
                        vec![
                            (Agent, person("digital_exhibition_team", "디지털 전시팀")),
                            (Theme, concept("guide_kiosk", "다국어 안내 키오스크")),
                            (Location, place("gallery_lobby", "전시관 로비")),
                        ],
                    ),
                    (
                        "sensor_gateway_reset",
                        "환경 센서 게이트웨이 초기화",
                        StateChange,
                        Reset,
                        Completed,
                        vec![
                            (Agent, person("museum_it_team", "미술관 전산팀")),
                            (Theme, concept("sensor_gateway", "온습도 센서 게이트웨이")),
                        ],
                    ),
                    (
                        "shelf_capacity_expansion",
                        "임시 선반 용량 확대",
                        StateChange,
                        Expand,
                        Planned,
                        vec![
                            (Agent, person("storage_team", "수장고 관리팀")),
                            (
                                Theme,
                                concept("temporary_shelf_capacity", "임시 선반 수용량"),
                            ),
                            (Target, concept("forty_crates", "작품 상자 40개")),
                        ],
                    ),
                    (
                        "loan_channel_open",
                        "대여기관 인계 채널 개방",
                        StateChange,
                        Open,
                        Scheduled,
                        vec![
                            (Agent, person("loan_coordinator", "대여 업무 담당자")),
                            (Theme, concept("handover_channel", "대여기관 인계 채널")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 9,
                                    day: 23,
                                },
                            ),
                        ],
                    ),
                    (
                        "temporary_gallery_close",
                        "임시 포장실 폐쇄",
                        StateChange,
                        Close,
                        Scheduled,
                        vec![
                            (Agent, person("operations_manager", "운영 관리자")),
                            (Theme, place("packing_room", "지하 포장실")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 9,
                                    day: 28,
                                },
                            ),
                        ],
                    ),
                ],
                {
                    let mut claims = long_claims(
                        "DLCH_TAIL",
                        &[
                        ("작품 상자", Status, "봉인 완료"),
                        ("완충재 산성도 검사", Status, "통과"),
                        ("운송 차량 내부 온도", Status, "허용 범위 확보"),
                        ("해외 대여작 보험 종료", Deadline, "반출 확인서 서명 뒤"),
                        ("국내 소장품 수장고 입고", Deadline, "9월 26일 오후 6시 전"),
                        ("반출 차량 배차", Owner, "물류 조정 담당자"),
                        ("세관 서류 확인", Owner, "국제 교류팀"),
                        ("대여기관 서명 누락", Impact, "작품 반출 일정 지연 가능성"),
                        ("운송 케이스 나사", Status, "재조임 필요"),
                        ("목재 케이스 해충 흔적", Status, "미확인"),
                        ("작품 인계 확인", Owner, "대여기관 동행 큐레이터"),
                        ("조명 레일 전원", Status, "차단됨"),
                        ("폐기 포장재 분류", Owner, "시설 지원팀"),
                        ("철수 완료 보고", Deadline, "9월 29일 오전 운영회의"),
                        ("최종 인계 상태", Status, "조건부 완료"),
                        ],
                    );
                    annotate_status_frames(
                        &mut claims,
                        &[ (
                            "DLCH_TAIL_02",
                            ApprovedStatusFrameIR::WithinRange {
                                range: node(
                                    "permitted_transport_temperature_range",
                                    "허용 범위",
                                    ApprovedSemanticTypeIR::State,
                                ),
                            },
                        ) ],
                    );
                    claims
                },
                vec![
                    action_frame("artwork_photo_record", "작품별 사진 기록", "artwork_photo_record_event", "포장 전후 작품 촬영", Creation, Record, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("artwork_front_and_corner_photos", "포장 전후 작품 정면과 모서리 사진"))]),
                    action_frame("esign_link_resend", "전자서명 링크 재발송", "esign_link_resend_event", "전자서명 링크 재발송", ApprovedEventRealizationClassIR::Transfer, Send, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("electronic_signature_link", "전자서명 링크")), (Destination, lexical("loan_institution", "대여기관", ApprovedSemanticTypeIR::Organization))]),
                    action_frame("pest_case_move", "격리실 이동", "pest_case_move_event", "목재 케이스 격리실 이동", Motion, Move, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("wooden_case", "목재 케이스")), (Destination, place("isolation_room", "격리실"))]),
                    action_frame("pest_surface_inspection", "표면 검사", "pest_surface_inspection_event", "목재 케이스 표면 검사", Inspection, Inspect, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("wooden_case_surface", "목재 케이스 표면"))]),
                    action_frame("nail_hole_filling", "못 구멍 충전", "nail_hole_filling_event", "못 구멍 충전", StateChange, Fill, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("filler_material", "충전재")), (Target, concept("wall_nail_holes", "전시장 벽면 못 구멍"))]),
                    action_frame("paint_surface_repair", "도장면 보수", "paint_surface_repair_event", "전시장 도장면 보수", StateChange, Repair, Planned, ApprovedEventVoiceIR::Passive, vec![(Theme, concept("gallery_paint_surface", "전시장 도장면"))]),
                ],
            ),
        },
        // This fixture intentionally carries an explicit EventFrame for each
        // operational action.  It is separate from the older Action claims
        // above: their free lexical labels do not supply a safe predicate
        // sense or role frame and must remain observable as opaque fallbacks.
        Scenario {
            id: "DOCUMENT_LONG_CONFIRM_04",
            family: "hospital_isolation_ward_handover_document",
            context: context(HandoffNote, Team, "격리 병동 교대"),
            response: phased_multi_event_document_response(
                {
                    let mut claims = long_claims(
                        "DLCI_LEAD",
                        &[
                        ("격리 병동 교대", Status, "진행 중"),
                        ("음압 격리실 압력", Status, "허용 범위 확보"),
                        ("보호구 재고", Status, "36시간분 확보"),
                        (
                            "검체 이송 상자 보관 위치",
                            Location,
                            "동측 전용 승강기 앞 대기 구역",
                        ),
                        ("격리 병동 방문", Status, "필수 인력으로 제한"),
                        ("야간 응급 호출", Owner, "감염관리 당직 간호사"),
                        ],
                    );
                    annotate_status_frames(
                        &mut claims,
                        &[ (
                            "DLCI_LEAD_01",
                            ApprovedStatusFrameIR::WithinRange {
                                range: node(
                                    "permitted_negative_pressure_range",
                                    "허용 범위",
                                    ApprovedSemanticTypeIR::State,
                                ),
                            },
                        ) ],
                    );
                    claims
                },
                vec![
                    (
                        "air_filter_inspection",
                        "음압 공조 필터 점검",
                        Inspection,
                        Inspect,
                        Completed,
                        vec![
                            (Agent, person("facility_engineer", "시설 엔지니어")),
                            (Theme, concept("negative_pressure_filter", "음압 공조 필터")),
                            (Location, place("isolation_ward", "격리 병동")),
                            (
                                Date,
                                ApprovedOpenValueIR::Date {
                                    year: 2026,
                                    month: 9,
                                    day: 21,
                                },
                            ),
                        ],
                    ),
                    (
                        "culture_result_send",
                        "배양 검사 결과 발송",
                        ApprovedEventRealizationClassIR::Transfer,
                        Send,
                        Completed,
                        vec![
                            (Agent, person("microbiology_team", "미생물 검사팀")),
                            (Theme, concept("culture_result", "배양 검사 결과")),
                            (
                                Destination,
                                place("infection_channel", "감염관리 전용 채널"),
                            ),
                        ],
                    ),
                    (
                        "specimen_record_delivery",
                        "검체 이송 기록 전달",
                        ApprovedEventRealizationClassIR::Transfer,
                        Deliver,
                        Completed,
                        vec![
                            (Agent, person("transport_nurse", "검체 이송 간호사")),
                            (Patient, person("ward_charge_nurse", "병동 책임 간호사")),
                            (Theme, concept("transport_record", "검체 이송 기록")),
                        ],
                    ),
                    (
                        "visitor_notice_provision",
                        "보호자 방문 안내 제공",
                        ApprovedEventRealizationClassIR::Transfer,
                        Give,
                        Completed,
                        vec![
                            (Agent, person("front_desk_staff", "병동 안내 직원")),
                            (Patient, person("patient_guardian", "환자 보호자")),
                            (Theme, concept("visitor_notice", "보호자 방문 안내")),
                        ],
                    ),
                    (
                        "isolation_checklist_create",
                        "격리실 교대 점검표 생성",
                        Creation,
                        Create,
                        Completed,
                        vec![
                            (Agent, person("charge_nurse", "책임 간호사")),
                            (Theme, concept("shift_checklist", "격리실 교대 점검표")),
                        ],
                    ),
                    (
                        "terminal_disinfection_start",
                        "퇴실 병상 소독 시작",
                        StateChange,
                        Start,
                        Ongoing,
                        vec![
                            (Agent, person("environmental_team", "환경관리팀")),
                            (Theme, concept("terminal_disinfection", "퇴실 병상 소독")),
                            (Location, place("room_712", "712호 격리실")),
                        ],
                    ),
                    (
                        "hand_hygiene_training_complete",
                        "교대 전 손 위생 교육 완료",
                        StateChange,
                        Complete,
                        Completed,
                        vec![
                            (Agent, person("infection_control_nurse", "감염관리 간호사")),
                            (
                                Theme,
                                concept("hand_hygiene_training", "교대 전 손 위생 교육"),
                            ),
                        ],
                    ),
                ],
                long_claims(
                    "DLCI_TAIL",
                    &[
                        ("음압 경보 이력", Status, "이상 없음"),
                        ("검체 이송 상자", Status, "봉인 완료"),
                        ("다음 필터 교체", Deadline, "10월 첫째 주"),
                        ("소독 완료 확인", Deadline, "다음 환자 입실 전"),
                        ("배양 결과 확인", Owner, "감염관리 전담의"),
                        ("격리 병동 운영", Status, "조건부 운영 가능"),
                    ],
                ),
            ),
        },
    ]);
    rows
}

fn expected_format_marker(format: WorkplaceCommunicationFormatIR) -> Option<&'static str> {
    match format {
        WorkplaceCommunicationFormatIR::InstantMessage => None,
        WorkplaceCommunicationFormatIR::Email => Some("제목:"),
        WorkplaceCommunicationFormatIR::StatusUpdate => Some(" 현황"),
        WorkplaceCommunicationFormatIR::IncidentAlert => Some("[긴급]"),
        WorkplaceCommunicationFormatIR::MeetingSummary => Some("요약"),
        WorkplaceCommunicationFormatIR::HandoffNote => Some(" 인수인계"),
        WorkplaceCommunicationFormatIR::ApprovalRequest => Some("검토"),
        WorkplaceCommunicationFormatIR::CorrectionNotice => Some(" 정정 안내"),
        WorkplaceCommunicationFormatIR::ExecutiveBrief => Some(" 핵심 보고"),
    }
}

fn diagnostics(
    surface: &str,
    response: &ApprovedCompositionalResponseIR,
    format: WorkplaceCommunicationFormatIR,
    opaque_action_claims: usize,
) -> Vec<String> {
    let mut result = Vec::new();
    if surface.contains("핵심 내용을 항목별로 정리")
        && matches!(
            format,
            WorkplaceCommunicationFormatIR::Email | WorkplaceCommunicationFormatIR::InstantMessage
        )
    {
        result.push("GENERIC_DOCUMENT_BOILERPLATE_IN_SHORT_CHANNEL".into());
    }
    if surface.contains("위 내용은 확인된 정보만 반영")
        && matches!(
            format,
            WorkplaceCommunicationFormatIR::Email | WorkplaceCommunicationFormatIR::InstantMessage
        )
    {
        result.push("EVIDENCE_BOUNDARY_BOILERPLATE_IN_SHORT_CHANNEL".into());
    }
    // A lexical phrase such as `보존 상태는` may be a legitimate event
    // argument.  Diagnose only the actual failure mode: a Status relation
    // whose subject does not itself name a state head but was rendered as
    // `{subject} 상태는 …`.  This keeps the probe focused on an IR-to-surface
    // nominalization defect rather than treating every natural occurrence of
    // the Korean noun `상태` as a failure.
    let redundant_status_nominalization = response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Status)
        .map(|claim| claim.subject.canonical_lexical_label.trim())
        .filter(|subject| !subject.ends_with("상태"))
        .any(|subject| surface.contains(&format!("{subject} 상태는")));
    if redundant_status_nominalization {
        result.push("NOMINAL_STATUS_REALIZATION".into());
    }
    let heading_count = surface
        .lines()
        .filter(|line| line.trim_start().starts_with('#'))
        .count();
    if heading_count > 6 {
        result.push("HEADING_OVERLOAD".into());
    }
    if surface.contains("또한,") && surface.lines().count() <= 4 {
        result.push("STIFF_ADDITIVE_CONNECTIVE".into());
    }
    if surface.contains("| 대상 | 항목 | 값 |") && surface.contains("xychart-beta") {
        result.push("TABLE_AND_CHART_DUPLICATION".into());
    }
    // Action(subject, free lexical label) is semantically valid, but it lacks
    // an approved predicate sense and role frame. Keep its safe generic
    // realization visible so an exact inverse is not misread as a naturalness
    // pass.
    if opaque_action_claims > 0 {
        result.push("OPAQUE_ACTION_FALLBACK".into());
    }
    result
}

fn opaque_actions(response: &ApprovedCompositionalResponseIR) -> Vec<OpaqueActionProbe> {
    response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Action && claim.polarity)
        .filter(|claim| {
            !matches!(
                &claim.value,
                ApprovedOpenValueIR::Lexical(event)
                    if event.semantic_type == ApprovedSemanticTypeIR::Event
                        && response.event_realizations.iter().any(|realization| {
                            realization.subject_node_id == event.node_id
                                && realization.predicate_sense.is_some()
                        })
            )
        })
        .map(|claim| OpaqueActionProbe {
            proposition_id: claim.proposition_id.clone(),
            subject_node_id: claim.subject.node_id.clone(),
            subject_label: claim.subject.canonical_lexical_label.clone(),
            action_value: match &claim.value {
                ApprovedOpenValueIR::Lexical(value) => value.canonical_lexical_label.clone(),
                ApprovedOpenValueIR::Text(value) => value.clone(),
                other => format!("{other:?}"),
            },
            modality: claim.modality,
        })
        .collect()
}

fn main() {
    let output_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("D:/B_Core_validation/workplace_language_generation_probe_v1.json")
    });
    let scenario_filter = env::var("BCORE_PROBE_SCENARIO").ok();
    let claim_limit = env::var("BCORE_PROBE_CLAIM_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let mut scenarios = scenarios()
        .into_iter()
        .filter(|scenario| {
            scenario_filter
                .as_deref()
                .is_none_or(|filter| scenario.id == filter)
        })
        .collect::<Vec<_>>();
    if let Some(limit) = claim_limit {
        for scenario in &mut scenarios {
            scenario.response.claims.truncate(limit);
            scenario.response.event_realizations.retain(|event| {
                scenario
                    .response
                    .claims
                    .iter()
                    .any(|claim| claim.subject.node_id == event.subject_node_id)
            });
            scenario.response.clause_plan = scenario
                .response
                .claims
                .iter()
                .enumerate()
                .map(|(index, claim)| ApprovedClauseUnitIR {
                    unit_index: index,
                    role: "ASSERT".into(),
                    proposition_ids: vec![claim.proposition_id.clone()],
                    predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
                })
                .collect();
            scenario.response.semantic_sha256 = compositional_response_sha256(&scenario.response);
            assert!(scenario.response.validate());
        }
    }
    let mut rows = Vec::new();
    let mut diagnostic_counts = std::collections::BTreeMap::new();
    let mut opaque_action_claims = 0usize;
    let mut typed_event_realizations = 0usize;
    for scenario in scenarios {
        let output = realize_workplace_response(&scenario.response, scenario.context.clone())
            .unwrap_or_else(|error| panic!("{} failed: {error}", scenario.id));
        let marker = expected_format_marker(scenario.context.format);
        let baseline_fit = marker.is_none();
        let formatted_fit = marker.is_none_or(|marker| output.rendered.contains(marker));
        let scenario_opaque_action_claims = opaque_action_claim_count(&scenario.response);
        let scenario_opaque_actions = opaque_actions(&scenario.response);
        assert_eq!(
            scenario_opaque_actions.len(),
            scenario_opaque_action_claims,
            "opaque-action inventory must account for every opaque Action claim"
        );
        let scenario_typed_event_realizations = scenario.response.event_realizations.len();
        opaque_action_claims += scenario_opaque_action_claims;
        typed_event_realizations += scenario_typed_event_realizations;
        let row_diagnostics = diagnostics(
            &output.rendered,
            &scenario.response,
            scenario.context.format,
            scenario_opaque_action_claims,
        );
        for diagnostic in &row_diagnostics {
            *diagnostic_counts.entry(diagnostic.clone()).or_insert(0) += 1;
        }
        rows.push(ProbeRow {
            id: scenario.id.into(),
            family: scenario.family.into(),
            format: scenario.context.format,
            audience: scenario.context.audience,
            speech_act: scenario.response.speech_act,
            register: scenario.response.style.register,
            claim_count: scenario.response.claims.len(),
            opaque_action_claims: scenario_opaque_action_claims,
            opaque_actions: scenario_opaque_actions,
            typed_event_realizations: scenario_typed_event_realizations,
            planned_event_predicates: output.body.plan.event_predicates.len(),
            planned_event_subject_ids: output
                .body
                .plan
                .event_predicates
                .iter()
                .map(|predicate| predicate.subject_node_id.clone())
                .collect(),
            baseline_body: output.body.markdown.clone(),
            rendered: output.rendered,
            semantic_inverse_pass: output
                .body
                .semantic_interpretation
                .validate(&scenario.response),
            unsupported_claims: output.body.unsupported_claims,
            baseline_format_fit: baseline_fit,
            formatted_fit,
            diagnostics: row_diagnostics,
        });
    }
    let mut format_counts = std::collections::BTreeMap::new();
    let mut speech_act_counts = std::collections::BTreeMap::new();
    for row in &rows {
        *format_counts
            .entry(format!("{:?}", row.format))
            .or_insert(0) += 1;
        *speech_act_counts
            .entry(format!("{:?}", row.speech_act))
            .or_insert(0) += 1;
    }
    let report = ProbeReport {
        schema: "B_CORE_WORKPLACE_LANGUAGE_GENERATION_PROBE_3",
        scenarios: rows.len(),
        formats: 9,
        semantic_inverse_pass: rows.iter().filter(|row| row.semantic_inverse_pass).count(),
        unsupported_claims: rows.iter().map(|row| row.unsupported_claims).sum(),
        opaque_action_claims,
        typed_event_realizations,
        baseline_format_fit: rows.iter().filter(|row| row.baseline_format_fit).count(),
        formatted_fit: rows.iter().filter(|row| row.formatted_fit).count(),
        format_counts,
        speech_act_counts,
        diagnostic_counts,
        rows,
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("output directory");
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report).unwrap()).expect("write report");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
