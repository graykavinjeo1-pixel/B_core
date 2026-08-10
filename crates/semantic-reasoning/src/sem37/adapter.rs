use std::{
    collections::BTreeSet,
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
        let python = vault.join("venv/Scripts/python.exe");
        let script = vault.join("sem37_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
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
            .map_err(|error| format!("SEM37_SPAWN_EXTERNAL_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_EXTERNAL_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_WRITE_EXTERNAL_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_WAIT_EXTERNAL_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_PARSE_EXTERNAL_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_EXTERNAL_EVALUATOR_REJECTED:{}",
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

    pub fn catalog(&self, set: ExternalSet) -> Result<ExternalCatalog, String> {
        let value = self.request(&json!({"action": "catalog", "set": set}))?;
        serde_json::from_value(value).map_err(|error| format!("SEM37_CATALOG_SCHEMA:{error}"))
    }

    pub fn observe(&self, case_id: &str, reveal_until: u64) -> Result<ExternalObservation, String> {
        let value = self.request(&json!({
            "action": "observe",
            "case_id": case_id,
            "reveal_until": reveal_until
        }))?;
        serde_json::from_value(value).map_err(|error| format!("SEM37_OBSERVATION_SCHEMA:{error}"))
    }

    pub fn execute_intervention(
        &self,
        case_id: &str,
        prediction_commitment: &str,
    ) -> Result<ExternalInterventionObservation, String> {
        let value = self.request(&json!({
            "action": "execute_intervention",
            "case_id": case_id,
            "predictions_frozen": true,
            "prediction_commitment": prediction_commitment
        }))?;
        serde_json::from_value(value)
            .map_err(|error| format!("SEM37_INTERVENTION_OBSERVATION_SCHEMA:{error}"))
    }

    pub fn evaluate(
        &self,
        lane: ExternalLane,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExternalSet {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExternalLane {
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalCaseDescriptor {
    pub case_id: String,
    pub lane: ExternalLane,
    pub set: ExternalSet,
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
pub struct ExternalCatalog {
    pub set: ExternalSet,
    pub cases: Vec<ExternalCaseDescriptor>,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub expected_external_result_lookups: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBinding {
    pub entity: u64,
    pub channel: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalInterventionContract {
    pub contract_id: String,
    pub targets: Vec<u64>,
    pub times: Vec<u64>,
    pub intervention_type: String,
    pub values: Value,
    pub query_target: Vec<u64>,
    pub query_time: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalObservation {
    pub case_id: String,
    pub lane: ExternalLane,
    pub set: ExternalSet,
    pub bindings: Vec<ExternalBinding>,
    pub time_start: u64,
    pub time_end_exclusive: u64,
    pub values_ieee754_bits: Vec<Vec<Option<u64>>>,
    pub legal_interventions: Vec<ExternalInterventionContract>,
    pub nonfinite_cells_have_numeric_authority: bool,
    pub missingness_transport: String,
    pub outcome_revealed: bool,
    pub ground_truth_revealed: bool,
    pub generator_source_revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalInterventionObservation {
    pub case_id: String,
    pub post_intervention_values_ieee754_bits: Vec<Vec<u64>>,
    pub query_outcome_ieee754_bits: Vec<u64>,
    pub outcome_revealed_after_prediction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem36ExternalTransportDisposition {
    Transportable,
    NumericStateGroundingLimit,
    VariableArityLimit,
    MissingObservationLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem36ExternalTransportReceipt {
    pub case_id: String,
    pub disposition: Sem36ExternalTransportDisposition,
    pub exact_integer_bindings: u64,
    pub continuous_numeric_bindings: u64,
    pub active_bindings: u64,
    pub benchmark_specific_causal_hint_branches: u64,
    pub numeric_value_as_new_primitive_events: u64,
}

/// Measures compatibility with the sealed SEM-36 discrete scientific world
/// interface. It does not discretize, round, or invent benchmark-specific
/// semantic properties. A measured failure is required before any repair.
pub fn transport_to_sealed_sem36(
    observation: &ExternalObservation,
) -> Sem36ExternalTransportReceipt {
    let active_bindings = observation.bindings.len() as u64;
    let mut exact_integer_bindings = BTreeSet::new();
    let mut continuous_numeric_bindings = BTreeSet::new();
    let mut missing = false;
    for row in &observation.values_ieee754_bits {
        if row.len() != observation.bindings.len() {
            missing = true;
            continue;
        }
        for (index, bits) in row.iter().enumerate() {
            let Some(bits) = bits else {
                missing = true;
                continue;
            };
            let value = f64::from_bits(*bits);
            if !value.is_finite() {
                missing = true;
            } else if value.fract() == 0.0
                && value >= f64::from(i16::MIN)
                && value <= f64::from(i16::MAX)
            {
                exact_integer_bindings.insert(index);
            } else {
                continuous_numeric_bindings.insert(index);
            }
        }
    }
    let disposition = if missing {
        Sem36ExternalTransportDisposition::MissingObservationLimit
    } else if active_bindings > 6 {
        Sem36ExternalTransportDisposition::VariableArityLimit
    } else if !continuous_numeric_bindings.is_empty() {
        Sem36ExternalTransportDisposition::NumericStateGroundingLimit
    } else {
        Sem36ExternalTransportDisposition::Transportable
    };
    Sem36ExternalTransportReceipt {
        case_id: observation.case_id.clone(),
        disposition,
        exact_integer_bindings: exact_integer_bindings.len() as u64,
        continuous_numeric_bindings: continuous_numeric_bindings.len() as u64,
        active_bindings,
        benchmark_specific_causal_hint_branches: 0,
        numeric_value_as_new_primitive_events: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_values_are_not_silently_discretized_for_sem36() {
        let observation = ExternalObservation {
            case_id: "case".to_string(),
            lane: ExternalLane::A,
            set: ExternalSet::A,
            bindings: vec![ExternalBinding {
                entity: 0,
                channel: 0,
            }],
            time_start: 0,
            time_end_exclusive: 1,
            values_ieee754_bits: vec![vec![Some(1.25_f64.to_bits())]],
            legal_interventions: Vec::new(),
            nonfinite_cells_have_numeric_authority: false,
            missingness_transport: "EXPLICIT_NULL_NO_SENTINEL".to_string(),
            outcome_revealed: false,
            ground_truth_revealed: false,
            generator_source_revealed: false,
        };
        let receipt = transport_to_sealed_sem36(&observation);
        assert_eq!(
            receipt.disposition,
            Sem36ExternalTransportDisposition::NumericStateGroundingLimit
        );
        assert_eq!(receipt.numeric_value_as_new_primitive_events, 0);
        assert_eq!(receipt.benchmark_specific_causal_hint_branches, 0);
    }
}
