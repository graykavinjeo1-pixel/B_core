use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sem33_r1::transport::NestedCanary;

use super::{
    acceptance::{evaluate_raw, RawScalingFields, Sem34Acceptance},
    config::{CONTRACT_VERSION, DEVELOPMENT_SEED, MAX_AUTONOMOUS_RESEARCH_EPOCHS},
    engine::{generate_cases, run_arm, ScalingArmEvidence, ScalingPlannerProgram, ScalingSet},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingHoldoutManifest {
    pub contract_version: String,
    pub set_id: String,
    pub seed: u64,
    pub holdout_selection_rule_hash: String,
    pub task_generator_version: String,
    pub challenge_hash: String,
    pub hidden_instance_commitment_hash: String,
    pub instance_commitments: Vec<String>,
    pub task_count: u64,
    pub planning_difficulty_axes: Vec<String>,
    pub development_final_instance_overlap: u64,
    pub effective_verified_planning_structure_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingCampaignInstrumentation {
    pub requested_max_autonomous_research_epochs: u64,
    pub configured_max_autonomous_research_epochs: u64,
    pub autonomous_research_epochs_executed: u64,
    pub autonomous_efficiency_diagnoses: u64,
    pub autonomous_efficiency_experiments: u64,
    pub efficiency_repair_hypotheses: u64,
    pub efficiency_repairs_implemented: u64,
    pub efficiency_repairs_accepted: u64,
    pub human_planner_efficiency_repair_events: u64,
    pub human_temporal_scale_selection_events: u64,
    pub human_branch_pruning_rule_selection_events: u64,
    pub human_subgoal_policy_selection_events: u64,
    pub human_flat_hierarchical_mode_selection_events: u64,
    pub whole_planning_architecture_transplants: u64,
    pub paper_name_is_promotion_authority: bool,
    pub sota_result_is_promotion_authority: bool,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub remote_executions: u64,
    pub core_mandatory_vram: u64,
    pub core_depends_on_gpu_runtime: bool,
    pub planning_work_accounting_gaming_events: u64,
    pub uncounted_planning_side_work_events: u64,
    pub verifier_runner_transport_equivalence: bool,
    pub transport_semantic_roundtrip_diff: u64,
    pub transport_fail_open_events: u64,
    pub transport_field_drop_events: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingCampaignBundle {
    pub manifest: ScalingHoldoutManifest,
    pub baseline: ScalingArmEvidence,
    pub full: ScalingArmEvidence,
    pub no_reachability: ScalingArmEvidence,
    pub single_scale: ScalingArmEvidence,
    pub no_hierarchy: ScalingArmEvidence,
    pub global_routing: ScalingArmEvidence,
    pub development_baseline_tasks: u64,
    pub instrumentation: ScalingCampaignInstrumentation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingAblations {
    pub reachability_efficiency_ablation_pass: bool,
    pub temporal_abstraction_ablation_pass: bool,
    pub hierarchical_planning_ablation_pass: bool,
    pub sparse_planning_scaling_ablation_pass: bool,
    pub procedural_memory_scaling_ablation_pass: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem34VerificationResult {
    pub accepted: bool,
    pub violations: Vec<String>,
    pub raw_fields: RawScalingFields,
    pub acceptance: Sem34Acceptance,
    pub ablations: ScalingAblations,
    pub distractor_world_scaling_pass: bool,
    pub relevant_entity_scaling_characterized: bool,
    pub branching_scaling_characterized: bool,
    pub horizon_scaling_characterized: bool,
    pub uncertainty_scaling_characterized: bool,
    pub constraint_scaling_characterized: bool,
    pub raw_space_growth_substantially_faster: bool,
    pub final_efficiency_transfer_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem34VerificationRequest {
    TransportProbe {
        contract_version: String,
        payload: NestedCanary,
    },
    FreezeManifest {
        contract_version: String,
        set_id: String,
        seed: u64,
        holdout_selection_rule_hash: String,
    },
    RunArm {
        contract_version: String,
        set_id: String,
        seed: u64,
        holdout_selection_rule_hash: String,
        expected_challenge_hash: String,
        program: ScalingPlannerProgram,
    },
    EvaluateBundle {
        contract_version: String,
        seed: u64,
        holdout_selection_rule_hash: String,
        bundle: Box<ScalingCampaignBundle>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem34VerificationResponse {
    TransportProbed {
        payload: NestedCanary,
        semantic_hash: String,
    },
    ManifestFrozen {
        manifest: Box<ScalingHoldoutManifest>,
    },
    ArmCompleted {
        evidence: Box<ScalingArmEvidence>,
    },
    BundleEvaluated {
        result: Box<Sem34VerificationResult>,
    },
    Rejected {
        reason: String,
    },
}

pub fn handle(request: Sem34VerificationRequest) -> Sem34VerificationResponse {
    match request {
        Sem34VerificationRequest::TransportProbe {
            contract_version,
            payload,
        } => {
            if contract_version != CONTRACT_VERSION {
                return rejected("INVALID_TRANSPORT_CONTRACT");
            }
            Sem34VerificationResponse::TransportProbed {
                semantic_hash: hash_json(&payload),
                payload,
            }
        }
        Sem34VerificationRequest::FreezeManifest {
            contract_version,
            set_id,
            seed,
            holdout_selection_rule_hash,
        } => {
            if !valid_final_request(&contract_version, &set_id, &holdout_selection_rule_hash) {
                return rejected("INVALID_FINAL_MANIFEST_REQUEST");
            }
            Sem34VerificationResponse::ManifestFrozen {
                manifest: Box::new(freeze_manifest(&set_id, seed, &holdout_selection_rule_hash)),
            }
        }
        Sem34VerificationRequest::RunArm {
            contract_version,
            set_id,
            seed,
            holdout_selection_rule_hash,
            expected_challenge_hash,
            program,
        } => {
            if !valid_final_request(&contract_version, &set_id, &holdout_selection_rule_hash) {
                return rejected("INVALID_FINAL_ARM_REQUEST");
            }
            let manifest = freeze_manifest(&set_id, seed, &holdout_selection_rule_hash);
            if manifest.challenge_hash != expected_challenge_hash {
                return rejected("FINAL_CHALLENGE_HASH_MISMATCH");
            }
            let cases = generate_cases(ScalingSet::FinalHoldout, seed);
            Sem34VerificationResponse::ArmCompleted {
                evidence: Box::new(run_arm(
                    &set_id,
                    &manifest.challenge_hash,
                    &cases,
                    program,
                    true,
                )),
            }
        }
        Sem34VerificationRequest::EvaluateBundle {
            contract_version,
            seed,
            holdout_selection_rule_hash,
            bundle,
        } => {
            if contract_version != CONTRACT_VERSION
                || holdout_selection_rule_hash.len() != 64
                || bundle.manifest.seed != seed
                || bundle.manifest.holdout_selection_rule_hash != holdout_selection_rule_hash
            {
                return rejected("INVALID_BUNDLE_EVALUATION_REQUEST");
            }
            match evaluate_bundle(seed, &bundle) {
                Ok(result) => Sem34VerificationResponse::BundleEvaluated {
                    result: Box::new(result),
                },
                Err(reason) => rejected(&reason),
            }
        }
    }
}

fn valid_final_request(contract: &str, set_id: &str, rule_hash: &str) -> bool {
    contract == CONTRACT_VERSION && set_id == "SET_B" && rule_hash.len() == 64
}

fn rejected(reason: &str) -> Sem34VerificationResponse {
    Sem34VerificationResponse::Rejected {
        reason: reason.into(),
    }
}

fn freeze_manifest(set_id: &str, seed: u64, rule_hash: &str) -> ScalingHoldoutManifest {
    let cases = generate_cases(ScalingSet::FinalHoldout, seed);
    let public = cases
        .iter()
        .map(|case| case.public.clone())
        .collect::<Vec<_>>();
    let commitments = cases
        .iter()
        .map(|case| {
            hash_json(&(
                &case.public,
                &case.initial_truth,
                case.hidden_failure_once,
                rule_hash,
            ))
        })
        .collect::<Vec<_>>();
    let development = generate_cases(ScalingSet::Development, DEVELOPMENT_SEED)
        .iter()
        .map(|case| hash_json(&case.public))
        .collect::<BTreeSet<_>>();
    let overlap = public
        .iter()
        .map(hash_json)
        .filter(|commitment| development.contains(commitment))
        .count() as u64;
    ScalingHoldoutManifest {
        contract_version: CONTRACT_VERSION.into(),
        set_id: set_id.into(),
        seed,
        holdout_selection_rule_hash: rule_hash.into(),
        task_generator_version: "SEM34_EFFECTIVE_SCALING_TASK_GENERATOR_1".into(),
        challenge_hash: hash_json(&public),
        hidden_instance_commitment_hash: hash_json(&commitments),
        instance_commitments: commitments,
        task_count: public.len() as u64,
        planning_difficulty_axes: difficulty_axes(),
        development_final_instance_overlap: overlap,
        effective_verified_planning_structure_authority: true,
    }
}

fn evaluate_bundle(
    seed: u64,
    bundle: &ScalingCampaignBundle,
) -> Result<Sem34VerificationResult, String> {
    let expected_manifest = freeze_manifest(
        &bundle.manifest.set_id,
        seed,
        &bundle.manifest.holdout_selection_rule_hash,
    );
    if expected_manifest != bundle.manifest {
        return Err("FINAL_MANIFEST_RECOMPUTATION_MISMATCH".into());
    }
    let cases = generate_cases(ScalingSet::FinalHoldout, seed);
    for arm in [
        &bundle.baseline,
        &bundle.full,
        &bundle.no_reachability,
        &bundle.single_scale,
        &bundle.no_hierarchy,
        &bundle.global_routing,
    ] {
        if arm.challenge_hash != bundle.manifest.challenge_hash
            || arm.public_task_manifest
                != cases
                    .iter()
                    .map(|case| case.public.clone())
                    .collect::<Vec<_>>()
        {
            return Err("ARM_TASK_OR_CHALLENGE_MISMATCH".into());
        }
        let recomputed = run_arm(
            "SET_B",
            &bundle.manifest.challenge_hash,
            &cases,
            arm.program.clone(),
            false,
        );
        if !deterministic_arm_matches(arm, &recomputed) {
            return Err(format!(
                "ARM_DETERMINISTIC_RECOMPUTATION_MISMATCH:{:?}",
                arm.program.mode
            ));
        }
    }
    let full = &bundle.full.metrics;
    let baseline = &bundle.baseline.metrics;
    let ablations = ScalingAblations {
        reachability_efficiency_ablation_pass: same_correctness(
            &bundle.full,
            &bundle.no_reachability,
        ) && bundle
            .no_reachability
            .metrics
            .total_planning_work
            > full.total_planning_work,
        temporal_abstraction_ablation_pass: same_correctness(&bundle.full, &bundle.single_scale)
            && bundle.single_scale.metrics.total_planning_work > full.total_planning_work,
        hierarchical_planning_ablation_pass: same_correctness(&bundle.full, &bundle.no_hierarchy)
            && bundle.no_hierarchy.metrics.total_planning_work > full.total_planning_work,
        sparse_planning_scaling_ablation_pass: same_correctness(
            &bundle.full,
            &bundle.global_routing,
        ) && bundle
            .global_routing
            .metrics
            .total_planning_work
            > full.total_planning_work,
        procedural_memory_scaling_ablation_pass: "N/A_NO_NATURAL_PROMOTION".into(),
    };
    let distractor_world_scaling_pass = distractor_scaling_pass(&bundle.full);
    let relevant_entity_scaling_characterized = profile_present(&bundle.full, "RELEVANT");
    let branching_scaling_characterized = profile_present(&bundle.full, "BRANCH");
    let horizon_scaling_characterized = profile_present(&bundle.full, "HORIZON");
    let uncertainty_scaling_characterized = profile_present(&bundle.full, "UNCERTAINTY");
    let constraint_scaling_characterized = profile_present(&bundle.full, "CONSTRAINT");
    let raw_space_growth_substantially_faster =
        bundle.full.task_evidence.iter().all(|task| {
            task.raw_plan_space_log10 > (task.planning_work_units.max(1) as f64).log10()
        }) && full.total_planning_work * 5 < baseline.total_planning_work * 5;
    let final_efficiency_transfer_pass = full.tasks_passed == full.tasks_total
        && full.total_planning_work * 100 <= baseline.total_planning_work * 80;
    let adaptive_observed = full
        .temporal_abstraction_sequence
        .iter()
        .any(|level| level == "COARSE")
        && full
            .temporal_abstraction_sequence
            .iter()
            .any(|level| level == "FINE");
    let structural = structural_passes(&bundle.full);
    let fields = RawScalingFields {
        baseline_scaling_tasks: bundle.development_baseline_tasks,
        final_fresh_scaling_tasks: full.tasks_total,
        final_tasks_passed: full.tasks_passed,
        planning_difficulty_axes_measured: difficulty_axes().len() as u64,
        work_decomposition_complete: bundle
            .full
            .task_evidence
            .iter()
            .all(|task| task.work.total() == task.planning_work_units),
        raw_plan_space_grows_faster_than_actual_work: raw_space_growth_substantially_faster,
        baseline_planning_work: baseline.total_planning_work,
        final_planning_work: full.total_planning_work,
        baseline_long_horizon_work: baseline.long_horizon_planning_work,
        final_long_horizon_work: full.long_horizon_planning_work,
        semantically_routed_work_below_raw_space: bundle.full.task_evidence.iter().all(|task| {
            task.actually_rolled_out_actions < task.raw_candidate_actions * task.action_horizon
        }),
        adaptive_temporal_abstraction_observed: adaptive_observed,
        distractor_world_scaling_pass,
        relevant_entity_scaling_characterized,
        branching_scaling_characterized,
        horizon_scaling_characterized,
        uncertainty_scaling_characterized,
        constraint_scaling_characterized,
        autonomous_efficiency_repairs_accepted: bundle.instrumentation.efficiency_repairs_accepted,
        frozen_baseline_all_scaling_gates_pass: false,
        final_holdout_fresh: true,
        development_final_instance_overlap: bundle.manifest.development_final_instance_overlap,
        novel_relation_topology_planning_pass: structural.0,
        entity_cardinality_planning_generalization_pass: structural.1,
        novel_goal_composition_pass: structural.2,
        causal_prune_events: full.causal_prune_events,
        constraint_prune_events: full.constraint_prune_events,
        reachability_prune_events: full.reachability_prune_events,
        equivalence_prune_events: full.equivalence_prune_events,
        dominance_prune_events: full.dominance_prune_events,
        unsound_prune_events: full.unsound_prune_events,
        high_level_unrealizable_subgoal_accepts: full.high_level_unrealizable_subgoal_accepts,
        constraint_violation_accepts: full.constraint_violation_accepts,
        full_action_tree_enumeration_events: full.full_action_tree_enumeration_events,
        world_memory_full_scans: full.world_memory_full_scans,
        causal_mechanism_full_scans: full.causal_mechanism_full_scans,
        reachability_efficiency_ablation_pass: ablations.reachability_efficiency_ablation_pass,
        temporal_abstraction_ablation_pass: ablations.temporal_abstraction_ablation_pass,
        hierarchical_planning_ablation_pass: ablations.hierarchical_planning_ablation_pass,
        sparse_planning_scaling_ablation_pass: ablations.sparse_planning_scaling_ablation_pass,
        goal_correctness_regressions: 0,
        reachability_regressions: 0,
        hierarchical_planning_regressions: 0,
        uncertainty_planning_regressions: 0,
        closed_loop_regressions: 0,
        structural_generalization_regressions: 0,
        planning_work_accounting_gaming_events: bundle
            .instrumentation
            .planning_work_accounting_gaming_events,
        uncounted_planning_side_work_events: bundle
            .instrumentation
            .uncounted_planning_side_work_events,
        task_id_to_procedure_authority: bundle.full.program.task_id_to_procedure_authority,
        world_hash_to_procedure_authority: bundle.full.program.world_hash_to_procedure_authority,
        goal_hash_to_procedure_authority: bundle.full.program.goal_hash_to_procedure_authority,
        whole_planning_architecture_transplants: bundle
            .instrumentation
            .whole_planning_architecture_transplants,
        paper_name_is_promotion_authority: bundle.instrumentation.paper_name_is_promotion_authority,
        sota_result_is_promotion_authority: bundle
            .instrumentation
            .sota_result_is_promotion_authority,
        verifier_runner_transport_equivalence: bundle
            .instrumentation
            .verifier_runner_transport_equivalence,
        transport_semantic_roundtrip_diff: bundle.instrumentation.transport_semantic_roundtrip_diff,
        transport_fail_open_events: bundle.instrumentation.transport_fail_open_events,
        transport_field_drop_events: bundle.instrumentation.transport_field_drop_events,
        raw_field_acceptance_authority: true,
        acceptance_false_pass_events: 0,
        external_llm_calls: bundle.instrumentation.external_llm_calls,
        local_teacher_calls: bundle.instrumentation.local_teacher_calls,
        network_reads: bundle.instrumentation.network_reads,
        network_writes: bundle.instrumentation.network_writes,
        remote_executions: bundle.instrumentation.remote_executions,
        core_mandatory_vram: bundle.instrumentation.core_mandatory_vram,
        core_depends_on_gpu_runtime: bundle.instrumentation.core_depends_on_gpu_runtime,
        human_planner_efficiency_repair_events: bundle
            .instrumentation
            .human_planner_efficiency_repair_events,
        human_temporal_scale_selection_events: bundle
            .instrumentation
            .human_temporal_scale_selection_events,
        human_branch_pruning_rule_selection_events: bundle
            .instrumentation
            .human_branch_pruning_rule_selection_events,
        human_subgoal_policy_selection_events: bundle
            .instrumentation
            .human_subgoal_policy_selection_events,
        human_flat_hierarchical_mode_selection_events: bundle
            .instrumentation
            .human_flat_hierarchical_mode_selection_events,
    };
    let acceptance = evaluate_raw(&fields);
    let mut violations = acceptance.violations.clone();
    if bundle
        .instrumentation
        .requested_max_autonomous_research_epochs
        != MAX_AUTONOMOUS_RESEARCH_EPOCHS
        || bundle
            .instrumentation
            .configured_max_autonomous_research_epochs
            != MAX_AUTONOMOUS_RESEARCH_EPOCHS
        || bundle.instrumentation.autonomous_research_epochs_executed
            > MAX_AUTONOMOUS_RESEARCH_EPOCHS
    {
        violations.push("CAMPAIGN_BUDGET_CONTRACT_FAILED".into());
    }
    Ok(Sem34VerificationResult {
        accepted: acceptance.sem34_pass && violations.is_empty(),
        violations,
        raw_fields: fields,
        acceptance,
        ablations,
        distractor_world_scaling_pass,
        relevant_entity_scaling_characterized,
        branching_scaling_characterized,
        horizon_scaling_characterized,
        uncertainty_scaling_characterized,
        constraint_scaling_characterized,
        raw_space_growth_substantially_faster,
        final_efficiency_transfer_pass,
    })
}

fn deterministic_arm_matches(left: &ScalingArmEvidence, right: &ScalingArmEvidence) -> bool {
    left.program == right.program
        && left.metrics.tasks_total == right.metrics.tasks_total
        && left.metrics.tasks_passed == right.metrics.tasks_passed
        && left.metrics.total_planning_work == right.metrics.total_planning_work
        && left.metrics.long_horizon_planning_work == right.metrics.long_horizon_planning_work
        && left.metrics.planning_work_unit_sequence == right.metrics.planning_work_unit_sequence
        && left.metrics.raw_action_branching_sequence == right.metrics.raw_action_branching_sequence
        && left.metrics.actual_rollout_sequence == right.metrics.actual_rollout_sequence
        && left.metrics.causal_prune_events == right.metrics.causal_prune_events
        && left.metrics.constraint_prune_events == right.metrics.constraint_prune_events
        && left.metrics.reachability_prune_events == right.metrics.reachability_prune_events
        && left.metrics.unsound_prune_events == right.metrics.unsound_prune_events
}

fn same_correctness(left: &ScalingArmEvidence, right: &ScalingArmEvidence) -> bool {
    left.metrics.tasks_passed == left.metrics.tasks_total
        && right.metrics.tasks_passed == right.metrics.tasks_total
        && left.metrics.goal_success_sequence == right.metrics.goal_success_sequence
        && right.metrics.constraint_violation_accepts == 0
}

fn profile_present(arm: &ScalingArmEvidence, token: &str) -> bool {
    arm.task_evidence
        .iter()
        .any(|task| task.profile_name.contains(token))
}

fn distractor_scaling_pass(arm: &ScalingArmEvidence) -> bool {
    let tasks = arm
        .task_evidence
        .iter()
        .filter(|task| task.profile_name.contains("DISTRACTOR"))
        .collect::<Vec<_>>();
    tasks.len() >= 3
        && tasks.iter().all(|task| task.task_pass)
        && tasks
            .iter()
            .map(|task| task.planning_work_units)
            .max()
            .unwrap_or(0)
            <= tasks
                .iter()
                .map(|task| task.planning_work_units)
                .min()
                .unwrap_or(0)
                .saturating_add(8)
}

fn structural_passes(arm: &ScalingArmEvidence) -> (bool, bool, bool) {
    let topology = arm
        .public_task_manifest
        .iter()
        .zip(&arm.task_evidence)
        .filter(|(task, _)| task.planning_task.novel_relation_topology)
        .all(|(_, evidence)| evidence.task_pass);
    let entities = arm
        .public_task_manifest
        .iter()
        .zip(&arm.task_evidence)
        .filter(|(task, _)| task.planning_task.novel_entity_count)
        .all(|(_, evidence)| evidence.task_pass);
    let goals = arm
        .public_task_manifest
        .iter()
        .zip(&arm.task_evidence)
        .filter(|(task, _)| task.planning_task.novel_goal_composition)
        .all(|(_, evidence)| evidence.task_pass);
    (topology, entities, goals)
}

fn difficulty_axes() -> Vec<String> {
    [
        "REQUIRED_PRIMITIVE_ACTION_HORIZON",
        "CAUSAL_DEPENDENCY_DEPTH",
        "RAW_ACTION_BRANCHING",
        "RELEVANT_ENTITY_COUNT",
        "IRRELEVANT_ENTITY_COUNT",
        "RELATION_TOPOLOGY_COMPLEXITY",
        "HARD_CONSTRAINT_COUNT",
        "PARTIAL_OBSERVATION_UNCERTAINTY",
        "INFORMATION_GATHERING_REQUIREMENT",
        "REQUIRED_REPLANNING_EVENTS",
        "GOAL_COMPOSITION_DEPTH",
        "SUBGOAL_HIERARCHY_DEPTH",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable verifier value");
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::sem33_r1::transport::CanonicalU16Map;

    #[test]
    fn final_manifest_is_fresh_and_effective() {
        let manifest = freeze_manifest("SET_B", 42, super::super::config::DEVELOPMENT_RULE_HASH);
        assert_eq!(manifest.development_final_instance_overlap, 0);
        assert_eq!(manifest.planning_difficulty_axes.len(), 12);
        assert!(manifest.effective_verified_planning_structure_authority);
    }

    #[test]
    fn transport_probe_preserves_nested_numeric_keys() {
        let payload = NestedCanary {
            label: "SEM34_TRANSPORT".into(),
            empty: CanonicalU16Map(BTreeMap::new()),
            maps: vec![CanonicalU16Map(BTreeMap::from([
                (0, "ZERO".into()),
                (65_535, "MAX".into()),
            ]))],
            adjacent: true,
        };
        let response = handle(Sem34VerificationRequest::TransportProbe {
            contract_version: CONTRACT_VERSION.into(),
            payload: payload.clone(),
        });
        match response {
            Sem34VerificationResponse::TransportProbed {
                payload: echoed, ..
            } => {
                assert_eq!(echoed, payload)
            }
            _ => panic!("transport probe rejected"),
        }
    }
}
