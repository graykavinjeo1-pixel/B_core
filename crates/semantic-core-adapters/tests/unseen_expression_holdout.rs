use std::collections::BTreeSet;

use semantic_core_adapters::{
    compositional_response_sha256, realize_document_response, ApprovedClauseUnitIR,
    ApprovedCompositionalClaimIR, ApprovedCompositionalResponseIR, ApprovedDiscourseRelationIR,
    ApprovedEventExpressionPreferenceIR, ApprovedEventFocusIR, ApprovedEventInformationRoleIR,
    ApprovedEventInformationStructureIR, ApprovedEventPerspectiveIR, ApprovedEventPhaseIR,
    ApprovedEventPragmaticContextIR, ApprovedEventPredicateSenseIR,
    ApprovedEventRealizationClassIR, ApprovedEventRealizationIR, ApprovedEventVoiceIR,
    ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR,
    ApprovedRelationTypeIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR, ApprovedSpeechActIR,
    ApprovedVerbosityIR, DiscourseFocusCandidateIR, DiscourseFocusStateIR,
    DocumentResponseOutputIR, LanguageCodeIR, LanguageRegisterIR,
    APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA, COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
};

#[derive(Clone, Copy)]
struct RoleValue {
    role: ApprovedRelationTypeIR,
    node_id: &'static str,
    semantic_type: ApprovedSemanticTypeIR,
    label: &'static str,
}

#[derive(Clone, Copy)]
struct HoldoutCase {
    id: &'static str,
    event_label: &'static str,
    class: ApprovedEventRealizationClassIR,
    sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    register: LanguageRegisterIR,
    roles: &'static [RoleValue],
    expected: &'static str,
}

const fn role(
    role: ApprovedRelationTypeIR,
    node_id: &'static str,
    semantic_type: ApprovedSemanticTypeIR,
    label: &'static str,
) -> RoleValue {
    RoleValue {
        role,
        node_id,
        semantic_type,
        label,
    }
}

fn event_response(case: &HoldoutCase) -> ApprovedCompositionalResponseIR {
    let event_id = format!("{}_event", case.id.to_ascii_lowercase());
    let claims = case
        .roles
        .iter()
        .enumerate()
        .map(|(index, value)| ApprovedCompositionalClaimIR {
            proposition_id: format!("{}_P_{index}", case.id),
            subject: ApprovedLexicalNodeIR {
                node_id: event_id.clone(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: case.event_label.into(),
            },
            relation: value.role,
            value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: value.node_id.into(),
                semantic_type: value.semantic_type,
                canonical_lexical_label: value.label.into(),
            }),
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
})
        .collect::<Vec<_>>();
    let mut response = ApprovedCompositionalResponseIR {
        schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
        speech_act: ApprovedSpeechActIR::Inform,
        operation: ApprovedOperationIR::Assert,
        clause_plan: claims
            .iter()
            .enumerate()
            .map(|(index, claim)| ApprovedClauseUnitIR {
                unit_index: index,
                role: "ASSERT".into(),
                proposition_ids: vec![claim.proposition_id.clone()],
                predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
            })
            .collect(),
        claims,
        event_realizations: vec![ApprovedEventRealizationIR {
            subject_node_id: event_id,
            class: case.class,
            predicate_sense: Some(case.sense),
            phase: Some(case.phase),
            perspective: Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
            voice: None,
            information_structure: None,
        }],
        discourse_relation: ApprovedDiscourseRelationIR::Statement,
        style: ApprovedResponseStyleIR {
            register: case.register,
            verbosity: ApprovedVerbosityIR::Short,
        },
        source_world_state_sha256: "c".repeat(64),
        source_deliberation_sha256: "d".repeat(64),
        approval_replay_verified: true,
        unsupported_claims: 0,
        semantic_sha256: String::new(),
    };
    response.semantic_sha256 = compositional_response_sha256(&response);
    assert!(response.validate(), "invalid holdout response: {}", case.id);
    response
}

fn semantic_frame(output: &DocumentResponseOutputIR) -> Vec<String> {
    output
        .semantic_interpretation
        .claims
        .iter()
        .map(|claim| {
            format!(
                "{}|{:?}|{:?}|{:?}|{}|{:?}",
                claim.proposition_id,
                claim.subject,
                claim.relation,
                claim.value,
                claim.polarity,
                claim.modality
            )
        })
        .collect()
}

fn assert_verified_surface(
    response: &ApprovedCompositionalResponseIR,
    expected: &str,
) -> DocumentResponseOutputIR {
    let output = realize_document_response(response, LanguageCodeIR::Korean)
        .unwrap_or_else(|error| panic!("failed to realize {expected:?}: {error}"));
    assert!(output.validate(response), "{}", output.markdown);
    assert!(
        output.markdown.contains(expected),
        "expected {expected:?}, got {:?}",
        output.markdown
    );
    assert_eq!(
        output.semantic_interpretation.recovered_claim_ids,
        response
            .claims
            .iter()
            .map(|claim| claim.proposition_id.clone())
            .collect::<Vec<_>>()
    );
    assert!(output
        .semantic_interpretation
        .unsupported_surfaces
        .is_empty());
    assert_eq!(output.semantic_interpretation.ambiguous_surface_claims, 0);
    for forbidden in response
        .claims
        .iter()
        .flat_map(|claim| {
            let mut ids = vec![claim.subject.node_id.as_str()];
            if let ApprovedOpenValueIR::Lexical(node) = &claim.value {
                ids.push(node.node_id.as_str());
            }
            ids
        })
        .chain(["B_CORE", "APPROVED_RESPONSE_IR"])
    {
        assert!(
            !output.markdown.contains(forbidden),
            "internal symbol {forbidden:?} leaked into {:?}",
            output.markdown
        );
    }
    output
}

#[test]
fn unseen_lexical_values_realize_all_registered_predicate_families() {
    const CASES: &[HoldoutCase] = &[
        HoldoutCase {
            id: "SEND_COMPLETED",
            event_label: "보고서 발송",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Send,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "igyeom_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "이겸",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "inspection_report_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "점검 보고서",
                ),
                role(
                    ApprovedRelationTypeIR::Destination,
                    "audit_repository_holdout",
                    ApprovedSemanticTypeIR::Location,
                    "감사 보관소",
                ),
            ],
            expected: "이겸이 점검 보고서를 감사 보관소로 발송했습니다.",
        },
        HoldoutCase {
            id: "SEND_PLANNED",
            event_label: "보고서 발송 계획",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Send,
            phase: ApprovedEventPhaseIR::Planned,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "igyeom_planned_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "이겸",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "inspection_report_planned_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "점검 보고서",
                ),
                role(
                    ApprovedRelationTypeIR::Destination,
                    "audit_repository_planned_holdout",
                    ApprovedSemanticTypeIR::Location,
                    "감사 보관소",
                ),
            ],
            expected: "이겸이 점검 보고서를 감사 보관소로 보낼 거예요.",
        },
        HoldoutCase {
            id: "GIVE_COMPLETED",
            event_label: "열쇠 제공",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Give,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "foundation_holdout",
                    ApprovedSemanticTypeIR::Organization,
                    "재단",
                ),
                role(
                    ApprovedRelationTypeIR::Patient,
                    "seoyun_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "서윤",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "recovery_key_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "복구 열쇠",
                ),
            ],
            expected: "재단이 서윤에게 복구 열쇠를 제공했습니다.",
        },
        HoldoutCase {
            id: "CREATE_ONGOING",
            event_label: "지도 생성",
            class: ApprovedEventRealizationClassIR::Creation,
            sense: ApprovedEventPredicateSenseIR::Create,
            phase: ApprovedEventPhaseIR::Ongoing,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "research_team_holdout",
                    ApprovedSemanticTypeIR::Organization,
                    "연구팀",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "simulation_map_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "시뮬레이션 지도",
                ),
            ],
            expected: "연구팀이 시뮬레이션 지도를 생성 중입니다.",
        },
        HoldoutCase {
            id: "START_SCHEDULED",
            event_label: "점검 시작",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Start,
            phase: ApprovedEventPhaseIR::Scheduled,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "operator_start_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "운영자",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "night_inspection_holdout",
                    ApprovedSemanticTypeIR::Event,
                    "야간 점검",
                ),
            ],
            expected: "운영자가 야간 점검을 시작합니다.",
        },
        HoldoutCase {
            id: "COMPLETE_COMPLETED",
            event_label: "색인 작업 완료",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Complete,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "operator_complete_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "운영자",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "index_rebuild_holdout",
                    ApprovedSemanticTypeIR::Event,
                    "색인 재구축",
                ),
            ],
            expected: "운영자가 색인 재구축을 마쳤어요.",
        },
        HoldoutCase {
            id: "OPEN_PLANNED",
            event_label: "출입구 개방",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Open,
            phase: ApprovedEventPhaseIR::Planned,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "administrator_open_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "관리자",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "south_gate_holdout",
                    ApprovedSemanticTypeIR::Location,
                    "남문",
                ),
            ],
            expected: "관리자가 남문을 개방할 예정입니다.",
        },
        HoldoutCase {
            id: "CLOSE_CANCELLED",
            event_label: "통로 폐쇄",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Close,
            phase: ApprovedEventPhaseIR::Cancelled,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "control_team_holdout",
                    ApprovedSemanticTypeIR::Organization,
                    "관제팀",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "temporary_passage_holdout",
                    ApprovedSemanticTypeIR::Location,
                    "임시 통로",
                ),
            ],
            expected: "관제팀이 임시 통로를 폐쇄할 예정이었지만 취소됐습니다.",
        },
        HoldoutCase {
            id: "DELIVER_COMPLETED",
            event_label: "상자 전달",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Deliver,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "delivery_team_holdout",
                    ApprovedSemanticTypeIR::Organization,
                    "배송팀",
                ),
                role(
                    ApprovedRelationTypeIR::Patient,
                    "recipient_holdout",
                    ApprovedSemanticTypeIR::Person,
                    "수신자",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "sealed_box_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "봉인 상자",
                ),
            ],
            expected: "배송팀이 수신자에게 봉인 상자를 전달했습니다.",
        },
        HoldoutCase {
            id: "MOVE_COMPLETED",
            event_label: "상자 이동",
            class: ApprovedEventRealizationClassIR::Motion,
            sense: ApprovedEventPredicateSenseIR::Move,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Formal,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "transport_team_holdout",
                    ApprovedSemanticTypeIR::Organization,
                    "운반팀",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "sample_box_holdout",
                    ApprovedSemanticTypeIR::Concept,
                    "시료 상자",
                ),
                role(
                    ApprovedRelationTypeIR::Destination,
                    "isolation_zone_holdout",
                    ApprovedSemanticTypeIR::Location,
                    "격리 구역",
                ),
            ],
            expected: "운반팀이 시료 상자를 격리 구역으로 옮겼습니다.",
        },
    ];

    for case in CASES {
        let response = event_response(case);
        let output = assert_verified_surface(&response, case.expected);
        println!("UNSEEN_EXPRESSION\t{}\t{}", case.id, case.expected);
        assert_eq!(semantic_frame(&output).len(), case.roles.len());
    }
}

#[test]
fn one_unseen_frame_supports_four_verified_information_structures() {
    const BASE: HoldoutCase = HoldoutCase {
        id: "TRANSFER_DIVERSITY",
        event_label: "기록 전송",
        class: ApprovedEventRealizationClassIR::Transfer,
        sense: ApprovedEventPredicateSenseIR::Transfer,
        phase: ApprovedEventPhaseIR::Completed,
        register: LanguageRegisterIR::Formal,
        roles: &[
            role(
                ApprovedRelationTypeIR::Agent,
                "jiu_holdout",
                ApprovedSemanticTypeIR::Person,
                "지우",
            ),
            role(
                ApprovedRelationTypeIR::Theme,
                "audit_record_holdout",
                ApprovedSemanticTypeIR::Concept,
                "감사 기록",
            ),
            role(
                ApprovedRelationTypeIR::Destination,
                "archive_vault_holdout",
                ApprovedSemanticTypeIR::Location,
                "장기 보관소",
            ),
        ],
        expected: "지우가 감사 기록을 장기 보관소로 전송했습니다.",
    };

    let mut base = event_response(&BASE);
    base.event_realizations[0].perspective = None;
    base.semantic_sha256 = compositional_response_sha256(&base);
    assert!(base.validate());

    let context = |requested_focus_node_id: Option<&str>,
                   backgrounded_node_ids: &[&str],
                   recoverable_node_ids: &[&str],
                   expression_preference| ApprovedEventPragmaticContextIR {
        schema: APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA.into(),
        subject_node_id: "transfer_diversity_event".into(),
        requested_focus_node_id: requested_focus_node_id.map(str::to_owned),
        backgrounded_node_ids: backgrounded_node_ids
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        recoverable_node_ids: recoverable_node_ids
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        expression_preference,
    };
    let focus_state = |concept_id_hint: &str, misleading_surface: &str| {
        let mut state = DiscourseFocusStateIR::default();
        state.apply_turn(
            1,
            &[DiscourseFocusCandidateIR::explicit_topic(
                misleading_surface,
                Some(concept_id_hint),
            )],
        );
        state
    };

    let event_focus = base
        .apply_event_pragmatic_contexts(
            &DiscourseFocusStateIR::default(),
            0,
            &[context(
                None,
                &[],
                &[],
                ApprovedEventExpressionPreferenceIR::Auto,
            )],
        )
        .unwrap();
    let mut variants = vec![("EVENT_FOCUS", event_focus, BASE.expected)];

    let theme_topic = base
        .apply_event_pragmatic_contexts(
            &focus_state("audit_record_holdout", "이 표면은 목적지라고 주장합니다"),
            1,
            &[context(
                Some("audit_record_holdout"),
                &[],
                &[],
                ApprovedEventExpressionPreferenceIR::Auto,
            )],
        )
        .unwrap();
    variants.push((
        "THEME_TOPIC",
        theme_topic,
        "감사 기록은 지우가 장기 보관소로 전송했습니다.",
    ));

    let concise_passive = base
        .apply_event_pragmatic_contexts(
            &DiscourseFocusStateIR::default(),
            0,
            &[context(
                Some("audit_record_holdout"),
                &["jiu_holdout"],
                &["jiu_holdout"],
                ApprovedEventExpressionPreferenceIR::Concise,
            )],
        )
        .unwrap();
    variants.push((
        "CONCISE_PASSIVE",
        concise_passive,
        "감사 기록이 장기 보관소로 전송됐습니다.",
    ));

    let destination_explicit = base
        .apply_event_pragmatic_contexts(
            &focus_state("archive_vault_holdout", "이 표면은 행위자라고 주장합니다"),
            1,
            &[context(
                Some("archive_vault_holdout"),
                &["jiu_holdout"],
                &["jiu_holdout"],
                ApprovedEventExpressionPreferenceIR::Explicit,
            )],
        )
        .unwrap();
    variants.push((
        "DESTINATION_EXPLICIT",
        destination_explicit,
        "장기 보관소에는 지우가 감사 기록을 전송했습니다.",
    ));

    let mut surfaces = BTreeSet::new();
    let mut frames = Vec::new();
    for (variant, response, expected) in variants {
        let output = assert_verified_surface(&response, expected);
        println!("UNSEEN_DIVERSITY\t{variant}\t{expected}");
        assert!(surfaces.insert(expected));
        frames.push(semantic_frame(&output));
    }
    assert_eq!(surfaces.len(), 4);
    assert!(frames.windows(2).all(|pair| pair[0] == pair[1]));
    assert!(surfaces.iter().all(|surface| !surface.contains("에 의해")));
}

#[test]
fn unseen_patient_and_source_topics_preserve_the_frame_in_both_voices() {
    const PATIENT_CASE: HoldoutCase = HoldoutCase {
        id: "PATIENT_TOPIC",
        event_label: "열쇠 제공 화제",
        class: ApprovedEventRealizationClassIR::Transfer,
        sense: ApprovedEventPredicateSenseIR::Give,
        phase: ApprovedEventPhaseIR::Completed,
        register: LanguageRegisterIR::Formal,
        roles: &[
            role(
                ApprovedRelationTypeIR::Agent,
                "patient_foundation_holdout",
                ApprovedSemanticTypeIR::Organization,
                "재단",
            ),
            role(
                ApprovedRelationTypeIR::Patient,
                "patient_seoyun_holdout",
                ApprovedSemanticTypeIR::Person,
                "서윤",
            ),
            role(
                ApprovedRelationTypeIR::Theme,
                "patient_recovery_key_holdout",
                ApprovedSemanticTypeIR::Concept,
                "복구 열쇠",
            ),
        ],
        expected: "서윤에게는 재단이 복구 열쇠를 제공했습니다.",
    };
    const SOURCE_CASE: HoldoutCase = HoldoutCase {
        id: "SOURCE_TOPIC",
        event_label: "시료 이동 화제",
        class: ApprovedEventRealizationClassIR::Motion,
        sense: ApprovedEventPredicateSenseIR::Move,
        phase: ApprovedEventPhaseIR::Completed,
        register: LanguageRegisterIR::Formal,
        roles: &[
            role(
                ApprovedRelationTypeIR::Agent,
                "source_transport_team_holdout",
                ApprovedSemanticTypeIR::Organization,
                "운반팀",
            ),
            role(
                ApprovedRelationTypeIR::Theme,
                "source_sample_box_holdout",
                ApprovedSemanticTypeIR::Concept,
                "시료 상자",
            ),
            role(
                ApprovedRelationTypeIR::Source,
                "preparation_room_holdout",
                ApprovedSemanticTypeIR::Location,
                "준비실",
            ),
            role(
                ApprovedRelationTypeIR::Destination,
                "source_isolation_zone_holdout",
                ApprovedSemanticTypeIR::Location,
                "격리 구역",
            ),
        ],
        expected: "운반팀이 시료 상자를 준비실에서 격리 구역으로 옮겼습니다.",
    };

    let project = |case: &HoldoutCase,
                   requested_node_id: &str,
                   agent_node_id: &str,
                   preference: ApprovedEventExpressionPreferenceIR| {
        let mut response = event_response(case);
        response.event_realizations[0].perspective = None;
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        let mut focus = DiscourseFocusStateIR::default();
        focus.apply_turn(
            1,
            &[DiscourseFocusCandidateIR::explicit_topic(
                "표면 문자열은 typed identity를 대체하지 않음",
                Some(requested_node_id),
            )],
        );
        response
            .apply_event_pragmatic_contexts(
                &focus,
                1,
                &[ApprovedEventPragmaticContextIR {
                    schema: APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA.into(),
                    subject_node_id: format!("{}_event", case.id.to_ascii_lowercase()),
                    requested_focus_node_id: Some(requested_node_id.into()),
                    backgrounded_node_ids: vec![agent_node_id.into()],
                    recoverable_node_ids: vec![agent_node_id.into()],
                    expression_preference: preference,
                }],
            )
            .unwrap()
    };

    let cases = [
        (
            &PATIENT_CASE,
            "patient_seoyun_holdout",
            "patient_foundation_holdout",
            PATIENT_CASE.expected,
            "서윤에게는 복구 열쇠가 제공됐습니다.",
            "To 서윤, 재단 gave 복구 열쇠.",
            "To 서윤, 복구 열쇠 was given.",
        ),
        (
            &SOURCE_CASE,
            "preparation_room_holdout",
            "source_transport_team_holdout",
            SOURCE_CASE.expected,
            "시료 상자가 준비실에서 격리 구역으로 옮겨졌습니다.",
            "From 준비실, 운반팀 moved 시료 상자 to 격리 구역.",
            "From 준비실, 시료 상자 was moved to 격리 구역.",
        ),
    ];

    for (
        case,
        requested_node_id,
        agent_node_id,
        explicit_korean,
        concise_korean,
        explicit_english,
        concise_english,
    ) in cases
    {
        let explicit = project(
            case,
            requested_node_id,
            agent_node_id,
            ApprovedEventExpressionPreferenceIR::Explicit,
        );
        let concise = project(
            case,
            requested_node_id,
            agent_node_id,
            ApprovedEventExpressionPreferenceIR::Concise,
        );
        assert_eq!(
            explicit.event_realizations[0].voice,
            Some(ApprovedEventVoiceIR::Active)
        );
        assert_eq!(
            concise.event_realizations[0].voice,
            Some(ApprovedEventVoiceIR::Passive)
        );

        let explicit_ko = assert_verified_surface(&explicit, explicit_korean);
        let concise_ko = assert_verified_surface(&concise, concise_korean);
        let explicit_en = realize_document_response(&explicit, LanguageCodeIR::English).unwrap();
        let concise_en = realize_document_response(&concise, LanguageCodeIR::English).unwrap();
        assert!(explicit_en.validate(&explicit));
        assert!(concise_en.validate(&concise));
        assert!(explicit_en.markdown.contains(explicit_english));
        assert!(concise_en.markdown.contains(concise_english));
        assert_eq!(semantic_frame(&explicit_ko), semantic_frame(&concise_ko));
        assert_eq!(semantic_frame(&explicit_ko), semantic_frame(&explicit_en));
        assert_eq!(semantic_frame(&explicit_ko), semantic_frame(&concise_en));
        assert!(!explicit_ko.markdown.contains("에 의해"));

        println!("UNSEEN_ROLE_TOPIC\tEXPLICIT\t{explicit_korean}");
        println!("UNSEEN_ROLE_TOPIC\tCONCISE\t{concise_korean}");
    }
}

#[test]
fn unseen_state_change_can_omit_only_an_approved_recoverable_agent() {
    const CASE: HoldoutCase = HoldoutCase {
        id: "CHANGE_PASSIVE",
        event_label: "구성 상태 변경",
        class: ApprovedEventRealizationClassIR::StateChange,
        sense: ApprovedEventPredicateSenseIR::Change,
        phase: ApprovedEventPhaseIR::Completed,
        register: LanguageRegisterIR::Formal,
        roles: &[
            role(
                ApprovedRelationTypeIR::Agent,
                "coordination_team_holdout",
                ApprovedSemanticTypeIR::Organization,
                "조정팀",
            ),
            role(
                ApprovedRelationTypeIR::Theme,
                "configuration_holdout",
                ApprovedSemanticTypeIR::Concept,
                "구성",
            ),
            role(
                ApprovedRelationTypeIR::InitialState,
                "standby_state_holdout",
                ApprovedSemanticTypeIR::State,
                "대기",
            ),
            role(
                ApprovedRelationTypeIR::ResultState,
                "active_state_holdout",
                ApprovedSemanticTypeIR::State,
                "활성",
            ),
        ],
        expected: "구성이 대기에서 활성으로 변경됐습니다.",
    };
    let mut response = event_response(&CASE);
    let realization = &mut response.event_realizations[0];
    realization.perspective = None;
    realization.voice = Some(ApprovedEventVoiceIR::Passive);
    realization.information_structure = Some(ApprovedEventInformationStructureIR {
        topic: None,
        focus: ApprovedEventInformationRoleIR::Theme,
        omitted_roles: vec![ApprovedRelationTypeIR::Agent],
        recoverable_roles: vec![ApprovedRelationTypeIR::Agent],
    });
    response.semantic_sha256 = compositional_response_sha256(&response);
    assert!(response.validate());

    assert_verified_surface(&response, CASE.expected);
    println!("UNSEEN_EXPRESSION\t{}\t{}", CASE.id, CASE.expected);
}

#[test]
fn unseen_everyday_events_distinguish_polite_and_casual_korean() {
    const CASES: &[HoldoutCase] = &[
        HoldoutCase {
            id: "FAMILY_PHOTO_SEND",
            event_label: "가족 사진 보내기",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Send,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "ara_probe",
                    ApprovedSemanticTypeIR::Person,
                    "아라",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "travel_photo_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "여행 사진",
                ),
                role(
                    ApprovedRelationTypeIR::Destination,
                    "family_chat_probe",
                    ApprovedSemanticTypeIR::Location,
                    "가족 대화방",
                ),
            ],
            expected: "아라가 여행 사진을 가족 대화방으로 보냈어요.",
        },
        HoldoutCase {
            id: "UMBRELLA_GIVE",
            event_label: "우산 건네기",
            class: ApprovedEventRealizationClassIR::Transfer,
            sense: ApprovedEventPredicateSenseIR::Give,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "aunt_probe",
                    ApprovedSemanticTypeIR::Person,
                    "이모",
                ),
                role(
                    ApprovedRelationTypeIR::Patient,
                    "niece_probe",
                    ApprovedSemanticTypeIR::Person,
                    "조카",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "umbrella_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "우산",
                ),
            ],
            expected: "이모가 조카에게 우산을 줬어요.",
        },
        HoldoutCase {
            id: "DINNER_MENU_CREATE",
            event_label: "저녁 메뉴 만들기",
            class: ApprovedEventRealizationClassIR::Creation,
            sense: ApprovedEventPredicateSenseIR::Create,
            phase: ApprovedEventPhaseIR::Ongoing,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "subin_probe",
                    ApprovedSemanticTypeIR::Person,
                    "수빈",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "dinner_menu_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "저녁 메뉴",
                ),
            ],
            expected: "수빈이 저녁 메뉴를 만들고 있어요.",
        },
        HoldoutCase {
            id: "BICYCLE_REPAIR_COMPLETE",
            event_label: "자전거 수리 마치기",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Complete,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "repair_team_probe",
                    ApprovedSemanticTypeIR::Organization,
                    "정비팀",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "bicycle_repair_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "자전거 수리",
                ),
            ],
            expected: "정비팀이 자전거 수리를 마쳤어요.",
        },
        HoldoutCase {
            id: "WINDOW_OPEN",
            event_label: "창문 열기",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Open,
            phase: ApprovedEventPhaseIR::Completed,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "hyeonu_probe",
                    ApprovedSemanticTypeIR::Person,
                    "현우",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "balcony_window_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "베란다 창문",
                ),
            ],
            expected: "현우가 베란다 창문을 열었어요.",
        },
        HoldoutCase {
            id: "BACK_DOOR_CLOSE",
            event_label: "뒷문 닫기",
            class: ApprovedEventRealizationClassIR::StateChange,
            sense: ApprovedEventPredicateSenseIR::Close,
            phase: ApprovedEventPhaseIR::Planned,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "jimin_probe",
                    ApprovedSemanticTypeIR::Person,
                    "지민",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "shop_back_door_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "가게 뒷문",
                ),
            ],
            expected: "지민이 가게 뒷문을 닫을 거예요.",
        },
        HoldoutCase {
            id: "BLANKET_MOVE",
            event_label: "응급 담요 옮기기",
            class: ApprovedEventRealizationClassIR::Motion,
            sense: ApprovedEventPredicateSenseIR::Move,
            phase: ApprovedEventPhaseIR::Planned,
            register: LanguageRegisterIR::Neutral,
            roles: &[
                role(
                    ApprovedRelationTypeIR::Agent,
                    "rescue_team_probe",
                    ApprovedSemanticTypeIR::Organization,
                    "구조대",
                ),
                role(
                    ApprovedRelationTypeIR::Theme,
                    "emergency_blanket_probe",
                    ApprovedSemanticTypeIR::Concept,
                    "응급 담요",
                ),
                role(
                    ApprovedRelationTypeIR::Source,
                    "gym_probe",
                    ApprovedSemanticTypeIR::Location,
                    "체육관",
                ),
                role(
                    ApprovedRelationTypeIR::Destination,
                    "temporary_shelter_probe",
                    ApprovedSemanticTypeIR::Location,
                    "임시 숙소",
                ),
            ],
            expected: "구조대가 응급 담요를 체육관에서 임시 숙소로 옮길 거예요.",
        },
    ];

    const CASUAL: &[&str] = &[
        "아라가 여행 사진을 가족 대화방으로 보냈어.",
        "이모가 조카에게 우산을 줬어.",
        "수빈이 저녁 메뉴를 만들고 있어.",
        "정비팀이 자전거 수리를 마쳤어.",
        "현우가 베란다 창문을 열었어.",
        "지민이 가게 뒷문을 닫을 거야.",
        "구조대가 응급 담요를 체육관에서 임시 숙소로 옮길 거야.",
    ];

    for (case, casual) in CASES.iter().zip(CASUAL) {
        let neutral_response = event_response(case);
        let neutral_output = assert_verified_surface(&neutral_response, case.expected);

        let mut informal = *case;
        informal.register = LanguageRegisterIR::Informal;
        informal.expected = casual;
        let informal_response = event_response(&informal);
        let informal_output = assert_verified_surface(&informal_response, casual);

        assert_eq!(neutral_output.markdown, case.expected);
        assert_eq!(informal_output.markdown, *casual);
        assert_ne!(neutral_output.markdown, informal_output.markdown);
        for output in [&neutral_output, &informal_output] {
            assert!(!output.markdown.contains("##"));
            assert!(!output.markdown.contains("확인된 내용을"));
        }
        println!(
            "COLLOQUIAL_HOLDOUT\t{}\tNeutral={}\tInformal={}",
            case.id, neutral_output.markdown, informal_output.markdown
        );
    }
}
