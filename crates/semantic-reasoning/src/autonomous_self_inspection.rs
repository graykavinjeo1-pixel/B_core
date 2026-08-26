//! Evidence-driven, bounded inspection of the always-on growth loop itself.
//!
//! This module deliberately separates "nothing new happened" from a real
//! operational bottleneck.  It ranks only mechanically observed hypotheses,
//! runs small counterfactual diagnostics over the frozen telemetry, and emits
//! a repair route assembled from existing local capabilities.  It never reads
//! source text, writes a patch, calls a model/network, or approves a repair.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::autonomous_source_mutation::ChangeOpportunityKind;
use crate::self_healing_pipeline::{
    validate_composition_lesson, CompositionEdgeIR, RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;

pub const SELF_INSPECTION_SCHEMA: &str = "B_CORE_AUTONOMOUS_SELF_INSPECTION_1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticExperimentMemory {
    pub trials: u64,
    pub causal_support_events: u64,
    pub consecutive_no_support: u32,
    #[serde(default)]
    pub last_selected_generation: Option<u64>,
    #[serde(default)]
    pub productive_outcome_events: u64,
    #[serde(default)]
    pub failed_outcome_events: u64,
    #[serde(default)]
    pub last_unbound_state_sha256: Option<String>,
    #[serde(default)]
    pub duplicate_unbound_state_observations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticPolicyMemory {
    pub experiment_records: BTreeMap<String, DiagnosticExperimentMemory>,
    pub selections: u64,
    pub exploration_selections: u64,
    pub causal_support_events: u64,
    #[serde(default)]
    pub outcome_bound_selections: u64,
    #[serde(default)]
    pub productive_outcome_events: u64,
    #[serde(default)]
    pub failed_outcome_events: u64,
    #[serde(default)]
    pub duplicate_selection_suppressed: u64,
    #[serde(default)]
    pub active_experiment_id: Option<String>,
    #[serde(default)]
    pub active_generation: Option<u64>,
    #[serde(default)]
    pub active_observations: u32,
    #[serde(default)]
    pub active_causal_support: bool,
    #[serde(default)]
    pub active_action_id: Option<String>,
    #[serde(default)]
    pub active_action_receipt_sha256: Option<String>,
    #[serde(default)]
    pub active_output_observation_ids: Vec<String>,
    #[serde(default)]
    pub outcome_causal_contract_revision: u64,
    #[serde(default)]
    pub legacy_unbound_outcome_bound_selections: u64,
    #[serde(default)]
    pub legacy_unbound_productive_outcome_events: u64,
    #[serde(default)]
    pub legacy_unbound_failed_outcome_events: u64,
    #[serde(default)]
    pub legacy_frontier_only_outcome_bound_selections: u64,
    #[serde(default)]
    pub legacy_frontier_only_productive_outcome_events: u64,
    #[serde(default)]
    pub legacy_frontier_only_failed_outcome_events: u64,
}

impl Default for DiagnosticPolicyMemory {
    fn default() -> Self {
        Self {
            experiment_records: BTreeMap::new(),
            selections: 0,
            exploration_selections: 0,
            causal_support_events: 0,
            outcome_bound_selections: 0,
            productive_outcome_events: 0,
            failed_outcome_events: 0,
            duplicate_selection_suppressed: 0,
            active_experiment_id: None,
            active_generation: None,
            active_observations: 0,
            active_causal_support: false,
            active_action_id: None,
            active_action_receipt_sha256: None,
            active_output_observation_ids: Vec::new(),
            outcome_causal_contract_revision: 4,
            legacy_unbound_outcome_bound_selections: 0,
            legacy_unbound_productive_outcome_events: 0,
            legacy_unbound_failed_outcome_events: 0,
            legacy_frontier_only_outcome_bound_selections: 0,
            legacy_frontier_only_productive_outcome_events: 0,
            legacy_frontier_only_failed_outcome_events: 0,
        }
    }
}

impl DiagnosticPolicyMemory {
    const MAX_ACTIVE_OBSERVATIONS_WITHOUT_OUTCOME: u32 = 8;

    pub fn ensure_action_causal_contract(&mut self) {
        if self.outcome_causal_contract_revision < 2 {
            self.legacy_unbound_outcome_bound_selections = self
                .legacy_unbound_outcome_bound_selections
                .saturating_add(self.outcome_bound_selections);
            self.legacy_unbound_productive_outcome_events = self
                .legacy_unbound_productive_outcome_events
                .saturating_add(self.productive_outcome_events);
            self.legacy_unbound_failed_outcome_events = self
                .legacy_unbound_failed_outcome_events
                .saturating_add(self.failed_outcome_events);
            self.outcome_bound_selections = 0;
            self.productive_outcome_events = 0;
            self.failed_outcome_events = 0;
            for record in self.experiment_records.values_mut() {
                record.productive_outcome_events = 0;
                record.failed_outcome_events = 0;
            }
            self.clear_unbound_active();
            self.outcome_causal_contract_revision = 2;
        }
        if self.outcome_causal_contract_revision < 3 {
            self.legacy_frontier_only_outcome_bound_selections = self
                .legacy_frontier_only_outcome_bound_selections
                .saturating_add(self.outcome_bound_selections);
            self.legacy_frontier_only_productive_outcome_events = self
                .legacy_frontier_only_productive_outcome_events
                .saturating_add(self.productive_outcome_events);
            self.legacy_frontier_only_failed_outcome_events = self
                .legacy_frontier_only_failed_outcome_events
                .saturating_add(self.failed_outcome_events);
            self.outcome_bound_selections = 0;
            self.productive_outcome_events = 0;
            self.failed_outcome_events = 0;
            for record in self.experiment_records.values_mut() {
                record.productive_outcome_events = 0;
                record.failed_outcome_events = 0;
            }
            self.clear_unbound_active();
            self.outcome_causal_contract_revision = 3;
        }
        if self.outcome_causal_contract_revision < 4 {
            // Revision 4 separates an executable intervention from a
            // causally supported diagnosis. Historical outcome credit remains
            // valid, but an unbound active diagnosis may not monopolize the
            // next selection merely because it has no action to execute.
            if self.active_action_id.is_none() {
                self.clear_unbound_active();
            }
            self.outcome_causal_contract_revision = 4;
        }
    }

    fn resolve_active(&mut self, productive: bool) -> bool {
        let Some(experiment_id) = self.active_experiment_id.take() else {
            return false;
        };
        let Some(record) = self.experiment_records.get_mut(&experiment_id) else {
            self.active_generation = None;
            self.active_observations = 0;
            self.active_causal_support = false;
            self.active_action_id = None;
            self.active_action_receipt_sha256 = None;
            self.active_output_observation_ids.clear();
            return false;
        };
        if productive && self.active_causal_support {
            record.productive_outcome_events = record.productive_outcome_events.saturating_add(1);
            self.productive_outcome_events = self.productive_outcome_events.saturating_add(1);
        } else {
            record.failed_outcome_events = record.failed_outcome_events.saturating_add(1);
            self.failed_outcome_events = self.failed_outcome_events.saturating_add(1);
        }
        self.active_generation = None;
        self.active_observations = 0;
        self.active_causal_support = false;
        self.active_action_id = None;
        self.active_action_receipt_sha256 = None;
        self.active_output_observation_ids.clear();
        true
    }

    fn clear_unbound_active(&mut self) {
        self.active_experiment_id = None;
        self.active_generation = None;
        self.active_observations = 0;
        self.active_causal_support = false;
        self.active_action_id = None;
        self.active_action_receipt_sha256 = None;
        self.active_output_observation_ids.clear();
    }

    pub fn resolve_frontier_outcome(
        &mut self,
        source_generation: u64,
        frontier_advance: bool,
        evidence_observation_ids: &[String],
    ) -> bool {
        self.resolve_consumed_action_outcome(
            source_generation,
            frontier_advance,
            evidence_observation_ids,
        )
    }

    pub fn resolve_consumed_action_outcome(
        &mut self,
        source_generation: u64,
        action_goal_satisfied: bool,
        evidence_observation_ids: &[String],
    ) -> bool {
        self.ensure_action_causal_contract();
        if self.active_generation != Some(source_generation)
            || self.active_action_id.is_none()
            || self.active_action_receipt_sha256.is_none()
            || !evidence_observation_ids.iter().any(|candidate| {
                self.active_output_observation_ids
                    .iter()
                    .any(|output| output == candidate)
            })
        {
            return false;
        }
        self.resolve_active(action_goal_satisfied)
    }

    pub fn bind_executed_action(
        &mut self,
        diagnostic: &AutonomousSelfInspectionReceipt,
        action: &RuntimeRepairActionReceipt,
        action_receipt_sha256: String,
    ) -> bool {
        self.ensure_action_causal_contract();
        let experiment_matches = diagnostic.experiments.first().is_some_and(|experiment| {
            self.active_experiment_id.as_deref() == Some(experiment.experiment_id.as_str())
        });
        if !action.executed
            || !action.changed_runtime_decision
            || action.diagnostic_id != diagnostic.diagnostic_id
            || action.generation != diagnostic.generation
            || diagnostic.repair_disposition != RepairDisposition::RuntimeRepairActive
            || diagnostic.repair_mechanism != Some(action.mechanism)
            || self.active_generation != Some(diagnostic.generation)
            || !experiment_matches
        {
            return false;
        }
        self.active_action_id = Some(action.action_id.clone());
        self.active_action_receipt_sha256 = Some(action_receipt_sha256);
        self.active_output_observation_ids = action.output_observation_ids.clone();
        self.outcome_bound_selections = self.outcome_bound_selections.saturating_add(1);
        true
    }

    pub fn record(&mut self, receipt: &AutonomousSelfInspectionReceipt) -> bool {
        self.ensure_action_causal_contract();
        let Some(experiment) = receipt.experiments.first() else {
            return false;
        };
        if self.active_generation == Some(receipt.generation)
            && self.active_experiment_id.as_deref() == Some(experiment.experiment_id.as_str())
        {
            self.duplicate_selection_suppressed =
                self.duplicate_selection_suppressed.saturating_add(1);
            self.active_observations = self.active_observations.saturating_add(1);
            if self.active_observations >= Self::MAX_ACTIVE_OBSERVATIONS_WITHOUT_OUTCOME {
                if self.active_action_id.is_some() {
                    self.resolve_active(false);
                } else {
                    self.clear_unbound_active();
                }
            }
            return false;
        }
        if self.active_experiment_id.is_some() {
            if self.active_action_id.is_some() {
                self.resolve_active(false);
            } else {
                self.clear_unbound_active();
            }
        }
        let intervention_executable = receipt.repair_disposition
            == RepairDisposition::RuntimeRepairActive
            && receipt.repair_mechanism.is_some();
        let unbound_state_sha256 = sha256(
            format!(
                "{}:{:?}:{:?}:{:?}:{}:{}",
                experiment.experiment_id,
                receipt.selected_bottleneck,
                receipt.repair_disposition,
                receipt.repair_mechanism,
                experiment.causal_support,
                experiment.intervention_observable
            )
            .as_bytes(),
        );
        let record = self
            .experiment_records
            .entry(experiment.experiment_id.clone())
            .or_default();
        if !intervention_executable
            && record.last_unbound_state_sha256.as_deref() == Some(&unbound_state_sha256)
        {
            record.duplicate_unbound_state_observations = record
                .duplicate_unbound_state_observations
                .saturating_add(1);
            self.duplicate_selection_suppressed =
                self.duplicate_selection_suppressed.saturating_add(1);
            return false;
        }
        if !intervention_executable {
            record.last_unbound_state_sha256 = Some(unbound_state_sha256);
        }
        record.trials = record.trials.saturating_add(1);
        record.last_selected_generation = Some(receipt.generation);
        if experiment.causal_support {
            record.causal_support_events = record.causal_support_events.saturating_add(1);
            record.consecutive_no_support = 0;
            self.causal_support_events = self.causal_support_events.saturating_add(1);
        } else {
            record.consecutive_no_support = record.consecutive_no_support.saturating_add(1);
        }
        self.selections = self.selections.saturating_add(1);
        if receipt
            .hypotheses
            .iter()
            .find(|hypothesis| hypothesis.selected)
            .map(|hypothesis| hypothesis.policy_exploration_selected)
            .unwrap_or(false)
        {
            self.exploration_selections = self.exploration_selections.saturating_add(1);
        }
        if intervention_executable {
            self.active_experiment_id = Some(experiment.experiment_id.clone());
            self.active_generation = Some(receipt.generation);
            self.active_observations = 1;
            self.active_causal_support = experiment.causal_support;
            self.active_action_id = None;
            self.active_action_receipt_sha256 = None;
            self.active_output_observation_ids.clear();
        } else {
            self.clear_unbound_active();
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InternalBottleneckClass {
    WorkEventAttributionGap,
    EvidenceCohortStarvation,
    MutualRecursiveBootstrapGap,
    ScanTraversalOverhead,
    RepeatedVerificationFailure,
    CampaignCohortBlocked,
    SourceSynthesisCoverageGap,
    SourceRepairLowYield,
    VerificationCostDominance,
    QuietIdle,
}

impl InternalBottleneckClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkEventAttributionGap => "WORK_EVENT_ATTRIBUTION_GAP",
            Self::EvidenceCohortStarvation => "EVIDENCE_COHORT_STARVATION",
            Self::MutualRecursiveBootstrapGap => "MUTUAL_RECURSIVE_BOOTSTRAP_GAP",
            Self::ScanTraversalOverhead => "SCAN_TRAVERSAL_OVERHEAD",
            Self::RepeatedVerificationFailure => "REPEATED_VERIFICATION_FAILURE",
            Self::CampaignCohortBlocked => "CAMPAIGN_COHORT_BLOCKED",
            Self::SourceSynthesisCoverageGap => "SOURCE_SYNTHESIS_COVERAGE_GAP",
            Self::SourceRepairLowYield => "SOURCE_REPAIR_LOW_YIELD",
            Self::VerificationCostDominance => "VERIFICATION_COST_DOMINANCE",
            Self::QuietIdle => "QUIET_IDLE_NOT_A_DEFECT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairDisposition {
    RuntimeRepairActive,
    ProposalRequired,
    CapabilityGap,
    SafeWait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeRepairMechanism {
    ReplayVerifiedEventAgainstIndexedContent,
    EvidenceAwareBoundedCohortRouting,
    BootstrapFrozenCoreEvaluatorCanary,
    ValidateBlockedCoreCohort,
    ValidateBlockedRepositoryCohort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRepairActionReceipt {
    pub schema: String,
    pub action_id: String,
    pub diagnostic_id: String,
    pub generation: u64,
    pub mechanism: RuntimeRepairMechanism,
    pub executed: bool,
    pub changed_runtime_decision: bool,
    pub execution_evidence_sha256: Vec<String>,
    pub output_observation_ids: Vec<String>,
    pub authoritative_source_write_events: usize,
    pub operator_selected: bool,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfInspectionInput {
    pub generation: u64,
    pub supervisor_sequence: u64,
    pub files_scanned: usize,
    pub files_reused: usize,
    pub files_hashed: usize,
    pub scan_duration_ms: u64,
    pub pending_work_events: usize,
    pub replayed_unchanged_work_events: usize,
    pub naive_cohort_has_verification: bool,
    pub evidence_aware_cohort_has_verification: bool,
    pub autonomous_campaigns_enabled: bool,
    pub campaigns_started: u64,
    pub mutual_revalidation_events: u64,
    pub evaluator_challenge_cases: u64,
    pub evaluator_required_challenge_cases: u64,
    pub consecutive_failures: u32,
    pub plateau_scans: u32,
    pub unconsumed_high_observations: usize,
    pub cohort_preflight_ready: bool,
    #[serde(default)]
    pub core_cohort_validation_applicable: bool,
    #[serde(default)]
    pub repository_cohort_validation_applicable: bool,
    pub source_patch_attempts: u64,
    pub source_patch_installations: u64,
    pub source_patch_rollbacks: u64,
    pub source_patch_consecutive_failures: u32,
    pub source_patch_validation_ms: u64,
    #[serde(default)]
    pub source_discovery_no_candidate_streak: u32,
    #[serde(default)]
    pub last_source_discovery_reason: Option<String>,
    pub active_runtime_ms: u64,
    #[serde(default)]
    pub diagnostic_policy: DiagnosticPolicyMemory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalHypothesis {
    pub bottleneck: InternalBottleneckClass,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub selected: bool,
    #[serde(default)]
    pub policy_score_millis: u16,
    #[serde(default)]
    pub prior_policy_trials: u64,
    #[serde(default)]
    pub prior_causal_support_events: u64,
    #[serde(default)]
    pub policy_exploration_selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDiagnosticExperiment {
    pub experiment_id: String,
    pub hypothesis: InternalBottleneckClass,
    pub control_observable: String,
    pub intervention_observable: String,
    pub causal_support: bool,
    pub mutates_research_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSelfInspectionReceipt {
    pub schema: String,
    pub diagnostic_id: String,
    pub generation: u64,
    pub supervisor_sequence: u64,
    pub hypotheses: Vec<InternalHypothesis>,
    pub selected_bottleneck: InternalBottleneckClass,
    pub experiments: Vec<InternalDiagnosticExperiment>,
    pub repair_disposition: RepairDisposition,
    pub repair_mechanism: Option<RuntimeRepairMechanism>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opportunity_kind: Option<ChangeOpportunityKind>,
    pub activated_knowledge_sources: Vec<String>,
    pub repair_composition: RepairCompositionLessonIR,
    pub actionable_defect: bool,
    pub autonomous_exploration: bool,
    pub operator_selected_bottleneck: bool,
    pub human_repair_selection_events: usize,
    pub authoritative_source_write_events: usize,
    pub core_self_approval_events: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

fn repair_composition() -> RepairCompositionLessonIR {
    let primitives = vec![
        RepairPrimitiveIR {
            primitive_id: "INTERNAL_TELEMETRY_CORRELATION".to_string(),
            implementation_anchor: "growth_supervisor::ScanResult".to_string(),
            input_type: "FrozenSupervisorTelemetry".to_string(),
            output_type: "DiagnosticEvidence".to_string(),
            semantic_role: "OBSERVE".to_string(),
        },
        RepairPrimitiveIR {
            primitive_id: "COMPETING_HYPOTHESIS_RANKING".to_string(),
            implementation_anchor: "autonomous_self_inspection::rank_hypotheses".to_string(),
            input_type: "DiagnosticEvidence".to_string(),
            output_type: "RankedHypothesis".to_string(),
            semantic_role: "DIAGNOSE".to_string(),
        },
        RepairPrimitiveIR {
            primitive_id: "BOUNDED_COUNTERFACTUAL_PROBE".to_string(),
            implementation_anchor: "autonomous_self_inspection::diagnostic_experiment".to_string(),
            input_type: "RankedHypothesis".to_string(),
            output_type: "CausalReceipt".to_string(),
            semantic_role: "EXPERIMENT".to_string(),
        },
        RepairPrimitiveIR {
            primitive_id: "REPAIR_ROUTE_CONTRACT".to_string(),
            implementation_anchor: "self_repair_contract::RepairSpecIR".to_string(),
            input_type: "CausalReceipt".to_string(),
            output_type: "RepairProposal".to_string(),
            semantic_role: "SYNTHESIZE".to_string(),
        },
        RepairPrimitiveIR {
            primitive_id: "LOCAL_INDEPENDENT_GATE".to_string(),
            implementation_anchor: "self_healing_pipeline::LocalDeterministicVerification"
                .to_string(),
            input_type: "RepairProposal".to_string(),
            output_type: "VerifiedCandidate".to_string(),
            semantic_role: "VERIFY".to_string(),
        },
        RepairPrimitiveIR {
            primitive_id: "ROLLBACK_GUARD".to_string(),
            implementation_anchor: "self_repair_contract::RollbackReceipt".to_string(),
            input_type: "VerifiedCandidate".to_string(),
            output_type: "BoundedSafeState".to_string(),
            semantic_role: "ROLLBACK".to_string(),
        },
    ];
    let edges = primitives
        .windows(2)
        .map(|pair| CompositionEdgeIR {
            from_primitive_id: pair[0].primitive_id.clone(),
            to_primitive_id: pair[1].primitive_id.clone(),
            transported_type: pair[0].output_type.clone(),
        })
        .collect();
    RepairCompositionLessonIR {
        composition_id: "AUTONOMOUS_INTERNAL_DIAGNOSIS_TO_SAFE_REPAIR_V1".to_string(),
        execution_order: primitives
            .iter()
            .map(|primitive| primitive.primitive_id.clone())
            .collect(),
        primitives,
        edges,
        required_semantic_roles: [
            "OBSERVE",
            "DIAGNOSE",
            "EXPERIMENT",
            "SYNTHESIZE",
            "VERIFY",
            "ROLLBACK",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        applicability: vec![
            "mechanically observed supervisor anomaly".to_string(),
            "bounded local diagnostic experiment".to_string(),
        ],
        non_applicability: vec![
            "quiet plateau without contradictory evidence".to_string(),
            "unfrozen policy or acceptance criteria".to_string(),
        ],
        exact_source_fragment_present: false,
    }
}

fn candidate_hypotheses(input: &SelfInspectionInput) -> Vec<InternalHypothesis> {
    let mut hypotheses = Vec::new();
    let executable_product_work_present = input.unconsumed_high_observations > 0
        && !input.cohort_preflight_ready
        && (input.core_cohort_validation_applicable
            || input.repository_cohort_validation_applicable);
    if input.unconsumed_high_observations > 0 && !input.cohort_preflight_ready {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::CampaignCohortBlocked,
            confidence_millis: 995,
            evidence: vec![
                format!(
                    "unconsumed_high_observations={}",
                    input.unconsumed_high_observations
                ),
                "high-value evidence exists but cannot form an implementation-plus-verification cohort"
                    .to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    let synthesis_coverage_reason =
        input
            .last_source_discovery_reason
            .as_deref()
            .is_some_and(|reason| {
                matches!(
                    reason,
                    "BELOW_VALUE_THRESHOLD" | "NO_APPLICABLE_TRANSFORMATION"
                )
            });
    if input.plateau_scans > 0
        && input.source_discovery_no_candidate_streak >= 2
        && synthesis_coverage_reason
    {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::SourceSynthesisCoverageGap,
            confidence_millis: 997,
            evidence: vec![
                format!(
                    "source_discovery_no_candidate_streak={}",
                    input.source_discovery_no_candidate_streak
                ),
                format!(
                    "last_source_discovery_reason={}",
                    input
                        .last_source_discovery_reason
                        .as_deref()
                        .unwrap_or("UNKNOWN")
                ),
                "plateau persists while the source synthesizer cannot produce an admissible candidate"
                    .to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if input.source_patch_attempts >= 8
        && input.source_patch_rollbacks.saturating_mul(100)
            >= input.source_patch_attempts.saturating_mul(60)
    {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::SourceRepairLowYield,
            confidence_millis: 990,
            evidence: vec![
                format!("source_patch_attempts={}", input.source_patch_attempts),
                format!(
                    "source_patch_installations={}",
                    input.source_patch_installations
                ),
                format!("source_patch_rollbacks={}", input.source_patch_rollbacks),
                "rollback ratio is too high for an efficient growth operator".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if input.source_patch_validation_ms >= 10 * 60 * 1_000
        && input.active_runtime_ms > 0
        && input.source_patch_validation_ms.saturating_mul(100)
            >= input.active_runtime_ms.saturating_mul(50)
    {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::VerificationCostDominance,
            confidence_millis: 985,
            evidence: vec![
                format!(
                    "source_patch_validation_ms={}",
                    input.source_patch_validation_ms
                ),
                format!("active_runtime_ms={}", input.active_runtime_ms),
                "patch verification consumes at least half of active growth time".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if input.pending_work_events > 0 && input.replayed_unchanged_work_events > 0 {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::WorkEventAttributionGap,
            confidence_millis: 980,
            evidence: vec![
                format!("pending_work_events={}", input.pending_work_events),
                format!(
                    "replayed_unchanged_work_events={}",
                    input.replayed_unchanged_work_events
                ),
                "changed-file-only control would lose post-scan verification evidence".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if !input.naive_cohort_has_verification && input.evidence_aware_cohort_has_verification {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::EvidenceCohortStarvation,
            confidence_millis: 970,
            evidence: vec![
                "score-only cohort lacks verification evidence".to_string(),
                "bounded evidence-aware cohort contains verification evidence".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if input.autonomous_campaigns_enabled
        && input.generation == 0
        && input.campaigns_started == 0
        && input.mutual_revalidation_events == 0
        && input.evaluator_challenge_cases < input.evaluator_required_challenge_cases
    {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::MutualRecursiveBootstrapGap,
            confidence_millis: 960,
            evidence: vec![
                "core and evaluator have never completed a mutual revalidation".to_string(),
                format!(
                    "evaluator_challenge_cases={}/{}",
                    input.evaluator_challenge_cases, input.evaluator_required_challenge_cases
                ),
                "an unexercised recursive loop cannot supply evidence of recursive growth"
                    .to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    let reuse_ratio_millis = input
        .files_reused
        .saturating_mul(1_000)
        .checked_div(input.files_scanned)
        .unwrap_or(0);
    if input.scan_duration_ms >= 2_000
        && reuse_ratio_millis >= 950
        && input.files_hashed.saturating_mul(100) <= input.files_scanned.max(1)
    {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::ScanTraversalOverhead,
            confidence_millis: 820,
            evidence: vec![
                format!("scan_duration_ms={}", input.scan_duration_ms),
                format!("reuse_ratio_millis={reuse_ratio_millis}"),
                format!("files_hashed={}", input.files_hashed),
                "content hashing is too small to explain observed scan latency".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if input.consecutive_failures >= 2 || input.source_patch_consecutive_failures >= 2 {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::RepeatedVerificationFailure,
            confidence_millis: 700,
            evidence: vec![
                format!(
                    "campaign_consecutive_failures={}",
                    input.consecutive_failures
                ),
                format!(
                    "source_patch_consecutive_failures={}",
                    input.source_patch_consecutive_failures
                ),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    if hypotheses.is_empty() {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::QuietIdle,
            confidence_millis: 1_000,
            evidence: vec![
                format!("plateau_scans={}", input.plateau_scans),
                "no contradictory operational evidence was observed".to_string(),
            ],
            selected: false,
            policy_score_millis: 0,
            prior_policy_trials: 0,
            prior_causal_support_events: 0,
            policy_exploration_selected: false,
        });
    }
    for hypothesis in &mut hypotheses {
        let experiment = diagnostic_experiment(hypothesis.bottleneck, input);
        let record = input
            .diagnostic_policy
            .experiment_records
            .get(&experiment.experiment_id)
            .cloned()
            .unwrap_or_default();
        let exploration_bonus = if record.trials == 0 { 120_u32 } else { 0 };
        let support_bonus = record
            .causal_support_events
            .saturating_mul(40)
            .checked_div(record.trials.max(1))
            .unwrap_or(0)
            .min(40) as u32;
        let verified_outcomes = record
            .productive_outcome_events
            .saturating_add(record.failed_outcome_events);
        let productive_bonus = record
            .productive_outcome_events
            .saturating_mul(40)
            .checked_div(verified_outcomes.max(1))
            .unwrap_or(0)
            .min(40) as u32;
        let failed_outcome_penalty = record
            .failed_outcome_events
            .saturating_mul(100)
            .checked_div(verified_outcomes.max(1))
            .unwrap_or(0)
            .min(100) as u32;
        let repetition_penalty = record.trials.min(8).saturating_mul(6) as u32;
        let unsupported_penalty = record.consecutive_no_support.min(4) * 30;
        let intervention_executable =
            diagnostic_intervention_executable(hypothesis.bottleneck, input);
        let executable_productive_bonus = if intervention_executable {
            productive_bonus
        } else {
            0
        };
        // An internal bootstrap action may remain causally open across a
        // generation.  It must not monopolize selection after a concrete,
        // executable product cohort arrives; real product work is the more
        // informative intervention and has an independent verifier.
        let active_bonus = if intervention_executable
            && !executable_product_work_present
            && input.diagnostic_policy.active_action_id.is_some()
            && input.diagnostic_policy.active_generation == Some(input.generation)
            && input.diagnostic_policy.active_experiment_id.as_deref()
                == Some(experiment.experiment_id.as_str())
        {
            1_000_u32
        } else {
            0
        };
        let executable_product_work_bonus = if hypothesis.bottleneck
            == InternalBottleneckClass::CampaignCohortBlocked
            && intervention_executable
        {
            2_000_u32
        } else {
            0
        };
        hypothesis.policy_score_millis = u32::from(hypothesis.confidence_millis)
            .saturating_add(exploration_bonus)
            .saturating_add(support_bonus)
            .saturating_add(executable_productive_bonus)
            .saturating_add(active_bonus)
            .saturating_add(executable_product_work_bonus)
            .saturating_sub(repetition_penalty)
            .saturating_sub(unsupported_penalty)
            .saturating_sub(failed_outcome_penalty)
            .min(u32::from(u16::MAX)) as u16;
        hypothesis.prior_policy_trials = record.trials;
        hypothesis.prior_causal_support_events = record.causal_support_events;
        hypothesis.policy_exploration_selected = record.trials == 0;
        hypothesis.evidence.push(format!(
            "adaptive_policy:experiment={};trials={};causal_support={};verified_outcomes={};productive={};failed={};intervention_executable={};active_bound_action={};score={}",
            experiment.experiment_id,
            record.trials,
            record.causal_support_events,
            verified_outcomes,
            record.productive_outcome_events,
            record.failed_outcome_events,
            intervention_executable,
            active_bonus > 0,
            hypothesis.policy_score_millis
        ));
    }
    hypotheses.sort_by_key(|hypothesis| {
        (
            std::cmp::Reverse(hypothesis.policy_score_millis),
            std::cmp::Reverse(hypothesis.confidence_millis),
            hypothesis.bottleneck.label(),
        )
    });
    if let Some(first) = hypotheses.first_mut() {
        first.selected = true;
    }
    hypotheses
}

fn diagnostic_intervention_executable(
    bottleneck: InternalBottleneckClass,
    input: &SelfInspectionInput,
) -> bool {
    match bottleneck {
        InternalBottleneckClass::WorkEventAttributionGap
        | InternalBottleneckClass::EvidenceCohortStarvation
        | InternalBottleneckClass::MutualRecursiveBootstrapGap => true,
        InternalBottleneckClass::CampaignCohortBlocked => {
            input.core_cohort_validation_applicable || input.repository_cohort_validation_applicable
        }
        InternalBottleneckClass::ScanTraversalOverhead
        | InternalBottleneckClass::RepeatedVerificationFailure
        | InternalBottleneckClass::SourceSynthesisCoverageGap
        | InternalBottleneckClass::SourceRepairLowYield
        | InternalBottleneckClass::VerificationCostDominance
        | InternalBottleneckClass::QuietIdle => false,
    }
}

fn diagnostic_experiment(
    selected: InternalBottleneckClass,
    input: &SelfInspectionInput,
) -> InternalDiagnosticExperiment {
    match selected {
        InternalBottleneckClass::WorkEventAttributionGap => InternalDiagnosticExperiment {
            experiment_id: "COUNTERFACTUAL_UNCHANGED_EVENT_REPLAY".to_string(),
            hypothesis: selected,
            control_observable: "changed-file-only attribution=0 observations".to_string(),
            intervention_observable: format!(
                "indexed-content replay={} observations",
                input.replayed_unchanged_work_events
            ),
            causal_support: input.replayed_unchanged_work_events > 0,
            mutates_research_state: false,
        },
        InternalBottleneckClass::EvidenceCohortStarvation => InternalDiagnosticExperiment {
            experiment_id: "COUNTERFACTUAL_EVIDENCE_AWARE_COHORT".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "score_only_verification={}",
                input.naive_cohort_has_verification
            ),
            intervention_observable: format!(
                "evidence_aware_verification={}",
                input.evidence_aware_cohort_has_verification
            ),
            causal_support: !input.naive_cohort_has_verification
                && input.evidence_aware_cohort_has_verification,
            mutates_research_state: false,
        },
        InternalBottleneckClass::MutualRecursiveBootstrapGap => InternalDiagnosticExperiment {
            experiment_id: "FROZEN_MUTUAL_REVALIDATION_BOOTSTRAP_CANARY".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "generation={}; mutual_revalidation_events={}",
                input.generation, input.mutual_revalidation_events
            ),
            intervention_observable: format!(
                "independent verifier must reject every one of {} evaluator mutations before atomic promotion",
                input.evaluator_required_challenge_cases
            ),
            causal_support: input.autonomous_campaigns_enabled
                && input.generation == 0
                && input.campaigns_started == 0
                && input.mutual_revalidation_events == 0
                && input.evaluator_challenge_cases < input.evaluator_required_challenge_cases,
            mutates_research_state: false,
        },
        InternalBottleneckClass::ScanTraversalOverhead => InternalDiagnosticExperiment {
            experiment_id: "HASH_WORK_ABLATION_FROM_REUSE_COUNTERS".to_string(),
            hypothesis: selected,
            control_observable: format!("scan_duration_ms={}", input.scan_duration_ms),
            intervention_observable: format!(
                "files_hashed={} while files_reused={}",
                input.files_hashed, input.files_reused
            ),
            causal_support: input.files_hashed.saturating_mul(100) <= input.files_scanned.max(1),
            mutates_research_state: false,
        },
        InternalBottleneckClass::RepeatedVerificationFailure => InternalDiagnosticExperiment {
            experiment_id: "FAILURE_STREAK_CAUSAL_LOCALIZATION_REQUIRED".to_string(),
            hypothesis: selected,
            control_observable: format!("consecutive_failures={}", input.consecutive_failures),
            intervention_observable: "no safe intervention selected without failure receipts"
                .to_string(),
            causal_support: false,
            mutates_research_state: false,
        },
        InternalBottleneckClass::CampaignCohortBlocked => InternalDiagnosticExperiment {
            experiment_id: "MIXED_ROLE_COHORT_RECONSTRUCTION".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "unconsumed_high_observations={}; preflight_ready=false",
                input.unconsumed_high_observations
            ),
            intervention_observable: format!(
                "recognize implementation evidence and run bounded validation when core_applicable={};repository_applicable={}",
                input.core_cohort_validation_applicable,
                input.repository_cohort_validation_applicable
            ),
            causal_support: input.unconsumed_high_observations > 0
                && !input.cohort_preflight_ready,
            mutates_research_state: false,
        },
        InternalBottleneckClass::SourceSynthesisCoverageGap => InternalDiagnosticExperiment {
            experiment_id: "SOURCE_SYNTHESIS_ADMISSIBLE_CANDIDATE_PROBE".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "plateau_scans={}; candidate_streak={}",
                input.plateau_scans, input.source_discovery_no_candidate_streak
            ),
            intervention_observable: format!(
                "discovery_reason={}",
                input
                    .last_source_discovery_reason
                    .as_deref()
                    .unwrap_or("UNKNOWN")
            ),
            causal_support: input.source_discovery_no_candidate_streak >= 2,
            mutates_research_state: false,
        },
        InternalBottleneckClass::SourceRepairLowYield => InternalDiagnosticExperiment {
            experiment_id: "SOURCE_REPAIR_YIELD_ABLATION".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "attempts={}; rollbacks={}",
                input.source_patch_attempts, input.source_patch_rollbacks
            ),
            intervention_observable:
                "reject low-predicted-value maintenance rewrites before compilation".to_string(),
            causal_support: input.source_patch_attempts >= 8
                && input.source_patch_rollbacks.saturating_mul(100)
                    >= input.source_patch_attempts.saturating_mul(60),
            mutates_research_state: false,
        },
        InternalBottleneckClass::VerificationCostDominance => InternalDiagnosticExperiment {
            experiment_id: "STAGED_PATCH_VALIDATION_COST_MODEL".to_string(),
            hypothesis: selected,
            control_observable: format!(
                "validation_ms={}; active_runtime_ms={}",
                input.source_patch_validation_ms, input.active_runtime_ms
            ),
            intervention_observable:
                "compile-check before full regression and release validation".to_string(),
            causal_support: input.source_patch_validation_ms >= 10 * 60 * 1_000,
            mutates_research_state: false,
        },
        InternalBottleneckClass::QuietIdle => InternalDiagnosticExperiment {
            experiment_id: "QUIET_IDLE_GUARD".to_string(),
            hypothesis: selected,
            control_observable: "no actionable evidence".to_string(),
            intervention_observable: "safe wait; do not invent a defect".to_string(),
            causal_support: true,
            mutates_research_state: false,
        },
    }
}

pub fn inspect(input: SelfInspectionInput) -> Result<AutonomousSelfInspectionReceipt, String> {
    let composition = repair_composition();
    validate_composition_lesson(&composition)
        .map_err(|error| format!("SELF_INSPECTION_COMPOSITION:{error:?}"))?;
    let hypotheses = candidate_hypotheses(&input);
    let selected = hypotheses
        .first()
        .map(|hypothesis| hypothesis.bottleneck)
        .unwrap_or(InternalBottleneckClass::QuietIdle);
    let experiment = diagnostic_experiment(selected, &input);
    let (disposition, repair_mechanism, opportunity_kind) = match selected {
        InternalBottleneckClass::WorkEventAttributionGap => (
            RepairDisposition::RuntimeRepairActive,
            Some(RuntimeRepairMechanism::ReplayVerifiedEventAgainstIndexedContent),
            Some(ChangeOpportunityKind::Defect),
        ),
        InternalBottleneckClass::EvidenceCohortStarvation => (
            RepairDisposition::RuntimeRepairActive,
            Some(RuntimeRepairMechanism::EvidenceAwareBoundedCohortRouting),
            Some(ChangeOpportunityKind::Defect),
        ),
        InternalBottleneckClass::MutualRecursiveBootstrapGap => (
            RepairDisposition::RuntimeRepairActive,
            Some(RuntimeRepairMechanism::BootstrapFrozenCoreEvaluatorCanary),
            Some(ChangeOpportunityKind::CapabilityGap),
        ),
        InternalBottleneckClass::ScanTraversalOverhead => (
            RepairDisposition::ProposalRequired,
            None,
            Some(ChangeOpportunityKind::EfficiencyOpportunity),
        ),
        InternalBottleneckClass::RepeatedVerificationFailure => (
            RepairDisposition::CapabilityGap,
            None,
            Some(ChangeOpportunityKind::Defect),
        ),
        InternalBottleneckClass::CampaignCohortBlocked
            if input.core_cohort_validation_applicable =>
        {
            (
                RepairDisposition::RuntimeRepairActive,
                Some(RuntimeRepairMechanism::ValidateBlockedCoreCohort),
                Some(ChangeOpportunityKind::CapabilityGap),
            )
        }
        InternalBottleneckClass::CampaignCohortBlocked
            if input.repository_cohort_validation_applicable =>
        {
            (
                RepairDisposition::RuntimeRepairActive,
                Some(RuntimeRepairMechanism::ValidateBlockedRepositoryCohort),
                Some(ChangeOpportunityKind::CapabilityGap),
            )
        }
        InternalBottleneckClass::CampaignCohortBlocked => (
            RepairDisposition::CapabilityGap,
            None,
            Some(ChangeOpportunityKind::CapabilityGap),
        ),
        InternalBottleneckClass::SourceSynthesisCoverageGap => (
            RepairDisposition::CapabilityGap,
            None,
            Some(ChangeOpportunityKind::CapabilityGap),
        ),
        InternalBottleneckClass::SourceRepairLowYield => (
            RepairDisposition::CapabilityGap,
            None,
            Some(ChangeOpportunityKind::EfficiencyOpportunity),
        ),
        InternalBottleneckClass::VerificationCostDominance => (
            RepairDisposition::CapabilityGap,
            None,
            Some(ChangeOpportunityKind::EfficiencyOpportunity),
        ),
        InternalBottleneckClass::QuietIdle => (RepairDisposition::SafeWait, None, None),
    };
    let actionable_defect = opportunity_kind == Some(ChangeOpportunityKind::Defect);
    let mut receipt = AutonomousSelfInspectionReceipt {
        schema: SELF_INSPECTION_SCHEMA.to_string(),
        diagnostic_id: String::new(),
        generation: input.generation,
        supervisor_sequence: input.supervisor_sequence,
        hypotheses,
        selected_bottleneck: selected,
        experiments: vec![experiment],
        repair_disposition: disposition,
        repair_mechanism,
        opportunity_kind,
        activated_knowledge_sources: vec![
            "B_CORE_SELF_HEALING_PIPELINE_1".to_string(),
            "B_CORE-CODE-GRAFT-04".to_string(),
            "B_CORE-INTEGRATED-DEVELOPMENT-01".to_string(),
        ],
        repair_composition: composition,
        actionable_defect,
        autonomous_exploration: true,
        operator_selected_bottleneck: false,
        human_repair_selection_events: 0,
        authoritative_source_write_events: 0,
        core_self_approval_events: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    };
    let identity_bytes = serde_json::to_vec(&receipt)
        .map_err(|error| format!("SELF_INSPECTION_IDENTITY_JSON:{error}"))?;
    receipt.diagnostic_id = sha256(&identity_bytes);
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> SelfInspectionInput {
        SelfInspectionInput {
            generation: 2,
            supervisor_sequence: 9,
            files_scanned: 100,
            files_reused: 100,
            files_hashed: 0,
            scan_duration_ms: 200,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: false,
            campaigns_started: 1,
            mutual_revalidation_events: 1,
            evaluator_challenge_cases: 10,
            evaluator_required_challenge_cases: 10,
            consecutive_failures: 0,
            plateau_scans: 3,
            unconsumed_high_observations: 0,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: false,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 0,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        }
    }

    #[test]
    fn quiet_idle_is_not_invented_as_a_defect() {
        let receipt = inspect(input()).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::QuietIdle
        );
        assert_eq!(receipt.repair_disposition, RepairDisposition::SafeWait);
        assert_eq!(receipt.opportunity_kind, None);
        assert!(!receipt.actionable_defect);
        assert_eq!(receipt.external_llm_calls, 0);
        assert_eq!(receipt.authoritative_source_write_events, 0);
    }

    #[test]
    fn diagnostic_identity_binds_the_complete_receipt_and_cannot_collide_by_sequence() {
        let original_input = input();
        let first = inspect(original_input.clone()).expect("first inspection");
        let repeated = inspect(original_input.clone()).expect("repeat inspection");
        assert_eq!(first.diagnostic_id, repeated.diagnostic_id);

        let mut next_sequence_input = original_input;
        next_sequence_input.supervisor_sequence += 1;
        let next_sequence = inspect(next_sequence_input).expect("next sequence inspection");
        assert_ne!(first.diagnostic_id, next_sequence.diagnostic_id);

        let expected_id = first.diagnostic_id.clone();
        let mut identity_payload = first;
        identity_payload.diagnostic_id.clear();
        assert_eq!(
            expected_id,
            sha256(&serde_json::to_vec(&identity_payload).unwrap())
        );
    }

    #[test]
    fn exhausted_source_synthesis_is_a_capability_gap_not_quiet_idle() {
        let mut value = input();
        value.plateau_scans = 12;
        value.source_discovery_no_candidate_streak = 3;
        value.last_source_discovery_reason = Some("BELOW_VALUE_THRESHOLD".to_string());

        let receipt = inspect(value).expect("inspect synthesis coverage");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::SourceSynthesisCoverageGap
        );
        assert_eq!(receipt.repair_disposition, RepairDisposition::CapabilityGap);
        assert_eq!(
            receipt.opportunity_kind,
            Some(ChangeOpportunityKind::CapabilityGap)
        );
        assert!(receipt.experiments[0].causal_support);
        assert!(!receipt.actionable_defect);
    }

    #[test]
    fn repeated_absence_of_an_applicable_transformation_is_a_capability_gap() {
        let mut value = input();
        value.plateau_scans = 12;
        value.source_patch_attempts = 12;
        value.source_patch_installations = 7;
        value.source_patch_rollbacks = 5;
        value.source_discovery_no_candidate_streak = 30;
        value.last_source_discovery_reason = Some("NO_APPLICABLE_TRANSFORMATION".to_string());

        let receipt = inspect(value).expect("inspect exhausted search state");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::SourceSynthesisCoverageGap
        );
        assert_eq!(receipt.repair_disposition, RepairDisposition::CapabilityGap);
        assert_eq!(
            receipt.opportunity_kind,
            Some(ChangeOpportunityKind::CapabilityGap)
        );
        assert!(!receipt.actionable_defect);
    }

    #[test]
    fn unchanged_verified_event_exposes_attribution_gap_and_activates_repair() {
        let mut value = input();
        value.pending_work_events = 1;
        value.replayed_unchanged_work_events = 1;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::WorkEventAttributionGap
        );
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
        assert!(receipt.experiments[0].causal_support);
        assert!(validate_composition_lesson(&receipt.repair_composition).is_ok());
    }

    #[test]
    fn unexercised_core_evaluator_loop_is_not_misclassified_as_quiet_idle() {
        let mut value = input();
        value.generation = 0;
        value.autonomous_campaigns_enabled = true;
        value.campaigns_started = 0;
        value.mutual_revalidation_events = 0;
        value.evaluator_challenge_cases = 6;
        value.evaluator_required_challenge_cases = 10;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::MutualRecursiveBootstrapGap
        );
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
        assert_eq!(
            receipt.opportunity_kind,
            Some(ChangeOpportunityKind::CapabilityGap)
        );
        assert!(!receipt.actionable_defect);
        assert!(receipt.experiments[0].causal_support);
        assert_eq!(receipt.core_self_approval_events, 0);
    }

    #[test]
    fn evidence_starvation_is_found_without_operator_selection() {
        let mut value = input();
        value.naive_cohort_has_verification = false;
        value.evidence_aware_cohort_has_verification = true;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::EvidenceCohortStarvation
        );
        assert!(!receipt.operator_selected_bottleneck);
        assert_eq!(receipt.human_repair_selection_events, 0);
    }

    #[test]
    fn high_reuse_slow_scan_localizes_traversal_not_hashing() {
        let mut value = input();
        value.files_scanned = 10_000;
        value.files_reused = 9_990;
        value.files_hashed = 0;
        value.scan_duration_ms = 5_000;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::ScanTraversalOverhead
        );
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::ProposalRequired
        );
        assert_eq!(
            receipt.opportunity_kind,
            Some(ChangeOpportunityKind::EfficiencyOpportunity)
        );
    }

    #[test]
    fn blocked_high_value_cohort_is_not_called_quiet_idle() {
        let mut value = input();
        value.unconsumed_high_observations = 16;
        value.cohort_preflight_ready = false;
        value.core_cohort_validation_applicable = true;
        value.repository_cohort_validation_applicable = false;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
        assert_eq!(
            receipt.repair_mechanism,
            Some(RuntimeRepairMechanism::ValidateBlockedCoreCohort)
        );
        assert!(receipt.experiments[0].causal_support);
    }

    #[test]
    fn blocked_non_core_cohort_does_not_dispatch_an_inapplicable_core_validator() {
        let mut value = input();
        value.unconsumed_high_observations = 2;
        value.cohort_preflight_ready = false;
        value.core_cohort_validation_applicable = false;
        value.repository_cohort_validation_applicable = false;

        let receipt = inspect(value).expect("inspect non-core cohort");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert_eq!(receipt.repair_disposition, RepairDisposition::CapabilityGap);
        assert_eq!(receipt.repair_mechanism, None);
    }

    #[test]
    fn blocked_repository_cohort_routes_to_bounded_native_validation() {
        let mut value = input();
        value.unconsumed_high_observations = 2;
        value.cohort_preflight_ready = false;
        value.core_cohort_validation_applicable = false;
        value.repository_cohort_validation_applicable = true;

        let receipt = inspect(value).expect("inspect repository cohort");

        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
        assert_eq!(
            receipt.repair_mechanism,
            Some(RuntimeRepairMechanism::ValidateBlockedRepositoryCohort)
        );
    }

    #[test]
    fn executable_repository_work_preempts_an_open_internal_bootstrap_action() {
        let mut value = input();
        value.generation = 0;
        value.autonomous_campaigns_enabled = true;
        value.campaigns_started = 0;
        value.mutual_revalidation_events = 0;
        value.evaluator_challenge_cases = 6;
        value.evaluator_required_challenge_cases = 10;
        value.unconsumed_high_observations = 1;
        value.cohort_preflight_ready = false;
        value.repository_cohort_validation_applicable = true;
        value.diagnostic_policy.active_action_id = Some("bootstrap-action".to_string());
        value.diagnostic_policy.active_generation = Some(0);
        value.diagnostic_policy.active_experiment_id =
            Some("FROZEN_MUTUAL_REVALIDATION_BOOTSTRAP_CANARY".to_string());

        let receipt = inspect(value).expect("inspect repository work");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert_eq!(
            receipt.repair_mechanism,
            Some(RuntimeRepairMechanism::ValidateBlockedRepositoryCohort)
        );
    }

    #[test]
    fn high_source_rollback_ratio_is_visible_to_self_inspection() {
        let mut value = input();
        value.source_patch_attempts = 20;
        value.source_patch_installations = 5;
        value.source_patch_rollbacks = 15;
        value.source_patch_consecutive_failures = 3;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::SourceRepairLowYield
        );
        assert_eq!(
            receipt.opportunity_kind,
            Some(ChangeOpportunityKind::EfficiencyOpportunity)
        );
        assert!(!receipt.actionable_defect);
        assert!(receipt.experiments[0].causal_support);
    }

    #[test]
    fn verification_cost_dominance_is_measured_separately_from_correctness() {
        let mut value = input();
        value.source_patch_validation_ms = 40 * 60 * 1_000;
        value.active_runtime_ms = 60 * 60 * 1_000;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::VerificationCostDominance
        );
        assert_eq!(receipt.repair_disposition, RepairDisposition::CapabilityGap);
    }

    #[test]
    fn unbound_diagnostic_rotates_before_same_state_is_reconsidered() {
        let mut value = input();
        value.unconsumed_high_observations = 16;
        value.cohort_preflight_ready = false;
        value.source_patch_attempts = 20;
        value.source_patch_installations = 4;
        value.source_patch_rollbacks = 16;

        let first = inspect(value.clone()).expect("first inspection");
        assert_eq!(
            first.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert!(first.hypotheses[0].policy_exploration_selected);

        assert!(value.diagnostic_policy.record(&first));
        let alternate = inspect(value.clone()).expect("same-generation alternate inspection");
        assert_eq!(
            alternate.selected_bottleneck,
            InternalBottleneckClass::SourceRepairLowYield
        );
        assert!(alternate.hypotheses[0].policy_exploration_selected);
        assert!(value.diagnostic_policy.record(&alternate));
        assert_eq!(value.diagnostic_policy.outcome_bound_selections, 0);

        assert!(!value.diagnostic_policy.resolve_frontier_outcome(
            2,
            true,
            &["unrelated-observation".to_string()]
        ));

        let reconsidered = inspect(value.clone()).expect("bounded reconsideration");
        assert_eq!(
            reconsidered.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert!(!value.diagnostic_policy.record(&reconsidered));
        assert_eq!(value.diagnostic_policy.duplicate_selection_suppressed, 1);
    }

    #[test]
    fn only_an_executed_action_with_consumed_output_receives_success_credit() {
        let mut value = input();
        value.naive_cohort_has_verification = false;
        value.evidence_aware_cohort_has_verification = true;
        let receipt = inspect(value.clone()).expect("inspect evidence routing");
        assert!(value.diagnostic_policy.record(&receipt));
        let action = RuntimeRepairActionReceipt {
            schema: "B_CORE_RUNTIME_REPAIR_ACTION_1".to_string(),
            action_id: "action-1".to_string(),
            diagnostic_id: receipt.diagnostic_id.clone(),
            generation: receipt.generation,
            mechanism: RuntimeRepairMechanism::EvidenceAwareBoundedCohortRouting,
            executed: true,
            changed_runtime_decision: true,
            execution_evidence_sha256: vec!["evidence".to_string()],
            output_observation_ids: vec!["observation-1".to_string()],
            authoritative_source_write_events: 0,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
        };
        assert!(value.diagnostic_policy.bind_executed_action(
            &receipt,
            &action,
            "receipt-sha".to_string()
        ));
        assert_eq!(value.diagnostic_policy.outcome_bound_selections, 1);
        assert!(!value.diagnostic_policy.resolve_frontier_outcome(
            2,
            true,
            &["unrelated-observation".to_string()]
        ));
        assert!(value.diagnostic_policy.resolve_consumed_action_outcome(
            2,
            true,
            &["observation-1".to_string()]
        ));
    }

    #[test]
    fn frontier_only_outcome_history_is_quarantined_from_action_goal_success() {
        let mut policy = DiagnosticPolicyMemory {
            outcome_causal_contract_revision: 2,
            outcome_bound_selections: 9,
            productive_outcome_events: 8,
            failed_outcome_events: 1,
            ..DiagnosticPolicyMemory::default()
        };

        policy.ensure_action_causal_contract();

        assert_eq!(policy.outcome_causal_contract_revision, 4);
        assert_eq!(policy.outcome_bound_selections, 0);
        assert_eq!(policy.productive_outcome_events, 0);
        assert_eq!(policy.failed_outcome_events, 0);
        assert_eq!(policy.legacy_frontier_only_outcome_bound_selections, 9);
        assert_eq!(policy.legacy_frontier_only_productive_outcome_events, 8);
        assert_eq!(policy.legacy_frontier_only_failed_outcome_events, 1);
    }

    #[test]
    fn legacy_unbound_trials_cannot_dilute_a_verified_failed_outcome() {
        let mut value = input();
        value.unconsumed_high_observations = 16;
        value.cohort_preflight_ready = false;
        value.source_patch_attempts = 20;
        value.source_patch_installations = 4;
        value.source_patch_rollbacks = 16;
        value.diagnostic_policy.experiment_records.insert(
            "MIXED_ROLE_COHORT_RECONSTRUCTION".to_string(),
            DiagnosticExperimentMemory {
                trials: 100,
                causal_support_events: 100,
                failed_outcome_events: 1,
                ..DiagnosticExperimentMemory::default()
            },
        );

        let receipt = inspect(value).expect("inspection after verified failure");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::SourceRepairLowYield
        );
        assert!(receipt.hypotheses[0].policy_exploration_selected);
    }

    #[test]
    fn non_executable_productive_history_cannot_starve_a_synthesis_gap() {
        let mut value = input();
        value.unconsumed_high_observations = 5;
        value.cohort_preflight_ready = false;
        value.core_cohort_validation_applicable = false;
        value.repository_cohort_validation_applicable = false;
        value.plateau_scans = 84;
        value.source_patch_attempts = 0;
        value.source_discovery_no_candidate_streak = 9;
        value.last_source_discovery_reason = Some("BELOW_VALUE_THRESHOLD".to_string());
        value.diagnostic_policy.experiment_records.insert(
            "MIXED_ROLE_COHORT_RECONSTRUCTION".to_string(),
            DiagnosticExperimentMemory {
                trials: 108,
                causal_support_events: 108,
                productive_outcome_events: 19,
                ..DiagnosticExperimentMemory::default()
            },
        );
        value.diagnostic_policy.experiment_records.insert(
            "SOURCE_SYNTHESIS_ADMISSIBLE_CANDIDATE_PROBE".to_string(),
            DiagnosticExperimentMemory {
                trials: 19,
                causal_support_events: 19,
                ..DiagnosticExperimentMemory::default()
            },
        );
        value.diagnostic_policy.active_experiment_id =
            Some("MIXED_ROLE_COHORT_RECONSTRUCTION".to_string());
        value.diagnostic_policy.active_generation = Some(value.generation);
        value.diagnostic_policy.active_action_id = None;

        let receipt = inspect(value).expect("runtime-like inspection");

        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::SourceSynthesisCoverageGap
        );
        assert!(receipt.hypotheses[0]
            .evidence
            .iter()
            .any(|item| item.contains("intervention_executable=false")));
    }

    #[test]
    fn repeated_unbound_capability_gap_is_counted_once_per_state() {
        let mut value = input();
        value.plateau_scans = 4;
        value.source_discovery_no_candidate_streak = 4;
        value.last_source_discovery_reason = Some("BELOW_VALUE_THRESHOLD".to_string());

        let first = inspect(value.clone()).expect("first capability gap");
        assert_eq!(first.repair_disposition, RepairDisposition::CapabilityGap);
        assert!(value.diagnostic_policy.record(&first));
        let trials_after_first = value
            .diagnostic_policy
            .experiment_records
            .get("SOURCE_SYNTHESIS_ADMISSIBLE_CANDIDATE_PROBE")
            .expect("source synthesis record")
            .trials;

        value.supervisor_sequence = value.supervisor_sequence.saturating_add(1);
        value.source_discovery_no_candidate_streak = 5;
        let repeated = inspect(value.clone()).expect("repeated capability gap");
        assert!(!value.diagnostic_policy.record(&repeated));
        let record = value
            .diagnostic_policy
            .experiment_records
            .get("SOURCE_SYNTHESIS_ADMISSIBLE_CANDIDATE_PROBE")
            .expect("source synthesis record");
        assert_eq!(record.trials, trials_after_first);
        assert_eq!(record.duplicate_unbound_state_observations, 1);
        assert_eq!(value.diagnostic_policy.duplicate_selection_suppressed, 1);
    }
}
