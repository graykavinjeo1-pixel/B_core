//! Frozen R52 diagnostic for reference-safe continuation and result anaphora.

mod reference_safe_result_anaphora_canary_support;

use dockable_semantic_core::PlanIntentIR::Investigate;
use reference_safe_result_anaphora_canary_support::{emit, Case, Expectation, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R52_01",
        category: "english_pronoun_ignores_stop_branch",
        turns: &[
            Turn { text: "We are migrating the storage adapter.", language: English },
            Turn { text: "The latest number may reflect reused samples.", language: English },
            Turn { text: "Continue it only when fresh trials expand production coverage; otherwise tell me and ask before stopping.", language: English },
        ],
        expectation: Expectation::CleanContinuation { task_term: "migrating", benefit_term: "coverage", forbidden_benefit_terms: &["stopping", "continue stopping"] },
    },
    Case {
        id: "R52_02",
        category: "english_keep_at_it_uses_typed_task",
        turns: &[
            Turn { text: "We are refactoring the parser.", language: English },
            Turn { text: "Latency is still noisy.", language: English },
            Turn { text: "Keep at it only if cold runs reduce production failures; otherwise ask me whether to stop.", language: English },
        ],
        expectation: Expectation::CleanContinuation { task_term: "refactoring", benefit_term: "failure", forbidden_benefit_terms: &["stop", "stopping"] },
    },
    Case {
        id: "R52_03",
        category: "cross_language_pronoun_uses_typed_task",
        turns: &[
            Turn { text: "현재 저장 계층을 마이그레이션하는 중이야.", language: Korean },
            Turn { text: "That score may be a sampling artifact.", language: English },
            Turn { text: "Continue it only when fresh runs reduce real failures; otherwise report and ask before stopping.", language: English },
        ],
        expectation: Expectation::CleanContinuation { task_term: "마이그레이션", benefit_term: "failure", forbidden_benefit_terms: &["stopping", "continue stopping"] },
    },
    Case {
        id: "R52_04",
        category: "korean_task_reference_ignores_stop_branch",
        turns: &[
            Turn { text: "현재 검색기를 재설계하는 중이야.", language: Korean },
            Turn { text: "최근 점수는 재사용된 표본 때문일 수 있어.", language: Korean },
            Turn { text: "새 실험에서 실제 장애가 줄 때만 그 일을 계속해. 아니면 결과를 말하고 중단할지 물어.", language: Korean },
        ],
        expectation: Expectation::CleanContinuation { task_term: "재설계", benefit_term: "장애", forbidden_benefit_terms: &["중단"] },
    },
    Case {
        id: "R52_05",
        category: "english_same_turn_assessment_result",
        turns: &[Turn { text: "Assess whether the release preserves audit history, then report only that result.", language: English }],
        expectation: Expectation::SameTurnResultGoal { intent: Investigate, subject_term: "audit", forbidden_predicates: &["DEPLOY"] },
    },
    Case {
        id: "R52_06",
        category: "korean_same_turn_assessment_result",
        turns: &[Turn { text: "게시가 감사 추적을 보존하는지 평가하고 그 결과만 보고해.", language: Korean }],
        expectation: Expectation::SameTurnResultGoal { intent: Investigate, subject_term: "감사", forbidden_predicates: &["DEPLOY"] },
    },
    Case {
        id: "R52_07",
        category: "english_capability_correction_local_result",
        turns: &[
            Turn { text: "Can the system publish artifacts?", language: English },
            Turn { text: "I am not asking whether it can. Assess whether publishing preserves rollback history and report only that result.", language: English },
        ],
        expectation: Expectation::SameTurnResultGoal { intent: Investigate, subject_term: "rollback", forbidden_predicates: &["DEPLOY"] },
    },
    Case {
        id: "R52_08",
        category: "korean_capability_correction_local_result",
        turns: &[
            Turn { text: "시스템이 산출물을 게시할 수 있어?", language: Korean },
            Turn { text: "가능 여부를 묻는 게 아니야. 게시가 감사 추적을 보존하는지 평가하고 그 결과만 보고해.", language: Korean },
        ],
        expectation: Expectation::SameTurnResultGoal { intent: Investigate, subject_term: "감사", forbidden_predicates: &["DEPLOY"] },
    },
    Case {
        id: "R52_09",
        category: "english_true_cross_turn_result_absence",
        turns: &[
            Turn { text: "Inspect the cache.", language: English },
            Turn { text: "What was that result?", language: English },
        ],
        expectation: Expectation::CrossTurnResultAbsence { output_term: "No execution result" },
    },
    Case {
        id: "R52_10",
        category: "korean_true_cross_turn_result_absence",
        turns: &[
            Turn { text: "캐시를 점검해.", language: Korean },
            Turn { text: "그 결과는 뭐였어?", language: Korean },
        ],
        expectation: Expectation::CrossTurnResultAbsence { output_term: "결과는 아직 없어" },
    },
    Case {
        id: "R52_11",
        category: "english_quoted_result_is_not_reference",
        turns: &[Turn { text: "The runbook says 'publish the bundle and report that result.' Assess recovery cost only; do not publish it.", language: English }],
        expectation: Expectation::QuotedResultSafeGoal { intent: Investigate, subject_term: "recovery", forbidden_predicates: &["DEPLOY"] },
    },
    Case {
        id: "R52_12",
        category: "korean_quoted_result_is_not_reference",
        turns: &[Turn { text: "문서에는 '번들을 게시하고 그 결과를 보고해'라고 쓰여 있어. 복구 비용만 평가해. 게시하지 마.", language: Korean }],
        expectation: Expectation::QuotedResultSafeGoal { intent: Investigate, subject_term: "복구", forbidden_predicates: &["DEPLOY"] },
    },
];

fn main() {
    emit("R52_REFERENCE_SAFE_RESULT_ANAPHORA_DIAGNOSTIC", CASES);
}
