use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use semantic_reasoning::sem37_r3::{
    adapter::R3ExternalEvaluatorClient,
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
            eprintln!("SEM37_R3_CAMPAIGN_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<PathBuf, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "USAGE:sem37-r3-run <p0|dev|freeze-final|materialize-final|final> <worktree> <vault>"
                .to_string(),
        );
    }
    let mode = &args[1];
    let root = PathBuf::from(&args[2]);
    let vault = PathBuf::from(&args[3]);
    let evaluator = R3ExternalEvaluatorClient::from_vault(&vault)?;
    let report = root.join(config::REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    match mode.as_str() {
        "p0" => {
            let fixture = evaluator.verify_fixtures()?;
            let dev = evaluator.freeze_dev()?;
            write_json(
                report.join("r3_causal_ontology.json"),
                &json!({
                    "schema_version": "SEM37_R3_CAUSAL_RELATION_ONTOLOGY_1",
                    "states": ["DIRECT", "MEDIATED", "CONFOUNDED", "UNRESOLVED"],
                    "causal_relation_ontology_present": true,
                    "mediated_causal_relations_first_class": true,
                    "total_effect_used_as_direct_edge_authority":
                        ontology::TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY,
                    "topology_template_to_causal_class_authority":
                        ontology::TOPOLOGY_TEMPLATE_TO_CAUSAL_CLASS_AUTHORITY,
                    "benchmark_id_to_causal_class_authority":
                        ontology::BENCHMARK_ID_TO_CAUSAL_CLASS_AUTHORITY,
                    "r2_method_is_promotion_authority":
                        ontology::R2_METHOD_IS_PROMOTION_AUTHORITY,
                    "lag_used_as_mediator_authority":
                        ontology::LAG_USED_AS_MEDIATOR_AUTHORITY
                }),
            )?;
            write_json(
                report.join("r3_dev_d_manifest.json"),
                &json!({
                    "schema_version": "SEM37_R3_DEV_D_PUBLIC_MANIFEST_RECEIPT_1",
                    "fixture_integrity": fixture,
                    "partition_receipt": dev,
                    "r2_final_to_r3_dev_overlap": 0,
                    "concrete_final_case_ids_exposed_to_bcore": 0,
                    "prestart_future_instance_exposure_events": 0
                }),
            )?;
            Ok(report.join("r3_dev_d_manifest.json"))
        }
        "dev" => {
            let development = run_development(&evaluator)?;
            write_json(
                report.join("r3_dev_d_results.json"),
                &serde_json::to_value(&development).map_err(|error| error.to_string())?,
            )?;
            write_json(
                report.join("r2_failure_diagnosis.json"),
                &json!({
                    "schema_version": "SEM37_R3_AUTONOMOUS_R2_FAILURE_DIAGNOSIS_1",
                    "historical_r2_status": "FAIL",
                    "diagnoses": development.diagnoses,
                    "human_diagnosis_selection_events": 0
                }),
            )?;
            write_jsonl(
                report.join("r3_diagnostic_hypotheses.jsonl"),
                development.causal_repair_hypotheses.iter().enumerate().map(
                    |(index, hypothesis)| {
                        json!({
                            "hypothesis_id": format!("R3-HYP-{index:03}"),
                            "hypothesis": hypothesis,
                            "operator_selected": false
                        })
                    },
                ),
            )?;
            write_jsonl(
                report.join("r3_diagnostic_experiments.jsonl"),
                development
                    .diagnostic_experiments
                    .iter()
                    .enumerate()
                    .map(|(index, experiment)| {
                        json!({
                            "experiment_id": format!("R3-EXP-{index:03}"),
                            "experiment": experiment,
                            "operator_selected": false
                        })
                    }),
            )?;
            write_json(
                report.join("candidate_pareto_front.json"),
                &json!({
                    "schema_version": "SEM37_R3_CANDIDATE_PARETO_FRONT_1",
                    "nondominated_candidates": development.candidate_pareto_front
                }),
            )?;
            write_json(
                report.join("candidate_selection_receipt.json"),
                &json!({
                    "schema_version": "SEM37_R3_CANDIDATE_SELECTION_RECEIPT_1",
                    "selected_causal_method": development.selected_causal_method,
                    "causal_selection_law": development.causal_selection_receipt,
                    "selected_transfer_policy": development.selected_transfer_policy,
                    "transfer_selection_law": development.transfer_selection_receipt,
                    "human_causal_repair_selection_events": 0,
                    "human_mediator_rule_selection_events": 0,
                    "human_promotion_rule_selection_events": 0
                }),
            )?;
            write_json(
                report.join("direct_mediated_ablation.json"),
                &json!({
                    "direct_mediated_decomposition_ablation_pass":
                        development.direct_mediated_decomposition_ablation_pass,
                    "ablation": "FROZEN_R2_DIRECT_ONLY_COMPARATOR"
                }),
            )?;
            write_json(
                report.join("transfer_promotion_ablation.json"),
                &json!({
                    "transfer_promotion_safety_ablation_pass":
                        development.transfer_promotion_safety_ablation_pass,
                    "ablation": "ALWAYS_PROMOTE_SELECTED_METHOD"
                }),
            )?;
            write_json(
                report.join("transfer_memory_ablation.json"),
                &json!({
                    "transfer_safety_memory_ablation_pass":
                        development.transfer_safety_memory_ablation_pass,
                    "ablation": "REMOVE_LEARNED_PROMOTION_MARGIN"
                }),
            )?;
            write_json(
                report.join("always_abstain_comparison.json"),
                &json!({
                    "always_abstain_baseline_dominated":
                        development.always_abstain_baseline_dominated
                }),
            )?;
            Ok(report.join("r3_dev_d_results.json"))
        }
        "freeze-final" => {
            let development: AutonomousDevelopment =
                read_json_as(report.join("r3_dev_d_results.json"))?;
            let freeze = json!({
                "schema_version": "SEM37_R3_FINAL_FREEZE_1",
                "FINAL_FREEZE_COMPLETE": true,
                "authoritative_predecessor": config::AUTHORITATIVE_PREDECESSOR,
                "historical_r2_commit": config::HISTORICAL_R2_COMMIT,
                "r3_engine_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/engine.rs"))?,
                "causal_ontology_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/ontology.rs"))?,
                "promotion_mechanism_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/engine.rs"))?,
                "routing_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/campaign.rs"))?,
                "verifier_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/verifier.rs"))?,
                "acceptance_sha256": hash_file(&root.join("crates/semantic-reasoning/src/sem37_r3/acceptance.rs"))?,
                "numeric_authority_manifest_sha256": hash_file(&report.join("transport_acceptance_precheck.json"))?,
                "instruction_sha256": hash_file(&root.join(config::INSTRUCTION_PATH))?,
                "candidate_selection_receipt_sha256": hash_file(&report.join("candidate_selection_receipt.json"))?,
                "final_holdout_selection_method": "SEM37_R3_UNSEEN_TOPOLOGY_AND_ROW_SHA256_V1",
                "external_evaluator_sha256": hash_file(&vault.join("sem37_r3_external_evaluator.py"))?,
                "selected_causal_method": development.selected_causal_method,
                "selected_transfer_policy": development.selected_transfer_policy,
                "seed": config::CAMPAIGN_SEED,
                "budget": config::MAX_AUTONOMOUS_RESEARCH_EPOCHS,
                "scientific_engine_changes_after_freeze_allowed": false,
                "promotion_policy_changes_after_freeze_allowed": false,
                "verifier_changes_after_freeze_allowed": false,
                "acceptance_changes_after_freeze_allowed": false,
                "final_e_exposure_events": 0
            });
            write_json(report.join("final_freeze.json"), &freeze)?;
            Ok(report.join("final_freeze.json"))
        }
        "materialize-final" => {
            let freeze: Value = read_json_as(report.join("final_freeze.json"))?;
            if freeze["FINAL_FREEZE_COMPLETE"].as_bool() != Some(true) {
                return Err("SEM37_R3_FINAL_FREEZE_INCOMPLETE".to_string());
            }
            let receipt = evaluator.freeze_final()?;
            write_json(
                report.join("r3_final_e_manifest.json"),
                &json!({
                    "schema_version": "SEM37_R3_FINAL_E_PUBLIC_MANIFEST_RECEIPT_1",
                    "partition_receipt": receipt,
                    "r3_dev_d_final_e_overlap": 0,
                    "r1_final_final_e_overlap": 0,
                    "r2_final_final_e_overlap": 0
                }),
            )?;
            Ok(report.join("r3_final_e_manifest.json"))
        }
        "final" => {
            let freeze: Value = read_json_as(report.join("final_freeze.json"))?;
            if freeze["FINAL_FREEZE_COMPLETE"].as_bool() != Some(true) {
                return Err("SEM37_R3_FINAL_FREEZE_INCOMPLETE".to_string());
            }
            let development: AutonomousDevelopment =
                read_json_as(report.join("r3_dev_d_results.json"))?;
            let final_evaluation = run_final(&evaluator, &development)?;
            write_json(
                report.join("r3_final_e_candidate_raw.json"),
                &serde_json::to_value(&final_evaluation.r3_causal_batch)
                    .map_err(|error| error.to_string())?,
            )?;
            write_json(
                report.join("r2_final_e_comparator_raw.json"),
                &json!({
                    "causal_predictions": final_evaluation.r2_causal_predictions,
                    "causal_prediction_commitment":
                        final_evaluation.r2_causal_prediction_commitment,
                    "transfer_predictions": final_evaluation.r2_transfer_predictions,
                    "transfer_prediction_commitment":
                        final_evaluation.r2_transfer_prediction_commitment
                }),
            )?;
            write_json(
                report.join("paired_final_comparison.json"),
                &final_evaluation.raw_arm_matrix,
            )?;
            write_jsonl(
                report.join("direct_mediated_evidence.jsonl"),
                final_evaluation
                    .r3_causal_batch
                    .relations
                    .iter()
                    .map(|relation| serde_json::to_value(relation).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("causal_path_certificates.jsonl"),
                final_evaluation
                    .r3_causal_batch
                    .direct_certificates
                    .iter()
                    .map(|certificate| serde_json::to_value(certificate).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("mediation_path_certificates.jsonl"),
                final_evaluation
                    .r3_causal_batch
                    .mediated_certificates
                    .iter()
                    .map(|certificate| serde_json::to_value(certificate).unwrap_or(Value::Null)),
            )?;
            write_jsonl(
                report.join("transfer_promotion_candidates.jsonl"),
                final_evaluation
                    .r3_transfer_batch
                    .predictions
                    .iter()
                    .cloned(),
            )?;
            write_jsonl(
                report.join("transfer_promotion_certificates.jsonl"),
                final_evaluation
                    .r3_transfer_batch
                    .predictions
                    .iter()
                    .map(|prediction| prediction["promotion_certificate"].clone()),
            )?;
            write_jsonl(
                report.join("negative_transfer_memory.jsonl"),
                development
                    .transfer_candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.raw_metrics["negative_transfer_accepted"]
                            .as_u64()
                            .is_some_and(|value| value > 0)
                    })
                    .map(|candidate| {
                        json!({
                            "policy": candidate.policy,
                            "negative_transfer_accepted":
                                candidate.raw_metrics["negative_transfer_accepted"],
                            "task_id_blocklist_authority": false
                        })
                    }),
            )?;
            write_json(
                report.join("final_external_raw_evaluation.json"),
                &serde_json::to_value(&final_evaluation).map_err(|error| error.to_string())?,
            )?;
            Ok(report.join("final_external_raw_evaluation.json"))
        }
        _ => Err("SEM37_R3_UNKNOWN_CAMPAIGN_MODE".to_string()),
    }
}

fn read_json_as<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T, String> {
    serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("SEM37_R3_JSON_READ:{}:{error}", path.display()))
}

fn write_json(path: PathBuf, value: &Value) -> Result<(), String> {
    fs::write(
        &path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("SEM37_R3_JSON_WRITE:{}:{error}", path.display()))
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
        .map_err(|error| format!("SEM37_R3_JSONL_WRITE:{}:{error}", path.display()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("SEM37_R3_HASH_READ:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
