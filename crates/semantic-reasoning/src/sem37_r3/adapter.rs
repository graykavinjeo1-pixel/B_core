use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::{json, Value};

use crate::sem37_r1::adapter::{
    R1CaseDescriptor, R1ExternalCatalog, R1ExternalLane, R1ExternalObservation, R1ExternalSet,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R3ExternalSet {
    DevD,
    FinalE,
}

impl R3ExternalSet {
    const fn evaluator_name(self) -> &'static str {
        match self {
            Self::DevD => "R3_DEV_D",
            Self::FinalE => "R3_FINAL_E",
        }
    }

    const fn compatibility_set(self) -> R1ExternalSet {
        match self {
            Self::DevD => R1ExternalSet::R1DevA,
            Self::FinalE => R1ExternalSet::R1FinalC,
        }
    }
}

#[derive(Debug, Clone)]
pub struct R3ExternalEvaluatorClient {
    python: PathBuf,
    script: PathBuf,
}

impl R3ExternalEvaluatorClient {
    pub fn from_vault(vault: &Path) -> Result<Self, String> {
        let python = PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT\venv\Scripts\python.exe");
        let script = vault.join("sem37_r3_external_evaluator.py");
        if !python.is_file() || !script.is_file() {
            return Err("SEM37_R3_EXTERNAL_EVALUATOR_RUNTIME_MISSING".to_string());
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
            .map_err(|error| format!("SEM37_R3_SPAWN_EVALUATOR:{error}"))?;
        child
            .stdin
            .take()
            .ok_or("SEM37_R3_EVALUATOR_STDIN_MISSING")?
            .write_all(&serde_json::to_vec(payload).map_err(|error| error.to_string())?)
            .map_err(|error| format!("SEM37_R3_WRITE_EVALUATOR:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("SEM37_R3_WAIT_EVALUATOR:{error}"))?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "SEM37_R3_PARSE_EVALUATOR:{error}:{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if !output.status.success() || response["status"].as_str() != Some("PASS") {
            return Err(format!(
                "SEM37_R3_EVALUATOR_REJECTED:{}",
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

    pub fn catalog(&self, set: R3ExternalSet) -> Result<R1ExternalCatalog, String> {
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
        serde_json::from_value(value).map_err(|error| format!("SEM37_R3_CATALOG_SCHEMA:{error}"))
    }

    pub fn observe(
        &self,
        set: R3ExternalSet,
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
            .map_err(|error| format!("SEM37_R3_OBSERVATION_SCHEMA:{error}"))
    }

    pub fn evaluate_causal(
        &self,
        predictions: &[Value],
        commitment: &str,
    ) -> Result<Value, String> {
        self.request(&json!({
            "action": "evaluate_causal",
            "predictions": predictions,
            "prediction_commitment": commitment
        }))
    }

    pub fn evaluate_transfer_development(
        &self,
        candidates: Value,
        no_change_predictions: &[Value],
    ) -> Result<Value, String> {
        self.request(&json!({
            "action": "evaluate_transfer_development",
            "candidates": candidates,
            "no_change_predictions": no_change_predictions
        }))
    }

    pub fn evaluate_matrix(&self, arms: Value) -> Result<Value, String> {
        self.request(&json!({"action": "evaluate_matrix", "arms": arms}))
    }
}

pub fn collect_cases(
    evaluator: &R3ExternalEvaluatorClient,
    set: R3ExternalSet,
) -> Result<Vec<(R1CaseDescriptor, R1ExternalObservation)>, String> {
    let catalog = evaluator.catalog(set)?;
    catalog
        .cases
        .into_iter()
        .map(|descriptor| {
            let reveal_until = if descriptor.lane == R1ExternalLane::A {
                800
            } else {
                160
            };
            let observation = evaluator.observe(set, &descriptor.case_id, reveal_until)?;
            Ok((descriptor, observation))
        })
        .collect()
}
