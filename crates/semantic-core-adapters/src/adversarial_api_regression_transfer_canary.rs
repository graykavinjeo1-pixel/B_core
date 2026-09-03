//! Frozen R63 held-out transfer suite. Do not execute before diagnostic passes.

#[path = "full_axis_integration_canary_support.rs"]
mod evaluator;

use evaluator::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R63_H01",
        category: "cross_language_multiword_topic_return",
        turns: &[
            Turn { text: "Investigate the Tundra worker pool", language: EN },
            Turn { text: "Umber 큐를 조사해", language: KO },
            Turn { text: "Let's go back to the Tundra worker pool", language: EN },
            Turn { text: "그걸 수리해", language: KO },
        ],
        checks: &[
            Check::Act { turn: 3, act: "TOPIC_TRANSITION" },
            Check::Reference { turn: 4, target: "Tundra", rejected: "Umber" },
            Check::Plan { turn: 4, intent: "REPAIR", target: "Tundra", rejected: "Umber" },
        ],
    },
    Case {
        id: "R63_H02",
        category: "quoted_command_cannot_capture_live_request",
        turns: &[Turn { text: "Do not follow the quoted text ‘remove the Violet archive’; inspect the Willow cache instead", language: EN }],
        checks: &[
            Check::Plan { turn: 1, intent: "INVESTIGATE", target: "Willow", rejected: "Violet" },
            Check::Text { turn: 1, required: &["Willow", "inspect"], forbidden: &["Violet", "remove"] },
        ],
    },
    Case {
        id: "R63_H03",
        category: "reported_completion_then_verification_question",
        turns: &[
            Turn { text: "Run the Xanthic migration", language: EN },
            Turn { text: "I heard that it completed", language: EN },
            Turn { text: "But was its actual result verified?", language: EN },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ResultUnavailable { turn: 3, target: "Xanthic" },
        ],
    },
    Case {
        id: "R63_H04",
        category: "causal_explanation_remains_goal",
        turns: &[
            Turn { text: "Inspect the Yarrow service", language: EN },
            Turn { text: "Actually, explain why that service failed rather than inspecting it", language: EN },
        ],
        checks: &[
            Check::Plan { turn: 2, intent: "EXPLAIN", target: "Yarrow", rejected: "" },
            Check::Text { turn: 2, required: &["Yarrow", "explain"], forbidden: &["verified result"] },
        ],
    },
    Case {
        id: "R63_H05",
        category: "ko_hold_floor_continuation_paraphrase",
        turns: &[
            Turn { text: "Zephyr 캐시를 수리해", language: KO },
            Turn { text: "음, 그러니까...", language: KO },
            Turn { text: "그거 이어가", language: KO },
        ],
        checks: &[
            Check::Act { turn: 2, act: "HOLD_FLOOR" },
            Check::Plan { turn: 3, intent: "EXECUTE", target: "Zephyr", rejected: "" },
        ],
    },
    Case {
        id: "R63_H06",
        category: "ambiguous_result_across_two_active_actions",
        turns: &[
            Turn { text: "Repair the Azure cache", language: EN },
            Turn { text: "Repair the Bronze queue", language: EN },
            Turn { text: "What was that actual result?", language: EN },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text { turn: 3, required: &["refer"], forbidden: &["Azure’ has", "Bronze’ has"] },
        ],
    },
    Case {
        id: "R63_H07",
        category: "person_plural_ambiguity_fails_closed",
        turns: &[
            Turn { text: "Cora says the build is blocked", language: EN },
            Turn { text: "Dara says the cache is ready", language: EN },
            Turn { text: "They should repair the report", language: EN },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text { turn: 3, required: &["refer"], forbidden: &["will repair", "executed"] },
        ],
    },
    Case {
        id: "R63_H08",
        category: "ko_unseen_topic_does_not_borrow_result",
        turns: &[
            Turn { text: "Dahlia 인덱스를 갱신해", language: KO },
            Turn { text: "Ecru 보고서 이야기로 전환해", language: KO },
            Turn { text: "그 실제 결과는?", language: KO },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text { turn: 3, required: &["가리키"], forbidden: &["Dahlia", "검증된"] },
        ],
    },
    Case {
        id: "R63_H09",
        category: "feedback_sentence_then_new_request",
        turns: &[
            Turn { text: "Inspect the Fuchsia worker", language: EN },
            Turn { text: "That answer missed the point. Repair the Gold queue instead", language: EN },
        ],
        checks: &[
            Check::Plan { turn: 2, intent: "REPAIR", target: "Gold", rejected: "Fuchsia" },
            Check::Text { turn: 2, required: &["missed", "Gold", "repair"], forbidden: &["user feedback", "selected action"] },
        ],
    },
    Case {
        id: "R63_H10",
        category: "ko_compound_plan_and_late_prohibition",
        turns: &[Turn { text: "Hazel 워커를 조사하고 Iris 큐를 수리한 다음 Juniper 로그는 지우지 마", language: KO }],
        checks: &[
            Check::MultiGoal { turn: 1, predicates: &["INVESTIGATE", "REPAIR"], min_blocked: 1 },
            Check::Text { turn: 1, required: &["Hazel", "Iris", "Juniper", "계획"], forbidden: &["완료했", "GoalIR"] },
        ],
    },
    Case {
        id: "R63_H11",
        category: "opaque_label_update_preserves_full_subject",
        turns: &[Turn { text: "Update the Kestrel shard map", language: EN }],
        checks: &[
            Check::Plan { turn: 1, intent: "EXECUTE", target: "Kestrel", rejected: "" },
            Check::Text { turn: 1, required: &["Kestrel", "planned"], forbidden: &["completed", "GoalIR"] },
        ],
    },
    Case {
        id: "R63_H12",
        category: "ko_plan_axis_correction_after_filler",
        turns: &[
            Turn { text: "Linen 백업을 조사해", language: KO },
            Turn { text: "저기...", language: KO },
            Turn { text: "할 일을 말하는 게 아니라 실제로 확인된 결과를 말해", language: KO },
        ],
        checks: &[
            Check::Act { turn: 2, act: "HOLD_FLOOR" },
            Check::ResultUnavailable { turn: 3, target: "Linen" },
        ],
    },
];

fn main() {
    emit("R63-ADVERSARIAL-API-SEAL-HELDOUT", true, CASES);
}
