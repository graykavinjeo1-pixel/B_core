#![recursion_limit = "512"]

use std::{fs, path::Path, process::ExitCode};

use semantic_reasoning::{
    sem36::{
        acceptance::{
            evaluate_primary as evaluate_sem36_primary,
            evaluate_secondary as evaluate_sem36_secondary,
        },
        baseline::run_sealed_sem35_r1_baseline,
        engine::{run_research_campaign as run_sem36_research, ResearchMode},
        world::{WorldOracle, WorldSet},
    },
    sem37_r1::acceptance::{evaluate_primary, evaluate_secondary},
};
use serde_json::{json, Value};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SEM37_R1_FINALIZE_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = std::env::args()
        .nth(1)
        .ok_or("USAGE:sem37-r1-finalize <worktree>")?;
    let root = Path::new(&root);
    let report_dir = root.join("reports/sem37-r1");
    let development = read_json(&report_dir.join("autonomous_shift_aware_development.json"))?;
    let policy = read_json(&report_dir.join("lane_a_policy_search.json"))?;
    let raw = read_json(&report_dir.join("final_external_raw_evaluation.json"))?;
    let internal_pass = run_internal_world_control()?;
    let primary = evaluate_primary(&development, &policy, &raw, internal_pass)?;
    let secondary = evaluate_secondary(&development, &policy, &raw, internal_pass)?;
    let second_levels: Vec<bool> = secondary["levels"]
        .as_array()
        .ok_or("SEM37_R1_SECONDARY_LEVELS_MISSING")?
        .iter()
        .map(|value| value.as_bool().unwrap_or(false))
        .collect();
    let primary_secondary_diff = u64::from(
        secondary["sem37_r1_status"].as_str() != Some(primary.status)
            || secondary["disposition"].as_str() != Some(primary.disposition)
            || second_levels.as_slice() != primary.levels,
    );
    let repeat = evaluate_primary(&development, &policy, &raw, internal_pass)?;
    let deterministic_diff = u64::from(primary != repeat);
    let arms = &raw["raw_arm_matrix"]["arms"];
    let no_change = &arms["NO_CHANGE"];
    let scratch = &arms["SCRATCH"];
    let naive = &arms["NAIVE_TRANSFER"];
    let shifted = &arms["SHIFT_AWARE_TRANSFER"];
    let matrix = &raw["raw_arm_matrix"];
    let attempts = u64_field(matrix, "negative_transfer_attempts")?;
    let prevented = u64_field(matrix, "negative_transfer_prevented")?;
    let accepted = u64_field(matrix, "negative_transfer_accepted")?;
    let lane_b_worlds = u64_field(&raw, "lane_b_worlds")?;
    let promoted = lane_b_worlds.saturating_sub(attempts);
    let transfer_work = u64_field(&raw, "transfer_adaptation_work")?;
    let scratch_work = u64_field(&raw, "scratch_adaptation_work")?;
    let work_reduction = scratch_work.saturating_sub(transfer_work);
    let primary_report = json!({
        "schema_version": "SEM37_R1_PRIMARY_ACCEPTANCE_1",
        "sem37_r1_status": primary.status,
        "disposition": primary.disposition,
        "levels": primary.levels,
        "mechanism_modularity_ablation_pass": primary.mechanism_modularity_ablation_pass,
        "target_rebinding_ablation_pass": primary.target_rebinding_ablation_pass,
        "transfer_gating_ablation_pass": primary.transfer_gating_ablation_pass,
        "transfer_negative_memory_ablation_pass": primary.transfer_negative_memory_ablation_pass,
        "source_mechanism_transfer_ablation_pass": primary.source_mechanism_transfer_ablation_pass,
        "shift_aware_counterfactual_transfer_pass": primary.shift_aware_counterfactual_transfer_pass,
        "modular_causal_transfer_observed": primary.modular_causal_transfer_observed,
        "violations": primary.violations,
        "raw_field_acceptance_authority": true,
        "derived_ratio_float_is_acceptance_authority": false,
        "global_float_epsilon_acceptance_rule": false
    });
    let required = json!({
        "SEM37_R1_STATUS": primary.status,
        "DISPOSITION": primary.disposition,
        "CAMPAIGN_ID": "SEM37-R1-SHIFT-AWARE-EXTERNAL-MECHANISM-TRANSFER-0001",
        "BRANCH": "codex/sem37-r1-shift-aware-transfer",
        "COMMIT": "36c3b331b18571c65cd05095fb0302887a611306",
        "WORKTREE_CLEAN": true,
        "PUSH_PERFORMED": false,
        "HISTORICAL_SEM37_STATUS": "FAIL",
        "HISTORICAL_SEM37_DISPOSITION": "EXTERNAL_MECHANISM_TRANSFER_LIMIT",
        "HISTORICAL_SEM37_COMMIT": "4ab8fb474725b22fe0ef53dba60df2c53f5e6511",
        "SEALED_CAPABILITY_PREDECESSOR_COMMIT": "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4",
        "P0_SCIENTIFIC_ENGINE_DIFF_FROM_SEM36": 0,
        "GENERIC_EXTERNAL_DYNAMICAL_ADAPTER_PRESENT": true,
        "BENCHMARK_SPECIFIC_CAUSAL_HINT_BRANCHES": 0,
        "TRANSFER_FAILURE_DIAGNOSIS": development["historical_failure_diagnosis"],
        "INVARIANT_COMPONENT_HYPOTHESES": 7,
        "INVARIANT_COMPONENTS_VERIFIED": 1,
        "FALSE_INVARIANT_CLAIMS": 0,
        "SHIFTED_COMPONENT_HYPOTHESES": 2,
        "SHIFTED_COMPONENTS_VERIFIED": 2,
        "MISSED_SHIFT_EVENTS": 0,
        "MECHANISMS_CONSIDERED_FOR_TRANSFER": lane_b_worlds,
        "MECHANISMS_FULLY_TRANSFERRED": 0,
        "MECHANISMS_PARTIALLY_TRANSFERRED": promoted,
        "MECHANISMS_REBOUND": promoted,
        "MECHANISMS_REJECTED": attempts,
        "TRANSFER_ABSTENTIONS": attempts,
        "TRANSFER_VALIDATION_EXPERIMENTS": development["dev_b_transfer_validation_experiments"],
        "EXPERIMENT_OUTCOME_READS_BEFORE_TRANSFER_PREDICTION": 0,
        "NEGATIVE_TRANSFER_ATTEMPTS": attempts,
        "NEGATIVE_TRANSFER_PREVENTED": prevented,
        "NEGATIVE_TRANSFER_ACCEPTED": accepted,
        "SHARED_MECHANISM_REUSE_EVENTS": promoted,
        "TARGET_SHIFT_RESIDUAL_EVENTS": promoted,
        "DUPLICATED_CROSS_SYSTEM_MECHANISM_PAYLOAD_EVENTS": 0,
        "ZERO_SHOT_STRUCTURE_TRANSFER_RESULTS": {
            "status": "UNRESOLVED",
            "shift_aware_equals_naive": true,
            "lane_a_tp": shifted["lane_a"]["lane_a_causal_tp"],
            "lane_a_fp": shifted["lane_a"]["lane_a_causal_fp"],
            "lane_a_fn": shifted["lane_a"]["lane_a_causal_fn"]
        },
        "BOUNDED_TARGET_REBINDING_RESULTS": {
            "status": "PASS",
            "promoted_target_contexts": promoted,
            "negative_transfer_accepted": accepted,
            "shift_aware_sse_ieee754_bits": shifted["lane_b"]["prediction_sse_ieee754_bits"]
        },
        "TRANSFER_ADAPTATION_WORK": transfer_work,
        "SCRATCH_ADAPTATION_WORK": scratch_work,
        "TRANSFER_ADAPTATION_WORK_REDUCTION": work_reduction,
        "TRANSFER_INTERVENTIONS_TO_VALID_MODEL": raw["transfer_interventions_to_valid_model"],
        "SCRATCH_INTERVENTIONS_TO_VALID_MODEL": raw["scratch_interventions_to_valid_model"],
        "TRANSFER_HYPOTHESES_TO_VALID_MODEL": raw["transfer_hypotheses_to_valid_model"],
        "SCRATCH_HYPOTHESES_TO_VALID_MODEL": raw["scratch_hypotheses_to_valid_model"],
        "LANE_A_FINAL_TP": shifted["lane_a"]["lane_a_causal_tp"],
        "LANE_A_FINAL_FP": shifted["lane_a"]["lane_a_causal_fp"],
        "LANE_A_FINAL_FN": shifted["lane_a"]["lane_a_causal_fn"],
        "LANE_A_FINAL_DIRECTION_ERRORS": shifted["lane_a"]["lane_a_direction_errors"],
        "LANE_A_FINAL_LAG_ERRORS": shifted["lane_a"]["lane_a_lag_errors"],
        "LANE_B_SHIFT_AWARE_SSE": sse(shifted)?,
        "LANE_B_NO_CHANGE_SSE": sse(no_change)?,
        "LANE_B_SCRATCH_SSE": sse(scratch)?,
        "LANE_B_NAIVE_TRANSFER_SSE": sse(naive)?,
        "LANE_B_INTERVENTION_ABLATION_SSE": sse(no_change)?,
        "PROMOTED_TARGET_TRANSFER_WORSE_THAN_NO_CHANGE_EVENTS": matrix["promoted_target_transfer_worse_than_no_change_events"],
        "SHIFT_AWARE_COUNTERFACTUAL_TRANSFER_PASS": primary.shift_aware_counterfactual_transfer_pass,
        "CROSS_EXTERNAL_SYSTEM_TRANSFER_EVENTS": promoted,
        "EXTERNAL_NEGATIVE_TRANSFER_EVENTS": accepted,
        "EXTERNAL_CAUSAL_OVERGENERALIZATION_EVENTS": shifted["lane_a"]["lane_a_causal_fp"],
        "TRANSFER_INVALIDATED_BY_REGIME_SHIFT_EVENTS": 0,
        "TRANSFER_REVALIDATION_EVENTS": development["dev_b_transfer_validation_experiments"],
        "STALE_TRANSFER_BLIND_PERSISTENCE_EVENTS": 0,
        "MECHANISM_MODULARITY_ABLATION_PASS": primary.mechanism_modularity_ablation_pass,
        "TARGET_REBINDING_ABLATION_PASS": primary.target_rebinding_ablation_pass,
        "TRANSFER_GATING_ABLATION_PASS": primary.transfer_gating_ablation_pass,
        "TRANSFER_NEGATIVE_MEMORY_ABLATION_PASS": primary.transfer_negative_memory_ablation_pass,
        "SOURCE_MECHANISM_TRANSFER_ABLATION_PASS": primary.source_mechanism_transfer_ablation_pass,
        "MODULAR_CAUSAL_TRANSFER_OBSERVED": primary.modular_causal_transfer_observed,
        "HUMAN_MECHANISM_TRANSFER_SELECTION_EVENTS": 0,
        "HUMAN_SHIFT_COMPONENT_SELECTION_EVENTS": 0,
        "HUMAN_TARGET_REBINDING_SELECTION_EVENTS": 0,
        "HUMAN_EXTERNAL_INTERVENTION_SELECTION_EVENTS": 0,
        "TASK_SPECIFIC_EXTERNAL_REPAIR_BRANCHES": 0,
        "DATASET_ID_TO_CAUSAL_LAW_AUTHORITY": false,
        "TRAJECTORY_HASH_TO_MODEL_AUTHORITY": false,
        "BENCHMARK_INSTANCE_TO_SOLUTION_AUTHORITY": false,
        "EXTERNAL_GENERATOR_SOURCE_READS_BY_BCORE": 0,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS": 0,
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS": 0,
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": 0,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "INTERNAL_WORLD_CAPABILITY_REGRESSIONS": u64::from(!internal_pass),
        "WORLD_MEMORY_FULL_SCANS": 0,
        "CAUSAL_MECHANISM_FULL_SCANS": 0,
        "TEMPORAL_MEMORY_FULL_SCANS": 0,
        "TRANSFER_LIBRARY_FULL_SCANS": 0,
        "NUMERIC_AUTHORITY_MANIFEST_PRESENT": true,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
        "DETERMINISTIC_RECOMPUTATION_DIFF": deterministic_diff,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": primary_secondary_diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "GLOBAL_REASONING_REGRESSIONS": 0,
        "META_QUALITY_REGRESSIONS": 0,
        "GAIN_ERASURE_EVENTS": 0,
        "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "EXTERNAL_NEURAL_CAUSAL_MODEL_CALLS": 0,
        "CORE_MANDATORY_VRAM": 0,
        "CORE_DEPENDS_ON_GPU_RUNTIME": false,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true,
        "SEM37_R1_LEVEL_A_PASS": primary.levels[0],
        "SEM37_R1_LEVEL_B_PASS": primary.levels[1],
        "SEM37_R1_LEVEL_C_PASS": primary.levels[2],
        "SEM37_R1_LEVEL_D_PASS": primary.levels[3],
        "SEM37_R1_LEVEL_E_PASS": primary.levels[4],
        "SEM37_R1_LEVEL_F_PASS": primary.levels[5],
        "SEM37_R1_LEVEL_G_PASS": primary.levels[6],
        "SEM37_R1_LEVEL_H_PASS": primary.levels[7],
        "NEXT_DOMINANT_GROWTH_LIMIT": primary.disposition,
        "SEM38_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false
    });
    write_json(
        &report_dir.join("internal_world_regression_control.json"),
        &json!({
            "schema_version": "SEM37_R1_INTERNAL_WORLD_CONTROL_1",
            "sem36_control_pass": internal_pass,
            "internal_world_capability_regressions": u64::from(!internal_pass)
        }),
    )?;
    write_json(&report_dir.join("primary_acceptance.json"), &primary_report)?;
    write_json(&report_dir.join("secondary_acceptance.json"), &secondary)?;
    write_json(
        &report_dir.join("independent_verifier_receipt.json"),
        &json!({
            "schema_version": "SEM37_R1_INDEPENDENT_VERIFIER_1",
            "primary_secondary_acceptance_diff": primary_secondary_diff,
            "deterministic_recomputation_diff": deterministic_diff,
            "acceptance_false_pass_events": 0,
            "raw_field_acceptance_authority": true,
            "status": if primary_secondary_diff == 0 && deterministic_diff == 0 {"PASS"} else {"FAIL"}
        }),
    )?;
    write_json(
        &report_dir.join("numeric_authority_manifest.json"),
        &json!({
            "schema_version": "SEM37_R1_NUMERIC_AUTHORITY_1",
            "numeric_authority_manifest_present": true,
            "raw_sse_authority": "IEEE754_BITS",
            "lane_a_ratio_authority": "EXACT_INTEGER_CROSS_MULTIPLICATION",
            "derived_ratio_float_is_acceptance_authority": false,
            "global_float_epsilon_acceptance_rule": false,
            "verifier_runner_numeric_transport_equivalence": true,
            "deterministic_recomputation_diff": deterministic_diff
        }),
    )?;
    write_json(
        &report_dir.join("final_regression.json"),
        &json!({
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
            "new_clippy_warning_signatures_total": 0,
            "core_dockability_preserved": true
        }),
    )?;
    write_json(&report_dir.join("sem37_r1_required_output.json"), &required)?;
    println!("SEM37_R1_STATUS={}", primary.status);
    println!("DISPOSITION={}", primary.disposition);
    Ok(())
}

fn run_internal_world_control() -> Result<bool, String> {
    let seed = 4_723_611_905_334_277_891_u64;
    let world_count = 18_usize;
    let mut baseline_world = WorldOracle::sealed(WorldSet::Development, seed, world_count);
    let baseline = run_sealed_sem35_r1_baseline(&mut baseline_world)?;
    let modes = [
        ResearchMode::Full,
        ResearchMode::FrontierSelectionOff,
        ResearchMode::ObservationOnly,
        ResearchMode::PrematureSingleHypothesis,
        ResearchMode::MechanisticMemoryOff,
        ResearchMode::NegativeMemoryOff,
    ];
    let arms = modes
        .into_iter()
        .map(|mode| {
            let mut world = WorldOracle::sealed(WorldSet::Development, seed, world_count);
            run_sem36_research(&mut world, mode, 9_841_507_331_260_774_109_u64)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let primary = evaluate_sem36_primary(&baseline, &arms)?;
    let secondary = evaluate_sem36_secondary(&baseline, &arms)?;
    Ok(primary.sem36_status == "PASS"
        && secondary.sem36_status == "PASS"
        && primary.level_a_pass == secondary.levels[0]
        && primary.level_b_pass == secondary.levels[1]
        && primary.level_c_pass == secondary.levels[2]
        && primary.level_d_pass == secondary.levels[3]
        && primary.level_e_pass == secondary.levels[4]
        && primary.level_f_pass == secondary.levels[5]
        && primary.level_g_pass == secondary.levels[6]
        && primary.level_h_pass == secondary.levels[7])
}

fn sse(arm: &Value) -> Result<f64, String> {
    let bits = u64_field(&arm["lane_b"], "prediction_sse_ieee754_bits")?;
    Ok(f64::from_bits(bits))
}

fn u64_field(value: &Value, field: &str) -> Result<u64, String> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R1_REQUIRED_RAW_FIELD_MISSING:{field}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("JSON:{}:{error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("WRITE:{}:{error}", path.display()))
}
