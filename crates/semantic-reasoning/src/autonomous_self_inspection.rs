//! Evidence-driven, bounded inspection of the always-on growth loop itself.
//!
//! This module deliberately separates "nothing new happened" from a real
//! operational bottleneck.  It ranks only mechanically observed hypotheses,
//! runs small counterfactual diagnostics over the frozen telemetry, and emits
//! a repair route assembled from existing local capabilities.  It never reads
//! source text, writes a patch, calls a model/network, or approves a repair.

use serde::{Deserialize, Serialize};

use crate::self_healing_pipeline::{
    validate_composition_lesson, CompositionEdgeIR, RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;

pub const SELF_INSPECTION_SCHEMA: &str = "B_CORE_AUTONOMOUS_SELF_INSPECTION_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InternalBottleneckClass {
    WorkEventAttributionGap,
    EvidenceCohortStarvation,
    ScanTraversalOverhead,
    RepeatedVerificationFailure,
    QuietIdle,
}

impl InternalBottleneckClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkEventAttributionGap => "WORK_EVENT_ATTRIBUTION_GAP",
            Self::EvidenceCohortStarvation => "EVIDENCE_COHORT_STARVATION",
            Self::ScanTraversalOverhead => "SCAN_TRAVERSAL_OVERHEAD",
            Self::RepeatedVerificationFailure => "REPEATED_VERIFICATION_FAILURE",
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
    pub consecutive_failures: u32,
    pub plateau_scans: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalHypothesis {
    pub bottleneck: InternalBottleneckClass,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub selected: bool,
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
        });
    }
    if input.consecutive_failures >= 2 {
        hypotheses.push(InternalHypothesis {
            bottleneck: InternalBottleneckClass::RepeatedVerificationFailure,
            confidence_millis: 700,
            evidence: vec![format!(
                "consecutive_failures={}",
                input.consecutive_failures
            )],
            selected: false,
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
        });
    }
    hypotheses.sort_by_key(|hypothesis| std::cmp::Reverse(hypothesis.confidence_millis));
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
        InternalBottleneckClass::ScanTraversalOverhead => (
            RepairDisposition::ProposalRequired,
            Some("DIRECTORY_SNAPSHOT_OR_WATCHER_CANDIDATE_REQUIRES_CANARY".to_string()),
            true,
        ),
        InternalBottleneckClass::RepeatedVerificationFailure => {
            (RepairDisposition::CapabilityGap, None, true)
        }
        InternalBottleneckClass::QuietIdle => (RepairDisposition::SafeWait, None, false),
    };
    let diagnostic_id = sha256(
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            selected.label(),
            input.files_scanned,
            input.files_reused,
            input.files_hashed,
            input.pending_work_events,
            input.replayed_unchanged_work_events,
            input.consecutive_failures
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
            consecutive_failures: 0,
            plateau_scans: 3,
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
}
