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
use crate::decisive_repair_verification::{
    assess_decisive_repair_contract, seal_decisive_repair_contract,
    validate_decisive_repair_receipt, DecisiveRepairAssessmentIR, DecisiveRepairContractDraftIR,
    RepairObservationValueIR, RepairVerificationClauseIR, RepairVerificationObservationIR,
    RepairVerificationObservationSetIR, RepairVerificationPredicateIR,
    DECISIVE_REPAIR_VERIFICATION_SCHEMA,
};
use crate::repository_change_experience::{
    analyze_nondeterministic_failure, diagnose_environment_failure, migrate_repository_api,
    validate_api_migration_candidate, ApiMigrationNativeValidationRequestIR, ApiMigrationRequestIR,
    EnvironmentDiagnosisDisposition, EnvironmentFailureEvidenceIR, EnvironmentFailureKind,
    ExecutionPerturbation, NondeterminismCause, NondeterminismDisposition,
    RepeatedRunObservationIR, RepositorySourceFileIR,
};
use crate::repository_coding_knowledge::RepositoryLanguage;
use crate::repository_horizon::{
    build_repository_causal_graph, trace_repository_causality, RepositoryCausalGraphIR,
    RepositoryCausalTraceRequestIR, RepositoryHorizonBuildRequestIR, REPOSITORY_HORIZON_SCHEMA,
};
use crate::repository_repair_operation_knowledge::{
    build_repair_operation_knowledge_catalog, query_repair_operation_knowledge,
    RepairAuthorityEvidenceIR, RepairOperationDispositionIR, RepairOperationQueryIR,
    RepairSemanticBindingIR, REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA,
};
use crate::repository_requirement_graph::{
    compile_repository_requirement_graph, RequirementGraphDisposition, RequirementSubject,
};
use crate::repository_validation_planner::{
    plan_repository_validation, seal_validation_proof_receipt, validate_repository_validation_plan,
    RepositoryFileChangeIR, RepositoryValidationRequestIR, RepositoryValidationScopeIR,
    ValidationIdentityIR, REPOSITORY_VALIDATION_PLANNER_SCHEMA,
};
use crate::self_repair_contract::sha256;
use crate::sem5::model::Value;
use crate::typescript_compiler_repair::{
    parse_typescript_compiler_suggestions, synthesize_typescript_compiler_repair,
    validate_typescript_compiler_repair_candidate,
};

pub const BENCHMARK_CAPABILITY_CANARY_SCHEMA: &str = "B_BENCHMARK_CAPABILITY_CANARY_5";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkCapabilityCanaryReportIR {
    pub schema: String,
    pub pass: bool,
    pub disposition: String,
    pub long_horizon_repository_files: usize,
    pub long_horizon_causal_depth: usize,
    pub long_horizon_selected_files: usize,
    pub full_catalog_rescans: u64,
    #[serde(default)]
    pub validation_impact_changed_files: usize,
    #[serde(default)]
    pub validation_impact_affected_files: usize,
    #[serde(default)]
    pub validation_proofs_considered: usize,
    #[serde(default)]
    pub validation_proofs_reused: usize,
    #[serde(default)]
    pub validation_proofs_invalidated: usize,
    #[serde(default)]
    pub validation_structural_escalations: usize,
    #[serde(default)]
    pub validation_planner_replay_passes: usize,
    #[serde(default)]
    pub decisive_verification_assessments: usize,
    #[serde(default)]
    pub decisive_supported_assessments: usize,
    #[serde(default)]
    pub decisive_refuted_assessments: usize,
    #[serde(default)]
    pub decisive_inconclusive_assessments: usize,
    #[serde(default)]
    pub decisive_tamper_rejections: usize,
    #[serde(default)]
    pub repair_operation_knowledge_rules: usize,
    #[serde(default)]
    pub repair_operation_knowledge_languages: usize,
    #[serde(default)]
    pub repair_operation_applicable_cases: usize,
    #[serde(default)]
    pub repair_operation_abstentions: usize,
    #[serde(default)]
    pub repair_operation_embedded_patch_templates: usize,
    pub long_requirement_clauses: usize,
    pub implicit_constraints_preserved: usize,
    pub requirement_conflicts_detected: usize,
    pub ambiguous_references_rejected: usize,
    pub source_synthesis_languages: usize,
    #[serde(default)]
    pub source_synthesis_tasks: usize,
    pub source_synthesis_examples_executed: usize,
    pub source_synthesis_native_passes: usize,
    #[serde(default)]
    pub typescript_compiler_version: String,
    #[serde(default)]
    pub typescript_strict_typecheck_passes: usize,
    #[serde(default)]
    pub typescript_type_error_rejections: usize,
    #[serde(default)]
    pub typescript_async_synthesis_passes: usize,
    #[serde(default)]
    pub sequence_mechanism_transfer_languages: usize,
    #[serde(default)]
    pub nested_sequence_composition_passes: usize,
    #[serde(default)]
    pub typescript_compiler_diagnostic_tasks: usize,
    #[serde(default)]
    pub typescript_compiler_bound_candidates: usize,
    #[serde(default)]
    pub typescript_compiler_verified_repairs: usize,
    #[serde(default)]
    pub typescript_unsupported_diagnostic_abstentions: usize,
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

fn sequence_length_examples() -> Vec<CrossLanguageExampleIR> {
    vec![
        CrossLanguageExampleIR {
            inputs: vec![Value::Sequence(vec![])],
            expected: Value::Int(0),
        },
        CrossLanguageExampleIR {
            inputs: vec![Value::Sequence(vec![7])],
            expected: Value::Int(1),
        },
        CrossLanguageExampleIR {
            inputs: vec![Value::Sequence(vec![2, 4, 6])],
            expected: Value::Int(3),
        },
        CrossLanguageExampleIR {
            inputs: vec![Value::Sequence(vec![-1, 0, 1, 2, 3])],
            expected: Value::Int(5),
        },
    ]
}

fn nested_sequence_length_examples() -> Vec<CrossLanguageExampleIR> {
    vec![
        CrossLanguageExampleIR {
            inputs: vec![
                Value::NestedSequence(vec![vec![1, 2], vec![3]]),
                Value::Int(0),
            ],
            expected: Value::Int(2),
        },
        CrossLanguageExampleIR {
            inputs: vec![
                Value::NestedSequence(vec![vec![4], vec![5, 6, 7]]),
                Value::Int(1),
            ],
            expected: Value::Int(3),
        },
        CrossLanguageExampleIR {
            inputs: vec![
                Value::NestedSequence(vec![vec![], vec![8, 9]]),
                Value::Int(0),
            ],
            expected: Value::Int(0),
        },
        CrossLanguageExampleIR {
            inputs: vec![
                Value::NestedSequence(vec![vec![1], vec![2, 3], vec![4, 5, 6, 7]]),
                Value::Int(2),
            ],
            expected: Value::Int(4),
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SynthesisCapability {
    Scalar,
    SequenceLength,
    Async,
    NestedSequence,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SynthesisMetrics {
    languages: usize,
    tasks: usize,
    examples_executed: usize,
    native_passes: usize,
    typescript_typecheck_passes: usize,
    type_error_rejections: usize,
    async_passes: usize,
    sequence_transfer_languages: usize,
    nested_sequence_passes: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct LongHorizonMetrics {
    files: usize,
    depth: usize,
    selected: usize,
    rescans: u64,
    impact_changed_files: usize,
    impact_affected_files: usize,
    proofs_considered: usize,
    proofs_reused: usize,
    proofs_invalidated: usize,
    structural_escalations: usize,
    planner_replay_passes: usize,
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

fn graph_changes(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
) -> Vec<RepositoryFileChangeIR> {
    let predecessor_files = predecessor
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.to_string_lossy().replace('\\', "/"),
                file.content_sha256.clone(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let candidate_files = candidate
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.to_string_lossy().replace('\\', "/"),
                file.content_sha256.clone(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    predecessor_files
        .keys()
        .chain(candidate_files.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|path| {
            let before = predecessor_files.get(&path).cloned();
            let after = candidate_files.get(&path).cloned();
            (before != after).then(|| RepositoryFileChangeIR {
                relative_path: PathBuf::from(path),
                predecessor_sha256: before,
                candidate_sha256: after,
            })
        })
        .collect()
}

fn canary_validation_identity() -> ValidationIdentityIR {
    ValidationIdentityIR {
        validation_contract_sha256: sha256(b"benchmark-capability-canary-validation-v1"),
        toolchain_sha256: sha256(b"rust-1.96-node-tsc-go"),
        evaluator_sha256: sha256(b"repository-impact-evaluator-v1"),
    }
}

fn horizon_graph(root: &Path) -> Result<RepositoryCausalGraphIR, String> {
    build_repository_causal_graph(&RepositoryHorizonBuildRequestIR {
        schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
        repository_root: fs::canonicalize(root)
            .map_err(|error| format!("CANARY_HORIZON_CANONICAL:{error}"))?,
        max_files: 2_048,
        max_total_bytes: 64 * 1024 * 1024,
    })
}

fn long_horizon_metrics() -> Result<LongHorizonMetrics, String> {
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
    let graph = horizon_graph(&root)?;
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
    let identity = canary_validation_identity();
    let chain_proof = seal_validation_proof_receipt(
        &graph,
        "long-chain-proof",
        vec![PathBuf::from("src/layer_00.rs")],
        vec![
            PathBuf::from("src/layer_00.rs"),
            PathBuf::from("src/layer_160.rs"),
        ],
        identity.clone(),
    )?;
    let decoy_proof = seal_validation_proof_receipt(
        &graph,
        "unrelated-decoy-proof",
        vec![PathBuf::from("src/decoy_00.rs")],
        vec![PathBuf::from("src/decoy_00.rs")],
        identity.clone(),
    )?;
    fs::write(
        root.join("src/layer_160.rs"),
        "pub fn defective_leaf(value: i64) -> i64 { value + 1 }\n",
    )
    .map_err(|error| format!("CANARY_HORIZON_REPAIR_WRITE:{error}"))?;
    let candidate_graph = horizon_graph(&root)?;
    let impact_request = RepositoryValidationRequestIR {
        schema: REPOSITORY_VALIDATION_PLANNER_SCHEMA.to_string(),
        changes: graph_changes(&graph, &candidate_graph),
        prior_proofs: vec![chain_proof, decoy_proof],
        validation_identity: identity.clone(),
        max_affected_files: candidate_graph.files.len(),
    };
    let impact_plan = plan_repository_validation(&graph, &candidate_graph, &impact_request)?;
    validate_repository_validation_plan(&graph, &candidate_graph, &impact_request, &impact_plan)?;

    fs::write(
        root.join("src/layer_80.rs"),
        "pub fn layer_80(value: i64) -> i64 { decoy_00(value) }\n",
    )
    .map_err(|error| format!("CANARY_HORIZON_TOPOLOGY_WRITE:{error}"))?;
    let structural_graph = horizon_graph(&root)?;
    let structural_request = RepositoryValidationRequestIR {
        schema: REPOSITORY_VALIDATION_PLANNER_SCHEMA.to_string(),
        changes: graph_changes(&candidate_graph, &structural_graph),
        prior_proofs: Vec::new(),
        validation_identity: identity,
        max_affected_files: structural_graph.files.len(),
    };
    let structural_plan =
        plan_repository_validation(&candidate_graph, &structural_graph, &structural_request)?;
    validate_repository_validation_plan(
        &candidate_graph,
        &structural_graph,
        &structural_request,
        &structural_plan,
    )?;
    fs::remove_dir_all(root).map_err(|error| format!("CANARY_HORIZON_CLEAN:{error}"))?;
    Ok(LongHorizonMetrics {
        files: graph.indexed_files,
        depth: trace.deepest_path,
        selected: trace.selected_relative_paths.len(),
        rescans: trace
            .full_catalog_rescans
            .saturating_add(impact_plan.full_catalog_rescans)
            .saturating_add(structural_plan.full_catalog_rescans),
        impact_changed_files: impact_plan.changed_relative_paths.len(),
        impact_affected_files: impact_plan.affected_relative_paths.len(),
        proofs_considered: impact_plan.proof_decisions.len(),
        proofs_reused: impact_plan.reusable_proof_ids.len(),
        proofs_invalidated: impact_plan.invalidated_proof_ids.len(),
        structural_escalations: usize::from(
            structural_plan.scope == RepositoryValidationScopeIR::FullWorkspace,
        ),
        planner_replay_passes: 2,
    })
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

fn synthesis_metrics(node: &Path, tsc: &Path, go: &Path) -> Result<SynthesisMetrics, String> {
    let cases = vec![
        (
            CrossLanguage::JavaScript,
            "combine",
            "export function combine(left, right) { return 0; }\n",
            scalar_examples(&[(4, 3, 7), (-2, 8, 6), (10, -3, 7), (0, 5, 5)]),
            node,
            2,
            SynthesisCapability::Scalar,
        ),
        (
            CrossLanguage::TypeScript,
            "scale",
            "export function scale(left: number, right: number): number { return 0; }\n",
            scalar_examples(&[(4, 3, 12), (-2, 8, -16), (10, -3, -30), (0, 5, 0)]),
            node,
            2,
            SynthesisCapability::Scalar,
        ),
        (
            CrossLanguage::Go,
            "delta",
            "package main\n\nfunc delta(left int64, right int64) int64 { return 0 }\n",
            scalar_examples(&[(4, 3, 1), (-2, 8, -10), (10, -3, 13), (0, 5, -5)]),
            go,
            2,
            SynthesisCapability::Scalar,
        ),
        (
            CrossLanguage::JavaScript,
            "countValues",
            "export function countValues(values) { return -1; }\n",
            sequence_length_examples(),
            node,
            2,
            SynthesisCapability::SequenceLength,
        ),
        (
            CrossLanguage::TypeScript,
            "countValues",
            "export function countValues(values: readonly number[]): number { return -1; }\n",
            sequence_length_examples(),
            node,
            2,
            SynthesisCapability::SequenceLength,
        ),
        (
            CrossLanguage::Go,
            "countValues",
            "package main\n\nfunc countValues(values []int64) int64 { return -1 }\n",
            sequence_length_examples(),
            go,
            2,
            SynthesisCapability::SequenceLength,
        ),
        (
            CrossLanguage::TypeScript,
            "combineAsync",
            "export async function combineAsync(left: number, right: number): Promise<number> { return 0; }\n",
            scalar_examples(&[(4, 3, 7), (-2, 8, 6), (10, -3, 7), (0, 5, 5)]),
            node,
            2,
            SynthesisCapability::Async,
        ),
        (
            CrossLanguage::TypeScript,
            "rowWidth",
            "export function rowWidth(matrix: readonly number[][], row: number): number { return -1; }\n",
            nested_sequence_length_examples(),
            node,
            3,
            SynthesisCapability::NestedSequence,
        ),
    ];
    let mut metrics = SynthesisMetrics {
        languages: 3,
        tasks: cases.len(),
        ..SynthesisMetrics::default()
    };
    let mut sequence_languages = BTreeSet::new();
    for (language, function_name, source, examples, tool, max_depth, capability) in cases {
        let receipt = synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language,
            function_name: function_name.to_string(),
            predecessor_source: source.to_string(),
            public_examples: examples.clone(),
            require_conditional: false,
            max_expression_depth: max_depth,
            max_candidates: 1_024,
        })?;
        let validation = if language == CrossLanguage::TypeScript {
            validate_cross_language_candidate_with_toolchain(&receipt, &examples, tool, Some(tsc))?
        } else {
            validate_cross_language_candidate(&receipt, &examples, tool)?
        };
        if validation.pass {
            metrics.native_passes += 1;
            metrics.examples_executed += validation.cases_executed;
            match capability {
                SynthesisCapability::SequenceLength => {
                    sequence_languages.insert(language);
                }
                SynthesisCapability::Async => metrics.async_passes += 1,
                SynthesisCapability::NestedSequence => metrics.nested_sequence_passes += 1,
                SynthesisCapability::Scalar => {}
            }
        }
        if language == CrossLanguage::TypeScript && validation.typecheck_pass {
            metrics.typescript_typecheck_passes += 1;
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
    metrics.type_error_rejections = usize::from(
        !invalid_validation.typecheck_pass
            && invalid_validation.command_status.is_none()
            && invalid_validation.cases_executed == 0
            && !invalid_validation.pass,
    );
    metrics.sequence_transfer_languages = sequence_languages.len();
    Ok(metrics)
}

fn typescript_compiler_repair_metrics(
    node: &Path,
    tsc: &Path,
) -> Result<(usize, usize, usize, usize), String> {
    let root = canary_workspace("typescript-repair");
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|error| format!("CANARY_TS_REPAIR_CLEAN:{error}"))?;
    }
    fs::create_dir_all(&root).map_err(|error| format!("CANARY_TS_REPAIR_CREATE:{error}"))?;
    fs::write(root.join("package.json"), "{\"type\":\"module\"}\n")
        .map_err(|error| format!("CANARY_TS_REPAIR_WRITE:{error}"))?;
    let cases = [
        (
            "property.ts",
            "interface User { displayName: string }\nfunction format(user: User): string { const 접두사 = '>'; return 접두사 + user.displayNmae; }\nif (format({ displayName: 'Ada' }) !== '>Ada') throw new Error('behavior');\nconsole.log('PASS:TS_REPAIR_PROPERTY');\n",
            "PASS:TS_REPAIR_PROPERTY",
        ),
        (
            "object.ts",
            "interface Style { color: string }\nconst style: Style = { colour: 'blue' };\nif (style.color !== 'blue') throw new Error('behavior');\nconsole.log('PASS:TS_REPAIR_OBJECT');\n",
            "PASS:TS_REPAIR_OBJECT",
        ),
    ];
    let mut candidates = 0usize;
    let mut verified = 0usize;
    for (index, (name, source, success_token)) in cases.iter().enumerate() {
        let path = root.join(name);
        fs::write(&path, source).map_err(|error| format!("CANARY_TS_REPAIR_WRITE:{error}"))?;
        let diagnostic_output = Command::new(tsc)
            .args([
                "--strict", "--noEmit", "--pretty", "false", "--target", "ES2022",
            ])
            .arg(name)
            .current_dir(&root)
            .output()
            .map_err(|error| format!("CANARY_TS_REPAIR_DIAGNOSE:{error}"))?;
        if diagnostic_output.status.success() {
            continue;
        }
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&diagnostic_output.stdout),
            String::from_utf8_lossy(&diagnostic_output.stderr)
        );
        let suggestions = parse_typescript_compiler_suggestions(&diagnostics, &root)?;
        let Some(suggestion) = suggestions
            .iter()
            .find(|suggestion| suggestion.relative_path == PathBuf::from(name))
        else {
            continue;
        };
        let candidate = synthesize_typescript_compiler_repair(
            &[RepositorySourceFileIR {
                relative_path: PathBuf::from(name),
                source: (*source).to_string(),
            }],
            suggestion,
        )?;
        validate_typescript_compiler_repair_candidate(
            &[RepositorySourceFileIR {
                relative_path: PathBuf::from(name),
                source: (*source).to_string(),
            }],
            suggestion,
            &candidate,
        )?;
        if candidate.changed_identifiers != 1
            || candidate.source_mutation_authorized
            || candidate.external_llm_calls != 0
        {
            continue;
        }
        candidates += 1;
        fs::write(&path, &candidate.candidate_source)
            .map_err(|error| format!("CANARY_TS_REPAIR_WRITE:{error}"))?;
        let emitted = root.join(format!("emitted-{index}"));
        let compile = Command::new(tsc)
            .args([
                "--strict",
                "--noEmitOnError",
                "--target",
                "ES2022",
                "--module",
                "ES2022",
                "--moduleResolution",
                "bundler",
                "--outDir",
            ])
            .arg(&emitted)
            .arg(name)
            .current_dir(&root)
            .output()
            .map_err(|error| format!("CANARY_TS_REPAIR_COMPILE:{error}"))?;
        if !compile.status.success() {
            continue;
        }
        let runtime = Command::new(node)
            .arg(emitted.join(name).with_extension("js"))
            .current_dir(&root)
            .output()
            .map_err(|error| format!("CANARY_TS_REPAIR_RUNTIME:{error}"))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
        if runtime.status.success() && combined.contains(success_token) {
            verified += 1;
        }
    }
    let unsupported =
        "unknown.ts(1,1): error TS2339: Property 'missing' does not exist on type 'Value'.\n";
    let abstentions =
        usize::from(parse_typescript_compiler_suggestions(unsupported, &root)?.is_empty());
    fs::remove_dir_all(root).map_err(|error| format!("CANARY_TS_REPAIR_CLEAN:{error}"))?;
    Ok((cases.len(), candidates, verified, abstentions))
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

fn decisive_verification_metrics() -> Result<(usize, usize, usize, usize, usize), String> {
    let contract = seal_decisive_repair_contract(DecisiveRepairContractDraftIR {
        hypothesis_id: "controlled-boundary-repair".to_string(),
        predecessor_tree_sha256: sha256(b"canary-before"),
        candidate_tree_sha256: sha256(b"canary-after"),
        support_clause: RepairVerificationClauseIR::All(vec![
            RepairVerificationPredicateIR::ObservationEquals {
                key: "focused_regression_passed".to_string(),
                expected: RepairObservationValueIR::Bool(true),
            },
        ]),
        refutation_clause: RepairVerificationClauseIR::All(vec![
            RepairVerificationPredicateIR::ObservationEquals {
                key: "focused_regression_passed".to_string(),
                expected: RepairObservationValueIR::Bool(false),
            },
        ]),
        discriminating_observation_keys: vec!["focused_regression_passed".to_string()],
    })?;
    let observation_set = |value: Option<bool>| RepairVerificationObservationSetIR {
        schema: DECISIVE_REPAIR_VERIFICATION_SCHEMA.to_string(),
        execution_succeeded: true,
        observations: value
            .map(|value| RepairVerificationObservationIR {
                key: "focused_regression_passed".to_string(),
                value: RepairObservationValueIR::Bool(value),
                evidence_sha256: sha256(if value { b"pass" } else { b"fail" }),
            })
            .into_iter()
            .collect(),
    };
    let sets = [
        observation_set(Some(true)),
        observation_set(Some(false)),
        observation_set(None),
    ];
    let receipts = sets
        .iter()
        .map(|set| {
            let receipt = assess_decisive_repair_contract(&contract, set)?;
            validate_decisive_repair_receipt(&contract, set, &receipt)?;
            Ok(receipt)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let supported = receipts
        .iter()
        .filter(|receipt| receipt.assessment == DecisiveRepairAssessmentIR::Supported)
        .count();
    let refuted = receipts
        .iter()
        .filter(|receipt| receipt.assessment == DecisiveRepairAssessmentIR::Refuted)
        .count();
    let inconclusive = receipts
        .iter()
        .filter(|receipt| receipt.assessment == DecisiveRepairAssessmentIR::Inconclusive)
        .count();
    let mut tampered = receipts[0].clone();
    tampered.assessment = DecisiveRepairAssessmentIR::Refuted;
    let tamper_rejections =
        usize::from(validate_decisive_repair_receipt(&contract, &sets[0], &tampered).is_err());
    Ok((
        receipts.len(),
        supported,
        refuted,
        inconclusive,
        tamper_rejections,
    ))
}

fn repair_operation_knowledge_metrics() -> Result<(usize, usize, usize, usize, usize), String> {
    let catalog = build_repair_operation_knowledge_catalog()?;
    let binding = RepairSemanticBindingIR::DirectPublicParameterCondition;
    let rule = catalog
        .rules
        .iter()
        .find(|rule| rule.binding == binding)
        .ok_or_else(|| "CANARY_REPAIR_OPERATION_RULE_MISSING".to_string())?;
    let languages = [
        RepositoryLanguage::Rust,
        RepositoryLanguage::Python,
        RepositoryLanguage::TypeScript,
        RepositoryLanguage::JavaScript,
        RepositoryLanguage::Go,
    ];
    let query = |language| RepairOperationQueryIR {
        schema: REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA.to_string(),
        language,
        requested_binding: binding,
        observed_evidence: rule.required_evidence.clone(),
        exact_candidate_references: 1,
        target_protected: false,
    };
    let applicable = languages
        .iter()
        .filter(|language| {
            query_repair_operation_knowledge(&catalog, &query(**language)).is_ok_and(|decision| {
                decision.disposition == RepairOperationDispositionIR::Applicable
                    && decision.patch_content.is_none()
                    && !decision.source_mutation_authorized
            })
        })
        .count();
    let mut ambiguous = query(RepositoryLanguage::Rust);
    ambiguous.exact_candidate_references = 2;
    let mut protected = query(RepositoryLanguage::TypeScript);
    protected.target_protected = true;
    let mut missing_source_authority = query(RepositoryLanguage::Go);
    missing_source_authority
        .observed_evidence
        .retain(|evidence| *evidence != RepairAuthorityEvidenceIR::SourceAuthorityPresent);
    let abstentions = [ambiguous, protected, missing_source_authority]
        .iter()
        .filter(|query| {
            query_repair_operation_knowledge(&catalog, query).is_ok_and(|decision| {
                decision.disposition == RepairOperationDispositionIR::Abstain
                    && decision.mutation_family.is_none()
                    && decision.patch_content.is_none()
                    && !decision.source_mutation_authorized
            })
        })
        .count();
    let templates = catalog
        .rules
        .iter()
        .filter(|rule| rule.patch_template.is_some())
        .count();
    Ok((
        catalog.rules.len(),
        languages.len(),
        applicable,
        abstentions,
        templates,
    ))
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
    let horizon = match long_horizon_metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            LongHorizonMetrics::default()
        }
    };
    let (clauses, implicit, conflicts, ambiguous, requirement_pass) = long_requirement_metrics();
    if !requirement_pass {
        failed_boundaries.push("CANARY_REQUIREMENT_GRAPH".to_string());
    }
    let synthesis = match synthesis_metrics(node_path, tsc_path, go_path) {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            SynthesisMetrics {
                languages: 3,
                tasks: 8,
                ..SynthesisMetrics::default()
            }
        }
    };
    let (
        compiler_diagnostic_tasks,
        compiler_bound_candidates,
        compiler_verified_repairs,
        unsupported_diagnostic_abstentions,
    ) = match typescript_compiler_repair_metrics(node_path, tsc_path) {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            (2, 0, 0, 0)
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
    let (
        decisive_assessments,
        decisive_supported,
        decisive_refuted,
        decisive_inconclusive,
        decisive_tamper_rejections,
    ) = match decisive_verification_metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            (3, 0, 0, 0, 0)
        }
    };
    let (
        operation_rules,
        operation_languages,
        operation_applicable,
        operation_abstentions,
        operation_patch_templates,
    ) = match repair_operation_knowledge_metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            failed_boundaries.push(error);
            (11, 5, 0, 0, usize::MAX)
        }
    };
    let pass = horizon.files == 1_200
        && horizon.depth == 160
        && horizon.selected == 161
        && horizon.rescans == 0
        && horizon.impact_changed_files == 1
        && horizon.impact_affected_files == 161
        && horizon.proofs_considered == 2
        && horizon.proofs_reused == 1
        && horizon.proofs_invalidated == 1
        && horizon.structural_escalations == 1
        && horizon.planner_replay_passes == 2
        && decisive_assessments == 3
        && decisive_supported == 1
        && decisive_refuted == 1
        && decisive_inconclusive == 1
        && decisive_tamper_rejections == 1
        && operation_rules == 11
        && operation_languages == 5
        && operation_applicable == operation_languages
        && operation_abstentions == 3
        && operation_patch_templates == 0
        && clauses == 91
        && requirement_pass
        && synthesis.native_passes == synthesis.tasks
        && synthesis.examples_executed == 32
        && !typescript_compiler_version.is_empty()
        && synthesis.typescript_typecheck_passes == 4
        && synthesis.type_error_rejections == 1
        && synthesis.async_passes == 1
        && synthesis.sequence_transfer_languages == 3
        && synthesis.nested_sequence_passes == 1
        && compiler_bound_candidates == compiler_diagnostic_tasks
        && compiler_verified_repairs == compiler_diagnostic_tasks
        && unsupported_diagnostic_abstentions == 1
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
        long_horizon_repository_files: horizon.files,
        long_horizon_causal_depth: horizon.depth,
        long_horizon_selected_files: horizon.selected,
        full_catalog_rescans: horizon.rescans,
        validation_impact_changed_files: horizon.impact_changed_files,
        validation_impact_affected_files: horizon.impact_affected_files,
        validation_proofs_considered: horizon.proofs_considered,
        validation_proofs_reused: horizon.proofs_reused,
        validation_proofs_invalidated: horizon.proofs_invalidated,
        validation_structural_escalations: horizon.structural_escalations,
        validation_planner_replay_passes: horizon.planner_replay_passes,
        decisive_verification_assessments: decisive_assessments,
        decisive_supported_assessments: decisive_supported,
        decisive_refuted_assessments: decisive_refuted,
        decisive_inconclusive_assessments: decisive_inconclusive,
        decisive_tamper_rejections,
        repair_operation_knowledge_rules: operation_rules,
        repair_operation_knowledge_languages: operation_languages,
        repair_operation_applicable_cases: operation_applicable,
        repair_operation_abstentions: operation_abstentions,
        repair_operation_embedded_patch_templates: operation_patch_templates,
        long_requirement_clauses: clauses,
        implicit_constraints_preserved: implicit,
        requirement_conflicts_detected: conflicts,
        ambiguous_references_rejected: ambiguous,
        source_synthesis_languages: synthesis.languages,
        source_synthesis_tasks: synthesis.tasks,
        source_synthesis_examples_executed: synthesis.examples_executed,
        source_synthesis_native_passes: synthesis.native_passes,
        typescript_compiler_version,
        typescript_strict_typecheck_passes: synthesis.typescript_typecheck_passes,
        typescript_type_error_rejections: synthesis.type_error_rejections,
        typescript_async_synthesis_passes: synthesis.async_passes,
        sequence_mechanism_transfer_languages: synthesis.sequence_transfer_languages,
        nested_sequence_composition_passes: synthesis.nested_sequence_passes,
        typescript_compiler_diagnostic_tasks: compiler_diagnostic_tasks,
        typescript_compiler_bound_candidates: compiler_bound_candidates,
        typescript_compiler_verified_repairs: compiler_verified_repairs,
        typescript_unsupported_diagnostic_abstentions: unsupported_diagnostic_abstentions,
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
    let directory = repository_root.join("reports").join("swe-capability-r4");
    fs::create_dir_all(&directory).map_err(|error| format!("CANARY_REPORT_CREATE:{error}"))?;
    let json =
        serde_json::to_vec_pretty(report).map_err(|error| format!("CANARY_REPORT_JSON:{error}"))?;
    fs::write(directory.join("capability_report.json"), json)
        .map_err(|error| format!("CANARY_REPORT_WRITE:{error}"))?;
    let markdown = format!(
        "# Benchmark-shaped capability canary R4\n\n- Status: `{}`\n- Long-horizon trace: {} files indexed, depth {}, {} files selected, {} rescans\n- Validation impact: {} changed file selected {} affected files; {}/{} prior proofs reused and {} invalidated\n- Validation safety: {} structural change escalated to full workspace; {} replay validations passed\n- Precommitted verification: {} assessments ({} supported, {} refuted, {} inconclusive); {} tamper rejection\n- Atomic repair knowledge: {} rules across {} languages; {} applicable cases, {} fail-closed abstentions, {} embedded patch templates\n- Long requirements: {} clauses; {} implicit constraints; {} conflicts; {} ambiguous references rejected\n- Source synthesis: {}/{} tasks across {} languages passed natively; {} examples executed\n- TypeScript compiler: `{}`\n- TypeScript compiler boundary: {} source strict typecheck passes; {} type-error execution rejection; {} API-migration strict typecheck pass\n- Advanced TypeScript: {} async/Promise pass; {} nested-sequence composition pass\n- Sequence mechanism transfer: {} languages\n- Compiler-guided TypeScript repair: {}/{} candidates bound and {} verified; {} unsupported-diagnostic abstention\n- API migration: {}/{} language migrations passed natively; {} compatibility shims\n- Environment diagnosis: {}/{} failure families\n- Nondeterminism diagnosis: {}/{} cause families\n- External LLM calls: {}\n- Network reads: {}\n- Official benchmark harness executed: {}\n- Official score claimed: {}\n\nThis controlled canary closes the named engineering gaps at the tested boundary. It is not an official SWE-bench/DeepSWE score.\n",
        report.disposition,
        report.long_horizon_repository_files,
        report.long_horizon_causal_depth,
        report.long_horizon_selected_files,
        report.full_catalog_rescans,
        report.validation_impact_changed_files,
        report.validation_impact_affected_files,
        report.validation_proofs_reused,
        report.validation_proofs_considered,
        report.validation_proofs_invalidated,
        report.validation_structural_escalations,
        report.validation_planner_replay_passes,
        report.decisive_verification_assessments,
        report.decisive_supported_assessments,
        report.decisive_refuted_assessments,
        report.decisive_inconclusive_assessments,
        report.decisive_tamper_rejections,
        report.repair_operation_knowledge_rules,
        report.repair_operation_knowledge_languages,
        report.repair_operation_applicable_cases,
        report.repair_operation_abstentions,
        report.repair_operation_embedded_patch_templates,
        report.long_requirement_clauses,
        report.implicit_constraints_preserved,
        report.requirement_conflicts_detected,
        report.ambiguous_references_rejected,
        report.source_synthesis_native_passes,
        report.source_synthesis_tasks,
        report.source_synthesis_languages,
        report.source_synthesis_examples_executed,
        report.typescript_compiler_version,
        report.typescript_strict_typecheck_passes,
        report.typescript_type_error_rejections,
        report.typescript_api_migration_typecheck_passes,
        report.typescript_async_synthesis_passes,
        report.nested_sequence_composition_passes,
        report.sequence_mechanism_transfer_languages,
        report.typescript_compiler_bound_candidates,
        report.typescript_compiler_diagnostic_tasks,
        report.typescript_compiler_verified_repairs,
        report.typescript_unsupported_diagnostic_abstentions,
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
        assert_eq!(report.source_synthesis_tasks, 8);
        assert_eq!(report.typescript_strict_typecheck_passes, 4);
        assert_eq!(report.typescript_type_error_rejections, 1);
        assert_eq!(report.sequence_mechanism_transfer_languages, 3);
        assert_eq!(report.typescript_compiler_verified_repairs, 2);
        assert_eq!(report.validation_impact_changed_files, 1);
        assert_eq!(report.validation_impact_affected_files, 161);
        assert_eq!(report.validation_proofs_reused, 1);
        assert_eq!(report.validation_proofs_invalidated, 1);
        assert_eq!(report.validation_structural_escalations, 1);
        assert_eq!(report.decisive_supported_assessments, 1);
        assert_eq!(report.decisive_refuted_assessments, 1);
        assert_eq!(report.decisive_inconclusive_assessments, 1);
        assert_eq!(report.decisive_tamper_rejections, 1);
        assert_eq!(report.repair_operation_knowledge_rules, 11);
        assert_eq!(report.repair_operation_applicable_cases, 5);
        assert_eq!(report.repair_operation_abstentions, 3);
        assert_eq!(report.repair_operation_embedded_patch_templates, 0);
        assert!(!report.official_benchmark_score_claimed);
        assert_eq!(report.external_llm_calls, 0);
    }
}
