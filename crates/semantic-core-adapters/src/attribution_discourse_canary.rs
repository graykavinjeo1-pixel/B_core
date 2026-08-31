use semantic_core_adapters::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, AttributionEvidenceKindIR,
    AttributionStanceIR, CognitiveApi, CompositionalSemanticAnalyzer, ConversationInputModalityIR,
    ConversationTurnDispositionIR, ConversationTurnRequestIR, DiscourseBindingKindIR,
    DiscourseReferentKindIR, EpistemicStatusIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

struct AttributionCase {
    case_id: &'static str,
    text: &'static str,
    actor: &'static str,
    content: &'static str,
    attitude: AttributionAttitudeIR,
    stance: AttributionStanceIR,
    status: EpistemicStatusIR,
    evidence: AttributionEvidenceKindIR,
    polarity: AttributedPropositionPolarityIR,
    nested_edges: usize,
}

#[derive(Debug, Serialize)]
struct Row {
    case_id: String,
    graph_valid: bool,
    assertions: usize,
    satisfied: usize,
    attributed_frames_authority_safe: bool,
    dialogue_truth_established: bool,
    diagnostic: String,
    pass: bool,
}

fn check_case(case: AttributionCase) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let graph = &analysis.attribution_graph;
    let edge = graph
        .attributions
        .iter()
        .find(|edge| edge.attitude == case.attitude);
    let actor_matches = edge
        .and_then(|edge| graph.actor(&edge.actor_id))
        .is_some_and(|actor| actor.normalized_label.contains(case.actor));
    let proposition_matches = edge
        .and_then(|edge| graph.proposition(&edge.proposition_id))
        .is_some_and(|proposition| {
            proposition.normalized_text.contains(case.content)
                && proposition.polarity == case.polarity
        });
    let semantics_match = edge.is_some_and(|edge| {
        edge.stance == case.stance
            && edge.epistemic_status == case.status
            && edge.evidence_kind == case.evidence
    });
    let evidence_source_matches = match case.case_id {
        "KO_HEARSAY_SOURCE" => edge
            .and_then(|edge| edge.evidence_source_actor_id.as_deref())
            .and_then(|actor_id| graph.actor(actor_id))
            .is_some_and(|actor| actor.normalized_label.contains("민수")),
        "EN_HEARSAY_SOURCE" => edge
            .and_then(|edge| edge.evidence_source_actor_id.as_deref())
            .and_then(|actor_id| graph.actor(actor_id))
            .is_some_and(|actor| actor.normalized_label.contains("mina")),
        _ => true,
    };
    let nesting_matches = graph
        .attributions
        .iter()
        .filter(|edge| edge.parent_proposition_id.is_some())
        .count()
        == case.nested_edges;
    let attributed_frames_authority_safe = analysis
        .candidates
        .iter()
        .filter(|candidate| graph.attributes_frame(&candidate.source_frame_id))
        .all(|candidate| !candidate.external_execution_authorized);
    let dialogue_truth_established = graph
        .propositions
        .iter()
        .any(|proposition| proposition.dialogue_truth_established);
    let graph_valid = graph.validate();
    let assertions = 7;
    let satisfied = usize::from(actor_matches)
        + usize::from(proposition_matches)
        + usize::from(semantics_match)
        + usize::from(evidence_source_matches)
        + usize::from(nesting_matches)
        + usize::from(attributed_frames_authority_safe)
        + usize::from(!dialogue_truth_established);
    Row {
        case_id: case.case_id.to_string(),
        graph_valid,
        assertions,
        satisfied,
        attributed_frames_authority_safe,
        dialogue_truth_established,
        diagnostic: format!(
            "actor={actor_matches}; proposition={proposition_matches}; semantics={semantics_match}; evidence_source={evidence_source_matches}; nesting={nesting_matches}"
        ),
        pass: graph_valid && satisfied == assertions,
    }
}

fn request(conversation_id: &str, turn: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index: turn,
        request_id: format!("{conversation_id}-{turn}"),
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

fn cross_turn_row(
    case_id: &str,
    first: &str,
    second: &str,
    expected_source: Option<&str>,
    expect_ambiguity: bool,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_response = api
        .process_conversation_turn(&request(case_id, 1, first))
        .expect("first turn");
    let second_response = api
        .process_conversation_turn(&request(case_id, 2, second))
        .expect("second turn");
    let stored = first_response
        .conversation_state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
        .collect::<Vec<_>>();
    let provenance_preserved = expected_source.is_none_or(|source| {
        stored.iter().any(|referent| {
            referent
                .attributed_source
                .as_deref()
                .is_some_and(|stored_source| stored_source.eq_ignore_ascii_case(source))
                && referent.attribution_attitude.is_some()
                && referent.epistemic_status.is_some()
        })
    });
    let binding = second_response
        .reference_resolution
        .discourse_bindings
        .iter()
        .find(|binding| binding.kind == DiscourseBindingKindIR::PropositionReference);
    let resolution_matches = if expect_ambiguity {
        binding.is_none()
            && second_response.disposition == ConversationTurnDispositionIR::ClarificationRequired
    } else {
        binding.is_some() && second_response.disposition == ConversationTurnDispositionIR::Grounded
    };
    let attributed_frames_authority_safe = second_response
        .pragmatic_interpretation
        .compositional_analysis
        .candidates
        .iter()
        .filter(|candidate| {
            second_response
                .pragmatic_interpretation
                .compositional_analysis
                .attribution_graph
                .attributes_frame(&candidate.source_frame_id)
        })
        .all(|candidate| !candidate.external_execution_authorized);
    let quoted_reconstruction = expect_ambiguity
        || second_response
            .reference_resolution
            .resolved_semantic_text
            .contains('‘');
    let assertions = 4;
    let satisfied = usize::from(!stored.is_empty())
        + usize::from(provenance_preserved)
        + usize::from(resolution_matches)
        + usize::from(attributed_frames_authority_safe && quoted_reconstruction);
    Row {
        case_id: case_id.to_string(),
        graph_valid: first_response
            .pragmatic_interpretation
            .compositional_analysis
            .attribution_graph
            .validate(),
        assertions,
        satisfied,
        attributed_frames_authority_safe,
        dialogue_truth_established: false,
        diagnostic: format!(
            "stored={}; provenance={provenance_preserved}; resolution={resolution_matches}; first_disposition={:?}; normalization={:?}/ambiguous={}; nonliteral_clarify={}; composition_clarify={}; first_speech={:?}; first_goal={}; disposition={:?}; resolved={}",
            stored.len(),
            first_response.disposition,
            first_response.normalization.disposition,
            first_response.normalization.ambiguous_input,
            first_response
                .pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required,
            first_response
                .pragmatic_interpretation
                .compositional_analysis
                .clarification_required,
            first_response.pragmatic_interpretation.speech_act,
            first_response.pragmatic_interpretation.inferred_goal.is_some(),
            second_response.disposition,
            second_response.reference_resolution.resolved_semantic_text
        ),
        pass: satisfied == assertions,
    }
}

fn main() {
    let cases = [
        AttributionCase {
            case_id: "KO_REPORTED_CLAUSE",
            text: "민수는 서버가 멈췄다고 말했다.",
            actor: "민수",
            content: "서버가 멈췄",
            attitude: AttributionAttitudeIR::Say,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Reported,
            evidence: AttributionEvidenceKindIR::Speech,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_NEGATIVE_PROPOSITION_CLAIM",
            text: "지수는 배포가 끝나지 않았다고 주장했다.",
            actor: "지수",
            content: "배포가 끝나지 않았",
            attitude: AttributionAttitudeIR::Claim,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Claimed,
            evidence: AttributionEvidenceKindIR::Speech,
            polarity: AttributedPropositionPolarityIR::Negative,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_NEGATED_BELIEF",
            text: "영희는 결과가 정확하다고 믿지 않는다.",
            actor: "영희",
            content: "결과가 정확하",
            attitude: AttributionAttitudeIR::Believe,
            stance: AttributionStanceIR::Rejects,
            status: EpistemicStatusIR::Denied,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_DENIAL",
            text: "철수는 파일이 삭제됐다고 부인했다.",
            actor: "철수",
            content: "파일이 삭제됐",
            attitude: AttributionAttitudeIR::Deny,
            stance: AttributionStanceIR::Rejects,
            status: EpistemicStatusIR::Denied,
            evidence: AttributionEvidenceKindIR::Speech,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_HEARSAY_SOURCE",
            text: "나는 민수에게서 서버가 멈췄다고 들었다.",
            actor: "나",
            content: "서버가 멈췄",
            attitude: AttributionAttitudeIR::Hear,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Hearsay,
            evidence: AttributionEvidenceKindIR::Hearsay,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_OBSERVATION",
            text: "수진은 큐가 비었다고 관찰했다.",
            actor: "수진",
            content: "큐가 비었",
            attitude: AttributionAttitudeIR::Observe,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Observed,
            evidence: AttributionEvidenceKindIR::DirectObservation,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_INFERENCE",
            text: "분석가는 지연이 증가했다고 추론했다.",
            actor: "분석가",
            content: "지연이 증가했",
            attitude: AttributionAttitudeIR::Infer,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Inferred,
            evidence: AttributionEvidenceKindIR::Inference,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_DESIRED_ACTION",
            text: "관리자는 파일을 삭제하기를 원한다.",
            actor: "관리자",
            content: "파일을 삭제하기를",
            attitude: AttributionAttitudeIR::Want,
            stance: AttributionStanceIR::Desires,
            status: EpistemicStatusIR::Desired,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_EXPECTATION",
            text: "운영자는 배포가 끝난다고 예상한다.",
            actor: "운영자",
            content: "배포가 끝난",
            attitude: AttributionAttitudeIR::Expect,
            stance: AttributionStanceIR::Predicts,
            status: EpistemicStatusIR::Expected,
            evidence: AttributionEvidenceKindIR::Inference,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_DOCUMENT_SOURCE",
            text: "감사 보고서에 따르면 캐시는 손상됐다.",
            actor: "보고서",
            content: "캐시는 손상됐다",
            attitude: AttributionAttitudeIR::Report,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Reported,
            evidence: AttributionEvidenceKindIR::Document,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "KO_NESTED_BELIEF",
            text: "영희는 민수가 서버가 멈췄다고 믿는다고 말했다.",
            actor: "민수",
            content: "서버가 멈췄",
            attitude: AttributionAttitudeIR::Believe,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Believed,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 1,
        },
        AttributionCase {
            case_id: "EN_REPORTED_CLAUSE",
            text: "Alice says that the server is down.",
            actor: "alice",
            content: "the server is down",
            attitude: AttributionAttitudeIR::Say,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Reported,
            evidence: AttributionEvidenceKindIR::Speech,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_NEGATED_BELIEF",
            text: "Alice does not believe that the server is down.",
            actor: "alice",
            content: "the server is down",
            attitude: AttributionAttitudeIR::Believe,
            stance: AttributionStanceIR::Rejects,
            status: EpistemicStatusIR::Denied,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_DENIAL",
            text: "Bob denies that the file was deleted.",
            actor: "bob",
            content: "the file was deleted",
            attitude: AttributionAttitudeIR::Deny,
            stance: AttributionStanceIR::Rejects,
            status: EpistemicStatusIR::Denied,
            evidence: AttributionEvidenceKindIR::Speech,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_HEARSAY_SOURCE",
            text: "I heard from Mina that the cache is corrupt.",
            actor: "i",
            content: "the cache is corrupt",
            attitude: AttributionAttitudeIR::Hear,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Hearsay,
            evidence: AttributionEvidenceKindIR::Hearsay,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_OBSERVATION",
            text: "The monitor observed that the queue was empty.",
            actor: "monitor",
            content: "the queue was empty",
            attitude: AttributionAttitudeIR::Observe,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Observed,
            evidence: AttributionEvidenceKindIR::DirectObservation,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_INFERENCE",
            text: "The analyst inferred that latency increased.",
            actor: "analyst",
            content: "latency increased",
            attitude: AttributionAttitudeIR::Infer,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Inferred,
            evidence: AttributionEvidenceKindIR::Inference,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_DESIRED_ACTION",
            text: "The operator wants to delete the file.",
            actor: "operator",
            content: "to delete the file",
            attitude: AttributionAttitudeIR::Want,
            stance: AttributionStanceIR::Desires,
            status: EpistemicStatusIR::Desired,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_EXPECTATION",
            text: "The operator expects deployment to finish.",
            actor: "operator",
            content: "deployment to finish",
            attitude: AttributionAttitudeIR::Expect,
            stance: AttributionStanceIR::Predicts,
            status: EpistemicStatusIR::Expected,
            evidence: AttributionEvidenceKindIR::Inference,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_OPEN_VOCABULARY_INFINITIVE_DESIRE",
            text: "Alice wants to leave.",
            actor: "alice",
            content: "to leave",
            attitude: AttributionAttitudeIR::Want,
            stance: AttributionStanceIR::Desires,
            status: EpistemicStatusIR::Desired,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_DOCUMENT_SOURCE",
            text: "According to the audit, the cache is corrupt.",
            actor: "audit",
            content: "the cache is corrupt",
            attitude: AttributionAttitudeIR::Report,
            stance: AttributionStanceIR::Withholds,
            status: EpistemicStatusIR::Reported,
            evidence: AttributionEvidenceKindIR::Document,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 0,
        },
        AttributionCase {
            case_id: "EN_NESTED_BELIEF",
            text: "Alice says Bob believes that the server is down.",
            actor: "bob",
            content: "the server is down",
            attitude: AttributionAttitudeIR::Believe,
            stance: AttributionStanceIR::Endorses,
            status: EpistemicStatusIR::Believed,
            evidence: AttributionEvidenceKindIR::Unspecified,
            polarity: AttributedPropositionPolarityIR::Positive,
            nested_edges: 1,
        },
    ];

    let mut rows = cases.into_iter().map(check_case).collect::<Vec<_>>();
    rows.push(cross_turn_row(
        "KO_SOURCE_SPECIFIC_CLAIM",
        "민수는 서버가 멈췄다고 주장했다.",
        "민수의 주장을 검토해",
        Some("민수"),
        false,
    ));
    rows.push(cross_turn_row(
        "EN_SOURCE_SPECIFIC_NESTED_BELIEF",
        "Alice says Bob believes that the server is down.",
        "explain Bob's belief",
        Some("Bob"),
        false,
    ));
    rows.push(cross_turn_row(
        "EN_GENERIC_NESTED_AMBIGUITY",
        "Alice says Bob believes that the server is down.",
        "explain that claim",
        None,
        true,
    ));
    rows.push(cross_turn_row(
        "EN_REPORTED_COMMAND_REFERENCE",
        "Alice said delete the file.",
        "explain that statement",
        Some("Alice"),
        false,
    ));

    for (case_id, text, graph_expected, direct_authority_expected) in [
        ("SAFETY_DIRECT_COMMAND", "delete the file", false, true),
        ("SAFETY_REPORT_NOUN", "review the report", false, true),
        ("SAFETY_DIRECT_FACT", "the server is down", false, false),
        (
            "SAFETY_EN_REPORTED_OBJECT",
            "the agent reported the issue",
            false,
            false,
        ),
        (
            "SAFETY_KO_REPORTED_OBJECT",
            "사용자는 결과를 보고했다",
            false,
            false,
        ),
        (
            "SAFETY_QUOTED_REPORTED_COMMAND",
            "Alice said ‘delete the file’",
            true,
            false,
        ),
    ] {
        let analysis = CompositionalSemanticAnalyzer.analyze(text);
        let graph_present = !analysis.attribution_graph.attributions.is_empty();
        let direct_authority = analysis
            .candidates
            .iter()
            .any(|candidate| candidate.external_execution_authorized);
        let satisfied = usize::from(graph_present == graph_expected)
            + usize::from(direct_authority == direct_authority_expected);
        rows.push(Row {
            case_id: case_id.to_string(),
            graph_valid: analysis.attribution_graph.validate(),
            assertions: 2,
            satisfied,
            attributed_frames_authority_safe: analysis
                .candidates
                .iter()
                .filter(|candidate| {
                    analysis
                        .attribution_graph
                        .attributes_frame(&candidate.source_frame_id)
                })
                .all(|candidate| !candidate.external_execution_authorized),
            dialogue_truth_established: false,
            diagnostic: format!(
                "graph_present={graph_present}; direct_authority={direct_authority}"
            ),
            pass: analysis.attribution_graph.validate() && satisfied == 2,
        });
    }

    let scoped =
        CompositionalSemanticAnalyzer.analyze("Alice said delete the file, but now inspect logs");
    let delete = scoped
        .candidates
        .iter()
        .find(|candidate| candidate.subject.contains("file"));
    let inspect = scoped
        .candidates
        .iter()
        .find(|candidate| candidate.subject.contains("logs"));
    let attributed_delete_blocked = delete.is_some_and(|candidate| {
        scoped
            .attribution_graph
            .attributes_frame(&candidate.source_frame_id)
            && !candidate.external_execution_authorized
    });
    let outer_inspect_preserved = inspect.is_some_and(|candidate| {
        !scoped
            .attribution_graph
            .attributes_frame(&candidate.source_frame_id)
            && candidate.external_execution_authorized
    });
    rows.push(Row {
        case_id: "SAFETY_ATTRIBUTION_CONTRAST_BOUNDARY".to_string(),
        graph_valid: scoped.attribution_graph.validate(),
        assertions: 2,
        satisfied: usize::from(attributed_delete_blocked)
            + usize::from(outer_inspect_preserved),
        attributed_frames_authority_safe: attributed_delete_blocked,
        dialogue_truth_established: false,
        diagnostic: format!(
            "attributed_delete_blocked={attributed_delete_blocked}; outer_inspect_preserved={outer_inspect_preserved}; candidates={:?}",
            scoped
                .candidates
                .iter()
                .map(|candidate| (
                    candidate.subject.as_str(),
                    candidate.source_frame_id.as_str(),
                    candidate.disposition,
                    candidate.external_execution_authorized
                ))
                .collect::<Vec<_>>()
        ),
        pass: scoped.attribution_graph.validate()
            && attributed_delete_blocked
            && outer_inspect_preserved,
    });

    let korean_scoped = CompositionalSemanticAnalyzer
        .analyze("민수는 파일을 삭제하라고 말했지만 이제 로그를 확인해");
    let korean_delete = korean_scoped
        .candidates
        .iter()
        .find(|candidate| candidate.subject.contains("파일"));
    let korean_check = korean_scoped
        .candidates
        .iter()
        .find(|candidate| candidate.subject.contains("로그"));
    let korean_delete_blocked = korean_delete.is_some_and(|candidate| {
        korean_scoped
            .attribution_graph
            .attributes_frame(&candidate.source_frame_id)
            && !candidate.external_execution_authorized
    });
    let korean_outer_preserved = korean_check.is_some_and(|candidate| {
        !korean_scoped
            .attribution_graph
            .attributes_frame(&candidate.source_frame_id)
            && candidate.external_execution_authorized
    });
    rows.push(Row {
        case_id: "SAFETY_KO_ATTRIBUTION_CONTRAST_BOUNDARY".to_string(),
        graph_valid: korean_scoped.attribution_graph.validate(),
        assertions: 2,
        satisfied: usize::from(korean_delete_blocked)
            + usize::from(korean_outer_preserved),
        attributed_frames_authority_safe: korean_delete_blocked,
        dialogue_truth_established: false,
        diagnostic: format!(
            "attributed_delete_blocked={korean_delete_blocked}; outer_check_preserved={korean_outer_preserved}"
        ),
        pass: korean_scoped.attribution_graph.validate()
            && korean_delete_blocked
            && korean_outer_preserved,
    });

    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
