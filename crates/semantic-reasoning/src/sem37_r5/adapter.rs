use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ExternalEvaluatorClient {
    python: PathBuf,
    script: PathBuf,
}

impl ExternalEvaluatorClient {
    pub fn from_vault(vault: &Path) -> Result<Self, String> {
        let python = PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT\venv\Scripts\python.exe");
        let script = vault.join("sem37_r5_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_R5_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
        }
        Ok(Self { python, script })
    }

    pub fn request(&self, payload: &Value) -> Result<Value, String> {
        let mut child = Command::new(&self.python)
            .arg(&self.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("SEM37_R5_SPAWN_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_R5_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_R5_WRITE_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_R5_WAIT_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_R5_PARSE_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_R5_EVALUATOR_REJECTED:{}",
                response["reason"].as_str().unwrap_or("UNKNOWN")
            ));
        }
        Ok(response["response"].clone())
    }

    pub fn verify_fixtures(&self) -> Result<Value, String> {
        self.request(&json!({"action": "verify_fixtures"}))
    }

    pub fn freeze_dev(&self) -> Result<Value, String> {
        self.request(&json!({"action": "freeze_dev"}))
    }

    pub fn freeze_final(&self) -> Result<Value, String> {
        self.request(&json!({"action": "freeze_final"}))
    }

    pub fn catalog(&self, set: &str) -> Result<ExternalCatalog, String> {
        serde_json::from_value(self.request(&json!({"action": "catalog", "set": set}))?)
            .map_err(|error| format!("SEM37_R5_CATALOG_SCHEMA:{error}"))
    }

    pub fn observe(&self, case_id: &str, reveal_until: u64) -> Result<ExternalObservation, String> {
        serde_json::from_value(self.request(&json!({
            "action": "observe",
            "case_id": case_id,
            "reveal_until": reveal_until
        }))?)
        .map_err(|error| format!("SEM37_R5_OBSERVATION_SCHEMA:{error}"))
    }

    pub fn execute_intervention(
        &self,
        case_id: &str,
        commitment: &str,
    ) -> Result<InterventionObservation, String> {
        serde_json::from_value(self.request(&json!({
            "action": "execute_intervention",
            "case_id": case_id,
            "predictions_frozen": true,
            "prediction_commitment": commitment
        }))?)
        .map_err(|error| format!("SEM37_R5_INTERVENTION_SCHEMA:{error}"))
    }

    pub fn evaluate_predictions(
        &self,
        predictions: &[Value],
        commitment: &str,
    ) -> Result<Value, String> {
        self.request(&json!({
            "action": "evaluate_predictions",
            "predictions": predictions,
            "prediction_commitment": commitment
        }))
    }

    pub fn evaluate_matrix(&self, arms: Value) -> Result<Value, String> {
        self.request(&json!({"action": "evaluate_matrix", "arms": arms}))
    }

    pub fn transfer_regression(&self) -> Result<Value, String> {
        self.request(&json!({"action": "transfer_regression"}))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseDescriptor {
    pub case_id: String,
    pub set: String,
    pub entity_count: u64,
    pub time_steps: u64,
    pub primary_source: u64,
    pub primary_target: u64,
    pub supports_passive_observation: bool,
    pub supports_legal_intervention: bool,
    pub supports_counterfactual_verification: bool,
    pub observed_entity_count: u64,
    pub unobserved_entity_slots_present: bool,
    pub observational_identification_contract: String,
    pub natural_language_is_semantic_authority: bool,
    pub benchmark_family_disclosed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalCatalog {
    pub set: String,
    pub cases: Vec<CaseDescriptor>,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub gold_path_specific_effect_reads: u64,
    pub expected_external_result_lookups: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBinding {
    pub entity: u64,
    pub channel: u64,
    pub observed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegalIntervention {
    pub contract_id: String,
    pub targets: Vec<u64>,
    pub times: Vec<u64>,
    pub intervention_type: String,
    pub values: Value,
    pub query_target: Vec<u64>,
    pub query_time: Vec<f64>,
    pub mediator_intervention_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalObservation {
    pub case_id: String,
    pub set: String,
    pub primary_source: u64,
    pub primary_target: u64,
    pub bindings: Vec<ExternalBinding>,
    pub time_start: u64,
    pub time_end_exclusive: u64,
    pub values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub legal_interventions: Vec<LegalIntervention>,
    pub unavailable_counterfactuals_observed: bool,
    pub outcome_revealed: bool,
    pub ground_truth_revealed: bool,
    pub structure_name_revealed: bool,
    pub generator_source_revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionObservation {
    pub case_id: String,
    pub post_intervention_values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub query_outcome_ieee754_bits: Vec<u64>,
    pub outcome_revealed_after_prediction: bool,
    pub prediction_commitment_verified: bool,
    pub gold_path_labels_revealed: bool,
}

pub fn collect_observations(
    evaluator: &ExternalEvaluatorClient,
    set: &str,
) -> Result<Vec<(CaseDescriptor, ExternalObservation)>, String> {
    let catalog = evaluator.catalog(set)?;
    catalog
        .cases
        .into_iter()
        .map(|descriptor| {
            let observation = evaluator.observe(&descriptor.case_id, descriptor.time_steps)?;
            Ok((descriptor, observation))
        })
        .collect()
}
