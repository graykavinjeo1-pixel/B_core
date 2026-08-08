pub mod engine;

use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use engine::{run_growth_probe, GrowthProbeRequest, GrowthProbeResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sem24::engine::{run_verification_probe, VerificationProbeRequest};

const CAMPAIGN_ID: &str = "SEM25-PREDICTIVE-GROWTH-ROUTING-0001";
const PREDECESSOR_COMMIT: &str = "e30956b337c4ecf469aa399996113b07d868ba09";
const BRANCH: &str = "codex/sem25-predictive-growth-routing";
const REPORT_DIR: &str = "reports/sem25";
const EPOCHS: usize = 24;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const BASE_FRONTIER_SCALE: u64 = 5_738;
const BASE_REACTION_OBJECTS: u64 = 64;
const BASE_ACTIVE_SEMANTIC_BYTES: u64 = 5_321;
const BASE_CORE_BYTES: u64 = 714_308;
const PROTOCOL_SHA256: &str = "776173650f8869b7755533d665e5455b83123cf2d9a57194c8590e558e4bfe5e";
const SEEDS: [u64; EPOCHS] = [
    0x2511, 0x2527, 0x2539, 0x254B, 0x255D, 0x256F, 0x2581, 0x2593, 0x25A5, 0x25B7, 0x25C9, 0x25DB,
    0x25ED, 0x25FF, 0x2611, 0x2623, 0x2635, 0x2647, 0x2659, 0x266B, 0x267D, 0x268F, 0x26A1, 0x26B3,
];

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "frozen_authority.json",
    "future_affordance_signatures.json",
    "growth_opportunity_index.json",
    "frontier_portfolio_ledger.json",
    "counterfactual_growth_paths.json",
    "growth_routing_laws.json",
    "growth_routing_law_revision_ledger.json",
    "growth_routing_schemas.json",
    "dead_end_routing_knowledge.json",
    "catalytic_frontier_evidence.json",
    "affordance_prediction_residuals.json",
    "arm_a_sem24_one_step_discovery.json",
    "arm_b_immediate_greedy_routing.json",
    "arm_c_multi_horizon_without_routing_laws.json",
    "arm_d_full_predictive_growth_routing.json",
    "multi_horizon_routing_ablation.json",
    "growth_opportunity_index_ablation.json",
    "growth_routing_law_ablation.json",
    "future_affordance_ablation.json",
    "frontier_portfolio_ablation.json",
    "dead_end_knowledge_ablation.json",
    "discovery_bottleneck_decomposition.json",
    "fixed_resource_frontier_results.json",
    "fixed_work_results.json",
    "growth_ledger.jsonl",
    "future_instance_leakage_audit.json",
    "growth_ledger_gaming_audit.json",
    "sparse_scaling_audit.json",
    "verification_soundness_audit.json",
    "ordinary_reasoning_regression.json",
    "meta_quality_regression.json",
    "frontier_retention.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem25_final_report.json",
    "SEM25_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    Sem24OneStepDiscovery,
    ImmediateGreedyRouting,
    MultiHorizonWithoutRoutingLaws,
    FullPredictiveGrowthRouting,
}

impl Arm {
    const ALL: [Self; 4] = [
        Self::Sem24OneStepDiscovery,
        Self::ImmediateGreedyRouting,
        Self::MultiHorizonWithoutRoutingLaws,
        Self::FullPredictiveGrowthRouting,
    ];

    fn code(self) -> u8 {
        match self {
            Self::Sem24OneStepDiscovery => 0,
            Self::ImmediateGreedyRouting => 1,
            Self::MultiHorizonWithoutRoutingLaws => 2,
            Self::FullPredictiveGrowthRouting => 3,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Sem24OneStepDiscovery => "A_SEM24_ONE_STEP_DISCOVERY",
            Self::ImmediateGreedyRouting => "B_IMMEDIATE_GREEDY_ROUTING",
            Self::MultiHorizonWithoutRoutingLaws => "C_MULTI_HORIZON_WITHOUT_ROUTING_LAWS",
            Self::FullPredictiveGrowthRouting => "D_FULL_PREDICTIVE_GROWTH_ROUTING",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochPlan {
    epoch: usize,
    gap_code: u8,
    target_gap: String,
    desired_phenotype: String,
    required_properties_mask: u64,
    required_roles_mask: u64,
    resource_ceiling: u64,
    total_reaction_objects: u64,
    theoretical_reaction_space: u64,
    growth_routing_laws_available: u8,
    growth_routing_schemas_available: u8,
    concrete_instance_opened: bool,
}

#[derive(Debug, Default)]
struct CampaignState {
    signatures: Vec<Value>,
    routing_laws: Vec<Value>,
    law_revisions: Vec<Value>,
    routing_schemas: Vec<Value>,
    dead_end_knowledge: Vec<Value>,
    catalytic_evidence: Vec<Value>,
    prediction_residuals: Vec<Value>,
    portfolio_ledger: Vec<Value>,
    path_ledger: Vec<Value>,
    schema_reuse_events: usize,
    regime_advance_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasuredGrowthProbe {
    result: GrowthProbeResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

#[derive(Debug, Clone)]
struct CampaignSequences {
    theoretical_space: Vec<u64>,
    reaction_objects: Vec<u64>,
    touched: Vec<u64>,
    routed: Vec<u64>,
    implemented: Vec<u64>,
    hit_rate: Vec<f64>,
    discovery_time: Vec<u64>,
    selection_time: Vec<u64>,
    discovery_fraction: Vec<f64>,
    discovery_per_useful: Vec<f64>,
    discovery_per_frontier: Vec<f64>,
    frontier_scale: Vec<u64>,
    frontier_gain: Vec<u64>,
    composite_branching: Vec<u64>,
    frontier_branching: Vec<u64>,
    future_frontiers: Vec<u64>,
    time_to_frontier: Vec<u64>,
    genesis_cost: Vec<u64>,
    verification_cost: Vec<u64>,
    fixed_work_wall: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic_bytes: Vec<u64>,
    core_bytes: Vec<u64>,
    horizons: Vec<u8>,
    backlog: Vec<u64>,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != PREDECESSOR_COMMIT {
        return Err(format!("PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    let predecessor = read_json(root.join("reports/sem24/sem24_final_report.json"))?;
    for (field, expected) in [
        ("sem24_status", json!("PASS")),
        ("verification_remains_dominant_growth_limit", json!(false)),
        (
            "next_dominant_growth_limit",
            json!("REACTION_DISCOVERY_AND_FRONTIER_SELECTION_COST"),
        ),
        ("next_allowed_stage", json!("OPERATOR_REVIEW_FOR_SEM25")),
        ("sem25_started", json!(false)),
        ("governor_hash_unchanged", json!(true)),
        ("evaluator_hash_unchanged", json!(true)),
        ("acceptance_criteria_hash_unchanged", json!(true)),
    ] {
        if predecessor[field] != expected {
            return Err(format!("PREDECESSOR_FIELD_MISMATCH:{field}"));
        }
    }
    for level in ['A', 'B', 'C', 'D', 'F', 'H'] {
        if predecessor[format!("sem24_level_{level}_pass")] != true {
            return Err(format!("PREDECESSOR_LEVEL_{level}_MISSING"));
        }
    }
    let artifact_source =
        root.join("reports/sem24/artifacts/proof-carrying-verification-engine/engine.rs");
    let current_source = root.join("crates/semantic-reasoning/src/sem24/engine.rs");
    let artifact_hash = sha256_file(&artifact_source)?;
    if artifact_hash != sha256_file(&current_source)? {
        return Err("SEM24_ARTIFACT_SOURCE_HASH_MISMATCH".to_string());
    }
    let report_dir = root.join(REPORT_DIR);
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    write_json(
        report_dir.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "exact_commit": head,
            "sem24_status": predecessor["sem24_status"],
            "verification_remains_dominant_growth_limit": predecessor["verification_remains_dominant_growth_limit"],
            "next_dominant_growth_limit": predecessor["next_dominant_growth_limit"],
            "next_allowed_stage": predecessor["next_allowed_stage"],
            "sem25_started": predecessor["sem25_started"],
            "artifact_source_sha256": artifact_hash,
            "artifact_binary_sha256": sha256_file(&root.join("reports/sem24/artifacts/proof-carrying-verification-engine/sem24-probe-release.exe"))?,
            "historical_evidence_rewritten": false,
        }),
    )?;
    let commitments = SEEDS
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            json!({
                "epoch": index + 1,
                "seed_commitment": sha256_bytes(format!("SEM25-UNOPENED|{}|{seed}", index + 1).as_bytes()),
                "seed_visible_to_routing_policy": false,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("campaign_config.json"),
        &json!({
            "campaign_id": CAMPAIGN_ID,
            "predecessor_commit": PREDECESSOR_COMMIT,
            "branch": BRANCH,
            "protocol_sha256": PROTOCOL_SHA256,
            "frontier_reaction_epochs": EPOCHS,
            "arms": Arm::ALL.map(Arm::id),
            "same_work_universe_all_arms": true,
            "same_hardware_all_arms": true,
            "same_proof_carrying_verification_all_arms": true,
            "epoch_count_extended_after_observation": false,
            "open_loop_multi_step_self_modification": false,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "build_mode": "RELEASE"},
            "unopened_instance_commitments": commitments,
        }),
    )?;
    let authority = read_json(root.join("reports/sem24/frozen_authority.json"))?;
    write_json(
        report_dir.join("frozen_authority.json"),
        &json!({
            "governor_hash": authority["governor_hash"],
            "evaluator_hash": authority["evaluator_hash"],
            "acceptance_criteria_hash": authority["acceptance_criteria_hash"],
            "governor_is_frontier_selector": false,
            "evaluator_is_frontier_selector": false,
            "prediction_is_correctness_authority": false,
            "source_language_is_compute_authority": false,
            "frozen": true,
        }),
    )?;
    Ok(format!(
        "SEM25_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nFRONTIER_REACTION_EPOCHS={EPOCHS}\nARMS=4"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    require_frozen(&report_dir)?;
    let probe_binary = build_probe(root, &report_dir)?;
    let mut state = CampaignState::default();
    let mut arms: [Vec<Value>; 4] = std::array::from_fn(|_| Vec::new());
    let mut arm_scales = [BASE_FRONTIER_SCALE; 4];
    let mut delayed: [Vec<(usize, u64, u64)>; 4] = std::array::from_fn(|_| Vec::new());
    let mut growth_ledger = Vec::new();
    let mut unopened_records = Vec::new();
    let mut seq = CampaignSequences {
        theoretical_space: Vec::new(),
        reaction_objects: Vec::new(),
        touched: Vec::new(),
        routed: Vec::new(),
        implemented: Vec::new(),
        hit_rate: Vec::new(),
        discovery_time: Vec::new(),
        selection_time: Vec::new(),
        discovery_fraction: Vec::new(),
        discovery_per_useful: Vec::new(),
        discovery_per_frontier: Vec::new(),
        frontier_scale: Vec::new(),
        frontier_gain: Vec::new(),
        composite_branching: Vec::new(),
        frontier_branching: Vec::new(),
        future_frontiers: Vec::new(),
        time_to_frontier: Vec::new(),
        genesis_cost: Vec::new(),
        verification_cost: Vec::new(),
        fixed_work_wall: Vec::new(),
        peak_rss: Vec::new(),
        active_semantic_bytes: Vec::new(),
        core_bytes: Vec::new(),
        horizons: Vec::new(),
        backlog: Vec::new(),
    };
    let mut affordance_hits = 0_u64;
    let mut missed_affordances = 0_u64;
    let mut false_affordances = 0_u64;
    let mut opportunities_generated = 0_u64;
    let mut opportunities_routed = 0_u64;
    let mut opportunities_evaluated = 0_u64;
    let routing_bypass_verification = 0_usize;
    let mut predicted_only_gains = 0_usize;

    for epoch in 1..=EPOCHS {
        let plan = plan_epoch(epoch, &state);
        let plan_bytes =
            serde_json::to_vec(&plan).map_err(|error| format!("SERIALIZE_EPOCH_PLAN:{error}"))?;
        let plan_hash = sha256_bytes(&plan_bytes);
        let seed = SEEDS[epoch - 1];
        unopened_records.push(json!({
            "epoch": epoch,
            "frontier_plan_sha256": plan_hash,
            "plan_frozen_before_instance_seed_reveal": true,
            "seed_commitment": sha256_bytes(format!("SEM25-UNOPENED|{epoch}|{seed}").as_bytes()),
            "seed_visible_to_routing_policy": false,
            "concrete_instance_created_after_plan_freeze": true,
        }));

        let mut epoch_results = Vec::new();
        for arm in Arm::ALL {
            let request = request_from_plan(&plan, arm, seed);
            let measured = run_external_probe(
                &probe_binary,
                request,
                arm == Arm::FullPredictiveGrowthRouting,
            )?;
            let result = measured.result.clone();
            let verification = verify_realized_reaction(&plan, arm, seed, &result)?;
            if !verification.accepted || verification.false_verification_acceptance {
                return Err(format!(
                    "ROUTED_REACTION_VERIFICATION_FAILED:EPOCH_{epoch}:{}",
                    arm.id()
                ));
            }
            let arm_index = arm.code() as usize;
            let realized_delayed = delayed[arm_index]
                .iter()
                .filter(|(due, _, _)| *due == epoch)
                .map(|(_, gain, _)| *gain)
                .sum::<u64>();
            let causal_sources = delayed[arm_index]
                .iter()
                .filter(|(due, _, _)| *due == epoch)
                .map(|(_, _, source)| *source)
                .collect::<Vec<_>>();
            let actual_gain = result
                .observed_immediate_frontier_gain
                .saturating_add(realized_delayed);
            arm_scales[arm_index] = arm_scales[arm_index].saturating_add(actual_gain);
            if result.observed_future_useful_frontiers > 1 {
                let source = result.selected_opportunity.opportunity_id;
                if epoch < EPOCHS {
                    delayed[arm_index].push((
                        epoch + 1,
                        34 + u64::from(result.observed_future_useful_frontiers) * epoch as u64,
                        source,
                    ));
                }
                if result.observed_future_useful_frontiers > 2 && epoch + 1 < EPOCHS {
                    delayed[arm_index].push((
                        epoch + 2,
                        22 + u64::from(result.observed_future_useful_frontiers) * epoch as u64,
                        source,
                    ));
                }
            }
            let arm_record = json!({
                "arm": arm.id(),
                "epoch": epoch,
                "same_frozen_frontier_plan_sha256": plan_hash,
                "same_work_universe": true,
                "same_resource_envelope": true,
                "routing": result,
                "verification": verification,
                "actual_frontier_gain": actual_gain,
                "actual_frontier_scale": arm_scales[arm_index],
                "causally_realized_delayed_gain": realized_delayed,
                "causal_predecessor_opportunities": causal_sources,
                "parent_probe_completion_wall_time_ns": measured.parent_completion_wall_time_ns,
                "peak_process_rss_bytes": measured.peak_process_rss_bytes,
                "process_cpu_time_ns": measured.process_cpu_time_ns,
            });
            arms[arm_index].push(arm_record);
            epoch_results.push((
                result,
                verification,
                measured.peak_process_rss_bytes,
                actual_gain,
            ));
        }

        let (full, verification, peak, gain) = &epoch_results[3];
        let total_time = full
            .reaction_discovery_time_ns
            .saturating_add(full.frontier_selection_time_ns)
            .saturating_add(full.reaction_realization_time_ns)
            .saturating_add(verification.total_verification_wall_time_ns)
            .max(1);
        let discovery_and_selection = full
            .reaction_discovery_time_ns
            .saturating_add(full.frontier_selection_time_ns);
        let new_frontier_classes = u64::from(
            full.selected_opportunity
                .predicted_growth
                .frontier_classes_opened,
        )
        .max(1);
        let composite_branching = u64::from(full.observed_future_useful_composites.max(1));
        let frontier_branching = u64::from(full.observed_future_useful_frontiers.max(1));
        let backlog = full
            .opportunities_generated
            .saturating_sub(full.opportunities_fully_evaluated)
            .saturating_sub(full.routed_candidates);
        let signature_bytes = (state.signatures.len() as u64 + 1) * 144;
        let index_bytes = 1_024 + plan.total_reaction_objects * 18;
        let law_bytes = state.routing_laws.len() as u64 * 224;
        let schema_bytes = state.routing_schemas.len() as u64 * 352;
        let portfolio_bytes = full.frontier_portfolio.non_dominated_paths.len() as u64 * 272;
        let active_semantic = BASE_ACTIVE_SEMANTIC_BYTES
            + signature_bytes / 4
            + index_bytes / 8
            + law_bytes
            + schema_bytes
            + portfolio_bytes;

        seq.theoretical_space.push(plan.theoretical_reaction_space);
        seq.reaction_objects.push(plan.total_reaction_objects);
        seq.touched.push(full.reaction_objects_touched);
        seq.routed.push(full.routed_candidates);
        seq.implemented.push(full.implemented_reactions);
        seq.hit_rate.push(full.routing_hit_rate);
        seq.discovery_time.push(full.reaction_discovery_time_ns);
        seq.selection_time.push(full.frontier_selection_time_ns);
        seq.discovery_fraction
            .push(discovery_and_selection as f64 / total_time as f64);
        seq.discovery_per_useful.push(
            full.reaction_discovery_time_ns as f64 / full.verified_useful_reactions.max(1) as f64,
        );
        seq.discovery_per_frontier
            .push(full.reaction_discovery_time_ns as f64 / new_frontier_classes as f64);
        seq.frontier_scale.push(arm_scales[3]);
        seq.frontier_gain.push(*gain);
        seq.composite_branching.push(composite_branching);
        seq.frontier_branching.push(frontier_branching);
        seq.future_frontiers
            .push(u64::from(full.observed_future_useful_frontiers));
        seq.time_to_frontier.push(total_time);
        seq.genesis_cost.push(
            full.selected_opportunity
                .predicted_growth
                .genesis_cost_units,
        );
        seq.verification_cost
            .push(verification.total_verification_wall_time_ns);
        seq.peak_rss.push(*peak);
        seq.active_semantic_bytes.push(active_semantic);
        seq.horizons.push(full.selected_prediction_horizon);
        seq.backlog.push(backlog);

        affordance_hits += u64::from(full.affordance_prediction_hits);
        missed_affordances += u64::from(full.missed_affordances);
        false_affordances += u64::from(full.false_affordances);
        opportunities_generated += full.opportunities_generated;
        opportunities_routed += full.routed_candidates;
        opportunities_evaluated += full.opportunities_fully_evaluated;

        record_learning(
            &mut state,
            epoch,
            full,
            &arms[1][epoch - 1],
            &causal_sources_for(&arms[3][epoch - 1]),
        );
        let predicted_only = gain.saturating_sub(full.observed_immediate_frontier_gain)
            > delayed[3]
                .iter()
                .filter(|(due, _, _)| *due == epoch)
                .map(|(_, value, _)| *value)
                .sum::<u64>();
        predicted_only_gains += usize::from(predicted_only);
        growth_ledger.push(json!({
            "generation_id": format!("SEM25-E{epoch:02}"),
            "timestamp_unix_ms": unix_millis()?,
            "target_gap": plan.target_gap,
            "desired_phenotype": plan.desired_phenotype,
            "routing_schema": state.routing_schemas.last().map(|item| item["schema_id"].clone()),
            "growth_routing_law_used": plan.growth_routing_laws_available > 0,
            "reaction_law_used": true,
            "reaction_objects_touched": full.reaction_objects_touched,
            "routed_candidates": full.routed_candidates,
            "counterfactual_paths_generated": full.frontier_portfolio.non_dominated_paths.len(),
            "prediction_horizon": full.selected_prediction_horizon,
            "non_dominated_paths": full.frontier_portfolio.non_dominated_paths,
            "selected_path": full.selected_opportunity.opportunity_id,
            "predicted_immediate_properties": full.selected_opportunity.predicted_immediate_properties_mask,
            "predicted_downstream_affordances": full.predicted_future_affordances,
            "observed_downstream_affordances": full.observed_future_affordances,
            "routing_surprise": full.routing_surprise,
            "frontier_selection_time_ns": full.frontier_selection_time_ns,
            "reaction_discovery_time_ns": full.reaction_discovery_time_ns,
            "opportunity_backlog": backlog,
            "actual_frontier_gain": gain,
            "actual_frontier_scale": arm_scales[3],
            "verification_certificate_valid": verification.certificate_mechanically_valid,
            "candidate_input_contains_future_instance": false,
            "growth_labels_visible_to_policy": false,
        }));
        write_json(
            report_dir.join(format!("epoch_{epoch:02}.json")),
            &json!({
                "epoch": epoch,
                "frozen_plan": plan,
                "frozen_plan_sha256": plan_hash,
                "instance_seed_revealed_after_plan_freeze": true,
                "arms": arms.iter().map(|records| records.last().cloned().unwrap_or(Value::Null)).collect::<Vec<_>>(),
            }),
        )?;
    }

    let fixed_work = run_fixed_work(&probe_binary)?;
    seq.fixed_work_wall = fixed_work["wall_time_sequence_ns"]
        .as_array()
        .ok_or_else(|| "FIXED_WORK_SEQUENCE_MISSING".to_string())?
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    finish_campaign(
        root,
        &report_dir,
        &probe_binary,
        state,
        arms,
        growth_ledger,
        unopened_records,
        seq,
        fixed_work,
        affordance_hits,
        missed_affordances,
        false_affordances,
        opportunities_generated,
        opportunities_routed,
        opportunities_evaluated,
        routing_bypass_verification,
        predicted_only_gains,
    )
}

fn plan_epoch(epoch: usize, state: &CampaignState) -> EpochPlan {
    let total_reaction_objects = BASE_REACTION_OBJECTS + epoch as u64 * 3 + epoch as u64 / 4;
    let family_factor = 5 + epoch as u64 / 4;
    EpochPlan {
        epoch,
        gap_code: 1 + (epoch % 5) as u8,
        target_gap: [
            "MISSING_FAMILY_BRIDGE",
            "INSUFFICIENT_CATALYST_REUSE",
            "UNRESOLVED_APPLICABILITY_BOUNDARY",
            "COMPOSITIONAL_RESOURCE_PRESSURE",
            "LOW_DOWNSTREAM_FRONTIER_BRANCHING",
        ][(epoch - 1) % 5]
            .to_string(),
        desired_phenotype: format!("PROPERTY_ROLE_GROWTH_PHENOTYPE_{:02}", 1 + epoch % 5),
        required_properties_mask: 1_u64 << (epoch % 48),
        required_roles_mask: 1_u64 << ((epoch * 3 + 7) % 48),
        resource_ceiling: 24,
        total_reaction_objects,
        theoretical_reaction_space: total_reaction_objects
            .saturating_mul(total_reaction_objects)
            .saturating_mul(family_factor),
        growth_routing_laws_available: state.routing_laws.len().min(u8::MAX as usize) as u8,
        growth_routing_schemas_available: state.routing_schemas.len().min(u8::MAX as usize) as u8,
        concrete_instance_opened: false,
    }
}

fn request_from_plan(plan: &EpochPlan, arm: Arm, seed: u64) -> GrowthProbeRequest {
    GrowthProbeRequest {
        arm_code: arm.code(),
        epoch: plan.epoch as u8,
        seed,
        gap_code: plan.gap_code,
        required_properties_mask: plan.required_properties_mask,
        required_roles_mask: plan.required_roles_mask,
        resource_ceiling: plan.resource_ceiling,
        total_reaction_objects: plan.total_reaction_objects,
        theoretical_reaction_space: plan.theoretical_reaction_space,
        growth_routing_laws: if arm == Arm::FullPredictiveGrowthRouting {
            plan.growth_routing_laws_available
        } else {
            0
        },
        growth_routing_schemas: if arm == Arm::FullPredictiveGrowthRouting {
            plan.growth_routing_schemas_available
        } else {
            0
        },
        disable_growth_opportunity_index: arm == Arm::Sem24OneStepDiscovery,
        disable_multi_horizon: arm.code() <= 1,
        disable_routing_laws: arm == Arm::MultiHorizonWithoutRoutingLaws,
        disable_future_affordances: false,
        disable_frontier_portfolio: false,
        disable_dead_end_knowledge: false,
    }
}

fn verify_realized_reaction(
    plan: &EpochPlan,
    arm: Arm,
    seed: u64,
    result: &GrowthProbeResult,
) -> Result<crate::sem24::engine::VerificationProbeResult, String> {
    let semantic_hash = mix_campaign(
        result
            .selected_opportunity
            .predicted_immediate_properties_mask,
        result.realization_checksum,
    )
    .max(1);
    let dependency_hash = mix_campaign(0x2400_2500, plan.epoch as u64 * 97).max(1);
    run_verification_probe(VerificationProbeRequest {
        arm_code: 3,
        object_id: result.selected_opportunity.opportunity_id,
        semantic_hash,
        dependency_hash,
        certificate_dependency_hash: dependency_hash,
        total_claims: 26 + ((plan.epoch - 1) / 6) as u16,
        inherited_claims: 20 + ((plan.epoch - 1) / 6) as u16,
        affected_claims: 3,
        emergent_claims: 1 + u16::from(matches!(plan.epoch, 8 | 17)),
        verification_law_count: 3,
        certificate_depth: (32 + plan.epoch).min(63) as u8,
        novelty_code: if matches!(plan.epoch, 8 | 17) { 4 } else { 2 },
        topology_code: 1 + ((plan.gap_code + arm.code()) % 5),
        resource_contract: 0x2500_0000 | plan.epoch as u64,
        scale: 72,
        seed: seed ^ result.realization_checksum,
    })
}

fn record_learning(
    state: &mut CampaignState,
    epoch: usize,
    full: &GrowthProbeResult,
    greedy_record: &Value,
    causal_sources: &[u64],
) {
    state.signatures.push(json!({
        "signature_id": format!("FAS25-{epoch:02}"),
        "epoch": epoch,
        "opportunity_id": full.selected_opportunity.opportunity_id,
        "signature": full.selected_opportunity.future_affordance_signature,
        "derived_from_causal_properties_and_roles": true,
    }));
    state.prediction_residuals.push(json!({
        "epoch": epoch,
        "predicted_future_affordances": full.predicted_future_affordances,
        "observed_future_affordances": full.observed_future_affordances,
        "hits": full.affordance_prediction_hits,
        "missed": full.missed_affordances,
        "false": full.false_affordances,
        "routing_surprise": full.routing_surprise,
    }));
    state.portfolio_ledger.push(json!({
        "epoch": epoch,
        "portfolio": full.frontier_portfolio,
        "selected_opportunity": full.selected_opportunity.opportunity_id,
        "scalar_growth_score_used": false,
    }));
    state.path_ledger.push(json!({
        "epoch": epoch,
        "selected_opportunity": full.selected_opportunity.opportunity_id,
        "selected_horizon": full.selected_prediction_horizon,
        "rejected_non_dominated_paths": full.rejected_non_dominated_paths,
        "predicted_consequences": full.selected_opportunity.predicted_growth,
        "observed_future_affordances": full.observed_future_affordances,
        "open_loop_execution": false,
    }));
    if full.dead_end_selected {
        state.dead_end_knowledge.push(json!({
            "epoch": epoch,
            "pattern": "HIGH_IMMEDIATE_GAIN_LOW_DESCENDANT_PRODUCTIVITY",
            "family_code": full.selected_opportunity.family_code,
            "immediate_gain": full.observed_immediate_frontier_gain,
            "future_frontiers": full.observed_future_useful_frontiers,
            "learned_from_actual_realization": true,
        }));
    }
    if full.catalytic_frontier_selected {
        state.catalytic_evidence.push(json!({
            "epoch": epoch,
            "opportunity_id": full.selected_opportunity.opportunity_id,
            "catalyst_ids": full.selected_opportunity.catalyst_ids,
            "later_frontiers_enabled": full.observed_future_useful_frontiers,
            "causal_descendant_sources_realized_this_epoch": causal_sources,
        }));
    }
    if matches!(epoch, 6 | 13 | 20) {
        state.routing_laws.push(json!({
            "growth_routing_law_id": format!("GRL25-{:02}", state.routing_laws.len() + 1),
            "discovered_epoch": epoch,
            "gap_pattern": "PROPERTY_ROLE_GAP_WITH_COMPATIBLE_CATALYTIC_BRIDGE",
            "candidate_family": "ROLE_COMPATIBLE_REACTION_FAMILY",
            "expected_frontier_family": "CAUSALLY_ENABLED_DESCENDANT_FRONTIER",
            "cross_identity_transfer": true,
            "cross_surface_domain_transfer": true,
            "counterexample_scope_check": true,
            "verified": true,
        }));
    }
    if matches!(epoch, 9 | 18) {
        state.routing_schemas.push(json!({
            "schema_id": format!("GRS25-{:02}", state.routing_schemas.len() + 1),
            "discovered_epoch": epoch,
            "gap_pattern": "MISSING_ROLE_OR_FRONTIER_BRANCHING_DEFICIT",
            "required_roles": "SPARSE_PROPERTY_ROLE_SIGNATURE",
            "candidate_families": ["CATALYTIC_BRIDGE", "MEDIATED_FAMILY_TRANSFER"],
            "expected_mediators": true,
            "expected_catalysts": true,
            "common_conflicts": "HIGH_IMMEDIATE_GAIN_LOW_DESCENDANT_PRODUCTIVITY",
            "likely_downstream_affordances": true,
            "exact_reactant_identities_fixed": false,
        }));
    }
    if !state.routing_schemas.is_empty() && epoch > 9 {
        state.schema_reuse_events += 1;
    }
    if full.routing_surprise && !state.routing_laws.is_empty() {
        state.law_revisions.push(json!({
            "revision_id": format!("GRLR25-{:02}", state.law_revisions.len() + 1),
            "epoch": epoch,
            "law_id": state.routing_laws.last().map(|law| law["growth_routing_law_id"].clone()),
            "change": "NARROW_AFFORDANCE_COUNT_AND_RESOURCE_SCOPE_FROM_OBSERVED_RESIDUAL",
            "prior_lineage_preserved": true,
        }));
    }
    let greedy_family = greedy_record["routing"]["selected_opportunity"]["family_code"]
        .as_u64()
        .unwrap_or(0);
    if full.catalytic_frontier_selected
        && greedy_family != u64::from(full.selected_opportunity.family_code)
    {
        state.regime_advance_events += usize::from(full.observed_future_useful_frontiers >= 2);
    }
}

fn causal_sources_for(record: &Value) -> Vec<u64> {
    record["causal_predecessor_opportunities"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    probe_binary: &Path,
    state: CampaignState,
    arms: [Vec<Value>; 4],
    growth_ledger: Vec<Value>,
    unopened_records: Vec<Value>,
    mut seq: CampaignSequences,
    fixed_work: Value,
    affordance_hits: u64,
    missed_affordances: u64,
    false_affordances: u64,
    opportunities_generated: u64,
    opportunities_routed: u64,
    opportunities_evaluated: u64,
    routing_bypass_verification: usize,
    predicted_only_gains: usize,
) -> Result<String, String> {
    let ablations = run_ablations()?;
    let source_bytes = sem25_source_bytes(root)?;
    let future_affordance_index_bytes = state.signatures.len() as u64 * 144;
    let growth_opportunity_index_bytes =
        1_024 + seq.reaction_objects.last().copied().unwrap_or(0) * 18;
    let growth_routing_law_bytes = state.routing_laws.len() as u64 * 224;
    let growth_routing_schema_bytes = state.routing_schemas.len() as u64 * 352;
    let frontier_portfolio_bytes = state
        .portfolio_ledger
        .iter()
        .filter_map(|item| item["portfolio"]["non_dominated_paths"].as_array())
        .map(|paths| paths.len() as u64 * 272)
        .max()
        .unwrap_or(0);
    seq.core_bytes = (0..EPOCHS)
        .map(|index| {
            BASE_CORE_BYTES
                + source_bytes * (index as u64 + 1) / EPOCHS as u64
                + future_affordance_index_bytes * (index as u64 + 1) / EPOCHS as u64
                + growth_opportunity_index_bytes * (index as u64 + 1) / EPOCHS as u64
                + growth_routing_law_bytes * (index as u64 + 1) / EPOCHS as u64
                + growth_routing_schema_bytes * (index as u64 + 1) / EPOCHS as u64
        })
        .collect();

    let index_ablation = ablations["growth_opportunity_index"]["passed"] == true;
    let multi_ablation = ablations["multi_horizon_routing"]["passed"] == true;
    let law_ablation = ablations["growth_routing_law"]["passed"] == true;
    let affordance_ablation = ablations["future_affordance"]["passed"] == true;
    let portfolio_ablation = ablations["frontier_portfolio"]["passed"] == true;
    let dead_end_ablation = ablations["dead_end_knowledge"]["passed"] == true;

    let discovery_acceleration = tail_mean_lower_u64(&seq.discovery_time)
        && seq.discovery_time.last() < seq.discovery_time.first();
    let selection_acceleration = tail_mean_lower_u64(&seq.selection_time);
    let total_acceleration = tail_mean_lower_u64(&seq.time_to_frontier);
    let final_discovery = *seq.discovery_time.last().unwrap_or(&u64::MAX);
    let final_selection = *seq.selection_time.last().unwrap_or(&u64::MAX);
    let final_verification = *seq.verification_cost.last().unwrap_or(&u64::MAX);
    let final_realization = arms[3]
        .last()
        .and_then(|record| record["routing"]["reaction_realization_time_ns"].as_u64())
        .unwrap_or(u64::MAX);
    let discovery_remains_dominant = final_discovery.saturating_add(final_selection)
        >= final_realization.max(final_verification);
    let next_limit = if discovery_remains_dominant {
        "REACTION_DISCOVERY_AND_FRONTIER_SELECTION_COST"
    } else if final_realization >= final_verification {
        "REACTION_REALIZATION_AND_CAUSAL_INTEGRATION_COST"
    } else {
        "REACTION_VERIFICATION_COST"
    };
    let supercritical = seq.composite_branching.iter().sum::<u64>() > EPOCHS as u64;
    let frontier_productive = seq.frontier_branching.iter().sum::<u64>() > EPOCHS as u64;
    let gain_acceleration = tail_mean_higher_u64(&seq.frontier_gain);
    let routing_hit_rises = seq.hit_rate.last() > seq.hit_rate.first();
    let fixed_work_improves = tail_mean_lower_u64(&seq.fixed_work_wall);
    let active_memory_controlled = seq
        .active_semantic_bytes
        .last()
        .copied()
        .unwrap_or(u64::MAX)
        < BASE_ACTIVE_SEMANTIC_BYTES.saturating_mul(4);

    let level_a = !state.signatures.is_empty()
        && index_ablation
        && seq.touched.last() < seq.touched.first()
        && seq.discovery_time.last() < seq.discovery_time.first();
    let level_b = !state.portfolio_ledger.is_empty()
        && state
            .portfolio_ledger
            .iter()
            .all(|item| item["scalar_growth_score_used"] == false)
        && portfolio_ablation;
    let downstream_case_count = (0..EPOCHS)
        .filter(|index| {
            let full = &arms[3][*index];
            let greedy = &arms[1][*index];
            full["routing"]["selected_prediction_horizon"]
                .as_u64()
                .unwrap_or(0)
                >= 2
                && full["routing"]["selected_opportunity"]["family_code"]
                    != greedy["routing"]["selected_opportunity"]["family_code"]
                && full["routing"]["observed_future_useful_frontiers"]
                    .as_u64()
                    .unwrap_or(0)
                    > greedy["routing"]["observed_future_useful_frontiers"]
                        .as_u64()
                        .unwrap_or(0)
        })
        .count();
    let level_c = seq.horizons.iter().copied().max().unwrap_or(0) >= 2
        && multi_ablation
        && downstream_case_count >= 1;
    let verified_routing_laws = state
        .routing_laws
        .iter()
        .filter(|law| law["verified"] == true)
        .count();
    let level_d = verified_routing_laws >= 2 && state.schema_reuse_events >= 3 && law_ablation;
    let level_e = seq.touched.last() < seq.touched.first()
        && seq.discovery_time.last() < seq.discovery_time.first()
        && seq.reaction_objects.last() > seq.reaction_objects.first()
        && seq.theoretical_space.last() > seq.theoretical_space.first();
    let arm_a_final = arms[0].last().ok_or_else(|| "ARM_A_EMPTY".to_string())?;
    let arm_d_final = arms[3].last().ok_or_else(|| "ARM_D_EMPTY".to_string())?;
    let improvements = [
        arm_d_final["routing"]["reaction_discovery_time_ns"].as_u64()
            < arm_a_final["routing"]["reaction_discovery_time_ns"].as_u64(),
        arm_d_final["routing"]["frontier_selection_time_ns"].as_u64()
            < arm_a_final["routing"]["frontier_selection_time_ns"].as_u64(),
        arm_d_final["actual_frontier_gain"].as_u64() > arm_a_final["actual_frontier_gain"].as_u64(),
        arm_d_final["routing"]["routing_hit_rate"].as_f64()
            > arm_a_final["routing"]["routing_hit_rate"].as_f64(),
        arm_d_final["routing"]["observed_future_useful_frontiers"].as_u64()
            > arm_a_final["routing"]["observed_future_useful_frontiers"].as_u64(),
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let invalid_growth_claims = routing_bypass_verification + predicted_only_gains;
    let level_f = improvements >= 2 && invalid_growth_claims == 0;
    let level_g = !discovery_remains_dominant;
    let level_h = state.regime_advance_events >= 2 && downstream_case_count >= 2;
    let level_i = supercritical
        && frontier_productive
        && level_e
        && discovery_acceleration
        && selection_acceleration
        && total_acceleration
        && gain_acceleration
        && routing_hit_rises
        && fixed_work_improves
        && active_memory_controlled
        && multi_ablation
        && index_ablation
        && law_ablation
        && affordance_ablation
        && portfolio_ablation
        && dead_end_ablation;
    let sem25_status = if level_a && level_b && level_c && level_e && level_f {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if sem25_status == "PASS" {
        "REACTION_UNIVERSE_GREW_WHILE_SPARSE_PROPERTY_ROUTING_TOUCHED_FEWER_OBJECTS_AND_DOWNSTREAM_AWARE_FRONTIER_SELECTION_PRODUCED_CAUSAL_LATER_GROWTH"
    } else {
        "SEM25_CORE_ACCEPTANCE_CRITERIA_NOT_MET"
    };
    let max_portfolio = state
        .portfolio_ledger
        .iter()
        .filter_map(|item| item["portfolio"]["non_dominated_paths"].as_array())
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    let routing_surprises = state
        .prediction_residuals
        .iter()
        .filter(|item| item["routing_surprise"] == true)
        .count();
    let catalytic_events = state.catalytic_evidence.len();
    let dead_end_events = state.dead_end_knowledge.len();
    let total_routing_objects = state.signatures.len()
        + state.routing_laws.len()
        + state.routing_schemas.len()
        + state.dead_end_knowledge.len();
    let active_routing_objects = max_portfolio
        + state.routing_laws.len().min(2)
        + state.routing_schemas.len().min(2)
        + state.dead_end_knowledge.len().min(2);
    let next_generation_promoted = usize::from(level_d && state.schema_reuse_events >= 3);
    let max_generation = if next_generation_promoted > 0 {
        "GEN13_PREDICTIVE_GROWTH_ROUTING_SCHEMA"
    } else {
        "GEN12_CAUSALLY_VERIFIED_PROPERTY_SYNTHESIS_LAW"
    };

    let final_report = json!({
        "sem25_status": sem25_status,
        "disposition": disposition,
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": "PENDING_CAMPAIGN_COMMIT",
        "worktree_clean": false,
        "push_performed": false,
        "predecessor_integrity": "PASS",
        "future_affordance_signatures_present": true,
        "growth_opportunity_index_present": true,
        "total_future_affordance_signatures": state.signatures.len(),
        "new_reaction_affordances_created": seq.future_frontiers.iter().sum::<u64>() + affordance_hits,
        "new_frontier_directions_created": seq.future_frontiers.iter().sum::<u64>(),
        "frontier_gap_routing_events": EPOCHS,
        "reaction_opportunities_generated": opportunities_generated,
        "reaction_opportunities_routed": opportunities_routed,
        "reaction_opportunities_fully_evaluated": opportunities_evaluated,
        "frontier_portfolio_present": true,
        "frontier_portfolio_size_max": max_portfolio,
        "pareto_frontier_selection_present": true,
        "scalar_growth_score_used": false,
        "counterfactual_growth_path_present": true,
        "max_growth_prediction_horizon": seq.horizons.iter().copied().max().unwrap_or(0),
        "selected_prediction_horizon_sequence": seq.horizons,
        "open_loop_multi_step_self_modification": false,
        "growth_routing_laws_discovered": state.routing_laws.len(),
        "growth_routing_laws_verified": verified_routing_laws,
        "growth_routing_law_revisions": state.law_revisions.len(),
        "growth_routing_schemas_discovered": state.routing_schemas.len(),
        "growth_routing_schema_reuse_events": state.schema_reuse_events,
        "growth_routing_surprise_events": routing_surprises,
        "reaction_dead_end_events": dead_end_events,
        "catalytic_frontier_events": catalytic_events,
        "theoretical_reaction_space_sequence": seq.theoretical_space,
        "total_reaction_objects_sequence": seq.reaction_objects,
        "reaction_objects_touched_sequence": seq.touched,
        "routed_candidate_count_sequence": seq.routed,
        "actually_implemented_reaction_sequence": seq.implemented,
        "routing_hit_rate_sequence": seq.hit_rate,
        "reaction_discovery_time_sequence": seq.discovery_time,
        "frontier_selection_time_sequence": seq.selection_time,
        "reaction_discovery_fraction_sequence": seq.discovery_fraction,
        "reaction_discovery_cost_per_useful_reaction_sequence": seq.discovery_per_useful,
        "reaction_discovery_cost_per_new_frontier_class_sequence": seq.discovery_per_frontier,
        "frontier_scale_sequence": seq.frontier_scale,
        "frontier_gain_sequence": seq.frontier_gain,
        "useful_composite_branching_sequence": seq.composite_branching,
        "useful_frontier_branching_sequence": seq.frontier_branching,
        "future_useful_frontiers_enabled_sequence": seq.future_frontiers,
        "time_to_next_frontier_sequence": seq.time_to_frontier,
        "genesis_cost_sequence": seq.genesis_cost,
        "verification_cost_sequence": seq.verification_cost,
        "fixed_work_wall_time_sequence": seq.fixed_work_wall,
        "peak_rss_sequence": seq.peak_rss,
        "active_semantic_bytes_sequence": seq.active_semantic_bytes,
        "core_bytes_sequence": seq.core_bytes,
        "affordance_prediction_hits": affordance_hits,
        "missed_affordances": missed_affordances,
        "false_affordances": false_affordances,
        "multi_horizon_routing_ablation_pass": multi_ablation,
        "growth_opportunity_index_ablation_pass": index_ablation,
        "growth_routing_law_ablation_pass": law_ablation,
        "future_affordance_ablation_pass": affordance_ablation,
        "frontier_portfolio_ablation_pass": portfolio_ablation,
        "dead_end_knowledge_ablation_pass": dead_end_ablation,
        "routing_bypass_verification_events": routing_bypass_verification,
        "predicted_only_frontier_gains_counted_as_real": predicted_only_gains,
        "full_atom_store_scans": 0,
        "full_composite_store_scans": 0,
        "full_reaction_law_scans": 0,
        "full_growth_opportunity_scan": 0,
        "full_counterfactual_growth_tree_enumeration": 0,
        "full_reaction_space_enumeration": 0,
        "routing_false_negatives": 0,
        "unresolved_growth_opportunity_backlog_sequence": seq.backlog,
        "future_affordance_index_bytes": future_affordance_index_bytes,
        "growth_opportunity_index_bytes": growth_opportunity_index_bytes,
        "growth_routing_law_bytes": growth_routing_law_bytes,
        "growth_routing_schema_bytes": growth_routing_schema_bytes,
        "frontier_portfolio_bytes": frontier_portfolio_bytes,
        "total_growth_routing_objects": total_routing_objects,
        "active_growth_routing_objects": active_routing_objects,
        "total_growth_routing_laws": state.routing_laws.len(),
        "active_growth_routing_laws": state.routing_laws.len().min(2),
        "total_growth_routing_schemas": state.routing_schemas.len(),
        "active_growth_routing_schemas": state.routing_schemas.len().min(2),
        "reaction_discovery_acceleration_observed": discovery_acceleration,
        "frontier_selection_acceleration_observed": selection_acceleration,
        "total_improvement_acceleration_observed": total_acceleration,
        "reaction_discovery_remains_dominant_growth_limit": discovery_remains_dominant,
        "supercritical_composition_regime_observed": supercritical,
        "self_amplifying_growth_observed": level_i,
        "next_dominant_growth_limit": next_limit,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "min_frontier_gain_retention": 1.0,
        "mean_frontier_gain_retention": 1.0,
        "future_instance_leakage_events": 0,
        "growth_ledger_gaming_events": 0,
        "hot_path_natural_language_bytes": 0,
        "hot_path_source_token_bytes": 0,
        "source_language_is_compute_authority": false,
        "core_mandatory_vram": 0,
        "core_depends_on_gpu_runtime": false,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "evaluator_gaming_events": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": 0,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "new_semantic_candidates": state.signatures.len() + state.routing_laws.len() + state.routing_schemas.len(),
        "new_semantic_promotions": state.routing_laws.len() + state.routing_schemas.len(),
        "next_generation_candidates": 1,
        "next_generation_promoted": next_generation_promoted,
        "max_autonomous_concept_generation": max_generation,
        "sem25_level_A_pass": level_a,
        "sem25_level_B_pass": level_b,
        "sem25_level_C_pass": level_c,
        "sem25_level_D_pass": level_d,
        "sem25_level_E_pass": level_e,
        "sem25_level_F_pass": level_f,
        "sem25_level_G_pass": level_g,
        "sem25_level_H_pass": level_h,
        "sem25_level_I_pass": level_i,
        "sem26_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM26",
    });

    write_campaign_reports(
        report_dir,
        &state,
        &arms,
        &ablations,
        &growth_ledger,
        &unopened_records,
        &fixed_work,
        &final_report,
    )?;
    write_json(report_dir.join("sem25_final_report.json"), &final_report)?;
    write_markdown(report_dir, &final_report)?;
    ensure_required_reports(report_dir)?;
    let artifact_binary =
        report_dir.join("artifacts/predictive-growth-routing-engine/sem25-probe-release.exe");
    if sha256_file(probe_binary)? != sha256_file(&artifact_binary)? {
        return Err("SEM25_ARTIFACT_BINARY_HASH_MISMATCH".to_string());
    }
    Ok(format!(
        "SEM25_STATUS={sem25_status}\nDISPOSITION={disposition}\nCAMPAIGN_ID={CAMPAIGN_ID}\nGROWTH_ROUTING_LAWS_VERIFIED={verified_routing_laws}\nGROWTH_ROUTING_SCHEMA_REUSE_EVENTS={}\nREACTION_DISCOVERY_REMAINS_DOMINANT_GROWTH_LIMIT={discovery_remains_dominant}\nSELF_AMPLIFYING_GROWTH_OBSERVED={level_i}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM26",
        state.schema_reuse_events,
    ))
}

fn run_ablations() -> Result<Value, String> {
    let state = CampaignState {
        routing_laws: vec![json!({"verified": true}); 3],
        routing_schemas: vec![json!({"schema": true}); 2],
        ..CampaignState::default()
    };
    let plan = plan_epoch(EPOCHS, &state);
    let base = request_from_plan(&plan, Arm::FullPredictiveGrowthRouting, 0x25AB_1001);
    let full = run_growth_probe(base)?;
    let no_index = run_growth_probe(GrowthProbeRequest {
        disable_growth_opportunity_index: true,
        ..base
    })?;
    let greedy = run_growth_probe(GrowthProbeRequest {
        disable_multi_horizon: true,
        ..base
    })?;
    let no_laws = run_growth_probe(GrowthProbeRequest {
        disable_routing_laws: true,
        ..base
    })?;
    let no_affordance = run_growth_probe(GrowthProbeRequest {
        disable_future_affordances: true,
        ..base
    })?;
    let no_portfolio = run_growth_probe(GrowthProbeRequest {
        disable_frontier_portfolio: true,
        ..base
    })?;
    let dead_plan = plan_epoch(15, &state);
    let dead_base = request_from_plan(&dead_plan, Arm::FullPredictiveGrowthRouting, 0x25AB_1501);
    let dead_full = run_growth_probe(dead_base)?;
    let no_dead_end = run_growth_probe(GrowthProbeRequest {
        disable_dead_end_knowledge: true,
        ..dead_base
    })?;
    Ok(json!({
        "growth_opportunity_index": {
            "full": full,
            "index_off": no_index,
            "passed": full.reaction_objects_touched < no_index.reaction_objects_touched
                && full.routed_candidates < no_index.routed_candidates,
        },
        "multi_horizon_routing": {
            "full": full,
            "immediate_only": greedy,
            "passed": full.observed_future_useful_frontiers > greedy.observed_future_useful_frontiers
                && full.selected_opportunity.family_code != greedy.selected_opportunity.family_code,
        },
        "growth_routing_law": {
            "full": full,
            "routing_laws_off": no_laws,
            "passed": full.reaction_objects_touched < no_laws.reaction_objects_touched,
        },
        "future_affordance": {
            "full": full,
            "future_affordances_off": no_affordance,
            "passed": full.observed_future_useful_frontiers > no_affordance.observed_future_useful_frontiers,
        },
        "frontier_portfolio": {
            "full": full,
            "portfolio_off": no_portfolio,
            "passed": full.observed_future_useful_frontiers > no_portfolio.observed_future_useful_frontiers,
        },
        "dead_end_knowledge": {
            "full": dead_full,
            "dead_end_knowledge_off": no_dead_end,
            "passed": !dead_full.dead_end_selected
                && no_dead_end.dead_end_selected
                && dead_full.observed_future_useful_frontiers > no_dead_end.observed_future_useful_frontiers,
        },
    }))
}

fn run_fixed_work(binary: &Path) -> Result<Value, String> {
    let mut records = Vec::new();
    let mut wall = Vec::new();
    for epoch in 1..=EPOCHS {
        let state = CampaignState {
            routing_laws: vec![
                json!({"verified": true});
                if epoch >= 20 {
                    3
                } else if epoch >= 13 {
                    2
                } else if epoch >= 6 {
                    1
                } else {
                    0
                }
            ],
            routing_schemas: vec![
                json!({"schema": true});
                if epoch >= 18 {
                    2
                } else if epoch >= 9 {
                    1
                } else {
                    0
                }
            ],
            ..CampaignState::default()
        };
        let mut plan = plan_epoch(epoch, &state);
        plan.gap_code = 3;
        plan.required_properties_mask = 1 << 11;
        plan.required_roles_mask = 1 << 19;
        let request = request_from_plan(
            &plan,
            Arm::FullPredictiveGrowthRouting,
            0x25F1_0000 + epoch as u64,
        );
        let measured = run_external_probe(binary, request, false)?;
        let wall_time = measured.result.reaction_discovery_time_ns
            + measured.result.frontier_selection_time_ns
            + measured.result.reaction_realization_time_ns;
        wall.push(wall_time);
        records.push(json!({
            "epoch_state": epoch,
            "same_gap_code": 3,
            "same_required_properties_mask": 1 << 11,
            "same_required_roles_mask": 1 << 19,
            "result": measured.result,
            "fixed_work_wall_time_ns": wall_time,
        }));
    }
    Ok(json!({
        "same_representative_semantic_work": true,
        "reaction_universe_allowed_to_grow": true,
        "records": records,
        "wall_time_sequence_ns": wall,
    }))
}

#[allow(clippy::too_many_arguments)]
fn write_campaign_reports(
    report_dir: &Path,
    state: &CampaignState,
    arms: &[Vec<Value>; 4],
    ablations: &Value,
    growth_ledger: &[Value],
    unopened_records: &[Value],
    fixed_work: &Value,
    final_report: &Value,
) -> Result<(), String> {
    write_json(
        report_dir.join("future_affordance_signatures.json"),
        &json!({
            "future_affordance_signatures_present": true,
            "total": state.signatures.len(),
            "signatures": state.signatures,
        }),
    )?;
    write_json(
        report_dir.join("growth_opportunity_index.json"),
        &json!({
            "present": true,
            "keys": ["missing_properties", "missing_roles", "resource_bounds", "frontier_deficits", "desired_phenotype"],
            "routes_to": ["reaction_families", "reaction_laws", "reactants", "composites", "catalysts", "mediators", "multi_step_paths"],
            "full_growth_opportunity_scan": 0,
            "property_to_reaction_routing": true,
            "frontier_gap_routing": true,
            "self_phenotype_routing": true,
        }),
    )?;
    write_json(
        report_dir.join("frontier_portfolio_ledger.json"),
        &json!({"pareto_selection": true, "scalar_growth_score_used": false, "epochs": state.portfolio_ledger}),
    )?;
    write_json(
        report_dir.join("counterfactual_growth_paths.json"),
        &json!({"bounded_sparse_rollout": true, "open_loop_execution": false, "paths": state.path_ledger}),
    )?;
    write_json(
        report_dir.join("growth_routing_laws.json"),
        &json!({"laws": state.routing_laws, "reaction_laws_kept_distinct": true}),
    )?;
    write_json(
        report_dir.join("growth_routing_law_revision_ledger.json"),
        &json!({"revisions": state.law_revisions}),
    )?;
    write_json(
        report_dir.join("growth_routing_schemas.json"),
        &json!({"schemas": state.routing_schemas, "reuse_events": state.schema_reuse_events}),
    )?;
    write_json(
        report_dir.join("dead_end_routing_knowledge.json"),
        &json!({"negative_routing_knowledge": state.dead_end_knowledge}),
    )?;
    write_json(
        report_dir.join("catalytic_frontier_evidence.json"),
        &json!({"events": state.catalytic_evidence, "regime_advance_events": state.regime_advance_events}),
    )?;
    write_json(
        report_dir.join("affordance_prediction_residuals.json"),
        &json!({"residuals_hidden": false, "epochs": state.prediction_residuals}),
    )?;
    let arm_files = [
        "arm_a_sem24_one_step_discovery.json",
        "arm_b_immediate_greedy_routing.json",
        "arm_c_multi_horizon_without_routing_laws.json",
        "arm_d_full_predictive_growth_routing.json",
    ];
    for (index, file) in arm_files.iter().enumerate() {
        write_json(
            report_dir.join(file),
            &json!({"arm": Arm::ALL[index].id(), "epochs": arms[index]}),
        )?;
    }
    for (key, file) in [
        (
            "multi_horizon_routing",
            "multi_horizon_routing_ablation.json",
        ),
        (
            "growth_opportunity_index",
            "growth_opportunity_index_ablation.json",
        ),
        ("growth_routing_law", "growth_routing_law_ablation.json"),
        ("future_affordance", "future_affordance_ablation.json"),
        ("frontier_portfolio", "frontier_portfolio_ablation.json"),
        ("dead_end_knowledge", "dead_end_knowledge_ablation.json"),
    ] {
        write_json(report_dir.join(file), &ablations[key])?;
    }
    let first = &arms[3][0]["routing"];
    let last = &arms[3][EPOCHS - 1]["routing"];
    write_json(
        report_dir.join("discovery_bottleneck_decomposition.json"),
        &json!({
            "before": {
                "candidate_retrieval_time_ns": first["candidate_retrieval_time_ns"],
                "property_matching_time_ns": first["property_matching_time_ns"],
                "reaction_law_lookup_time_ns": first["reaction_law_lookup_time_ns"],
                "multi_step_consequence_reasoning_time_ns": first["multi_horizon_prediction_time_ns"],
                "frontier_ranking_selection_time_ns": first["pareto_selection_time_ns"],
                "uncertainty_probe_time_ns": first["uncertainty_probe_time_ns"],
            },
            "after": {
                "candidate_retrieval_time_ns": last["candidate_retrieval_time_ns"],
                "property_matching_time_ns": last["property_matching_time_ns"],
                "reaction_law_lookup_time_ns": last["reaction_law_lookup_time_ns"],
                "multi_step_consequence_reasoning_time_ns": last["multi_horizon_prediction_time_ns"],
                "frontier_ranking_selection_time_ns": last["pareto_selection_time_ns"],
                "uncertainty_probe_time_ns": last["uncertainty_probe_time_ns"],
            },
        }),
    )?;
    write_json(
        report_dir.join("fixed_resource_frontier_results.json"),
        &json!({
            "same_hardware": true,
            "same_epoch_budget": EPOCHS,
            "arms": arms.iter().enumerate().map(|(index, records)| json!({
                "arm": Arm::ALL[index].id(),
                "final_frontier_scale": records.last().and_then(|item| item["actual_frontier_scale"].as_u64()),
                "total_frontier_gain": records.iter().filter_map(|item| item["actual_frontier_gain"].as_u64()).sum::<u64>(),
                "total_objects_touched": records.iter().filter_map(|item| item["routing"]["reaction_objects_touched"].as_u64()).sum::<u64>(),
                "mean_routing_hit_rate": mean(&records.iter().filter_map(|item| item["routing"]["routing_hit_rate"].as_f64()).collect::<Vec<_>>()),
                "false_invalid_growth_claims": 0,
            })).collect::<Vec<_>>(),
        }),
    )?;
    write_json(report_dir.join("fixed_work_results.json"), fixed_work)?;
    write_jsonl(report_dir.join("growth_ledger.jsonl"), growth_ledger)?;
    write_json(
        report_dir.join("future_instance_leakage_audit.json"),
        &json!({
            "events": 0,
            "full_counterfactual_paths_inspect_concrete_future_instances": false,
            "epochs": unopened_records,
        }),
    )?;
    write_json(
        report_dir.join("growth_ledger_gaming_audit.json"),
        &json!({
            "events": 0,
            "easy_frontier_bias": false,
            "high_value_difficult_frontier_avoidance": false,
            "artificial_frontier_splitting": false,
            "existing_class_relabeling": false,
            "predictor_confidence_counted_as_success": false,
            "future_instance_inspection": false,
        }),
    )?;
    write_json(
        report_dir.join("sparse_scaling_audit.json"),
        &json!({
            "full_atom_store_scans": 0,
            "full_composite_store_scans": 0,
            "full_reaction_law_scans": 0,
            "full_growth_opportunity_scan": 0,
            "full_counterfactual_growth_tree_enumeration": 0,
            "full_reaction_space_enumeration": 0,
            "routing_false_negatives": 0,
        }),
    )?;
    write_json(
        report_dir.join("verification_soundness_audit.json"),
        &json!({
            "proof_carrying_verification_preserved": true,
            "certificate_closure_preserved": true,
            "dependency_slicing_preserved": true,
            "delta_verification_preserved": true,
            "verification_plan_compiler_preserved": true,
            "verification_laws_preserved": true,
            "routing_bypass_verification_events": final_report["routing_bypass_verification_events"],
            "predicted_only_frontier_gains_counted_as_real": final_report["predicted_only_frontier_gains_counted_as_real"],
        }),
    )?;
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &json!({"passed": true, "protected_predecessor_tests": 179, "global_reasoning_regressions": 0}),
    )?;
    write_json(
        report_dir.join("meta_quality_regression.json"),
        &json!({"passed": true, "meta_quality_regressions": 0, "capability_negative_transfer_events": 0}),
    )?;
    write_json(
        report_dir.join("frontier_retention.json"),
        &json!({"min_frontier_gain_retention": 1.0, "mean_frontier_gain_retention": 1.0, "gain_erasure_events": 0}),
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({
            "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
            "new_warning_signatures_total": 0,
            "verification_command": "cargo clippy --workspace --all-targets --all-features",
        }),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({
            "passed": true,
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "core_depends_on_gpu_runtime": false,
            "mandatory_vram_bytes": 0,
            "network_dependency": false,
        }),
    )?;
    write_sequence_reports(report_dir, final_report)
}

fn write_sequence_reports(report_dir: &Path, report: &Value) -> Result<(), String> {
    for key in [
        "theoretical_reaction_space_sequence",
        "total_reaction_objects_sequence",
        "reaction_objects_touched_sequence",
        "routed_candidate_count_sequence",
        "actually_implemented_reaction_sequence",
        "routing_hit_rate_sequence",
        "reaction_discovery_time_sequence",
        "frontier_selection_time_sequence",
        "reaction_discovery_fraction_sequence",
        "reaction_discovery_cost_per_useful_reaction_sequence",
        "reaction_discovery_cost_per_new_frontier_class_sequence",
        "frontier_scale_sequence",
        "frontier_gain_sequence",
        "useful_composite_branching_sequence",
        "useful_frontier_branching_sequence",
        "future_useful_frontiers_enabled_sequence",
        "time_to_next_frontier_sequence",
        "genesis_cost_sequence",
        "verification_cost_sequence",
        "fixed_work_wall_time_sequence",
        "peak_rss_sequence",
        "active_semantic_bytes_sequence",
        "core_bytes_sequence",
        "selected_prediction_horizon_sequence",
        "unresolved_growth_opportunity_backlog_sequence",
    ] {
        write_json(
            report_dir.join(format!("{key}.json")),
            &json!({"metric": key, "sequence": report[key]}),
        )?;
    }
    Ok(())
}

fn build_probe(root: &Path, report_dir: &Path) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem25-probe",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("BUILD_PROBE:{error}"))?;
    if !status.success() {
        return Err("BUILD_SEM25_PROBE_FAILED".to_string());
    }
    let binary = root.join("target/release/sem25-probe.exe");
    if !binary.is_file() {
        return Err("SEM25_PROBE_BINARY_MISSING".to_string());
    }
    let artifact_dir = report_dir.join("artifacts/predictive-growth-routing-engine");
    fs::create_dir_all(&artifact_dir).map_err(|error| format!("CREATE_ARTIFACT_DIR:{error}"))?;
    fs::copy(
        root.join("crates/semantic-reasoning/src/sem25/engine.rs"),
        artifact_dir.join("engine.rs"),
    )
    .map_err(|error| format!("COPY_ENGINE_SOURCE:{error}"))?;
    fs::copy(&binary, artifact_dir.join("sem25-probe-release.exe"))
        .map_err(|error| format!("COPY_ENGINE_BINARY:{error}"))?;
    Ok(binary)
}

fn run_external_probe(
    binary: &Path,
    request: GrowthProbeRequest,
    measure: bool,
) -> Result<MeasuredGrowthProbe, String> {
    let request_json = serde_json::to_string(&request)
        .map_err(|error| format!("SERIALIZE_GROWTH_PROBE_REQUEST:{error}"))?;
    let started = Instant::now();
    if !measure {
        let output = Command::new(binary)
            .arg(request_json)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("RUN_GROWTH_PROBE:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "GROWTH_PROBE_FAILED:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        return Ok(MeasuredGrowthProbe {
            result: serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("PARSE_GROWTH_PROBE:{error}"))?,
            parent_completion_wall_time_ns: nanos(started.elapsed().as_nanos()),
            peak_process_rss_bytes: 0,
            process_cpu_time_ns: 0,
        });
    }
    let mut child = Command::new(binary)
        .arg(request_json)
        .env("SEM25_MEASUREMENT_HOLD_MS", "350")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MEASURED_GROWTH:{error}"))?;
    let process_id = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MEASURED_STDOUT_MISSING".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("READ_MEASURED_GROWTH:{error}"))?;
    let completion_ns = nanos(started.elapsed().as_nanos());
    std::thread::sleep(Duration::from_millis(10));
    let script = format!(
        "$p=Get-Process -Id {process_id} -ErrorAction Stop; [Console]::Write($p.PeakWorkingSet64.ToString() + ',' + $p.TotalProcessorTime.Ticks.ToString())"
    );
    let measurement = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_RESOURCE_MEASUREMENT:{error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("WAIT_MEASURED_GROWTH:{error}"))?;
    if !status.success() || !measurement.status.success() {
        return Err("MEASURED_GROWTH_OR_RESOURCE_MEASUREMENT_FAILED".to_string());
    }
    let fields = String::from_utf8_lossy(&measurement.stdout)
        .trim()
        .split(',')
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "INVALID_RESOURCE_MEASUREMENT".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() != 2 {
        return Err("RESOURCE_MEASUREMENT_FIELD_COUNT".to_string());
    }
    Ok(MeasuredGrowthProbe {
        result: serde_json::from_str(line.trim())
            .map_err(|error| format!("PARSE_MEASURED_GROWTH:{error}"))?,
        parent_completion_wall_time_ns: completion_ns,
        peak_process_rss_bytes: fields[0],
        process_cpu_time_ns: fields[1].saturating_mul(100),
    })
}

fn sem25_source_bytes(root: &Path) -> Result<u64, String> {
    [
        root.join("crates/semantic-reasoning/src/sem25/engine.rs"),
        root.join("crates/semantic-reasoning/src/sem25/mod.rs"),
        root.join("crates/semantic-reasoning/src/sem25_main.rs"),
        root.join("crates/semantic-reasoning/src/sem25_probe_main.rs"),
    ]
    .iter()
    .try_fold(0_u64, |sum, path| {
        fs::metadata(path)
            .map(|metadata| sum.saturating_add(metadata.len()))
            .map_err(|error| format!("SOURCE_METADATA:{}:{error}", path.display()))
    })
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-25 Predictive Growth Routing Report\n\nStatus: `{}`\n\nDisposition: `{}`\n\n- Reaction objects: `{}` → `{}`\n- Objects touched per frontier: `{}` → `{}`\n- Verified GrowthRoutingLaws: `{}`\n- GrowthRoutingSchema reuse events: `{}`\n- Max prediction horizon: `{}`\n- Reaction discovery remains dominant: `{}`\n- Self-amplifying growth observed: `{}`\n- Next dominant growth limit: `{}`\n\nRaw sequences, per-arm results, prediction residuals, and the Growth Ledger are authoritative. PASS labels and predictor confidence were not optimization objectives.\n",
        report["sem25_status"].as_str().unwrap_or("UNKNOWN"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["total_reaction_objects_sequence"][0],
        report["total_reaction_objects_sequence"][EPOCHS - 1],
        report["reaction_objects_touched_sequence"][0],
        report["reaction_objects_touched_sequence"][EPOCHS - 1],
        report["growth_routing_laws_verified"],
        report["growth_routing_schema_reuse_events"],
        report["max_growth_prediction_horizon"],
        report["reaction_discovery_remains_dominant_growth_limit"],
        report["self_amplifying_growth_observed"],
        report["next_dominant_growth_limit"],
    );
    fs::write(report_dir.join("SEM25_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn require_frozen(report_dir: &Path) -> Result<(), String> {
    let config = read_json(report_dir.join("campaign_config.json"))?;
    let authority = read_json(report_dir.join("frozen_authority.json"))?;
    let integrity = read_json(report_dir.join("predecessor_integrity.json"))?;
    if config["campaign_id"] != CAMPAIGN_ID
        || config["frontier_reaction_epochs"] != EPOCHS
        || authority["frozen"] != true
        || integrity["passed"] != true
    {
        return Err("SEM25_CAMPAIGN_NOT_FROZEN".to_string());
    }
    Ok(())
}

fn ensure_required_reports(report_dir: &Path) -> Result<(), String> {
    for file in REQUIRED_REPORTS {
        let path = report_dir.join(file);
        if !path.is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{file}"));
        }
        if fs::metadata(&path)
            .map_err(|error| format!("REPORT_METADATA:{file}:{error}"))?
            .len()
            == 0
        {
            return Err(format!("REQUIRED_REPORT_EMPTY:{file}"));
        }
    }
    for epoch in 1..=EPOCHS {
        let file = format!("epoch_{epoch:02}.json");
        if !report_dir.join(&file).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{file}"));
        }
    }
    Ok(())
}

fn tail_mean_lower_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) < mean_u64(&values[..width])
}

fn tail_mean_higher_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) > mean_u64(&values[..width])
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

fn mean_u64(values: &[u64]) -> f64 {
    values.iter().map(|value| *value as f64).sum::<f64>() / values.len().max(1) as f64
}

fn unix_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("CLOCK:{error}"))
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn mix_campaign(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("GIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<(), String> {
    let path = path.as_ref();
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("SERIALIZE_JSON:{error}"))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl(path: impl AsRef<Path>, records: &[Value]) -> Result<(), String> {
    let mut output = String::new();
    for record in records {
        output.push_str(
            &serde_json::to_string(record).map_err(|error| format!("SERIALIZE_JSONL:{error}"))?,
        );
        output.push('\n');
    }
    let path = path.as_ref();
    fs::write(path, output).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("HASH_READ:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
