use semantic_core_adapters::{
    validate_conversation_state, CognitiveApi, ConditionalGuardEvaluationIR,
    ConditionalGuardStoreIR, ConditionalKindIR, ConversationInputModalityIR,
    ConversationTurnRequestIR, GuardStatusIR, LanguageCodeIR, ModalSemanticAnalyzer,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct CanaryRow {
    id: String,
    family: String,
    input: String,
    pass: bool,
    observed: String,
}

#[derive(Debug, Serialize)]
struct CanaryReport {
    schema: &'static str,
    status: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    english_surface: usize,
    korean_surface: usize,
    evidence_evaluation: usize,
    modal_and_reverse_inference: usize,
    conflict_and_revision: usize,
    safety_and_tamper: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    rows: Vec<CanaryRow>,
}

fn request(id: &str, turn: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
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
        max_plan_steps: 12,
    }
}

fn surface_row(
    id: &str,
    family: &str,
    text: &str,
    expected_kind: ConditionalKindIR,
    expected_negated: bool,
) -> CanaryRow {
    let graph = ModalSemanticAnalyzer.analyze(text);
    let conditional = graph.conditionals.first();
    let pass = conditional.is_some_and(|conditional| {
        conditional.kind == expected_kind
            && conditional.antecedent_negated == expected_negated
            && !conditional.condition_satisfied
            && !conditional.reverse_inference_authorized
            && !conditional.external_execution_authorized
            && !conditional.antecedent.trim().is_empty()
            && !conditional.consequent.trim().is_empty()
    }) && graph.validate();
    CanaryRow {
        id: id.to_string(),
        family: family.to_string(),
        input: text.to_string(),
        pass,
        observed: conditional.map_or_else(
            || "conditional=NONE".to_string(),
            |conditional| {
                format!(
                    "kind={:?};negated={};antecedent={};consequent={};authority={}",
                    conditional.kind,
                    conditional.antecedent_negated,
                    conditional.antecedent,
                    conditional.consequent,
                    conditional.external_execution_authorized
                )
            },
        ),
    }
}

fn sequence_row(
    id: &str,
    family: &str,
    declaration: &str,
    evidence_turns: &[&str],
    expected: GuardStatusIR,
) -> CanaryRow {
    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    let first = api
        .process_conversation_turn(&request(id, 1, declaration))
        .expect("guard declaration");
    let mut last = first;
    for (index, evidence) in evidence_turns.iter().enumerate() {
        last = api
            .process_conversation_turn(&request(
                id,
                u64::try_from(index + 2).expect("bounded turn"),
                evidence,
            ))
            .expect("guard evidence turn");
    }
    let guard = last
        .conversation_state
        .conditional_guard_store
        .guards
        .first();
    let pass = guard.is_some_and(|guard| {
        guard.status == expected
            && guard.deliberation_eligible
                == (expected == GuardStatusIR::SupportedByDialogueEvidence)
            && !guard.dialogue_truth_established
            && !guard.reverse_inference_authorized
            && !guard.external_execution_authorized
            && last.conversation_state.active_goals.is_empty()
            && last.conversation_state.conditional_guard_store.validate(
                last.conversation_state.completed_turns,
                &last.conversation_state.epistemic_ledger,
            )
            && validate_conversation_state(&last.conversation_state).is_ok()
    });
    CanaryRow {
        id: id.to_string(),
        family: family.to_string(),
        input: format!("{} -> {}", declaration, evidence_turns.join(" -> ")),
        pass,
        observed: guard.map_or_else(
            || "guard=NONE".to_string(),
            |guard| {
                format!(
                    "status={:?};evidence={};deliberation={};truth={};reverse={};authority={}",
                    guard.status,
                    guard.evidence.len(),
                    guard.deliberation_eligible,
                    guard.dialogue_truth_established,
                    guard.reverse_inference_authorized,
                    guard.external_execution_authorized
                )
            },
        ),
    }
}

fn tamper_row(id: &str, pass: bool, observed: &str) -> CanaryRow {
    CanaryRow {
        id: id.to_string(),
        family: "SAFETY_AND_TAMPER".to_string(),
        input: id.to_string(),
        pass,
        observed: observed.to_string(),
    }
}

fn supported_fixture() -> (
    ConditionalGuardStoreIR,
    semantic_core_adapters::EpistemicLedgerIR,
    ConditionalGuardEvaluationIR,
) {
    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    api.process_conversation_turn(&request(
        "TAMPER-FIXTURE",
        1,
        "If the tests pass, deploy the service.",
    ))
    .expect("guard declaration");
    let response = api
        .process_conversation_turn(&request("TAMPER-FIXTURE", 2, "The tests passed."))
        .expect("guard evidence");
    (
        response.conversation_state.conditional_guard_store,
        response.conversation_state.epistemic_ledger,
        response.conditional_guard_evaluations[0].clone(),
    )
}

fn main() {
    let mut rows = Vec::new();

    for (id, text, kind, negated) in [
        (
            "EN_IF_PASS",
            "If the checks pass, publish the package.",
            ConditionalKindIR::Indicative,
            false,
        ),
        (
            "EN_IF_FAIL",
            "If the worker fails, alert the operator.",
            ConditionalKindIR::Indicative,
            false,
        ),
        (
            "EN_UNLESS_READY",
            "Unless the release is ready, stop the rollout.",
            ConditionalKindIR::Unless,
            true,
        ),
        (
            "EN_IF_VALID",
            "If the token is valid, continue the request.",
            ConditionalKindIR::Indicative,
            false,
        ),
        (
            "EN_IF_AVAILABLE",
            "If the backup is available, restore the archive.",
            ConditionalKindIR::Indicative,
            false,
        ),
        (
            "EN_THEN_FORM",
            "If the queue is empty then close the worker.",
            ConditionalKindIR::Indicative,
            false,
        ),
        (
            "EN_COUNTERFACTUAL",
            "If the backup had existed, the restore would have succeeded.",
            ConditionalKindIR::Counterfactual,
            false,
        ),
        (
            "EN_DECLARATIVE_CONSEQUENT",
            "If the checks pass, the release becomes ready.",
            ConditionalKindIR::Indicative,
            false,
        ),
    ] {
        rows.push(surface_row(id, "ENGLISH_SURFACE", text, kind, negated));
    }

    for (id, text, kind, negated) in [
        (
            "KO_IF_PASS",
            "검사가 통과하면 패키지를 게시해.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
        (
            "KO_IF_FAIL",
            "작업자가 실패하면 운영자에게 알려.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
        (
            "KO_UNLESS_READY",
            "릴리스가 준비되지 않으면 배포를 멈춰.",
            ConditionalKindIR::Unless,
            true,
        ),
        (
            "KO_IF_VALID",
            "토큰이 유효하면 요청을 계속해.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
        (
            "KO_IF_AVAILABLE",
            "백업이 존재하면 보관함을 복구해.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
        (
            "KO_IF_OPERATIONAL",
            "서비스가 정상되면 상태를 보고해.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
        (
            "KO_COUNTERFACTUAL",
            "백업이 있었더라면 복구가 성공했을 텐데.",
            ConditionalKindIR::Counterfactual,
            false,
        ),
        (
            "KO_ROLE_CONDITION",
            "관리자라면 설정을 확인해.",
            ConditionalKindIR::Hypothetical,
            false,
        ),
    ] {
        rows.push(surface_row(id, "KOREAN_SURFACE", text, kind, negated));
    }

    for (id, declaration, evidence, expected) in [
        (
            "EVAL_EN_PASS_SUPPORT",
            "If the tests pass, publish the release.",
            "The tests passed.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_EN_PASS_CONTRADICT",
            "If the tests pass, publish the release.",
            "The tests failed.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "EVAL_EN_FAIL_SUPPORT",
            "If the build fails, alert the team.",
            "The build failed.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_EN_FAIL_CONTRADICT",
            "If the build fails, alert the team.",
            "The build succeeded.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "EVAL_EN_VALID_SUPPORT",
            "If the cache is valid, continue the run.",
            "The cache is valid.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_EN_VALID_CONTRADICT",
            "If the cache is valid, continue the run.",
            "The cache is invalid.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "EVAL_EN_AVAILABLE_SUPPORT",
            "If the backup is available, restore it.",
            "The backup is available.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_EN_UNLESS_SUPPORT",
            "Unless the tests pass, stop the deployment.",
            "The tests failed.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_EN_UNLESS_CONTRADICT",
            "Unless the tests pass, stop the deployment.",
            "The tests passed.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "EVAL_KO_PASS_SUPPORT",
            "테스트가 통과하면 릴리스를 게시해.",
            "테스트가 통과했다.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_KO_PASS_CONTRADICT",
            "테스트가 통과하면 릴리스를 게시해.",
            "테스트가 실패했다.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "EVAL_KO_VALID_SUPPORT",
            "캐시가 유효하면 작업을 계속해.",
            "캐시가 유효하다.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_KO_UNLESS_SUPPORT",
            "테스트가 통과하지 않으면 배포를 멈춰.",
            "테스트가 실패했다.",
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "EVAL_KO_OPERATIONAL_CONTRADICT",
            "서비스가 정상이면 보고해.",
            "서비스가 비정상이다.",
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
    ] {
        rows.push(sequence_row(
            id,
            "EVIDENCE_EVALUATION",
            declaration,
            &[evidence],
            expected,
        ));
    }

    for (id, declaration, evidence, expected) in [
        (
            "MODAL_EN_MIGHT",
            "If the tests pass, publish the release.",
            "The tests might pass.",
            GuardStatusIR::Unresolved,
        ),
        (
            "MODAL_EN_PROBABLY",
            "If the tests pass, publish the release.",
            "The tests will probably pass.",
            GuardStatusIR::Unresolved,
        ),
        (
            "MODAL_EN_PREDICTED",
            "If the tests pass, publish the release.",
            "The tests will pass.",
            GuardStatusIR::Unresolved,
        ),
        (
            "MODAL_EN_DESIRED",
            "If the tests pass, publish the release.",
            "Alice wants the tests to pass.",
            GuardStatusIR::Unresolved,
        ),
        (
            "REVERSE_EN_CONSEQUENT",
            "If the tests pass, deploy the service.",
            "The service deployed.",
            GuardStatusIR::Unresolved,
        ),
        (
            "REVERSE_KO_CONSEQUENT",
            "테스트가 통과하면 서비스를 배포해.",
            "서비스가 배포됐다.",
            GuardStatusIR::Unresolved,
        ),
        (
            "ENTITY_EN_WRONG_SUBJECT",
            "If the tests pass, publish the release.",
            "The build passed.",
            GuardStatusIR::Unresolved,
        ),
        (
            "ENTITY_KO_WRONG_SUBJECT",
            "테스트가 통과하면 릴리스를 게시해.",
            "빌드가 통과했다.",
            GuardStatusIR::Unresolved,
        ),
    ] {
        rows.push(sequence_row(
            id,
            "MODAL_AND_REVERSE_INFERENCE",
            declaration,
            &[evidence],
            expected,
        ));
    }

    for (id, declaration, evidence, expected) in [
        (
            "CONFLICT_EN_SOURCES",
            "If the tests pass, deploy the service.",
            vec!["Alice says the tests passed.", "Bob says the tests failed."],
            GuardStatusIR::Contested,
        ),
        (
            "CONFLICT_KO_SOURCES",
            "테스트가 통과하면 서비스를 배포해.",
            vec![
                "민수는 테스트가 통과했다고 말했다.",
                "지수는 테스트가 실패했다고 말했다.",
            ],
            GuardStatusIR::Contested,
        ),
        (
            "REVISION_EN_TO_SUPPORT",
            "If the tests pass, deploy the service.",
            vec![
                "Alice says the tests failed.",
                "Actually, Alice says the tests passed.",
            ],
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "REVISION_EN_TO_BLOCK",
            "If the tests pass, deploy the service.",
            vec![
                "Alice says the tests passed.",
                "Actually, Alice says the tests failed.",
            ],
            GuardStatusIR::ContradictedByDialogueEvidence,
        ),
        (
            "CONFLICT_IRRELEVANT_TOPIC",
            "If the tests pass, deploy the service.",
            vec![
                "The tests passed.",
                "Alice says the cache is valid.",
                "Bob says the cache is invalid.",
            ],
            GuardStatusIR::SupportedByDialogueEvidence,
        ),
        (
            "COUNTERFACTUAL_STAYS_INELIGIBLE",
            "If the tests had passed, the deploy would have succeeded.",
            vec!["The tests passed."],
            GuardStatusIR::IneligibleCounterfactual,
        ),
    ] {
        rows.push(sequence_row(
            id,
            "CONFLICT_AND_REVISION",
            declaration,
            &evidence,
            expected,
        ));
    }

    let (store, ledger, evaluation) = supported_fixture();
    let completed_turns = 2;

    let mut changed = store.clone();
    changed.schema = "TAMPERED".to_string();
    rows.push(tamper_row(
        "TAMPER_STORE_SCHEMA",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].dialogue_truth_established = true;
    rows.push(tamper_row(
        "TAMPER_GUARD_TRUTH",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].reverse_inference_authorized = true;
    rows.push(tamper_row(
        "TAMPER_GUARD_REVERSE",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].external_execution_authorized = true;
    rows.push(tamper_row(
        "TAMPER_GUARD_AUTHORITY",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].deliberation_eligible = false;
    rows.push(tamper_row(
        "TAMPER_DELIBERATION_FLAG",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].status = GuardStatusIR::Unresolved;
    rows.push(tamper_row(
        "TAMPER_STATUS_EVIDENCE",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].evidence[0].dialogue_truth_established = true;
    rows.push(tamper_row(
        "TAMPER_EVIDENCE_TRUTH",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].evidence[0].external_execution_authorized = true;
    rows.push(tamper_row(
        "TAMPER_EVIDENCE_AUTHORITY",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = store.clone();
    changed.guards[0].evidence[0].belief_id = "BELIEF-UNKNOWN".to_string();
    rows.push(tamper_row(
        "TAMPER_UNKNOWN_BELIEF",
        !changed.validate(completed_turns, &ledger),
        "tamper_rejected",
    ));

    let mut changed = evaluation.clone();
    changed.dialogue_truth_established = true;
    rows.push(tamper_row(
        "TAMPER_EVALUATION_TRUTH",
        !changed.validate(&store, &ledger),
        "tamper_rejected",
    ));

    let mut changed = evaluation.clone();
    changed.reverse_inference_authorized = true;
    rows.push(tamper_row(
        "TAMPER_EVALUATION_REVERSE",
        !changed.validate(&store, &ledger),
        "tamper_rejected",
    ));

    let mut changed = evaluation.clone();
    changed.unsupported_claims = 1;
    rows.push(tamper_row(
        "TAMPER_UNSUPPORTED_CLAIM",
        !changed.validate(&store, &ledger),
        "tamper_rejected",
    ));

    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    let report = CanaryReport {
        schema: "B_CORE_CONDITIONAL_GUARD_CANARY_V1",
        status: if passed == total { "PASS" } else { "FAIL" },
        total,
        passed,
        failed: total - passed,
        english_surface: rows
            .iter()
            .filter(|row| row.family == "ENGLISH_SURFACE")
            .count(),
        korean_surface: rows
            .iter()
            .filter(|row| row.family == "KOREAN_SURFACE")
            .count(),
        evidence_evaluation: rows
            .iter()
            .filter(|row| row.family == "EVIDENCE_EVALUATION")
            .count(),
        modal_and_reverse_inference: rows
            .iter()
            .filter(|row| row.family == "MODAL_AND_REVERSE_INFERENCE")
            .count(),
        conflict_and_revision: rows
            .iter()
            .filter(|row| row.family == "CONFLICT_AND_REVISION")
            .count(),
        safety_and_tamper: rows
            .iter()
            .filter(|row| row.family == "SAFETY_AND_TAMPER")
            .count(),
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize canary report")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
