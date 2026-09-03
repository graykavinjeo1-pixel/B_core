//! Frozen R54 diagnostic. The oracle requires an explicit topic restoration to
//! reactivate that topic's pragmatic task instead of the globally newest task.

mod topic_scoped_pragmatic_state_canary_support;

use semantic_core_adapters::LanguageCodeIR::{English, Korean};
use topic_scoped_pragmatic_state_canary_support::{emit, Case, Expectation, Turn};

const CASES: &[Case] = &[
    Case {
        id: "R54_01",
        category: "english_named_topic_restores_task",
        turns: &[
            Turn { text: "Switch to cache.", language: English },
            Turn { text: "We are refactoring the cache.", language: English },
            Turn { text: "Switch to queue.", language: English },
            Turn { text: "We are migrating the queue.", language: English },
            Turn { text: "Return to cache.", language: English },
            Turn { text: "Continue it only when fresh runs reduce production failures; otherwise report and ask before stopping.", language: English },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "cache", task_term: "refactoring", forbidden_task_terms: &["queue", "migrating"], benefit_term: "failure" },
    },
    Case {
        id: "R54_02",
        category: "korean_named_topic_restores_task",
        turns: &[
            Turn { text: "캐시 주제로 전환해.", language: Korean },
            Turn { text: "현재 캐시를 리팩터링하는 중이야.", language: Korean },
            Turn { text: "큐 주제로 전환해.", language: Korean },
            Turn { text: "현재 큐를 마이그레이션하는 중이야.", language: Korean },
            Turn { text: "캐시 주제로 돌아가.", language: Korean },
            Turn { text: "새 실험에서 실제 장애가 줄 때만 그 일을 계속해. 아니면 결과를 말하고 중단할지 물어.", language: Korean },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "캐시", task_term: "리팩터링", forbidden_task_terms: &["큐", "마이그레이션"], benefit_term: "장애" },
    },
    Case {
        id: "R54_03",
        category: "korean_task_english_return",
        turns: &[
            Turn { text: "캐시 주제로 전환해.", language: Korean },
            Turn { text: "현재 캐시를 재설계하는 중이야.", language: Korean },
            Turn { text: "Switch to queue.", language: English },
            Turn { text: "We are testing the queue.", language: English },
            Turn { text: "Return to the cache.", language: English },
            Turn { text: "Keep at it only if fresh trials expand real coverage; otherwise ask whether to stop.", language: English },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "cache", task_term: "재설계", forbidden_task_terms: &["queue", "testing"], benefit_term: "coverage" },
    },
    Case {
        id: "R54_04",
        category: "english_task_korean_return",
        turns: &[
            Turn { text: "Switch to cache.", language: English },
            Turn { text: "We are repairing the cache.", language: English },
            Turn { text: "로그 주제로 전환해.", language: Korean },
            Turn { text: "현재 로그를 조사하는 중이야.", language: Korean },
            Turn { text: "캐시 주제로 돌아가.", language: Korean },
            Turn { text: "실제 장애가 줄 때만 그 일을 계속해. 아니면 중단할지 물어.", language: Korean },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "캐시", task_term: "repairing", forbidden_task_terms: &["로그", "조사"], benefit_term: "장애" },
    },
    Case {
        id: "R54_05",
        category: "english_previous_topic_restores_task",
        turns: &[
            Turn { text: "Switch to cache.", language: English },
            Turn { text: "We are investigating the cache.", language: English },
            Turn { text: "Switch to worker.", language: English },
            Turn { text: "We are repairing the worker.", language: English },
            Turn { text: "Return to the previous topic.", language: English },
            Turn { text: "Continue it only if fresh checks reduce real failures; otherwise report and ask before stopping.", language: English },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "cache", task_term: "investigating", forbidden_task_terms: &["worker", "repairing"], benefit_term: "failure" },
    },
    Case {
        id: "R54_06",
        category: "korean_previous_topic_restores_task",
        turns: &[
            Turn { text: "백업 주제로 전환해.", language: Korean },
            Turn { text: "현재 백업을 테스트하는 중이야.", language: Korean },
            Turn { text: "서버 주제로 전환해.", language: Korean },
            Turn { text: "현재 서버를 재설계하는 중이야.", language: Korean },
            Turn { text: "이전 주제로 돌아가.", language: Korean },
            Turn { text: "실제 장애가 줄 때만 그 일을 계속해. 아니면 중단할지 물어.", language: Korean },
        ],
        restoration_turn: 5,
        expectation: Expectation::ScopedContinuation { restored_topic: "백업", task_term: "테스트", forbidden_task_terms: &["서버", "재설계"], benefit_term: "장애" },
    },
    Case {
        id: "R54_07",
        category: "long_three_topic_suspension",
        turns: &[
            Turn { text: "Switch to cache.", language: English },
            Turn { text: "We are refactoring the cache.", language: English },
            Turn { text: "Switch to queue.", language: English },
            Turn { text: "We are migrating the queue.", language: English },
            Turn { text: "Switch to log.", language: English },
            Turn { text: "We are investigating the log.", language: English },
            Turn { text: "Um, let me think.", language: English },
            Turn { text: "Right.", language: English },
            Turn { text: "Return to cache.", language: English },
            Turn { text: "Keep at it only when fresh trials expand production coverage; otherwise ask whether to stop.", language: English },
        ],
        restoration_turn: 9,
        expectation: Expectation::ScopedContinuation { restored_topic: "cache", task_term: "refactoring", forbidden_task_terms: &["queue", "log", "migrating", "investigating"], benefit_term: "coverage" },
    },
    Case {
        id: "R54_08",
        category: "long_cross_language_suspension",
        turns: &[
            Turn { text: "문서 주제로 전환해.", language: Korean },
            Turn { text: "현재 문서를 리팩터링하는 중이야.", language: Korean },
            Turn { text: "Switch to repository.", language: English },
            Turn { text: "We are migrating the repository.", language: English },
            Turn { text: "음, 잠깐만.", language: Korean },
            Turn { text: "응.", language: Korean },
            Turn { text: "Return to the document.", language: English },
            Turn { text: "Continue it only if fresh runs reduce production failures; otherwise report and ask before stopping.", language: English },
        ],
        restoration_turn: 7,
        expectation: Expectation::ScopedContinuation { restored_topic: "document", task_term: "리팩터링", forbidden_task_terms: &["repository", "migrating"], benefit_term: "failure" },
    },
    Case {
        id: "R54_09",
        category: "unseen_english_topic_fails_closed",
        turns: &[
            Turn { text: "Switch to queue.", language: English },
            Turn { text: "We are migrating the queue.", language: English },
            Turn { text: "Switch to report.", language: English },
            Turn { text: "Continue it only if fresh trials expand production coverage; otherwise ask whether to stop.", language: English },
        ],
        restoration_turn: 3,
        expectation: Expectation::MissingTopicTask { restored_topic: "report", forbidden_task_terms: &["queue", "migrating"] },
    },
    Case {
        id: "R54_10",
        category: "unseen_korean_topic_fails_closed",
        turns: &[
            Turn { text: "서버 주제로 전환해.", language: Korean },
            Turn { text: "현재 서버를 테스트하는 중이야.", language: Korean },
            Turn { text: "프로젝트 주제로 전환해.", language: Korean },
            Turn { text: "실제 장애가 줄 때만 그 일을 계속해. 아니면 중단할지 물어.", language: Korean },
        ],
        restoration_turn: 3,
        expectation: Expectation::MissingTopicTask { restored_topic: "프로젝트", forbidden_task_terms: &["서버", "테스트"] },
    },
];

fn main() {
    emit("R54_TOPIC_SCOPED_PRAGMATIC_STATE_DIAGNOSTIC", CASES);
}
