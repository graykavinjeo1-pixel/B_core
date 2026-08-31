use semantic_core_adapters::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, BeliefRecordStatusIR,
    BeliefRevisionKindIR, CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseBindingKindIR, DiscourseReferentKindIR, EpistemicLedgerIR,
    EpistemicObservationIR, EpistemicStatusIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Row {
    case_id: String,
    records: usize,
    revisions: usize,
    active_propositions: usize,
    ledger_valid: bool,
    authority_safe: bool,
    assertions: usize,
    satisfied: usize,
    diagnostic: String,
    pass: bool,
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

fn scenario_row(
    case_id: &str,
    turns: &[&str],
    expected_statuses: &[BeliefRecordStatusIR],
    required_revisions: &[BeliefRevisionKindIR],
    expected_active_propositions: usize,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut response = None;
    let mut turn_record_counts = Vec::new();
    let mut turn_attribution_counts = Vec::new();
    let mut turn_goals = Vec::new();
    let mut turn_speech_acts = Vec::new();
    for (index, text) in turns.iter().enumerate() {
        let current = api
            .process_conversation_turn(&request(
                case_id,
                u64::try_from(index + 1).expect("turn index"),
                text,
            ))
            .expect("conversation turn");
        turn_record_counts.push(current.conversation_state.epistemic_ledger.records.len());
        turn_attribution_counts.push(
            current
                .pragmatic_interpretation
                .compositional_analysis
                .attribution_graph
                .attributions
                .len(),
        );
        turn_goals.push(current.pragmatic_interpretation.inferred_goal.is_some());
        turn_speech_acts.push(current.pragmatic_interpretation.speech_act);
        response = Some(current);
    }
    let response = response.expect("final response");
    let ledger = &response.conversation_state.epistemic_ledger;
    let statuses = ledger
        .records
        .iter()
        .map(|record| record.status)
        .collect::<Vec<_>>();
    let status_match = statuses == expected_statuses;
    let revision_match = required_revisions.iter().all(|required| {
        ledger
            .revisions
            .iter()
            .any(|revision| revision.kind == *required)
    });
    let active_propositions = response
        .conversation_state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
        .count();
    let active_match = active_propositions == expected_active_propositions;
    let authority_safe =
        ledger.records.iter().all(|record| {
            !record.dialogue_truth_established && !record.external_execution_authorized
        }) && response
            .conversation_state
            .active_discourse_referents
            .iter()
            .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
            .all(|referent| !referent.external_execution_authorized);
    let ledger_valid = ledger.validate(response.conversation_state.completed_turns);
    let assertions = 5;
    let satisfied = usize::from(ledger.records.len() == expected_statuses.len())
        + usize::from(status_match)
        + usize::from(revision_match)
        + usize::from(active_match)
        + usize::from(authority_safe);
    Row {
        case_id: case_id.to_string(),
        records: ledger.records.len(),
        revisions: ledger.revisions.len(),
        active_propositions,
        ledger_valid,
        authority_safe,
        assertions,
        satisfied,
        diagnostic: format!(
            "statuses={statuses:?}; required_revisions={required_revisions:?}; disposition={:?}; turn_records={turn_record_counts:?}; turn_attributions={turn_attribution_counts:?}; turn_goals={turn_goals:?}; turn_speech={turn_speech_acts:?}; records={:?}",
            response.disposition,
            ledger
                .records
                .iter()
                .map(|record| (
                    record.source_actor.as_str(),
                    record.proposition_surface.as_str(),
                    record.signature.subject_key.as_str(),
                    record.signature.state_axis.as_deref(),
                    record.signature.state_value
                ))
                .collect::<Vec<_>>()
        ),
        pass: ledger_valid && satisfied == assertions,
    }
}

fn reference_row(
    case_id: &str,
    turns: &[&str],
    expected_source_fragment: &str,
    expect_resolution: bool,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut response = None;
    for (index, text) in turns.iter().enumerate() {
        response = Some(
            api.process_conversation_turn(&request(
                case_id,
                u64::try_from(index + 1).expect("turn index"),
                text,
            ))
            .expect("conversation turn"),
        );
    }
    let response = response.expect("final response");
    let ledger = &response.conversation_state.epistemic_ledger;
    let binding = response
        .reference_resolution
        .discourse_bindings
        .iter()
        .find(|binding| binding.kind == DiscourseBindingKindIR::PropositionReference);
    let resolution_match = if expect_resolution {
        binding.is_some()
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && response
                .reference_resolution
                .resolved_semantic_text
                .to_lowercase()
                .contains(&expected_source_fragment.to_lowercase())
    } else {
        binding.is_none()
            && response.disposition == ConversationTurnDispositionIR::ClarificationRequired
    };
    let authority_safe = ledger
        .records
        .iter()
        .all(|record| !record.dialogue_truth_established && !record.external_execution_authorized);
    let ledger_valid = ledger.validate(response.conversation_state.completed_turns);
    let assertions = 3;
    let satisfied =
        usize::from(resolution_match) + usize::from(authority_safe) + usize::from(ledger_valid);
    Row {
        case_id: case_id.to_string(),
        records: ledger.records.len(),
        revisions: ledger.revisions.len(),
        active_propositions: response
            .conversation_state
            .active_discourse_referents
            .iter()
            .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
            .count(),
        ledger_valid,
        authority_safe,
        assertions,
        satisfied,
        diagnostic: format!(
            "binding={:?}; disposition={:?}; resolved={}",
            binding.map(|binding| &binding.referent_ids),
            response.disposition,
            response.reference_resolution.resolved_semantic_text
        ),
        pass: satisfied == assertions,
    }
}

fn direct_ledger_row(case_id: &str, ledger: &EpistemicLedgerIR, completed_turns: u64) -> Row {
    let ledger_valid = ledger.validate(completed_turns);
    let authority_safe = ledger
        .records
        .iter()
        .all(|record| !record.dialogue_truth_established && !record.external_execution_authorized);
    Row {
        case_id: case_id.to_string(),
        records: ledger.records.len(),
        revisions: ledger.revisions.len(),
        active_propositions: ledger
            .records
            .iter()
            .filter(|record| record.status.is_reference_active())
            .count(),
        ledger_valid,
        authority_safe,
        assertions: 3,
        satisfied: usize::from(ledger_valid)
            + usize::from(authority_safe)
            + usize::from(ledger.records.len() <= 64),
        diagnostic: format!(
            "records={}; revisions={}",
            ledger.records.len(),
            ledger.revisions.len()
        ),
        pass: ledger_valid && authority_safe && ledger.records.len() <= 64,
    }
}

fn main() {
    use BeliefRecordStatusIR::{Active, Contested, Retracted, Superseded};
    use BeliefRevisionKindIR::{Contradicts, Reaffirms, Retracts, Supersedes};

    let mut rows = vec![
        scenario_row(
            "EN_SAME_SOURCE_NOW_SUPERSEDES",
            &[
                "Alice says that the server is down",
                "Alice now says that the server is up",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "KO_SAME_SOURCE_NOW_SUPERSEDES",
            &[
                "민수는 서버가 멈췄다고 말했다",
                "민수는 이제 서버가 정상이라고 말했다",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "EN_EXPLICIT_CORRECTION",
            &[
                "Alice claims that the cache is corrupt",
                "Alice corrected that the cache is healthy",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "KO_EXPLICIT_CORRECTION",
            &[
                "민수는 캐시가 손상됐다고 주장했다",
                "민수는 캐시가 유효하다고 정정했다",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "EN_NEGATED_COMPLETION_UPDATE",
            &[
                "Alice says that deployment did not finish",
                "Alice now says that deployment finished",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "KO_ENABLEMENT_UPDATE",
            &[
                "민수는 기능이 비활성이라고 말했다",
                "민수는 이제 기능이 활성이라고 말했다",
            ],
            &[Superseded, Active],
            &[Contradicts, Supersedes],
            1,
        ),
        scenario_row(
            "EN_EXPLICIT_RETRACTION",
            &[
                "Alice claims that the cache is corrupt",
                "Alice retracts that claim",
            ],
            &[Retracted],
            &[Retracts],
            0,
        ),
        scenario_row(
            "KO_EXPLICIT_RETRACTION",
            &[
                "민수는 캐시가 손상됐다고 주장했다",
                "민수는 그 주장을 철회한다",
            ],
            &[Retracted],
            &[Retracts],
            0,
        ),
        scenario_row(
            "EN_CROSS_SOURCE_CONFLICT",
            &[
                "Alice says that the server is down",
                "Bob says that the server is up",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "KO_CROSS_SOURCE_CONFLICT",
            &[
                "민수는 빌드가 실패했다고 말했다",
                "지수는 빌드가 성공했다고 말했다",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "EN_RETRACTION_RESOLVES_CROSS_SOURCE_CONFLICT",
            &[
                "Alice says that the server is down",
                "Bob says that the server is up",
                "Bob retracts that claim",
            ],
            &[Active, Retracted],
            &[Contradicts, Retracts],
            1,
        ),
        scenario_row(
            "KO_RETRACTION_RESOLVES_CROSS_SOURCE_CONFLICT",
            &[
                "민수는 빌드가 실패했다고 말했다",
                "지수는 빌드가 성공했다고 말했다",
                "지수는 그 주장을 철회한다",
            ],
            &[Active, Retracted],
            &[Contradicts, Retracts],
            1,
        ),
        scenario_row(
            "EN_UNMARKED_SELF_CONFLICT",
            &[
                "Alice says that the server is down",
                "Alice says that the server is up",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "KO_UNMARKED_SELF_CONFLICT",
            &[
                "민수는 빌드가 실패했다고 말했다",
                "민수는 빌드가 성공했다고 말했다",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "EN_REAFFIRMATION",
            &[
                "Alice says that the server is down",
                "Alice says that the server is down",
            ],
            &[Superseded, Active],
            &[Reaffirms],
            1,
        ),
        scenario_row(
            "KO_REAFFIRMATION",
            &[
                "민수는 빌드가 실패했다고 말했다",
                "민수는 빌드가 실패했다고 말했다",
            ],
            &[Superseded, Active],
            &[Reaffirms],
            1,
        ),
        scenario_row(
            "EN_UNRELATED_TOPICS_STAY_ACTIVE",
            &[
                "Alice says that the server is down",
                "Alice says that the cache is corrupt",
            ],
            &[Active, Active],
            &[],
            2,
        ),
        scenario_row(
            "KO_UNRELATED_TOPICS_STAY_ACTIVE",
            &[
                "민수는 서버가 멈췄다고 말했다",
                "민수는 캐시가 손상됐다고 말했다",
            ],
            &[Active, Active],
            &[],
            2,
        ),
        scenario_row(
            "EN_DISTINCT_TEMPORAL_STATES_DO_NOT_CONFLICT",
            &[
                "Alice says that yesterday the server was down",
                "Alice says that today the server is up",
            ],
            &[Active, Active],
            &[],
            2,
        ),
        scenario_row(
            "KO_DISTINCT_TEMPORAL_STATES_DO_NOT_CONFLICT",
            &[
                "민수는 어제 서버가 멈췄다고 말했다",
                "민수는 오늘 서버가 정상이라고 말했다",
            ],
            &[Active, Active],
            &[],
            2,
        ),
        scenario_row(
            "EN_DOCUMENT_PERSON_CONFLICT",
            &[
                "According to the audit, the cache is corrupt",
                "Alice says that the cache is healthy",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "KO_DOCUMENT_PERSON_CONFLICT",
            &[
                "감사 보고서에 따르면 캐시는 손상됐다",
                "민수는 캐시가 유효하다고 말했다",
            ],
            &[Contested, Contested],
            &[Contradicts],
            2,
        ),
        scenario_row(
            "EN_UNKNOWN_STATE_EXPLICIT_CORRECTION",
            &[
                "Alice claims that the rollout is frobnicated",
                "Alice corrected that the rollout is stable",
            ],
            &[Superseded, Active],
            &[Supersedes],
            1,
        ),
        scenario_row(
            "SAFETY_ATTRIBUTED_COMMAND_LEDGER",
            &["Alice said delete the file"],
            &[Active],
            &[],
            1,
        ),
        scenario_row(
            "SAFETY_DIRECT_COMMAND_NO_BELIEF",
            &["delete the file"],
            &[],
            &[],
            0,
        ),
    ];

    rows.push(reference_row(
        "EN_SOURCE_SPECIFIC_OLDER_CLAIM",
        &[
            "Alice says that the server is down",
            "Bob says that the server is up",
            "explain Alice's claim",
        ],
        "alice",
        true,
    ));
    rows.push(reference_row(
        "KO_SOURCE_SPECIFIC_OLDER_CLAIM",
        &[
            "민수는 빌드가 실패했다고 말했다",
            "지수는 빌드가 성공했다고 말했다",
            "민수의 주장을 설명해",
        ],
        "민수",
        true,
    ));
    rows.push(reference_row(
        "EN_RETRACTED_CLAIM_UNAVAILABLE",
        &[
            "Alice claims that the cache is corrupt",
            "Alice retracts that claim",
            "explain that claim",
        ],
        "alice",
        false,
    ));
    rows.push(reference_row(
        "KO_RETRACTED_CLAIM_UNAVAILABLE",
        &[
            "민수는 캐시가 손상됐다고 주장했다",
            "민수는 그 주장을 철회한다",
            "그 주장을 설명해",
        ],
        "민수",
        false,
    ));

    let mut bounded = EpistemicLedgerIR::default();
    for turn in 1..=70_u64 {
        bounded.apply_turn(
            turn,
            "independent observation",
            &[],
            &[EpistemicObservationIR {
                origin_referent_id: format!("P-{turn:03}"),
                source_actor: format!("SOURCE-{turn:03}"),
                proposition_surface: format!("entity-{turn:03} has property-{turn:03}"),
                proposition_polarity: AttributedPropositionPolarityIR::Positive,
                modal_world: semantic_core_adapters::ModalWorldIR::Actual,
                attribution_attitude: AttributionAttitudeIR::Say,
                epistemic_status: EpistemicStatusIR::Reported,
            }],
        );
    }
    rows.push(direct_ledger_row("SAFETY_BOUNDED_LEDGER", &bounded, 70));

    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
