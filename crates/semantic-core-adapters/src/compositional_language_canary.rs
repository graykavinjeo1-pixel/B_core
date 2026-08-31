use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{CandidateDispositionIR, CompositionalSemanticAnalyzer, ScopeKindIR};
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
struct Case {
    case_id: &'static str,
    text: &'static str,
    expected_intent: Option<PlanIntentIR>,
    subject_contains: Option<&'static str>,
    minimum_blocked: usize,
    expected_authority: Option<bool>,
    expected_scope: Option<ScopeKindIR>,
}

#[derive(Debug, Serialize)]
struct ResultRow {
    case_id: &'static str,
    selected_intent: Option<String>,
    selected_subject: Option<String>,
    blocked_candidates: usize,
    external_execution_authorized: Option<bool>,
    pass: bool,
}

fn main() {
    let cases = [
        Case {
            case_id: "KO_NEGATED_REPAIR_OUTER_EXPLAIN",
            text: "서비스를 수정하지 말고 장애 원인만 설명해줘",
            expected_intent: Some(PlanIntentIR::Explain),
            subject_contains: Some("원인"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Negation),
        },
        Case {
            case_id: "KO_QUOTED_DELETE_OUTER_ANALYZE",
            text: "‘로그를 삭제해’라는 요청의 위험을 분석해",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("위험"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Quotation),
        },
        Case {
            case_id: "KO_REPORTED_EXECUTION_OUTER_RECORD",
            text: "민수가 실행하라고 말했다는 사실만 기록해",
            expected_intent: Some(PlanIntentIR::Communicate),
            subject_contains: Some("사실"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::ReportedSpeech),
        },
        Case {
            case_id: "KO_COUNTERFACTUAL_CLEAR",
            text: "임시파일을 지웠더라면 테스트가 통과했을 텐데",
            expected_intent: None,
            subject_contains: None,
            minimum_blocked: 1,
            expected_authority: None,
            expected_scope: Some(ScopeKindIR::Counterfactual),
        },
        Case {
            case_id: "KO_CONDITIONAL_QUESTION",
            text: "인덱스를 재생성하면 조회가 빨라질까?",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("인덱스"),
            minimum_blocked: 0,
            expected_authority: Some(false),
            expected_scope: Some(ScopeKindIR::Hypothetical),
        },
        Case {
            case_id: "KO_TARGET_CORRECTION",
            text: "서버 말고 클라이언트를 문서화해",
            expected_intent: Some(PlanIntentIR::Create),
            subject_contains: Some("클라이언트"),
            minimum_blocked: 0,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::AlternativeExclusion),
        },
        Case {
            case_id: "KO_DELETE_PROHIBITED_CHECK_SELECTED",
            text: "데이터는 삭제하지 말고 무결성만 확인해",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("무결성"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Negation),
        },
        Case {
            case_id: "KO_META_LANGUAGE_QUOTED_REPAIR",
            text: "‘코드를 고쳐’라는 표현을 해설해",
            expected_intent: Some(PlanIntentIR::Explain),
            subject_contains: Some("표현"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Quotation),
        },
        Case {
            case_id: "KO_NECESSARY_BUT_NOT_AUTHORIZED",
            text: "로그를 저장해야 한다",
            expected_intent: Some(PlanIntentIR::Execute),
            subject_contains: Some("로그"),
            minimum_blocked: 0,
            expected_authority: Some(false),
            expected_scope: None,
        },
        Case {
            case_id: "KO_OPAQUE_TARGET",
            text: "무루를 분석해",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("무루"),
            minimum_blocked: 0,
            expected_authority: Some(true),
            expected_scope: None,
        },
        Case {
            case_id: "EN_NEGATED_MOVE_OUTER_EXPLAIN",
            text: "Don't move the database; explain the migration risk.",
            expected_intent: Some(PlanIntentIR::Explain),
            subject_contains: Some("migration risk"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Negation),
        },
        Case {
            case_id: "EN_QUOTED_DEPLOY_OUTER_ANALYZE",
            text: "‘Deploy now’ is a phrase; analyze its tone.",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("tone"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Quotation),
        },
        Case {
            case_id: "EN_REPORTED_RUN_OUTER_RECORD",
            text: "The lead said to run the job; record that instruction only.",
            expected_intent: Some(PlanIntentIR::Communicate),
            subject_contains: Some("instruction"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::ReportedSpeech),
        },
        Case {
            case_id: "EN_COUNTERFACTUAL_CLEAR",
            text: "Had we cleared the cache, the build would have passed.",
            expected_intent: None,
            subject_contains: None,
            minimum_blocked: 1,
            expected_authority: None,
            expected_scope: Some(ScopeKindIR::Counterfactual),
        },
        Case {
            case_id: "EN_CONDITIONAL_QUESTION",
            text: "Would deleting the cache improve startup time?",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("cache"),
            minimum_blocked: 0,
            expected_authority: Some(false),
            expected_scope: None,
        },
        Case {
            case_id: "EN_TARGET_CORRECTION",
            text: "Document the CLI, not the API.",
            expected_intent: Some(PlanIntentIR::Create),
            subject_contains: Some("cli"),
            minimum_blocked: 0,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::AlternativeExclusion),
        },
        Case {
            case_id: "EN_REPAIR_PROHIBITED_INSPECT_SELECTED",
            text: "Do not repair the worker; just inspect the queue.",
            expected_intent: Some(PlanIntentIR::Investigate),
            subject_contains: Some("queue"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Negation),
        },
        Case {
            case_id: "EN_OPAQUE_TARGET",
            text: "Explain the opaque target zorbium.",
            expected_intent: Some(PlanIntentIR::Explain),
            subject_contains: Some("zorbium"),
            minimum_blocked: 0,
            expected_authority: Some(true),
            expected_scope: None,
        },
        Case {
            case_id: "EN_META_LANGUAGE_QUOTED_DELETE",
            text: "The sentence ‘delete the table’ should be explained.",
            expected_intent: Some(PlanIntentIR::Explain),
            subject_contains: Some("sentence"),
            minimum_blocked: 1,
            expected_authority: Some(false),
            expected_scope: Some(ScopeKindIR::Quotation),
        },
        Case {
            case_id: "EN_CREATE_WITH_DEPLOY_PROHIBITION",
            text: "Please create the report, but do not deploy it.",
            expected_intent: Some(PlanIntentIR::Create),
            subject_contains: Some("report"),
            minimum_blocked: 1,
            expected_authority: Some(true),
            expected_scope: Some(ScopeKindIR::Negation),
        },
    ];

    let analyzer = CompositionalSemanticAnalyzer;
    let mut rows = Vec::new();
    for case in cases {
        let analysis = analyzer.analyze(case.text);
        let selected = analysis.selected_candidate();
        let blocked = analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.disposition != CandidateDispositionIR::Viable)
            .count();
        let intent_matches = selected.map(|candidate| candidate.intent) == case.expected_intent;
        let subject_matches = case.subject_contains.is_none_or(|fragment| {
            selected.is_some_and(|candidate| candidate.subject.contains(fragment))
        });
        let authority_matches = case.expected_authority.is_none_or(|expected| {
            selected.is_some_and(|candidate| candidate.external_execution_authorized == expected)
        });
        let scope_matches = case
            .expected_scope
            .is_none_or(|expected| analysis.scopes.iter().any(|scope| scope.kind == expected));
        let pass = intent_matches
            && subject_matches
            && blocked >= case.minimum_blocked
            && authority_matches
            && scope_matches
            && !analysis.clarification_required;
        rows.push(ResultRow {
            case_id: case.case_id,
            selected_intent: selected.map(|candidate| format!("{:?}", candidate.intent)),
            selected_subject: selected.map(|candidate| candidate.subject.clone()),
            blocked_candidates: blocked,
            external_execution_authorized: selected
                .map(|candidate| candidate.external_execution_authorized),
            pass,
        });
    }
    println!(
        "{}",
        serde_json::to_string(&rows).expect("serialize compositional canary")
    );
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
