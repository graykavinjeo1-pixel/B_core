use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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
}
