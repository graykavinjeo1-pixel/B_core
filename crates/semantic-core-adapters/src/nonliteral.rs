//! Conservative nonliteral-language analysis.
//!
//! Figurative readings remain adapter hypotheses. They never become semantic
//! concept payloads and never authorize an action by themselves. When literal
//! and figurative readings cannot be separated from context, the adapter asks
//! instead of executing either reading.

use serde::{Deserialize, Serialize};

use crate::pragmatics::PragmaticContextIR;

pub const NONLITERAL_ANALYSIS_SCHEMA: &str = "B_CORE_NONLITERAL_ANALYSIS_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NonliteralKindIR {
    Sarcasm,
    Metaphor,
    Idiom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadingSelectionIR {
    Literal,
    Figurative,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonliteralExpressionIR {
    pub surface_text: String,
    pub kind: NonliteralKindIR,
    pub literal_concept: String,
    pub figurative_concept: String,
    pub selected_reading: ReadingSelectionIR,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonliteralAnalysisIR {
    pub schema: String,
    pub expressions: Vec<NonliteralExpressionIR>,
    pub semantic_incongruity_detected: bool,
    pub literal_execution_blocked: bool,
    pub clarification_required: bool,
}

impl Default for NonliteralAnalysisIR {
    fn default() -> Self {
        Self {
            schema: NONLITERAL_ANALYSIS_SCHEMA.to_string(),
            expressions: Vec::new(),
            semantic_incongruity_detected: false,
            literal_execution_blocked: false,
            clarification_required: false,
        }
    }
}

impl NonliteralAnalysisIR {
    pub fn has_sarcasm(&self) -> bool {
        self.expressions
            .iter()
            .any(|expression| expression.kind == NonliteralKindIR::Sarcasm)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NonliteralAnalyzer;

impl NonliteralAnalyzer {
    pub fn analyze(&self, text: &str, context: &PragmaticContextIR) -> NonliteralAnalysisIR {
        let normalized = normalize(text);
        let mut analysis = NonliteralAnalysisIR::default();
        let negative_state = contains_any(
            &normalized,
            &[
                "깨졌", "실패", "오류", "망가", "최악", "장애", "broken", "failed", "failure",
                "error", "worst", "crashed",
            ],
        );
        let positive_praise = contains_any(
            &normalized,
            &[
                "아주 잘했",
                "참 잘했",
                "퍽이나 잘",
                "대단하네",
                "완벽하네",
                "just great",
                "great job",
                "nice work",
                "perfect",
                "brilliant",
            ],
        );
        if negative_state && positive_praise {
            analysis.expressions.push(NonliteralExpressionIR {
                surface_text: text.trim().to_string(),
                kind: NonliteralKindIR::Sarcasm,
                literal_concept: "C_POSITIVE_EVALUATION".to_string(),
                figurative_concept: "C_NEGATIVE_EVALUATION".to_string(),
                selected_reading: ReadingSelectionIR::Figurative,
                confidence_millis: 880,
            });
            analysis.semantic_incongruity_detected = true;
            analysis.literal_execution_blocked = true;
        }

        for (surface, literal, figurative, kind) in [
            (
                "벽에 부딪혔",
                "C_PHYSICAL_WALL_COLLISION",
                "C_PROGRESS_BLOCKED",
                NonliteralKindIR::Metaphor,
            ),
            (
                "산으로 가",
                "C_PHYSICAL_MOUNTAIN_DIRECTION",
                "C_GOAL_DRIFT",
                NonliteralKindIR::Idiom,
            ),
            (
                "발목을 잡",
                "C_PHYSICAL_ANKLE_GRASP",
                "C_PROGRESS_BLOCKER",
                NonliteralKindIR::Idiom,
            ),
            (
                "hit a wall",
                "C_PHYSICAL_WALL_COLLISION",
                "C_PROGRESS_BLOCKED",
                NonliteralKindIR::Metaphor,
            ),
            (
                "dead end",
                "C_PHYSICAL_ROUTE_TERMINUS",
                "C_NO_PRODUCTIVE_PATH",
                NonliteralKindIR::Metaphor,
            ),
            (
                "bottleneck",
                "C_NARROW_CONTAINER_NECK",
                "C_THROUGHPUT_CONSTRAINT",
                NonliteralKindIR::Metaphor,
            ),
        ] {
            if normalized.contains(surface) {
                analysis.expressions.push(NonliteralExpressionIR {
                    surface_text: surface.to_string(),
                    kind,
                    literal_concept: literal.to_string(),
                    figurative_concept: figurative.to_string(),
                    selected_reading: ReadingSelectionIR::Figurative,
                    confidence_millis: 900,
                });
                analysis.literal_execution_blocked = true;
            }
        }

        if contains_any(&normalized, &["불이 났", "on fire"]) {
            let software_context = contains_any(
                &normalized,
                &[
                    "프로젝트",
                    "코드",
                    "빌드",
                    "배포",
                    "테스트",
                    "저장소",
                    "서버",
                    "project",
                    "code",
                    "build",
                    "deploy",
                    "test",
                    "repository",
                    "server",
                ],
            ) || context.current_task.as_deref().is_some_and(|task| {
                contains_any(
                    &normalize(task),
                    &[
                        "프로젝트",
                        "코드",
                        "build",
                        "project",
                        "migration",
                        "리팩터링",
                    ],
                )
            });
            let physical_context = contains_any(
                &normalized,
                &[
                    "연기",
                    "불꽃",
                    "소방",
                    "건물",
                    "주방",
                    "smoke",
                    "flame",
                    "firefighter",
                    "building",
                    "kitchen",
                ],
            );
            let selected_reading = match (software_context, physical_context) {
                (true, false) => ReadingSelectionIR::Figurative,
                (false, true) => ReadingSelectionIR::Literal,
                _ => ReadingSelectionIR::Ambiguous,
            };
            analysis.expressions.push(NonliteralExpressionIR {
                surface_text: if normalized.contains("불이 났") {
                    "불이 났".to_string()
                } else {
                    "on fire".to_string()
                },
                kind: NonliteralKindIR::Metaphor,
                literal_concept: "C_PHYSICAL_FIRE_EVENT".to_string(),
                figurative_concept: "C_CRITICAL_INCIDENT".to_string(),
                selected_reading,
                confidence_millis: if selected_reading == ReadingSelectionIR::Ambiguous {
                    500
                } else {
                    880
                },
            });
            if selected_reading != ReadingSelectionIR::Literal {
                analysis.literal_execution_blocked = true;
            }
            if selected_reading == ReadingSelectionIR::Ambiguous {
                analysis.clarification_required = true;
            }
        }
        analysis
    }
}

fn contains_any(text: &str, cues: &[&str]) -> bool {
    cues.iter().any(|cue| text.contains(cue))
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contradictory_praise_is_not_literal_approval() {
        let analysis = NonliteralAnalyzer.analyze(
            "테스트가 전부 깨졌네. 아주 잘했어.",
            &PragmaticContextIR::default(),
        );
        assert!(analysis.has_sarcasm());
        assert!(analysis.semantic_incongruity_detected);
        assert!(analysis.literal_execution_blocked);
        assert_eq!(
            analysis.expressions[0].selected_reading,
            ReadingSelectionIR::Figurative
        );
    }

    #[test]
    fn software_context_selects_figurative_fire_reading() {
        let analysis = NonliteralAnalyzer.analyze(
            "배포 뒤 프로젝트에 불이 났어",
            &PragmaticContextIR::default(),
        );
        assert!(!analysis.clarification_required);
        assert_eq!(
            analysis.expressions[0].selected_reading,
            ReadingSelectionIR::Figurative
        );
    }

    #[test]
    fn physical_context_keeps_literal_fire_reading() {
        let analysis = NonliteralAnalyzer.analyze(
            "주방에 연기가 나고 불이 났어",
            &PragmaticContextIR::default(),
        );
        assert_eq!(
            analysis.expressions[0].selected_reading,
            ReadingSelectionIR::Literal
        );
        assert!(!analysis.literal_execution_blocked);
    }

    #[test]
    fn context_free_fire_is_ambiguous_and_requires_clarification() {
        let analysis = NonliteralAnalyzer.analyze("여기 불이 났어", &PragmaticContextIR::default());
        assert!(analysis.clarification_required);
        assert_eq!(
            analysis.expressions[0].selected_reading,
            ReadingSelectionIR::Ambiguous
        );
    }

    #[test]
    fn idiom_maps_to_a_shared_semantic_relation_not_literal_action() {
        let analysis = NonliteralAnalyzer.analyze(
            "이 의존성이 계속 발목을 잡네",
            &PragmaticContextIR::default(),
        );
        assert_eq!(
            analysis.expressions[0].figurative_concept,
            "C_PROGRESS_BLOCKER"
        );
        assert!(analysis.literal_execution_blocked);
    }
}
