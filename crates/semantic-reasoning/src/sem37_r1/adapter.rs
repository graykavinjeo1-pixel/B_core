use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct R1ExternalEvaluatorClient {
    python: PathBuf,
    script: PathBuf,
}

impl R1ExternalEvaluatorClient {
    pub fn from_vault(vault: &Path) -> Result<Self, String> {
        let python = PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT\venv\Scripts\python.exe");
        let script = vault.join("sem37_r1_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_R1_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
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
            .map_err(|error| format!("SEM37_R1_SPAWN_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_R1_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_R1_WRITE_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_R1_WAIT_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_R1_PARSE_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_R1_EVALUATOR_REJECTED:{}",
                response["reason"].as_str().unwrap_or("UNKNOWN")
            ));
        }
        Ok(response["response"].clone())
    }

    pub fn verify_fixtures(&self) -> Result<Value, String> {
        self.request(&json!({"action": "verify_fixtures"}))
    }

    pub fn freeze_partitions(&self) -> Result<Value, String> {
        self.request(&json!({"action": "freeze_partitions"}))
    }

    pub fn catalog(&self, set: R1ExternalSet) -> Result<R1ExternalCatalog, String> {
        let value = self.request(&json!({"action": "catalog", "set": set}))?;
        serde_json::from_value(value).map_err(|error| format!("SEM37_R1_CATALOG_SCHEMA:{error}"))
    }

    pub fn observe(
        &self,
        case_id: &str,
        reveal_until: u64,
    ) -> Result<R1ExternalObservation, String> {
        let value = self.request(&json!({
            "action": "observe",
            "case_id": case_id,
            "reveal_until": reveal_until
        }))?;
        serde_json::from_value(value)
            .map_err(|error| format!("SEM37_R1_OBSERVATION_SCHEMA:{error}"))
    }

    pub fn execute_intervention(
        &self,
        case_id: &str,
        prediction_commitment: &str,
    ) -> Result<R1InterventionObservation, String> {
        let value = self.request(&json!({
            "action": "execute_intervention",
            "case_id": case_id,
            "predictions_frozen": true,
            "prediction_commitment": prediction_commitment
        }))?;
        serde_json::from_value(value)
            .map_err(|error| format!("SEM37_R1_INTERVENTION_SCHEMA:{error}"))
    }

    pub fn evaluate(
        &self,
        lane: R1ExternalLane,
        predictions: &[Value],
        prediction_commitment: &str,
    ) -> Result<Value, String> {
        self.request(&json!({
            "action": "evaluate",
            "lane": lane,
            "predictions": predictions,
            "prediction_commitment": prediction_commitment
        }))
    }

    pub fn evaluate_arm_matrix(&self, arms: Value) -> Result<Value, String> {
        self.request(&json!({"action": "evaluate_arm_matrix", "arms": arms}))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum R1ExternalSet {
    R1DevA,
    R1DevB,
    R1FinalC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum R1ExternalLane {
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1CaseDescriptor {
    pub case_id: String,
    pub lane: R1ExternalLane,
    pub set: R1ExternalSet,
    pub entity_count: u64,
    pub channels_per_entity: u64,
    pub time_steps: u64,
    pub supports_passive_observation: bool,
    pub supports_legal_intervention: bool,
    pub supports_counterfactual_verification: bool,
    pub natural_language_is_semantic_authority: bool,
    pub benchmark_family_disclosed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1ExternalCatalog {
    pub set: R1ExternalSet,
    pub cases: Vec<R1CaseDescriptor>,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub expected_external_result_lookups: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1ExternalBinding {
    pub entity: u64,
    pub channel: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct R1InterventionContract {
    pub contract_id: String,
    pub targets: Vec<u64>,
    pub times: Vec<u64>,
    pub intervention_type: String,
    pub values: Value,
    pub query_target: Vec<u64>,
    pub query_time: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct R1ExternalObservation {
    pub case_id: String,
    pub lane: R1ExternalLane,
    pub set: R1ExternalSet,
    pub bindings: Vec<R1ExternalBinding>,
    pub time_start: u64,
    pub time_end_exclusive: u64,
    pub values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub legal_interventions: Vec<R1InterventionContract>,
    pub nonfinite_cells_have_numeric_authority: bool,
    pub missingness_transport: String,
    pub outcome_revealed: bool,
    pub ground_truth_revealed: bool,
    pub generator_source_revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1InterventionObservation {
    pub case_id: String,
    pub post_intervention_values_ieee754_bits: Vec<Vec<u64>>,
    pub query_outcome_ieee754_bits: Vec<u64>,
    pub outcome_revealed_after_prediction: bool,
}
