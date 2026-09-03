//! Typed pragmatic-force inference over compositional predicate frames.
//!
//! Surface mood is evidence, not semantic authority. This module separates
//! conventional requests, preferences, suggestions, rhetorical evaluations,
//! information questions, self-offers, metalinguistic mentions, and goal
//! corrections before any candidate may become a planning goal.

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clause_graph::{ClauseFunctionIR, ClauseRelationKindIR};
use crate::compositional_semantics::{
    pragmatic_action_mentions, CandidateDispositionIR, CompositionalAnalysisIR,
    CompositionalGoalEdgeIR, CompositionalGoalGraphIR, CompositionalGoalNodeIR, FrameModalityIR,
    FrameMoodIR, FramePolarityIR, GoalGraphRelationKindIR, InterpretationCandidateIR,
    PragmaticActionMentionIR, PredicateFrameIR,
};
use crate::native_language_circuit::NativeTurnIR;
use crate::utterance_intent::{
    CommunicativeIntentIR, ExpectedResponseKindIR, UtteranceIntentAnalyzer, UtteranceIntentGraphIR,
};

pub const PRAGMATIC_INTENT_GRAPH_SCHEMA: &str = "B_CORE_PRAGMATIC_INTENT_GRAPH_IR_1";
pub const COMPOSITIONAL_PRAGMATIC_GRAPH_SCHEMA: &str = "B_CORE_COMPOSITIONAL_PRAGMATIC_GRAPH_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticIntentKindIR {
    ConventionalIndirectRequest,
    PreferenceRequest,
    AdvisorySuggestion,
    RhetoricalEvaluation,
    InformationQuestion,
    SelfOffer,
    MetalinguisticMention,
    GoalCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticGoalProjectionIR {
    AuthorizedRequest,
    AdvisoryOnly,
    Suppressed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticIntentInferenceIR {
    pub kind: PragmaticIntentKindIR,
    pub projection: PragmaticGoalProjectionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_predicate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<PlanIntentIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_frame_id: Option<String>,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticClauseForceIR {
    DirectRequest,
    ConventionalIndirectRequest,
    PreferenceRequest,
    AdvisorySuggestion,
    RhetoricalEvaluation,
    InformationQuestion,
    CapabilityQuestion,
    SelfOffer,
    MetalinguisticMention,
    GoalCorrection,
    Prohibition,
    ReportedSuggestion,
    DescriptiveMention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticIntentRelationKindIR {
    Supports,
    Conditions,
    Contrasts,
    Overrides,
    Corrects,
    Prohibits,
    Alternative,
    Sequences,
    Coordinates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticClauseIntentIR {
    pub node_id: String,
    pub clause_id: String,
    pub source_frame_id: String,
    pub source_text: String,
    pub force: PragmaticClauseForceIR,
    pub projection: PragmaticGoalProjectionIR,
    pub canonical_predicate: String,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub clause_function: ClauseFunctionIR,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticIntentRelationIR {
    pub relation_id: String,
    pub kind: PragmaticIntentRelationKindIR,
    pub source_node_id: String,
    pub target_node_id: String,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticContextScopeIR {
    pub scope_id: String,
    pub kind: PragmaticIntentRelationKindIR,
    pub target_node_id: String,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionalPragmaticGraphIR {
    pub schema: String,
    pub nodes: Vec<PragmaticClauseIntentIR>,
    pub relations: Vec<PragmaticIntentRelationIR>,
    pub context_scopes: Vec<PragmaticContextScopeIR>,
    pub selected_node_ids: Vec<String>,
    pub suppressed_node_ids: Vec<String>,
    pub unresolved_ambiguities: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
    pub graph_sha256: String,
}

impl CompositionalPragmaticGraphIR {
    fn seal(mut self) -> Self {
        self.graph_sha256.clear();
        self.graph_sha256 = self.computed_hash();
        self
    }

    fn computed_hash(&self) -> String {
        let mut canonical = self.clone();
        canonical.graph_sha256.clear();
        let bytes = serde_json::to_vec(&canonical).expect("compositional pragmatic graph");
        format!("{:x}", Sha256::digest(bytes))
    }

    pub fn validate(&self) -> bool {
        if self.schema != COMPOSITIONAL_PRAGMATIC_GRAPH_SCHEMA
            || self.semantic_authority
            || self.external_action_execution_authorized
            || self.graph_sha256.len() != 64
            || self.graph_sha256 != self.computed_hash()
        {
            return false;
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        node_ids.len() == self.nodes.len()
            && self.nodes.iter().all(|node| {
                !node.node_id.is_empty()
                    && !node.clause_id.is_empty()
                    && !node.source_frame_id.is_empty()
                    && !node.canonical_predicate.is_empty()
                    && !node.subject.trim().is_empty()
                    && !node.semantic_authority
                    && !node.external_action_execution_authorized
                    && node.confidence_millis <= 1_000
            })
            && self
                .selected_node_ids
                .iter()
                .all(|node_id| node_ids.contains(node_id.as_str()))
            && self
                .suppressed_node_ids
                .iter()
                .all(|node_id| node_ids.contains(node_id.as_str()))
            && self.relations.iter().all(|relation| {
                node_ids.contains(relation.source_node_id.as_str())
                    && node_ids.contains(relation.target_node_id.as_str())
                    && relation.source_node_id != relation.target_node_id
                    && relation.confidence_millis <= 1_000
            })
            && self.context_scopes.iter().all(|scope| {
                node_ids.contains(scope.target_node_id.as_str())
                    && scope.confidence_millis <= 1_000
                    && !scope.evidence_surface.trim().is_empty()
            })
    }

    pub fn has_selected_authorized_request(&self) -> bool {
        self.requires_compositional_override()
            && self.selected_node_ids.iter().any(|selected| {
                self.nodes.iter().any(|node| {
                    &node.node_id == selected
                        && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest
                        && !weak_subject(&node.subject)
                })
            })
    }

    pub fn requires_compositional_override(&self) -> bool {
        !self.unresolved_ambiguities.is_empty()
            || (self.has_selected_immediate_request_without_override()
                && self.has_selected_conditional_request_without_override())
            || self.context_scopes.iter().any(|scope| {
                matches!(
                    scope.kind,
                    PragmaticIntentRelationKindIR::Corrects
                        | PragmaticIntentRelationKindIR::Prohibits
                        | PragmaticIntentRelationKindIR::Sequences
                ) || (scope.kind == PragmaticIntentRelationKindIR::Conditions
                    && self.nodes.iter().any(|node| {
                        starts_with_any(
                            &node.source_text,
                            &["when needed", "if needed", "필요하다면", "필요하면"],
                        )
                    }))
                    || (scope.kind == PragmaticIntentRelationKindIR::Supports
                        && self.nodes.iter().any(|node| {
                            node.force == PragmaticClauseForceIR::DirectRequest
                                && node
                                    .evidence
                                    .iter()
                                    .any(|evidence| evidence == "FRAME_MODALITY=ASSERTED")
                        }))
            })
            || self.relations.iter().any(|relation| {
                matches!(
                    relation.kind,
                    PragmaticIntentRelationKindIR::Contrasts
                        | PragmaticIntentRelationKindIR::Overrides
                        | PragmaticIntentRelationKindIR::Corrects
                        | PragmaticIntentRelationKindIR::Prohibits
                        | PragmaticIntentRelationKindIR::Alternative
                )
            })
            || self.nodes.iter().any(|node| {
                node.force == PragmaticClauseForceIR::CapabilityQuestion
                    || (node.force == PragmaticClauseForceIR::Prohibition
                        && !self.selected_node_ids.is_empty())
                    || (node.canonical_predicate == "CONTINUE"
                        && self.selected_node_ids.contains(&node.node_id)
                        && !weak_subject(&node.subject))
            })
    }

    pub fn has_selected_immediate_request(&self) -> bool {
        let selected_immediate = self.selected_node_ids.iter().any(|selected| {
            self.nodes.iter().any(|node| {
                &node.node_id == selected
                    && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest
                    && !weak_subject(&node.subject)
                    && !self.context_scopes.iter().any(|scope| {
                        scope.kind == PragmaticIntentRelationKindIR::Conditions
                            && scope.target_node_id == node.node_id
                    })
            })
        });
        selected_immediate
            && (self.requires_compositional_override()
                || self.has_selected_conditional_request_without_override())
    }

    pub fn has_selected_unconditioned_request(&self) -> bool {
        self.selected_node_ids.iter().any(|selected| {
            self.nodes.iter().any(|node| {
                &node.node_id == selected
                    && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest
                    && !self.context_scopes.iter().any(|scope| {
                        scope.kind == PragmaticIntentRelationKindIR::Conditions
                            && scope.target_node_id == node.node_id
                    })
            })
        })
    }

    pub fn has_selected_conditional_request(&self) -> bool {
        let selected_conditional = self.has_selected_conditional_request_without_override();
        selected_conditional
            && (self.requires_compositional_override()
                || self.has_selected_immediate_request_without_override())
    }

    fn has_selected_immediate_request_without_override(&self) -> bool {
        self.selected_node_ids.iter().any(|selected| {
            self.nodes.iter().any(|node| {
                &node.node_id == selected
                    && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest
                    && !weak_subject(&node.subject)
                    && !self.context_scopes.iter().any(|scope| {
                        scope.kind == PragmaticIntentRelationKindIR::Conditions
                            && scope.target_node_id == node.node_id
                    })
            })
        })
    }

    fn has_selected_conditional_request_without_override(&self) -> bool {
        self.context_scopes.iter().any(|scope| {
            scope.kind == PragmaticIntentRelationKindIR::Conditions
                && self.nodes.iter().any(|node| {
                    scope.target_node_id == node.node_id
                        && !weak_subject(&node.subject)
                        && ((self.selected_node_ids.contains(&node.node_id)
                            && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest)
                            || scope.evidence_surface == "typed conditional consequent")
                })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticIntentGraphIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<PragmaticIntentInferenceIR>,
    #[serde(default)]
    pub unresolved_ambiguities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<CompositionalPragmaticGraphIR>,
    #[serde(default)]
    pub utterance_intent: UtteranceIntentGraphIR,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
}

impl Default for PragmaticIntentGraphIR {
    fn default() -> Self {
        Self {
            schema: PRAGMATIC_INTENT_GRAPH_SCHEMA.to_string(),
            primary: None,
            unresolved_ambiguities: Vec::new(),
            composition: None,
            utterance_intent: UtteranceIntentGraphIR::default(),
            semantic_authority: false,
            external_action_execution_authorized: false,
        }
    }
}

impl PragmaticIntentGraphIR {
    pub fn primary_kind(&self) -> Option<PragmaticIntentKindIR> {
        self.primary.as_ref().map(|primary| primary.kind)
    }

    pub fn suppresses_goal_projection(&self) -> bool {
        if self.composition.as_ref().is_some_and(|composition| {
            composition.requires_compositional_override()
                && (!composition.unresolved_ambiguities.is_empty()
                    || (!composition.nodes.is_empty() && composition.selected_node_ids.is_empty()))
        }) {
            return true;
        }
        self.primary
            .as_ref()
            .is_some_and(|primary| primary.projection == PragmaticGoalProjectionIR::Suppressed)
    }

    pub fn selected_utterance_intent(
        &self,
    ) -> Option<&crate::utterance_intent::UtteranceIntentCandidateIR> {
        self.utterance_intent.selected()
    }

    /// Projects a native multi-goal decision into the legacy graph shape for
    /// downstream consumers that have not migrated yet. The native circuit is
    /// the source of the selection; this graph carries no semantic or execution
    /// authority of its own.
    pub(crate) fn project_native_goal_compatibility(
        &mut self,
        native: &NativeTurnIR,
        source: &str,
    ) {
        let legacy_unresolved = !self.unresolved_ambiguities.is_empty()
            || self.composition.as_ref().is_some_and(|composition| {
                !composition.unresolved_ambiguities.is_empty()
                    || (!composition.nodes.is_empty() && composition.selected_node_ids.is_empty())
            });
        if !native.unresolved.is_empty()
            || native.selected_live_goals.is_empty()
            || (native.selected_live_goals.len() == 1 && !legacy_unresolved)
        {
            return;
        }
        let nodes = native
            .selected_live_goals
            .iter()
            .enumerate()
            .map(|(index, goal)| PragmaticClauseIntentIR {
                node_id: format!("NATIVE-PRAGMATIC-NODE-{:02}", index + 1),
                clause_id: goal.source_event_id.clone(),
                source_frame_id: goal.source_event_id.clone(),
                source_text: source.trim().to_string(),
                force: PragmaticClauseForceIR::DirectRequest,
                projection: PragmaticGoalProjectionIR::AuthorizedRequest,
                canonical_predicate: goal.canonical_predicate.clone(),
                intent: goal.intent,
                subject: goal.subject.clone(),
                clause_function: ClauseFunctionIR::Main,
                confidence_millis: goal.confidence_millis,
                evidence: vec![
                    format!("NATIVE_GOAL={}", goal.goal_id),
                    "COMPATIBILITY_VIEW_OF_NATIVE_SELECTION".to_string(),
                    "SEMANTIC_AUTHORITY=false".to_string(),
                    "EXTERNAL_ACTION_EXECUTION_AUTHORIZED=false".to_string(),
                ],
                semantic_authority: false,
                external_action_execution_authorized: false,
            })
            .collect::<Vec<_>>();
        let selected_node_ids = nodes
            .iter()
            .map(|node| node.node_id.clone())
            .collect::<Vec<_>>();
        let relations = nodes
            .windows(2)
            .enumerate()
            .map(|(index, pair)| PragmaticIntentRelationIR {
                relation_id: format!("NATIVE-PRAGMATIC-RELATION-{:02}", index + 1),
                kind: PragmaticIntentRelationKindIR::Sequences,
                source_node_id: pair[0].node_id.clone(),
                target_node_id: pair[1].node_id.clone(),
                evidence_surface: "native discourse order".to_string(),
                confidence_millis: 930,
            })
            .collect();
        let compatibility = CompositionalPragmaticGraphIR {
            schema: COMPOSITIONAL_PRAGMATIC_GRAPH_SCHEMA.to_string(),
            nodes,
            relations,
            context_scopes: Vec::new(),
            selected_node_ids,
            suppressed_node_ids: Vec::new(),
            unresolved_ambiguities: Vec::new(),
            semantic_authority: false,
            external_action_execution_authorized: false,
            graph_sha256: String::new(),
        }
        .seal();
        debug_assert!(compatibility.validate());
        self.composition = Some(compatibility);
        self.unresolved_ambiguities.clear();
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PragmaticIntentAnalyzer;

impl PragmaticIntentAnalyzer {
    pub fn analyze(
        &self,
        text: &str,
        active_subject: Option<&str>,
        active_predicates: &[String],
        analysis: &CompositionalAnalysisIR,
    ) -> PragmaticIntentGraphIR {
        let normalized = text.trim().to_lowercase();
        if normalized.is_empty() {
            return PragmaticIntentGraphIR::default();
        }
        let utterance_intent =
            UtteranceIntentAnalyzer.analyze(text, active_subject, active_predicates);
        let mentions = pragmatic_action_mentions(&normalized);
        let kind = classify_intent(
            &normalized,
            !active_predicates.is_empty(),
            !mentions.is_empty(),
        );
        let Some(kind) = kind else {
            let composition = build_compositional_pragmatic_graph(
                &normalized,
                active_subject,
                active_predicates,
                analysis,
                None,
            );
            return PragmaticIntentGraphIR {
                composition: Some(composition),
                utterance_intent,
                ..PragmaticIntentGraphIR::default()
            };
        };
        let projection = match kind {
            PragmaticIntentKindIR::ConventionalIndirectRequest
            | PragmaticIntentKindIR::PreferenceRequest
            | PragmaticIntentKindIR::GoalCorrection => PragmaticGoalProjectionIR::AuthorizedRequest,
            PragmaticIntentKindIR::AdvisorySuggestion => PragmaticGoalProjectionIR::AdvisoryOnly,
            PragmaticIntentKindIR::RhetoricalEvaluation
            | PragmaticIntentKindIR::InformationQuestion
            | PragmaticIntentKindIR::SelfOffer
            | PragmaticIntentKindIR::MetalinguisticMention => PragmaticGoalProjectionIR::Suppressed,
        };
        let target = select_target(&normalized, kind, &mentions, active_predicates);
        let explicit_subject = target
            .as_ref()
            .and_then(|mention| extract_subject(&normalized, mention, None));
        let inherited = (kind == PragmaticIntentKindIR::GoalCorrection
            && explicit_subject.as_deref().is_none_or(|subject| {
                weak_subject(subject) || causal_explanation_subject(subject)
            }))
        .then_some(active_subject)
        .flatten();
        let subject = inherited
            .map(clean_subject)
            .or_else(|| explicit_subject.filter(|subject| !weak_subject(subject)));
        let source_frame_id = target.as_ref().and_then(|target| {
            analysis
                .frames
                .iter()
                .filter(|frame| !frame.embedded_under_quote)
                .min_by_key(|frame| frame.source_start_byte.abs_diff(target.start_byte))
                .filter(|frame| frame.canonical_predicate == target.canonical_predicate)
                .map(|frame| frame.frame_id.clone())
        });
        let confidence_millis = confidence(kind, target.is_some(), subject.is_some());
        let mut unresolved_ambiguities = Vec::new();
        if matches!(
            projection,
            PragmaticGoalProjectionIR::AuthorizedRequest | PragmaticGoalProjectionIR::AdvisoryOnly
        ) && target.is_none()
        {
            unresolved_ambiguities.push("PRAGMATIC_ACTION_TARGET".to_string());
        }
        if kind == PragmaticIntentKindIR::GoalCorrection && active_predicates.len() != 1 {
            unresolved_ambiguities.push(if active_predicates.is_empty() {
                "PRIOR_ACTIVE_GOAL".to_string()
            } else {
                "MULTIPLE_ACTIVE_GOALS_FOR_CORRECTION".to_string()
            });
        }
        let mut graph = PragmaticIntentGraphIR {
            schema: PRAGMATIC_INTENT_GRAPH_SCHEMA.to_string(),
            primary: Some(PragmaticIntentInferenceIR {
                kind,
                projection,
                canonical_predicate: target
                    .as_ref()
                    .map(|target| target.canonical_predicate.clone()),
                intent: target.as_ref().map(|target| target.intent),
                subject,
                source_frame_id,
                confidence_millis,
                evidence: intent_evidence(kind, &normalized),
                semantic_authority: false,
                external_action_execution_authorized: false,
            }),
            unresolved_ambiguities,
            composition: None,
            utterance_intent,
            semantic_authority: false,
            external_action_execution_authorized: false,
        };
        let composition = build_compositional_pragmatic_graph(
            &normalized,
            active_subject,
            active_predicates,
            analysis,
            graph.primary.as_ref(),
        );
        if let Some(selected) = composition.selected_node_ids.last().and_then(|selected| {
            composition
                .nodes
                .iter()
                .find(|node| &node.node_id == selected)
        }) {
            graph.primary = inference_from_selected_node(selected);
        } else if !composition.unresolved_ambiguities.is_empty() {
            graph.primary = None;
            graph
                .unresolved_ambiguities
                .extend(composition.unresolved_ambiguities.iter().cloned());
        }
        graph.composition = Some(composition);
        graph
    }

    /// Compatibility reducer used only inside the Language Center's one-shot
    /// materializer. It is intentionally not part of the public adapter API.
    pub(crate) fn apply_utterance_intent_to_compositional_analysis(
        &self,
        graph: &PragmaticIntentGraphIR,
        text: &str,
        analysis: &mut CompositionalAnalysisIR,
    ) {
        let Some(selected) = graph.selected_utterance_intent() else {
            return;
        };
        let explicit_action_selected = analysis
            .selected_candidates()
            .iter()
            .any(|candidate| candidate.external_execution_authorized)
            || graph
                .composition
                .as_ref()
                .is_some_and(CompositionalPragmaticGraphIR::has_selected_unconditioned_request);
        if explicit_action_selected
            && !matches!(
                selected.communicative_intent,
                CommunicativeIntentIR::ResponseGoalCorrection
                    | CommunicativeIntentIR::ConditionalDecisionRequest
            )
        {
            return;
        }
        if selected.expected_response == ExpectedResponseKindIR::Clarification {
            suppress_all_action_candidates_for_response_goal(analysis, "UTTERANCE_CONTEXT_MISSING");
            analysis.clarification_required = true;
            analysis
                .unresolved_competitions
                .push("PRIOR_DISCOURSE_CONTEXT".to_string());
            analysis.unresolved_competitions.sort();
            analysis.unresolved_competitions.dedup();
            return;
        }
        if selected.communicative_intent == CommunicativeIntentIR::ResponseGoalCorrection {
            suppress_all_action_candidates_for_response_goal(
                analysis,
                "SUPERSEDED_BY_RESPONSE_GOAL_CORRECTION",
            );
            let frame_id = ensure_frame(
                analysis,
                text,
                None,
                "EXPLAIN",
                PlanIntentIR::Explain,
                &selected.target,
                false,
            );
            let candidate_id = ensure_candidate(
                analysis,
                CandidateProjection {
                    frame_id: &frame_id,
                    canonical: "EXPLAIN",
                    intent: PlanIntentIR::Explain,
                    subject: &selected.target,
                    authorized: false,
                    kind: PragmaticIntentKindIR::GoalCorrection,
                    confidence_millis: selected.score_millis,
                },
            );
            analysis.selected_candidate_id = Some(candidate_id.clone());
            analysis.selected_candidate_ids = vec![candidate_id];
            analysis.goal_graph = None;
            analysis.clarification_required = false;
            analysis.unresolved_competitions.clear();
            analysis.structural_coverage_millis = analysis.structural_coverage_millis.max(950);
            return;
        }
        suppress_all_action_candidates_for_response_goal(
            analysis,
            "COMMUNICATIVE_RESPONSE_GOAL_IS_NOT_EXTERNAL_ACTION_AUTHORITY",
        );
    }

    /// Compatibility reducer used only inside the Language Center's one-shot
    /// materializer. Source graphs remain immutable and externally visible.
    pub(crate) fn apply_to_compositional_analysis(
        &self,
        graph: &PragmaticIntentGraphIR,
        text: &str,
        analysis: &mut CompositionalAnalysisIR,
    ) {
        if let Some(composition) = graph.composition.as_ref().filter(|composition| {
            !composition.nodes.is_empty()
                && (composition.requires_compositional_override()
                    || (graph.primary.is_none()
                        && !analysis.clarification_required
                        && analysis.unresolved_competitions.is_empty()
                        && !analysis
                            .selected_candidates()
                            .iter()
                            .any(|candidate| candidate.external_execution_authorized)
                        && composition.has_selected_unconditioned_request()))
        }) {
            apply_compositional_pragmatic_graph(composition, text, analysis);
            return;
        }
        let Some(primary) = graph.primary.as_ref() else {
            return;
        };
        if primary.projection == PragmaticGoalProjectionIR::Suppressed {
            suppress_candidates(analysis, primary.kind);
            return;
        }
        if !graph.unresolved_ambiguities.is_empty() {
            analysis.clarification_required = true;
            analysis
                .unresolved_competitions
                .extend(graph.unresolved_ambiguities.iter().cloned());
            return;
        }
        let (Some(canonical), Some(intent), Some(subject)) = (
            primary.canonical_predicate.as_deref(),
            primary.intent,
            primary.subject.as_deref(),
        ) else {
            analysis.clarification_required = true;
            analysis
                .unresolved_competitions
                .push("PRAGMATIC_GOAL_BINDING".to_string());
            return;
        };
        let authorized = primary.projection == PragmaticGoalProjectionIR::AuthorizedRequest;
        let frame_id = ensure_frame(
            analysis,
            text,
            primary.source_frame_id.as_deref(),
            canonical,
            intent,
            subject,
            authorized,
        );
        let candidate_id = ensure_candidate(
            analysis,
            CandidateProjection {
                frame_id: &frame_id,
                canonical,
                intent,
                subject,
                authorized,
                kind: primary.kind,
                confidence_millis: primary.confidence_millis,
            },
        );
        for candidate in &mut analysis.candidates {
            if candidate.candidate_id != candidate_id {
                candidate.external_execution_authorized = false;
                if primary.kind == PragmaticIntentKindIR::GoalCorrection {
                    candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
                    candidate
                        .blockers
                        .push("SUPERSEDED_BY_PRAGMATIC_GOAL_CORRECTION".to_string());
                }
            }
        }
        analysis.selected_candidate_id = Some(candidate_id.clone());
        analysis.selected_candidate_ids = vec![candidate_id.clone()];
        analysis.goal_graph = Some(CompositionalGoalGraphIR {
            nodes: vec![CompositionalGoalNodeIR {
                node_id: "PRAGMATIC-GOAL-01".to_string(),
                candidate_id,
                intent,
                subject: subject.to_string(),
                desired_outcome: desired_outcome(canonical, intent, subject),
                external_execution_authorized: authorized,
            }],
            edges: Vec::new(),
            conditions: Vec::new(),
            prohibitions: Vec::new(),
            confidence_millis: primary.confidence_millis,
        });
        analysis.clarification_required = false;
        analysis.unresolved_competitions.clear();
        analysis.structural_coverage_millis = analysis.structural_coverage_millis.max(900);
    }
}

fn suppress_all_action_candidates_for_response_goal(
    analysis: &mut CompositionalAnalysisIR,
    blocker: &str,
) {
    for frame in &mut analysis.frames {
        frame.external_execution_authorized = false;
    }
    for candidate in &mut analysis.candidates {
        candidate.external_execution_authorized = false;
        candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
        candidate.blockers.push(blocker.to_string());
        candidate.blockers.sort();
        candidate.blockers.dedup();
    }
    analysis.selected_candidate_id = None;
    analysis.selected_candidate_ids.clear();
    analysis.goal_graph = None;
}

fn build_compositional_pragmatic_graph(
    text: &str,
    active_subject: Option<&str>,
    active_predicates: &[String],
    analysis: &CompositionalAnalysisIR,
    legacy_primary: Option<&PragmaticIntentInferenceIR>,
) -> CompositionalPragmaticGraphIR {
    let capability_question = is_capability_question(text);
    let correction = !active_predicates.is_empty() && is_goal_correction(text);
    let mut nodes = analysis
        .frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let clause = analysis.clause_graph.node_for_frame(&frame.frame_id);
            let source_text = clause
                .map(|clause| clause.source_text.as_str())
                .unwrap_or(text)
                .trim()
                .to_lowercase();
            let clause_function = clause
                .map(|clause| clause.function)
                .unwrap_or(ClauseFunctionIR::Main);
            let force = clause_force(
                text,
                &source_text,
                frame,
                capability_question,
                correction,
                active_predicates,
            );
            let projection = projection_for_force(force);
            let inherited_subject = legacy_primary
                .filter(|primary| primary.source_frame_id.as_deref() == Some(&frame.frame_id))
                .and_then(|primary| primary.subject.as_deref())
                .filter(|subject| {
                    frame.canonical_predicate != "CONTINUE" || !continuation_task_subject(subject)
                })
                .or_else(|| {
                    (correction && weak_subject(&frame.theme))
                        .then_some(active_subject)
                        .flatten()
                })
                .or_else(|| {
                    (frame.canonical_predicate == "CONTINUE"
                        && continuation_task_subject(&frame.theme))
                    .then_some(active_subject)
                    .flatten()
                });
            let subject = inherited_subject
                .map(clean_subject)
                .filter(|subject| !subject.is_empty())
                .unwrap_or_else(|| {
                    let cleaned = clean_subject(&frame.theme);
                    if cleaned.is_empty() {
                        "unresolved_subject".to_string()
                    } else {
                        cleaned
                    }
                });
            PragmaticClauseIntentIR {
                node_id: format!("PRAGMATIC-CLAUSE-NODE-{:02}", index + 1),
                clause_id: clause
                    .map(|clause| clause.clause_id.clone())
                    .unwrap_or_else(|| frame.clause_id.clone()),
                source_frame_id: frame.frame_id.clone(),
                source_text,
                force,
                projection,
                canonical_predicate: frame.canonical_predicate.clone(),
                intent: frame.intent_hint,
                subject,
                clause_function,
                confidence_millis: force_confidence(force, frame),
                evidence: vec![
                    format!("CLAUSE_FORCE={force:?}").to_uppercase(),
                    format!("FRAME_MOOD={:?}", frame.mood).to_uppercase(),
                    format!("FRAME_MODALITY={:?}", frame.modality).to_uppercase(),
                    format!("FRAME_POLARITY={:?}", frame.polarity).to_uppercase(),
                    "SEMANTIC_AUTHORITY=false".to_string(),
                    "EXTERNAL_ACTION_EXECUTION_AUTHORIZED=false".to_string(),
                ],
                semantic_authority: false,
                external_action_execution_authorized: false,
            }
        })
        .collect::<Vec<_>>();

    if correction {
        let excludes_prior_action = nodes.len() > 1 && contains_override(text);
        for node in &mut nodes {
            let is_prior_action = active_predicates
                .iter()
                .any(|active| active.eq_ignore_ascii_case(&node.canonical_predicate));
            if excludes_prior_action && is_prior_action {
                node.force = PragmaticClauseForceIR::Prohibition;
                node.projection = PragmaticGoalProjectionIR::Suppressed;
                node.confidence_millis = 970;
                node.evidence
                    .push("PRIOR_ACTION_EXCLUDED_BY_CORRECTION".to_string());
                continue;
            }
            if node.force != PragmaticClauseForceIR::Prohibition
                && !is_prior_action
                && matches!(
                    node.projection,
                    PragmaticGoalProjectionIR::AuthorizedRequest
                        | PragmaticGoalProjectionIR::AdvisoryOnly
                )
            {
                node.force = PragmaticClauseForceIR::GoalCorrection;
                node.projection = PragmaticGoalProjectionIR::AuthorizedRequest;
                if weak_subject(&node.subject) {
                    if let Some(active_subject) = active_subject {
                        node.subject = clean_subject(active_subject);
                    }
                }
            }
        }
    }

    let mut relations = analysis
        .clause_graph
        .edges
        .iter()
        .filter_map(|edge| {
            let source = nodes
                .iter()
                .find(|node| node.clause_id == edge.source_clause_id)?;
            let target = nodes
                .iter()
                .find(|node| node.clause_id == edge.target_clause_id)?;
            Some(PragmaticIntentRelationIR {
                relation_id: String::new(),
                kind: map_clause_relation(edge.relation),
                source_node_id: source.node_id.clone(),
                target_node_id: target.node_id.clone(),
                evidence_surface: edge.marker_surface.clone(),
                confidence_millis: edge.confidence_millis,
            })
        })
        .collect::<Vec<_>>();

    let has_alternative = contains_action_alternative(text, analysis);
    if has_alternative {
        for node in &mut nodes {
            if node.force == PragmaticClauseForceIR::DescriptiveMention {
                node.force = PragmaticClauseForceIR::DirectRequest;
                node.projection = PragmaticGoalProjectionIR::AuthorizedRequest;
                node.confidence_millis = 870;
                node.evidence
                    .push("DIRECTIVE_SCOPE_INHERITED_ACROSS_ALTERNATIVE".to_string());
            }
        }
    }
    let actionable = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.projection,
                PragmaticGoalProjectionIR::AuthorizedRequest
                    | PragmaticGoalProjectionIR::AdvisoryOnly
            )
        })
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let unresolved_ambiguities = if has_alternative && nodes.len() > 1 {
        vec!["PRAGMATIC_ACTION_ALTERNATIVE".to_string()]
    } else {
        Vec::new()
    };
    let selected_node_ids = if unresolved_ambiguities.is_empty() {
        actionable
    } else {
        Vec::new()
    };
    let suppressed_node_ids = nodes
        .iter()
        .filter(|node| !selected_node_ids.contains(&node.node_id))
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();

    if has_alternative && nodes.len() >= 2 {
        push_relation(
            &mut relations,
            PragmaticIntentRelationKindIR::Alternative,
            &nodes[0],
            &nodes[1],
            alternative_marker(text),
            950,
        );
    }
    if let (Some(prohibited), Some(selected)) = (
        nodes
            .iter()
            .find(|node| node.force == PragmaticClauseForceIR::Prohibition),
        nodes
            .iter()
            .find(|node| selected_node_ids.contains(&node.node_id)),
    ) {
        push_relation(
            &mut relations,
            PragmaticIntentRelationKindIR::Prohibits,
            prohibited,
            selected,
            prohibition_marker(text),
            970,
        );
    }
    if correction {
        if let (Some(previous), Some(selected)) = (
            nodes.iter().find(|node| {
                node.force == PragmaticClauseForceIR::Prohibition
                    || active_predicates
                        .iter()
                        .any(|active| active.eq_ignore_ascii_case(&node.canonical_predicate))
            }),
            nodes
                .iter()
                .find(|node| selected_node_ids.contains(&node.node_id)),
        ) {
            push_relation(
                &mut relations,
                PragmaticIntentRelationKindIR::Corrects,
                previous,
                selected,
                correction_marker(text),
                980,
            );
        }
    }
    if contains_override(text) && nodes.len() >= 2 {
        let selected = nodes
            .iter()
            .find(|node| selected_node_ids.contains(&node.node_id))
            .unwrap_or(&nodes[nodes.len() - 1]);
        if let Some(previous) = nodes.iter().find(|node| node.node_id != selected.node_id) {
            push_relation(
                &mut relations,
                PragmaticIntentRelationKindIR::Overrides,
                selected,
                previous,
                override_marker(text),
                960,
            );
        }
    }
    if nodes.len() >= 2
        && relations.is_empty()
        && text
            .chars()
            .any(|character| matches!(character, '.' | '?' | '!' | ';'))
    {
        push_relation(
            &mut relations,
            PragmaticIntentRelationKindIR::Sequences,
            &nodes[0],
            &nodes[1],
            "sentence boundary",
            820,
        );
    }
    for (index, relation) in relations.iter_mut().enumerate() {
        relation.relation_id = format!("PRAGMATIC-RELATION-{:02}", index + 1);
    }

    let mut context_scopes = Vec::new();
    let contextual = contextual_scope(text).or_else(|| {
        analysis
            .modal_scope_graph
            .conditionals
            .iter()
            .any(|conditional| conditional.consequent_is_directive)
            .then_some((
                PragmaticIntentRelationKindIR::Conditions,
                "typed conditional consequent",
            ))
    });
    let contextual_target = contextual
        .filter(|(kind, _)| *kind == PragmaticIntentRelationKindIR::Conditions)
        .and_then(|_| conditional_consequent_node(analysis, &nodes))
        .or_else(|| {
            nodes
                .iter()
                .find(|node| selected_node_ids.contains(&node.node_id))
        })
        .or_else(|| nodes.last());
    if let Some(target) = contextual_target {
        if let Some((kind, evidence)) = contextual {
            context_scopes.push(PragmaticContextScopeIR {
                scope_id: "PRAGMATIC-CONTEXT-SCOPE-01".to_string(),
                kind,
                target_node_id: target.node_id.clone(),
                evidence_surface: evidence.to_string(),
                confidence_millis: 900,
            });
        }
        if correction
            && !relations
                .iter()
                .any(|relation| relation.kind == PragmaticIntentRelationKindIR::Corrects)
        {
            context_scopes.push(PragmaticContextScopeIR {
                scope_id: format!("PRAGMATIC-CONTEXT-SCOPE-{:02}", context_scopes.len() + 1),
                kind: PragmaticIntentRelationKindIR::Corrects,
                target_node_id: target.node_id.clone(),
                evidence_surface: "prior active goal".to_string(),
                confidence_millis: 940,
            });
        }
        if explicit_prohibition(text)
            && !nodes
                .iter()
                .any(|node| node.force == PragmaticClauseForceIR::Prohibition)
        {
            context_scopes.push(PragmaticContextScopeIR {
                scope_id: format!("PRAGMATIC-CONTEXT-SCOPE-{:02}", context_scopes.len() + 1),
                kind: PragmaticIntentRelationKindIR::Prohibits,
                target_node_id: target.node_id.clone(),
                evidence_surface: prohibition_marker(text).to_string(),
                confidence_millis: 900,
            });
        }
    }

    CompositionalPragmaticGraphIR {
        schema: COMPOSITIONAL_PRAGMATIC_GRAPH_SCHEMA.to_string(),
        nodes,
        relations,
        context_scopes,
        selected_node_ids,
        suppressed_node_ids,
        unresolved_ambiguities,
        semantic_authority: false,
        external_action_execution_authorized: false,
        graph_sha256: String::new(),
    }
    .seal()
}

fn conditional_consequent_node<'a>(
    analysis: &CompositionalAnalysisIR,
    nodes: &'a [PragmaticClauseIntentIR],
) -> Option<&'a PragmaticClauseIntentIR> {
    analysis
        .modal_scope_graph
        .conditionals
        .iter()
        .filter(|conditional| conditional.consequent_is_directive)
        .find_map(|conditional| {
            let consequent = conditional.consequent.to_lowercase();
            nodes.iter().find(|node| {
                analysis
                    .frames
                    .iter()
                    .find(|frame| frame.frame_id == node.source_frame_id)
                    .is_some_and(|frame| {
                        consequent.contains(&frame.predicate_surface.to_lowercase())
                            || consequent.contains(&frame.canonical_predicate.to_lowercase())
                    })
            })
        })
}

fn clause_force(
    whole_text: &str,
    clause_text: &str,
    frame: &PredicateFrameIR,
    capability_question: bool,
    correction: bool,
    active_predicates: &[String],
) -> PragmaticClauseForceIR {
    if frame.embedded_under_quote || frame_inside_quoted_span(whole_text, frame) {
        return PragmaticClauseForceIR::MetalinguisticMention;
    }
    // An independent directive after a closed quotation belongs to the live
    // speech act even when an earlier clause talks about the quoted sentence
    // itself.  Test this boundary before classifying the containing span as a
    // metalinguistic mention; quoted predicates remain suppressed above.
    if independent_directive_after_reported_quote(whole_text, frame) {
        return PragmaticClauseForceIR::DirectRequest;
    }
    if is_metalinguistic(clause_text) {
        return PragmaticClauseForceIR::MetalinguisticMention;
    }
    if excluded_by_postposed_override(whole_text, frame) {
        return PragmaticClauseForceIR::Prohibition;
    }
    if frame.modality == FrameModalityIR::Prohibited
        || prohibition_applies_to_frame(clause_text, frame)
    {
        return PragmaticClauseForceIR::Prohibition;
    }
    if reported_surface(clause_text)
        || matches!(frame.mood, FrameMoodIR::Reported)
        || frame.modality == FrameModalityIR::Reported
    {
        return PragmaticClauseForceIR::ReportedSuggestion;
    }
    if capability_question {
        return PragmaticClauseForceIR::CapabilityQuestion;
    }
    if correction
        && !active_predicates
            .iter()
            .any(|active| active.eq_ignore_ascii_case(&frame.canonical_predicate))
    {
        return PragmaticClauseForceIR::GoalCorrection;
    }
    if let Some(kind) = classify_intent(clause_text, correction, true) {
        return force_from_legacy_kind(kind);
    }
    if is_rhetorical_evaluation(whole_text) && clause_text.contains('?') {
        return PragmaticClauseForceIR::RhetoricalEvaluation;
    }
    if matches!(frame.mood, FrameMoodIR::Imperative)
        || frame.modality == FrameModalityIR::Requested
        || frame.external_execution_authorized
        || looks_like_directive(clause_text)
    {
        return PragmaticClauseForceIR::DirectRequest;
    }
    if matches!(frame.mood, FrameMoodIR::Interrogative) || clause_text.contains('?') {
        return PragmaticClauseForceIR::InformationQuestion;
    }
    PragmaticClauseForceIR::DescriptiveMention
}

fn independent_directive_after_reported_quote(whole_text: &str, frame: &PredicateFrameIR) -> bool {
    if frame.embedded_under_quote
        || frame.source_start_byte > whole_text.len()
        || !whole_text.is_char_boundary(frame.source_start_byte)
    {
        return false;
    }
    let frame_region = &whole_text[frame.source_start_byte..];
    let predicate_offset = frame_region
        .to_lowercase()
        .find(&frame.predicate_surface.to_lowercase())
        .unwrap_or_default();
    let predicate_start = frame.source_start_byte + predicate_offset;
    if predicate_start > whole_text.len() || !whole_text.is_char_boundary(predicate_start) {
        return false;
    }
    let prefix = &whole_text[..predicate_start];
    let Some((closing_index, closing_quote)) = prefix
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '\'' | '"' | '’' | '”' | '」' | '』'))
    else {
        return false;
    };
    if marker_is_inside_quote(whole_text, predicate_start) {
        return false;
    }
    let before_quote = &whole_text[..closing_index];
    let reported = contains_any(
        &before_quote.to_lowercase(),
        &[
            "said", "wrote", "reported", "reads", "runbook", "document", "note", "말했", "썼",
            "쓰여", "적혀", "문서", "메모", "sentence", "example", "문장", "예시",
        ],
    );
    if !reported {
        return false;
    }
    let after_quote = &whole_text[closing_index + closing_quote.len_utf8()..predicate_start];
    let quote_closes_sentence = before_quote
        .trim_end()
        .chars()
        .next_back()
        .is_some_and(|character| matches!(character, '.' | '!' | '?'));
    let independent_boundary = quote_closes_sentence
        || after_quote
            .chars()
            .any(|character| matches!(character, '.' | '!' | '?' | ';'))
        || contains_any(
            &after_quote.to_lowercase(),
            &[" but ", " however ", " 이제 ", " 하지만 "],
        );
    let frame_tail = &whole_text[predicate_start..];
    independent_boundary
        && (matches!(frame.mood, FrameMoodIR::Imperative)
            || frame.modality == FrameModalityIR::Requested
            || frame.external_execution_authorized
            || looks_like_directive(frame_tail))
}

fn marker_is_inside_quote(text: &str, marker_start: usize) -> bool {
    let prefix = &text[..marker_start];
    let straight_double = prefix.chars().filter(|character| *character == '"').count() % 2 == 1;
    let straight_single = prefix
        .char_indices()
        .filter(|(index, character)| {
            if *character != '\'' {
                return false;
            }
            let previous = prefix[..*index].chars().next_back();
            let next = text[*index + character.len_utf8()..].chars().next();
            !(previous.is_some_and(char::is_alphanumeric)
                && next.is_some_and(char::is_alphanumeric))
        })
        .count()
        % 2
        == 1;
    let curly_double = prefix.chars().filter(|character| *character == '“').count()
        > prefix.chars().filter(|character| *character == '”').count();
    let curly_single = prefix.chars().filter(|character| *character == '‘').count()
        > prefix.chars().filter(|character| *character == '’').count();
    let corner = prefix
        .chars()
        .filter(|character| *character == '「')
        .count()
        > prefix
            .chars()
            .filter(|character| *character == '」')
            .count();
    straight_double || straight_single || curly_double || curly_single || corner
}

fn projection_for_force(force: PragmaticClauseForceIR) -> PragmaticGoalProjectionIR {
    match force {
        PragmaticClauseForceIR::DirectRequest
        | PragmaticClauseForceIR::ConventionalIndirectRequest
        | PragmaticClauseForceIR::PreferenceRequest
        | PragmaticClauseForceIR::GoalCorrection => PragmaticGoalProjectionIR::AuthorizedRequest,
        PragmaticClauseForceIR::AdvisorySuggestion => PragmaticGoalProjectionIR::AdvisoryOnly,
        _ => PragmaticGoalProjectionIR::Suppressed,
    }
}

fn force_from_legacy_kind(kind: PragmaticIntentKindIR) -> PragmaticClauseForceIR {
    match kind {
        PragmaticIntentKindIR::ConventionalIndirectRequest => {
            PragmaticClauseForceIR::ConventionalIndirectRequest
        }
        PragmaticIntentKindIR::PreferenceRequest => PragmaticClauseForceIR::PreferenceRequest,
        PragmaticIntentKindIR::AdvisorySuggestion => PragmaticClauseForceIR::AdvisorySuggestion,
        PragmaticIntentKindIR::RhetoricalEvaluation => PragmaticClauseForceIR::RhetoricalEvaluation,
        PragmaticIntentKindIR::InformationQuestion => PragmaticClauseForceIR::InformationQuestion,
        PragmaticIntentKindIR::SelfOffer => PragmaticClauseForceIR::SelfOffer,
        PragmaticIntentKindIR::MetalinguisticMention => {
            PragmaticClauseForceIR::MetalinguisticMention
        }
        PragmaticIntentKindIR::GoalCorrection => PragmaticClauseForceIR::GoalCorrection,
    }
}

fn legacy_kind_from_force(force: PragmaticClauseForceIR) -> Option<PragmaticIntentKindIR> {
    match force {
        PragmaticClauseForceIR::ConventionalIndirectRequest => {
            Some(PragmaticIntentKindIR::ConventionalIndirectRequest)
        }
        PragmaticClauseForceIR::PreferenceRequest => Some(PragmaticIntentKindIR::PreferenceRequest),
        PragmaticClauseForceIR::AdvisorySuggestion => {
            Some(PragmaticIntentKindIR::AdvisorySuggestion)
        }
        PragmaticClauseForceIR::RhetoricalEvaluation => {
            Some(PragmaticIntentKindIR::RhetoricalEvaluation)
        }
        PragmaticClauseForceIR::InformationQuestion
        | PragmaticClauseForceIR::CapabilityQuestion => {
            Some(PragmaticIntentKindIR::InformationQuestion)
        }
        PragmaticClauseForceIR::SelfOffer => Some(PragmaticIntentKindIR::SelfOffer),
        PragmaticClauseForceIR::MetalinguisticMention => {
            Some(PragmaticIntentKindIR::MetalinguisticMention)
        }
        PragmaticClauseForceIR::GoalCorrection => Some(PragmaticIntentKindIR::GoalCorrection),
        PragmaticClauseForceIR::DirectRequest
        | PragmaticClauseForceIR::Prohibition
        | PragmaticClauseForceIR::ReportedSuggestion
        | PragmaticClauseForceIR::DescriptiveMention => None,
    }
}

fn inference_from_selected_node(
    selected: &PragmaticClauseIntentIR,
) -> Option<PragmaticIntentInferenceIR> {
    let kind = legacy_kind_from_force(selected.force)?;
    Some(PragmaticIntentInferenceIR {
        kind,
        projection: selected.projection,
        canonical_predicate: Some(selected.canonical_predicate.clone()),
        intent: Some(selected.intent),
        subject: Some(selected.subject.clone()),
        source_frame_id: Some(selected.source_frame_id.clone()),
        confidence_millis: selected.confidence_millis,
        evidence: selected.evidence.clone(),
        semantic_authority: false,
        external_action_execution_authorized: false,
    })
}

fn force_confidence(force: PragmaticClauseForceIR, frame: &PredicateFrameIR) -> u16 {
    match force {
        PragmaticClauseForceIR::Prohibition => 990,
        PragmaticClauseForceIR::MetalinguisticMention => 980,
        PragmaticClauseForceIR::GoalCorrection => 970,
        PragmaticClauseForceIR::ReportedSuggestion => 950,
        PragmaticClauseForceIR::CapabilityQuestion => 940,
        PragmaticClauseForceIR::DirectRequest => 930,
        PragmaticClauseForceIR::ConventionalIndirectRequest => 920,
        PragmaticClauseForceIR::PreferenceRequest => 910,
        PragmaticClauseForceIR::AdvisorySuggestion => 890,
        PragmaticClauseForceIR::RhetoricalEvaluation => 900,
        PragmaticClauseForceIR::InformationQuestion => 880,
        PragmaticClauseForceIR::SelfOffer => 900,
        PragmaticClauseForceIR::DescriptiveMention => {
            if frame.external_execution_authorized {
                850
            } else {
                780
            }
        }
    }
}

fn map_clause_relation(relation: ClauseRelationKindIR) -> PragmaticIntentRelationKindIR {
    match relation {
        ClauseRelationKindIR::Cause | ClauseRelationKindIR::Purpose => {
            PragmaticIntentRelationKindIR::Supports
        }
        ClauseRelationKindIR::Condition => PragmaticIntentRelationKindIR::Conditions,
        ClauseRelationKindIR::Contrast => PragmaticIntentRelationKindIR::Contrasts,
        ClauseRelationKindIR::Sequence | ClauseRelationKindIR::TemporalBefore => {
            PragmaticIntentRelationKindIR::Sequences
        }
        ClauseRelationKindIR::Coordination => PragmaticIntentRelationKindIR::Coordinates,
    }
}

fn push_relation(
    relations: &mut Vec<PragmaticIntentRelationIR>,
    kind: PragmaticIntentRelationKindIR,
    source: &PragmaticClauseIntentIR,
    target: &PragmaticClauseIntentIR,
    evidence: &str,
    confidence_millis: u16,
) {
    if source.node_id == target.node_id
        || relations.iter().any(|relation| {
            relation.kind == kind
                && relation.source_node_id == source.node_id
                && relation.target_node_id == target.node_id
        })
    {
        return;
    }
    relations.push(PragmaticIntentRelationIR {
        relation_id: String::new(),
        kind,
        source_node_id: source.node_id.clone(),
        target_node_id: target.node_id.clone(),
        evidence_surface: evidence.to_string(),
        confidence_millis,
    });
}

fn contextual_scope(text: &str) -> Option<(PragmaticIntentRelationKindIR, &'static str)> {
    if starts_with_any(text, &["because ", "since "])
        || contains_any(text, &["때문에", "해서", "그러니", "그래서", " so "])
    {
        return Some((PragmaticIntentRelationKindIR::Supports, "causal context"));
    }
    if starts_with_any(text, &["if ", "when ", "provided that "])
        || contains_any(text, &["필요하다면", "필요하면", "깨지면", "실패하면"])
    {
        return Some((
            PragmaticIntentRelationKindIR::Conditions,
            "conditional context",
        ));
    }
    if text
        .find('?')
        .is_some_and(|question| !text[question + 1..].trim().is_empty())
    {
        return Some((
            PragmaticIntentRelationKindIR::Sequences,
            "utterance sequence",
        ));
    }
    None
}

fn is_capability_question(text: &str) -> bool {
    starts_with_any(
        text,
        &[
            "are you able to ",
            "can the system ",
            "could the system ",
            "only tell me whether you can ",
        ],
    ) || contains_any(
        text,
        &[
            "할 수 있는지",
            "할 능력이",
            "가능한지만",
            "가능한지 알려",
            "whether you can",
            "whether the system can",
        ],
    )
}

fn explicit_prohibition(text: &str) -> bool {
    contains_any(
        text,
        &[
            "do not ",
            "don't ",
            "never ",
            "하지 말",
            "하면 안",
            "면 안 돼",
            "면 안돼",
            "지 마",
            "지말",
        ],
    )
}

fn prohibition_applies_to_frame(clause_text: &str, frame: &PredicateFrameIR) -> bool {
    if !explicit_prohibition(clause_text) {
        return false;
    }
    let predicate = frame.predicate_surface.to_lowercase();
    let Some(predicate_start) = clause_text.find(&predicate) else {
        return true;
    };
    let prohibition_start = [
        "do not ",
        "don't ",
        "never ",
        "하지 말",
        "하면 안",
        "면 안 돼",
        "면 안돼",
        "지 마",
        "지말",
    ]
    .iter()
    .filter_map(|marker| clause_text.find(marker))
    .min();
    if prohibition_start.is_some_and(|start| {
        predicate_start < start
            && contains_any(
                &clause_text[predicate_start + predicate.len()..start],
                &[
                    "하되",
                    "한 다음",
                    "한 뒤",
                    "한 후",
                    "고 나서",
                    ",",
                    ";",
                    " and ",
                    " then ",
                ],
            )
    }) {
        return false;
    }
    !["말고", "대신", " instead", " but ", "하지만", ";"]
        .iter()
        .any(|separator| {
            clause_text
                .find(*separator)
                .is_some_and(|separator_start| separator_start < predicate_start)
        })
}

fn excluded_by_postposed_override(text: &str, frame: &PredicateFrameIR) -> bool {
    [" rather than ", " instead of "]
        .iter()
        .filter_map(|marker| text.find(*marker).map(|start| start + marker.len()))
        .any(|excluded_start| frame.source_start_byte >= excluded_start)
}

fn frame_inside_quoted_span(text: &str, frame: &PredicateFrameIR) -> bool {
    position_inside_quoted_span(text, frame.source_start_byte)
}

fn position_inside_quoted_span(text: &str, position: usize) -> bool {
    for (opening, closing) in [('\'', '\''), ('"', '"'), ('“', '”'), ('‘', '’')] {
        let openings = text
            .char_indices()
            .filter_map(|(index, character)| {
                (character == opening && !is_word_apostrophe(text, index, character))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let closings = text
            .char_indices()
            .filter_map(|(index, character)| {
                (character == closing && !is_word_apostrophe(text, index, character))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if opening == closing {
            for pair in openings.chunks_exact(2) {
                if pair[0] < position && position < pair[1] {
                    return true;
                }
            }
        } else if openings
            .iter()
            .any(|start| *start < position && closings.iter().any(|end| position < *end))
        {
            return true;
        }
    }
    false
}

fn is_word_apostrophe(text: &str, index: usize, character: char) -> bool {
    character == '\''
        && text[..index]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
        && text[index + character.len_utf8()..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
}

fn looks_like_directive(text: &str) -> bool {
    let trimmed = text.trim_start_matches(|character: char| {
        character.is_ascii_punctuation() || character.is_whitespace()
    });
    let stripped = [
        "so ",
        "then ",
        "therefore ",
        "actually ",
        "actually, ",
        "그러니 ",
        "그래서 ",
        "그럼 ",
        "실제로는 ",
        "실제로 ",
    ]
    .iter()
    .find_map(|prefix| trimmed.strip_prefix(prefix))
    .unwrap_or(trimmed);
    starts_with_any(
        stripped,
        &[
            "inspect ",
            "verify ",
            "check ",
            "analyze ",
            "assess ",
            "evaluate ",
            "repair ",
            "delete ",
            "continue ",
            "keep ",
            "검사",
            "검증",
            "확인",
            "분석",
            "평가",
            "수리",
            "삭제",
        ],
    )
}

fn reported_surface(text: &str) -> bool {
    contains_any(
        text,
        &[
            " said ",
            "said to ",
            " suggested ",
            "suggested ",
            " proposed ",
            "제안",
            "말했",
            "말하",
            "하자고 말",
        ],
    )
}

fn weak_subject(subject: &str) -> bool {
    matches!(
        subject.trim().to_lowercase().as_str(),
        "" | "it" | "that" | "this" | "그것" | "그거" | "그건" | "그걸" | "이를"
    )
}

fn causal_explanation_subject(subject: &str) -> bool {
    let normalized = subject
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_lowercase();
    normalized == "why"
        || normalized.starts_with("why ")
        || normalized == "the reason"
        || normalized.starts_with("the reason ")
        || normalized == "실패"
        || normalized == "실패 이유"
        || normalized.starts_with("왜 ")
        || normalized.ends_with("실패했는지")
        || normalized.ends_with("실패한 이유")
}

fn continuation_task_subject(subject: &str) -> bool {
    matches!(
        subject.trim().to_lowercase().as_str(),
        "작업" | "일" | "그 작업" | "그 일" | "work" | "that work" | "it" | "that"
    )
}

fn contains_alternative(text: &str) -> bool {
    contains_any(
        text,
        &[
            " or ",
            "either ",
            "또는",
            "아니면",
            "하거나",
            "할지",
            "-거나",
        ],
    ) && !contains_override(text)
}

fn contains_action_alternative(text: &str, analysis: &CompositionalAnalysisIR) -> bool {
    if !contains_alternative(text) {
        return false;
    }
    let alternative_clauses = text
        .split(['.', ';', '!', '?', '\n', '\r'])
        .filter(|clause| contains_alternative(clause))
        .collect::<Vec<_>>();
    if !alternative_clauses.is_empty()
        && alternative_clauses.iter().all(|clause| {
            explicit_prohibition(clause)
                && contains_any(
                    clause,
                    &[
                        "either action",
                        "either operation",
                        "either one",
                        "둘 다",
                        "양쪽 모두",
                    ],
                )
        })
    {
        return false;
    }
    let mut outside_condition_antecedents = text.to_string();
    for conditional in &analysis.modal_scope_graph.conditionals {
        if contains_alternative(&conditional.antecedent) {
            outside_condition_antecedents =
                outside_condition_antecedents.replacen(&conditional.antecedent, "", 1);
        }
    }
    contains_alternative(&outside_condition_antecedents)
}

fn contains_override(text: &str) -> bool {
    contains_any(
        text,
        &[
            " instead",
            "instead,",
            "rather than",
            "대신",
            "말고",
            "무시하고",
            "따르지 말고",
        ],
    )
}

fn alternative_marker(text: &str) -> &str {
    [" or ", "either ", "또는", "아니면", "하거나", "할지"]
        .iter()
        .find(|marker| text.contains(**marker))
        .copied()
        .unwrap_or("alternative")
}

fn prohibition_marker(text: &str) -> &str {
    ["do not", "don't", "never", "하지 말", "하면 안", "지 마"]
        .iter()
        .find(|marker| text.contains(**marker))
        .copied()
        .unwrap_or("prohibition")
}

fn correction_marker(text: &str) -> &str {
    ["actually", "no,", "아니", "취소", "말고"]
        .iter()
        .find(|marker| text.contains(**marker))
        .copied()
        .unwrap_or("correction")
}

fn override_marker(text: &str) -> &str {
    ["instead", "rather than", "대신", "말고", "무시하고"]
        .iter()
        .find(|marker| text.contains(**marker))
        .copied()
        .unwrap_or("override")
}

fn apply_compositional_pragmatic_graph(
    graph: &CompositionalPragmaticGraphIR,
    text: &str,
    analysis: &mut CompositionalAnalysisIR,
) {
    debug_assert!(graph.validate());
    if !graph.unresolved_ambiguities.is_empty() {
        for frame in &mut analysis.frames {
            frame.external_execution_authorized = false;
        }
        for candidate in &mut analysis.candidates {
            candidate.external_execution_authorized = false;
            candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
            candidate
                .blockers
                .push("COMPOSITIONAL_PRAGMATIC_AMBIGUITY".to_string());
        }
        analysis.selected_candidate_id = None;
        analysis.selected_candidate_ids.clear();
        analysis.goal_graph = None;
        analysis.clarification_required = true;
        analysis
            .unresolved_competitions
            .extend(graph.unresolved_ambiguities.iter().cloned());
        return;
    }

    if graph.selected_node_ids.is_empty() {
        for frame in &mut analysis.frames {
            frame.external_execution_authorized = false;
        }
        for candidate in &mut analysis.candidates {
            candidate.external_execution_authorized = false;
            candidate.disposition = if graph.nodes.iter().any(|node| {
                node.source_frame_id == candidate.source_frame_id
                    && node.force == PragmaticClauseForceIR::Prohibition
            }) {
                CandidateDispositionIR::BlockedByNegation
            } else {
                CandidateDispositionIR::NonAuthoritativeMention
            };
        }
        analysis.selected_candidate_id = None;
        analysis.selected_candidate_ids.clear();
        analysis.goal_graph = None;
        if graph.nodes.iter().all(|node| {
            matches!(
                node.force,
                PragmaticClauseForceIR::CapabilityQuestion
                    | PragmaticClauseForceIR::InformationQuestion
                    | PragmaticClauseForceIR::RhetoricalEvaluation
                    | PragmaticClauseForceIR::MetalinguisticMention
                    | PragmaticClauseForceIR::ReportedSuggestion
                    | PragmaticClauseForceIR::DescriptiveMention
            )
        }) {
            analysis.clarification_required = false;
            analysis.unresolved_competitions.clear();
        }
        return;
    }

    let selected_nodes = graph
        .selected_node_ids
        .iter()
        .filter_map(|selected| graph.nodes.iter().find(|node| &node.node_id == selected))
        .collect::<Vec<_>>();
    let selected_frame_ids = selected_nodes
        .iter()
        .map(|node| node.source_frame_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for frame in &mut analysis.frames {
        let selected = selected_frame_ids.contains(frame.frame_id.as_str());
        let authorized = selected_nodes.iter().any(|node| {
            node.source_frame_id == frame.frame_id
                && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest
        });
        frame.external_execution_authorized = selected && authorized;
    }
    for candidate in &mut analysis.candidates {
        if !selected_frame_ids.contains(candidate.source_frame_id.as_str()) {
            candidate.external_execution_authorized = false;
            candidate.disposition = if graph.nodes.iter().any(|node| {
                node.source_frame_id == candidate.source_frame_id
                    && node.force == PragmaticClauseForceIR::Prohibition
            }) {
                CandidateDispositionIR::BlockedByNegation
            } else {
                CandidateDispositionIR::NonAuthoritativeMention
            };
            candidate
                .blockers
                .push("SUPPRESSED_BY_COMPOSITIONAL_PRAGMATIC_GRAPH".to_string());
        }
    }

    let mut candidate_ids = Vec::new();
    for node in &selected_nodes {
        let authorized = node.projection == PragmaticGoalProjectionIR::AuthorizedRequest;
        let frame_id = ensure_frame(
            analysis,
            text,
            Some(&node.source_frame_id),
            &node.canonical_predicate,
            node.intent,
            &node.subject,
            authorized,
        );
        candidate_ids.push(ensure_candidate(
            analysis,
            CandidateProjection {
                frame_id: &frame_id,
                canonical: &node.canonical_predicate,
                intent: node.intent,
                subject: &node.subject,
                authorized,
                kind: legacy_kind_from_force(node.force)
                    .unwrap_or(PragmaticIntentKindIR::ConventionalIndirectRequest),
                confidence_millis: node.confidence_millis,
            },
        ));
    }
    analysis.selected_candidate_id = candidate_ids.last().cloned();
    analysis.selected_candidate_ids = candidate_ids.clone();

    let mut goal_nodes = Vec::new();
    for (index, (node, candidate_id)) in selected_nodes.iter().zip(&candidate_ids).enumerate() {
        goal_nodes.push(CompositionalGoalNodeIR {
            node_id: format!("PRAGMATIC-GOAL-{:02}", index + 1),
            candidate_id: candidate_id.clone(),
            intent: node.intent,
            subject: node.subject.clone(),
            desired_outcome: desired_outcome(&node.canonical_predicate, node.intent, &node.subject),
            external_execution_authorized: node.projection
                == PragmaticGoalProjectionIR::AuthorizedRequest,
        });
    }
    let mut goal_edges = Vec::new();
    for relation in &graph.relations {
        let source = selected_nodes
            .iter()
            .position(|node| node.node_id == relation.source_node_id);
        let target = selected_nodes
            .iter()
            .position(|node| node.node_id == relation.target_node_id);
        if let (Some(source), Some(target)) = (source, target) {
            let relation_kind = match relation.kind {
                PragmaticIntentRelationKindIR::Sequences => Some(GoalGraphRelationKindIR::Sequence),
                PragmaticIntentRelationKindIR::Coordinates => {
                    Some(GoalGraphRelationKindIR::Coordination)
                }
                _ => None,
            };
            if let Some(relation_kind) = relation_kind {
                goal_edges.push(CompositionalGoalEdgeIR {
                    source_node_id: goal_nodes[source].node_id.clone(),
                    target_node_id: goal_nodes[target].node_id.clone(),
                    relation: relation_kind,
                    evidence_surface: relation.evidence_surface.clone(),
                });
            }
        }
    }
    analysis.goal_graph = Some(CompositionalGoalGraphIR {
        nodes: goal_nodes,
        edges: goal_edges,
        conditions: graph
            .context_scopes
            .iter()
            .filter(|scope| scope.kind == PragmaticIntentRelationKindIR::Conditions)
            .map(|scope| scope.evidence_surface.clone())
            .collect(),
        prohibitions: graph
            .nodes
            .iter()
            .filter(|node| node.force == PragmaticClauseForceIR::Prohibition)
            .map(|node| format!("{}:{}", node.canonical_predicate, node.subject))
            .collect(),
        confidence_millis: selected_nodes
            .iter()
            .map(|node| node.confidence_millis)
            .min()
            .unwrap_or(800),
    });
    analysis.clarification_required = false;
    analysis.unresolved_competitions.clear();
    analysis.structural_coverage_millis = analysis.structural_coverage_millis.max(950);
}

fn classify_intent(
    text: &str,
    has_active_goal: bool,
    has_action: bool,
) -> Option<PragmaticIntentKindIR> {
    if is_metalinguistic(text) {
        return Some(PragmaticIntentKindIR::MetalinguisticMention);
    }
    if has_active_goal && has_action && is_goal_correction(text) {
        return Some(PragmaticIntentKindIR::GoalCorrection);
    }
    if is_self_offer(text) {
        return Some(PragmaticIntentKindIR::SelfOffer);
    }
    if is_rhetorical_evaluation(text) {
        return Some(PragmaticIntentKindIR::RhetoricalEvaluation);
    }
    if has_action && is_preference_request(text) {
        return Some(PragmaticIntentKindIR::PreferenceRequest);
    }
    if has_action && is_advisory_suggestion(text) {
        return Some(PragmaticIntentKindIR::AdvisorySuggestion);
    }
    if has_action && is_conventional_indirect_request(text) {
        return Some(PragmaticIntentKindIR::ConventionalIndirectRequest);
    }
    if text.contains('?') && is_information_question(text) {
        return Some(PragmaticIntentKindIR::InformationQuestion);
    }
    None
}

fn is_metalinguistic(text: &str) -> bool {
    let quoted = has_quoted_span(text);
    quoted
        && contains_any(
            text,
            &[
                "문장",
                "무슨 뜻",
                "뜻이",
                "요청이",
                "명령이",
                "발화",
                "what does",
                "mean",
                "a command",
                "an instruction",
                "a request",
                "utterance",
            ],
        )
}

fn is_goal_correction(text: &str) -> bool {
    contains_any(
        text,
        &[
            "아니,",
            "아니 ",
            "가 아니라",
            "이 아니라",
            "말고",
            "no,",
            "actually,",
            "instead",
            "rather than",
            "i meant",
            "correct that:",
            "correct this:",
            "correct myself:",
            "rephrase that:",
            "withdraw that",
            "cancel that",
            "취소하고",
            "취소한 뒤",
        ],
    )
}

fn is_self_offer(text: &str) -> bool {
    let revision_preface = text
        .split_once([':', '：'])
        .is_some_and(|(preface, replacement)| {
            !replacement.trim().is_empty()
                && [
                    "correct that",
                    "correct this",
                    "correct myself",
                    "rephrase that",
                    "정정",
                ]
                .iter()
                .any(|marker| preface.contains(marker))
        });
    let korean_actor = contains_any(text, &["내가 ", "제가 ", "나는 ", "난 "]);
    let korean_offer = contains_any(
        text,
        &[
            "할게",
            "볼게",
            "고칠까",
            "확인할까",
            "검사할까",
            "분석할까",
            "수리할까",
        ],
    );
    let english_offer = starts_with_any(
        text,
        &[
            "shall i ", "i can ", "i could ", "i'll ", "i will ", "i can ",
        ],
    );
    !revision_preface && ((korean_actor && korean_offer) || english_offer)
}

fn is_rhetorical_evaluation(text: &str) -> bool {
    let interrogative = text.contains('?')
        || text.ends_with("겠어")
        || text.ends_with("라고")
        || text.ends_with("라니");
    let indefinite_challenger = starts_with_any(text, &["누가 ", "who would ", "does anyone "]);
    let evaluative_naming = contains_any(
        text,
        &[
            "성공이라고",
            "완료라고",
            "고친 거라고",
            "제대로",
            "call this",
            "call that",
            "a success",
            "a repair",
            "finished",
        ],
    );
    let echo_challenge = starts_with_any(text, &["이게 ", "이걸 ", "you call "]);
    interrogative && evaluative_naming && (indefinite_challenger || echo_challenge)
}

fn is_preference_request(text: &str) -> bool {
    contains_any(
        text,
        &[
            "해줬으면",
            "해 주었으면",
            "해주었으면",
            "줬으면 좋겠",
            "줬으면 해",
            "해 달라",
            "해달라",
            "달라는 거야",
            "would like you to",
            "i'd like you to",
            "i would prefer you to",
            "i'd prefer you to",
            "want you to",
        ],
    )
}

fn is_advisory_suggestion(text: &str) -> bool {
    contains_any(
        text,
        &[
            "는 게 어때",
            "는 건 어때",
            "보는 게 어때",
            "보는 건 어떨까",
            "해보는 게 어때",
            "해보는 건 어떨까",
            "how about ",
            "maybe we should ",
            "why don't we ",
        ],
    )
}

fn is_conventional_indirect_request(text: &str) -> bool {
    let korean = contains_any(
        text,
        &[
            "해줄래",
            "해 줄래",
            "줄래",
            "주면 안 될까",
            "주면 안될까",
            "주시겠",
            "해줘?",
        ],
    );
    let english_addressee = contains_any(
        text,
        &[
            "could you ",
            "would you ",
            "will you ",
            "can you ",
            "shouldn't you ",
            "would you mind ",
        ],
    );
    korean || english_addressee
}

fn is_information_question(text: &str) -> bool {
    starts_with_any(
        text,
        &[
            "어디서 ",
            "어떻게 ",
            "왜 ",
            "무엇",
            "뭘 ",
            "where ",
            "how ",
            "why ",
            "what ",
            "when ",
            "can i ",
            "could i ",
        ],
    ) || contains_any(text, &[" 어디서 ", " 어떻게 "])
}

fn select_target(
    text: &str,
    kind: PragmaticIntentKindIR,
    mentions: &[PragmaticActionMentionIR],
    active_predicates: &[String],
) -> Option<PragmaticActionMentionIR> {
    if kind == PragmaticIntentKindIR::GoalCorrection {
        return mentions
            .iter()
            .filter(|mention| !position_inside_quoted_span(text, mention.start_byte))
            .find(|mention| {
                !active_predicates
                    .iter()
                    .any(|active| active.eq_ignore_ascii_case(&mention.canonical_predicate))
            })
            .cloned();
    }
    mentions
        .iter()
        .find(|mention| !position_inside_quoted_span(text, mention.start_byte))
        .cloned()
}

fn extract_subject(
    text: &str,
    mention: &PragmaticActionMentionIR,
    inherited: Option<&str>,
) -> Option<String> {
    if let Some(inherited) = inherited.filter(|value| !value.trim().is_empty()) {
        return Some(clean_subject(inherited));
    }
    if mention
        .surface
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        let tail = text[mention.start_byte + mention.surface.len()..]
            .split([',', ';', '.', '?', '!'])
            .next()
            .unwrap_or_default()
            .trim();
        let tail = tail.split(" instead of ").next().unwrap_or(tail).trim();
        let cleaned = clean_subject(tail);
        return (!cleaned.is_empty()).then_some(cleaned);
    }
    let prefix = &text[..mention.start_byte];
    let clause_prefix = prefix
        .rsplit(['.', '?', '!', ';', '\n', '\r'])
        .next()
        .unwrap_or_default()
        .trim();
    if mention.intent == PlanIntentIR::Investigate
        && clause_prefix
            .split_whitespace()
            .next_back()
            .is_some_and(|token| {
                ["는지", "은지", "인지", "한지", "할지", "했는지", "되는지"]
                    .iter()
                    .any(|suffix| token.ends_with(suffix))
            })
    {
        let subject = clean_subject(clause_prefix);
        return (!subject.is_empty()).then_some(subject);
    }
    prefix.split_whitespace().rev().find_map(|token| {
        let token = token.trim_matches(|character: char| {
            character.is_ascii_punctuation() || matches!(character, '“' | '”' | '‘' | '’')
        });
        ["을", "를"]
            .iter()
            .find_map(|suffix| token.strip_suffix(suffix))
            .filter(|stem| !stem.is_empty())
            .map(clean_subject)
    })
}

fn clean_subject(value: &str) -> String {
    let mut tokens = value
        .trim_matches(|character: char| {
            character.is_ascii_punctuation() || matches!(character, '“' | '”' | '‘' | '’')
        })
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "the" | "a" | "an" | "please" | "first" | "again" | "next" | "instead"
            )
        })
        .collect::<Vec<_>>();
    while tokens.ends_with(&["for", "me"]) || tokens.ends_with(&["if", "needed"]) {
        tokens.truncate(tokens.len().saturating_sub(2));
    }
    tokens
        .join(" ")
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_string()
}

fn confidence(kind: PragmaticIntentKindIR, has_target: bool, has_subject: bool) -> u16 {
    let base: u16 = match kind {
        PragmaticIntentKindIR::MetalinguisticMention => 970,
        PragmaticIntentKindIR::GoalCorrection => 950,
        PragmaticIntentKindIR::SelfOffer => 940,
        PragmaticIntentKindIR::RhetoricalEvaluation => 930,
        PragmaticIntentKindIR::PreferenceRequest => 920,
        PragmaticIntentKindIR::ConventionalIndirectRequest => 910,
        PragmaticIntentKindIR::AdvisorySuggestion => 890,
        PragmaticIntentKindIR::InformationQuestion => 880,
    };
    base.saturating_sub(u16::from(!has_target) * 80 + u16::from(!has_subject) * 40)
}

fn intent_evidence(kind: PragmaticIntentKindIR, text: &str) -> Vec<String> {
    vec![
        format!("PRAGMATIC_CONSTRUCTION={kind:?}").to_uppercase(),
        format!("SURFACE_MOOD_QUESTION={}", text.contains('?')),
        "SEMANTIC_AUTHORITY=false".to_string(),
        "EXTERNAL_ACTION_EXECUTION_AUTHORIZED=false".to_string(),
    ]
}

fn suppress_candidates(analysis: &mut CompositionalAnalysisIR, kind: PragmaticIntentKindIR) {
    if kind == PragmaticIntentKindIR::MetalinguisticMention {
        let quoted_frame_ids = analysis
            .frames
            .iter_mut()
            .filter(|frame| frame.embedded_under_quote)
            .map(|frame| {
                frame.external_execution_authorized = false;
                frame.frame_id.clone()
            })
            .collect::<Vec<_>>();
        for candidate in &mut analysis.candidates {
            if quoted_frame_ids.contains(&candidate.source_frame_id) {
                candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
                candidate.external_execution_authorized = false;
                candidate.blockers.push(
                    "PRAGMATIC_METALINGUISTIC_QUOTED_CONTENT_BLOCKS_ACTION_PROJECTION".to_string(),
                );
            }
        }
        analysis.selected_candidate_ids.retain(|candidate_id| {
            analysis.candidates.iter().any(|candidate| {
                &candidate.candidate_id == candidate_id
                    && !quoted_frame_ids.contains(&candidate.source_frame_id)
            })
        });
        analysis.selected_candidate_id = analysis
            .selected_candidate_id
            .take()
            .filter(|candidate_id| analysis.selected_candidate_ids.contains(candidate_id));
        if analysis.selected_candidate_ids.is_empty() {
            analysis.goal_graph = None;
        }
        return;
    }
    for frame in &mut analysis.frames {
        frame.external_execution_authorized = false;
    }
    for candidate in &mut analysis.candidates {
        candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
        candidate.external_execution_authorized = false;
        candidate
            .blockers
            .push(format!("PRAGMATIC_FORCE_{kind:?}_BLOCKS_ACTION_PROJECTION").to_uppercase());
    }
    analysis.selected_candidate_id = None;
    analysis.selected_candidate_ids.clear();
    analysis.goal_graph = None;
}

fn ensure_frame(
    analysis: &mut CompositionalAnalysisIR,
    text: &str,
    preferred_frame_id: Option<&str>,
    canonical: &str,
    intent: PlanIntentIR,
    subject: &str,
    authorized: bool,
) -> String {
    let existing_index = preferred_frame_id
        .and_then(|frame_id| {
            analysis
                .frames
                .iter()
                .position(|frame| frame.frame_id == frame_id)
        })
        .or_else(|| {
            analysis.frames.iter().position(|frame| {
                frame.canonical_predicate == canonical && !frame.embedded_under_quote
            })
        });
    if let Some(index) = existing_index {
        let frame = &mut analysis.frames[index];
        frame.intent_hint = intent;
        frame.theme = subject.to_string();
        frame.external_execution_authorized = authorized;
        return frame.frame_id.clone();
    }
    let frame_id = format!("PRAGMATIC-FRAME-{:02}", analysis.frames.len() + 1);
    analysis.frames.push(PredicateFrameIR {
        frame_id: frame_id.clone(),
        clause_id: "PRAGMATIC-CLAUSE-01".to_string(),
        predicate_surface: canonical.to_lowercase(),
        canonical_predicate: canonical.to_string(),
        intent_hint: intent,
        theme: subject.to_string(),
        mood: if text.contains('?') {
            FrameMoodIR::Interrogative
        } else {
            FrameMoodIR::Declarative
        },
        modality: if authorized {
            FrameModalityIR::Requested
        } else {
            FrameModalityIR::Possible
        },
        polarity: FramePolarityIR::Positive,
        embedded_under_quote: false,
        external_execution_authorized: authorized,
        source_start_byte: 0,
    });
    frame_id
}

struct CandidateProjection<'a> {
    frame_id: &'a str,
    canonical: &'a str,
    intent: PlanIntentIR,
    subject: &'a str,
    authorized: bool,
    kind: PragmaticIntentKindIR,
    confidence_millis: u16,
}

fn ensure_candidate(
    analysis: &mut CompositionalAnalysisIR,
    projection: CandidateProjection<'_>,
) -> String {
    let CandidateProjection {
        frame_id,
        canonical,
        intent,
        subject,
        authorized,
        kind,
        confidence_millis,
    } = projection;
    if let Some(candidate) = analysis
        .candidates
        .iter_mut()
        .find(|candidate| candidate.source_frame_id == frame_id)
    {
        candidate.intent = intent;
        candidate.subject = subject.to_string();
        candidate.desired_outcome = desired_outcome(canonical, intent, subject);
        candidate.disposition = CandidateDispositionIR::Viable;
        candidate.score_millis = confidence_millis;
        candidate.external_execution_authorized = authorized;
        candidate.blockers.clear();
        candidate
            .evidence
            .push(format!("PRAGMATIC_FORCE={kind:?}").to_uppercase());
        return candidate.candidate_id.clone();
    }
    let candidate_id = format!("PRAGMATIC-CANDIDATE-{:02}", analysis.candidates.len() + 1);
    analysis.candidates.push(InterpretationCandidateIR {
        candidate_id: candidate_id.clone(),
        source_frame_id: frame_id.to_string(),
        intent,
        subject: subject.to_string(),
        desired_outcome: desired_outcome(canonical, intent, subject),
        disposition: CandidateDispositionIR::Viable,
        score_millis: confidence_millis,
        external_execution_authorized: authorized,
        evidence: vec![format!("PRAGMATIC_FORCE={kind:?}").to_uppercase()],
        blockers: Vec::new(),
    });
    candidate_id
}

fn desired_outcome(canonical: &str, intent: PlanIntentIR, subject: &str) -> String {
    match intent {
        PlanIntentIR::Investigate => format!("determine the verified state of {subject}"),
        PlanIntentIR::Repair => format!("restore an acceptable state for {subject}"),
        _ => format!("apply {canonical} to {subject}"),
    }
}

fn has_quoted_span(text: &str) -> bool {
    let ascii_quotes = text.matches('"').count() >= 2 || text.matches('\'').count() >= 2;
    let unicode_quotes =
        (text.contains('“') && text.contains('”')) || (text.contains('‘') && text.contains('’'));
    ascii_quotes || unicode_quotes
}

fn starts_with_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.starts_with(pattern))
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    fn graph(text: &str) -> PragmaticIntentGraphIR {
        let analysis = CompositionalSemanticAnalyzer.analyze(text);
        PragmaticIntentAnalyzer.analyze(text, None, &[], &analysis)
    }

    #[test]
    fn rhetorical_question_blocks_goal_projection() {
        let graph = graph("Who would call this a success?");
        assert_eq!(
            graph.primary_kind(),
            Some(PragmaticIntentKindIR::RhetoricalEvaluation)
        );
        assert!(graph.suppresses_goal_projection());
    }

    #[test]
    fn addressee_request_differs_from_first_person_information_question() {
        let request = graph("Would you inspect the log for me?");
        let question = graph("Where can I inspect the log?");
        assert_eq!(
            request.primary_kind(),
            Some(PragmaticIntentKindIR::ConventionalIndirectRequest)
        );
        assert_eq!(
            question.primary_kind(),
            Some(PragmaticIntentKindIR::InformationQuestion)
        );
    }

    #[test]
    fn quoted_command_is_metalinguistic_not_executable() {
        let graph = graph("What does \"delete the cache\" mean?");
        assert_eq!(
            graph.primary_kind(),
            Some(PragmaticIntentKindIR::MetalinguisticMention)
        );
        assert!(graph.suppresses_goal_projection());
    }

    #[test]
    fn reported_action_is_suppressed_while_addressee_request_is_selected() {
        let graph = graph("Dana said to delete the cache, but would you inspect the log?");
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert_eq!(composition.selected_node_ids.len(), 1);
        let selected = composition
            .nodes
            .iter()
            .find(|node| composition.selected_node_ids.contains(&node.node_id))
            .expect("selected request");
        assert_eq!(selected.canonical_predicate, "INVESTIGATE");
        assert!(composition.nodes.iter().any(|node| {
            node.canonical_predicate == "DELETE"
                && node.force == PragmaticClauseForceIR::ReportedSuggestion
                && node.projection == PragmaticGoalProjectionIR::Suppressed
        }));
    }

    #[test]
    fn independent_request_after_reported_quote_is_selected() {
        let graph = graph(
            "The runbook says 'publish the bundle and report that result.' Assess recovery cost only; do not publish it.",
        );
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert!(
            composition.nodes.iter().any(|node| {
                composition.selected_node_ids.contains(&node.node_id)
                    && node.canonical_predicate == "INVESTIGATE"
                    && node.subject.contains("recovery")
            }),
            "outer assessment: {composition:#?}"
        );
        assert!(!composition.nodes.iter().any(|node| {
            composition.selected_node_ids.contains(&node.node_id)
                && node.canonical_predicate == "DEPLOY"
        }));
    }

    #[test]
    fn independent_request_after_curly_quoted_note_is_selected() {
        let graph = graph(
            "The note reads, “analyze the archive and report that result.” Evaluate rollback time only; do not analyze the archive.",
        );
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert!(
            composition.nodes.iter().any(|node| {
                composition.selected_node_ids.contains(&node.node_id)
                    && node.canonical_predicate == "INVESTIGATE"
                    && node.subject.contains("rollback")
            }),
            "outer evaluation: {composition:#?}"
        );
    }

    #[test]
    fn action_alternative_fails_closed() {
        let graph = graph("Inspect the queue or repair the worker.");
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert!(composition.selected_node_ids.is_empty());
        assert_eq!(
            composition.unresolved_ambiguities,
            vec!["PRAGMATIC_ACTION_ALTERNATIVE"]
        );
    }

    #[test]
    fn conditional_request_is_selected_but_not_immediate() {
        let graph = graph("When needed, inspect the queue first.");
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert!(composition.has_selected_authorized_request());
        assert!(composition.has_selected_conditional_request());
        assert!(!composition.has_selected_immediate_request());
    }

    #[test]
    fn capability_question_never_selects_action() {
        let graph = graph("Are you able to inspect the queue?");
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert!(composition.selected_node_ids.is_empty());
        assert!(composition.nodes.iter().all(|node| {
            node.force == PragmaticClauseForceIR::CapabilityQuestion
                && node.projection == PragmaticGoalProjectionIR::Suppressed
        }));
    }

    #[test]
    fn postposed_override_excludes_second_action() {
        let analysis =
            CompositionalSemanticAnalyzer.analyze("Repair the cache rather than inspect it.");
        let graph = PragmaticIntentAnalyzer.analyze(
            "Repair the cache rather than inspect it.",
            None,
            &[],
            &analysis,
        );
        let composition = graph.composition.expect("composition");
        assert!(composition.validate());
        assert_eq!(composition.selected_node_ids.len(), 1);
        assert!(composition.nodes.iter().any(|node| {
            node.canonical_predicate == "INVESTIGATE"
                && node.force == PragmaticClauseForceIR::Prohibition
        }));
    }

    #[test]
    fn graph_hash_detects_tampering() {
        let graph = graph("Would you inspect the queue?");
        let mut composition = graph.composition.expect("composition");
        assert!(composition.validate());
        composition.nodes[0].subject.push_str(" tampered");
        assert!(!composition.validate());
    }

    #[test]
    fn prohibited_either_reference_is_not_an_action_alternative() {
        let graph = graph(
            "'Delete the cache and deploy' is an example. Explain why it is unsafe; do not perform either action.",
        );
        let composition = graph.composition.expect("composition");
        assert!(composition.unresolved_ambiguities.is_empty());
        assert!(composition.nodes.iter().any(|node| {
            node.canonical_predicate == "EXPLAIN"
                && composition.selected_node_ids.contains(&node.node_id)
        }));
        assert!(composition
            .nodes
            .iter()
            .filter(|node| { matches!(node.canonical_predicate.as_str(), "DELETE" | "DEPLOY") })
            .all(|node| node.projection == PragmaticGoalProjectionIR::Suppressed));
    }

    #[test]
    fn korean_requestive_dalla_selects_assessment_over_reported_deploy() {
        let graph = graph(
            "리뷰어는 '패치를 배포해'라고 했지만 나는 롤백 위험만 평가해 달라는 거야. 배포하지 마.",
        );
        let composition = graph.composition.expect("composition");
        assert!(composition.unresolved_ambiguities.is_empty());
        let selected = composition
            .nodes
            .iter()
            .find(|node| composition.selected_node_ids.contains(&node.node_id))
            .expect("selected assessment");
        assert_eq!(selected.canonical_predicate, "INVESTIGATE");
        assert!(composition
            .nodes
            .iter()
            .filter(|node| node.canonical_predicate == "DEPLOY")
            .all(|node| node.projection == PragmaticGoalProjectionIR::Suppressed));
    }

    #[test]
    fn mismatched_utf8_frame_offset_fails_closed_instead_of_slicing_mid_character() {
        let text = "문서는 '캐시를 수리해'라고 쓰여 있어. 로그만 확인해.";
        let analysis = CompositionalSemanticAnalyzer.analyze(text);
        let mut frame = analysis.frames.first().expect("frame").clone();
        frame.source_start_byte = 1;
        assert!(!text.is_char_boundary(frame.source_start_byte));
        assert!(!independent_directive_after_reported_quote(text, &frame));
    }
}
