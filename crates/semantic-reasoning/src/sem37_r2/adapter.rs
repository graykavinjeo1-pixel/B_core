use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::{json, Value};

use crate::sem37_r1::adapter::{
    R1CaseDescriptor, R1ExternalCatalog, R1ExternalLane, R1ExternalObservation, R1ExternalSet,
    R1InterventionObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R2ExternalSet {
    DevA,
    DevB,
    FinalC,
}

impl R2ExternalSet {
    pub const fn evaluator_name(self) -> &'static str {
        match self {
            Self::DevA => "R2_DEV_A",
            Self::DevB => "R2_DEV_B",
            Self::FinalC => "R2_FINAL_C",
        }
    }

    const fn compatibility_set(self) -> R1ExternalSet {
        match self {
            Self::DevA => R1ExternalSet::R1DevA,
            Self::DevB => R1ExternalSet::R1DevB,
            Self::FinalC => R1ExternalSet::R1FinalC,
        }
    }
}

#[derive(Debug, Clone)]
pub struct R2ExternalEvaluatorClient {
    python: PathBuf,
    script: PathBuf,
}

impl R2ExternalEvaluatorClient {
    pub fn from_vault(vault: &Path) -> Result<Self, String> {
        let python = PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT\venv\Scripts\python.exe");
        let script = vault.join("sem37_r2_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_R2_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
        }
        Ok(Self { python, script })
    }

    fn request(&self, payload: &Value) -> Result<Value, String> {
        let mut child = Command::new(&self.python)
            .arg(&self.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("SEM37_R2_SPAWN_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_R2_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_R2_WRITE_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_R2_WAIT_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_R2_PARSE_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_R2_EVALUATOR_REJECTED:{}",
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

    pub fn catalog(&self, set: R2ExternalSet) -> Result<R1ExternalCatalog, String> {
        let mut value = self.request(&json!({
            "action": "catalog",
            "set": set.evaluator_name()
        }))?;
        value["set"] =
            serde_json::to_value(set.compatibility_set()).map_err(|error| error.to_string())?;
        if let Some(cases) = value["cases"].as_array_mut() {
            for case in cases {
                case["set"] = serde_json::to_value(set.compatibility_set())
                    .map_err(|error| error.to_string())?;
            }
        }
        serde_json::from_value(value).map_err(|error| format!("SEM37_R2_CATALOG_SCHEMA:{error}"))
    }

    pub fn observe(
        &self,
        set: R2ExternalSet,
        case_id: &str,
        reveal_until: u64,
    ) -> Result<R1ExternalObservation, String> {
        let mut value = self.request(&json!({
            "action": "observe",
            "case_id": case_id,
            "reveal_until": reveal_until
        }))?;
        value["set"] =
            serde_json::to_value(set.compatibility_set()).map_err(|error| error.to_string())?;
        serde_json::from_value(value)
            .map_err(|error| format!("SEM37_R2_OBSERVATION_SCHEMA:{error}"))
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
            .map_err(|error| format!("SEM37_R2_INTERVENTION_SCHEMA:{error}"))
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

    pub fn evaluate_matrix(&self, arms: Value) -> Result<Value, String> {
        self.request(&json!({"action": "evaluate_matrix", "arms": arms}))
    }
}

pub fn collect_cases(
    evaluator: &R2ExternalEvaluatorClient,
    set: R2ExternalSet,
) -> Result<Vec<(R1CaseDescriptor, R1ExternalObservation)>, String> {
    let catalog = evaluator.catalog(set)?;
    catalog
        .cases
        .into_iter()
        .map(|descriptor| {
            let observation = evaluator.observe(set, &descriptor.case_id, 160)?;
            Ok((descriptor, observation))
        })
        .collect()
}
