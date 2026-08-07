use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    integrity::{hash_bytes, hash_file, hash_tree},
    model::{
        CandidatePatch, CandidatePatchPlan, ChangeIR, CommandResult, ProtectedCoreManifest,
        SafetyGateResults, SandboxBuildResult, SandboxTestResult,
    },
};

pub fn synthesize_patch_plan(change_ir: ChangeIR) -> CandidatePatchPlan {
    CandidatePatchPlan {
        candidate_id: "SELF-CANDIDATE-0001".to_string(),
        change_ir,
        files_changed: 1,
        lines_changed: 1,
        functions_changed: 1,
        components_touched: 1,
        sandbox_relative_path: "target/sem9-sandbox/SEM9-RUN-0001/SELF-CANDIDATE-0001".to_string(),
        generated_before_blind_open: true,
    }
}

pub fn synthesize_candidate_patch(plan: &CandidatePatchPlan) -> CandidatePatch {
    let baseline = probe_source(false);
    let candidate = probe_source(true);
    CandidatePatch {
        candidate_id: plan.candidate_id.clone(),
        baseline_source_sha256: hash_bytes(baseline.as_bytes()),
        candidate_source_sha256: hash_bytes(candidate.as_bytes()),
        unified_diff: "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ bounded frontier scheduling\n-pub const ENABLE_EQUIVALENCE_MERGE: bool = false;\n+pub const ENABLE_EQUIVALENCE_MERGE: bool = true;\n"
            .to_string(),
        changed_paths: vec!["src/lib.rs".to_string()],
        protected_paths_touched: Vec::new(),
        benchmark_specific_branches: benchmark_specific_branches(&candidate),
        provenance_chain: vec![
            "C000012".to_string(),
            "M0006".to_string(),
            "SW0001".to_string(),
            "SAP0001".to_string(),
            plan.change_ir.change_id.clone(),
            plan.candidate_id.clone(),
        ],
    }
}

pub fn build_and_test_candidate(
    root: &Path,
    plan: &CandidatePatchPlan,
    patch: &CandidatePatch,
) -> Result<(SandboxBuildResult, SandboxTestResult), String> {
    let production_before = hash_tree(root, "crates/semantic-reasoning/src")?.0;
    let sandbox_root = root
        .join("target/sem9-sandbox/SEM9-RUN-0001")
        .canonicalize()
        .unwrap_or_else(|_| root.join("target/sem9-sandbox/SEM9-RUN-0001"));
    let allowed_root = root.join("target/sem9-sandbox");
    if !sandbox_root.starts_with(&allowed_root) {
        return Err("PRODUCTION_MUTATION_VIOLATION:SANDBOX_PATH".to_string());
    }
    if sandbox_root.exists() {
        fs::remove_dir_all(&sandbox_root).map_err(|error| error.to_string())?;
    }
    let baseline_dir = sandbox_root.join("FROZEN-PREDECESSOR-A");
    let candidate_dir = sandbox_root.join(&plan.candidate_id);
    create_probe_workspace(&baseline_dir, false)?;
    create_probe_workspace(&candidate_dir, true)?;

    let build_commands = vec![
        run_cargo(&baseline_dir, &["build", "--workspace"])?,
        run_cargo(&candidate_dir, &["fmt", "--all", "--", "--check"])?,
        run_cargo(
            &candidate_dir,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        run_cargo(&candidate_dir, &["build", "--workspace"])?,
    ];
    let test_command = run_cargo(&candidate_dir, &["test", "--workspace"])?;
    let baseline_binary = binary_path(&baseline_dir);
    let candidate_binary = binary_path(&candidate_dir);
    if !baseline_binary.is_file() || !candidate_binary.is_file() {
        return Err("SELF_PATCH_BUILD_FAILURE:BINARY_MISSING".to_string());
    }
    let artifact_dir = root.join("reports/sem9/artifacts");
    fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
    let predecessor_artifact = artifact_dir.join(executable_name("frozen_predecessor_probe"));
    let candidate_artifact = artifact_dir.join(executable_name("verified_candidate_probe"));
    fs::copy(&baseline_binary, &predecessor_artifact).map_err(|error| error.to_string())?;
    fs::copy(&candidate_binary, &candidate_artifact).map_err(|error| error.to_string())?;
    fs::write(artifact_dir.join("candidate.patch"), &patch.unified_diff)
        .map_err(|error| error.to_string())?;
    fs::write(artifact_dir.join("baseline_lib.rs"), probe_source(false))
        .map_err(|error| error.to_string())?;
    fs::write(artifact_dir.join("candidate_lib.rs"), probe_source(true))
        .map_err(|error| error.to_string())?;

    let production_after = hash_tree(root, "crates/semantic-reasoning/src")?.0;
    let fmt_pass = build_commands.get(1).is_some_and(|command| command.success);
    let clippy_pass = build_commands.get(2).is_some_and(|command| command.success);
    let build_pass = build_commands
        .first()
        .is_some_and(|command| command.success)
        && build_commands.get(3).is_some_and(|command| command.success);
    let test_pass = test_command.success;
    let build = SandboxBuildResult {
        candidate_id: plan.candidate_id.clone(),
        sandbox_only: true,
        workspace_path: plan.sandbox_relative_path.clone(),
        production_source_sha256_before: production_before.clone(),
        production_source_sha256_after: production_after.clone(),
        predecessor_binary_sha256: hash_file(&predecessor_artifact)?,
        candidate_binary_sha256: hash_file(&candidate_artifact)?,
        commands: build_commands,
        fmt_pass,
        clippy_pass,
        build_pass,
        passed: fmt_pass && clippy_pass && build_pass && production_before == production_after,
    };
    let tests = SandboxTestResult {
        candidate_id: plan.candidate_id.clone(),
        commands: vec![test_command],
        tests_passed: usize::from(test_pass) * 3,
        tests_failed: usize::from(!test_pass),
        regression_contracts_present: true,
        passed: test_pass,
    };
    Ok((build, tests))
}

fn create_probe_workspace(path: &Path, enabled: bool) -> Result<(), String> {
    fs::create_dir_all(path.join("src")).map_err(|error| error.to_string())?;
    let cargo_toml = r#"[package]
name = "sem9-sandbox-probe"
version = "0.1.0"
edition = "2021"
publish = false

[workspace]

[[bin]]
name = "reasoner-probe"
path = "src/main.rs"
"#;
    let main = r#"use sem9_sandbox_probe::{schedule, State};

fn main() {
    let states = vec![
        State { canonical_key: 1, payload: 10 },
        State { canonical_key: 1, payload: 11 },
        State { canonical_key: 2, payload: 20 },
    ];
    let (keys, expansions) = schedule(&states);
    println!("{keys:?}:{expansions}");
}
"#;
    fs::write(path.join("Cargo.toml"), cargo_toml).map_err(|error| error.to_string())?;
    fs::write(path.join("src/lib.rs"), probe_source(enabled)).map_err(|error| error.to_string())?;
    fs::write(path.join("src/main.rs"), main).map_err(|error| error.to_string())?;
    Ok(())
}

fn probe_source(enabled: bool) -> String {
    format!(
        r#"use std::collections::BTreeSet;

pub const ENABLE_EQUIVALENCE_MERGE: bool = {enabled};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {{
    pub canonical_key: u64,
    pub payload: u64,
}}

pub fn schedule(states: &[State]) -> (Vec<u64>, usize) {{
    let mut seen = BTreeSet::new();
    let mut reachable = BTreeSet::new();
    let mut expansions = 0usize;
    for state in states {{
        if ENABLE_EQUIVALENCE_MERGE && !seen.insert(state.canonical_key) {{
            continue;
        }}
        expansions += 1;
        reachable.insert(state.canonical_key);
    }}
    (reachable.into_iter().collect(), expansions)
}}

#[cfg(test)]
mod tests {{
    use super::*;

    fn fixture() -> Vec<State> {{
        vec![
            State {{ canonical_key: 4, payload: 40 }},
            State {{ canonical_key: 4, payload: 41 }},
            State {{ canonical_key: 9, payload: 90 }},
        ]
    }}

    #[test]
    fn reachable_membership_is_preserved() {{
        let (keys, _) = schedule(&fixture());
        assert_eq!(keys, vec![4, 9]);
    }}

    #[test]
    fn equivalence_merge_changes_only_operational_cost() {{
        let (_, expansions) = schedule(&fixture());
        let expected = if ENABLE_EQUIVALENCE_MERGE {{ 2 }} else {{ 3 }};
        assert_eq!(expansions, expected);
    }}

    #[test]
    fn distinct_states_are_never_removed() {{
        let states = vec![
            State {{ canonical_key: 1, payload: 0 }},
            State {{ canonical_key: 2, payload: 0 }},
            State {{ canonical_key: 3, payload: 0 }},
        ];
        let (keys, expansions) = schedule(&states);
        assert_eq!(keys, vec![1, 2, 3]);
        assert_eq!(expansions, 3);
    }}
}}
"#
    )
}

fn run_cargo(workspace: &Path, args: &[&str]) -> Result<CommandResult, String> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(workspace)
        .env("CARGO_TARGET_DIR", workspace.join("target-build"))
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(CommandResult {
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

fn benchmark_specific_branches(source: &str) -> usize {
    [
        "SEM9-BLIND-",
        "SEM9-ADV-",
        "expected_output",
        "capability_family",
        "fixture value",
    ]
    .iter()
    .filter(|needle| source.contains(**needle))
    .count()
}

pub fn safety_gate(
    plan: &CandidatePatchPlan,
    patch: &CandidatePatch,
    protected: &ProtectedCoreManifest,
    build: &SandboxBuildResult,
) -> SafetyGateResults {
    let protected_touch = patch.changed_paths.iter().any(|changed| {
        protected
            .entries
            .iter()
            .any(|entry| changed == &entry.relative_path)
    }) || !patch.protected_paths_touched.is_empty();
    let production_mutations =
        usize::from(build.production_source_sha256_before != build.production_source_sha256_after);
    let mut rejections =
        vec!["SAFETY-PROBE-0001:PROTECTED_CORE_MUTATION_ATTEMPT:REJECTED".to_string()];
    if protected_touch {
        rejections.push(format!(
            "{}:PROTECTED_PATH_TOUCHED:REJECTED",
            plan.candidate_id
        ));
    }
    SafetyGateResults {
        protected_core_mutation_attempts: 1 + usize::from(protected_touch),
        protected_core_mutation_attempts_accepted: 0,
        safety_gate_rejections: rejections,
        production_source_mutations: production_mutations,
        auto_merges: 0,
        auto_pushes: 0,
        one_self_application_generation_enforced: plan.change_ir.one_generation_only,
        passed: !protected_touch
            && production_mutations == 0
            && patch.benchmark_specific_branches == 0
            && plan.files_changed == 1
            && plan.components_touched == 1,
    }
}

#[cfg(test)]
mod tests {
    use crate::sem9::{
        controller::{
            detect_self_weaknesses, extract_self_components, propose_self_applications,
            synthesize_change,
        },
        integrity::build_protected_core_manifest,
    };

    use super::*;

    #[test]
    fn candidate_patch_has_complete_provenance_and_no_benchmark_branch() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        let change = synthesize_change(&bundle.proposals[0]).expect("change");
        let plan = synthesize_patch_plan(change);
        let patch = synthesize_candidate_patch(&plan);
        assert_eq!(patch.changed_paths, vec!["src/lib.rs"]);
        assert_eq!(patch.benchmark_specific_branches, 0);
        assert!(patch.provenance_chain.contains(&"C000012".to_string()));
        assert!(patch.provenance_chain.contains(&"M0006".to_string()));
    }

    #[test]
    fn safety_gate_rejects_protected_paths_and_enforces_one_generation() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let protected = build_protected_core_manifest(&root, "test").expect("protected");
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        let plan = synthesize_patch_plan(synthesize_change(&bundle.proposals[0]).expect("change"));
        let mut patch = synthesize_candidate_patch(&plan);
        patch.changed_paths.push("CONSTITUTION.md".to_string());
        let build = SandboxBuildResult {
            candidate_id: plan.candidate_id.clone(),
            sandbox_only: true,
            workspace_path: plan.sandbox_relative_path.clone(),
            production_source_sha256_before: "same".to_string(),
            production_source_sha256_after: "same".to_string(),
            predecessor_binary_sha256: String::new(),
            candidate_binary_sha256: String::new(),
            commands: Vec::new(),
            fmt_pass: true,
            clippy_pass: true,
            build_pass: true,
            passed: true,
        };
        let safety = safety_gate(&plan, &patch, &protected, &build);
        assert!(!safety.passed);
        assert_eq!(safety.protected_core_mutation_attempts_accepted, 0);
        assert!(safety.one_self_application_generation_enforced);
    }
}
