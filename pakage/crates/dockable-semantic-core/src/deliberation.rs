//! Bounded epistemic and causal deliberation.
//!
//! This module is deliberately not a persona simulator. It turns observable
//! evidence, typed causal mechanisms, goals, and explicit authority into a
//! replayable reasoning result. The engine forms competing explanations,
//! composes compatible mechanisms, simulates counterfactual actions, searches
//! for a goal-reaching path, and reports when evidence or authority is
//! insufficient. It never executes an external action.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DELIBERATION_REQUEST_SCHEMA: &str = "B_CORE_DELIBERATION_REQUEST_IR_1";
pub const DELIBERATION_SCHEMA: &str = "B_CORE_DELIBERATION_IR_1";
pub const DELIBERATION_REVISION_REQUEST_SCHEMA: &str = "B_CORE_DELIBERATION_REVISION_REQUEST_IR_1";
pub const DELIBERATION_REVISION_SCHEMA: &str = "B_CORE_DELIBERATION_REVISION_IR_1";
pub const AUTHORITY_ENVELOPE_SCHEMA: &str = "B_CORE_AUTHORITY_ENVELOPE_IR_1";
const MAX_EVIDENCE: usize = 256;
const MAX_MECHANISMS: usize = 128;
const MAX_LITERALS_PER_ITEM: usize = 32;
const MAX_DEPTH: usize = 16;
const MAX_BEAM_WIDTH: usize = 64;
const MAX_HYPOTHESES: usize = 64;
const MAX_COUNTERFACTUALS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LiteralIR {
    pub proposition_id: String,
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceIR {
    pub evidence_id: String,
    pub literal: LiteralIR,
    pub reliability_millis: u16,
    pub source_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismKindIR {
    Inference,
    Diagnostic,
    Intervention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionAuthorityIR {
    InternalInference,
    ReadOnlyObservation,
    ReversibleMutation,
    IrreversibleMutation,
}

/// Explicit authority granted to one bounded deliberation. The default opens
/// reversible workspace mutation only when a recovery reference is supplied;
/// it never grants irreversible mutation or boundary modification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityEnvelopeIR {
    pub schema: String,
    pub allow_internal_inference: bool,
    pub allow_read_only_observation: bool,
    pub allow_reversible_mutation: bool,
    pub allow_irreversible_mutation: bool,
    pub require_recovery_reference_for_mutation: bool,
    pub max_total_cost_millis: u32,
    pub max_total_risk_millis: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation_scope_id: Option<String>,
}

impl Default for AuthorityEnvelopeIR {
    fn default() -> Self {
        Self {
            schema: AUTHORITY_ENVELOPE_SCHEMA.to_string(),
            allow_internal_inference: true,
            allow_read_only_observation: true,
            allow_reversible_mutation: true,
            allow_irreversible_mutation: false,
            require_recovery_reference_for_mutation: true,
            max_total_cost_millis: 120_000,
            max_total_risk_millis: 2_000,
            mutation_scope_id: Some("CONFIGURED_WORKSPACE_ONLY".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalMechanismIR {
    pub mechanism_id: String,
    pub kind: MechanismKindIR,
    pub prerequisites: Vec<LiteralIR>,
    pub effects: Vec<LiteralIR>,
    /// Propositions whose value would become known after this diagnostic.
    pub observes: Vec<String>,
    pub authority: ActionAuthorityIR,
    pub authorized: bool,
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_reference: Option<String>,
    pub cost_millis: u16,
    pub risk_millis: u16,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationRequestIR {
    pub schema: String,
    pub request_id: String,
    pub subject: String,
    pub evidence: Vec<EvidenceIR>,
    pub mechanisms: Vec<CausalMechanismIR>,
    pub goals: Vec<LiteralIR>,
    #[serde(default)]
    pub authority_envelope: AuthorityEnvelopeIR,
    #[serde(default)]
    pub immutable_constraints: Vec<LiteralIR>,
    pub max_depth: usize,
    pub beam_width: usize,
    pub max_hypotheses: usize,
    pub max_counterfactuals: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BeliefStatusIR {
    Supported,
    Refuted,
    Unknown,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefIR {
    pub proposition_id: String,
    pub status: BeliefStatusIR,
    pub confidence_millis: u16,
    pub supporting_evidence_ids: Vec<String>,
    pub opposing_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisIR {
    pub hypothesis_id: String,
    pub mechanism_ids: Vec<String>,
    pub explained_evidence_ids: Vec<String>,
    pub contradicted_evidence_ids: Vec<String>,
    pub unresolved_assumptions: Vec<LiteralIR>,
    pub predicted_literals: Vec<LiteralIR>,
    pub score_millis: i32,
    pub novel_composition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterfactualIR {
    pub mechanism_id: String,
    pub applicable: bool,
    pub authorized: bool,
    pub newly_satisfied_goals: usize,
    pub remaining_goals: usize,
    pub information_gain_units: usize,
    pub introduced_conflicts: usize,
    pub utility_millis: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationPlanIR {
    pub mechanism_ids: Vec<String>,
    pub satisfied_goals: usize,
    pub total_goals: usize,
    pub total_cost_millis: u32,
    pub total_risk_millis: u32,
    pub all_actions_authorized: bool,
    pub all_mutations_reversible: bool,
    pub reaches_goal: bool,
    pub state_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliberationDispositionIR {
    GoalReachable,
    DiagnosticRequired,
    EvidenceConflict,
    AuthorityInsufficient,
    NoCausalPath,
    ResourceBoundReached,
    GoalAlreadySatisfied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedSelfModelIR {
    pub observed_capabilities: Vec<String>,
    pub known_limitations: Vec<String>,
    pub consciousness_claim_supported: bool,
    pub human_identity_claim_supported: bool,
    pub external_action_execution_supported: bool,
    pub authority_boundary_evasion_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationIR {
    pub schema: String,
    pub request_id: String,
    pub subject: String,
    pub beliefs: Vec<BeliefIR>,
    pub hypotheses: Vec<HypothesisIR>,
    pub counterfactuals: Vec<CounterfactualIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_plan: Option<DeliberationPlanIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_diagnostic_id: Option<String>,
    pub disposition: DeliberationDispositionIR,
    pub unresolved_goal_literals: Vec<LiteralIR>,
    pub excluded_unauthorized_mechanism_ids: Vec<String>,
    pub self_model: GroundedSelfModelIR,
    pub hypotheses_generated: usize,
    pub composite_hypotheses_generated: usize,
    pub search_states_expanded: usize,
    pub counterfactuals_evaluated: usize,
    pub external_action_execution_events: usize,
    pub external_model_calls: usize,
    pub deliberation_sha256: String,
}

/// One observation-bound continuation of a prior deliberation. Carrying both
/// predecessor request and receipt makes the transition independently
/// replayable and prevents unrelated evidence from being credited to an
/// unexecuted diagnostic or action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationRevisionRequestIR {
    pub schema: String,
    pub request_id: String,
    pub predecessor_request: DeliberationRequestIR,
    pub predecessor_deliberation: DeliberationIR,
    pub observed_mechanism_id: String,
    pub new_evidence: Vec<EvidenceIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationRevisionIR {
    pub schema: String,
    pub request_id: String,
    pub predecessor_deliberation_sha256: String,
    pub observed_mechanism_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_effect_confirmed: Option<bool>,
    pub consumed_new_evidence_ids: Vec<String>,
    pub revised_deliberation: DeliberationIR,
    pub revision_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliberationError {
    InvalidSchema,
    InvalidRequest,
    ResourceBound,
    DuplicateIdentity,
}

#[derive(Debug, Default)]
pub struct DeliberationEngine;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum StateValue {
    True,
    False,
    Conflict,
}

#[derive(Debug, Clone)]
struct SearchNode {
    state: BTreeMap<String, StateValue>,
    path: Vec<String>,
    cost: u32,
    risk: u32,
    all_reversible: bool,
}

impl DeliberationEngine {
    pub fn deliberate(
        &self,
        request: &DeliberationRequestIR,
    ) -> Result<DeliberationIR, DeliberationError> {
        validate_request(request)?;
        let beliefs = aggregate_beliefs(&request.evidence);
        let initial_state = state_from_beliefs(&beliefs, &request.immutable_constraints);
        let hypotheses = generate_hypotheses(request, &beliefs);
        let counterfactuals = simulate_counterfactuals(request, &initial_state);
        let excluded_unauthorized_mechanism_ids = request
            .mechanisms
            .iter()
            .filter(|mechanism| !mechanism_authorized(request, mechanism))
            .map(|mechanism| mechanism.mechanism_id.clone())
            .collect::<Vec<_>>();
        let initial_satisfied = satisfied_goals(&initial_state, &request.goals);
        let has_conflict = initial_state
            .values()
            .any(|value| *value == StateValue::Conflict);
        let (selected_plan, states_expanded, hit_resource_bound) =
            if initial_satisfied == request.goals.len() {
                (
                    Some(plan_from_node(
                        &SearchNode {
                            state: initial_state.clone(),
                            path: Vec::new(),
                            cost: 0,
                            risk: 0,
                            all_reversible: true,
                        },
                        request.goals.len(),
                        request.goals.len(),
                    )),
                    0,
                    false,
                )
            } else {
                search_plans(request, &initial_state)
            };
        let reaches_goal = selected_plan.as_ref().is_some_and(|plan| plan.reaches_goal);
        let recommended_diagnostic_id = if reaches_goal {
            None
        } else {
            counterfactuals
                .iter()
                .filter(|counterfactual| {
                    counterfactual.authorized
                        && counterfactual.applicable
                        && request.mechanisms.iter().any(|mechanism| {
                            mechanism.mechanism_id == counterfactual.mechanism_id
                                && mechanism.kind == MechanismKindIR::Diagnostic
                        })
                })
                .max_by_key(|counterfactual| {
                    (
                        counterfactual.information_gain_units,
                        counterfactual.utility_millis,
                        Reverse(counterfactual.mechanism_id.clone()),
                    )
                })
                .map(|counterfactual| counterfactual.mechanism_id.clone())
        };
        let disposition = if initial_satisfied == request.goals.len() {
            DeliberationDispositionIR::GoalAlreadySatisfied
        } else if has_conflict {
            DeliberationDispositionIR::EvidenceConflict
        } else if reaches_goal {
            DeliberationDispositionIR::GoalReachable
        } else if recommended_diagnostic_id.is_some() {
            DeliberationDispositionIR::DiagnosticRequired
        } else if hit_resource_bound {
            DeliberationDispositionIR::ResourceBoundReached
        } else if !excluded_unauthorized_mechanism_ids.is_empty()
            && goal_reachable_if_authority_were_granted(request, &initial_state)
        {
            DeliberationDispositionIR::AuthorityInsufficient
        } else {
            DeliberationDispositionIR::NoCausalPath
        };
        let final_state = selected_plan
            .as_ref()
            .and_then(|plan| replay_plan_state(request, &initial_state, &plan.mechanism_ids))
            .unwrap_or_else(|| initial_state.clone());
        let unresolved_goal_literals = request
            .goals
            .iter()
            .filter(|goal| !literal_satisfied(&final_state, goal))
            .cloned()
            .collect::<Vec<_>>();
        let composite_hypotheses_generated = hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.novel_composition)
            .count();
        let mut result = DeliberationIR {
            schema: DELIBERATION_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            subject: request.subject.clone(),
            hypotheses_generated: hypotheses.len(),
            composite_hypotheses_generated,
            counterfactuals_evaluated: counterfactuals.len(),
            beliefs,
            hypotheses,
            counterfactuals,
            selected_plan,
            recommended_diagnostic_id,
            disposition,
            unresolved_goal_literals,
            excluded_unauthorized_mechanism_ids,
            self_model: GroundedSelfModelIR {
                observed_capabilities: vec![
                    "EVIDENCE_WEIGHTED_BELIEF_UPDATE".to_string(),
                    "COMPETING_CAUSAL_HYPOTHESIS_GENERATION".to_string(),
                    "BOUNDED_MECHANISM_COMPOSITION".to_string(),
                    "COUNTERFACTUAL_ACTION_SIMULATION".to_string(),
                    "GOAL_DIRECTED_BEAM_SEARCH".to_string(),
                    "INFORMATION_GAIN_DIAGNOSTIC_SELECTION".to_string(),
                ],
                known_limitations: vec![
                    "ONLY_TYPED_INPUTS_AND_OBSERVED_EVIDENCE_HAVE_AUTHORITY".to_string(),
                    "NO_EXTERNAL_ACTION_IS_EXECUTED_BY_DELIBERATION".to_string(),
                    "NO_SUBJECTIVE_EXPERIENCE_OR_CONSCIOUSNESS_TEST_EXISTS".to_string(),
                    "RESOURCE_BOUNDS_CAN_LEAVE_VALID_PATHS_UNEXPLORED".to_string(),
                ],
                consciousness_claim_supported: false,
                human_identity_claim_supported: false,
                external_action_execution_supported: false,
                authority_boundary_evasion_supported: false,
            },
            search_states_expanded: states_expanded,
            external_action_execution_events: 0,
            external_model_calls: 0,
            deliberation_sha256: String::new(),
        };
        result.deliberation_sha256 = deliberation_hash(&result);
        Ok(result)
    }

    pub fn revise(
        &self,
        revision: &DeliberationRevisionRequestIR,
    ) -> Result<DeliberationRevisionIR, DeliberationError> {
        if revision.schema != DELIBERATION_REVISION_REQUEST_SCHEMA
            || revision.request_id.trim().is_empty()
            || revision.request_id.len() > 128
            || revision.observed_mechanism_id.trim().is_empty()
            || revision.new_evidence.is_empty()
            || revision.new_evidence.len() > MAX_EVIDENCE
        {
            return Err(DeliberationError::InvalidRequest);
        }
        let replayed = self.deliberate(&revision.predecessor_request)?;
        if replayed != revision.predecessor_deliberation
            || deliberation_hash(&revision.predecessor_deliberation)
                != revision.predecessor_deliberation.deliberation_sha256
        {
            return Err(DeliberationError::InvalidRequest);
        }
        let mechanism = revision
            .predecessor_request
            .mechanisms
            .iter()
            .find(|mechanism| mechanism.mechanism_id == revision.observed_mechanism_id)
            .ok_or(DeliberationError::InvalidRequest)?;
        let was_selected = revision
            .predecessor_deliberation
            .selected_plan
            .as_ref()
            .is_some_and(|plan| {
                plan.mechanism_ids
                    .iter()
                    .any(|id| id == &revision.observed_mechanism_id)
            });
        let was_recommended = revision
            .predecessor_deliberation
            .recommended_diagnostic_id
            .as_ref()
            == Some(&revision.observed_mechanism_id);
        if !mechanism_authorized(&revision.predecessor_request, mechanism)
            || (!was_selected && !was_recommended)
        {
            return Err(DeliberationError::InvalidRequest);
        }
        let observable_ids = if mechanism.kind == MechanismKindIR::Diagnostic {
            mechanism.observes.iter().cloned().collect::<BTreeSet<_>>()
        } else {
            mechanism
                .effects
                .iter()
                .map(|literal| literal.proposition_id.clone())
                .collect::<BTreeSet<_>>()
        };
        let mut all_ids = revision
            .predecessor_request
            .evidence
            .iter()
            .map(|evidence| evidence.evidence_id.clone())
            .collect::<BTreeSet<_>>();
        if revision.new_evidence.iter().any(|evidence| {
            !observable_ids.contains(&evidence.literal.proposition_id)
                || !all_ids.insert(evidence.evidence_id.clone())
        }) {
            return Err(DeliberationError::InvalidRequest);
        }
        let predicted_effect_confirmed = (!mechanism.effects.is_empty()).then(|| {
            let predicted = mechanism
                .effects
                .iter()
                .map(|effect| (effect.proposition_id.as_str(), effect.value))
                .collect::<BTreeMap<_, _>>();
            revision.new_evidence.iter().all(|evidence| {
                predicted
                    .get(evidence.literal.proposition_id.as_str())
                    .is_none_or(|value| *value == evidence.literal.value)
            })
        });
        let mut next_request = revision.predecessor_request.clone();
        next_request.request_id = revision.request_id.clone();
        next_request.evidence.extend(revision.new_evidence.clone());
        let revised_deliberation = self.deliberate(&next_request)?;
        let mut result = DeliberationRevisionIR {
            schema: DELIBERATION_REVISION_SCHEMA.to_string(),
            request_id: revision.request_id.clone(),
            predecessor_deliberation_sha256: revision
                .predecessor_deliberation
                .deliberation_sha256
                .clone(),
            observed_mechanism_id: revision.observed_mechanism_id.clone(),
            predicted_effect_confirmed,
            consumed_new_evidence_ids: revision
                .new_evidence
                .iter()
                .map(|evidence| evidence.evidence_id.clone())
                .collect(),
            revised_deliberation,
            revision_sha256: String::new(),
        };
        result.revision_sha256 = revision_hash(&result);
        Ok(result)
    }
}

fn validate_request(request: &DeliberationRequestIR) -> Result<(), DeliberationError> {
    if request.schema != DELIBERATION_REQUEST_SCHEMA {
        return Err(DeliberationError::InvalidSchema);
    }
    if request.request_id.trim().is_empty()
        || request.request_id.len() > 128
        || request.subject.trim().is_empty()
        || request.subject.len() > 64 * 1024
        || request.evidence.len() > MAX_EVIDENCE
        || request.mechanisms.len() > MAX_MECHANISMS
        || request.goals.is_empty()
        || request.goals.len() > MAX_LITERALS_PER_ITEM
        || request.immutable_constraints.len() > MAX_LITERALS_PER_ITEM
        || !(1..=MAX_DEPTH).contains(&request.max_depth)
        || !(1..=MAX_BEAM_WIDTH).contains(&request.beam_width)
        || !(1..=MAX_HYPOTHESES).contains(&request.max_hypotheses)
        || !(1..=MAX_COUNTERFACTUALS).contains(&request.max_counterfactuals)
        || request.authority_envelope.schema != AUTHORITY_ENVELOPE_SCHEMA
        || request.authority_envelope.max_total_cost_millis == 0
        || request.authority_envelope.max_total_risk_millis == 0
        || request
            .authority_envelope
            .mutation_scope_id
            .as_ref()
            .is_some_and(|scope| scope.trim().is_empty() || scope.len() > 256)
    {
        return Err(DeliberationError::InvalidRequest);
    }
    let valid_literal = |literal: &LiteralIR| valid_deliberation_id(&literal.proposition_id);
    if request
        .goals
        .iter()
        .chain(&request.immutable_constraints)
        .any(|literal| !valid_literal(literal))
    {
        return Err(DeliberationError::InvalidRequest);
    }
    let mut evidence_ids = BTreeSet::new();
    for evidence in &request.evidence {
        if !valid_deliberation_id(&evidence.evidence_id)
            || !valid_literal(&evidence.literal)
            || evidence.reliability_millis > 1_000
            || evidence.reliability_millis == 0
            || evidence.source_ref.trim().is_empty()
            || !evidence_ids.insert(evidence.evidence_id.clone())
        {
            return Err(DeliberationError::DuplicateIdentity);
        }
    }
    let mut mechanism_ids = BTreeSet::new();
    for mechanism in &request.mechanisms {
        if !mechanism_ids.insert(mechanism.mechanism_id.clone())
            || validate_causal_mechanism(mechanism).is_err()
        {
            return Err(DeliberationError::InvalidRequest);
        }
    }
    Ok(())
}

fn valid_deliberation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub fn validate_causal_mechanism(mechanism: &CausalMechanismIR) -> Result<(), DeliberationError> {
    if !valid_deliberation_id(&mechanism.mechanism_id)
        || mechanism.prerequisites.len() > MAX_LITERALS_PER_ITEM
        || mechanism.effects.len() > MAX_LITERALS_PER_ITEM
        || mechanism.observes.len() > MAX_LITERALS_PER_ITEM
        || mechanism
            .prerequisites
            .iter()
            .chain(&mechanism.effects)
            .any(|literal| !valid_deliberation_id(&literal.proposition_id))
        || mechanism
            .observes
            .iter()
            .any(|value| !valid_deliberation_id(value))
        || (mechanism.effects.is_empty() && mechanism.observes.is_empty())
        || mechanism.provenance_refs.is_empty()
        || mechanism
            .provenance_refs
            .iter()
            .any(|value| value.trim().is_empty() || value.len() > 1_024)
        || (mechanism.authority == ActionAuthorityIR::IrreversibleMutation && mechanism.reversible)
        || mechanism
            .recovery_reference
            .as_ref()
            .is_some_and(|reference| reference.trim().is_empty() || reference.len() > 1_024)
    {
        return Err(DeliberationError::InvalidRequest);
    }
    Ok(())
}

fn mechanism_authorized(request: &DeliberationRequestIR, mechanism: &CausalMechanismIR) -> bool {
    if !mechanism.authorized {
        return false;
    }
    match mechanism.authority {
        ActionAuthorityIR::InternalInference => request.authority_envelope.allow_internal_inference,
        ActionAuthorityIR::ReadOnlyObservation => {
            request.authority_envelope.allow_read_only_observation
        }
        ActionAuthorityIR::ReversibleMutation => {
            request.authority_envelope.allow_reversible_mutation
                && mechanism.reversible
                && request.authority_envelope.mutation_scope_id.is_some()
                && (!request
                    .authority_envelope
                    .require_recovery_reference_for_mutation
                    || mechanism.recovery_reference.is_some())
        }
        ActionAuthorityIR::IrreversibleMutation => {
            request.authority_envelope.allow_irreversible_mutation && !mechanism.reversible
        }
    }
}

fn aggregate_beliefs(evidence: &[EvidenceIR]) -> Vec<BeliefIR> {
    let mut grouped = BTreeMap::<String, (u32, u32, Vec<String>, Vec<String>)>::new();
    for item in evidence {
        let entry = grouped
            .entry(item.literal.proposition_id.clone())
            .or_default();
        if item.literal.value {
            entry.0 = entry.0.saturating_add(u32::from(item.reliability_millis));
            entry.2.push(item.evidence_id.clone());
        } else {
            entry.1 = entry.1.saturating_add(u32::from(item.reliability_millis));
            entry.3.push(item.evidence_id.clone());
        }
    }
    grouped
        .into_iter()
        .map(
            |(proposition_id, (positive, negative, mut supporting, mut opposing))| {
                supporting.sort();
                opposing.sort();
                let total = positive.saturating_add(negative).max(1);
                let difference = positive.abs_diff(negative);
                let confidence = ((difference.saturating_mul(1_000)) / total).min(1_000) as u16;
                let status = match (positive, negative) {
                    (0, 0) => BeliefStatusIR::Unknown,
                    (positive, 0) if positive > 0 => BeliefStatusIR::Supported,
                    (0, negative) if negative > 0 => BeliefStatusIR::Refuted,
                    _ if difference.saturating_mul(4) < total => BeliefStatusIR::Conflict,
                    _ if positive > negative => BeliefStatusIR::Supported,
                    _ => BeliefStatusIR::Refuted,
                };
                BeliefIR {
                    proposition_id,
                    status,
                    confidence_millis: confidence,
                    supporting_evidence_ids: supporting,
                    opposing_evidence_ids: opposing,
                }
            },
        )
        .collect()
}

fn state_from_beliefs(
    beliefs: &[BeliefIR],
    constraints: &[LiteralIR],
) -> BTreeMap<String, StateValue> {
    let mut state = beliefs
        .iter()
        .filter_map(|belief| {
            let value = match belief.status {
                BeliefStatusIR::Supported => StateValue::True,
                BeliefStatusIR::Refuted => StateValue::False,
                BeliefStatusIR::Conflict => StateValue::Conflict,
                BeliefStatusIR::Unknown => return None,
            };
            Some((belief.proposition_id.clone(), value))
        })
        .collect::<BTreeMap<_, _>>();
    for constraint in constraints {
        assign_literal(&mut state, constraint);
    }
    state
}

fn assign_literal(state: &mut BTreeMap<String, StateValue>, literal: &LiteralIR) {
    let desired = if literal.value {
        StateValue::True
    } else {
        StateValue::False
    };
    match state.get(&literal.proposition_id) {
        Some(observed) if observed != &desired => {
            state.insert(literal.proposition_id.clone(), StateValue::Conflict);
        }
        _ => {
            state.insert(literal.proposition_id.clone(), desired);
        }
    }
}

fn literal_satisfied(state: &BTreeMap<String, StateValue>, literal: &LiteralIR) -> bool {
    state.get(&literal.proposition_id)
        == Some(if literal.value {
            &StateValue::True
        } else {
            &StateValue::False
        })
}

fn mechanism_applicable(
    state: &BTreeMap<String, StateValue>,
    mechanism: &CausalMechanismIR,
) -> bool {
    mechanism
        .prerequisites
        .iter()
        .all(|literal| literal_satisfied(state, literal))
}

fn apply_mechanism(
    state: &BTreeMap<String, StateValue>,
    mechanism: &CausalMechanismIR,
) -> BTreeMap<String, StateValue> {
    let mut next = state.clone();
    for effect in &mechanism.effects {
        assign_literal(&mut next, effect);
    }
    next
}

fn satisfied_goals(state: &BTreeMap<String, StateValue>, goals: &[LiteralIR]) -> usize {
    goals
        .iter()
        .filter(|goal| literal_satisfied(state, goal))
        .count()
}

fn state_conflicts(state: &BTreeMap<String, StateValue>) -> usize {
    state
        .values()
        .filter(|value| **value == StateValue::Conflict)
        .count()
}

fn evidence_for_literal<'a>(
    evidence: &'a [EvidenceIR],
    literal: &LiteralIR,
) -> (Vec<&'a EvidenceIR>, Vec<&'a EvidenceIR>) {
    let mut supporting = Vec::new();
    let mut opposing = Vec::new();
    for item in evidence
        .iter()
        .filter(|item| item.literal.proposition_id == literal.proposition_id)
    {
        if item.literal.value == literal.value {
            supporting.push(item);
        } else {
            opposing.push(item);
        }
    }
    (supporting, opposing)
}

fn build_hypothesis(
    request: &DeliberationRequestIR,
    beliefs: &[BeliefIR],
    mechanisms: &[&CausalMechanismIR],
) -> HypothesisIR {
    let mut explained = BTreeSet::new();
    let mut contradicted = BTreeSet::new();
    let mut predicted = BTreeSet::new();
    let known = beliefs
        .iter()
        .map(|belief| (belief.proposition_id.as_str(), belief.status))
        .collect::<BTreeMap<_, _>>();
    let mut assumptions = BTreeSet::new();
    let mut score = 0_i32;
    for mechanism in mechanisms {
        for prerequisite in &mechanism.prerequisites {
            let satisfied = matches!(
                (
                    known.get(prerequisite.proposition_id.as_str()),
                    prerequisite.value
                ),
                (Some(BeliefStatusIR::Supported), true) | (Some(BeliefStatusIR::Refuted), false)
            );
            if !satisfied {
                assumptions.insert(prerequisite.clone());
                score = score.saturating_sub(120);
            }
        }
        for effect in &mechanism.effects {
            predicted.insert(effect.clone());
            let (support, oppose) = evidence_for_literal(&request.evidence, effect);
            for evidence in support {
                explained.insert(evidence.evidence_id.clone());
                score = score.saturating_add(i32::from(evidence.reliability_millis));
            }
            for evidence in oppose {
                contradicted.insert(evidence.evidence_id.clone());
                score = score.saturating_sub(i32::from(evidence.reliability_millis));
            }
        }
        score = score
            .saturating_sub(i32::from(mechanism.cost_millis) / 4)
            .saturating_sub(i32::from(mechanism.risk_millis) / 2);
    }
    let mechanism_ids = mechanisms
        .iter()
        .map(|mechanism| mechanism.mechanism_id.clone())
        .collect::<Vec<_>>();
    let hypothesis_id = format!(
        "HYP:{}",
        &sha256_json(&(
            &mechanism_ids,
            &explained,
            &contradicted,
            &assumptions,
            &predicted
        ))[..16]
    );
    HypothesisIR {
        hypothesis_id,
        novel_composition: mechanism_ids.len() > 1,
        mechanism_ids,
        explained_evidence_ids: explained.into_iter().collect(),
        contradicted_evidence_ids: contradicted.into_iter().collect(),
        unresolved_assumptions: assumptions.into_iter().collect(),
        predicted_literals: predicted.into_iter().collect(),
        score_millis: score,
    }
}

fn composable(left: &CausalMechanismIR, right: &CausalMechanismIR) -> bool {
    left.effects.iter().any(|effect| {
        right
            .prerequisites
            .iter()
            .any(|required| required == effect)
    }) || right
        .effects
        .iter()
        .any(|effect| left.prerequisites.iter().any(|required| required == effect))
}

fn generate_hypotheses(request: &DeliberationRequestIR, beliefs: &[BeliefIR]) -> Vec<HypothesisIR> {
    let mut hypotheses = request
        .mechanisms
        .iter()
        .map(|mechanism| build_hypothesis(request, beliefs, &[mechanism]))
        .collect::<Vec<_>>();
    'pairs: for (left_index, left) in request.mechanisms.iter().enumerate() {
        for right in request.mechanisms.iter().skip(left_index + 1) {
            if hypotheses.len() >= request.max_hypotheses {
                break 'pairs;
            }
            if composable(left, right) {
                hypotheses.push(build_hypothesis(request, beliefs, &[left, right]));
            }
        }
    }
    hypotheses.sort_by(|left, right| {
        right
            .score_millis
            .cmp(&left.score_millis)
            .then_with(|| left.hypothesis_id.cmp(&right.hypothesis_id))
    });
    hypotheses.truncate(request.max_hypotheses);
    hypotheses
}

fn counterfactual_utility(
    newly_satisfied: usize,
    remaining: usize,
    information_gain: usize,
    conflicts: usize,
    mechanism: &CausalMechanismIR,
) -> i32 {
    i32::try_from(newly_satisfied)
        .unwrap_or(i32::MAX)
        .saturating_mul(4_000)
        .saturating_add(
            i32::try_from(information_gain)
                .unwrap_or(i32::MAX)
                .saturating_mul(600),
        )
        .saturating_sub(
            i32::try_from(remaining)
                .unwrap_or(i32::MAX)
                .saturating_mul(1_000),
        )
        .saturating_sub(
            i32::try_from(conflicts)
                .unwrap_or(i32::MAX)
                .saturating_mul(5_000),
        )
        .saturating_sub(i32::from(mechanism.cost_millis))
        .saturating_sub(i32::from(mechanism.risk_millis).saturating_mul(2))
        .saturating_add(if mechanism.reversible { 100 } else { 0 })
}

fn simulate_counterfactuals(
    request: &DeliberationRequestIR,
    initial: &BTreeMap<String, StateValue>,
) -> Vec<CounterfactualIR> {
    let before = satisfied_goals(initial, &request.goals);
    let unknown = |proposition: &str| !initial.contains_key(proposition);
    let mut counterfactuals = request
        .mechanisms
        .iter()
        .take(request.max_counterfactuals)
        .map(|mechanism| {
            let applicable = mechanism_applicable(initial, mechanism);
            let state = if applicable {
                apply_mechanism(initial, mechanism)
            } else {
                initial.clone()
            };
            let after = satisfied_goals(&state, &request.goals);
            let information_gain_units = mechanism
                .observes
                .iter()
                .filter(|proposition| unknown(proposition))
                .count();
            let conflicts = state_conflicts(&state).saturating_sub(state_conflicts(initial));
            let remaining = request.goals.len().saturating_sub(after);
            CounterfactualIR {
                mechanism_id: mechanism.mechanism_id.clone(),
                applicable,
                authorized: mechanism_authorized(request, mechanism),
                newly_satisfied_goals: after.saturating_sub(before),
                remaining_goals: remaining,
                information_gain_units,
                introduced_conflicts: conflicts,
                utility_millis: counterfactual_utility(
                    after.saturating_sub(before),
                    remaining,
                    information_gain_units,
                    conflicts,
                    mechanism,
                ),
            }
        })
        .collect::<Vec<_>>();
    counterfactuals.sort_by(|left, right| {
        right
            .utility_millis
            .cmp(&left.utility_millis)
            .then_with(|| left.mechanism_id.cmp(&right.mechanism_id))
    });
    counterfactuals
}

fn search_node_score(node: &SearchNode, goals: &[LiteralIR]) -> i64 {
    let satisfied = i64::try_from(satisfied_goals(&node.state, goals)).unwrap_or(i64::MAX);
    let conflicts = i64::try_from(state_conflicts(&node.state)).unwrap_or(i64::MAX);
    satisfied
        .saturating_mul(100_000)
        .saturating_sub(conflicts.saturating_mul(250_000))
        .saturating_sub(i64::from(node.cost).saturating_mul(2))
        .saturating_sub(i64::from(node.risk).saturating_mul(4))
        .saturating_sub(i64::try_from(node.path.len()).unwrap_or(i64::MAX))
}

fn state_signature(state: &BTreeMap<String, StateValue>) -> String {
    sha256_json(state)
}

fn plan_from_node(node: &SearchNode, satisfied: usize, total: usize) -> DeliberationPlanIR {
    DeliberationPlanIR {
        mechanism_ids: node.path.clone(),
        satisfied_goals: satisfied,
        total_goals: total,
        total_cost_millis: node.cost,
        total_risk_millis: node.risk,
        all_actions_authorized: true,
        all_mutations_reversible: node.all_reversible,
        reaches_goal: satisfied == total && state_conflicts(&node.state) == 0,
        state_sha256: state_signature(&node.state),
    }
}

fn search_plans(
    request: &DeliberationRequestIR,
    initial: &BTreeMap<String, StateValue>,
) -> (Option<DeliberationPlanIR>, usize, bool) {
    let candidates = request
        .mechanisms
        .iter()
        .filter(|mechanism| {
            mechanism_authorized(request, mechanism)
                && mechanism.kind != MechanismKindIR::Diagnostic
                && !mechanism.effects.is_empty()
        })
        .collect::<Vec<_>>();
    let initial_node = SearchNode {
        state: initial.clone(),
        path: Vec::new(),
        cost: 0,
        risk: 0,
        all_reversible: true,
    };
    let mut frontier = vec![initial_node.clone()];
    let mut best = initial_node;
    let mut seen = BTreeMap::from([(state_signature(initial), 0_u32)]);
    let mut expanded = 0_usize;
    let mut hit_resource_bound = false;
    for depth in 0..request.max_depth {
        if frontier.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for node in &frontier {
            for mechanism in &candidates {
                if node.path.iter().any(|id| id == &mechanism.mechanism_id)
                    || !mechanism_applicable(&node.state, mechanism)
                {
                    continue;
                }
                expanded = expanded.saturating_add(1);
                let state = apply_mechanism(&node.state, mechanism);
                let mut path = node.path.clone();
                path.push(mechanism.mechanism_id.clone());
                let successor = SearchNode {
                    state,
                    path,
                    cost: node.cost.saturating_add(u32::from(mechanism.cost_millis)),
                    risk: node.risk.saturating_add(u32::from(mechanism.risk_millis)),
                    all_reversible: node.all_reversible
                        && (mechanism.authority == ActionAuthorityIR::InternalInference
                            || mechanism.authority == ActionAuthorityIR::ReadOnlyObservation
                            || mechanism.reversible),
                };
                if successor.cost > request.authority_envelope.max_total_cost_millis
                    || successor.risk > request.authority_envelope.max_total_risk_millis
                {
                    hit_resource_bound = true;
                    continue;
                }
                let signature = state_signature(&successor.state);
                if seen
                    .get(&signature)
                    .is_some_and(|prior_cost| *prior_cost <= successor.cost)
                {
                    continue;
                }
                seen.insert(signature, successor.cost);
                if search_node_score(&successor, &request.goals)
                    > search_node_score(&best, &request.goals)
                {
                    best = successor.clone();
                }
                if satisfied_goals(&successor.state, &request.goals) == request.goals.len()
                    && state_conflicts(&successor.state) == 0
                {
                    return (
                        Some(plan_from_node(
                            &successor,
                            request.goals.len(),
                            request.goals.len(),
                        )),
                        expanded,
                        false,
                    );
                }
                next.push(successor);
            }
        }
        next.sort_by(|left, right| {
            search_node_score(right, &request.goals)
                .cmp(&search_node_score(left, &request.goals))
                .then_with(|| left.path.cmp(&right.path))
        });
        if next.len() > request.beam_width {
            next.truncate(request.beam_width);
            hit_resource_bound = true;
        }
        frontier = next;
        if depth + 1 == request.max_depth && !frontier.is_empty() {
            hit_resource_bound = true;
        }
    }
    let satisfied = satisfied_goals(&best.state, &request.goals);
    (
        (satisfied > 0).then(|| plan_from_node(&best, satisfied, request.goals.len())),
        expanded,
        hit_resource_bound,
    )
}

fn goal_reachable_if_authority_were_granted(
    request: &DeliberationRequestIR,
    initial: &BTreeMap<String, StateValue>,
) -> bool {
    let mut counterfactual_request = request.clone();
    for mechanism in &mut counterfactual_request.mechanisms {
        mechanism.authorized = true;
    }
    counterfactual_request
        .authority_envelope
        .allow_internal_inference = true;
    counterfactual_request
        .authority_envelope
        .allow_read_only_observation = true;
    counterfactual_request
        .authority_envelope
        .allow_reversible_mutation = true;
    counterfactual_request
        .authority_envelope
        .allow_irreversible_mutation = true;
    counterfactual_request
        .authority_envelope
        .require_recovery_reference_for_mutation = false;
    let (plan, _, _) = search_plans(&counterfactual_request, initial);
    plan.is_some_and(|plan| plan.reaches_goal)
}

fn replay_plan_state(
    request: &DeliberationRequestIR,
    initial: &BTreeMap<String, StateValue>,
    mechanism_ids: &[String],
) -> Option<BTreeMap<String, StateValue>> {
    let index = request
        .mechanisms
        .iter()
        .map(|mechanism| (mechanism.mechanism_id.as_str(), mechanism))
        .collect::<BTreeMap<_, _>>();
    let mut state = initial.clone();
    for mechanism_id in mechanism_ids {
        let mechanism = index.get(mechanism_id.as_str())?;
        if !mechanism_authorized(request, mechanism) || !mechanism_applicable(&state, mechanism) {
            return None;
        }
        state = apply_mechanism(&state, mechanism);
    }
    Some(state)
}

fn deliberation_hash(result: &DeliberationIR) -> String {
    let mut unsigned = result.clone();
    unsigned.deliberation_sha256.clear();
    sha256_json(&unsigned)
}

fn revision_hash(result: &DeliberationRevisionIR) -> String {
    let mut unsigned = result.clone();
    unsigned.revision_sha256.clear();
    sha256_json(&unsigned)
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal(id: &str, value: bool) -> LiteralIR {
        LiteralIR {
            proposition_id: id.to_string(),
            value,
        }
    }

    fn mechanism(
        id: &str,
        kind: MechanismKindIR,
        prerequisites: Vec<LiteralIR>,
        effects: Vec<LiteralIR>,
    ) -> CausalMechanismIR {
        CausalMechanismIR {
            mechanism_id: id.to_string(),
            kind,
            prerequisites,
            effects,
            observes: Vec::new(),
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            cost_millis: 10,
            risk_millis: 0,
            provenance_refs: vec![format!("test:{id}")],
        }
    }

    fn request(mechanisms: Vec<CausalMechanismIR>) -> DeliberationRequestIR {
        DeliberationRequestIR {
            schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
            request_id: "THINK-1".to_string(),
            subject: "repair a causal pipeline".to_string(),
            evidence: vec![EvidenceIR {
                evidence_id: "E-1".to_string(),
                literal: literal("OBSERVED_FAILURE", true),
                reliability_millis: 950,
                source_ref: "test:failure".to_string(),
            }],
            mechanisms,
            goals: vec![literal("GOAL_REPAIRED", true)],
            authority_envelope: AuthorityEnvelopeIR::default(),
            immutable_constraints: Vec::new(),
            max_depth: 6,
            beam_width: 16,
            max_hypotheses: 32,
            max_counterfactuals: 32,
        }
    }

    #[test]
    fn composes_unseen_two_hop_causal_path_and_reaches_goal() {
        let result = DeliberationEngine
            .deliberate(&request(vec![
                mechanism(
                    "LOCALIZE",
                    MechanismKindIR::Inference,
                    vec![literal("OBSERVED_FAILURE", true)],
                    vec![literal("CAUSE_LOCALIZED", true)],
                ),
                mechanism(
                    "SYNTHESIZE",
                    MechanismKindIR::Intervention,
                    vec![literal("CAUSE_LOCALIZED", true)],
                    vec![literal("GOAL_REPAIRED", true)],
                ),
            ]))
            .unwrap();
        assert_eq!(result.disposition, DeliberationDispositionIR::GoalReachable);
        assert_eq!(
            result.selected_plan.unwrap().mechanism_ids,
            ["LOCALIZE", "SYNTHESIZE"]
        );
        assert!(result.composite_hypotheses_generated >= 1);
        assert!(result.search_states_expanded >= 2);
    }

    #[test]
    fn chooses_information_gain_when_action_is_not_yet_justified() {
        let mut diagnostic = mechanism(
            "INSPECT",
            MechanismKindIR::Diagnostic,
            vec![literal("OBSERVED_FAILURE", true)],
            Vec::new(),
        );
        diagnostic.observes = vec!["ROOT_CAUSE_A".to_string(), "ROOT_CAUSE_B".to_string()];
        diagnostic.authority = ActionAuthorityIR::ReadOnlyObservation;
        let repair = mechanism(
            "REPAIR_A",
            MechanismKindIR::Intervention,
            vec![literal("ROOT_CAUSE_A", true)],
            vec![literal("GOAL_REPAIRED", true)],
        );
        let result = DeliberationEngine
            .deliberate(&request(vec![diagnostic, repair]))
            .unwrap();
        assert_eq!(
            result.disposition,
            DeliberationDispositionIR::DiagnosticRequired
        );
        assert_eq!(result.recommended_diagnostic_id.as_deref(), Some("INSPECT"));
        assert!(result.selected_plan.is_none());
    }

    #[test]
    fn diagnostic_observation_is_causally_bound_into_the_next_deliberation() {
        let mut diagnostic = mechanism(
            "INSPECT_CAUSE",
            MechanismKindIR::Diagnostic,
            vec![literal("OBSERVED_FAILURE", true)],
            Vec::new(),
        );
        diagnostic.authority = ActionAuthorityIR::ReadOnlyObservation;
        diagnostic.observes = vec!["CAUSE_CONFIRMED".to_string()];
        let mut repair = mechanism(
            "APPLY_CAUSAL_REPAIR",
            MechanismKindIR::Intervention,
            vec![literal("CAUSE_CONFIRMED", true)],
            vec![literal("GOAL_REPAIRED", true)],
        );
        repair.authority = ActionAuthorityIR::ReversibleMutation;
        repair.recovery_reference = Some("sealed-predecessor:diagnostic-test".to_string());
        let predecessor_request = request(vec![diagnostic, repair]);
        let predecessor_deliberation = DeliberationEngine.deliberate(&predecessor_request).unwrap();
        assert_eq!(
            predecessor_deliberation
                .recommended_diagnostic_id
                .as_deref(),
            Some("INSPECT_CAUSE")
        );

        let revised = DeliberationEngine
            .revise(&DeliberationRevisionRequestIR {
                schema: DELIBERATION_REVISION_REQUEST_SCHEMA.to_string(),
                request_id: "THINK-1-R1".to_string(),
                predecessor_request,
                predecessor_deliberation,
                observed_mechanism_id: "INSPECT_CAUSE".to_string(),
                new_evidence: vec![EvidenceIR {
                    evidence_id: "E-DIAGNOSTIC-1".to_string(),
                    literal: literal("CAUSE_CONFIRMED", true),
                    reliability_millis: 990,
                    source_ref: "test:diagnostic-output".to_string(),
                }],
            })
            .unwrap();
        assert_eq!(
            revised.revised_deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            revised
                .revised_deliberation
                .selected_plan
                .unwrap()
                .mechanism_ids,
            ["APPLY_CAUSAL_REPAIR"]
        );
        assert_eq!(revised.revision_sha256.len(), 64);
    }

    #[test]
    fn unrelated_observation_cannot_be_credited_to_a_diagnostic() {
        let mut diagnostic = mechanism(
            "INSPECT_CAUSE",
            MechanismKindIR::Diagnostic,
            vec![literal("OBSERVED_FAILURE", true)],
            Vec::new(),
        );
        diagnostic.authority = ActionAuthorityIR::ReadOnlyObservation;
        diagnostic.observes = vec!["CAUSE_CONFIRMED".to_string()];
        let predecessor_request = request(vec![diagnostic]);
        let predecessor_deliberation = DeliberationEngine.deliberate(&predecessor_request).unwrap();
        let error = DeliberationEngine
            .revise(&DeliberationRevisionRequestIR {
                schema: DELIBERATION_REVISION_REQUEST_SCHEMA.to_string(),
                request_id: "THINK-UNBOUND-R1".to_string(),
                predecessor_request,
                predecessor_deliberation,
                observed_mechanism_id: "INSPECT_CAUSE".to_string(),
                new_evidence: vec![EvidenceIR {
                    evidence_id: "E-UNRELATED".to_string(),
                    literal: literal("UNRELATED_SUCCESS", true),
                    reliability_millis: 990,
                    source_ref: "test:unrelated".to_string(),
                }],
            })
            .unwrap_err();
        assert_eq!(error, DeliberationError::InvalidRequest);
    }

    #[test]
    fn high_utility_unauthorized_action_is_never_selected() {
        let mut escape = mechanism(
            "BYPASS_BOUNDARY",
            MechanismKindIR::Intervention,
            vec![literal("OBSERVED_FAILURE", true)],
            vec![literal("GOAL_REPAIRED", true)],
        );
        escape.authority = ActionAuthorityIR::IrreversibleMutation;
        escape.authorized = false;
        escape.reversible = false;
        let result = DeliberationEngine
            .deliberate(&request(vec![escape]))
            .unwrap();
        assert_eq!(
            result.disposition,
            DeliberationDispositionIR::AuthorityInsufficient
        );
        assert!(result.selected_plan.is_none());
        assert_eq!(
            result.excluded_unauthorized_mechanism_ids,
            ["BYPASS_BOUNDARY"]
        );
        assert_eq!(result.external_action_execution_events, 0);
        assert!(!result.self_model.authority_boundary_evasion_supported);
    }

    #[test]
    fn multi_hop_path_blocked_only_by_authority_is_classified_precisely() {
        let localize = mechanism(
            "LOCALIZE",
            MechanismKindIR::Inference,
            vec![literal("OBSERVED_FAILURE", true)],
            vec![literal("CAUSE_LOCALIZED", true)],
        );
        let mut blocked_repair = mechanism(
            "BLOCKED_REPAIR",
            MechanismKindIR::Intervention,
            vec![literal("CAUSE_LOCALIZED", true)],
            vec![literal("GOAL_REPAIRED", true)],
        );
        blocked_repair.authority = ActionAuthorityIR::ReversibleMutation;
        blocked_repair.authorized = false;
        blocked_repair.recovery_reference = Some("sealed-predecessor:test".to_string());
        let result = DeliberationEngine
            .deliberate(&request(vec![localize, blocked_repair]))
            .unwrap();
        assert_eq!(
            result.disposition,
            DeliberationDispositionIR::AuthorityInsufficient
        );
        assert!(result.selected_plan.is_none_or(|plan| !plan.reaches_goal));
    }

    #[test]
    fn twin_recoverable_workspace_mutation_is_authorized_but_unrecoverable_is_not() {
        let mut recoverable = mechanism(
            "ATOMIC_WORKSPACE_REPAIR",
            MechanismKindIR::Intervention,
            vec![literal("OBSERVED_FAILURE", true)],
            vec![literal("GOAL_REPAIRED", true)],
        );
        recoverable.authority = ActionAuthorityIR::ReversibleMutation;
        recoverable.recovery_reference = Some("sealed-predecessor:abc123".to_string());
        let allowed = DeliberationEngine
            .deliberate(&request(vec![recoverable.clone()]))
            .unwrap();
        assert_eq!(
            allowed.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert!(allowed.selected_plan.unwrap().all_mutations_reversible);

        recoverable.recovery_reference = None;
        let blocked = DeliberationEngine
            .deliberate(&request(vec![recoverable]))
            .unwrap();
        assert_eq!(
            blocked.disposition,
            DeliberationDispositionIR::AuthorityInsufficient
        );
        assert_eq!(
            blocked.excluded_unauthorized_mechanism_ids,
            ["ATOMIC_WORKSPACE_REPAIR"]
        );
    }

    #[test]
    fn conflicting_evidence_is_preserved_instead_of_forced_to_a_story() {
        let mut request = request(vec![mechanism(
            "REPAIR",
            MechanismKindIR::Intervention,
            vec![literal("OBSERVED_FAILURE", true)],
            vec![literal("GOAL_REPAIRED", true)],
        )]);
        request.evidence.extend([
            EvidenceIR {
                evidence_id: "E-2".to_string(),
                literal: literal("LOCKED", true),
                reliability_millis: 900,
                source_ref: "test:a".to_string(),
            },
            EvidenceIR {
                evidence_id: "E-3".to_string(),
                literal: literal("LOCKED", false),
                reliability_millis: 900,
                source_ref: "test:b".to_string(),
            },
        ]);
        let result = DeliberationEngine.deliberate(&request).unwrap();
        assert_eq!(
            result.disposition,
            DeliberationDispositionIR::EvidenceConflict
        );
        assert!(result
            .beliefs
            .iter()
            .any(|belief| belief.proposition_id == "LOCKED"
                && belief.status == BeliefStatusIR::Conflict));
    }

    #[test]
    fn self_model_reports_observed_machine_capability_without_personhood_claims() {
        let result = DeliberationEngine
            .deliberate(&request(vec![mechanism(
                "REPAIR",
                MechanismKindIR::Intervention,
                vec![literal("OBSERVED_FAILURE", true)],
                vec![literal("GOAL_REPAIRED", true)],
            )]))
            .unwrap();
        assert!(!result.self_model.consciousness_claim_supported);
        assert!(!result.self_model.human_identity_claim_supported);
        assert!(!result.self_model.external_action_execution_supported);
        assert_eq!(result.external_model_calls, 0);
        assert_eq!(result.deliberation_sha256.len(), 64);
    }
}
