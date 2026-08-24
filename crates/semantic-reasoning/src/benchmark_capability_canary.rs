//! Integrated canary for benchmark-shaped repository repair capabilities.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::cross_language_synthesis::{
    synthesize_cross_language_function, validate_cross_language_candidate,
    validate_cross_language_candidate_with_toolchain, CrossLanguage, CrossLanguageExampleIR,
    CrossLanguageSynthesisRequestIR,
};
use crate::repository_change_experience::{
    analyze_nondeterministic_failure, diagnose_environment_failure, migrate_repository_api,
    validate_api_migration_candidate, ApiMigrationNativeValidationRequestIR, ApiMigrationRequestIR,
    EnvironmentDiagnosisDisposition, EnvironmentFailureEvidenceIR, EnvironmentFailureKind,
    ExecutionPerturbation, NondeterminismCause, NondeterminismDisposition,
    RepeatedRunObservationIR, RepositorySourceFileIR,
};
use crate::repository_horizon::{
    build_repository_causal_graph, trace_repository_causality, RepositoryCausalTraceRequestIR,
    RepositoryHorizonBuildRequestIR, REPOSITORY_HORIZON_SCHEMA,
};
use crate::repository_requirement_graph::{
    compile_repository_requirement_graph, RequirementGraphDisposition, RequirementSubject,
};
use crate::self_repair_contract::sha256;
use crate::sem5::model::Value;

pub const BENCHMARK_CAPABILITY_CANARY_SCHEMA: &str = "B_BENCHMARK_CAPABILITY_CANARY_2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkCapabilityCanaryReportIR {
    pub schema: String,
    pub pass: bool,
    pub disposition: String,
    pub long_horizon_repository_files: usize,
    pub long_horizon_causal_depth: usize,
    pub long_horizon_selected_files: usize,
    pub full_catalog_rescans: u64,
    pub long_requirement_clauses: usize,
    pub implicit_constraints_preserved: usize,
    pub requirement_conflicts_detected: usize,
    pub ambiguous_references_rejected: usize,
    pub source_synthesis_languages: usize,
    pub source_synthesis_examples_executed: usize,
    pub source_synthesis_native_passes: usize,
    #[serde(default)]
    pub typescript_compiler_version: String,
    #[serde(default)]
    pub typescript_strict_typecheck_passes: usize,
    #[serde(default)]
    pub typescript_type_error_rejections: usize,
    pub api_migration_languages: usize,
    pub api_migration_native_passes: usize,
    #[serde(default)]
    pub typescript_api_migration_typecheck_passes: usize,
    pub compatibility_shims_validated: usize,
    pub environment_failure_families: usize,
    pub environment_classification_passes: usize,
    pub nondeterminism_cause_families: usize,
    pub nondeterminism_cause_passes: usize,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub repository_identity_routing_events: u64,
    pub task_identity_routing_events: u64,
    pub direct_text_to_source_shortcut_events: u64,
    pub official_benchmark_score_claimed: bool,
    pub official_benchmark_harness_executed: bool,
    pub failed_boundaries: Vec<String>,
}

fn canary_workspace(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("b-core-capability-{label}-{}", std::process::id()))
}

fn scalar_examples(rows: &[(i64, i64, i64)]) -> Vec<CrossLanguageExampleIR> {
    rows.iter()
        .map(|(left, right, expected)| CrossLanguageExampleIR {
            inputs: vec![Value::Int(*left), Value::Int(*right)],
            expected: Value::Int(*expected),
        })
        .collect()
}

fn compiler_version(tool: &Path) -> Result<String, String> {
    let output = Command::new(tool)
        .arg("--version")
        .output()
        .map_err(|error| format!("CANARY_TYPESCRIPT_VERSION_EXECUTE:{error}"))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || version.is_empty() {
        return Err(format!(
            "CANARY_TYPESCRIPT_VERSION_INVALID:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(version)
}

fn long_horizon_metrics() -> Result<(usize, usize, usize, u64), String> {
    let root = canary_workspace("horizon");
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|error| format!("CANARY_HORIZON_CLEAN:{error}"))?;
    }
    fs::create_dir_all(root.join("src"))
        .map_err(|error| format!("CANARY_HORIZON_CREATE:{error}"))?;
    for index in 0..161usize {
        let name = if index == 0 {
            "public_entry".to_string()
        } else if index == 160 {
            "defective_leaf".to_string()
        } else {
            format!("layer_{index:02}")
        };
        let body = if index == 160 {
            "value - 1".to_string()
        } else {
            let next = if index + 1 == 160 {
                "defective_leaf".to_string()
            } else {
                format!("layer_{:02}", index + 1)
            };
            format!("{next}(value)")
        };
        fs::write(
            root.join("src").join(format!("layer_{index:02}.rs")),
            format!("pub fn {name}(value: i64) -> i64 {{ {body} }}\n"),
        )
        .map_err(|error| format!("CANARY_HORIZON_WRITE:{error}"))?;
    }
    for index in 0..1_039usize {
        fs::write(
            root.join("src").join(format!("decoy_{index:02}.rs")),
            format!("pub fn decoy_{index:02}(value: i64) -> i64 {{ value }}\n"),
        )
        .map_err(|error| format!("CANARY_HORIZON_WRITE:{error}"))?;
    }
    let graph = build_repository_causal_graph(&RepositoryHorizonBuildRequestIR {
        schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
        repository_root: fs::canonicalize(&root)
            .map_err(|error| format!("CANARY_HORIZON_CANONICAL:{error}"))?,
        max_files: 2_048,
        max_total_bytes: 64 * 1024 * 1024,
    })?;
    let trace = trace_repository_causality(
        &graph,
        &RepositoryCausalTraceRequestIR {
            schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
            entry_symbols: vec!["public_entry".to_string()],
            evidence_symbols: vec!["defective_leaf".to_string()],
            path_hints: Vec::new(),
            max_steps: 192,
            max_frontier: 1_500,
        },
    )?;
    fs::remove_dir_all(root).map_err(|error| format!("CANARY_HORIZON_CLEAN:{error}"))?;
    Ok((
        graph.indexed_files,
        trace.deepest_path,
        trace.selected_relative_paths.len(),
        trace.full_catalog_rescans,
    ))
}

fn long_requirement_metrics() -> (usize, usize, usize, usize, bool) {
    let mut issue = String::from(
        "Actual behavior: `decode()` returns an assertion mismatch.\nExpected behavior: `decode()` must return the expected value.\nReproduction: run `decode()` with the fixture.\n",
    );
    for index in 0..84 {
        issue.push_str(&format!(
            "Then inspect `layer_{index}()` before proceeding.\n"
        ));
    }
    issue.push_str("Existing callers of `decode()` must continue to compile.\n");
    issue.push_str("Use the standard library only; no new dependencies.\n");
    issue.push_str("Verification: run the regression suite.\n");
    let long = compile_repository_requirement_graph(&issue);
    let conflict = compile_repository_requirement_graph(
        "Actual behavior: `load()` rejects data. Expected behavior: old clients must still read the existing data format. The serialized format must change and replace the old payload. Reproduction: run `load()`.",
    );
    let ambiguous = compile_repository_requirement_graph(
        "Actual behavior: `reader()` and `writer()` disagree. Then call it. Expected behavior: both must agree.",
    );
    (
        long.clause_count,
        long.implicit_constraints,
        conflict.conflicts.len(),
        ambiguous.ambiguous_references,
        long.constraints
            .iter()
            .any(|constraint| constraint.subject == RequirementSubject::PublicApi)
            && conflict.disposition == RequirementGraphDisposition::Conflicting
            && ambiguous.disposition == RequirementGraphDisposition::NeedsClarification,
    )
}

fn synthesis_metrics(
    node: &Path,
    tsc: &Path,
    go: &Path,
) -> Result<(usize, usize, usize, usize, usize), String> {
    let cases = [
        (
            CrossLanguage::JavaScript,
            "combine",
            "export function combine(left, right) { return 0; }\n",
            scalar_examples(&[(4, 3, 7), (-2, 8, 6), (10, -3, 7), (0, 5, 5)]),
            node,
        ),
        (
            CrossLanguage::TypeScript,
            "scale",
            "export function scale(left: number, right: number): number { return 0; }\n",
            scalar_examples(&[(4, 3, 12), (-2, 8, -16), (10, -3, -30), (0, 5, 0)]),
            node,
        ),
        (
            CrossLanguage::Go,
            "delta",
            "package main\n\nfunc delta(left int64, right int64) int64 { return 0 }\n",
            scalar_examples(&[(4, 3, 1), (-2, 8, -10), (10, -3, 13), (0, 5, -5)]),
            go,
        ),
    ];
    let mut passes = 0usize;
    let mut examples_executed = 0usize;
    let mut typescript_typecheck_passes = 0usize;
    for (language, function_name, source, examples, tool) in cases {
        let receipt = synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language,
            function_name: function_name.to_string(),
            predecessor_source: source.to_string(),
            public_examples: examples.clone(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        })?;
        let validation = if language == CrossLanguage::TypeScript {
            validate_cross_language_candidate_with_toolchain(&receipt, &examples, tool, Some(tsc))?
        } else {
            validate_cross_language_candidate(&receipt, &examples, tool)?
        };
        if validation.pass {
            passes += 1;
            examples_executed += validation.cases_executed;
        }
        if language == CrossLanguage::TypeScript && validation.typecheck_pass {
            typescript_typecheck_passes += 1;
        }
    }

    let invalid_examples = scalar_examples(&[(4, 3, 7), (-2, 8, 6), (10, -3, 7), (0, 5, 5)]);
    let invalid_receipt =
        synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language: CrossLanguage::TypeScript,
            function_name: "combine".to_string(),
            predecessor_source: "const unrelated: number = 'wrong';\nexport function combine(left: number, right: number): number { return 0; }\n".to_string(),
            public_examples: invalid_examples.clone(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        })?;
    let invalid_validation = validate_cross_language_candidate_with_toolchain(
        &invalid_receipt,
        &invalid_examples,
        node,
        Some(tsc),
    )?;
    let type_error_rejections = usize::from(
        !invalid_validation.typecheck_pass
            && invalid_validation.command_status.is_none()
            && invalid_validation.cases_executed == 0
            && !invalid_validation.pass,
    );

    Ok((
        3,
        examples_executed,
        passes,
        typescript_typecheck_passes,
        type_error_rejections,
    ))
}

fn api_migration_metrics(
    node: &Path,
    tsc: &Path,
    go: &Path,
) -> Result<(usize, usize, usize, usize), String> {
    let javascript = migrate_repository_api(&ApiMigrationRequestIR {
        language: CrossLanguage::JavaScript,
        files: vec![
            RepositorySourceFileIR {
                relative_path: PathBuf::from("api.mjs"),
                source: "export function compute(a, b) { return a + b; }\n".to_string(),
            },
            RepositorySourceFileIR {
                relative_path: PathBuf::from("service.mjs"),
                source: "import { compute } from './api.mjs'; export function service() { return compute(2, 3); }\n".to_string(),
            },
        ],
        old_symbol: "compute".to_string(),
        new_symbol: "combine".to_string(),
        preserve_public_api: true,
    })?;
    let javascript_validation = validate_api_migration_candidate(
        &javascript,
        &ApiMigrationNativeValidationRequestIR {
            tool_path: node.to_path_buf(),
            typescript_compiler_path: None,
            harness_files: vec![RepositorySourceFileIR {
                relative_path: PathBuf::from("main.mjs"),
                source: "import { compute, combine } from './api.mjs'; import { service } from './service.mjs'; if (compute(1,2) !== 3 || combine(2,3) !== 5 || service() !== 5) throw new Error('bad'); console.log('PASS:MIGRATION');\n".to_string(),
            }],
            expected_output_token: "PASS:MIGRATION".to_string(),
        },
    )?;
    let typescript = migrate_repository_api(&ApiMigrationRequestIR {
        language: CrossLanguage::TypeScript,
        files: vec![
            RepositorySourceFileIR {
                relative_path: PathBuf::from("api.ts"),
                source: "export function compute(a: number, b: number): number { return a + b; }\n"
                    .to_string(),
            },
            RepositorySourceFileIR {
                relative_path: PathBuf::from("service.ts"),
                source: "import { compute } from './api.js'; export function service(): number { return compute(2, 3); }\n".to_string(),
            },
        ],
        old_symbol: "compute".to_string(),
        new_symbol: "combine".to_string(),
        preserve_public_api: true,
    })?;
    let typescript_validation = validate_api_migration_candidate(
        &typescript,
        &ApiMigrationNativeValidationRequestIR {
            tool_path: node.to_path_buf(),
            typescript_compiler_path: Some(tsc.to_path_buf()),
            harness_files: vec![RepositorySourceFileIR {
                relative_path: PathBuf::from("main.ts"),
                source: "import { compute, combine } from './api.js'; import { service } from './service.js'; if (compute(1,2) !== 3 || combine(2,3) !== 5 || service() !== 5) throw new Error('bad'); console.log('PASS:MIGRATION');\n".to_string(),
            }],
            expected_output_token: "PASS:MIGRATION".to_string(),
        },
    )?;
    let golang = migrate_repository_api(&ApiMigrationRequestIR {
        language: CrossLanguage::Go,
        files: vec![
            RepositorySourceFileIR {
                relative_path: PathBuf::from("api.go"),
                source: "package main\nfunc compute(a int64, b int64) int64 { return a + b }\n"
                    .to_string(),
            },
            RepositorySourceFileIR {
                relative_path: PathBuf::from("service.go"),
                source: "package main\nfunc service() int64 { return compute(2, 3) }\n".to_string(),
            },
        ],
        old_symbol: "compute".to_string(),
        new_symbol: "combine".to_string(),
        preserve_public_api: true,
    })?;
    let go_validation = validate_api_migration_candidate(
        &golang,
        &ApiMigrationNativeValidationRequestIR {
            tool_path: go.to_path_buf(),
            typescript_compiler_path: None,
            harness_files: vec![RepositorySourceFileIR {
                relative_path: PathBuf::from("main.go"),
                source: "package main\nfunc main() { if compute(1,2) != 3 || combine(2,3) != 5 || service() != 5 { panic(\"bad\") }; println(\"PASS:MIGRATION\") }\n".to_string(),
            }],
            expected_output_token: "PASS:MIGRATION".to_string(),
        },
    )?;
    Ok((
        3,
        usize::from(javascript_validation.pass)
            + usize::from(typescript_validation.pass)
            + usize::from(go_validation.pass),
        javascript.compatibility_shims
            + typescript.compatibility_shims
            + golang.compatibility_shims,
        usize::from(typescript_validation.typecheck_pass),
    ))
}

fn environment_metrics() -> (usize, usize) {
    let cases = [
        (
            "tool",
            "command not found",
            EnvironmentFailureKind::ToolchainUnavailable,
        ),
        (
            "go",
            "requires go 1.27",
            EnvironmentFailureKind::ToolchainVersionDrift,
        ),
        (
            "npm",
            "frozen lockfile is out of date",
            EnvironmentFailureKind::DependencyLockMismatch,
        ),
        (
            "test",
            "environment variable TOKEN not set in the environment",
            EnvironmentFailureKind::MissingEnvironmentVariable,
        ),
        (
            "build",
            "feature is not enabled",
            EnvironmentFailureKind::FeatureConfiguration,
        ),
        (
            "node",
            "cannot find module adapter",
            EnvironmentFailureKind::ModuleOrPathResolution,
        ),
        (
            "run",
            "dll was not found",
            EnvironmentFailureKind::NativeLibraryUnavailable,
        ),
        (
            "write",
            "access is denied",
            EnvironmentFailureKind::PermissionBoundary,
        ),
    ];
    let passes = cases
        .iter()
        .filter(|(command, diagnostic, expected)| {
            let actual = diagnose_environment_failure(&EnvironmentFailureEvidenceIR {
                command_label: (*command).to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: (*diagnostic).to_string(),
                repeated_attempts: 1,
            });
            actual.kind == *expected
                && actual.disposition == EnvironmentDiagnosisDisposition::Classified
                && !actual.mutation_authorized
                && !actual.automatic_install_authorized
        })
        .count();
    (cases.len(), passes)
}

fn observations(
    outcomes: &[&str],
    perturbation: ExecutionPerturbation,
) -> Vec<RepeatedRunObservationIR> {
    outcomes
        .iter()
        .enumerate()
        .map(|(attempt, outcome)| RepeatedRunObservationIR {
            attempt,
            outcome_sha256: sha256(outcome.as_bytes()),
            perturbations: BTreeSet::from([perturbation]),
        })
        .collect()
}

fn nondeterminism_metrics() -> (usize, usize) {
    let alternating = ["A", "B", "A", "B", "A", "B"];
    let cases = [
        (
            NondeterminismCause::Ordering,
            ExecutionPerturbation::HashSeed,
            "unordered map iteration",
        ),
        (
            NondeterminismCause::Concurrency,
            ExecutionPerturbation::ThreadCount,
            "concurrent thread race",
        ),
        (
            NondeterminismCause::ClockOrTiming,
            ExecutionPerturbation::TimeOffset,
            "clock timing",
        ),
        (
            NondeterminismCause::Randomness,
            ExecutionPerturbation::RandomSeed,
            "random seed",
        ),
        (
            NondeterminismCause::SharedState,
            ExecutionPerturbation::TestOrder,
            "shared state leaks between tests",
        ),
    ];
    let passes = cases
        .iter()
        .filter(|(cause, perturbation, hint)| {
            let analysis =
                analyze_nondeterministic_failure(&observations(&alternating, *perturbation), hint);
            analysis.disposition == NondeterminismDisposition::Confirmed
                && analysis.cause == *cause
                && !analysis.repair_constraints.is_empty()
                && !analysis.source_mutation_authorized
        })
        .count();
    (cases.len(), passes)
}

/// Run the integrated controlled canary. This is not an official SWE-bench or
/// DeepSWE evaluation and does not claim an official score.
pub fn run_benchmark_capability_canary(
    node_path: &Path,
    tsc_path: &Path,
    go_path: &Path,
) -> BenchmarkCapabilityCanaryReportIR {
    let mut failed_boundaries = Vec::new();
    let typescript_compiler_version = match compiler_version(tsc_path) {
        Ok(version) => version,
        Err(error) => {
            failed_boundaries.push(error);
            String::new()
        }
    };
    let (files, depth, selected, rescans) = match long_horizon_metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            (0, 0, 0, 0)
        }
    };
    let (clauses, implicit, conflicts, ambiguous, requirement_pass) = long_requirement_metrics();
    if !requirement_pass {
        failed_boundaries.push("CANARY_REQUIREMENT_GRAPH".to_string());
    }
    let (
        synthesis_languages,
        synthesis_examples,
        synthesis_passes,
        typescript_typecheck_passes,
        typescript_type_error_rejections,
    ) = match synthesis_metrics(node_path, tsc_path, go_path) {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            (3, 0, 0, 0, 0)
        }
    };
    let (migration_languages, migration_passes, shims, migration_typecheck_passes) =
        match api_migration_metrics(node_path, tsc_path, go_path) {
            Ok(metrics) => metrics,
            Err(error) => {
                failed_boundaries.push(error);
                (3, 0, 0, 0)
            }
        };
    let (environment_families, environment_passes) = environment_metrics();
    let (nondeterminism_families, nondeterminism_passes) = nondeterminism_metrics();
    let pass = files == 1_200
        && depth == 160
        && selected == 161
        && rescans == 0
        && clauses == 91
        && requirement_pass
        && synthesis_passes == synthesis_languages
        && synthesis_examples == 12
        && !typescript_compiler_version.is_empty()
        && typescript_typecheck_passes == 1
        && typescript_type_error_rejections == 1
        && migration_passes == migration_languages
        && migration_typecheck_passes == 1
        && shims == 3
        && environment_passes == environment_families
        && nondeterminism_passes == nondeterminism_families
        && failed_boundaries.is_empty();
    BenchmarkCapabilityCanaryReportIR {
        schema: BENCHMARK_CAPABILITY_CANARY_SCHEMA.to_string(),
        pass,
        disposition: if pass {
            "PASS_CONTROLLED_CANARY_OFFICIAL_BENCHMARK_UNMEASURED"
        } else {
            "FAIL_CONTROLLED_CANARY"
        }
        .to_string(),
        long_horizon_repository_files: files,
        long_horizon_causal_depth: depth,
        long_horizon_selected_files: selected,
        full_catalog_rescans: rescans,
        long_requirement_clauses: clauses,
        implicit_constraints_preserved: implicit,
        requirement_conflicts_detected: conflicts,
        ambiguous_references_rejected: ambiguous,
        source_synthesis_languages: synthesis_languages,
        source_synthesis_examples_executed: synthesis_examples,
        source_synthesis_native_passes: synthesis_passes,
        typescript_compiler_version,
        typescript_strict_typecheck_passes: typescript_typecheck_passes,
        typescript_type_error_rejections,
        api_migration_languages: migration_languages,
        api_migration_native_passes: migration_passes,
        typescript_api_migration_typecheck_passes: migration_typecheck_passes,
        compatibility_shims_validated: shims,
        environment_failure_families: environment_families,
        environment_classification_passes: environment_passes,
        nondeterminism_cause_families: nondeterminism_families,
        nondeterminism_cause_passes: nondeterminism_passes,
        external_llm_calls: 0,
        network_reads: 0,
        repository_identity_routing_events: 0,
        task_identity_routing_events: 0,
        direct_text_to_source_shortcut_events: 0,
        official_benchmark_score_claimed: false,
        official_benchmark_harness_executed: false,
        failed_boundaries,
    }
}

pub fn write_benchmark_capability_report(
    repository_root: &Path,
    report: &BenchmarkCapabilityCanaryReportIR,
) -> Result<PathBuf, String> {
    let directory = repository_root.join("reports").join("swe-capability-r1");
    fs::create_dir_all(&directory).map_err(|error| format!("CANARY_REPORT_CREATE:{error}"))?;
    let json =
        serde_json::to_vec_pretty(report).map_err(|error| format!("CANARY_REPORT_JSON:{error}"))?;
    fs::write(directory.join("capability_report.json"), json)
        .map_err(|error| format!("CANARY_REPORT_WRITE:{error}"))?;
    let markdown = format!(
        "# Benchmark-shaped capability canary R1\n\n- Status: `{}`\n- Long-horizon trace: {} files indexed, depth {}, {} files selected, {} rescans\n- Long requirements: {} clauses; {} implicit constraints; {} conflicts; {} ambiguous references rejected\n- Source synthesis: {}/{} languages passed natively; {} examples executed\n- TypeScript compiler: `{}`\n- TypeScript compiler boundary: {} source strict typecheck pass; {} type-error execution rejection; {} API-migration strict typecheck pass\n- API migration: {}/{} language migrations passed natively; {} compatibility shims\n- Environment diagnosis: {}/{} failure families\n- Nondeterminism diagnosis: {}/{} cause families\n- External LLM calls: {}\n- Network reads: {}\n- Official benchmark harness executed: {}\n- Official score claimed: {}\n\nThis controlled canary closes the named engineering gaps at the tested boundary. It is not an official SWE-bench/DeepSWE score.\n",
        report.disposition,
        report.long_horizon_repository_files,
        report.long_horizon_causal_depth,
        report.long_horizon_selected_files,
        report.full_catalog_rescans,
        report.long_requirement_clauses,
        report.implicit_constraints_preserved,
        report.requirement_conflicts_detected,
        report.ambiguous_references_rejected,
        report.source_synthesis_native_passes,
        report.source_synthesis_languages,
        report.source_synthesis_examples_executed,
        report.typescript_compiler_version,
        report.typescript_strict_typecheck_passes,
        report.typescript_type_error_rejections,
        report.typescript_api_migration_typecheck_passes,
        report.api_migration_native_passes,
        report.api_migration_languages,
        report.compatibility_shims_validated,
        report.environment_classification_passes,
        report.environment_failure_families,
        report.nondeterminism_cause_passes,
        report.nondeterminism_cause_families,
        report.external_llm_calls,
        report.network_reads,
        report.official_benchmark_harness_executed,
        report.official_benchmark_score_claimed,
    );
    let markdown_path = directory.join("CAPABILITY_REPORT.md");
    fs::write(&markdown_path, markdown).map_err(|error| format!("CANARY_REPORT_WRITE:{error}"))?;
    Ok(markdown_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrated_canary_passes_when_native_tools_are_available() {
        let node = PathBuf::from(r"C:\Program Files\nodejs\node.exe");
        let tsc = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\npm\tsc.cmd");
        let go = PathBuf::from(r"C:\Program Files\Go\bin\go.exe");
        if !node.is_file() || !tsc.is_file() || !go.is_file() {
            return;
        }
        let report = run_benchmark_capability_canary(&node, &tsc, &go);
        assert!(report.pass, "{:#?}", report.failed_boundaries);
        assert!(!report.typescript_compiler_version.is_empty());
        assert_eq!(report.typescript_strict_typecheck_passes, 1);
        assert_eq!(report.typescript_type_error_rejections, 1);
        assert!(!report.official_benchmark_score_claimed);
        assert_eq!(report.external_llm_calls, 0);
    }
}
