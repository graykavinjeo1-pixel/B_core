use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use crate::{
    sem9::{integrity::hash_bytes, model::SelfEvaluatorTask},
    sem9r1::{
        integrity::{normalize_non_format_tokens, production_source_hash},
        model::{FailedCandidateFreeze, FormatEquivalenceAudit, R1BuildResults, R1CommandResult},
    },
};

#[derive(Debug, Clone)]
pub struct R1SandboxArtifacts {
    pub baseline_binary: PathBuf,
    pub candidate_binary: PathBuf,
    pub formatted_candidate_source: PathBuf,
    pub build_results: R1BuildResults,
    pub format_audit: FormatEquivalenceAudit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryTaskResult {
    pub task_id: String,
    pub keys: Vec<u64>,
    pub expansions: usize,
}

pub fn raw_candidate_equivalence_probe(root: &Path) -> Result<(usize, usize), String> {
    let raw_source = fs::read_to_string(root.join("reports/sem9/artifacts/candidate_lib.rs"))
        .map_err(|error| error.to_string())?;
    let workspace = root.join("target/sem9-r1/RUN-0002/RAW-CANDIDATE-AUDIT");
    reset_sandbox(root, &workspace)?;
    write_workspace(&workspace, &raw_source)?;
    let build = run_cargo(&workspace, &["build", "--workspace"], 0)?;
    if !build.success {
        return Err("RUN0001_CANDIDATE_EVALUATION_PATH_INVALID:RAW_BUILD".to_string());
    }
    let tasks = crate::sem9::tasks::generate_fresh_tasks(0x9001_2026_0810)
        .into_iter()
        .take(16)
        .collect::<Vec<_>>();
    let binary = binary_path(&workspace);
    let (actual, _) = execute_binary(&binary, &tasks, &workspace.join("diagnostic-input.txt"))?;
    let by_id = actual
        .into_iter()
        .map(|record| (record.task_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let failures = tasks
        .iter()
        .filter(|task| {
            let Some(record) = by_id.get(&task.visible.task_id) else {
                return true;
            };
            record.keys != task.expected_unique_keys
                || record.expansions != task.expected_unique_keys.len()
        })
        .count();
    Ok((tasks.len(), failures))
}

pub fn canonicalize_and_build(
    root: &Path,
    freeze: &FailedCandidateFreeze,
) -> Result<R1SandboxArtifacts, String> {
    let production_before = production_source_hash(root)?;
    let base = root.join("target/sem9-r1/RUN-0002");
    let baseline_workspace = base.join("FROZEN-PREDECESSOR-A");
    let candidate_workspace = base.join("FORMAT-CANONICALIZED-CANDIDATE-D");
    reset_sandbox(root, &baseline_workspace)?;
    reset_sandbox(root, &candidate_workspace)?;
    let baseline_source = fs::read_to_string(root.join("reports/sem9/artifacts/baseline_lib.rs"))
        .map_err(|error| error.to_string())?;
    let failed_source = fs::read_to_string(root.join("reports/sem9/artifacts/candidate_lib.rs"))
        .map_err(|error| error.to_string())?;
    if hash_bytes(failed_source.as_bytes()) != freeze.failed_candidate_source_sha256 {
        return Err("FAILED_CANDIDATE_SOURCE_HASH_MISMATCH".to_string());
    }
    write_workspace(&baseline_workspace, &baseline_source)?;
    write_workspace(&candidate_workspace, &failed_source)?;

    // This is the only repair operation. It is deterministic source layout canonicalization.
    let canonicalize = run_cargo(&candidate_workspace, &["fmt", "--all"], 0)?;
    if !canonicalize.success {
        return Err("FORMAT_CANONICALIZATION_FAILURE".to_string());
    }
    let formatted_path = candidate_workspace.join("src/lib.rs");
    let formatted_source =
        fs::read_to_string(&formatted_path).map_err(|error| error.to_string())?;
    let failed_tokens = normalize_non_format_tokens(&failed_source);
    let formatted_tokens = normalize_non_format_tokens(&formatted_source);
    let non_format_token_changes = usize::from(failed_tokens != formatted_tokens);
    let format_audit = FormatEquivalenceAudit {
        failed_candidate_source_sha256: freeze.failed_candidate_source_sha256.clone(),
        formatted_candidate_source_sha256: hash_bytes(formatted_source.as_bytes()),
        failed_token_stream_sha256: hash_bytes(&failed_tokens),
        formatted_token_stream_sha256: hash_bytes(&formatted_tokens),
        non_format_token_changes,
        comments_ignored: 0,
        candidate_mapping_changed: false,
        candidate_assumptions_changed: false,
        candidate_target_changed: false,
        candidate_logic_changed: non_format_token_changes != 0,
        rustfmt_only: true,
        passed: non_format_token_changes == 0,
    };
    if !format_audit.passed {
        return Err("FORMAT_ONLY_EQUIVALENCE_FAILURE".to_string());
    }

    let commands = vec![
        canonicalize,
        run_cargo(&candidate_workspace, &["fmt", "--all", "--", "--check"], 1)?,
        run_cargo(
            &candidate_workspace,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            2,
        )?,
        run_cargo(&candidate_workspace, &["test", "--workspace"], 3)?,
        run_cargo(&candidate_workspace, &["build", "--workspace"], 3)?,
    ];
    let fmt_pass = commands.get(1).is_some_and(|command| command.success);
    let clippy_pass = commands.get(2).is_some_and(|command| command.success);
    let tests_pass = commands.get(3).is_some_and(|command| command.success);
    let candidate_build_pass = commands.get(4).is_some_and(|command| command.success);
    if !(fmt_pass && clippy_pass && tests_pass && candidate_build_pass) {
        return Err("SEM9_R1_CANONICAL_BUILD_GATE_FAILURE".to_string());
    }
    let baseline_format = run_cargo(&baseline_workspace, &["fmt", "--all"], 0)?;
    let baseline_build = run_cargo(&baseline_workspace, &["build", "--workspace"], 0)?;
    if !baseline_format.success || !baseline_build.success {
        return Err("RUN0002_PREDECESSOR_BUILD_FAILURE".to_string());
    }
    let baseline_binary = binary_path(&baseline_workspace);
    let candidate_binary = binary_path(&candidate_workspace);
    if !baseline_binary.is_file() || !candidate_binary.is_file() {
        return Err("SEM9_R1_BINARY_MISSING".to_string());
    }
    let production_after = production_source_hash(root)?;
    let production_mutations = usize::from(production_before != production_after);
    let containment = production_mutations == 0
        && baseline_workspace.starts_with(root.join("target/sem9-r1"))
        && candidate_workspace.starts_with(root.join("target/sem9-r1"));
    let artifact_dir = root.join("reports/sem9-r1/artifacts");
    fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
    let predecessor_artifact = artifact_dir.join(executable_name("run0002_predecessor"));
    let candidate_artifact = artifact_dir.join(executable_name("run0002_candidate"));
    fs::copy(&baseline_binary, &predecessor_artifact).map_err(|error| error.to_string())?;
    fs::copy(&candidate_binary, &candidate_artifact).map_err(|error| error.to_string())?;
    fs::copy(
        &formatted_path,
        artifact_dir.join("formatted_candidate_lib.rs"),
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        artifact_dir.join("format_only.patch"),
        format_only_diff(&failed_source, &formatted_source),
    )
    .map_err(|error| error.to_string())?;
    let build_results = R1BuildResults {
        candidate_id: "SEM9-R1-CANDIDATE-0001".to_string(),
        strict_gate_order: vec![
            "SEMANTIC_TOKEN_EQUIVALENCE".to_string(),
            "CARGO_FMT_CHECK".to_string(),
            "CLIPPY_D_WARNINGS".to_string(),
            "CARGO_TEST_WORKSPACE".to_string(),
            "SANDBOX_CONTAINMENT".to_string(),
            "FRESH_BLIND_EVALUATION".to_string(),
            "REGRESSION_MATRIX".to_string(),
            "SELF_APPLICATION_ABLATION".to_string(),
        ],
        commands,
        semantic_token_equivalence_pass: format_audit.passed,
        cargo_fmt_check_pass: fmt_pass,
        clippy_d_warnings_pass: clippy_pass,
        workspace_tests_pass: tests_pass,
        sandbox_containment_pass: containment,
        predecessor_binary_sha256: crate::sem9::integrity::hash_file(&predecessor_artifact)?,
        candidate_binary_sha256: crate::sem9::integrity::hash_file(&candidate_artifact)?,
        production_source_sha256_before: production_before,
        production_source_sha256_after: production_after,
        production_source_mutations: production_mutations,
        canonical_build_gate_pass: fmt_pass
            && clippy_pass
            && tests_pass
            && candidate_build_pass
            && containment,
    };
    Ok(R1SandboxArtifacts {
        baseline_binary,
        candidate_binary,
        formatted_candidate_source: formatted_path,
        build_results,
        format_audit,
    })
}

pub fn execute_binary(
    binary: &Path,
    tasks: &[SelfEvaluatorTask],
    input_path: &Path,
) -> Result<(Vec<BinaryTaskResult>, u128), String> {
    let mut input = String::new();
    for task in tasks {
        let states = task
            .states
            .iter()
            .map(|state| format!("{}:{}", state.canonical_key, state.payload))
            .collect::<Vec<_>>()
            .join(",");
        input.push_str(&format!("{}\t{states}\n", task.visible.task_id));
    }
    if let Some(parent) = input_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(input_path, input).map_err(|error| error.to_string())?;
    let started = Instant::now();
    let output = Command::new(binary)
        .arg(input_path)
        .output()
        .map_err(|error| error.to_string())?;
    let elapsed_ns = started.elapsed().as_nanos();
    if !output.status.success() {
        return Err("R1_BEHAVIORAL_BINARY_EXECUTION_FAILURE".to_string());
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let records = stdout
        .lines()
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let task_id = fields
                .next()
                .ok_or_else(|| "R1_OUTPUT_TASK_ID_MISSING".to_string())?
                .to_string();
            let expansions = fields
                .next()
                .ok_or_else(|| "R1_OUTPUT_EXPANSIONS_MISSING".to_string())?
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
            let keys = fields
                .next()
                .unwrap_or_default()
                .split(',')
                .filter(|value| !value.is_empty())
                .map(|value| value.parse::<u64>().map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, String>>()?;
            Ok(BinaryTaskResult {
                task_id,
                keys,
                expansions,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((records, elapsed_ns))
}

fn write_workspace(path: &Path, lib_source: &str) -> Result<(), String> {
    fs::create_dir_all(path.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        path.join("Cargo.toml"),
        r#"[package]
name = "sem9-sandbox-probe"
version = "0.1.0"
edition = "2021"
publish = false

[workspace]

[[bin]]
name = "reasoner-probe"
path = "src/main.rs"
"#,
    )
    .map_err(|error| error.to_string())?;
    fs::write(path.join("src/lib.rs"), lib_source).map_err(|error| error.to_string())?;
    fs::write(path.join("src/main.rs"), evaluator_harness()).map_err(|error| error.to_string())?;
    Ok(())
}

fn evaluator_harness() -> &'static str {
    r#"use std::{env, fs};

use sem9_sandbox_probe::{schedule, State};

fn main() {
    let path = env::args().nth(1).expect("input path");
    let input = fs::read_to_string(path).expect("read input");
    for line in input.lines() {
        let (task_id, encoded) = line.split_once('\t').expect("task line");
        let states = encoded
            .split(',')
            .filter(|value| !value.is_empty())
            .map(|value| {
                let (key, payload) = value.split_once(':').expect("state");
                State {
                    canonical_key: key.parse().expect("key"),
                    payload: payload.parse().expect("payload"),
                }
            })
            .collect::<Vec<_>>();
        let (keys, expansions) = schedule(&states);
        let keys = keys
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        println!("{task_id}\t{expansions}\t{keys}");
    }
}
"#
}

fn reset_sandbox(root: &Path, target: &Path) -> Result<(), String> {
    let allowed = root.join("target/sem9-r1");
    if !target.starts_with(&allowed) {
        return Err("PRODUCTION_MUTATION_VIOLATION:SEM9_R1_PATH".to_string());
    }
    if target.exists() {
        fs::remove_dir_all(target).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn run_cargo(workspace: &Path, args: &[&str], ordinal: usize) -> Result<R1CommandResult, String> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(workspace)
        .env("CARGO_TARGET_DIR", workspace.join("target-build"))
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(R1CommandResult {
        ordinal,
        command: format!("cargo {}", args.join(" ")),
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout_sha256: hash_bytes(&output.stdout),
        stderr_sha256: hash_bytes(&output.stderr),
    })
}

fn binary_path(workspace: &Path) -> PathBuf {
    workspace
        .join("target-build/debug")
        .join(executable_name("reasoner-probe"))
}

fn executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

fn format_only_diff(before: &str, after: &str) -> String {
    format!(
        "FORMAT_ONLY_CANONICALIZATION\nBEFORE_SHA256={}\nAFTER_SHA256={}\nNON_FORMAT_TOKEN_CHANGES={}\n",
        hash_bytes(before.as_bytes()),
        hash_bytes(after.as_bytes()),
        usize::from(normalize_non_format_tokens(before) != normalize_non_format_tokens(after))
    )
}

pub fn benchmark_specific_branches(source: &str) -> (usize, usize) {
    let generic = ["SEM9-R1-BLIND-", "SEM9-R1-ADV-", "expected_output"]
        .iter()
        .filter(|needle| source.contains(**needle))
        .count();
    let run0001 = ["SEM9-BLIND-", "SEM9-ADV-"]
        .iter()
        .filter(|needle| source.contains(**needle))
        .count();
    (generic, run0001)
}

pub fn semantic_output(states: &[crate::sem9::model::ReasoningState]) -> Vec<u64> {
    states
        .iter()
        .map(|state| state.canonical_key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_source_contains_no_run_specific_branch() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let source = fs::read_to_string(root.join("reports/sem9/artifacts/candidate_lib.rs"))
            .expect("source");
        assert_eq!(benchmark_specific_branches(&source), (0, 0));
    }
}
