//! Communicative obligations are not action predicates. This boundary is
//! shared by routing and state ingestion, before any wording is selected.

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

use crate::native_language_circuit::NativeTurnIR;
use crate::pragmatics::{IllocutionaryForceIR, PragmaticInterpretationIR};
use crate::utterance_intent::ExpectedResponseKindIR;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationContractIR {
    pub information_requested: bool,
    pub explanation_requested: bool,
    pub independent_action_requested: bool,
    pub question_surface: bool,
    pub assertion_only: bool,
    pub evidence: Vec<String>,
}

impl ConversationContractIR {
    pub fn derive(
        text: &str,
        pragmatic: &PragmaticInterpretationIR,
        native: &NativeTurnIR,
    ) -> Self {
        let force = pragmatic.illocutionary_commitments.primary_force();
        let question_surface = is_interrogative(text);
        let explanation_requested = crate::discourse_qa::is_answer_reformulation(text)
            || pragmatic
                .pragmatic_intent_graph
                .selected_utterance_intent()
                .is_some_and(|intent| {
                    matches!(
                        intent.expected_response,
                        ExpectedResponseKindIR::Explanation
                            | ExpectedResponseKindIR::Summary
                            | ExpectedResponseKindIR::Evidence
                    )
                })
            || (!native.selected_live_goals.is_empty()
                && native.selected_live_goals.iter().all(|goal| {
                    matches!(
                        goal.intent,
                        PlanIntentIR::Explain | PlanIntentIR::Communicate
                    )
                }));
        let indirect_action = force == Some(IllocutionaryForceIR::IndirectActionRequest);
        let information_requested = explanation_requested
            || force == Some(IllocutionaryForceIR::AnswerOnlyInformationRequest)
            || (question_surface && !indirect_action);
        let frames = &pragmatic.compositional_analysis.frames;
        let asserted_frames = !frames.is_empty()
            && frames.iter().all(|frame| {
                matches!(
                    frame.mood,
                    crate::compositional_semantics::FrameMoodIR::Declarative
                        | crate::compositional_semantics::FrameMoodIR::Reported
                )
            });
        let selected_request = pragmatic
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .is_some_and(|graph| {
                graph.nodes.iter().any(|node| {
                    graph.selected_node_ids.contains(&node.node_id)
                        && node.projection
                            == crate::pragmatic_intent::PragmaticGoalProjectionIR::AuthorizedRequest
                })
            })
            && native.reference_bindings.iter().any(|binding| {
                matches!(
                    binding.kind,
                    crate::native_language_circuit::NativeReferenceKindIR::ExplicitPriorTheme
                        | crate::native_language_circuit::NativeReferenceKindIR::OperationEllipsis
                )
            });
        let problem_statement = pragmatic
            .pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|intent| {
                intent.communicative_intent
                    == crate::utterance_intent::CommunicativeIntentIR::ProblemDisclosure
            });
        let problem_disclosure = problem_statement
            && pragmatic
                .inferred_goal
                .as_ref()
                .is_some_and(|goal| goal.intent == PlanIntentIR::Repair);
        let assertion_only = !information_requested
            && !indirect_action
            && !selected_request
            && !problem_disclosure
            && pragmatic.continuation_gate.is_none()
            && !pragmatic
                .nonliteral_analysis
                .expressions
                .iter()
                .any(|expression| {
                    expression.selected_reading == crate::nonliteral::ReadingSelectionIR::Figurative
                })
            && !(frames.is_empty()
                && matches!(
                    pragmatic.inferred_goal.as_ref().map(|goal| goal.intent),
                    Some(PlanIntentIR::Repair)
                ))
            && (asserted_frames
                || (problem_statement && native.selected_live_goals.is_empty())
                || (native.selected_live_goals.is_empty()
                    && pragmatic.speech_act == crate::pragmatics::SpeechActIR::Inform));
        let independent_imperative = native.selected_live_goals.iter().any(|goal| {
            !matches!(
                goal.intent,
                PlanIntentIR::Explain | PlanIntentIR::Communicate
            ) && frames.iter().any(|frame| {
                frame.canonical_predicate == goal.canonical_predicate
                    && frame.external_execution_authorized
            })
        });
        let independent_action_requested = !assertion_only
            && (indirect_action
                || independent_imperative
                || (!information_requested
                    && (selected_request
                        || problem_disclosure
                        || !native.selected_live_goals.is_empty())));
        let evidence = [
            question_surface.then_some("INTERROGATIVE_SCOPE"),
            explanation_requested.then_some("ANSWER_CONTENT_REQUIRED"),
            indirect_action.then_some("TYPED_INDIRECT_ACTION_REQUEST"),
            information_requested.then_some("QUESTION_IS_NOT_AN_OUTCOME_REPORT"),
        ]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect();
        Self {
            information_requested,
            explanation_requested,
            independent_action_requested,
            question_surface,
            assertion_only,
            evidence,
        }
    }

    pub fn answer_only(&self) -> bool {
        self.information_requested && !self.independent_action_requested
    }
}

/// Grammatical question cues, never subject/domain lookup or whole-utterance
/// dispatch. Embedded wh-complements without a question ending are not questions.
pub fn is_interrogative(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    let last = text
        .trim_end_matches(['.', '!'])
        .split(['.', '!', ';', '\n'])
        .next_back()
        .unwrap_or(&text)
        .trim();
    let words = last.split_whitespace().collect::<Vec<_>>();
    let first = words.first().copied().unwrap_or("");
    if matches!(
        first,
        "why"
            | "who"
            | "whom"
            | "whose"
            | "what"
            | "which"
            | "where"
            | "when"
            | "how"
            | "왜"
            | "누가"
            | "뭘"
            | "언제"
            | "어디"
            | "어떻게"
    ) {
        return true;
    }
    if matches!(
        first,
        "is" | "are"
            | "was"
            | "were"
            | "has"
            | "have"
            | "had"
            | "did"
            | "does"
            | "can"
            | "could"
            | "would"
            | "should"
    ) && words.len() >= 2
    {
        return true;
    }
    if first == "do"
        && matches!(words.get(1).copied(), Some("you" | "we" | "they" | "i"))
        && words.len() >= 3
    {
        return true;
    }
    last.ends_with(['?', '？'])
        || ["인가", "인가요", "나요", "까요", "는지", "는가"]
            .iter()
            .any(|ending| last.ends_with(ending))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_do_not_depend_on_domain_or_sentence_length() {
        for subject in ["cache", "scheduler", "zorb", "Q17"] {
            for prefix in ["Why did", "Who changed", "Has", "What is"] {
                assert!(is_interrogative(&format!("{prefix} {subject}?")));
            }
            assert!(!is_interrogative(&format!("Explain why {subject} failed.")));
            assert!(!is_interrogative(&format!("Do not modify {subject}.")));
        }
        assert!(is_interrogative("왜 실패했어?"));
        assert!(is_interrogative("누가 수정했어?"));
        assert!(!is_interrogative("민수가 수정했어."));
    }
}
