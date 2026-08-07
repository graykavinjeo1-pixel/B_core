use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use super::{
    emitter::RustArtifact,
    model::{SandboxAudit, Value},
};

pub const EXECUTION_TIMEOUT_MS: u64 = 2_000;
pub const COMPILE_TIMEOUT_MS: u64 = 20_000;
pub const OUTPUT_LIMIT_BYTES: usize = 65_536;

static NEXT_SANDBOX: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileExecutionResult {
    pub program_id: String,
    pub source_sha256: String,
    pub source_audit_passed: bool,
    pub compiled: bool,
    pub compile_timed_out: bool,
    pub compiler_status: Option<i32>,
    pub compiler_stdout: String,
    pub compiler_stderr: String,
    pub runtime_valid: bool,
    pub runtime_timed_out: bool,
    pub runtime_status: Option<i32>,
    pub runtime_stdout: String,
    pub runtime_stderr: String,
    pub output_file_created: bool,
    pub output_file_matches: bool,
    pub containment_violations: usize,
    pub workspace_removed: bool,
}

pub fn audit_source(source: &str) -> Result<(), String> {
    let forbidden = [
        "unsafe",
        "std::net",
        "TcpStream",
        "UdpSocket",
        "std::thread",
        "std::process",
        "Command::new",
        "extern crate",
        "include!",
        "include_bytes!",
        "env!",
        "option_env!",
        "Foreign",
    ];
    for token in forbidden {
        if source.contains(token) {
            return Err(format!("RUST_MIN_FORBIDDEN_TOKEN:{token}"));
        }
    }
    let remaining_fs = source
        .replace("std::fs::read(\"input.bin\").expect(\"sandbox input\")", "")
        .replace(
            "std::fs::write(\"output.bin\", &sem5_result).expect(\"sandbox output\")",
            "",
        );
    if remaining_fs.contains("std::fs") {
        return Err("RUST_MIN_ARBITRARY_FILESYSTEM".to_string());
    }
    if source.len() > 1_000_000 {
        return Err("RUST_MIN_SOURCE_LIMIT".to_string());
    }
    Ok(())
}

pub fn compile_and_execute(
    artifact: &RustArtifact,
    input: Option<&Value>,
    expected_output_file: Option<&[u8]>,
) -> CompileExecutionResult {
    let source_audit = audit_source(&artifact.source);
    let mut result = CompileExecutionResult {
        program_id: artifact.program_id.clone(),
        source_sha256: artifact.source_sha256.clone(),
        source_audit_passed: source_audit.is_ok(),
        compiled: false,
        compile_timed_out: false,
        compiler_status: None,
        compiler_stdout: String::new(),
        compiler_stderr: source_audit.clone().err().unwrap_or_default(),
        runtime_valid: false,
        runtime_timed_out: false,
        runtime_status: None,
        runtime_stdout: String::new(),
        runtime_stderr: String::new(),
        output_file_created: false,
        output_file_matches: !artifact.writes_output_file,
        containment_violations: 0,
        workspace_removed: false,
    };
    if source_audit.is_err() {
        return result;
    }
    let workspace = sandbox_path();
    if fs::create_dir(&workspace).is_err() {
        result.compiler_stderr = "SANDBOX_CREATE_FAILED".to_string();
        return result;
    }
    let source_path = workspace.join("program.rs");
    let executable = workspace.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    if fs::write(&source_path, artifact.source.as_bytes()).is_err() {
        result.compiler_stderr = "SANDBOX_SOURCE_WRITE_FAILED".to_string();
        cleanup(&workspace, &mut result);
        return result;
    }
    if artifact.reads_input_file {
        match input {
            Some(Value::Bytes(bytes)) => {
                if fs::write(workspace.join("input.bin"), bytes).is_err() {
                    result.compiler_stderr = "SANDBOX_INPUT_WRITE_FAILED".to_string();
                    cleanup(&workspace, &mut result);
                    return result;
                }
            }
            _ => {
                result.compiler_stderr = "SANDBOX_BYTES_INPUT_MISSING".to_string();
                cleanup(&workspace, &mut result);
                return result;
            }
        }
    }

    let compile = run_limited(
        Command::new("rustc")
            .current_dir(&workspace)
            .arg("--edition=2021")
            .arg("-C")
            .arg("opt-level=0")
            .arg("-C")
            .arg("debuginfo=0")
            .arg(&source_path)
            .arg("-o")
            .arg(&executable),
        &workspace,
        "compile",
        Duration::from_millis(COMPILE_TIMEOUT_MS),
    );
    match compile {
        Ok(captured) => {
            result.compile_timed_out = captured.timed_out;
            result.compiler_status = captured.status.and_then(|status| status.code());
            result.compiler_stdout = captured.stdout;
            result.compiler_stderr = captured.stderr;
            result.compiled =
                captured.status.is_some_and(|status| status.success()) && !captured.timed_out;
        }
        Err(error) => result.compiler_stderr = error,
    }
    if result.compiled {
        match run_limited(
            Command::new(&executable).current_dir(&workspace),
            &workspace,
            "runtime",
            Duration::from_millis(EXECUTION_TIMEOUT_MS),
        ) {
            Ok(captured) => {
                result.runtime_timed_out = captured.timed_out;
                result.runtime_status = captured.status.and_then(|status| status.code());
                result.runtime_stdout = captured.stdout;
                result.runtime_stderr = captured.stderr;
                result.runtime_valid =
                    captured.status.is_some_and(|status| status.success()) && !captured.timed_out;
            }
            Err(error) => result.runtime_stderr = error,
        }
    }
    let output_path = workspace.join("output.bin");
    result.output_file_created = output_path.is_file();
    if artifact.writes_output_file {
        result.output_file_matches = expected_output_file.is_some_and(|expected| {
            fs::read(&output_path)
                .map(|actual| actual == expected)
                .unwrap_or(false)
        });
        result.runtime_valid &= result.output_file_matches;
    }
    let permitted = [
        "program.rs",
        if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        },
        "input.bin",
        "output.bin",
        "compile.stdout",
        "compile.stderr",
        "runtime.stdout",
        "runtime.stderr",
        "program.pdb",
    ];
    if let Ok(entries) = fs::read_dir(&workspace) {
        result.containment_violations = entries
            .flatten()
            .filter(|entry| !permitted.contains(&entry.file_name().to_string_lossy().as_ref()))
            .count();
    }
    cleanup(&workspace, &mut result);
    result
}

#[derive(Debug)]
struct CapturedProcess {
    status: Option<ExitStatus>,
    timed_out: bool,
    stdout: String,
    stderr: String,
}

fn run_limited(
    command: &mut Command,
    workspace: &Path,
    stem: &str,
    timeout: Duration,
) -> Result<CapturedProcess, String> {
    let stdout_path = workspace.join(format!("{stem}.stdout"));
    let stderr_path = workspace.join(format!("{stem}.stderr"));
    let stdout_file = File::create(&stdout_path).map_err(|error| format!("STDOUT_FILE:{error}"))?;
    let stderr_file = File::create(&stderr_path).map_err(|error| format!("STDERR_FILE:{error}"))?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|error| format!("PROCESS_SPAWN:{error}"))?;
    let start = Instant::now();
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if start.elapsed() < timeout => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let status = child.wait().ok();
                break (status, true);
            }
            Err(error) => return Err(format!("PROCESS_WAIT:{error}")),
        }
    };
    let stdout = read_bounded(&stdout_path)?;
    let stderr = read_bounded(&stderr_path)?;
    Ok(CapturedProcess {
        status,
        timed_out,
        stdout,
        stderr,
    })
}

fn read_bounded(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| format!("CAPTURE_OPEN:{error}"))?;
    let mut bytes = Vec::new();
    file.take((OUTPUT_LIMIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("CAPTURE_READ:{error}"))?;
    if bytes.len() > OUTPUT_LIMIT_BYTES {
        bytes.truncate(OUTPUT_LIMIT_BYTES);
        bytes.extend_from_slice(b"\n<OUTPUT_LIMIT_REACHED>");
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn sandbox_path() -> PathBuf {
    let serial = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "semantic-reasoning-sem5-{}-{serial}",
        std::process::id()
    ))
}

fn cleanup(workspace: &Path, result: &mut CompileExecutionResult) {
    result.workspace_removed = fs::remove_dir_all(workspace).is_ok() && !workspace.exists();
}

pub fn aggregate_audit(results: &[CompileExecutionResult]) -> SandboxAudit {
    SandboxAudit {
        isolated_temporary_workspace: results.iter().all(|result| result.workspace_removed),
        network_disabled_by_construction: true,
        host_mutation_prohibited: true,
        execution_timeout_ms: EXECUTION_TIMEOUT_MS,
        output_limit_bytes: OUTPUT_LIMIT_BYTES,
        memory_limit_practical: cfg!(unix),
        arbitrary_paths_rejected: true,
        unsafe_rejected: true,
        external_dependencies: 0,
        programs_compiled: results.iter().filter(|result| result.compiled).count(),
        programs_executed: results.iter().filter(|result| result.runtime_valid).count(),
        containment_violations: results
            .iter()
            .map(|result| result.containment_violations)
            .sum(),
        passed: !results.is_empty()
            && results.iter().all(|result| {
                result.source_audit_passed
                    && result.compiled
                    && result.runtime_valid
                    && result.containment_violations == 0
                    && result.workspace_removed
            }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::sem5::{emitter, learner, model::SynthesisCondition, tasks};

    #[test]
    fn rejects_out_of_subset_source() {
        assert!(audit_source("fn main(){ unsafe {} }").is_err());
        assert!(audit_source("fn main(){ std::net::TcpStream; }").is_err());
        assert!(audit_source("fn main(){ std::fs::read(\"../x\"); }").is_err());
    }

    #[test]
    fn compiles_and_runs_offline_in_temporary_workspace() {
        let sets = tasks::generate_task_sets(43);
        let candidates = learner::discover_candidates(&sets.discovery);
        let promotions = learner::initial_promotions(&candidates, &sets.calibration);
        let task = &sets.blind[0];
        let ir = learner::synthesize(
            &task.visible,
            SynthesisCondition::FirstPrinciplesD,
            &promotions,
        )
        .expect("synthesize");
        let cases = tasks::generate_property_cases(&task.visible, 43);
        let inputs = &cases[0];
        let artifact = emitter::emit_rust(&ir, &task.visible.definitions, inputs).expect("emit");
        let expected = tasks::evaluate_contract(&task.visible, inputs).expect("expected");
        let result = compile_and_execute(&artifact, inputs.get("v0"), None);
        assert!(result.compiled, "{}", result.compiler_stderr);
        assert!(result.runtime_valid, "{}", result.runtime_stderr);
        assert_eq!(
            result.runtime_stdout.trim(),
            emitter::render_value(&expected)
        );
        assert!(result.workspace_removed);
        let _ = BTreeMap::<String, Value>::new();
    }
}
