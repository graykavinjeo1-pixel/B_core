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
use crate::language_knowledge::{LanguageUnderstandingIR, PragmaticFunctionIR};
use crate::modality::{ModalIllocutionIR, ModalWorldIR};
use crate::nonliteral::{NonliteralAnalysisIR, NonliteralAnalyzer};

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
    pub compositional_analysis: CompositionalAnalysisIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_goal: Option<InferredPragmaticGoalIR>,
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

    /// Projects the pragmatic decision into the existing planning request.
    /// The planner receives a verification goal, never the whole paragraph as
    /// an opaque executable target.
    pub fn apply_to_understanding(&self, understanding: &mut LanguageUnderstandingIR) {
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
        let compositional_analysis =
            CompositionalSemanticAnalyzer.analyze_with_predicates(text, learned_predicates);
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
            && (has_evaluation || has_decision || !accepted_costs.is_empty());

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
        if compositional_analysis.clarification_required {
            unresolved_bindings.push("COMPOSITIONAL_INTENT_COMPETITION".to_string());
        }

        let inferred_goal = if let Some(gate) = &continuation_gate {
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
        } else if negative_override || compositional_analysis.clarification_required {
            None
        } else if let Some(candidate) = compositional_analysis.selected_candidate().filter(|_| {
            !compositional_analysis
                .modal_scope_graph
                .blocks_goal_projection()
        }) {
            Some(InferredPragmaticGoalIR {
                intent: candidate.intent,
                subject: candidate.subject.clone(),
                desired_outcome: candidate.desired_outcome.clone(),
                commitment: if candidate.external_execution_authorized {
                    GoalCommitmentIR::ExplicitRequest
                } else {
                    GoalCommitmentIR::ImplicitRequest
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
        let speech_act = if nonliteral_analysis.has_sarcasm() {
            SpeechActIR::NegativeEvaluation
        } else if continuation_gate.is_some() {
            SpeechActIR::ConditionalContinuation
        } else if negative_override {
            SpeechActIR::Reject
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
        } else if !compositional_analysis
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
            compositional_analysis,
            inferred_goal,
            continuation_gate,
            unresolved_bindings,
            semantic_tags: semantic_tags.into_iter().collect(),
            confidence_millis,
        }
    }
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
            "퇴행",
            "느려",
            "비었",
            "error",
            "failure",
            "broken",
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
            "hard",
            "difficult",
            "cost",
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
            "증가",
            "감소",
            "향상",
            "절감",
            "줄어",
            "줄이",
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
            "decrease",
            "decreases",
            "decreased",
            "gain",
            "coverage",
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
        (
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
                .filter_map(|clause| action_from_explicit_current_work(&clause.surface_text))
                .next()
        })
}

fn action_from_conditional_construction(text: &str) -> Option<String> {
    let normalized = normalize(text);
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
    for prefix in ["current task is ", "working on ", "continuing "] {
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
    fn bare_causal_claim_does_not_authorize_continuation() {
        let result = interpret("통합을 하면 커버리지가 확장된다.");
        assert!(result.continuation_gate.is_none());
        assert_eq!(result.speech_act, SpeechActIR::Inform);
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
    fn bare_problem_observation_does_not_invent_a_repair_request() {
        let result = interpret("배포 후 오류가 세 번 발생했다.");
        assert!(result.inferred_goal.is_none());
        assert_eq!(result.speech_act, SpeechActIR::Inform);
    }
}
