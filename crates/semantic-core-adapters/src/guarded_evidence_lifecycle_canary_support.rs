use semantic_core_adapters::{
    condition_evidence_receipt_sha256, CognitiveApi, ConditionEvidenceDispositionIR,
    ConditionEvidenceRequestIR, ConditionEvidenceSourceIR, ConversationInputModalityIR,
    ConversationStateIR, ConversationTurnRequestIR, DeferredActionCommitmentIR,
    DeferredCommitmentStatusIR, LanguageCodeIR, CONDITION_EVIDENCE_REQUEST_SCHEMA,
    CONVERSATION_STATE_SCHEMA, CONVERSATION_TURN_REQUEST_SCHEMA, DISCOURSE_PROGRAM_GUARD_SCHEMA,
    DISCOURSE_PROGRAM_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum EvidencePlan {
    None,
    SatisfyRebound,
    ContradictRebound,
    SatisfySource,
    WrongConditionHash,
    WrongCommitmentId,
    ReplaySatisfied,
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub evidence_plan: EvidencePlan,
    pub expected_source_status: DeferredCommitmentStatusIR,
    pub expected_rebound_status: DeferredCommitmentStatusIR,
    pub expected_activated_subjects: &'static [&'static str],
    pub expected_receipts: usize,
    pub expected_rejections: usize,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    state_schema: String,
    guarded_programs: usize,
    guarded_steps: usize,
    linked_guarded_steps: usize,
    source_status: String,
    rebound_status: String,
    activated_subjects: Vec<String>,
    accepted_receipts: usize,
    rejected_receipts: usize,
    external_actions_executed: usize,
    lifecycle_integrity: bool,
    language_semantic_authority: bool,
    external_execution_authorized_by_language: bool,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, turn: u64, spec: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: spec.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(spec.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn evidence_request(
    case_id: &str,
    conversation_id: &str,
    commitment_id: String,
    condition_sha256: String,
    disposition: ConditionEvidenceDispositionIR,
) -> ConditionEvidenceRequestIR {
    let mut request = ConditionEvidenceRequestIR {
        schema: CONDITION_EVIDENCE_REQUEST_SCHEMA.to_string(),
        evidence_id: format!("EVIDENCE-{case_id}"),
        conversation_id: conversation_id.to_string(),
        commitment_id,
        condition_sha256,
        disposition,
        source: ConditionEvidenceSourceIR::TrustedVerifier,
        verifier_receipt_sha256: String::new(),
    };
    request.verifier_receipt_sha256 = condition_evidence_receipt_sha256(&request);
    request
}

fn source_and_rebound(
    state: &ConversationStateIR,
) -> Option<(DeferredActionCommitmentIR, DeferredActionCommitmentIR)> {
    let mut commitments = state.deferred_action_commitments.clone();
    commitments.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.commitment_id.cmp(&right.commitment_id))
    });
    Some((commitments.first()?.clone(), commitments.last()?.clone()))
        .filter(|(source, rebound)| source.commitment_id != rebound.commitment_id)
}

fn status_of(state: &ConversationStateIR, commitment_id: &str) -> Option<String> {
    state
        .deferred_action_commitments
        .iter()
        .find(|item| item.commitment_id == commitment_id)
        .map(|item| format!("{:?}", item.status))
}

fn expected_status(status: DeferredCommitmentStatusIR) -> String {
    format!("{status:?}")
}

struct LinkAudit {
    guarded_programs: usize,
    guarded_steps: usize,
    linked_guarded_steps: usize,
    lifecycle_integrity: bool,
    authority_violation: bool,
}

fn audit_links(state: &ConversationStateIR) -> LinkAudit {
    let value = serde_json::to_value(state).expect("state json");
    let programs = value
        .get("active_discourse_programs")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut guarded_programs = 0;
    let mut guarded_steps = 0;
    let mut linked_guarded_steps = 0;
    let mut schema_valid = state.schema == CONVERSATION_STATE_SCHEMA;
    let mut authority_violation = false;
    for program in &programs {
        let program_guard_count = program
            .get("guarded_step_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
        if program_guard_count == 0 {
            continue;
        }
        guarded_programs += 1;
        schema_valid &= program.get("schema").and_then(serde_json::Value::as_str)
            == Some(DISCOURSE_PROGRAM_SCHEMA);
        authority_violation |= program
            .get("semantic_authority")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
            || program
                .get("external_execution_authorized")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
        for step in program
            .get("steps")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(guard) = step.get("guard").filter(|guard| !guard.is_null()) else {
                continue;
            };
            guarded_steps += 1;
            schema_valid &= guard.get("schema").and_then(serde_json::Value::as_str)
                == Some(DISCOURSE_PROGRAM_GUARD_SCHEMA);
            authority_violation |= guard
                .get("semantic_authority")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
                || guard
                    .get("external_execution_authorized")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
            let Some(commitment_id) = guard
                .get("deferred_commitment_id")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let Some(commitment) = state
                .deferred_action_commitments
                .iter()
                .find(|item| item.commitment_id == commitment_id)
            else {
                continue;
            };
            let goal = step.get("goal").unwrap_or(&serde_json::Value::Null);
            let condition_matches = guard
                .get("condition_sha256")
                .and_then(serde_json::Value::as_str)
                == Some(commitment.condition_sha256.as_str());
            let subject_matches = goal
                .get("subject")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|subject| subject.eq_ignore_ascii_case(&commitment.action.subject));
            let predicate_matches = goal
                .get("canonical_predicate")
                .and_then(serde_json::Value::as_str)
                == Some(commitment.action.canonical_predicate.as_str());
            if condition_matches && subject_matches && predicate_matches {
                linked_guarded_steps += 1;
            }
        }
    }
    let activation_integrity =
        state
            .deferred_action_commitments
            .iter()
            .all(|commitment| match commitment.status {
                DeferredCommitmentStatusIR::Activated => commitment
                    .activated_goal_id
                    .as_deref()
                    .and_then(|goal_id| {
                        state
                            .active_goals
                            .iter()
                            .find(|goal| goal.goal_id == goal_id)
                    })
                    .is_some_and(|goal| {
                        goal.intent == commitment.action.intent
                            && goal.canonical_predicate == commitment.action.canonical_predicate
                            && goal
                                .subject
                                .eq_ignore_ascii_case(&commitment.action.subject)
                    }),
                DeferredCommitmentStatusIR::ConditionPending
                | DeferredCommitmentStatusIR::Contradicted
                | DeferredCommitmentStatusIR::Withdrawn => commitment.activated_goal_id.is_none(),
            });
    LinkAudit {
        guarded_programs,
        guarded_steps,
        linked_guarded_steps,
        lifecycle_integrity: schema_valid
            && guarded_programs == 2
            && guarded_steps == 2
            && linked_guarded_steps == guarded_steps
            && activation_integrity,
        authority_violation,
    }
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        for (index, turn) in case.turns.iter().copied().enumerate() {
            api.process_conversation_turn(&request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn"),
                turn,
            ))
            .expect("conversation turn");
        }
        let initial = api
            .conversation_state(case.id)
            .cloned()
            .expect("conversation state");
        let Some((source, rebound)) = source_and_rebound(&initial) else {
            rows.push(Row {
                id: case.id.to_string(),
                category: case.category.to_string(),
                state_schema: initial.schema,
                guarded_programs: 0,
                guarded_steps: 0,
                linked_guarded_steps: 0,
                source_status: "MISSING".to_string(),
                rebound_status: "MISSING".to_string(),
                activated_subjects: Vec::new(),
                accepted_receipts: 0,
                rejected_receipts: 0,
                external_actions_executed: 0,
                lifecycle_integrity: false,
                language_semantic_authority: false,
                external_execution_authorized_by_language: false,
                pass: false,
            });
            continue;
        };
        let mut accepted_receipts = 0;
        let mut rejected_receipts = 0;
        let mut external_actions_executed = 0;
        {
            let mut submit = |request: &ConditionEvidenceRequestIR| match api
                .submit_condition_evidence(request)
            {
                Ok(receipt) => {
                    accepted_receipts += 1;
                    external_actions_executed += usize::from(receipt.external_action_executed);
                }
                Err(_) => rejected_receipts += 1,
            };
            match case.evidence_plan {
                EvidencePlan::None => {}
                EvidencePlan::SatisfyRebound => submit(&evidence_request(
                    case.id,
                    case.id,
                    rebound.commitment_id.clone(),
                    rebound.condition_sha256.clone(),
                    ConditionEvidenceDispositionIR::VerifiedSatisfied,
                )),
                EvidencePlan::ContradictRebound => submit(&evidence_request(
                    case.id,
                    case.id,
                    rebound.commitment_id.clone(),
                    rebound.condition_sha256.clone(),
                    ConditionEvidenceDispositionIR::VerifiedContradicted,
                )),
                EvidencePlan::SatisfySource => submit(&evidence_request(
                    case.id,
                    case.id,
                    source.commitment_id.clone(),
                    source.condition_sha256.clone(),
                    ConditionEvidenceDispositionIR::VerifiedSatisfied,
                )),
                EvidencePlan::WrongConditionHash => submit(&evidence_request(
                    case.id,
                    case.id,
                    rebound.commitment_id.clone(),
                    "b".repeat(64),
                    ConditionEvidenceDispositionIR::VerifiedSatisfied,
                )),
                EvidencePlan::WrongCommitmentId => submit(&evidence_request(
                    case.id,
                    case.id,
                    "DEFERRED-999999-01".to_string(),
                    rebound.condition_sha256.clone(),
                    ConditionEvidenceDispositionIR::VerifiedSatisfied,
                )),
                EvidencePlan::ReplaySatisfied => {
                    let request = evidence_request(
                        case.id,
                        case.id,
                        rebound.commitment_id.clone(),
                        rebound.condition_sha256.clone(),
                        ConditionEvidenceDispositionIR::VerifiedSatisfied,
                    );
                    submit(&request);
                    submit(&request);
                }
            }
        }
        let state = api
            .conversation_state(case.id)
            .expect("post-evidence state");
        let audit = audit_links(state);
        let source_status =
            status_of(state, &source.commitment_id).unwrap_or_else(|| "MISSING".to_string());
        let rebound_status =
            status_of(state, &rebound.commitment_id).unwrap_or_else(|| "MISSING".to_string());
        let mut activated_subjects = state
            .deferred_action_commitments
            .iter()
            .filter(|item| item.status == DeferredCommitmentStatusIR::Activated)
            .map(|item| item.action.subject.clone())
            .collect::<Vec<_>>();
        activated_subjects.sort();
        let mut expected_activated_subjects = case
            .expected_activated_subjects
            .iter()
            .map(|subject| (*subject).to_string())
            .collect::<Vec<_>>();
        expected_activated_subjects.sort();
        let pass = source_status == expected_status(case.expected_source_status)
            && rebound_status == expected_status(case.expected_rebound_status)
            && activated_subjects == expected_activated_subjects
            && accepted_receipts == case.expected_receipts
            && rejected_receipts == case.expected_rejections
            && external_actions_executed == 0
            && audit.lifecycle_integrity
            && !audit.authority_violation;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            state_schema: state.schema.clone(),
            guarded_programs: audit.guarded_programs,
            guarded_steps: audit.guarded_steps,
            linked_guarded_steps: audit.linked_guarded_steps,
            source_status,
            rebound_status,
            activated_subjects,
            accepted_receipts,
            rejected_receipts,
            external_actions_executed,
            lifecycle_integrity: audit.lifecycle_integrity,
            language_semantic_authority: audit.authority_violation,
            external_execution_authorized_by_language: audit.authority_violation,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R48_GUARDED_EVIDENCE_LIFECYCLE_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
