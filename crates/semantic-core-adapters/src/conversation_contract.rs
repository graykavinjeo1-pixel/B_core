//! Communicative obligations are not action predicates. This boundary is
//! shared by routing and state ingestion, before any wording is selected.

use std::time::Instant;

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

use crate::native_language_circuit::NativeTurnIR;
use crate::pragmatics::{IllocutionaryForceIR, PragmaticInterpretationIR};
use crate::utterance_intent::ExpectedResponseKindIR;

/// A request's conversational force does not identify its effect. These are
/// requested response obligations, never execution permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestedEffectIR {
    ResponseContent,
    TaskWorkflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEffectBindingIR {
    pub source_id: String,
    pub explicit_request_frame: bool,
    pub canonical_predicate: String,
    pub intent: PlanIntentIR,
    pub effect: RequestedEffectIR,
    pub recipient_node_ids: Vec<String>,
}

pub(crate) fn has_other_recipient(
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    frame_id: &str,
) -> bool {
    use crate::semantic_roles::SemanticRoleKindIR;
    analysis
        .semantic_role_graph
        .arguments_for_frame(frame_id)
        .iter()
        .any(|(role, node)| {
            matches!(
                role,
                SemanticRoleKindIR::Recipient | SemanticRoleKindIR::Destination
            ) && !matches!(
                node.normalized_label.as_str(),
                "me" | "us" | "나" | "저" | "내" | "제" | "우리"
            )
        })
}

fn request_effects(
    text: &str,
    pragmatic: &PragmaticInterpretationIR,
    native: &NativeTurnIR,
) -> Vec<RequestEffectBindingIR> {
    use crate::compositional_semantics::FramePolarityIR;
    use crate::semantic_roles::SemanticRoleKindIR;
    let analysis = &pragmatic.compositional_analysis;
    // A structural override removes the source alternative from the active
    // request set.  For `계획 말고 설명해`, PLAN is superseded while EXPLAIN
    // remains an independently authorized response-content request.  This is
    // graph topology, not a lexical exception for the surface phrase.
    let frame_is_superseded_by_override = |frame_id: &str| {
        pragmatic
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .is_some_and(|composition| {
                composition
                    .nodes
                    .iter()
                    .filter(|node| node.source_frame_id == frame_id)
                    .any(|source| {
                        composition.relations.iter().any(|relation| {
                            relation.kind
                                == crate::pragmatic_intent::PragmaticIntentRelationKindIR::Overrides
                                && relation.source_node_id == source.node_id
                                && composition.selected_node_ids.contains(&relation.target_node_id)
                                && composition
                                    .nodes
                                    .iter()
                                    .find(|node| node.node_id == relation.target_node_id)
                                    .is_some_and(|target| {
                                        target.projection
                                            == crate::pragmatic_intent::PragmaticGoalProjectionIR::AuthorizedRequest
                                    })
                        })
                    })
            })
    };
    // Request effects can only be emitted from an authorized positive surface
    // frame or an already-grounded native live goal.  If neither exists, the
    // later content/question surface scans cannot contribute an effect; avoid
    // repeatedly classifying a long declarative report as a possible request.
    let has_authorized_positive_frame = analysis.frames.iter().any(|frame| {
        frame.external_execution_authorized
            && frame.polarity == FramePolarityIR::Positive
            && !frame.embedded_under_quote
            && !frame_is_superseded_by_override(&frame.frame_id)
    });
    if !has_authorized_positive_frame && native.selected_live_goals.is_empty() {
        return Vec::new();
    }
    let needs_content_classification = analysis.frames.iter().any(|frame| {
        frame.external_execution_authorized
            && frame.polarity == FramePolarityIR::Positive
            && !frame.embedded_under_quote
            && !frame_is_superseded_by_override(&frame.frame_id)
            && matches!(
                frame.intent_hint,
                PlanIntentIR::Communicate | PlanIntentIR::Investigate
            )
    }) || native.selected_live_goals.iter().any(|goal| {
        matches!(
            goal.intent,
            PlanIntentIR::Communicate | PlanIntentIR::Investigate
        )
    });
    let typed_content = needs_content_classification
        && (crate::proposition_content::content_request(text).is_some()
            || crate::proposition_content::question_request(text).is_some());
    let information_force = needs_content_classification
        && pragmatic.illocutionary_commitments.primary_force()
            == Some(IllocutionaryForceIR::AnswerOnlyInformationRequest);
    let mut effects = Vec::new();
    for frame in analysis.frames.iter().filter(|f| {
        f.external_execution_authorized
            && f.polarity == FramePolarityIR::Positive
            && !f.embedded_under_quote
            && !frame_is_superseded_by_override(&f.frame_id)
    }) {
        let typed_content = typed_content
            || crate::proposition_content::question_request_for_frame(
                text,
                analysis,
                &frame.frame_id,
            )
            .is_some();
        let recipients = analysis
            .semantic_role_graph
            .arguments_for_frame(&frame.frame_id)
            .into_iter()
            .filter(|(role, _)| {
                matches!(
                    role,
                    SemanticRoleKindIR::Recipient | SemanticRoleKindIR::Destination
                )
            })
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        let other_recipient = has_other_recipient(analysis, &frame.frame_id);
        effects.push(RequestEffectBindingIR {
            source_id: frame.frame_id.clone(),
            explicit_request_frame: true,
            canonical_predicate: frame.canonical_predicate.clone(),
            intent: frame.intent_hint,
            effect: if !other_recipient
                && (frame.intent_hint == PlanIntentIR::Explain
                    || typed_content
                        && matches!(
                            frame.intent_hint,
                            PlanIntentIR::Communicate | PlanIntentIR::Investigate
                        )
                    || information_force
                        && matches!(
                            frame.intent_hint,
                            PlanIntentIR::Investigate
                                | PlanIntentIR::Plan
                                | PlanIntentIR::Communicate
                        ))
            {
                RequestedEffectIR::ResponseContent
            } else {
                RequestedEffectIR::TaskWorkflow
            },
            recipient_node_ids: recipients.iter().map(|n| n.node_id.clone()).collect(),
        });
    }
    // Native ellipsis can have no live surface frame. Retain its typed goal;
    // never let failure to select one native goal erase other active frames.
    for goal in &native.selected_live_goals {
        if !effects
            .iter()
            .any(|e| e.canonical_predicate == goal.canonical_predicate)
        {
            effects.push(RequestEffectBindingIR {
                source_id: goal.goal_id.clone(),
                explicit_request_frame: false,
                canonical_predicate: goal.canonical_predicate.clone(),
                intent: goal.intent,
                effect: if goal.intent == PlanIntentIR::Explain
                    || typed_content
                        && matches!(
                            goal.intent,
                            PlanIntentIR::Communicate | PlanIntentIR::Investigate
                        )
                    || information_force
                        && matches!(
                            goal.intent,
                            PlanIntentIR::Investigate
                                | PlanIntentIR::Plan
                                | PlanIntentIR::Communicate
                        ) {
                    RequestedEffectIR::ResponseContent
                } else {
                    RequestedEffectIR::TaskWorkflow
                },
                recipient_node_ids: Vec::new(),
            });
        }
    }
    effects
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationContractIR {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prohibited_response_frames: Vec<String>,
    pub request_effects: Vec<RequestEffectBindingIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_preference: Option<crate::proposition_content::InteractionPreferenceIR>,
    pub information_requested: bool,
    pub explanation_requested: bool,
    pub independent_action_requested: bool,
    pub question_surface: bool,
    pub assertion_only: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConversationContractTimingIR {
    pub initial_prohibition_micros: u64,
    pub local_prohibition_fallback_micros: u64,
    pub request_effects_micros: u64,
    pub remaining_derivation_micros: u64,
}

impl ConversationContractIR {
    pub fn derive(
        text: &str,
        pragmatic: &PragmaticInterpretationIR,
        native: &NativeTurnIR,
    ) -> Self {
        Self::derive_inner(text, pragmatic, native, None)
    }

    pub(crate) fn derive_profiled(
        text: &str,
        pragmatic: &PragmaticInterpretationIR,
        native: &NativeTurnIR,
    ) -> (Self, ConversationContractTimingIR) {
        let mut timing = ConversationContractTimingIR::default();
        let contract = Self::derive_inner(text, pragmatic, native, Some(&mut timing));
        (contract, timing)
    }

    fn derive_inner(
        text: &str,
        pragmatic: &PragmaticInterpretationIR,
        native: &NativeTurnIR,
        mut timing: Option<&mut ConversationContractTimingIR>,
    ) -> Self {
        let initial_prohibition_started = Instant::now();
        let mut prohibited_response_frames =
            response_prohibition(text, &pragmatic.compositional_analysis);
        if let Some(timing) = timing.as_deref_mut() {
            timing.initial_prohibition_micros = initial_prohibition_started.elapsed().as_micros() as u64;
        }
        // A partially understood multi-clause request must not fall through to
        // whole-record quotation and expose explicitly prohibited content.
        // Supported response batches enforce their slot constraints locally;
        // otherwise preserve the negative act and suppress this answer turn.
        // The fallback below is only capable of adding a prohibition for a
        // negative, independently directive clause.  A negative descriptive
        // clause in a long incident report cannot satisfy that predicate, so
        // do not run its whole-surface batch and lifecycle classifiers first.
        // This keeps the exact scoped fallback for a viable directive while
        // avoiding unrelated reparsing work on ordinary negative reports.
        let local_prohibition_fallback_started = Instant::now();
        let has_negative_independent_directive_candidate = pragmatic
            .compositional_analysis
            .frames
            .iter()
            .any(|frame| {
                frame.polarity == crate::compositional_semantics::FramePolarityIR::Negative
                    && !frame.embedded_under_quote
                    && pragmatic
                        .compositional_analysis
                        .clause_graph
                        .node_for_frame(&frame.frame_id)
                        .is_some_and(|node| node.function.permits_independent_directive())
            });
        if prohibited_response_frames.is_empty()
            && has_negative_independent_directive_candidate
            && !crate::discourse_qa::is_response_operation_batch(text)
            && crate::plan_result_boundary::classify_plan_result_query_focus(text)
                == crate::plan_result_boundary::PlanResultQueryFocusIR::None
        {
            for frame in &pragmatic.compositional_analysis.frames {
                let Some(node) = pragmatic
                    .compositional_analysis
                    .clause_graph
                    .node_for_frame(&frame.frame_id)
                else {
                    continue;
                };
                if node.function.permits_independent_directive()
                    && !frame.embedded_under_quote
                    && frame.polarity == crate::compositional_semantics::FramePolarityIR::Negative
                    && response_prohibition_in_existing_clause(
                        &pragmatic.compositional_analysis,
                        frame,
                        node,
                    )
                {
                    prohibited_response_frames.push(frame.frame_id.clone());
                }
            }
        }
        if let Some(timing) = timing.as_deref_mut() {
            timing.local_prohibition_fallback_micros =
                local_prohibition_fallback_started.elapsed().as_micros() as u64;
        }
        let request_effects_started = Instant::now();
        let request_effects = request_effects(text, pragmatic, native);
        if let Some(timing) = timing.as_deref_mut() {
            timing.request_effects_micros = request_effects_started.elapsed().as_micros() as u64;
        }
        let remaining_derivation_started = Instant::now();
        let response_effect_only = !request_effects.is_empty()
            && request_effects
                .iter()
                .all(|e| e.effect == RequestedEffectIR::ResponseContent);
        let has_task_effect = request_effects
            .iter()
            .any(|e| e.effect == RequestedEffectIR::TaskWorkflow);
        let interaction_preference = crate::proposition_content::interaction_preference(text);
        let interaction_only = interaction_preference.as_ref().is_some_and(|p| {
            p.owns_response()
                && (p.desired != crate::proposition_content::InteractionModeIR::Concise
                    || request_effects.is_empty())
        });
        let force = pragmatic.illocutionary_commitments.primary_force();
        let decision_support = pragmatic
            .pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|i| i.expected_response == ExpectedResponseKindIR::DecisionSupport);
        let question_surface = is_interrogative(text);
        let explanation_requested = !interaction_only
            && (request_effects.iter().any(|effect| {
                effect.explicit_request_frame
                    && effect.effect == RequestedEffectIR::ResponseContent
                    && effect.intent == PlanIntentIR::Explain
            }) || interaction_preference.as_ref().is_some_and(|p| {
                matches!(
                    p.desired,
                    crate::proposition_content::InteractionModeIR::Explanation
                        | crate::proposition_content::InteractionModeIR::Summary
                )
            }) || crate::discourse_qa::is_answer_reformulation(text)
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
                    })));
        let event_question = crate::proposition_content::is_event_question(text);
        let indirect_action = !event_question
            && !decision_support
            && !response_effect_only
            && force == Some(IllocutionaryForceIR::IndirectActionRequest);
        let information_requested = !interaction_only
            && (response_effect_only
                || request_effects
                    .iter()
                    .any(|effect| effect.effect == RequestedEffectIR::ResponseContent)
                || event_question
                || decision_support
                || explanation_requested
                || force == Some(IllocutionaryForceIR::AnswerOnlyInformationRequest)
                || (question_surface && !indirect_action));
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
        // Descriptive vocabulary is wider than the planning predicate set.
        // A complete source-bound event supplies assertion evidence even when
        // a surface cue (e.g. an action noun) nominated RequestAction.
        let source_event_report = !question_surface
            && !has_task_effect
            && crate::proposition_content::is_event_report(text);
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
                || (source_event_report && native.selected_live_goals.is_empty())
                || (problem_statement && native.selected_live_goals.is_empty())
                || (native.selected_live_goals.is_empty()
                    && pragmatic.speech_act == crate::pragmatics::SpeechActIR::Inform));
        let modal = &pragmatic.compositional_analysis.modal_scope_graph;
        let conditional_report = !question_surface
            && !text.contains(['"', '“', '”', '`'])
            && (pragmatic.continuation_gate.is_none()
                || text.split_once(". ").is_some_and(|(activity, _)| {
                    activity.trim().ends_with("고 있어") || activity.trim().ends_with("고 있어요")
                }))
            && !selected_request
            && !indirect_action
            && !modal.conditionals.is_empty()
            && modal
                .conditionals
                .iter()
                .all(|c| !c.consequent_is_directive)
            // Pragmatic adapters can nominate a diagnostic from an action noun.
            // That nomination cannot turn a declarative activity report into
            // an independent imperative. Actual indirect requests were excluded above.
            && frames.iter().all(|f| !f.external_execution_authorized
                || matches!(f.mood, crate::compositional_semantics::FrameMoodIR::Declarative | crate::compositional_semantics::FrameMoodIR::Reported));
        let assertion_only = assertion_only || conditional_report || interaction_only;
        let independent_imperative = request_effects
            .iter()
            .any(|e| e.effect == RequestedEffectIR::TaskWorkflow && e.explicit_request_frame);
        let independent_action_requested = !event_question
            && !decision_support
            && !assertion_only
            && (indirect_action
                || independent_imperative
                || (!information_requested
                    && (selected_request
                        || problem_disclosure
                        || !native.selected_live_goals.is_empty())));
        let evidence = [
            (source_event_report && assertion_only).then_some("SOURCE_BOUND_EVENT_REPORT"),
            crate::grammatical_scope::embedded_information_statement(text)
                .is_some()
                .then_some("EMBEDDED_INFORMATION_UNDER_STATEMENT"),
            interaction_only.then_some("TYPED_INTERACTION_PREFERENCE"),
            event_question.then_some("TYPED_EVENT_QUERY_NOT_ACTION"),
            conditional_report.then_some("CONDITIONAL_REPORT"),
            question_surface.then_some("INTERROGATIVE_SCOPE"),
            explanation_requested.then_some("ANSWER_CONTENT_REQUIRED"),
            response_effect_only.then_some("RESPONSE_EFFECT_NOT_TASK_EXECUTION"),
            has_task_effect.then_some("TYPED_TASK_EFFECT"),
            indirect_action.then_some("TYPED_INDIRECT_ACTION_REQUEST"),
            information_requested.then_some("QUESTION_IS_NOT_AN_OUTCOME_REPORT"),
            decision_support.then_some("DECISION_SUPPORT_NOT_EXECUTION"),
        ]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect();
        let contract = Self {
            information_requested: information_requested && prohibited_response_frames.is_empty(),
            explanation_requested: explanation_requested && prohibited_response_frames.is_empty(),
            independent_action_requested: independent_action_requested
                && prohibited_response_frames.is_empty(),
            prohibited_response_frames,
            request_effects,
            interaction_preference,
            question_surface,
            assertion_only,
            evidence,
        };
        if let Some(timing) = timing.as_deref_mut() {
            timing.remaining_derivation_micros =
                remaining_derivation_started.elapsed().as_micros() as u64;
        }
        contract
    }

    pub fn answer_only(&self) -> bool {
        self.information_requested && !self.independent_action_requested
    }

    pub fn suppresses_answer(&self) -> bool {
        !self.prohibited_response_frames.is_empty()
    }

    pub fn interaction_only(&self) -> bool {
        self.interaction_preference.as_ref().is_some_and(|p| {
            p.owns_response()
                && (p.desired != crate::proposition_content::InteractionModeIR::Concise
                    || self.request_effects.is_empty())
        })
    }
}

/// A fully scoped negative matrix information act is an output constraint,
/// not a positive question merely because its complement contains a wh word.
pub(crate) fn response_prohibition(
    text: &str,
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
) -> Vec<String> {
    use crate::clause_graph::ClauseFunctionIR;
    use crate::compositional_semantics::FramePolarityIR;
    if text.contains(['"', '“', '”', '‘', '’', '`']) {
        return vec![];
    }
    let roots = analysis
        .frames
        .iter()
        .filter(|f| {
            analysis
                .clause_graph
                .node_for_frame(&f.frame_id)
                .is_some_and(|n| n.function.permits_independent_directive())
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return vec![];
    }
    let root = roots[0];
    let node = analysis
        .clause_graph
        .node_for_frame(&root.frame_id)
        .unwrap();
    let clean = |s: &str| {
        s.trim()
            .trim_end_matches(['.', '?', '!'])
            .trim()
            .to_lowercase()
    };
    if root.embedded_under_quote
        || root.polarity != FramePolarityIR::Negative
        || !matches!(
            root.intent_hint,
            PlanIntentIR::Explain | PlanIntentIR::Communicate | PlanIntentIR::Investigate
        )
        || clean(&node.source_text) != clean(text)
        || analysis.frames.iter().any(|f| {
            f.frame_id != root.frame_id
                && analysis
                    .clause_graph
                    .node_for_frame(&f.frame_id)
                    .is_none_or(|n| n.function != ClauseFunctionIR::ContentComplement)
        })
    {
        return vec![];
    }
    let surface = clean(text);
    let directive =
        crate::compositional_semantics::strip_conversational_directive_lead_in(&surface);
    let english = directive.strip_prefix("please ").unwrap_or(directive);
    let prohibitive = ["do not ", "don't ", "never "]
        .iter()
        .any(|p| english.starts_with(p))
        || ["마세요", "마십시오", "말아주세요", "말아줘", "마"]
            .iter()
            .any(|ending| {
                surface
                    .strip_suffix(ending)
                    .is_some_and(|p| p.trim_end().ends_with('지'))
            });
    if prohibitive {
        vec![root.frame_id.clone()]
    } else {
        vec![]
    }
}

/// Evaluates the same scoped prohibition predicate as `response_prohibition`
/// for one clause that already belongs to the authoritative whole-turn parse.
/// The legacy fallback reparsed `node.source_text` just to recover this local
/// view.  Restricting the existing analysis to the clause's byte span retains
/// the same local roots and content-complement guard without a second parse.
fn response_prohibition_in_existing_clause(
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    candidate: &crate::compositional_semantics::PredicateFrameIR,
    candidate_node: &crate::clause_graph::ClauseNodeIR,
) -> bool {
    use crate::clause_graph::ClauseFunctionIR;
    use crate::compositional_semantics::FramePolarityIR;

    if candidate_node.source_text.contains(['"', '“', '”', '‘', '’', '`']) {
        return false;
    }
    let local_frames = analysis
        .frames
        .iter()
        .filter(|frame| {
            frame.source_start_byte >= candidate_node.source_start_byte
                && frame.source_start_byte < candidate_node.source_end_byte
        })
        .collect::<Vec<_>>();
    let roots = local_frames
        .iter()
        .copied()
        .filter(|frame| {
            analysis
                .clause_graph
                .node_for_frame(&frame.frame_id)
                .is_some_and(|node| node.function.permits_independent_directive())
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 || roots[0].frame_id != candidate.frame_id {
        return false;
    }
    if candidate.embedded_under_quote
        || candidate.polarity != FramePolarityIR::Negative
        || !matches!(
            candidate.intent_hint,
            PlanIntentIR::Explain | PlanIntentIR::Communicate | PlanIntentIR::Investigate
        )
        || local_frames.iter().any(|frame| {
            frame.frame_id != candidate.frame_id
                && analysis
                    .clause_graph
                    .node_for_frame(&frame.frame_id)
                    .is_none_or(|node| node.function != ClauseFunctionIR::ContentComplement)
        })
    {
        return false;
    }
    let surface = candidate_node
        .source_text
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .trim()
        .to_lowercase();
    let directive =
        crate::compositional_semantics::strip_conversational_directive_lead_in(&surface);
    let english = directive.strip_prefix("please ").unwrap_or(directive);
    ["do not ", "don't ", "never "]
        .iter()
        .any(|prefix| english.starts_with(prefix))
        || ["마세요", "마십시오", "말아주세요", "말아줘", "마"]
            .iter()
            .any(|ending| {
                surface
                    .strip_suffix(ending)
                    .is_some_and(|prefix| prefix.trim_end().ends_with('지'))
            })
}

/// Grammatical question cues, never subject/domain lookup or whole-utterance
/// dispatch. Embedded wh-complements without a question ending are not questions.
/// WH expressions denote open query variables, not grounded entity names.
/// Shared by the native planner and world-atom boundary.
pub(crate) fn is_interrogative_placeholder(subject: &str) -> bool {
    matches!(
        subject.trim().to_lowercase().as_str(),
        "what"
            | "who"
            | "where"
            | "when"
            | "why"
            | "how"
            | "which"
            | "무엇"
            | "뭐"
            | "누구"
            | "어디"
            | "언제"
            | "왜"
            | "어떻게"
            | "무슨"
    )
}

pub fn is_interrogative(text: &str) -> bool {
    if crate::grammatical_scope::embedded_information_statement(text).is_some() {
        return false;
    }
    if crate::discourse_qa::quotative_content_question(text) {
        return true;
    }
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

    #[test]
    fn existing_clause_prohibition_matches_the_legacy_local_reparse() {
        let analyzer = crate::compositional_semantics::CompositionalSemanticAnalyzer;
        for source in [
            "현재 장애는 조사 중입니다. 고객에게 결제 실패가 확정됐다고 안내하지 마세요.",
            "The incident is under investigation. Do not tell customers that payment failure is confirmed.",
        ] {
            let analysis = analyzer.analyze(source);
            for frame in analysis.frames.iter().filter(|frame| {
                frame.polarity == crate::compositional_semantics::FramePolarityIR::Negative
                    && !frame.embedded_under_quote
            }) {
                let Some(node) = analysis.clause_graph.node_for_frame(&frame.frame_id) else {
                    continue;
                };
                if !node.function.permits_independent_directive() {
                    continue;
                }
                let legacy = !response_prohibition(&node.source_text, &analyzer.analyze(&node.source_text))
                    .is_empty();
                assert_eq!(
                    response_prohibition_in_existing_clause(&analysis, frame, node),
                    legacy,
                    "source={source}; clause={}",
                    node.source_text
                );
            }
        }
    }
}
