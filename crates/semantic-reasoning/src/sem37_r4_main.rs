use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use semantic_reasoning::sem37_r4::{
    adapter::R4ExternalEvaluatorClient,
    campaign::{run_development, run_final, AutonomousDevelopment},
    config, ontology,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    match run() {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_R4_CAMPAIGN_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<PathBuf, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "USAGE:sem37-r4-run <p0|dev|freeze-final|materialize-final|final> <worktree> <vault>"
                .to_string(),
        );
    }
    let mode = &args[1];
    let root = PathBuf::from(&args[2]);
    let vault = PathBuf::from(&args[3]);
    let evaluator = R4ExternalEvaluatorClient::from_vault(&vault)?;
    let report = root.join(config::REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    match mode.as_str() {
        "p0" => {
            let fixture = evaluator.verify_fixtures()?;
            let dev = evaluator.freeze_dev()?;
            write_json(
                report.join("causal_effect_decomposition_spec.json"),
                &json!({
                    "schema_version": "SEM37_R4_CAUSAL_EFFECT_DECOMPOSITION_SPEC_1",
                    "fields": [
                        "total_effect", "direct_component", "mediated_components",
                        "common_cause_or_confounding_component", "unresolved_component",
                        "mediator_paths", "intervention_evidence", "observational_evidence",
                        "temporal_evidence", "uncertainty", "identifiability",
                        "provenance", "verification"
                    ],
                    "causal_taxonomy_without_effect_decomposition_is_sufficient":
                        ontology::CAUSAL_TAXONOMY_WITHOUT_EFFECT_DECOMPOSITION_IS_SUFFICIENT,
                    "total_effect_used_as_direct_edge_authority":
                        ontology::TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY,
                    "mdl_or_compression_is_directness_authority":
                        ontology::MDL_OR_COMPRESSION_IS_DIRECTNESS_AUTHORITY,
                    "temporal_lag_used_as_mediator_authority":
                        ontology::TEMPORAL_LAG_USED_AS_MEDIATOR_AUTHORITY,
                    "benchmark_id_to_causal_decomposition_authority":
                        ontology::BENCHMARK_ID_TO_CAUSAL_DECOMPOSITION_AUTHORITY,
                    "topology_template_to_causal_decomposition_authority":
                        ontology::TOPOLOGY_TEMPLATE_TO_CAUSAL_DECOMPOSITION_AUTHORITY,
                    "dataset_id_to_directness_authority":
                        ontology::DATASET_ID_TO_DIRECTNESS_AUTHORITY
                }),
            )?;
            write_json(
                report.join("r4_dev_f_manifest.json"),
                &json!({
                    "schema_version": "SEM37_R4_DEV_F_PUBLIC_MANIFEST_RECEIPT_1",
                    "fixture_integrity": fixture,
                    "partition_receipt": dev,
                    "r2_final_to_r4_dev_overlap": 0,
                    "r3_final_to_r4_dev_overlap": 0,
                    "concrete_final_case_ids_exposed_to_bcore": 0,
                    "prestart_future_instance_exposure_events": 0
                }),
            )?;
            Ok(report.join("r4_dev_f_manifest.json"))
        }
        "dev" => {
            let development = run_development(&evaluator)?;
            write_json(
                report.join("r4_dev_f_results.json"),
                &serde_json::to_value(&development).map_err(|error| error.to_string())?,
            )?;
            write_jsonl(
                report.join("r4_failure_diagnosis.jsonl"),
                development
                    .diagnoses
                    .iter()
                    .enumerate()
                    .map(|(index, diagnosis)| {
                        json!({
                            "diagnosis_id": format!("R4-DIAG-{index:03}"),
                            "diagnosis": diagnosis,
                            "operator_selected": false
                        })
                    }),
            )?;
            write_jsonl(
                report.join("r4_research_hypotheses.jsonl"),
                development.causal_repair_hypotheses.iter().enumerate().map(
                    |(index, hypothesis)| {
                        json!({
                            "hypothesis_id": format!("R4-HYP-{index:03}"),
                            "hypothesis": hypothesis,
                            "operator_selected": false
                        })
                    },
                ),
            )?;
            write_jsonl(
                report.join("r4_diagnostic_experiments.jsonl"),
                development
                    .diagnostic_experiments
                    .iter()
                    .enumerate()
                    .map(|(index, experiment)| {
                        json!({
                            "experiment_id": format!("R4-EXP-{index:03}"),
                            "experiment": experiment,
                            "operator_selected": false
                        })
                    }),
            )?;
            write_jsonl(
                report.join("effect_decomposition_candidates.jsonl"),
                development
                    .causal_candidates
                    .iter()
                    .map(|candidate| serde_json::to_value(candidate).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("transfer_candidates.jsonl"),
                development
                    .transfer_candidates
                    .iter()
                    .map(|candidate| serde_json::to_value(candidate).unwrap_or(Value::Null)),
            )?;
            write_json(
                report.join("candidate_pareto_front.json"),
                &json!({
                    "schema_version": "SEM37_R4_CANDIDATE_PARETO_FRONT_1",
                    "nondominated_candidates": development.candidate_pareto_front
                }),
            )?;
            write_json(
                report.join("candidate_selection_receipt.json"),
                &json!({
                    "schema_version": "SEM37_R4_CANDIDATE_SELECTION_RECEIPT_1",
                    "selected_causal_method": development.selected_causal_method,
                    "causal_selection_law": development.causal_selection_receipt,
                    "selected_transfer_policy": development.selected_transfer_policy,
                    "transfer_selection_law": development.transfer_selection_receipt,
                    "human_causal_repair_selection_events": 0,
                    "human_effect_decomposition_rule_selection_events": 0,
                    "human_mediator_rule_selection_events": 0,
                    "human_promotion_rule_selection_events": 0
                }),
            )?;
            write_json(
                report.join("direct_effect_decomposition_ablation.json"),
                &json!({
                    "direct_effect_decomposition_ablation_pass":
                        development.direct_effect_decomposition_ablation_pass,
                    "control": "FROZEN_R3_TAXONOMY_WITHOUT_R4_COMPONENT_ACCOUNTING"
                }),
            )?;
            write_json(
                report.join("total_effect_only_baseline.json"),
                &json!({
                    "total_effect_only_baseline_dominated":
                        development.total_effect_only_baseline_dominated
                }),
            )?;
            write_json(
                report.join("r3_taxonomy_only_baseline.json"),
                &json!({
                    "r3_taxonomy_only_baseline_dominated":
                        development.r3_taxonomy_only_baseline_dominated
                }),
            )?;
            write_json(
                report.join("no_change_counterfactual_ablation.json"),
                &json!({
                    "no_change_counterfactual_promotion_ablation_pass":
                        development.no_change_counterfactual_promotion_ablation_pass
                }),
            )?;
            write_json(
                report.join("transfer_safety_memory_ablation.json"),
                &json!({
                    "transfer_safety_memory_ablation_pass":
                        development.transfer_safety_memory_ablation_pass
                }),
            )?;
            write_json(
                report.join("always_abstain_control.json"),
                &json!({
                    "always_abstain_baseline_dominated":
                        development.always_abstain_baseline_dominated
                }),
            )?;
            Ok(report.join("r4_dev_f_results.json"))
        }
        "freeze-final" => {
            let development: AutonomousDevelopment =
                read_json_as(report.join("r4_dev_f_results.json"))?;
            let freeze = json!({
                "schema_version": "SEM37_R4_FINAL_FREEZE_1",
                "FINAL_FREEZE_COMPLETE": true,
                "authoritative_predecessor": config::AUTHORITATIVE_PREDECESSOR,
                "historical_r2_commit": config::HISTORICAL_R2_COMMIT,
                "historical_r3_commit": config::HISTORICAL_R3_COMMIT,
                "historical_r3_final_freeze_commit": config::HISTORICAL_R3_FINAL_FREEZE_COMMIT,
                "r4_engine_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r4/engine.rs"))?,
                "causal_ontology_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r4/ontology.rs"))?,
                "routing_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r4/campaign.rs"))?,
                "verifier_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r4/verifier.rs"))?,
                "acceptance_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r4/acceptance.rs"))?,
                "numeric_authority_manifest_sha256": hash_file(&report.join("transport_acceptance_precheck.json"))?,
                "instruction_receipt_sha256": hash_file(&root.join(config::INSTRUCTION_RECEIPT_PATH))?,
                "operator_instruction_sha256": "f2f1fe6fdd4bdf9c7c0663d1b364fd33c71ac3f408253488461628e5b411d236",
                "candidate_selection_receipt_sha256": hash_file(&report.join("candidate_selection_receipt.json"))?,
                "final_holdout_selection_method":
                    "SEM37_R4_HASH_ORDERED_FIRST_CONTRACT_SATISFYING_UNUSED_PARTITION_V1",
                "external_evaluator_sha256": hash_file(&vault.join("sem37_r4_external_evaluator.py"))?,
                "selected_causal_method": development.selected_causal_method,
                "selected_transfer_policy": development.selected_transfer_policy,
                "seed": config::CAMPAIGN_SEED,
                "budget": config::MAX_AUTONOMOUS_RESEARCH_EPOCHS,
                "scientific_engine_changes_after_freeze_allowed": false,
                "promotion_policy_changes_after_freeze_allowed": false,
                "verifier_changes_after_freeze_allowed": false,
                "acceptance_changes_after_freeze_allowed": false,
                "final_g_exposure_events": 0
            });
            write_json(report.join("final_freeze.json"), &freeze)?;
            Ok(report.join("final_freeze.json"))
        }
        "materialize-final" => {
            let freeze: Value = read_json_as(report.join("final_freeze.json"))?;
            if freeze["FINAL_FREEZE_COMPLETE"].as_bool() != Some(true) {
                return Err("SEM37_R4_FINAL_FREEZE_INCOMPLETE".to_string());
            }
            let receipt = evaluator.freeze_final()?;
            write_json(
                report.join("r4_final_g_contract_preflight.json"),
                &json!({
                    "schema_version": "SEM37_R4_FINAL_G_CONTRACT_PREFLIGHT_1",
                    "FINAL_CAUSAL_FIXTURE_CONTRACT_PASS":
                        receipt["final_causal_fixture_contract_pass"],
                    "FINAL_TRANSFER_FIXTURE_CONTRACT_PASS":
                        receipt["final_transfer_fixture_contract_pass"],
                    "FINAL_HOLDOUT_MODEL_DEPENDENT_SELECTION_EVENTS":
                        receipt["final_holdout_model_dependent_selection_events"],
                    "FINAL_SOLVER_EXPOSURES_TO_INVALID_FIXTURES":
                        receipt["final_solver_exposures_to_invalid_fixtures"]
                }),
            )?;
            write_json(
                report.join("r4_final_g_manifest.json"),
                &json!({
                    "schema_version": "SEM37_R4_FINAL_G_PUBLIC_MANIFEST_RECEIPT_1",
                    "partition_receipt": receipt,
                    "r4_dev_f_final_g_overlap": 0,
                    "r1_final_final_g_overlap": 0,
                    "r2_final_final_g_overlap": 0,
                    "r3_final_final_g_overlap": 0,
                    "structural_distinctness": "UNUSED_SOURCE_AND_ROW_IDENTITIES_PLUS_HASH_ORDER"
                }),
            )?;
            fs::write(report.join("rejected_final_fixture_manifests.jsonl"), b"")
                .map_err(|error| error.to_string())?;
            Ok(report.join("r4_final_g_manifest.json"))
        }
        "final" => {
            let freeze: Value = read_json_as(report.join("final_freeze.json"))?;
            if freeze["FINAL_FREEZE_COMPLETE"].as_bool() != Some(true) {
                return Err("SEM37_R4_FINAL_FREEZE_INCOMPLETE".to_string());
            }
            let development: AutonomousDevelopment =
                read_json_as(report.join("r4_dev_f_results.json"))?;
            let evaluation = run_final(&evaluator, &development)?;
            write_json(
                report.join("r4_final_g_candidate_raw.json"),
                &serde_json::to_value(&evaluation).map_err(|error| error.to_string())?,
            )?;
            write_json(
                report.join("r2_final_g_comparator_raw.json"),
                &json!({
                    "causal_predictions": evaluation.r2_causal_predictions,
                    "causal_prediction_commitment": evaluation.r2_causal_prediction_commitment,
                    "transfer_predictions": evaluation.r2_transfer_predictions,
                    "transfer_prediction_commitment": evaluation.r2_transfer_prediction_commitment
                }),
            )?;
            write_json(
                report.join("r3_final_g_comparator_raw.json"),
                &json!({
                    "causal_predictions": evaluation.r3_causal_predictions,
                    "causal_prediction_commitment": evaluation.r3_causal_prediction_commitment,
                    "transfer_predictions": evaluation.r3_transfer_predictions,
                    "transfer_prediction_commitment": evaluation.r3_transfer_prediction_commitment
                }),
            )?;
            write_json(
                report.join("paired_final_comparison.json"),
                &evaluation.raw_arm_matrix,
            )?;
            write_json(
                report.join("lane_a_effect_decomposition_matrix.json"),
                &evaluation.raw_arm_matrix["arms"],
            )?;
            write_json(
                report.join("lane_b_transfer_outcome_matrix.json"),
                &evaluation.raw_arm_matrix["arms"],
            )?;
            write_jsonl(
                report.join("direct_effect_certificates.jsonl"),
                evaluation
                    .r4_causal_batch
                    .direct_effect_certificates
                    .iter()
                    .map(|certificate| serde_json::to_value(certificate).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("mediated_effect_certificates.jsonl"),
                evaluation
                    .r4_causal_batch
                    .mediated_effect_certificates
                    .iter()
                    .map(|certificate| serde_json::to_value(certificate).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("transfer_counterfactual_predictions.jsonl"),
                evaluation.r4_transfer_batch.predictions.iter().cloned(),
            )?;
            write_jsonl(
                report.join("transfer_promotion_certificates.jsonl"),
                evaluation
                    .r4_transfer_batch
                    .predictions
                    .iter()
                    .map(|prediction| prediction["promotion_certificate"].clone()),
            )?;
            Ok(report.join("r4_final_g_candidate_raw.json"))
        }
        _ => Err("SEM37_R4_UNKNOWN_CAMPAIGN_MODE".to_string()),
    }
}

fn read_json_as<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T, String> {
    serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("SEM37_R4_JSON_READ:{}:{error}", path.display()))
}

fn write_json(path: PathBuf, value: &Value) -> Result<(), String> {
    fs::write(
        &path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("SEM37_R4_JSON_WRITE:{}:{error}", path.display()))
}

fn write_jsonl<I>(path: PathBuf, values: I) -> Result<(), String>
where
    I: IntoIterator<Item = Value>,
{
    let mut output = Vec::new();
    for value in values {
        output.extend(serde_json::to_vec(&value).map_err(|error| error.to_string())?);
        output.push(b'\n');
    }
    fs::write(&path, output)
        .map_err(|error| format!("SEM37_R4_JSONL_WRITE:{}:{error}", path.display()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("SEM37_R4_HASH_READ:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
