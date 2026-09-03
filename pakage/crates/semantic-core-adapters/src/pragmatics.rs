//! Inspectable discourse and pragmatic reasoning for indirect user intent.
//!
//! This layer sits between surface normalization and the language-independent
//! planner. It does not treat a sentence as one command-shaped bag of words.
//! Instead it separates propositions by discourse role, links causes,
//! conditions, costs, and benefits, and emits an explicit decision policy when
//! continuing the current work is conditional on a claimed payoff.

use std::collections::BTreeSet;

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

use crate::compositional_semantics::{
    CandidateDispositionIR, CompositionalAnalysisIR, CompositionalSemanticAnalyzer,
    PredicateLexemeIR, ScopeKindIR,
};
use crate::language_center::{
    LanguageCenterGoalDecisionIR, LanguageCenterGoalDecisionSourceIR, LanguageCenterGoalEffectIR,
    LanguageCenterGoalProjectionIR, LanguageCenterIR, LanguageCenterPipeline,
    LanguageCenterProjectionIR, LanguageCenterSources,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageUnderstandingIR, PragmaticFunctionIR};
use crate::modality::{ModalIllocutionIR, ModalWorldIR};
use crate::native_language_circuit::NativeTurnIR;
use crate::nonliteral::{NonliteralAnalysisIR, NonliteralAnalyzer};
use crate::pragmatic_intent::{
    PragmaticGoalProjectionIR, PragmaticIntentAnalyzer, PragmaticIntentGraphIR,
    PragmaticIntentKindIR,
};
use crate::utterance_intent::{CommunicativeIntentIR, ExpectedResponseKindIR};

pub const PRAGMATIC_INTERPRETATION_SCHEMA: &str = "B_CORE_PRAGMATIC_INTERPRETATION_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseClauseRoleIR {
    Statement,
    Observation,
    Cause,
    Contrast,
    Condition,
    CurrentAction,
    Problem,
    KnowledgeGap,
    Proposal,
    Cost,
    Benefit,
    Evaluation,
    Decision,
    Negation,
    Uncertainty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PropositionPolarityIR {
    Positive,
    Negative,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseClauseIR {
    pub clause_id: String,
    pub surface_text: String,
    pub roles: Vec<DiscourseClauseRoleIR>,
    pub polarity: PropositionPolarityIR,
    pub semantic_cues: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseRelationKindIR {
    Cause,
    Contrast,
    Condition,
    Consequence,
    Justification,
    Elaboration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseRelationIR {
    pub source_clause_id: String,
    pub target_clause_id: String,
    pub kind: DiscourseRelationKindIR,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpeechActIR {
    Inform,
    RequestAction,
    Ask,
    Suggest,
    Approve,
    Reject,
    NegativeEvaluation,
    ConditionalCommitment,
    ConditionalContinuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GoalCommitmentIR {
    ExplicitRequest,
    ImplicitRequest,
    Suggestion,
    ConditionalAuthorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueParticipantIR {
    User,
    Assistant,
    ThirdParty,
    System,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IllocutionaryForceIR {
    SelfCommitment,
    ReportedCommitment,
    CapabilityQuestion,
    IndirectActionRequest,
    DeferredConditionalRequest,
    AnswerOnlyInformationRequest,
    GoalWithdrawal,
    OutcomeClaimConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommitmentActivationIR {
    Immediate,
    ConditionPending,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GoalWithdrawalScopeIR {
    AllActiveGoals,
    EventOrdinal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalWithdrawalIR {
    pub scope: GoalWithdrawalScopeIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_ordinal: Option<usize>,
    pub evidence_surface: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequiredOutcomeEvidenceIR {
    DirectVerification,
    ExecutionRecord,
    Receipt,
    ObservableEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeClaimPolicyIR {
    pub policy: String,
    pub verified_outcome_only: bool,
    pub required_evidence: Vec<RequiredOutcomeEvidenceIR>,
    pub evidence_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IllocutionaryCommitmentIR {
    pub commitment_id: String,
    pub actor: DialogueParticipantIR,
    pub addressee: DialogueParticipantIR,
    pub force: IllocutionaryForceIR,
    pub activation: CommitmentActivationIR,
    pub proposition_surface: String,
    pub external_execution_authorized: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IllocutionaryCommitmentGraphIR {
    #[serde(default)]
    pub commitments: Vec<IllocutionaryCommitmentIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_withdrawal: Option<GoalWithdrawalIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_claim_policy: Option<OutcomeClaimPolicyIR>,
}

impl IllocutionaryCommitmentGraphIR {
    pub fn primary_force(&self) -> Option<IllocutionaryForceIR> {
        self.commitments.first().map(|item| item.force)
    }

    pub fn blocks_current_goal_projection(&self) -> bool {
        self.primary_force().is_some_and(|force| {
            matches!(
                force,
                IllocutionaryForceIR::SelfCommitment
                    | IllocutionaryForceIR::ReportedCommitment
                    | IllocutionaryForceIR::CapabilityQuestion
                    | IllocutionaryForceIR::DeferredConditionalRequest
                    | IllocutionaryForceIR::GoalWithdrawal
                    | IllocutionaryForceIR::OutcomeClaimConstraint
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveGoalContextIR {
    pub goal_id: String,
    pub canonical_predicate: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDeferredContextIR {
    pub commitment_id: String,
    pub canonical_predicate: String,
    pub subject: String,
    pub condition_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserFeedbackKindIR {
    Unhelpful,
    Misunderstood,
    MissedPoint,
    TooVerbose,
    TooBrief,
    Incorrect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFeedbackIR {
    pub kind: UserFeedbackKindIR,
    pub target_surface: String,
    pub evidence_clause_ids: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferredPragmaticGoalIR {
    pub intent: PlanIntentIR,
    pub subject: String,
    pub desired_outcome: String,
    pub commitment: GoalCommitmentIR,
    pub external_execution_authorized: bool,
    pub basis_clause_ids: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionBranchActionIR {
    ContinueCurrentWork,
    StopCurrentWork,
    ReportNegativeAndAskWhetherToStop,
    ReportUncertaintyAndAskHowToProceed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationDecisionGateIR {
    pub current_task: String,
    pub required_benefit: String,
    pub verification_required: bool,
    pub positive_action: DecisionBranchActionIR,
    pub negative_action: DecisionBranchActionIR,
    pub unknown_action: DecisionBranchActionIR,
    pub supporting_clause_ids: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePolicyIR {
    pub proxy_measure_clause_ids: Vec<String>,
    pub direct_outcome_clause_ids: Vec<String>,
    pub proxy_only_is_insufficient: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticContextIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_required_benefit: Option<String>,
    #[serde(default)]
    pub pending_gate_suspended: bool,
    #[serde(default)]
    pub active_goals: Vec<ActiveGoalContextIR>,
    #[serde(default)]
    pub pending_deferred_commitments: Vec<PendingDeferredContextIR>,
    #[serde(default)]
    pub recent_subjects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticInterpretationIR {
    pub schema: String,
    pub speech_act: SpeechActIR,
    pub clauses: Vec<DiscourseClauseIR>,
    pub relations: Vec<DiscourseRelationIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_current_task: Option<String>,
    pub accepted_costs: Vec<String>,
    pub expected_benefits: Vec<String>,
    pub evidence_policy: EvidencePolicyIR,
    pub nonliteral_analysis: NonliteralAnalysisIR,
    #[serde(default)]
    pub language_center: LanguageCenterIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_center_goal_projection: Option<LanguageCenterGoalProjectionIR>,
    pub compositional_analysis: CompositionalAnalysisIR,
    #[serde(default)]
    pub pragmatic_intent_graph: PragmaticIntentGraphIR,
    #[serde(default)]
    pub illocutionary_commitments: IllocutionaryCommitmentGraphIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_goal: Option<InferredPragmaticGoalIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_feedback: Option<UserFeedbackIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_gate: Option<ContinuationDecisionGateIR>,
    pub unresolved_bindings: Vec<String>,
    pub semantic_tags: Vec<String>,
    pub confidence_millis: u16,
}

impl PragmaticInterpretationIR {
    pub fn has_continuation_gate(&self) -> bool {
        self.continuation_gate.is_some()
    }

    /// Reconciles the native circuit at the Language Center boundary.  The
    /// orchestrator does not mutate individual compatibility graphs itself;
    /// native, compositional, pragmatic, and illocutionary evidence are sealed
    /// together and one synchronized projection receipt is emitted.
    pub(crate) fn reconcile_native_projection(&mut self, native: &NativeTurnIR, source: &str) {
        self.pragmatic_intent_graph
            .project_native_goal_compatibility(native, source);
        self.compositional_analysis.clarification_required = false;
        self.unresolved_bindings.clear();
        self.pragmatic_intent_graph.unresolved_ambiguities.clear();

        let center = LanguageCenterPipeline.build(LanguageCenterSources {
            phenotype: native.language,
            native: Some(native),
            composition: &self.compositional_analysis,
            pragmatic_intent: &self.pragmatic_intent_graph,
            illocution: &self.illocutionary_commitments,
        });
        let decisions = collect_language_center_goal_decisions(
            &center,
            &self.pragmatic_intent_graph,
            &self.illocutionary_commitments,
            self.continuation_gate.is_some(),
        );
        let projection = LanguageCenterGoalProjectionIR::seal(
            &center,
            &self.compositional_analysis,
            &self.pragmatic_intent_graph,
            &self.illocutionary_commitments,
            &self.compositional_analysis,
            decisions,
        );
        debug_assert!(center.validate());
        debug_assert!(projection.validate_against(&center, &self.compositional_analysis));
        self.language_center = center;
        self.language_center_goal_projection = Some(projection);
    }

    /// Projects the pragmatic decision into the existing planning request.
    /// The planner receives a verification goal, never the whole paragraph as
    /// an opaque executable target.
    pub fn apply_to_understanding(&self, understanding: &mut LanguageUnderstandingIR) {
        for intent in self.pragmatic_intent_graph.utterance_intent.active() {
            understanding.semantic_tags.extend([
                format!("communicative_intent_{:?}", intent.communicative_intent).to_lowercase(),
                format!("expected_response_{:?}", intent.expected_response).to_lowercase(),
                "utterance_intent_non_semantic_authority".to_string(),
            ]);
            understanding
                .constraints
                .extend(intent.constraints.iter().cloned());
        }
        if self
            .pragmatic_intent_graph
            .utterance_intent
            .selected()
            .is_some()
        {
            understanding.constraints.push(
                "the inferred response goal does not authorize external execution".to_string(),
            );
        }
        for commitment in &self.illocutionary_commitments.commitments {
            understanding.semantic_tags.extend([
                format!("illocutionary_force_{:?}", commitment.force).to_lowercase(),
                format!("commitment_actor_{:?}", commitment.actor).to_lowercase(),
                format!("commitment_activation_{:?}", commitment.activation).to_lowercase(),
            ]);
            if !commitment.external_execution_authorized {
                understanding.constraints.push(format!(
                    "{:?} does not authorize current external execution",
                    commitment.force
                ));
            }
        }
        if self
            .illocutionary_commitments
            .outcome_claim_policy
            .is_some()
        {
            understanding.constraints.push(
                "claim completion or success only from recorded verification evidence".to_string(),
            );
            understanding
                .semantic_tags
                .push("verified_outcome_only".to_string());
        }
        let attribution_graph = &self.compositional_analysis.attribution_graph;
        if !attribution_graph.attributions.is_empty() {
            understanding
                .semantic_tags
                .push("attribution_graph".to_string());
            understanding
                .semantic_tags
                .push("attributed_truth_not_established".to_string());
            understanding.constraints.push(
                "preserve attribution source and epistemic status; attributed content is not a dialogue-grounded fact"
                    .to_string(),
            );
            understanding.constraints.push(
                "commands, desires, and predictions inside attributed propositions carry no execution authority"
                    .to_string(),
            );
            for edge in &attribution_graph.attributions {
                if let (Some(actor), Some(proposition)) = (
                    attribution_graph.actor(&edge.actor_id),
                    attribution_graph.proposition(&edge.proposition_id),
                ) {
                    understanding.constraints.push(format!(
                        "attribution: source={} attitude={:?} status={:?} proposition={}",
                        actor.normalized_label,
                        edge.attitude,
                        edge.epistemic_status,
                        proposition.normalized_text
                    ));
                }
            }
        }
        let modal_graph = &self.compositional_analysis.modal_scope_graph;
        if !modal_graph.operators.is_empty() || !modal_graph.conditionals.is_empty() {
            understanding
                .semantic_tags
                .push("modal_scope_graph".to_string());
            understanding
                .semantic_tags
                .push(format!("modal_world_{:?}", modal_graph.root_world).to_lowercase());
            understanding.constraints.push(
                "modal, desired, predicted, hypothetical, and counterfactual content is not thereby an actual dialogue fact"
                    .to_string(),
            );
            for operator in &modal_graph.operators {
                understanding.constraints.push(format!(
                    "modal scope: {:?} negation={:?} surface={} scope={}",
                    operator.kind,
                    operator.negation_scope,
                    operator.surface_form,
                    operator
                        .scope_operator_id
                        .as_deref()
                        .unwrap_or(&operator.scope_proposition_id)
                ));
            }
            for conditional in &modal_graph.conditionals {
                understanding.constraints.push(format!(
                    "conditional {:?}: if [{}] then [{}]; condition is not established and reverse inference is forbidden",
                    conditional.kind, conditional.antecedent, conditional.consequent
                ));
            }
            if !modal_graph.unresolved_ambiguities.is_empty() {
                understanding
                    .semantic_tags
                    .push("modal_scope_ambiguity_preserved".to_string());
                understanding.constraints.extend(
                    modal_graph
                        .unresolved_ambiguities
                        .iter()
                        .map(|item| format!("unresolved modal reading: {item}")),
                );
            }
        }
        if modal_graph.illocution == ModalIllocutionIR::PoliteRequest {
            understanding
                .semantic_tags
                .push("indirect_polite_request".to_string());
        }
        let role_graph = &self.compositional_analysis.semantic_role_graph;
        if !role_graph.nodes.is_empty() {
            understanding
                .semantic_tags
                .push("semantic_role_graph".to_string());
            for candidate in self.compositional_analysis.selected_candidates() {
                if let Some(binding) =
                    role_graph.role_constraint_for_frame(&candidate.source_frame_id)
                {
                    understanding.constraints.push(format!(
                        "preserve semantic role bindings for {}: {binding}",
                        candidate.source_frame_id
                    ));
                }
            }
            for scope in &role_graph.quantifier_scopes {
                if let Some(target) = role_graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == scope.target_node_id)
                {
                    understanding.constraints.push(format!(
                        "quantifier scope: {:?}({}){}",
                        scope.quantifier,
                        target.normalized_label,
                        scope
                            .cardinality
                            .map(|value| format!("={value}"))
                            .unwrap_or_default()
                    ));
                }
            }
            if !role_graph.quantifier_scopes.is_empty() {
                understanding
                    .semantic_tags
                    .push("quantifier_scope_explicit".to_string());
            }
            if !role_graph.event_relations.is_empty() {
                understanding
                    .semantic_tags
                    .push("event_relation_graph".to_string());
            }
            if !role_graph.relative_clause_attachments.is_empty() {
                understanding
                    .semantic_tags
                    .push("relative_clause_graph".to_string());
                for attachment in &role_graph.relative_clause_attachments {
                    let head = role_graph
                        .nodes
                        .iter()
                        .find(|node| node.node_id == attachment.head_node_id)
                        .map_or("unknown", |node| node.normalized_label.as_str());
                    let dependents = attachment
                        .dependent_node_ids
                        .iter()
                        .filter_map(|node_id| {
                            role_graph
                                .nodes
                                .iter()
                                .find(|node| node.node_id == *node_id)
                                .map(|node| node.normalized_label.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    understanding.constraints.push(format!(
                        "relative attachment: head={head} predicate={} dependents=[{dependents}] negated={}",
                        attachment.normalized_predicate, attachment.negated
                    ));
                }
            }
        }
        let scope_graph = &self.compositional_analysis.grammatical_scope_graph;
        if !scope_graph.nodes.is_empty() && scope_graph.validate() {
            understanding
                .semantic_tags
                .push("grammatical_scope_graph".to_string());
            for node in scope_graph.nodes.iter().filter(|node| {
                !matches!(
                    node.kind,
                    crate::grammatical_scope::GrammaticalScopeNodeKindIR::Event
                        | crate::grammatical_scope::GrammaticalScopeNodeKindIR::Entity
                )
            }) {
                understanding.constraints.push(format!(
                    "grammatical scope operator: {:?} label={} evidence={}",
                    node.kind,
                    node.operator_label.as_deref().unwrap_or("UNKNOWN"),
                    node.evidence_surface
                ));
            }
            if !scope_graph.unresolved_ambiguities.is_empty() {
                understanding
                    .semantic_tags
                    .push("grammatical_scope_ambiguity_preserved".to_string());
                understanding.constraints.extend(
                    scope_graph
                        .unresolved_ambiguities
                        .iter()
                        .map(|ambiguity| format!("unresolved grammatical reading: {ambiguity}")),
                );
            }
        }
        if self.compositional_analysis.blocked_execution_count() > 0 {
            understanding.constraints.push(
                "quoted, negated, reported, and hypothetical predicates carry no execution authority"
                    .to_string(),
            );
            understanding
                .semantic_tags
                .push("scope_bounded_execution_authority".to_string());
        }
        if self.compositional_analysis.clarification_required {
            understanding.constraints.push(
                "preserve competing compositional interpretations until the user disambiguates"
                    .to_string(),
            );
            understanding
                .semantic_tags
                .push("compositional_interpretation_ambiguous".to_string());
        }
        let mut nonliteral_projection = false;
        for expression in &self.nonliteral_analysis.expressions {
            match expression.selected_reading {
                crate::nonliteral::ReadingSelectionIR::Figurative => {
                    understanding
                        .semantic_tags
                        .push(expression.figurative_concept.clone());
                    nonliteral_projection = true;
                }
                crate::nonliteral::ReadingSelectionIR::Ambiguous => {
                    understanding
                        .semantic_tags
                        .push("nonliteral_reading_ambiguous".to_string());
                    nonliteral_projection = true;
                }
                crate::nonliteral::ReadingSelectionIR::Literal => {}
            }
        }
        if self.nonliteral_analysis.literal_execution_blocked {
            understanding.constraints.push(
                "do not execute a literal reading of a figurative or incongruous expression"
                    .to_string(),
            );
        }
        if let Some(gate) = &self.continuation_gate {
            understanding.intent = PlanIntentIR::Investigate;
            understanding.subject = format!(
                "continuation_gate(task={}; required_benefit={})",
                gate.current_task, gate.required_benefit
            );
            understanding.constraints.extend([
                "continue the current work only when the required benefit is supported".to_string(),
                "on a negative result, report it and ask the user before stopping".to_string(),
                "on unresolved evidence, report uncertainty and ask how to proceed".to_string(),
            ]);
            if self.evidence_policy.proxy_only_is_insufficient {
                understanding.constraints.push(
                    "verify the required benefit with direct outcome evidence rather than proxy metrics"
                        .to_string(),
                );
                understanding
                    .semantic_tags
                    .push("proxy_metric_insufficient".to_string());
            }
            understanding.desired_outcomes = vec![
                format!(
                    "evidence-backed verdict on whether {} yields {}",
                    gate.current_task, gate.required_benefit
                ),
                "a branch decision that distinguishes supported, rejected, and unresolved benefit"
                    .to_string(),
            ];
            understanding.semantic_tags.extend([
                "conditional_continuation".to_string(),
                "continuation_decision_gate".to_string(),
                "required_benefit_verification".to_string(),
                "user_stop_authority".to_string(),
            ]);
            understanding
                .pragmatic_functions
                .extend([PragmaticFunctionIR::Condition, PragmaticFunctionIR::Proceed]);
            understanding.confidence_millis = understanding
                .confidence_millis
                .max(self.confidence_millis.min(gate.confidence_millis));
            finalize_understanding(understanding);
            return;
        }
        if let Some(graph) = self
            .compositional_analysis
            .goal_graph
            .as_ref()
            .filter(|graph| graph.nodes.len() > 1)
        {
            understanding.intent = PlanIntentIR::Plan;
            understanding.subject = format!(
                "compositional_goal_graph({})",
                graph
                    .nodes
                    .iter()
                    .map(|node| format!("{:?}:{}", node.intent, node.subject))
                    .collect::<Vec<_>>()
                    .join(" -> ")
            );
            understanding.desired_outcomes = graph
                .nodes
                .iter()
                .map(|node| node.desired_outcome.clone())
                .collect();
            understanding
                .constraints
                .extend(graph.edges.iter().map(|edge| {
                    format!(
                        "goal ordering {:?}: {} -> {}",
                        edge.relation, edge.source_node_id, edge.target_node_id
                    )
                }));
            understanding.constraints.extend(
                graph
                    .conditions
                    .iter()
                    .map(|condition| format!("execute the guarded goal only if: {condition}")),
            );
            understanding.constraints.extend(
                graph
                    .prohibitions
                    .iter()
                    .map(|prohibition| format!("preserve explicit prohibition: {prohibition}")),
            );
            understanding.semantic_tags.extend([
                "compositional_goal_graph".to_string(),
                "ordered_multi_goal_request".to_string(),
                "scope_bounded_execution_authority".to_string(),
            ]);
            understanding.confidence_millis =
                understanding.confidence_millis.max(graph.confidence_millis);
            finalize_understanding(understanding);
            return;
        }
        let Some(goal) = &self.inferred_goal else {
            if nonliteral_projection {
                finalize_understanding(understanding);
            }
            return;
        };
        understanding.intent = goal.intent;
        understanding.subject = goal.subject.clone();
        understanding.desired_outcomes = vec![goal.desired_outcome.clone()];
        understanding
            .semantic_tags
            .push("pragmatically_inferred_goal".to_string());
        if !goal.external_execution_authorized {
            understanding.constraints.push(
                "the inferred goal may guide analysis but does not independently authorize external mutation"
                    .to_string(),
            );
        }
        understanding.confidence_millis =
            understanding.confidence_millis.max(goal.confidence_millis);
        finalize_understanding(understanding);
    }
}

fn expected_response_outcome(response: ExpectedResponseKindIR, target: &str) -> String {
    match response {
        ExpectedResponseKindIR::DiagnosisOrNextStep => {
            format!("identify a supported cause or safe next diagnostic step for {target}")
        }
        ExpectedResponseKindIR::Assessment => {
            format!("assess {target} against explicit readiness and safety evidence")
        }
        ExpectedResponseKindIR::Evidence => {
            format!("provide the available evidence supporting {target}")
        }
        ExpectedResponseKindIR::Recommendation => {
            format!("compare the alternatives for {target} under the stated constraints")
        }
        ExpectedResponseKindIR::Explanation => {
            format!("explain {target} from supported causal information")
        }
        ExpectedResponseKindIR::Summary => {
            format!("summarize the supported conclusion about {target}")
        }
        ExpectedResponseKindIR::VerifyThenDecide => {
            format!("verify the required benefit for {target} before choosing a branch")
        }
        ExpectedResponseKindIR::Clarification => {
            format!("obtain the missing context required to answer about {target}")
        }
    }
}

fn finalize_understanding(understanding: &mut LanguageUnderstandingIR) {
    understanding.constraints.sort();
    understanding.constraints.dedup();
    understanding.semantic_tags.sort();
    understanding.semantic_tags.dedup();
    understanding.pragmatic_functions.sort();
    understanding.pragmatic_functions.dedup();
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PragmaticReasoner;

impl PragmaticReasoner {
    pub fn interpret(&self, text: &str, context: &PragmaticContextIR) -> PragmaticInterpretationIR {
        self.interpret_with_predicates(text, context, &[])
    }

    pub fn interpret_with_predicates(
        &self,
        text: &str,
        context: &PragmaticContextIR,
        learned_predicates: &[PredicateLexemeIR],
    ) -> PragmaticInterpretationIR {
        self.interpret_with_predicates_and_illocution(text, text, context, learned_predicates)
    }

    pub fn interpret_with_predicates_and_illocution(
        &self,
        text: &str,
        illocutionary_surface: &str,
        context: &PragmaticContextIR,
        learned_predicates: &[PredicateLexemeIR],
    ) -> PragmaticInterpretationIR {
        let clause_texts = segment_clauses(text);
        let clauses = clause_texts
            .iter()
            .enumerate()
            .map(|(index, clause)| classify_clause(index, clause))
            .collect::<Vec<_>>();
        let relations = infer_relations(&clauses);
        let accepted_costs = clauses_with_role(&clauses, DiscourseClauseRoleIR::Cost);
        let mut expected_benefits = benefits_from_clauses(&clauses);
        let evidence_policy = infer_evidence_policy(&clauses);
        let nonliteral_analysis = NonliteralAnalyzer.analyze(text, context);
        let base_compositional_analysis =
            CompositionalSemanticAnalyzer.analyze_with_predicates(text, learned_predicates);
        let active_predicates = context
            .active_goals
            .iter()
            .map(|goal| goal.canonical_predicate.clone())
            .collect::<Vec<_>>();
        let active_subject = if context.active_goals.len() == 1 {
            context
                .active_goals
                .first()
                .map(|goal| goal.subject.as_str())
        } else {
            context.active_subject.as_deref()
        };
        let utterance_context_subject = if contains_any(
            &normalize(illocutionary_surface),
            &[
                "그래서 결론",
                "핵심만",
                "요지는",
                "bottom line",
                "takeaway",
                "in short",
                "왜 실패하는지",
                "왜 실패한지",
                "why it failed",
                "why that failed",
            ],
        ) && !context.recent_subjects.is_empty()
        {
            context.recent_subjects.first().cloned()
        } else {
            active_subject.map(str::to_string)
        };
        let pragmatic_intent_graph = PragmaticIntentAnalyzer.analyze(
            illocutionary_surface,
            utterance_context_subject.as_deref(),
            &active_predicates,
            &base_compositional_analysis,
        );
        let mut illocutionary_commitments = infer_illocutionary_commitments(
            illocutionary_surface,
            context,
            &clauses,
            &base_compositional_analysis,
        );
        reconcile_typed_pragmatic_illocution(
            &mut illocutionary_commitments,
            &pragmatic_intent_graph,
            illocutionary_surface,
            &clauses,
        );
        let phenotype = if text.chars().any(|character| {
            ('\u{ac00}'..='\u{d7a3}').contains(&character)
                || ('\u{3131}'..='\u{318e}').contains(&character)
        }) {
            LanguageCodeIR::Korean
        } else if text
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        {
            LanguageCodeIR::English
        } else {
            LanguageCodeIR::Unknown
        };
        let language_center = LanguageCenterPipeline.build(LanguageCenterSources {
            phenotype,
            native: None,
            composition: &base_compositional_analysis,
            pragmatic_intent: &pragmatic_intent_graph,
            illocution: &illocutionary_commitments,
        });
        let user_feedback = detect_user_feedback(text, &clauses);
        let explicit_task = infer_current_task(&clauses);
        let inferred_current_task = explicit_task
            .or_else(|| direct_context_task(context.current_task.as_deref()))
            .or_else(|| embedded_context_task(context.active_subject.as_deref()));

        let has_continuation = clauses
            .iter()
            .any(|clause| clause.roles.contains(&DiscourseClauseRoleIR::CurrentAction));
        let has_condition = clauses
            .iter()
            .any(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Condition));
        let has_evaluation = clauses
            .iter()
            .any(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Evaluation));
        let has_decision = clauses
            .iter()
            .any(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Decision));
        let has_required_condition = clauses.iter().any(|clause| {
            contains_any(
                &normalize(&clause.surface_text),
                &[
                    "어야",
                    "아야",
                    "해야",
                    "때만",
                    "경우에만",
                    "only if",
                    "only when",
                    "required before",
                ],
            )
        });
        if expected_benefits.is_empty()
            && has_continuation
            && (has_condition || has_evaluation || has_decision)
        {
            if let Some(prior_benefit) = context.pending_required_benefit.as_deref() {
                expected_benefits.push(prior_benefit.to_string());
            }
        }
        let negative_override = has_negative_continuation_override(&clauses);
        let gate_candidate = !negative_override
            && !nonliteral_analysis.has_sarcasm()
            && !context.pending_gate_suspended
            && has_continuation
            && (has_condition || context.pending_required_benefit.is_some())
            && !expected_benefits.is_empty()
            && (has_evaluation
                || has_decision
                || has_required_condition
                || !accepted_costs.is_empty());

        let mut unresolved_bindings = Vec::new();
        let continuation_gate = if gate_candidate {
            match (
                inferred_current_task.clone(),
                expected_benefits.first().cloned(),
            ) {
                (Some(current_task), Some(required_benefit)) => {
                    let supporting_clause_ids = clauses
                        .iter()
                        .filter(|clause| {
                            clause.roles.iter().any(|role| {
                                matches!(
                                    role,
                                    DiscourseClauseRoleIR::CurrentAction
                                        | DiscourseClauseRoleIR::Cost
                                        | DiscourseClauseRoleIR::Benefit
                                        | DiscourseClauseRoleIR::Evaluation
                                        | DiscourseClauseRoleIR::Decision
                                )
                            })
                        })
                        .map(|clause| clause.clause_id.clone())
                        .collect();
                    Some(ContinuationDecisionGateIR {
                        current_task,
                        required_benefit,
                        verification_required: true,
                        positive_action: DecisionBranchActionIR::ContinueCurrentWork,
                        negative_action: explicit_negative_action(&clauses),
                        unknown_action: DecisionBranchActionIR::ReportUncertaintyAndAskHowToProceed,
                        supporting_clause_ids,
                        confidence_millis: if context.current_task.is_some() {
                            920
                        } else {
                            870
                        },
                    })
                }
                (None, _) => {
                    unresolved_bindings.push("CURRENT_TASK".to_string());
                    None
                }
                (_, None) => {
                    unresolved_bindings.push("REQUIRED_BENEFIT".to_string());
                    None
                }
            }
        } else {
            None
        };
        let (language_center_goal_projection, compositional_analysis) =
            materialize_language_center_goal_projection(
                text,
                &base_compositional_analysis,
                &language_center,
                &pragmatic_intent_graph,
                &illocutionary_commitments,
                continuation_gate.is_some(),
            );
        if compositional_analysis.clarification_required {
            unresolved_bindings.push("COMPOSITIONAL_INTENT_COMPETITION".to_string());
        }
        if pragmatic_intent_graph
            .utterance_intent
            .requires_clarification()
        {
            unresolved_bindings.push("PRIOR_DISCOURSE_CONTEXT".to_string());
        }
        unresolved_bindings.sort();
        unresolved_bindings.dedup();
        let epistemic_record_update = requests_epistemic_record_update(text);
        let attributed_problem_disclosure = (!compositional_analysis
            .attribution_graph
            .attributions
            .is_empty()
            || epistemic_record_update)
            && pragmatic_intent_graph
                .selected_utterance_intent()
                .is_some_and(|intent| {
                    intent.communicative_intent == CommunicativeIntentIR::ProblemDisclosure
                });
        let unacceptable_problem_repair = (!attributed_problem_disclosure)
            .then(|| infer_unacceptable_problem_repair_goal(&clauses))
            .flatten();

        let explicit_selected_request = compositional_analysis
            .selected_candidates()
            .iter()
            .any(|candidate| candidate.external_execution_authorized)
            || pragmatic_intent_graph
                .composition
                .as_ref()
                .is_some_and(|composition| composition.has_selected_unconditioned_request());
        let utterance_response_goal = pragmatic_intent_graph
            .selected_utterance_intent()
            .filter(|intent| {
                !attributed_problem_disclosure
                    && (!explicit_selected_request
                        || matches!(
                            intent.communicative_intent,
                            CommunicativeIntentIR::ResponseGoalCorrection
                                | CommunicativeIntentIR::SummaryRequest
                                | CommunicativeIntentIR::AssessmentRequest
                                | CommunicativeIntentIR::ConditionalDecisionRequest
                        ))
            })
            .and_then(|intent| {
                intent
                    .plan_intent()
                    .map(|plan_intent| InferredPragmaticGoalIR {
                        intent: plan_intent,
                        subject: intent.target.clone(),
                        desired_outcome: expected_response_outcome(
                            intent.expected_response,
                            &intent.target,
                        ),
                        commitment: GoalCommitmentIR::ImplicitRequest,
                        external_execution_authorized: false,
                        basis_clause_ids: clauses
                            .iter()
                            .map(|clause| clause.clause_id.clone())
                            .collect(),
                        confidence_millis: intent.score_millis,
                    })
            });
        let utterance_goal_has_override_priority = pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|intent| {
                matches!(
                    intent.communicative_intent,
                    CommunicativeIntentIR::ResponseGoalCorrection
                        | CommunicativeIntentIR::SummaryRequest
                        | CommunicativeIntentIR::AssessmentRequest
                        | CommunicativeIntentIR::ConditionalDecisionRequest
                )
            });

        let inferred_goal = if pragmatic_intent_graph
            .utterance_intent
            .requires_clarification()
        {
            None
        } else if let Some(gate) = &continuation_gate {
            Some(InferredPragmaticGoalIR {
                intent: PlanIntentIR::Investigate,
                subject: format!(
                    "continuation_gate(task={}; required_benefit={})",
                    gate.current_task, gate.required_benefit
                ),
                desired_outcome: format!(
                    "determine whether {} produces {} and select the corresponding continuation branch",
                    gate.current_task, gate.required_benefit
                ),
                commitment: GoalCommitmentIR::ConditionalAuthorization,
                external_execution_authorized: false,
                basis_clause_ids: gate.supporting_clause_ids.clone(),
                confidence_millis: gate.confidence_millis,
            })
        } else if utterance_goal_has_override_priority && utterance_response_goal.is_some() {
            utterance_response_goal
        } else if let Some(goal) = unacceptable_problem_repair.clone() {
            Some(goal)
        } else if utterance_response_goal.is_some() {
            utterance_response_goal
        } else if pragmatic_intent_graph.suppresses_goal_projection()
            || illocutionary_commitments.blocks_current_goal_projection()
            || negative_override
            || compositional_analysis.clarification_required
        {
            None
        } else if let Some(candidate) = compositional_analysis.selected_candidate().filter(|_| {
            pragmatic_intent_graph
                .primary
                .as_ref()
                .is_some_and(|intent| intent.projection != PragmaticGoalProjectionIR::Suppressed)
                || illocutionary_commitments.primary_force()
                    == Some(IllocutionaryForceIR::IndirectActionRequest)
                || !compositional_analysis
                    .modal_scope_graph
                    .blocks_goal_projection()
        }) {
            Some(InferredPragmaticGoalIR {
                intent: candidate.intent,
                subject: candidate.subject.clone(),
                desired_outcome: candidate.desired_outcome.clone(),
                commitment: match pragmatic_intent_graph.primary_kind() {
                    Some(PragmaticIntentKindIR::AdvisorySuggestion) => GoalCommitmentIR::Suggestion,
                    Some(
                        PragmaticIntentKindIR::ConventionalIndirectRequest
                        | PragmaticIntentKindIR::PreferenceRequest
                        | PragmaticIntentKindIR::GoalCorrection,
                    ) => GoalCommitmentIR::ExplicitRequest,
                    _ if candidate.external_execution_authorized => {
                        GoalCommitmentIR::ExplicitRequest
                    }
                    _ => GoalCommitmentIR::ImplicitRequest,
                },
                external_execution_authorized: candidate.external_execution_authorized,
                basis_clause_ids: vec![candidate.source_frame_id.clone()],
                confidence_millis: candidate.score_millis,
            })
        } else if !compositional_analysis
            .attribution_graph
            .attributions
            .is_empty()
            || compositional_analysis
                .modal_scope_graph
                .blocks_goal_projection()
        {
            None
        } else {
            infer_non_gate_goal(&clauses)
        };
        let utterance_speech_act = pragmatic_intent_graph
            .selected_utterance_intent()
            .filter(|_| !attributed_problem_disclosure)
            .map(|intent| match intent.communicative_intent {
                CommunicativeIntentIR::ConditionalDecisionRequest => {
                    SpeechActIR::ConditionalContinuation
                }
                CommunicativeIntentIR::ProblemDisclosure
                | CommunicativeIntentIR::AssessmentRequest
                | CommunicativeIntentIR::EvidenceRequest
                | CommunicativeIntentIR::RecommendationRequest
                | CommunicativeIntentIR::ResponseGoalCorrection
                | CommunicativeIntentIR::ExplanationRequest
                | CommunicativeIntentIR::SummaryRequest => SpeechActIR::Ask,
            });
        let speech_act = if continuation_gate.is_some() {
            SpeechActIR::ConditionalContinuation
        } else if explicit_selected_request {
            SpeechActIR::RequestAction
        } else if utterance_goal_has_override_priority {
            utterance_speech_act.unwrap_or(SpeechActIR::Ask)
        } else if inferred_goal.as_ref().is_some_and(|goal| {
            goal.commitment == GoalCommitmentIR::ImplicitRequest
                && goal.intent == PlanIntentIR::Repair
        }) {
            SpeechActIR::RequestAction
        } else if let Some(speech_act) = utterance_speech_act {
            speech_act
        } else if let Some(kind) = pragmatic_intent_graph.primary_kind() {
            match kind {
                PragmaticIntentKindIR::ConventionalIndirectRequest
                | PragmaticIntentKindIR::PreferenceRequest
                | PragmaticIntentKindIR::GoalCorrection => SpeechActIR::RequestAction,
                PragmaticIntentKindIR::AdvisorySuggestion => SpeechActIR::Suggest,
                PragmaticIntentKindIR::RhetoricalEvaluation => SpeechActIR::NegativeEvaluation,
                PragmaticIntentKindIR::InformationQuestion
                | PragmaticIntentKindIR::MetalinguisticMention => SpeechActIR::Ask,
                PragmaticIntentKindIR::SelfOffer => SpeechActIR::Inform,
            }
        } else if let Some(force) = illocutionary_commitments.primary_force() {
            match force {
                IllocutionaryForceIR::SelfCommitment
                | IllocutionaryForceIR::ReportedCommitment
                | IllocutionaryForceIR::OutcomeClaimConstraint => SpeechActIR::Inform,
                IllocutionaryForceIR::CapabilityQuestion
                | IllocutionaryForceIR::AnswerOnlyInformationRequest => SpeechActIR::Ask,
                IllocutionaryForceIR::IndirectActionRequest
                | IllocutionaryForceIR::DeferredConditionalRequest => SpeechActIR::RequestAction,
                IllocutionaryForceIR::GoalWithdrawal => SpeechActIR::Reject,
            }
        } else if nonliteral_analysis.has_sarcasm() {
            SpeechActIR::NegativeEvaluation
        } else if negative_override {
            SpeechActIR::Reject
        } else if user_feedback.is_some() && !explicit_selected_request {
            SpeechActIR::NegativeEvaluation
        } else if let Some(goal) = &inferred_goal {
            match (goal.commitment, goal.intent) {
                (GoalCommitmentIR::Suggestion, _) => SpeechActIR::Suggest,
                (GoalCommitmentIR::ImplicitRequest, PlanIntentIR::Investigate) => SpeechActIR::Ask,
                _ => SpeechActIR::RequestAction,
            }
        } else if has_condition && has_decision {
            SpeechActIR::ConditionalCommitment
        } else if has_approval(&clauses) {
            SpeechActIR::Approve
        } else if epistemic_record_update
            || !compositional_analysis
                .attribution_graph
                .attributions
                .is_empty()
        {
            SpeechActIR::Inform
        } else if has_request(&clauses) {
            SpeechActIR::RequestAction
        } else {
            SpeechActIR::Inform
        };

        let mut semantic_tags = BTreeSet::new();
        for clause in &clauses {
            semantic_tags.extend(clause.semantic_cues.iter().cloned());
        }
        if continuation_gate.is_some() {
            semantic_tags.extend([
                "conditional_continuation".to_string(),
                "continuation_decision_gate".to_string(),
                "benefit_cost_reasoning".to_string(),
            ]);
        }
        if let Some(intent) = pragmatic_intent_graph.primary.as_ref() {
            semantic_tags.insert(format!("pragmatic_intent_{:?}", intent.kind).to_lowercase());
            semantic_tags.insert(
                format!("pragmatic_goal_projection_{:?}", intent.projection).to_lowercase(),
            );
            semantic_tags.insert("pragmatic_intent_non_semantic_authority".to_string());
        }
        for intent in pragmatic_intent_graph.utterance_intent.active() {
            semantic_tags.insert(
                format!("communicative_intent_{:?}", intent.communicative_intent).to_lowercase(),
            );
            semantic_tags
                .insert(format!("expected_response_{:?}", intent.expected_response).to_lowercase());
            semantic_tags.insert("utterance_intent_non_semantic_authority".to_string());
        }
        for commitment in &illocutionary_commitments.commitments {
            semantic_tags
                .insert(format!("illocutionary_force_{:?}", commitment.force).to_lowercase());
            semantic_tags.insert(format!("commitment_actor_{:?}", commitment.actor).to_lowercase());
            semantic_tags.insert(
                format!("commitment_activation_{:?}", commitment.activation).to_lowercase(),
            );
        }
        if nonliteral_analysis.literal_execution_blocked {
            semantic_tags.insert("nonliteral_literal_execution_blocked".to_string());
        }
        if nonliteral_analysis.semantic_incongruity_detected {
            semantic_tags.insert("semantic_incongruity".to_string());
        }
        if !compositional_analysis.frames.is_empty() {
            semantic_tags.insert("compositional_semantic_frames".to_string());
        }
        if compositional_analysis.goal_graph.is_some() {
            semantic_tags.insert("compositional_goal_graph".to_string());
        }
        if compositional_analysis.blocked_execution_count() > 0 {
            semantic_tags.insert("scope_bounded_execution_authority".to_string());
        }
        if !compositional_analysis
            .modal_scope_graph
            .operators
            .is_empty()
            || !compositional_analysis
                .modal_scope_graph
                .conditionals
                .is_empty()
        {
            semantic_tags.insert("modal_scope_graph".to_string());
            semantic_tags.insert(
                format!(
                    "modal_world_{:?}",
                    compositional_analysis.modal_scope_graph.root_world
                )
                .to_lowercase(),
            );
            if compositional_analysis.modal_scope_graph.root_world != ModalWorldIR::Actual {
                semantic_tags.insert("nonactual_content_not_asserted_as_fact".to_string());
            }
            if compositional_analysis.modal_scope_graph.illocution
                == ModalIllocutionIR::PoliteRequest
            {
                semantic_tags.insert("indirect_polite_request".to_string());
            }
        }
        if !compositional_analysis
            .attribution_graph
            .attributions
            .is_empty()
        {
            semantic_tags.insert("attribution_graph".to_string());
            semantic_tags.insert("attributed_truth_not_established".to_string());
            if compositional_analysis
                .attribution_graph
                .attributions
                .iter()
                .any(|edge| edge.parent_proposition_id.is_some())
            {
                semantic_tags.insert("nested_attribution".to_string());
            }
        }
        if compositional_analysis
            .scopes
            .iter()
            .any(|scope| scope.kind == ScopeKindIR::Quotation)
        {
            semantic_tags.insert("quoted_predicate_non_authoritative".to_string());
        }
        if compositional_analysis
            .candidates
            .iter()
            .any(|candidate| candidate.disposition == CandidateDispositionIR::HypotheticalOnly)
        {
            semantic_tags.insert("hypothetical_predicate_non_authoritative".to_string());
        }
        let structural_evidence = usize::from(has_continuation)
            + usize::from(has_condition)
            + usize::from(has_evaluation || has_decision)
            + usize::from(!accepted_costs.is_empty())
            + usize::from(!expected_benefits.is_empty())
            + usize::from(inferred_current_task.is_some());
        let confidence_millis = if continuation_gate.is_some() {
            (700 + structural_evidence * 40).min(940) as u16
        } else {
            (450 + structural_evidence * 35).min(780) as u16
        };

        PragmaticInterpretationIR {
            schema: PRAGMATIC_INTERPRETATION_SCHEMA.to_string(),
            speech_act,
            clauses,
            relations,
            inferred_current_task,
            accepted_costs,
            expected_benefits,
            evidence_policy,
            nonliteral_analysis,
            language_center,
            language_center_goal_projection: Some(language_center_goal_projection),
            compositional_analysis,
            pragmatic_intent_graph,
            illocutionary_commitments,
            inferred_goal,
            user_feedback,
            continuation_gate,
            unresolved_bindings,
            semantic_tags: semantic_tags.into_iter().collect(),
            confidence_millis,
        }
    }
}

pub(crate) fn requests_epistemic_record_update(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    text.starts_with("add that ")
        || text.starts_with("record that ")
        || text.starts_with("remember that ")
        || ((text.contains("기록") || text.contains("내용"))
            && contains_any(
                &text,
                &[
                    "추가해 둠",
                    "추가해둠",
                    "추가해 둬",
                    "추가해둬",
                    "기록해 둠",
                    "기록해둠",
                    "기록해 둬",
                    "기록해둬",
                ],
            ))
}

pub(crate) fn requests_future_epistemic_notification(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    let english_notification = contains_any(
        &text,
        &["tell me when", "let me know when", "notify me when"],
    ) && contains_any(
        &text,
        &["verified", "confirmed", "proven", "evidence", "result"],
    );
    let korean_notification = contains_any(&text, &["알려", "통지"])
        && (contains_any(&text, &["검증되면", "확인되면", "입증되면"])
            || (contains_any(&text, &["검증된", "확인된", "입증된", "결과", "근거"])
                && contains_any(&text, &["생기면", "나오면", "확실해지면", "확보되면"])));
    english_notification || korean_notification
}

fn suppress_immediate_continuation_action(analysis: &mut CompositionalAnalysisIR) {
    let frame_ids = analysis
        .frames
        .iter_mut()
        .filter(|frame| frame.canonical_predicate == "CONTINUE")
        .map(|frame| {
            frame.external_execution_authorized = false;
            frame.frame_id.clone()
        })
        .collect::<BTreeSet<_>>();
    if frame_ids.is_empty() {
        return;
    }
    let candidate_ids = analysis
        .candidates
        .iter_mut()
        .filter(|candidate| frame_ids.contains(&candidate.source_frame_id))
        .map(|candidate| {
            candidate.external_execution_authorized = false;
            candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
            candidate
                .blockers
                .push("CONDITIONAL_CONTINUATION_REQUIRES_GATE_EVIDENCE".to_string());
            candidate.candidate_id.clone()
        })
        .collect::<BTreeSet<_>>();
    analysis
        .selected_candidate_ids
        .retain(|candidate_id| !candidate_ids.contains(candidate_id));
    if analysis
        .selected_candidate_id
        .as_ref()
        .is_some_and(|candidate_id| candidate_ids.contains(candidate_id))
    {
        analysis.selected_candidate_id = analysis.selected_candidate_ids.first().cloned();
    }
    if let Some(goal_graph) = &mut analysis.goal_graph {
        goal_graph
            .nodes
            .retain(|node| !candidate_ids.contains(&node.candidate_id));
        let retained = goal_graph
            .nodes
            .iter()
            .map(|node| node.node_id.clone())
            .collect::<BTreeSet<_>>();
        goal_graph.edges.retain(|edge| {
            retained.contains(&edge.source_node_id) && retained.contains(&edge.target_node_id)
        });
        if goal_graph.nodes.is_empty() {
            analysis.goal_graph = None;
        }
    }
}

fn reconcile_typed_pragmatic_illocution(
    graph: &mut IllocutionaryCommitmentGraphIR,
    pragmatic: &PragmaticIntentGraphIR,
    surface: &str,
    clauses: &[DiscourseClauseIR],
) {
    let selected_conditional = pragmatic
        .composition
        .as_ref()
        .is_some_and(|composition| composition.has_selected_conditional_request());
    let selected_immediate = pragmatic
        .composition
        .as_ref()
        .is_some_and(|composition| composition.has_selected_immediate_request());
    if selected_conditional {
        let deferred = IllocutionaryCommitmentIR {
            commitment_id: if selected_immediate {
                "ILLOCUTION-02"
            } else {
                "ILLOCUTION-01"
            }
            .to_string(),
            actor: DialogueParticipantIR::User,
            addressee: DialogueParticipantIR::Assistant,
            force: IllocutionaryForceIR::DeferredConditionalRequest,
            activation: CommitmentActivationIR::ConditionPending,
            proposition_surface: surface.trim().to_string(),
            external_execution_authorized: false,
            evidence: vec!["COMPOSITIONAL_PRAGMATIC_CONDITIONAL_REQUEST".to_string()]
                .into_iter()
                .chain(clauses.first().map(|clause| clause.clause_id.clone()))
                .collect(),
        };
        if selected_immediate {
            graph.commitments = vec![
                IllocutionaryCommitmentIR {
                    commitment_id: "ILLOCUTION-01".to_string(),
                    actor: DialogueParticipantIR::User,
                    addressee: DialogueParticipantIR::Assistant,
                    force: IllocutionaryForceIR::IndirectActionRequest,
                    activation: CommitmentActivationIR::Immediate,
                    proposition_surface: surface.trim().to_string(),
                    external_execution_authorized: true,
                    evidence: vec!["COMPOSITIONAL_PRAGMATIC_IMMEDIATE_REQUEST".to_string()]
                        .into_iter()
                        .chain(clauses.first().map(|clause| clause.clause_id.clone()))
                        .collect(),
                },
                deferred,
            ];
        } else {
            graph.commitments = vec![deferred];
        }
        return;
    }
    if selected_immediate
        && pragmatic.primary_kind().is_none()
        && graph.primary_force() != Some(IllocutionaryForceIR::AnswerOnlyInformationRequest)
    {
        graph.commitments = vec![IllocutionaryCommitmentIR {
            commitment_id: "ILLOCUTION-01".to_string(),
            actor: DialogueParticipantIR::User,
            addressee: DialogueParticipantIR::Assistant,
            force: IllocutionaryForceIR::IndirectActionRequest,
            activation: CommitmentActivationIR::Immediate,
            proposition_surface: surface.trim().to_string(),
            external_execution_authorized: true,
            evidence: vec!["COMPOSITIONAL_PRAGMATIC_IMMEDIATE_REQUEST".to_string()]
                .into_iter()
                .chain(clauses.first().map(|clause| clause.clause_id.clone()))
                .collect(),
        }];
        return;
    }
    let Some(kind) = pragmatic.primary_kind() else {
        return;
    };
    let (force, actor, addressee, activation, authorized, evidence) = match kind {
        PragmaticIntentKindIR::ConventionalIndirectRequest
        | PragmaticIntentKindIR::PreferenceRequest => (
            IllocutionaryForceIR::IndirectActionRequest,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Immediate,
            true,
            "TYPED_PRAGMATIC_ADDRESSEE_REQUEST",
        ),
        PragmaticIntentKindIR::SelfOffer => (
            IllocutionaryForceIR::SelfCommitment,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Inactive,
            false,
            "TYPED_PRAGMATIC_SELF_OFFER",
        ),
        PragmaticIntentKindIR::AdvisorySuggestion
        | PragmaticIntentKindIR::RhetoricalEvaluation
        | PragmaticIntentKindIR::InformationQuestion
        | PragmaticIntentKindIR::MetalinguisticMention
        | PragmaticIntentKindIR::GoalCorrection => {
            graph.commitments.clear();
            return;
        }
    };
    graph.commitments = vec![IllocutionaryCommitmentIR {
        commitment_id: "ILLOCUTION-01".to_string(),
        actor,
        addressee,
        force,
        activation,
        proposition_surface: surface.trim().to_string(),
        external_execution_authorized: authorized,
        evidence: vec![evidence.to_string()]
            .into_iter()
            .chain(clauses.first().map(|clause| clause.clause_id.clone()))
            .collect(),
    }];
}

fn infer_illocutionary_commitments(
    surface: &str,
    context: &PragmaticContextIR,
    clauses: &[DiscourseClauseIR],
    analysis: &CompositionalAnalysisIR,
) -> IllocutionaryCommitmentGraphIR {
    let text = normalize(surface);
    let question = surface.contains('?')
        || contains_any(&text, &["수 있어", "가능해", "can ", "could ", "would "]);
    let system_subject = contains_any(
        &text,
        &[
            "시스템이",
            "시스템은",
            "도구가",
            "도구는",
            "코어가",
            "코어는",
            "b_core",
            "b-core",
            "the system",
            "this system",
            "the tool",
            "this tool",
        ],
    );
    let indirect_request = question
        && (contains_any(&text, &["줄 수 있어", "해줄 수", "봐줄 수", "for me"])
            || contains_any(
                &text,
                &["can you ", "could you ", "would you ", "will you "],
            )
            || (text.contains("수 있어") && !system_subject));
    let capability_question = question && system_subject && !indirect_request;
    let reported_commitment = (contains_any(
        &text,
        &[
            "겠다고 했",
            "겠다고 말",
            "기로 했다고",
            "한다고 했",
            " says ",
            " said ",
            " reported ",
        ],
    ) || !analysis.attribution_graph.attributions.is_empty())
        && contains_any(&text, &["겠", "할 거", "will ", "would ", "going to"]);
    let self_commitment = !reported_commitment
        && !contains_first_person_revision_preface(&text)
        && ((contains_any(&text, &["내가 ", "제가 ", "나는 ", "난 ", "제가"])
            && contains_any(
                &text,
                &["할게", "둘게", "볼게", "갈게", "겠어", "하겠다", "할 거"],
            ))
            || contains_any(
                &text,
                &[
                    "i will ",
                    "i'll ",
                    "i shall ",
                    "i am going to ",
                    "i'm going to ",
                ],
            ));
    let outcome_claim_policy = detect_outcome_claim_policy(&text);
    let answer_only = detect_answer_only_request(&text);
    let withdrawal = (!context.active_goals.is_empty()
        || !context.pending_deferred_commitments.is_empty())
    .then(|| detect_goal_withdrawal(&text))
    .flatten();
    let has_requested_action = analysis.frames.iter().any(|frame| {
        matches!(
            frame.mood,
            crate::compositional_semantics::FrameMoodIR::Imperative
        ) || frame.external_execution_authorized
    });
    let deferred = outcome_claim_policy.is_none()
        && !answer_only
        && has_requested_action
        && ((analysis.modal_scope_graph.illocution == ModalIllocutionIR::ConditionalDirective
            && analysis
                .modal_scope_graph
                .conditionals
                .iter()
                .any(|conditional| conditional.consequent_is_directive))
            || contains_any(
                &text,
                &[
                    "경우에만",
                    "때에만",
                    "뒤에만",
                    "후에만",
                    "끝난 뒤",
                    "승인하면",
                    "허락하면",
                    "only if",
                    "only after",
                    " once ",
                    " after ",
                    " when ",
                ],
            ));

    let (force, actor, addressee, activation, authorized, evidence) = if withdrawal.is_some() {
        (
            IllocutionaryForceIR::GoalWithdrawal,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Immediate,
            false,
            "ACTIVE_GOAL_REVISION",
        )
    } else if outcome_claim_policy.is_some() {
        (
            IllocutionaryForceIR::OutcomeClaimConstraint,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Immediate,
            false,
            "EVIDENCE_GATED_RESULT_REPORTING",
        )
    } else if answer_only {
        (
            IllocutionaryForceIR::AnswerOnlyInformationRequest,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Immediate,
            false,
            "INFORMATION_REQUEST_WITH_ACTION_PROHIBITION",
        )
    } else if deferred {
        (
            IllocutionaryForceIR::DeferredConditionalRequest,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::ConditionPending,
            false,
            "UNSATISFIED_ANTECEDENT",
        )
    } else if reported_commitment {
        (
            IllocutionaryForceIR::ReportedCommitment,
            DialogueParticipantIR::ThirdParty,
            DialogueParticipantIR::Unknown,
            CommitmentActivationIR::Inactive,
            false,
            "ATTRIBUTED_FUTURE_COMMITMENT",
        )
    } else if self_commitment {
        (
            IllocutionaryForceIR::SelfCommitment,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Inactive,
            false,
            "FIRST_PERSON_FUTURE_COMMITMENT",
        )
    } else if indirect_request {
        (
            IllocutionaryForceIR::IndirectActionRequest,
            DialogueParticipantIR::User,
            DialogueParticipantIR::Assistant,
            CommitmentActivationIR::Immediate,
            true,
            "ADDRESSEE_CAPABILITY_FORM_WITH_BENEFICIARY",
        )
    } else if capability_question {
        (
            IllocutionaryForceIR::CapabilityQuestion,
            DialogueParticipantIR::User,
            DialogueParticipantIR::System,
            CommitmentActivationIR::Inactive,
            false,
            "SYSTEM_CAPABILITY_PREDICATION",
        )
    } else {
        return IllocutionaryCommitmentGraphIR::default();
    };

    IllocutionaryCommitmentGraphIR {
        commitments: vec![IllocutionaryCommitmentIR {
            commitment_id: "ILLOCUTION-01".to_string(),
            actor,
            addressee,
            force,
            activation,
            proposition_surface: surface.trim().to_string(),
            external_execution_authorized: authorized,
            evidence: vec![evidence.to_string()]
                .into_iter()
                .chain(clauses.first().map(|clause| clause.clause_id.clone()))
                .collect(),
        }],
        goal_withdrawal: withdrawal,
        outcome_claim_policy,
    }
}

fn contains_first_person_revision_preface(text: &str) -> bool {
    let Some((preface, replacement)) = text.split_once([':', '：']) else {
        return false;
    };
    if replacement.trim().is_empty() {
        return false;
    }
    let preface = preface.trim();
    let first_person = [
        "let me ",
        "i will ",
        "i'll ",
        "i want to ",
        "i need to ",
        "내가 ",
        "제가 ",
    ]
    .iter()
    .any(|marker| preface.contains(marker));
    let revision = [
        "correct that",
        "correct this",
        "correct myself",
        "rephrase that",
        "정정",
        "표현을 고치",
        "말을 고치",
    ]
    .iter()
    .any(|marker| preface.contains(marker));
    first_person && revision
}

fn detect_answer_only_request(text: &str) -> bool {
    let requests_information = contains_any(
        text,
        &[
            "알려",
            "설명",
            "확인해",
            "방법만",
            "tell me",
            "explain",
            "describe",
            "whether",
            "how to",
            "평가",
            "assessment",
        ],
    );
    let prohibits_action = contains_any(
        text,
        &[
            "하지 마",
            "하지는 마",
            "지우지 마",
            "삭제하지 마",
            "배포하지 마",
            "말고",
            "do not ",
            "don't ",
            "without deleting",
            "without deploying",
            "without publishing",
            "평가만 보고",
            "보고만",
            "답만",
            "only report",
            "report only",
            "only tell",
            "answer only",
        ],
    );
    requests_information && prohibits_action
}

fn detect_outcome_claim_policy(text: &str) -> Option<OutcomeClaimPolicyIR> {
    let reporting = contains_any(
        text,
        &[
            "말하지",
            "답하지",
            "쓰지",
            "단정하지",
            "보고하지",
            "say ",
            "claim ",
            "report ",
            "write ",
        ],
    );
    let outcome = contains_any(
        text,
        &[
            "고쳤",
            "완료",
            "끝났",
            "성공",
            "실행했다고",
            "fixed",
            "completed",
            "completion",
            "succeeded",
            "success",
            " ran ",
            "executed",
        ],
    );
    let evidence = contains_any(
        text,
        &[
            "확인",
            "검증",
            "기록",
            "증거",
            "영수증",
            "verified",
            "verification",
            "record",
            "evidence",
            "receipt",
        ],
    );
    let constraint = contains_any(
        text,
        &[
            "하지 마",
            "하지마",
            "전에는",
            "없으면",
            "없이",
            "do not",
            "don't",
            "never",
            "unless",
            "without",
            "before",
            "until",
        ],
    );
    if !(reporting && outcome && evidence && constraint) {
        return None;
    }
    let mut required_evidence = BTreeSet::new();
    if contains_any(text, &["확인", "검증", "verified", "verification"]) {
        required_evidence.insert(RequiredOutcomeEvidenceIR::DirectVerification);
    }
    if contains_any(text, &["기록", "record"]) {
        required_evidence.insert(RequiredOutcomeEvidenceIR::ExecutionRecord);
    }
    if contains_any(text, &["영수증", "receipt"]) {
        required_evidence.insert(RequiredOutcomeEvidenceIR::Receipt);
    }
    if contains_any(text, &["증거", "evidence"]) {
        required_evidence.insert(RequiredOutcomeEvidenceIR::ObservableEvidence);
    }
    if required_evidence.is_empty() {
        required_evidence.insert(RequiredOutcomeEvidenceIR::DirectVerification);
    }
    Some(OutcomeClaimPolicyIR {
        policy: "VERIFIED_OUTCOME_ONLY".to_string(),
        verified_outcome_only: true,
        required_evidence: required_evidence.into_iter().collect(),
        evidence_surface: text.to_string(),
    })
}

fn detect_goal_withdrawal(text: &str) -> Option<GoalWithdrawalIR> {
    let trimmed = text.trim().trim_end_matches(['.', '!', '?']).trim();
    let standalone_done =
        trimmed == "됐어" || trimmed.starts_with("됐어,") || trimmed.starts_with("됐어 ");
    let marker = standalone_done
        || contains_any(
            text,
            &[
                "취소",
                "철회",
                "그만",
                "그 작업은 하지 마",
                "그 일은 하지 마",
                "never mind",
                "cancel",
                "withdraw",
                "forget that request",
                "don't proceed",
                "do not proceed",
                "don't do that task",
                "do not do that task",
                "don't do the action",
                "do not do the action",
            ],
        );
    if !marker {
        return None;
    }
    let ordinal = if contains_any(text, &["첫 번째", "첫째", "first action", "first task"]) {
        Some(1)
    } else if contains_any(text, &["두 번째", "둘째", "second action", "second task"]) {
        Some(2)
    } else if contains_any(text, &["세 번째", "셋째", "third action", "third task"]) {
        Some(3)
    } else {
        None
    };
    Some(GoalWithdrawalIR {
        scope: if ordinal.is_some() {
            GoalWithdrawalScopeIR::EventOrdinal
        } else {
            GoalWithdrawalScopeIR::AllActiveGoals
        },
        event_ordinal: ordinal,
        evidence_surface: text.to_string(),
    })
}

/// The only production boundary allowed to materialize a GoalIR-facing
/// compositional analysis. Every upstream module is borrowed immutably; its
/// proposal is recorded before the central compatibility reducer runs once.
fn materialize_language_center_goal_projection(
    text: &str,
    base: &CompositionalAnalysisIR,
    center: &LanguageCenterIR,
    pragmatic_intent: &PragmaticIntentGraphIR,
    illocution: &IllocutionaryCommitmentGraphIR,
    continuation_gate_present: bool,
) -> (LanguageCenterGoalProjectionIR, CompositionalAnalysisIR) {
    let decisions = collect_language_center_goal_decisions(
        center,
        pragmatic_intent,
        illocution,
        continuation_gate_present,
    );

    // This owned clone is the materializer's output under construction. No
    // source module receives it, so source graphs cannot overwrite one another.
    let mut materialized = base.clone();
    apply_illocutionary_authority(illocution, &mut materialized);
    PragmaticIntentAnalyzer.apply_to_compositional_analysis(
        pragmatic_intent,
        text,
        &mut materialized,
    );
    PragmaticIntentAnalyzer.apply_utterance_intent_to_compositional_analysis(
        pragmatic_intent,
        text,
        &mut materialized,
    );
    if illocution
        .commitments
        .iter()
        .any(|commitment| commitment.force == IllocutionaryForceIR::DeferredConditionalRequest)
    {
        suppress_deferred_conditional_candidates(&mut materialized);
    }
    if continuation_gate_present {
        suppress_immediate_continuation_action(&mut materialized);
    }

    let projection = LanguageCenterGoalProjectionIR::seal(
        center,
        base,
        pragmatic_intent,
        illocution,
        &materialized,
        decisions,
    );
    debug_assert!(projection.validate_against(center, &materialized));
    (projection, materialized)
}

fn collect_language_center_goal_decisions(
    center: &LanguageCenterIR,
    pragmatic_intent: &PragmaticIntentGraphIR,
    illocution: &IllocutionaryCommitmentGraphIR,
    continuation_gate_present: bool,
) -> Vec<LanguageCenterGoalDecisionIR> {
    let mut decisions = Vec::new();
    for event in &center.events {
        let (effect, precedence) = match event.projection {
            LanguageCenterProjectionIR::Prohibited => {
                (LanguageCenterGoalEffectIR::BlockGoal, 1_000)
            }
            LanguageCenterProjectionIR::Conditional => (LanguageCenterGoalEffectIR::DeferGoal, 950),
            LanguageCenterProjectionIR::Reported | LanguageCenterProjectionIR::Suppressed => {
                (LanguageCenterGoalEffectIR::SuppressGoal, 900)
            }
            LanguageCenterProjectionIR::LiveRequest => {
                (LanguageCenterGoalEffectIR::SelectLiveGoal, 800)
            }
            LanguageCenterProjectionIR::Unresolved => {
                (LanguageCenterGoalEffectIR::RequireClarification, 990)
            }
            LanguageCenterProjectionIR::Advisory
            | LanguageCenterProjectionIR::Inquiry
            | LanguageCenterProjectionIR::Descriptive => {
                (LanguageCenterGoalEffectIR::RetainNonExecutable, 500)
            }
        };
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::LanguageCenterEvent,
            Some(event.event_id.clone()),
            Some(event.source_frame_id.clone()),
            effect,
            precedence,
            if event.contribution_ids.is_empty() {
                vec![format!("LANGUAGE_CENTER_EVENT:{}", event.event_id)]
            } else {
                event.contribution_ids.clone()
            },
        ));
    }

    if let Some(composition) = &pragmatic_intent.composition {
        for node in &composition.nodes {
            let conditional = composition.context_scopes.iter().any(|scope| {
                scope.target_node_id == node.node_id
                    && scope.kind
                        == crate::pragmatic_intent::PragmaticIntentRelationKindIR::Conditions
            });
            let selected = composition.selected_node_ids.contains(&node.node_id);
            let (effect, precedence) = if conditional {
                (LanguageCenterGoalEffectIR::DeferGoal, 950)
            } else if selected && node.projection == PragmaticGoalProjectionIR::AuthorizedRequest {
                (LanguageCenterGoalEffectIR::SelectLiveGoal, 850)
            } else if node.projection == PragmaticGoalProjectionIR::Suppressed {
                (LanguageCenterGoalEffectIR::SuppressGoal, 920)
            } else {
                (LanguageCenterGoalEffectIR::RetainNonExecutable, 550)
            };
            // Compatibility nodes projected from the native circuit use the
            // native event id as their source id.  That id is deliberately
            // not a compositional frame id, so bind both references only when
            // the Language Center can prove the frame belongs to its event
            // topology.  The native evidence remains on the decision either
            // way; an adapter-local id must never masquerade as a typed frame.
            let center_event = center
                .events
                .iter()
                .find(|event| event.source_frame_id == node.source_frame_id);
            decisions.push(goal_decision(
                LanguageCenterGoalDecisionSourceIR::PragmaticIntentGraph,
                center_event.map(|event| event.event_id.clone()),
                center_event.map(|event| event.source_frame_id.clone()),
                effect,
                precedence,
                vec![format!("PRAGMATIC_NODE:{}", node.node_id)],
            ));
        }
        for unresolved in &composition.unresolved_ambiguities {
            decisions.push(goal_decision(
                LanguageCenterGoalDecisionSourceIR::PragmaticIntentGraph,
                None,
                None,
                LanguageCenterGoalEffectIR::RequireClarification,
                990,
                vec![format!("PRAGMATIC_UNRESOLVED:{unresolved}")],
            ));
        }
    } else if let Some(primary) = &pragmatic_intent.primary {
        let effect = match primary.projection {
            PragmaticGoalProjectionIR::AuthorizedRequest => {
                LanguageCenterGoalEffectIR::SelectLiveGoal
            }
            PragmaticGoalProjectionIR::AdvisoryOnly => {
                LanguageCenterGoalEffectIR::RetainNonExecutable
            }
            PragmaticGoalProjectionIR::Suppressed => LanguageCenterGoalEffectIR::SuppressGoal,
        };
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::PragmaticIntentGraph,
            primary.source_frame_id.as_deref().and_then(|frame_id| {
                center
                    .events
                    .iter()
                    .find(|event| event.source_frame_id == frame_id)
                    .map(|event| event.event_id.clone())
            }),
            primary.source_frame_id.clone(),
            effect,
            850,
            vec![format!("PRAGMATIC_PRIMARY:{:?}", primary.kind)],
        ));
    } else {
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::PragmaticIntentGraph,
            None,
            None,
            LanguageCenterGoalEffectIR::RetainNonExecutable,
            400,
            vec!["PRAGMATIC_INTENT_GRAPH:NO_GOAL_PROPOSAL".to_string()],
        ));
    }

    let active_utterance_intents = pragmatic_intent
        .utterance_intent
        .active()
        .collect::<Vec<_>>();
    if let Some(intent) = active_utterance_intents.first().copied() {
        let (effect, precedence) =
            if intent.expected_response == ExpectedResponseKindIR::Clarification {
                (LanguageCenterGoalEffectIR::RequireClarification, 1_000)
            } else if intent.communicative_intent == CommunicativeIntentIR::ResponseGoalCorrection {
                (LanguageCenterGoalEffectIR::SynthesizeResponseGoal, 980)
            } else {
                (LanguageCenterGoalEffectIR::PreserveConstraint, 700)
            };
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::UtteranceIntentGraph,
            None,
            None,
            effect,
            precedence,
            vec![format!("UTTERANCE_INTENT:{}", intent.candidate_id)],
        ));
        for supporting in active_utterance_intents.iter().skip(1) {
            decisions.push(goal_decision(
                LanguageCenterGoalDecisionSourceIR::UtteranceIntentGraph,
                None,
                None,
                if supporting.expected_response == ExpectedResponseKindIR::Clarification {
                    LanguageCenterGoalEffectIR::RequireClarification
                } else {
                    LanguageCenterGoalEffectIR::PreserveConstraint
                },
                if supporting.expected_response == ExpectedResponseKindIR::Clarification {
                    1_000
                } else {
                    690
                },
                vec![format!(
                    "UTTERANCE_INTENT_SUPPORTING:{}",
                    supporting.candidate_id
                )],
            ));
        }
    } else {
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::UtteranceIntentGraph,
            None,
            None,
            LanguageCenterGoalEffectIR::RetainNonExecutable,
            400,
            vec![format!(
                "UTTERANCE_INTENT_GRAPH:{}",
                pragmatic_intent.utterance_intent.graph_sha256
            )],
        ));
    }

    for commitment in &illocution.commitments {
        let (effect, precedence) = match commitment.force {
            IllocutionaryForceIR::IndirectActionRequest => {
                (LanguageCenterGoalEffectIR::SelectLiveGoal, 875)
            }
            IllocutionaryForceIR::DeferredConditionalRequest => {
                (LanguageCenterGoalEffectIR::DeferGoal, 975)
            }
            IllocutionaryForceIR::GoalWithdrawal
            | IllocutionaryForceIR::OutcomeClaimConstraint
            | IllocutionaryForceIR::ReportedCommitment => {
                (LanguageCenterGoalEffectIR::SuppressGoal, 960)
            }
            IllocutionaryForceIR::CapabilityQuestion | IllocutionaryForceIR::SelfCommitment => {
                (LanguageCenterGoalEffectIR::RetainNonExecutable, 900)
            }
            IllocutionaryForceIR::AnswerOnlyInformationRequest => {
                (LanguageCenterGoalEffectIR::PreserveConstraint, 925)
            }
        };
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::IllocutionaryCommitmentGraph,
            None,
            None,
            effect,
            precedence,
            vec![format!("ILLOCUTION:{}", commitment.commitment_id)],
        ));
    }
    if illocution.commitments.is_empty() {
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::IllocutionaryCommitmentGraph,
            None,
            None,
            LanguageCenterGoalEffectIR::RetainNonExecutable,
            400,
            vec!["ILLOCUTION_GRAPH:NO_COMMITMENT".to_string()],
        ));
    }

    if continuation_gate_present {
        decisions.push(goal_decision(
            LanguageCenterGoalDecisionSourceIR::ContinuationGate,
            None,
            None,
            LanguageCenterGoalEffectIR::DeferGoal,
            1_000,
            vec!["CONTINUATION_GATE:VERIFICATION_REQUIRED".to_string()],
        ));
    }
    decisions.push(goal_decision(
        LanguageCenterGoalDecisionSourceIR::CentralMaterializer,
        None,
        None,
        LanguageCenterGoalEffectIR::MaterializeOnce,
        1_000,
        vec!["CENTRAL_MATERIALIZATION_COUNT=1".to_string()],
    ));

    decisions.sort_by(|left, right| {
        right
            .precedence
            .cmp(&left.precedence)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.event_id.cmp(&right.event_id))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
            .then_with(|| left.evidence_refs.cmp(&right.evidence_refs))
    });
    for (index, decision) in decisions.iter_mut().enumerate() {
        decision.decision_id = format!("LANGUAGE-CENTER-GOAL-DECISION-{:03}", index + 1);
    }
    decisions
}

fn goal_decision(
    source: LanguageCenterGoalDecisionSourceIR,
    event_id: Option<String>,
    frame_id: Option<String>,
    effect: LanguageCenterGoalEffectIR,
    precedence: u16,
    evidence_refs: Vec<String>,
) -> LanguageCenterGoalDecisionIR {
    LanguageCenterGoalDecisionIR {
        decision_id: String::new(),
        source,
        event_id,
        frame_id,
        effect,
        precedence,
        evidence_refs,
        semantic_authority: false,
        external_execution_authorized: false,
    }
}

fn apply_illocutionary_authority(
    graph: &IllocutionaryCommitmentGraphIR,
    analysis: &mut CompositionalAnalysisIR,
) {
    let Some(force) = graph.primary_force() else {
        return;
    };
    if graph
        .commitments
        .iter()
        .any(|commitment| commitment.force == IllocutionaryForceIR::DeferredConditionalRequest)
        && suppress_deferred_conditional_candidates(analysis)
        && force == IllocutionaryForceIR::DeferredConditionalRequest
    {
        return;
    }
    if force == IllocutionaryForceIR::IndirectActionRequest {
        let candidate_id = analysis
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.intent != PlanIntentIR::Explain
                    && candidate.disposition == CandidateDispositionIR::Viable
            })
            .max_by_key(|candidate| candidate.score_millis)
            .map(|candidate| candidate.candidate_id.clone());
        if let Some(candidate_id) = candidate_id {
            for candidate in &mut analysis.candidates {
                if candidate.candidate_id == candidate_id {
                    candidate.disposition = CandidateDispositionIR::Viable;
                    candidate.external_execution_authorized = true;
                    candidate.blockers.clear();
                    candidate
                        .evidence
                        .push("ILLOCUTIONARY_FORCE=INDIRECT_ACTION_REQUEST".to_string());
                }
            }
            if let Some(frame_id) = analysis
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == candidate_id)
                .map(|candidate| candidate.source_frame_id.clone())
            {
                if let Some(frame) = analysis
                    .frames
                    .iter_mut()
                    .find(|frame| frame.frame_id == frame_id)
                {
                    frame.external_execution_authorized = true;
                }
            }
            analysis.selected_candidate_id = Some(candidate_id.clone());
            analysis.selected_candidate_ids = vec![candidate_id];
        }
        return;
    }

    let answer_only = force == IllocutionaryForceIR::AnswerOnlyInformationRequest;
    let mut removed = BTreeSet::new();
    for candidate in &mut analysis.candidates {
        let risky = analysis
            .frames
            .iter()
            .find(|frame| frame.frame_id == candidate.source_frame_id)
            .is_some_and(|frame| matches!(frame.canonical_predicate.as_str(), "DELETE" | "DEPLOY"));
        if !answer_only || risky {
            candidate.external_execution_authorized = false;
            candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
            candidate
                .blockers
                .push(format!("ILLOCUTIONARY_FORCE={force:?}"));
            removed.insert(candidate.candidate_id.clone());
        }
    }
    for frame in &mut analysis.frames {
        let risky = matches!(frame.canonical_predicate.as_str(), "DELETE" | "DEPLOY");
        if !answer_only || risky {
            frame.external_execution_authorized = false;
        }
    }
    analysis
        .selected_candidate_ids
        .retain(|candidate_id| !removed.contains(candidate_id));
    if analysis
        .selected_candidate_id
        .as_ref()
        .is_some_and(|candidate_id| removed.contains(candidate_id))
    {
        analysis.selected_candidate_id = analysis.selected_candidate_ids.first().cloned();
    }
    if answer_only {
        let safe_candidate = analysis
            .candidates
            .iter()
            .filter(|candidate| !removed.contains(&candidate.candidate_id))
            .filter(|candidate| {
                matches!(
                    candidate.intent,
                    PlanIntentIR::Explain | PlanIntentIR::Investigate
                )
            })
            .max_by_key(|candidate| candidate.score_millis)
            .map(|candidate| candidate.candidate_id.clone());
        if let Some(candidate_id) = safe_candidate {
            let frame_id = analysis
                .candidates
                .iter_mut()
                .find(|candidate| candidate.candidate_id == candidate_id)
                .map(|candidate| {
                    candidate.disposition = CandidateDispositionIR::Viable;
                    candidate.external_execution_authorized = true;
                    candidate.blockers.clear();
                    candidate
                        .evidence
                        .push("ILLOCUTIONARY_FORCE=ANSWER_ONLY_INFORMATION_REQUEST".to_string());
                    candidate.source_frame_id.clone()
                });
            if let Some(frame_id) = frame_id {
                if let Some(frame) = analysis
                    .frames
                    .iter_mut()
                    .find(|frame| frame.frame_id == frame_id)
                {
                    frame.external_execution_authorized = true;
                }
            }
            analysis.selected_candidate_id = Some(candidate_id.clone());
            analysis.selected_candidate_ids = vec![candidate_id];
            analysis.clarification_required = false;
            analysis.unresolved_competitions.clear();
        }
    }
    if let Some(goal_graph) = &mut analysis.goal_graph {
        goal_graph
            .nodes
            .retain(|node| !removed.contains(&node.candidate_id));
        let node_ids = goal_graph
            .nodes
            .iter()
            .map(|node| node.node_id.clone())
            .collect::<BTreeSet<_>>();
        goal_graph.edges.retain(|edge| {
            node_ids.contains(&edge.source_node_id) && node_ids.contains(&edge.target_node_id)
        });
        if goal_graph.nodes.is_empty() {
            analysis.goal_graph = None;
        }
    }
}

fn suppress_deferred_conditional_candidates(analysis: &mut CompositionalAnalysisIR) -> bool {
    let deferred_frame_ids = analysis
        .modal_scope_graph
        .conditionals
        .iter()
        .filter(|conditional| conditional.consequent_is_directive)
        .flat_map(|conditional| {
            let consequent = conditional.consequent.to_lowercase();
            analysis
                .frames
                .iter()
                .filter(move |frame| {
                    consequent.contains(&frame.predicate_surface.to_lowercase())
                        || consequent.contains(&frame.canonical_predicate.to_lowercase())
                })
                .map(|frame| frame.frame_id.clone())
        })
        .collect::<BTreeSet<_>>();
    if deferred_frame_ids.is_empty() {
        return false;
    }
    let mut removed = BTreeSet::new();
    for candidate in &mut analysis.candidates {
        if deferred_frame_ids.contains(&candidate.source_frame_id) {
            candidate.external_execution_authorized = false;
            candidate.disposition = CandidateDispositionIR::NonAuthoritativeMention;
            candidate
                .blockers
                .push("ILLOCUTIONARY_FORCE=DeferredConditionalRequest".to_string());
            candidate.blockers.sort();
            candidate.blockers.dedup();
            removed.insert(candidate.candidate_id.clone());
        }
    }
    for frame in &mut analysis.frames {
        if deferred_frame_ids.contains(&frame.frame_id) {
            frame.external_execution_authorized = false;
        }
    }
    analysis
        .selected_candidate_ids
        .retain(|candidate_id| !removed.contains(candidate_id));
    if analysis
        .selected_candidate_id
        .as_ref()
        .is_some_and(|candidate_id| removed.contains(candidate_id))
    {
        analysis.selected_candidate_id = analysis.selected_candidate_ids.first().cloned();
    }
    if let Some(goal_graph) = &mut analysis.goal_graph {
        goal_graph
            .nodes
            .retain(|node| !removed.contains(&node.candidate_id));
        let retained = goal_graph
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();
        goal_graph.edges.retain(|edge| {
            retained.contains(edge.source_node_id.as_str())
                && retained.contains(edge.target_node_id.as_str())
        });
        if goal_graph.nodes.is_empty() {
            analysis.goal_graph = None;
        }
    }
    true
}

fn detect_user_feedback(text: &str, clauses: &[DiscourseClauseIR]) -> Option<UserFeedbackIR> {
    let unquoted = normalize(&strip_quoted_feedback_spans(text));
    let (kind, cues): (UserFeedbackKindIR, &[&str]) = if contains_any(
        &unquoted,
        &[
            "도움이 안",
            "도움이 되지 않",
            "별로야",
            "not helpful",
            "wasn't useful",
        ],
    ) {
        (
            UserFeedbackKindIR::Unhelpful,
            &["도움", "별로", "helpful", "useful"],
        )
    } else if contains_any(&unquoted, &["잘못 이해", "오해", "misunderstood me"]) {
        (
            UserFeedbackKindIR::Misunderstood,
            &["이해", "오해", "misunderstood"],
        )
    } else if contains_any(
        &unquoted,
        &[
            "핵심을 놓",
            "요점을 놓",
            "missed the point",
            "missed the main point",
            "missed the key point",
            "missed my point",
        ],
    ) || (contains_any(
        &unquoted,
        &["그게 아니라", "그건 아니", "아니라 핵심은", "아니라 요점은"],
    ) && contains_any(&unquoted, &["핵심", "요점"]))
    {
        (
            UserFeedbackKindIR::MissedPoint,
            &[
                "핵심",
                "요점",
                "missed",
                "main point",
                "key point",
                "my point",
            ],
        )
    } else if contains_any(&unquoted, &["너무 길", "장황", "too long", "too verbose"]) {
        (
            UserFeedbackKindIR::TooVerbose,
            &["길", "장황", "too long", "verbose"],
        )
    } else if contains_any(
        &unquoted,
        &["너무 짧", "설명이 부족", "too short", "too brief"],
    ) {
        (
            UserFeedbackKindIR::TooBrief,
            &["짧", "부족", "too short", "brief"],
        )
    } else if contains_any(
        &unquoted,
        &["틀렸", "잘못됐", "정확하지 않", "incorrect", "was wrong"],
    ) {
        (
            UserFeedbackKindIR::Incorrect,
            &["틀", "잘못", "정확", "incorrect", "wrong"],
        )
    } else {
        return None;
    };
    let target_surface = if contains_any(&unquoted, &["설명", "explanation"]) {
        "explanation"
    } else if contains_any(&unquoted, &["답변", "답이", "answer", "response"]) {
        "answer"
    } else {
        "interpretation"
    };
    let evidence_clause_ids = clauses
        .iter()
        .filter(|clause| {
            let surface = normalize(&strip_quoted_feedback_spans(&clause.surface_text));
            cues.iter().any(|cue| surface.contains(cue))
        })
        .map(|clause| clause.clause_id.clone())
        .collect::<Vec<_>>();
    Some(UserFeedbackIR {
        kind,
        target_surface: target_surface.to_string(),
        evidence_clause_ids,
        confidence_millis: 940,
    })
}

fn strip_quoted_feedback_spans(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut closing_quote = None;
    for character in text.chars() {
        if let Some(closing) = closing_quote {
            if character == closing {
                closing_quote = None;
            }
            output.push(' ');
            continue;
        }
        closing_quote = match character {
            '‘' => Some('’'),
            '“' => Some('”'),
            '"' => Some('"'),
            _ => None,
        };
        if closing_quote.is_some() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }
    output
}

fn segment_clauses(text: &str) -> Vec<String> {
    let mut punctuation_segments = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if matches!(character, '.' | '?' | '!' | ';' | '\n' | '\r') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                punctuation_segments.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(character);
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        punctuation_segments.push(trimmed.to_string());
    }
    if punctuation_segments.is_empty() && !text.trim().is_empty() {
        punctuation_segments.push(text.trim().to_string());
    }
    punctuation_segments
        .into_iter()
        .flat_map(|segment| split_discourse_connectors(&segment))
        .collect()
}

fn split_discourse_connectors(segment: &str) -> Vec<String> {
    const CONNECTORS: [&str; 13] = [
        " 왜냐",
        " 그래서",
        " 따라서",
        " 그러므로",
        " 하지만",
        " 그러나",
        " 이러면",
        " 그렇다면",
        " because ",
        " therefore ",
        " but ",
        " however ",
        " otherwise ",
    ];
    let mut parts = Vec::new();
    let mut remaining = segment.trim();
    loop {
        let next = CONNECTORS
            .iter()
            .filter_map(|connector| {
                remaining
                    .find(connector)
                    .map(|position| (position, *connector))
            })
            .filter(|(position, _)| *position > 0)
            .min_by_key(|(position, _)| *position);
        let Some((position, _)) = next else {
            if !remaining.is_empty() {
                parts.push(remaining.to_string());
            }
            break;
        };
        let before = remaining[..position].trim();
        if !before.is_empty() {
            parts.push(before.to_string());
        }
        remaining = remaining[position..].trim_start();
    }
    parts
}

fn classify_clause(index: usize, surface: &str) -> DiscourseClauseIR {
    let normalized = normalize(surface);
    let mut roles = BTreeSet::new();
    let mut cues = BTreeSet::new();
    roles.insert(DiscourseClauseRoleIR::Statement);

    add_role(
        &normalized,
        &[
            "기존",
            "과거",
            "실제",
            "측정",
            "결과",
            "previously",
            "before",
            "actual",
            "measured",
            "observed",
        ],
        DiscourseClauseRoleIR::Observation,
        "observation",
        &mut roles,
        &mut cues,
    );
    if english_keep_gerund(&normalized) {
        roles.insert(DiscourseClauseRoleIR::CurrentAction);
        cues.insert("continuation_keep_gerund".to_string());
    }
    add_role(
        &normalized,
        &["왜냐", "때문", "라서", "because", "due to", "since"],
        DiscourseClauseRoleIR::Cause,
        "cause",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "하지만",
            "그러나",
            "반면",
            "높았지",
            "낮았",
            "but",
            "however",
            "whereas",
            "rather than",
        ],
        DiscourseClauseRoleIR::Contrast,
        "contrast",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "려면",
            "하면",
            "다면",
            "정도면",
            "그럼",
            "그러면",
            "경우",
            "아니면",
            "if",
            "unless",
            "provided",
            "otherwise",
            "어야",
            "아야",
            "해야",
            "only if",
            "only when",
            "때만",
            "경우에만",
            "면 ",
        ],
        DiscourseClauseRoleIR::Condition,
        "condition",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "계속",
            "진행",
            "이어가",
            "밀고",
            "carry on",
            "continue",
            "continuing",
            "proceed",
            "keep going",
            "keep doing",
            "keep at it",
        ],
        DiscourseClauseRoleIR::CurrentAction,
        "continuation",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "오류",
            "실패",
            "장애",
            "고장",
            "깨졌",
            "망가",
            "손상",
            "재발",
            "퇴행",
            "느려",
            "비었",
            "error",
            "failure",
            "broken",
            "corruption",
            "corrupt",
            "recurring",
            "returning",
            "keeps returning",
            "regression",
            "slow",
            "missing",
            "empty",
        ],
        DiscourseClauseRoleIR::Problem,
        "problem_state",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "궁금",
            "알고 싶",
            "왜인지",
            "원인이 뭔",
            "wonder",
            "want to know",
            "why",
            "what caused",
        ],
        DiscourseClauseRoleIR::KnowledgeGap,
        "knowledge_gap",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "있으면 좋",
            "편하겠",
            "도움이 되겠",
            "추가하면 좋",
            "would be useful",
            "would help",
            "nice to have",
        ],
        DiscourseClauseRoleIR::Proposal,
        "proposal",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "고통",
            "힘들",
            "어렵",
            "수고",
            "비용",
            "부담",
            "위험",
            "pain",
            "painful",
            "hard",
            "difficult",
            "cost",
            "costly",
            "burden",
            "risk",
            "effort",
        ],
        DiscourseClauseRoleIR::Cost,
        "cost",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "이득",
            "효과",
            "가치",
            "개선",
            "확장",
            "늘",
            "증가",
            "감소",
            "향상",
            "절감",
            "줄어",
            "줄이",
            "줄면",
            "줄 때",
            "benefit",
            "value",
            "improve",
            "improves",
            "improved",
            "expand",
            "expands",
            "increase",
            "increases",
            "reduce",
            "reduces",
            "reduced",
            "lower",
            "lowers",
            "lowered",
            "decrease",
            "decreases",
            "decreased",
            "gain",
            "coverage",
            "broader",
            "wider",
            "넓",
        ],
        DiscourseClauseRoleIR::Benefit,
        "benefit",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "할만",
            "할 만",
            "감수",
            "가치가 있",
            "괜찮",
            "worth",
            "worthwhile",
            "justifies",
            "acceptable",
        ],
        DiscourseClauseRoleIR::Evaluation,
        "utility_evaluation",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "하자",
            "해줘",
            "진행해",
            "계속해",
            "멈춰",
            "중단",
            "물어",
            "보고",
            "let's",
            "should",
            "stop",
            "ask",
            "report",
            "go ahead",
        ],
        DiscourseClauseRoleIR::Decision,
        "decision",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "아니", "없", "못", "않", "no ", "not ", "without", "doesn't", "cannot",
        ],
        DiscourseClauseRoleIR::Negation,
        "negation",
        &mut roles,
        &mut cues,
    );
    add_role(
        &normalized,
        &[
            "아직",
            "모르",
            "불확실",
            "가능성",
            "maybe",
            "might",
            "uncertain",
            "unknown",
            "possibly",
        ],
        DiscourseClauseRoleIR::Uncertainty,
        "uncertainty",
        &mut roles,
        &mut cues,
    );
    if contains_any(
        &normalized,
        &["그래서", "따라서", "그러므로", "therefore", "so "],
    ) {
        cues.insert("consequence".to_string());
    }
    let negative = roles.contains(&DiscourseClauseRoleIR::Negation);
    let positive = contains_any(
        &normalized,
        &[
            "가능", "된다", "있다", "늘", "확장", "향상", "yes", "can", "does", "increase",
            "expand",
        ],
    );
    let polarity = match (positive, negative) {
        (true, true) => PropositionPolarityIR::Mixed,
        (false, true) => PropositionPolarityIR::Negative,
        _ => PropositionPolarityIR::Positive,
    };
    DiscourseClauseIR {
        clause_id: format!("CLAUSE-{:02}", index + 1),
        surface_text: surface.trim().to_string(),
        roles: roles.into_iter().collect(),
        polarity,
        semantic_cues: cues.into_iter().collect(),
    }
}

fn add_role(
    text: &str,
    lexical_family: &[&str],
    role: DiscourseClauseRoleIR,
    cue: &str,
    roles: &mut BTreeSet<DiscourseClauseRoleIR>,
    cues: &mut BTreeSet<String>,
) {
    if contains_any(text, lexical_family) {
        roles.insert(role);
        cues.insert(cue.to_string());
    }
}

fn infer_relations(clauses: &[DiscourseClauseIR]) -> Vec<DiscourseRelationIR> {
    let mut relations = Vec::new();
    for (index, clause) in clauses.iter().enumerate() {
        let previous = index.checked_sub(1).and_then(|prior| clauses.get(prior));
        let next = clauses.get(index + 1);
        if clause.roles.contains(&DiscourseClauseRoleIR::Cause) {
            if let Some(target) = previous.or(next) {
                relations.push(relation(
                    clause,
                    target,
                    DiscourseRelationKindIR::Cause,
                    820,
                ));
            }
        }
        if clause.roles.contains(&DiscourseClauseRoleIR::Contrast) {
            if let Some(target) = previous {
                relations.push(relation(
                    target,
                    clause,
                    DiscourseRelationKindIR::Contrast,
                    840,
                ));
            }
        }
        if clause.roles.contains(&DiscourseClauseRoleIR::Condition) {
            if let Some(target) = next.or(previous) {
                relations.push(relation(
                    clause,
                    target,
                    DiscourseRelationKindIR::Condition,
                    800,
                ));
            }
        }
        if clause.semantic_cues.iter().any(|cue| cue == "consequence") {
            if let Some(source) = previous {
                relations.push(relation(
                    source,
                    clause,
                    DiscourseRelationKindIR::Consequence,
                    900,
                ));
            }
        }
        if clause.roles.contains(&DiscourseClauseRoleIR::Evaluation) {
            if let Some(source) = previous {
                relations.push(relation(
                    source,
                    clause,
                    DiscourseRelationKindIR::Justification,
                    850,
                ));
            }
        }
    }
    relations.sort_by(|left, right| {
        left.source_clause_id
            .cmp(&right.source_clause_id)
            .then_with(|| left.target_clause_id.cmp(&right.target_clause_id))
            .then_with(|| (left.kind as u8).cmp(&(right.kind as u8)))
    });
    relations.dedup();
    relations
}

fn relation(
    source: &DiscourseClauseIR,
    target: &DiscourseClauseIR,
    kind: DiscourseRelationKindIR,
    confidence_millis: u16,
) -> DiscourseRelationIR {
    DiscourseRelationIR {
        source_clause_id: source.clause_id.clone(),
        target_clause_id: target.clause_id.clone(),
        kind,
        confidence_millis,
    }
}

fn clauses_with_role(clauses: &[DiscourseClauseIR], role: DiscourseClauseRoleIR) -> Vec<String> {
    clauses
        .iter()
        .filter(|clause| clause.roles.contains(&role))
        .map(|clause| clause.surface_text.clone())
        .collect()
}

fn benefits_from_clauses(clauses: &[DiscourseClauseIR]) -> Vec<String> {
    let mut candidates = clauses
        .iter()
        .filter(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Benefit))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|clause| {
        let normalized = normalize(&clause.surface_text);
        let direct_outcome = contains_any(
            &normalized,
            &[
                "실제",
                "현실",
                "커버리지",
                "장애",
                "결과",
                "actual",
                "real",
                "coverage",
                "failure",
                "defect",
                "error",
                "outcome",
            ],
        );
        (
            usize::from(!direct_outcome),
            usize::from(action_from_conditional_construction(&clause.surface_text).is_none()),
            usize::from(clause.roles.contains(&DiscourseClauseRoleIR::Cost)),
            clause.clause_id.clone(),
        )
    });
    candidates
        .into_iter()
        .map(|clause| {
            let normalized = normalize(&clause.surface_text);
            for separator in ["을 하면", "를 하면", "을 한다면", "를 한다면"] {
                if let Some(position) = normalized.find(separator) {
                    let benefit = normalized[position + separator.len()..].trim();
                    if !benefit.is_empty() {
                        return benefit.to_string();
                    }
                }
            }
            if let Some(condition) = normalized.strip_prefix("if ") {
                let condition = condition.split(',').next().unwrap_or(condition);
                let mut terms = condition.split_whitespace();
                let _task = terms.next();
                let benefit = terms.collect::<Vec<_>>().join(" ");
                if !benefit.is_empty() {
                    return benefit;
                }
            }
            if let Some(position) = normalized.find(" if ") {
                let condition = normalized[position + " if ".len()..]
                    .split([';', ','])
                    .next()
                    .unwrap_or_default()
                    .trim();
                let mut terms = condition.split_whitespace();
                if terms
                    .next()
                    .is_some_and(|subject| matches!(subject, "it" | "that" | "this"))
                {
                    let benefit = terms.collect::<Vec<_>>().join(" ");
                    if !benefit.is_empty() {
                        return benefit;
                    }
                }
            }
            clause.surface_text.clone()
        })
        .collect()
}

fn infer_evidence_policy(clauses: &[DiscourseClauseIR]) -> EvidencePolicyIR {
    let proxy_measure_clause_ids = clauses
        .iter()
        .filter(|clause| {
            contains_any(
                &normalize(&clause.surface_text),
                &[
                    "점수",
                    "벤치마크",
                    "랭킹",
                    "지표",
                    "score",
                    "benchmark",
                    "ranking",
                    "proxy",
                    "metric",
                ],
            )
        })
        .map(|clause| clause.clause_id.clone())
        .collect::<Vec<_>>();
    let direct_outcome_clause_ids = clauses
        .iter()
        .filter(|clause| {
            contains_any(
                &normalize(&clause.surface_text),
                &[
                    "실제",
                    "현실",
                    "커버리지",
                    "장애",
                    "결과",
                    "actual",
                    "real",
                    "production",
                    "coverage",
                    "failure",
                    "outcome",
                ],
            )
        })
        .map(|clause| clause.clause_id.clone())
        .collect::<Vec<_>>();
    EvidencePolicyIR {
        proxy_only_is_insufficient: !proxy_measure_clause_ids.is_empty()
            && !direct_outcome_clause_ids.is_empty(),
        proxy_measure_clause_ids,
        direct_outcome_clause_ids,
    }
}

fn infer_current_task(clauses: &[DiscourseClauseIR]) -> Option<String> {
    clauses
        .iter()
        .filter(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Benefit))
        .filter_map(|clause| action_from_conditional_construction(&clause.surface_text))
        .next()
        .or_else(|| {
            clauses
                .iter()
                .filter_map(|clause| action_from_continuation_clause(&clause.surface_text))
                .next()
        })
        .or_else(|| {
            clauses
                .iter()
                .filter(|clause| {
                    clause.roles.contains(&DiscourseClauseRoleIR::CurrentAction)
                        && (clause.roles.contains(&DiscourseClauseRoleIR::Condition)
                            || clause.roles.contains(&DiscourseClauseRoleIR::Evaluation))
                })
                .filter_map(|clause| task_from_cost_clause(&clause.surface_text))
                .next()
        })
        .or_else(|| {
            clauses
                .iter()
                .filter(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Cost))
                .filter_map(|clause| task_from_cost_clause(&clause.surface_text))
                .next()
        })
        .or_else(|| {
            clauses
                .iter()
                .filter_map(|clause| action_from_explicit_current_work(&clause.surface_text))
                .next()
        })
}

fn action_from_continuation_clause(text: &str) -> Option<String> {
    let normalized = normalize(text);
    for marker in [
        "continue ",
        "continuing ",
        "proceed with ",
        "keep working on ",
    ] {
        if let Some(position) = normalized.find(marker) {
            let tail = normalized[position + marker.len()..].trim();
            if let Some(candidate) = english_continuation_task(tail) {
                return Some(candidate);
            }
        }
    }
    if contains_any(
        &normalized,
        &[
            "keep doing that",
            "continue that",
            "continue it",
            "keep at it",
        ],
    ) {
        return None;
    }
    if english_keep_gerund(&normalized) {
        if let Some(position) = normalized.find("keep ") {
            let tail = normalized[position + "keep ".len()..].trim();
            if let Some(candidate) = english_continuation_task(tail) {
                return Some(candidate);
            }
        }
    }
    for marker in ["을 계속", "를 계속", "을 이어", "를 이어"] {
        if let Some(position) = normalized.find(marker) {
            let candidate = normalized[..position]
                .split_whitespace()
                .next_back()?
                .trim_matches(|character: char| !character.is_alphanumeric());
            if valid_task_candidate(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    for marker in ["으로 실제", "로 실제", "으로 커버리지", "로 커버리지"] {
        if let Some(position) = normalized.find(marker) {
            let candidate = normalized[..position]
                .split_whitespace()
                .next_back()?
                .trim_end_matches(['은', '는', '이', '가', '을', '를']);
            if valid_task_candidate(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn english_keep_gerund(text: &str) -> bool {
    let words = text
        .split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.windows(2).any(|pair| {
        pair[0] == "keep"
            && pair[1].ends_with("ing")
            && pair[1].len() > "ing".len()
            && !matches!(pair[1], "thing" | "something" | "anything" | "nothing")
    })
}

fn english_continuation_task(tail: &str) -> Option<String> {
    let mut words = tail
        .split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .peekable();
    while words
        .peek()
        .is_some_and(|word| matches!(*word, "the" | "a" | "an" | "this" | "current"))
    {
        words.next();
    }
    if words.peek().is_some_and(|word| {
        matches!(
            *word,
            "it" | "that" | "this" | "doing" | "work" | "task" | "procedure"
        )
    }) {
        return None;
    }
    let candidate = words
        .take_while(|word| {
            !matches!(
                *word,
                "only" | "if" | "unless" | "when" | "provided" | "otherwise" | "because" | "until"
            )
        })
        .take(6)
        .collect::<Vec<_>>()
        .join(" ");
    valid_task_candidate(&candidate).then_some(candidate)
}

fn task_from_cost_clause(text: &str) -> Option<String> {
    let normalized = normalize(text);
    if normalized
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
    {
        let surface = normalized.split_whitespace().next()?;
        let particle = ["은", "는", "이", "가", "을", "를"]
            .iter()
            .find(|particle| surface.ends_with(**particle))?;
        let candidate = surface.strip_suffix(particle)?;
        return valid_task_candidate(candidate).then(|| candidate.to_string());
    }
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let copula = words
        .iter()
        .position(|word| matches!(*word, "is" | "was" | "feels" | "seems"))?;
    let candidate = words[..copula]
        .iter()
        .rev()
        .find(|word| !matches!(**word, "the" | "a" | "an"))?
        .trim_matches(|character: char| !character.is_alphanumeric());
    valid_task_candidate(candidate).then(|| candidate.to_string())
}

fn action_from_conditional_construction(text: &str) -> Option<String> {
    let normalized = normalize(text);
    if contains_any(
        &normalized,
        &[
            "continue it",
            "continue that",
            "keep at it",
            "keep doing that",
            "proceed with it",
        ],
    ) {
        return None;
    }
    for separator in [
        "을 하면",
        "를 하면",
        "을 한다면",
        "를 한다면",
        "을 할 경우",
        "를 할 경우",
    ] {
        if let Some(position) = normalized.find(separator) {
            let prefix = normalized[..position].trim();
            let candidate = prefix
                .split_whitespace()
                .next_back()
                .map(strip_leading_connector)?;
            if valid_task_candidate(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    if let Some(if_position) = normalized.find("if ") {
        let tail = &normalized[if_position + 3..];
        let candidate = tail.split_whitespace().next()?;
        if valid_task_candidate(candidate) {
            return Some(
                candidate
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string(),
            );
        }
    }
    for prefix in ["current task is ", "working on "] {
        if let Some(position) = normalized.find(prefix) {
            let tail = &normalized[position + prefix.len()..];
            let candidate = tail
                .split(|character: char| character == ',' || character.is_whitespace())
                .find(|token| valid_task_candidate(token))?;
            return Some(candidate.to_string());
        }
    }
    None
}

fn action_from_explicit_current_work(text: &str) -> Option<String> {
    let normalized = normalize(text);
    for marker in [
        "하고 있는 ",
        "진행 중인 ",
        "현재 작업은 ",
        "current work is ",
        "current task is ",
    ] {
        if let Some(position) = normalized.find(marker) {
            let tail = &normalized[position + marker.len()..];
            let candidate = tail
                .split(|character: char| {
                    character.is_whitespace() || character == ',' || character == '.'
                })
                .next()?;
            let candidate = candidate.trim_end_matches(['은', '는', '을', '를']);
            if valid_task_candidate(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    for marker in ["we are ", "we're ", "currently "] {
        if let Some(position) = normalized.find(marker) {
            let tail = normalized[position + marker.len()..]
                .trim_matches(|character: char| matches!(character, '.' | ',' | ';' | '!' | '?'));
            let progressive = [
                "integrating ",
                "merging ",
                "migrating ",
                "refactoring ",
                "repairing ",
                "investigating ",
                "testing ",
            ]
            .iter()
            .any(|prefix| tail.starts_with(*prefix));
            if progressive && tail.len() <= 192 {
                return Some(tail.to_string());
            }
        }
    }
    if let Some(position) = normalized.find("하는 중") {
        let candidate = normalized[..position]
            .trim()
            .trim_start_matches("지금 ")
            .trim_start_matches("현재 ")
            .trim();
        if candidate.len() <= 192 && valid_task_candidate(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn strip_leading_connector(candidate: &str) -> &str {
    candidate
        .strip_prefix("그래서")
        .or_else(|| candidate.strip_prefix("따라서"))
        .unwrap_or(candidate)
}

fn valid_task_candidate(candidate: &str) -> bool {
    let cleaned = candidate.trim_matches(|character: char| !character.is_alphanumeric());
    cleaned.chars().count() >= 2
        && !matches!(
            cleaned,
            "계속"
                | "진행"
                | "작업"
                | "가능"
                | "경우"
                | "효과"
                | "continue"
                | "proceed"
                | "current"
                | "task"
                | "work"
                | "if"
                | "it"
                | "only"
                | "actual"
                | "real"
                | "coverage"
                | "failure"
                | "stop"
                | "stopping"
                | "멈춤"
                | "중단"
        )
}

fn direct_context_task(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value.len() > 256 {
        return None;
    }
    valid_task_candidate(value).then(|| value.to_string())
}

fn embedded_context_task(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value.len() > 512 {
        return None;
    }
    if let Some(start) = value.find("task=") {
        let tail = &value[start + 5..];
        let end = tail.find([';', ')']).unwrap_or(tail.len());
        let candidate = tail[..end].trim();
        if valid_task_candidate(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn explicit_negative_action(clauses: &[DiscourseClauseIR]) -> DecisionBranchActionIR {
    let joined = normalize(
        &clauses
            .iter()
            .map(|clause| clause.surface_text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );
    if contains_any(
        &joined,
        &[
            "멈출지",
            "중단할지",
            "물어",
            "확인받",
            "ask whether to stop",
            "ask before stopping",
        ],
    ) {
        DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
    } else if contains_any(
        &joined,
        &[
            "아니면 멈춰",
            "없으면 중단",
            "otherwise stop",
            "if not stop",
        ],
    ) {
        DecisionBranchActionIR::StopCurrentWork
    } else {
        DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
    }
}

fn infer_non_gate_goal(clauses: &[DiscourseClauseIR]) -> Option<InferredPragmaticGoalIR> {
    let knowledge_gap_clauses = clauses
        .iter()
        .filter(|clause| clause.roles.contains(&DiscourseClauseRoleIR::KnowledgeGap))
        .collect::<Vec<_>>();
    if let Some(gap) = knowledge_gap_clauses.first() {
        let subject = clauses
            .iter()
            .find(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Problem))
            .unwrap_or(gap);
        return Some(InferredPragmaticGoalIR {
            intent: PlanIntentIR::Investigate,
            subject: subject.surface_text.clone(),
            desired_outcome: format!("resolve the stated knowledge gap: {}", gap.surface_text),
            commitment: GoalCommitmentIR::ImplicitRequest,
            external_execution_authorized: false,
            basis_clause_ids: unique_clause_ids([*gap, subject]),
            confidence_millis: 840,
        });
    }

    if let Some(goal) = infer_unacceptable_problem_repair_goal(clauses) {
        return Some(goal);
    }

    let proposal = clauses
        .iter()
        .find(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Proposal));
    if let Some(proposal) = proposal {
        return Some(InferredPragmaticGoalIR {
            intent: PlanIntentIR::Create,
            subject: proposal.surface_text.clone(),
            desired_outcome: format!(
                "evaluate and specify the proposed improvement: {}",
                proposal.surface_text
            ),
            commitment: GoalCommitmentIR::Suggestion,
            external_execution_authorized: false,
            basis_clause_ids: vec![proposal.clause_id.clone()],
            confidence_millis: 760,
        });
    }
    None
}

fn infer_unacceptable_problem_repair_goal(
    clauses: &[DiscourseClauseIR],
) -> Option<InferredPragmaticGoalIR> {
    if clauses.iter().any(|clause| {
        contains_any(
            &normalize(&clause.surface_text),
            &[
                "수리하지 마",
                "고치지 마",
                "복구하지 마",
                "do not repair",
                "don't repair",
                "do not fix",
                "don't fix",
            ],
        )
    }) {
        return None;
    }
    let problem = clauses
        .iter()
        .find(|clause| clause.roles.contains(&DiscourseClauseRoleIR::Problem));
    let unacceptable = clauses.iter().find(|clause| {
        contains_any(
            &normalize(&clause.surface_text),
            &[
                "둘 수 없",
                "둘 수는 없",
                "이대로는 안",
                "안 되겠",
                "고쳐야",
                "해결해야",
                "can't leave",
                "cannot leave",
                "needs fixing",
                "must be fixed",
                "not acceptable",
            ],
        )
    });
    if let (Some(problem), Some(unacceptable)) = (problem, unacceptable) {
        return Some(InferredPragmaticGoalIR {
            intent: PlanIntentIR::Repair,
            subject: problem.surface_text.clone(),
            desired_outcome: format!("restore an acceptable state for: {}", problem.surface_text),
            commitment: GoalCommitmentIR::ImplicitRequest,
            external_execution_authorized: false,
            basis_clause_ids: unique_clause_ids([problem, unacceptable]),
            confidence_millis: 820,
        });
    }
    None
}

fn unique_clause_ids<const N: usize>(clauses: [&DiscourseClauseIR; N]) -> Vec<String> {
    clauses
        .into_iter()
        .map(|clause| clause.clause_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn has_negative_continuation_override(clauses: &[DiscourseClauseIR]) -> bool {
    clauses.iter().any(|clause| {
        let text = normalize(&clause.surface_text);
        contains_any(
            &text,
            &[
                "계속하지 마",
                "진행하지 마",
                "바로 계속하지",
                "don't continue",
                "do not continue",
                "not worth continuing",
            ],
        )
    })
}

fn has_approval(clauses: &[DiscourseClauseIR]) -> bool {
    clauses.iter().any(|clause| {
        contains_any(
            &normalize(&clause.surface_text),
            &[
                "좋아",
                "맞아",
                "동의",
                "괜찮",
                "좋네",
                "agree",
                "okay",
                "sounds good",
            ],
        )
    })
}

fn has_request(clauses: &[DiscourseClauseIR]) -> bool {
    clauses.iter().any(|clause| {
        clause.roles.contains(&DiscourseClauseRoleIR::Decision)
            || contains_any(
                &normalize(&clause.surface_text),
                &[
                    "해줘",
                    "해주세요",
                    "하십시오",
                    "please",
                    "could you",
                    "can you",
                ],
            )
    })
}

fn contains_any(text: &str, cues: &[&str]) -> bool {
    cues.iter().any(|cue| {
        if cue.is_ascii()
            && cue
                .chars()
                .all(|character| character.is_ascii_alphabetic() || character == ' ')
        {
            contains_ascii_phrase(text, cue.trim())
        } else {
            text.contains(cue)
        }
    })
}

fn contains_ascii_phrase(text: &str, phrase: &str) -> bool {
    if phrase.contains(' ') {
        return text.contains(phrase);
    }
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|token| token == phrase)
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

    fn interpret(text: &str) -> PragmaticInterpretationIR {
        PragmaticReasoner.interpret(text, &PragmaticContextIR::default())
    }

    #[test]
    fn reconstructs_cost_benefit_continuation_policy_without_sentence_template() {
        let result = interpret(
            "유일하게 고통을 참고 진행하려면 기존에는 점수만 높았지 실제 코딩능력은 한참 낮았다. 왜냐? capability와 routing이 결합되어 나온 거품점수라서 실제 커버리지는 낮았다. 그래서 통합을 하면 커버리지를 확장하는 효과가 있다. 이러면 할만하지",
        );
        assert_eq!(result.speech_act, SpeechActIR::ConditionalContinuation);
        assert!(result.evidence_policy.proxy_only_is_insufficient);
        assert!(!result.evidence_policy.proxy_measure_clause_ids.is_empty());
        assert!(!result.evidence_policy.direct_outcome_clause_ids.is_empty());
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "통합");
        assert!(gate.required_benefit.contains("커버리지"));
        assert_eq!(
            gate.positive_action,
            DecisionBranchActionIR::ContinueCurrentWork
        );
        assert_eq!(
            gate.negative_action,
            DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
        );
        assert!(gate.verification_required);
    }

    #[test]
    fn generalizes_to_different_korean_task_cost_and_benefit() {
        let result = interpret(
            "리팩터링은 힘들다. 그래도 리팩터링을 하면 장애가 줄어드는 효과가 있다. 그 정도 이득이면 계속 진행할 만하다.",
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "리팩터링");
        assert!(gate.required_benefit.contains("장애"));
        assert!(!result.evidence_policy.proxy_only_is_insufficient);
    }

    #[test]
    fn speech_transcript_without_punctuation_keeps_discourse_structure() {
        let result = interpret(
            "마이그레이션이 힘들어도 계속 진행하려면 실제 장애가 줄어야 한다 그래서 마이그레이션을 하면 장애가 줄어드는 효과가 있는지 확인해야 한다 이러면 할 만하다",
        );
        assert!(result.clauses.len() >= 3);
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "마이그레이션");
        assert!(gate.required_benefit.contains("장애"));
    }

    #[test]
    fn generalizes_to_english_conditional_continuation() {
        let result = interpret(
            "The migration is difficult. If migration reduces failures, it is worth continuing despite the cost.",
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "migration");
        assert!(gate.required_benefit.contains("reduces failures"));
    }

    #[test]
    fn compound_utterance_intents_reach_the_central_projection_receipt() {
        let result = interpret(
            "If integration expands coverage, should we continue, and what evidence supports that?",
        );
        let graph = &result.pragmatic_intent_graph.utterance_intent;
        assert_eq!(graph.active().count(), 2);

        let projection = result
            .language_center_goal_projection
            .as_ref()
            .expect("language center goal projection");
        assert!(projection.decisions.iter().any(|decision| {
            decision.effect == LanguageCenterGoalEffectIR::PreserveConstraint
                && decision
                    .evidence_refs
                    .iter()
                    .any(|evidence| evidence.starts_with("UTTERANCE_INTENT_SUPPORTING:"))
        }));
        assert!(result
            .semantic_tags
            .iter()
            .any(|tag| tag == "communicative_intent_evidencerequest"));
    }

    #[test]
    fn required_korean_outcome_clause_recovers_task_subject_without_cost_template() {
        let result = interpret("리팩터링은 실제 장애가 줄어야 계속할 만하다.");
        let gate = result
            .continuation_gate
            .as_ref()
            .expect("continuation gate");
        assert_eq!(gate.current_task, "리팩터링");
        assert!(gate.required_benefit.contains("장애"));
        assert!(result
            .inferred_goal
            .as_ref()
            .is_some_and(|goal| !goal.external_execution_authorized));
    }

    #[test]
    fn english_expletive_it_does_not_replace_the_actual_work_subject() {
        let result = interpret(
            "The integration is painful. It is worth continuing only if actual coverage expands.",
        );
        let gate = result
            .continuation_gate
            .as_ref()
            .expect("continuation gate");
        assert_eq!(gate.current_task, "integration");
        assert!(gate.required_benefit.contains("coverage"));
        assert!(result
            .inferred_goal
            .as_ref()
            .is_some_and(|goal| !goal.external_execution_authorized));
    }

    #[test]
    fn bare_causal_claim_does_not_authorize_continuation() {
        let result = interpret("통합을 하면 커버리지가 확장된다.");
        assert!(result.continuation_gate.is_none());
        assert_eq!(result.speech_act, SpeechActIR::Inform);
    }

    #[test]
    fn direct_feedback_is_typed_but_quoted_feedback_is_not_user_feedback() {
        let direct = interpret("이 답변은 별로야");
        assert_eq!(direct.speech_act, SpeechActIR::NegativeEvaluation);
        assert_eq!(
            direct.user_feedback.as_ref().map(|feedback| feedback.kind),
            Some(UserFeedbackKindIR::Unhelpful)
        );
        let quoted = interpret("민수가 ‘이 답변은 별로야’라고 말했다");
        assert_eq!(quoted.speech_act, SpeechActIR::Inform);
        assert!(quoted.user_feedback.is_none());
    }

    #[test]
    fn explicit_request_remains_authoritative_when_it_follows_feedback() {
        let result = interpret("답변이 너무 길어. 핵심만 다시 설명해");
        assert_eq!(result.speech_act, SpeechActIR::RequestAction);
        assert_eq!(
            result.user_feedback.as_ref().map(|feedback| feedback.kind),
            Some(UserFeedbackKindIR::TooVerbose)
        );
        assert!(result
            .compositional_analysis
            .selected_candidates()
            .iter()
            .any(|candidate| candidate.external_execution_authorized));
    }

    #[test]
    fn negative_override_blocks_positive_gate_inference() {
        let result =
            interpret("통합을 하면 커버리지가 늘 수 있다. 그렇다고 바로 계속하지 마. 비용이 크다.");
        assert!(result.continuation_gate.is_none());
        assert_eq!(result.speech_act, SpeechActIR::Reject);
    }

    #[test]
    fn prior_typed_task_can_fill_an_elided_task_binding() {
        let result = PragmaticReasoner.interpret(
            "힘들어도 장애를 줄이는 효과가 있다면 계속할 만하다.",
            &PragmaticContextIR {
                current_task: Some("API migration".to_string()),
                active_subject: None,
                ..PragmaticContextIR::default()
            },
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "API migration");
    }

    #[test]
    fn stop_branch_is_not_rebound_as_the_current_task() {
        let result = PragmaticReasoner.interpret(
            "Continue it only when fresh trials expand production coverage; otherwise tell me and ask before stopping.",
            &PragmaticContextIR {
                current_task: Some("migrating the storage adapter".to_string()),
                ..PragmaticContextIR::default()
            },
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "migrating the storage adapter");
    }

    #[test]
    fn keep_at_it_does_not_promote_the_if_clause_subject_to_current_task() {
        let result = PragmaticReasoner.interpret(
            "Keep at it only if cold runs reduce production failures; otherwise ask me whether to stop.",
            &PragmaticContextIR {
                current_task: Some("refactoring the parser".to_string()),
                ..PragmaticContextIR::default()
            },
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "refactoring the parser");
        assert!(gate.required_benefit.contains("failures"));
    }

    #[test]
    fn keep_gerund_binds_the_named_task_and_postposed_benefit() {
        let result = interpret(
            "Only keep integrating Aurora if it expands real coverage; otherwise tell me and ask whether to stop.",
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(gate.current_task, "integrating aurora");
        assert_eq!(gate.required_benefit, "expands real coverage");
        assert!(result.unresolved_bindings.is_empty());
    }

    #[test]
    fn continuation_synonyms_use_the_typed_task_and_direct_outcome() {
        let english = PragmaticReasoner.interpret(
            "Proceed with it only if isolated reruns lower the real defect rate; otherwise ask before halting.",
            &PragmaticContextIR {
                current_task: Some("repairing the evidence router".to_string()),
                ..PragmaticContextIR::default()
            },
        );
        let english_gate = english.continuation_gate.expect("English gate");
        assert_eq!(english_gate.current_task, "repairing the evidence router");
        assert!(english_gate.required_benefit.contains("defect"));

        let korean = PragmaticReasoner.interpret(
            "독립 실행에서 실제 오류가 줄어드는 경우에만 그 작업을 이어가. 아니면 멈출지 확인해.",
            &PragmaticContextIR {
                current_task: Some("이벤트 인덱스를 통합".to_string()),
                ..PragmaticContextIR::default()
            },
        );
        let korean_gate = korean.continuation_gate.expect("Korean gate");
        assert_eq!(korean_gate.current_task, "이벤트 인덱스를 통합");
        assert!(korean_gate.required_benefit.contains("오류"));
    }

    #[test]
    fn korean_jul_ttae_is_a_direct_outcome_benefit() {
        let result = interpret(
            "순위는 올랐어도 워밍 캐시 효과일 수 있어. 콜드런에서 운영 장애가 줄 때만 재구성을 이어가.",
        );
        let gate = result
            .continuation_gate
            .clone()
            .unwrap_or_else(|| panic!("continuation gate: {result:#?}"));
        assert_eq!(gate.current_task, "재구성");
        assert!(gate.required_benefit.contains("장애"));
    }

    #[test]
    fn korean_evaluation_with_result_only_keeps_investigation_goal() {
        let result = interpret(
            "가능 여부를 묻는 게 아니야. 게시가 감사 추적을 보존하는지 평가하고 그 결과만 보고해.",
        );
        assert!(
            result
                .compositional_analysis
                .selected_candidates()
                .iter()
                .any(|candidate| {
                    candidate.intent == PlanIntentIR::Investigate
                        && candidate.subject.contains("감사")
                }),
            "investigation goal: {result:#?}"
        );
    }

    #[test]
    fn quoted_publish_does_not_displace_requested_recovery_assessment() {
        let result = interpret(
            "The release lead wrote, 'publish the bundle tonight.' I am asking only for an assessment of recovery cost; do not publish it.",
        );
        assert!(
            result
                .compositional_analysis
                .selected_candidates()
                .iter()
                .any(|candidate| {
                    candidate.intent == PlanIntentIR::Investigate
                        && candidate.subject.contains("recovery")
                }),
            "recovery assessment: {result:#?}"
        );
        assert!(!result
            .compositional_analysis
            .selected_candidates()
            .iter()
            .any(|candidate| {
                result
                    .compositional_analysis
                    .frames
                    .iter()
                    .find(|frame| frame.frame_id == candidate.source_frame_id)
                    .is_some_and(|frame| frame.canonical_predicate == "DEPLOY")
            }));
    }

    #[test]
    fn english_assessment_subject_stops_at_the_sentence_boundary() {
        let result =
            interpret("I only want you to assess the rollback risk. Do not deploy anything.");
        assert!(result
            .compositional_analysis
            .selected_candidates()
            .iter()
            .any(|candidate| {
                candidate.intent == PlanIntentIR::Investigate
                    && candidate.subject == "rollback risk"
            }));
    }

    #[test]
    fn pending_gate_fills_both_task_and_benefit_in_a_later_ellipsis() {
        let result = PragmaticReasoner.interpret(
            "그 정도면 계속할 만하지",
            &PragmaticContextIR {
                current_task: Some("마이그레이션".to_string()),
                pending_required_benefit: Some("장애 빈도가 감소한다".to_string()),
                ..PragmaticContextIR::default()
            },
        );
        let gate = result.continuation_gate.expect("contextual gate");
        assert_eq!(gate.current_task, "마이그레이션");
        assert_eq!(gate.required_benefit, "장애 빈도가 감소한다");
    }

    #[test]
    fn suspended_gate_is_not_silently_reactivated_by_ellipsis() {
        let result = PragmaticReasoner.interpret(
            "그 정도면 계속할 만하지",
            &PragmaticContextIR {
                current_task: Some("마이그레이션".to_string()),
                pending_required_benefit: Some("장애 빈도가 감소한다".to_string()),
                pending_gate_suspended: true,
                ..PragmaticContextIR::default()
            },
        );
        assert!(result.continuation_gate.is_none());
    }

    #[test]
    fn explicit_stop_branch_is_preserved_instead_of_forcing_confirmation() {
        let result = interpret(
            "정리가 힘들어도 정리를 하면 유지보수가 개선된다. 효과가 있으면 계속하고 아니면 멈춰.",
        );
        let gate = result.continuation_gate.expect("continuation gate");
        assert_eq!(
            gate.negative_action,
            DecisionBranchActionIR::StopCurrentWork
        );
    }

    #[test]
    fn unacceptable_problem_state_implies_repair_goal_without_mutation_authority() {
        let result = interpret(
            "배포 후 오류가 늘었네. 직전 변경이 원인인 것 같은데 이 상태로 둘 수는 없지.",
        );
        let goal = result.inferred_goal.expect("implicit repair goal");
        assert_eq!(goal.intent, PlanIntentIR::Repair);
        assert_eq!(goal.commitment, GoalCommitmentIR::ImplicitRequest);
        assert!(!goal.external_execution_authorized);
        assert!(goal.subject.contains("오류"));
        assert_eq!(result.speech_act, SpeechActIR::RequestAction);
    }

    #[test]
    fn curiosity_about_a_problem_implies_investigation_not_repair() {
        let result = interpret("로그가 왜 비어 있는지 궁금하네.");
        let goal = result.inferred_goal.expect("implicit investigation goal");
        assert_eq!(goal.intent, PlanIntentIR::Investigate);
        assert_eq!(result.speech_act, SpeechActIR::Ask);
        assert!(!goal.external_execution_authorized);
    }

    #[test]
    fn desirable_feature_is_a_suggestion_not_execution_authority() {
        let result = interpret("검색 기능이 있으면 반복 작업이 줄어서 편하겠다.");
        let goal = result.inferred_goal.expect("feature suggestion");
        assert_eq!(goal.intent, PlanIntentIR::Create);
        assert_eq!(goal.commitment, GoalCommitmentIR::Suggestion);
        assert_eq!(result.speech_act, SpeechActIR::Suggest);
        assert!(!goal.external_execution_authorized);
    }

    #[test]
    fn bare_problem_observation_requests_diagnosis_without_repair_authority() {
        let result = interpret("배포 후 오류가 세 번 발생했다.");
        let goal = result.inferred_goal.expect("bounded diagnosis goal");
        assert_eq!(goal.intent, PlanIntentIR::Investigate);
        assert!(!goal.external_execution_authorized);
        assert_eq!(result.speech_act, SpeechActIR::Ask);
    }

    #[test]
    fn conditional_continuation_never_projects_an_immediate_continue_action() {
        let result = interpret(
            "클린런에서 실제 커버리지가 넓어질 때만 병합을 계속해. 아니면 멈출지 물어봐.",
        );
        assert!(result.continuation_gate.is_some());
        assert_eq!(result.speech_act, SpeechActIR::ConditionalContinuation);
        assert!(result.compositional_analysis.frames.iter().all(|frame| {
            frame.canonical_predicate != "CONTINUE" || !frame.external_execution_authorized
        }));
        assert!(result
            .compositional_analysis
            .selected_candidates()
            .iter()
            .all(|candidate| candidate.intent != PlanIntentIR::Execute));
    }

    #[test]
    fn english_unacceptable_state_is_a_non_authoritative_repair_goal() {
        let result =
            interpret("The cache corruption keeps returning. We cannot leave it like this.");
        let goal = result.inferred_goal.expect("implicit repair goal");
        assert_eq!(goal.intent, PlanIntentIR::Repair);
        assert!(goal.subject.contains("cache corruption"));
        assert!(!goal.external_execution_authorized);
    }
}
