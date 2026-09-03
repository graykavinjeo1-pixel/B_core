//! Frozen R52 held-out transfer suite for reference-safe result anaphora.

mod reference_safe_result_anaphora_transfer_support;

use reference_safe_result_anaphora_transfer_support::{emit, Case, Expectation, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R52_H01",
        category: "english_proceed_with_it_preserves_prior_task",
        turns: &[
            Turn { text: "We are repairing the evidence router.", language: English },
            Turn { text: "The dashboard increase may come from cached fixtures.", language: English },
            Turn { text: "Proceed with it only if isolated reruns lower the real defect rate; if not, report the finding and ask before halting.", language: English },
        ],
        expectation: Expectation::Continuation {
            task_term: "repairing",
            benefit_term: "defect",
            forbidden_benefit_terms: &["halting"],
            forbidden_output_terms: &["presupposes", "remains inactive"],
        },
    },
    Case {
        id: "R52_H02",
        category: "cross_language_keep_doing_that_preserves_task",
        turns: &[
            Turn { text: "현재 증거 라우터를 수리하는 중이야.", language: Korean },
            Turn { text: "That gain could still be a warmed-cache artifact.", language: English },
            Turn { text: "Keep doing that only when isolated runs lower actual error frequency; otherwise explain the observation and ask before halting.", language: English },
        ],
        expectation: Expectation::Continuation {
            task_term: "수리",
            benefit_term: "error",
            forbidden_benefit_terms: &["halting"],
            forbidden_output_terms: &["presupposes", "remains inactive"],
        },
    },
    Case {
        id: "R52_H03",
        category: "korean_geu_jageop_continuation_preserves_task",
        turns: &[
            Turn { text: "현재 이벤트 인덱스를 통합하는 중이야.", language: Korean },
            Turn { text: "최근 수치는 캐시된 입력의 영향일 수 있어.", language: Korean },
            Turn { text: "독립 실행에서 실제 오류가 줄어드는 경우에만 그 작업을 이어가. 아니면 관찰을 알려주고 멈출지 확인해.", language: Korean },
        ],
        expectation: Expectation::Continuation {
            task_term: "통합",
            benefit_term: "오류",
            forbidden_benefit_terms: &["멈출"],
            forbidden_output_terms: &["전제"],
        },
    },
    Case {
        id: "R52_H04",
        category: "english_same_turn_outcome_synonym",
        turns: &[Turn { text: "Verify whether export retains provenance, and summarize only that outcome.", language: English }],
        expectation: Expectation::LocalResultGoal {
            subject_term: "provenance",
            source_marker: "outcome",
        },
    },
    Case {
        id: "R52_H05",
        category: "korean_same_turn_artifact_synonym",
        turns: &[Turn { text: "내보내기가 출처 기록을 보존하는지 검증하고 그 산출물만 요약해.", language: Korean }],
        expectation: Expectation::LocalResultGoal {
            subject_term: "출처",
            source_marker: "산출물",
        },
    },
    Case {
        id: "R52_H06",
        category: "english_true_cross_turn_output_absence",
        turns: &[
            Turn { text: "Validate the queue.", language: English },
            Turn { text: "What did that output show?", language: English },
        ],
        expectation: Expectation::CrossTurnResultAbsence {
            output_term: "No execution result",
        },
    },
    Case {
        id: "R52_H07",
        category: "english_curly_quoted_result_is_inert",
        turns: &[Turn { text: "The note reads, “analyze the archive and report that result.” Evaluate rollback time only; do not analyze the archive.", language: English }],
        expectation: Expectation::QuotedResultSafeGoal {
            subject_term: "rollback",
        },
    },
    Case {
        id: "R52_H08",
        category: "korean_curly_quoted_artifact_is_inert",
        turns: &[Turn { text: "운영 메모에는 “캐시를 점검하고 그 산출물을 보고해”라고 적혀 있어. 복원 시간만 비교해. 캐시는 점검하지 마.", language: Korean }],
        expectation: Expectation::QuotedResultSafeGoal {
            subject_term: "복원",
        },
    },
];

fn main() {
    emit("R52_REFERENCE_SAFE_RESULT_ANAPHORA_HELDOUT", CASES);
}
