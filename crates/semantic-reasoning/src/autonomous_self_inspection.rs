//! Evidence-driven, bounded inspection of the always-on growth loop itself.
//!
//! This module deliberately separates "nothing new happened" from a real
//! operational bottleneck.  It ranks only mechanically observed hypotheses,
//! runs small counterfactual diagnostics over the frozen telemetry, and emits
//! a repair route assembled from existing local capabilities.  It never reads
//! source text, writes a patch, calls a model/network, or approves a repair.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl DiagnosticPolicyMemory {
    const MAX_ACTIVE_OBSERVATIONS_WITHOUT_OUTCOME: u32 = 8;

    fn resolve_active(&mut self, productive: bool) -> bool {
        let Some(experiment_id) = self.active_experiment_id.take() else {
            return false;
        };
        let Some(record) = self.experiment_records.get_mut(&experiment_id) else {
            self.active_generation = None;
            self.active_observations = 0;
            self.active_causal_support = false;
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
        true
    }

    pub fn resolve_frontier_outcome(
        &mut self,
        source_generation: u64,
        frontier_advance: bool,
    ) -> bool {
        if self.active_generation != Some(source_generation) {
            return false;
        }
        self.resolve_active(frontier_advance)
    }

    pub fn record(&mut self, receipt: &AutonomousSelfInspectionReceipt) -> bool {
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
                self.resolve_active(false);
            }
            return false;
        }
        if self.active_experiment_id.is_some() {
            self.resolve_active(false);
        }
        let record = self
            .experiment_records
            .entry(experiment.experiment_id.clone())
            .or_default();
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
        self.outcome_bound_selections = self.outcome_bound_selections.saturating_add(1);
        if receipt
            .hypotheses
            .iter()
            .find(|hypothesis| hypothesis.selected)
            .map(|hypothesis| hypothesis.policy_exploration_selected)
            .unwrap_or(false)
        {
            self.exploration_selections = self.exploration_selections.saturating_add(1);
        }
        self.active_experiment_id = Some(experiment.experiment_id.clone());
        self.active_generation = Some(receipt.generation);
        self.active_observations = 1;
        self.active_causal_support = experiment.causal_support;
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
    pub source_patch_attempts: u64,
    pub source_patch_installations: u64,
    pub source_patch_rollbacks: u64,
    pub source_patch_consecutive_failures: u32,
    pub source_patch_validation_ms: u64,
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
    pub repair_mechanism: Option<String>,
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
        let active_bonus = if input.diagnostic_policy.active_generation == Some(input.generation)
            && input.diagnostic_policy.active_experiment_id.as_deref()
                == Some(experiment.experiment_id.as_str())
        {
            1_000_u32
        } else {
            0
        };
        hypothesis.policy_score_millis = u32::from(hypothesis.confidence_millis)
            .saturating_add(exploration_bonus)
            .saturating_add(support_bonus)
            .saturating_add(productive_bonus)
            .saturating_add(active_bonus)
            .saturating_sub(repetition_penalty)
            .saturating_sub(unsupported_penalty)
            .saturating_sub(failed_outcome_penalty)
            .min(u32::from(u16::MAX)) as u16;
        hypothesis.prior_policy_trials = record.trials;
        hypothesis.prior_causal_support_events = record.causal_support_events;
        hypothesis.policy_exploration_selected = record.trials == 0;
        hypothesis.evidence.push(format!(
            "adaptive_policy:experiment={};trials={};causal_support={};verified_outcomes={};productive={};failed={};active={};score={}",
            experiment.experiment_id,
            record.trials,
            record.causal_support_events,
            verified_outcomes,
            record.productive_outcome_events,
            record.failed_outcome_events,
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
            intervention_observable:
                "recognize implementation evidence inside production files that also contain tests"
                    .to_string(),
            causal_support: input.unconsumed_high_observations > 0
                && !input.cohort_preflight_ready,
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
    let (disposition, repair_mechanism, actionable_defect) = match selected {
        InternalBottleneckClass::WorkEventAttributionGap => (
            RepairDisposition::RuntimeRepairActive,
            Some("REPLAY_VERIFIED_EVENT_AGAINST_INDEXED_CONTENT".to_string()),
            true,
        ),
        InternalBottleneckClass::EvidenceCohortStarvation => (
            RepairDisposition::RuntimeRepairActive,
            Some("EVIDENCE_AWARE_BOUNDED_COHORT_ROUTING".to_string()),
            true,
        ),
        InternalBottleneckClass::MutualRecursiveBootstrapGap => (
            RepairDisposition::RuntimeRepairActive,
            Some("BOOTSTRAP_FROZEN_CORE_EVALUATOR_CANARY".to_string()),
            true,
        ),
        InternalBottleneckClass::ScanTraversalOverhead => (
            RepairDisposition::ProposalRequired,
            Some("DIRECTORY_SNAPSHOT_OR_WATCHER_CANDIDATE_REQUIRES_CANARY".to_string()),
            true,
        ),
        InternalBottleneckClass::RepeatedVerificationFailure => {
            (RepairDisposition::CapabilityGap, None, true)
        }
        InternalBottleneckClass::CampaignCohortBlocked => (
            RepairDisposition::RuntimeRepairActive,
            Some("MIXED_PRODUCTION_FILE_IMPLEMENTATION_EVIDENCE_ROUTING".to_string()),
            true,
        ),
        InternalBottleneckClass::SourceRepairLowYield => (
            RepairDisposition::RuntimeRepairActive,
            Some("PREDICTED_UTILITY_GATE_BEFORE_SOURCE_VALIDATION".to_string()),
            true,
        ),
        InternalBottleneckClass::VerificationCostDominance => (
            RepairDisposition::RuntimeRepairActive,
            Some("STAGED_COMPILE_CHECK_BEFORE_FULL_REGRESSION".to_string()),
            true,
        ),
        InternalBottleneckClass::QuietIdle => (RepairDisposition::SafeWait, None, false),
    };
    let diagnostic_id = sha256(
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            selected.label(),
            input.files_scanned,
            input.files_reused,
            input.files_hashed,
            input.pending_work_events,
            input.replayed_unchanged_work_events,
            input.consecutive_failures,
            input.campaigns_started,
            input.mutual_revalidation_events,
            input.evaluator_challenge_cases,
            input.unconsumed_high_observations,
            input.source_patch_attempts,
            input.source_patch_rollbacks,
            input.source_patch_consecutive_failures,
            input.source_patch_validation_ms
        )
        .as_bytes(),
    );
    Ok(AutonomousSelfInspectionReceipt {
        schema: SELF_INSPECTION_SCHEMA.to_string(),
        diagnostic_id,
        generation: input.generation,
        supervisor_sequence: input.supervisor_sequence,
        hypotheses,
        selected_bottleneck: selected,
        experiments: vec![experiment],
        repair_disposition: disposition,
        repair_mechanism,
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
    })
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
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
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
        assert!(!receipt.actionable_defect);
        assert_eq!(receipt.external_llm_calls, 0);
        assert_eq!(receipt.authoritative_source_write_events, 0);
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
        assert!(receipt.actionable_defect);
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
    }

    #[test]
    fn blocked_high_value_cohort_is_not_called_quiet_idle() {
        let mut value = input();
        value.unconsumed_high_observations = 16;
        value.cohort_preflight_ready = false;
        let receipt = inspect(value).expect("inspect");
        assert_eq!(
            receipt.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
        assert!(receipt.experiments[0].causal_support);
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
        assert!(receipt.actionable_defect);
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
        assert_eq!(
            receipt.repair_disposition,
            RepairDisposition::RuntimeRepairActive
        );
    }

    #[test]
    fn verified_diagnostic_history_changes_the_next_bottleneck_choice() {
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
        let repeated = inspect(value.clone()).expect("same-generation inspection");
        assert_eq!(
            repeated.selected_bottleneck,
            InternalBottleneckClass::CampaignCohortBlocked
        );
        assert!(!value.diagnostic_policy.record(&repeated));
        assert_eq!(value.diagnostic_policy.outcome_bound_selections, 1);
        assert_eq!(value.diagnostic_policy.duplicate_selection_suppressed, 1);

        assert!(value.diagnostic_policy.resolve_frontier_outcome(2, true));
        value.generation = 3;
        let next_generation = inspect(value).expect("next-generation inspection");
        assert_eq!(
            next_generation.selected_bottleneck,
            InternalBottleneckClass::SourceRepairLowYield
        );
        assert!(next_generation.hypotheses[0].policy_exploration_selected);
        assert_eq!(next_generation.hypotheses[0].prior_policy_trials, 0);
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
}
