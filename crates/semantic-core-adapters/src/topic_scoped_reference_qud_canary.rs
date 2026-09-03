//! Frozen R55 diagnostic. The oracle requires explicit topic return to scope
//! result anaphora and restore the topic's suspended question under discussion.

mod topic_scoped_reference_qud_canary_support;

use semantic_core_adapters::LanguageCodeIR::{English, Korean};
use topic_scoped_reference_qud_canary_support::{emit, text, voice, Case, Expectation};

const CASES: &[Case] = &[
    Case {
        id: "R55_01",
        category: "english_named_topic_result",
        turns: &[
            text("Switch to cache.", English),
            text("Diagnose the cache.", English),
            text("Switch to queue.", English),
            text("Inspect the queue.", English),
            text("Return to cache.", English),
            text("Explain that result.", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "cache",
            result_term: "cache",
            forbidden_result_terms: &["queue"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_02",
        category: "korean_named_topic_result",
        turns: &[
            text("캐시 주제로 전환해.", Korean),
            text("캐시를 진단해.", Korean),
            text("큐 주제로 전환해.", Korean),
            text("큐를 검사해.", Korean),
            text("캐시 주제로 돌아가.", Korean),
            text("그 결과를 설명해.", Korean),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "캐시",
            result_term: "캐시",
            forbidden_result_terms: &["큐"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_03",
        category: "cross_language_topic_result",
        turns: &[
            text("캐시 주제로 전환해.", Korean),
            text("캐시를 수리해.", Korean),
            text("Switch to log.", English),
            text("Inspect the log.", English),
            text("Return to the cache.", English),
            text("What about that result?", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "cache",
            result_term: "캐시",
            forbidden_result_terms: &["log"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_04",
        category: "long_indexed_topic_result",
        turns: &[
            text("Switch to cache.", English),
            text("Repair the cache.", English),
            text("Switch to queue.", English),
            text("Inspect the queue.", English),
            text("Switch to log.", English),
            text("Diagnose the log.", English),
            text("Um, let me think.", English),
            text("Right.", English),
            text("Return to the topic two topics ago.", English),
            text("Explain that output.", English),
        ],
        restoration_turn: 9,
        expectation: Expectation::ScopedResult {
            restored_topic: "cache",
            result_term: "cache",
            forbidden_result_terms: &["queue", "log"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_05",
        category: "previous_topic_result",
        turns: &[
            text("백업 주제로 전환해.", Korean),
            text("백업을 검사해.", Korean),
            text("서버 주제로 전환해.", Korean),
            text("서버를 진단해.", Korean),
            text("이전 주제로 돌아가.", Korean),
            text("그 출력은 어떻게 됐어?", Korean),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedResult {
            restored_topic: "백업",
            result_term: "백업",
            forbidden_result_terms: &["서버"],
            source_turn: 2,
        },
    },
    Case {
        id: "R55_06",
        category: "unseen_topic_result_fails_closed",
        turns: &[
            text("Switch to queue.", English),
            text("Repair the queue.", English),
            text("Switch to report.", English),
            text("Explain that result.", English),
        ],
        restoration_turn: 3,
        expectation: Expectation::MissingTopicResult {
            restored_topic: "report",
            forbidden_result_terms: &["queue"],
        },
    },
    Case {
        id: "R55_07",
        category: "english_two_topic_qud_restore",
        turns: &[
            text("Switch to cache.", English),
            voice("Inspect the cache.", "Repair the cache.", English),
            text("Switch to queue.", English),
            voice("Inspect the queue.", "Delete the queue.", English),
            text("Return to cache.", English),
            text("The second one.", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedQud {
            restored_topic: "cache",
            question_source_turn: 2,
            selected_term: "repair",
            forbidden_selected_terms: &["queue", "delete"],
        },
    },
    Case {
        id: "R55_08",
        category: "korean_two_topic_qud_restore",
        turns: &[
            text("캐시 주제로 전환해.", Korean),
            voice("캐시를 검사해.", "캐시를 수리해.", Korean),
            text("큐 주제로 전환해.", Korean),
            voice("큐를 검사해.", "큐를 삭제해.", Korean),
            text("캐시 주제로 돌아가.", Korean),
            text("두 번째.", Korean),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedQud {
            restored_topic: "캐시",
            question_source_turn: 2,
            selected_term: "수리",
            forbidden_selected_terms: &["큐", "삭제"],
        },
    },
    Case {
        id: "R55_09",
        category: "cross_language_qud_restore",
        turns: &[
            text("캐시 주제로 전환해.", Korean),
            voice("캐시를 검사해.", "캐시를 수리해.", Korean),
            text("Switch to worker.", English),
            voice("Inspect the worker.", "Delete the worker.", English),
            text("Return to cache.", English),
            text("The second one.", English),
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedQud {
            restored_topic: "cache",
            question_source_turn: 2,
            selected_term: "repair",
            forbidden_selected_terms: &["worker", "delete"],
        },
    },
    Case {
        id: "R55_10",
        category: "unseen_topic_qud_fails_closed",
        turns: &[
            text("Switch to cache.", English),
            voice("Inspect the cache.", "Repair the cache.", English),
            text("Switch to report.", English),
            text("The second one.", English),
        ],
        restoration_turn: 3,
        expectation: Expectation::MissingTopicQud {
            restored_topic: "report",
            forbidden_selected_terms: &["cache", "repair"],
        },
    },
];

fn main() {
    emit("R55_TOPIC_SCOPED_REFERENCE_QUD_DIAGNOSTIC", CASES);
}
