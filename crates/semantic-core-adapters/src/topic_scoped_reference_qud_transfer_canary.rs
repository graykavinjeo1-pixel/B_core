//! Frozen R55 held-out transfer suite. These cases use topic surfaces,
//! ellipsis forms, stack depths, and QUD lifecycles absent from the diagnostic.

#[allow(dead_code)]
mod topic_scoped_reference_qud_canary_support;

use semantic_core_adapters::LanguageCodeIR::{English, Korean};
use topic_scoped_reference_qud_canary_support::{emit, text, voice, Case, Expectation};

const CASES: &[Case] = &[
    Case {
        id: "R55_H01",
        category: "novel_english_topic_bare_result_ellipsis",
        turns: &[
            text("Switch to parser.", English),
            text("Inspect the parser.", English),
            text("Switch to scheduler.", English),
            text("Diagnose the scheduler.", English),
            text("Return to parser.", English),
            text("And the output?", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "parser",
            result_term: "parser",
            forbidden_result_terms: &["scheduler"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_H02",
        category: "novel_korean_topic_bare_result_ellipsis",
        turns: &[
            text("인덱스 주제로 전환해.", Korean),
            text("인덱스를 검사해.", Korean),
            text("라우터 주제로 전환해.", Korean),
            text("라우터를 진단해.", Korean),
            text("인덱스 주제로 돌아가.", Korean),
            text("결과는 어떻게 됐어?", Korean),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "인덱스",
            result_term: "인덱스",
            forbidden_result_terms: &["라우터"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_H03",
        category: "cross_language_bare_result_ellipsis",
        turns: &[
            text("백업 주제로 전환해.", Korean),
            text("백업을 수리해.", Korean),
            text("Switch to worker.", English),
            text("Inspect the worker.", English),
            text("Return to backup.", English),
            text("What about the outcome?", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "backup",
            result_term: "백업",
            forbidden_result_terms: &["worker"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_H04",
        category: "three_topic_indexed_long_result",
        turns: &[
            text("Switch to parser.", English),
            text("Repair the parser.", English),
            text("Switch to scheduler.", English),
            text("Inspect the scheduler.", English),
            text("Switch to telemetry.", English),
            text("Diagnose the telemetry.", English),
            text("Um.", English),
            text("Let me think.", English),
            text("Return to the topic two topics ago.", English),
            text("And the result?", English),
        ],
        restoration_turn: 9,
        expectation: Expectation::ScopedResult {
            restored_topic: "parser",
            result_term: "parser",
            forbidden_result_terms: &["scheduler", "telemetry"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_H05",
        category: "novel_unseen_topic_result_fails_closed",
        turns: &[
            text("Switch to parser.", English),
            text("Repair the parser.", English),
            text("Switch to telemetry.", English),
            text("And the output?", English),
        ],
        restoration_turn: 3,
        expectation: Expectation::MissingTopicResult {
            restored_topic: "telemetry",
            forbidden_result_terms: &["parser"],
        },
    },
    Case {
        id: "R55_H06",
        category: "novel_cross_language_qud_restore",
        turns: &[
            text("Switch to parser.", English),
            voice("Inspect the parser.", "Repair the parser.", English),
            text("스케줄러 주제로 전환해.", Korean),
            voice("스케줄러를 검사해.", "스케줄러를 삭제해.", Korean),
            text("parser 주제로 돌아가.", Korean),
            text("두 번째.", Korean),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedQud {
            restored_topic: "parser",
            question_source_turn: 2,
            selected_term: "repair",
            forbidden_selected_terms: &["스케줄러", "삭제"],
        },
    },
    Case {
        id: "R55_H07",
        category: "three_topic_indexed_qud_restore",
        turns: &[
            text("Switch to parser.", English),
            voice("Inspect the parser.", "Repair the parser.", English),
            text("Switch to scheduler.", English),
            voice("Inspect the scheduler.", "Delete the scheduler.", English),
            text("Switch to telemetry.", English),
            text("Return to the topic two topics ago.", English),
            text("The second one.", English),
        ],
        restoration_turn: 6,
        expectation: Expectation::ScopedQud {
            restored_topic: "parser",
            question_source_turn: 2,
            selected_term: "repair",
            forbidden_selected_terms: &["scheduler", "delete"],
        },
    },
    Case {
        id: "R55_H08",
        category: "resolving_one_qud_preserves_another",
        turns: &[
            text("Switch to cache.", English),
            voice("Inspect the cache.", "Repair the cache.", English),
            text("Switch to queue.", English),
            voice("Inspect the queue.", "Delete the queue.", English),
            text("Return to cache.", English),
            text("The second one.", English),
            text("Return to queue.", English),
            text("The second one.", English),
        ],
        restoration_turn: 7,
        expectation: Expectation::ScopedQud {
            restored_topic: "queue",
            question_source_turn: 4,
            selected_term: "delete",
            forbidden_selected_terms: &["cache", "repair"],
        },
    },
];

fn main() {
    emit("R55_TOPIC_SCOPED_REFERENCE_QUD_HELDOUT", CASES);
}
