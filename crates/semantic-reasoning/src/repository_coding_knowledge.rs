//! Repository-scale coding knowledge for evidence-bound repair planning.
//!
//! This module deliberately stops before source generation. It converts public
//! repository evidence into a bounded, language-neutral repair DAG. Task names,
//! repository identities, benchmark labels, and reference solutions are not
//! represented in the IR, so they cannot become routing authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA: &str = "B_REPOSITORY_REPAIR_KNOWLEDGE_1";
pub const MAX_ACTIVE_REPAIR_ATOMS: usize = 18;
pub const MAX_COMPETING_HYPOTHESES: usize = 3;
pub const MAX_REVISION_ROUNDS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepositoryLanguage {
    Rust,
    Python,
    #[serde(rename = "TYPESCRIPT")]
    TypeScript,
    #[serde(rename = "JAVASCRIPT")]
    JavaScript,
    Go,
    Unknown,
}

impl RepositoryLanguage {
    pub fn supported(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticFamily {
    CompilationOrType,
    ImportOrModule,
    AssertionContract,
    ExceptionOrPanic,
    TimeoutOrLiveness,
    RaceOrDeadlock,
    ResourceLifecycle,
    ProtocolOrSchema,
    OrderingOrDeterminism,
    ApiCompatibility,
    PerformanceRegression,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    IssueStatement,
    SourceObservation,
    CompilerOrTypeDiagnostic,
    FailingPublicTest,
    PassingRegressionTest,
    PublicContract,
    RuntimeTrace,
    DependencyMetadata,
    ChangeHistory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryObservationIR {
    pub kind: EvidenceKind,
    pub diagnostic_family: DiagnosticFamily,
    /// Content address of the evidence. Raw issue, source, and test text are
    /// intentionally outside the reusable planning substrate.
    pub evidence_sha256: String,
    pub target_symbols: Vec<String>,
    /// True only when the observation came from an executed, repeatable local
    /// command or an equivalent deterministic verifier.
    pub reproducible: bool,
}

impl RepositoryObservationIR {
    fn valid(&self) -> bool {
        self.evidence_sha256.len() == 64
            && self
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && self.target_symbols.len() <= 32
            && self
                .target_symbols
                .iter()
                .all(|symbol| !symbol.trim().is_empty() && symbol.len() <= 256)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryTaskIR {
    pub schema: String,
    pub language: RepositoryLanguage,
    pub observations: Vec<RepositoryObservationIR>,
    pub preserve_public_api: bool,
    pub preserve_data_compatibility: bool,
    pub allow_dependency_changes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairAtom {
    ReproduceFailure,
    ExtractPublicContract,
    LocateOwningSymbol,
    SliceCallAndDataFlow,
    InspectDependencyBoundary,
    FormCompetingHypotheses,
    SelectInformationGainProbe,
    CheckTypeAndOwnershipConstraints,
    CheckResourceLifecycle,
    CheckConcurrencyAndLiveness,
    CheckProtocolAndSerializationBoundary,
    CheckOrderingAndDeterminism,
    CheckApiAndDataCompatibility,
    MeasureBeforeOptimizing,
    ConstructMinimalEdit,
    RunTargetedVerifier,
    RunRegressionMatrix,
    ReviseFromCounterexample,
    PreserveExactRollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairDependency {
    pub prerequisite: RepairAtom,
    pub dependent: RepairAtom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStage {
    Targeted,
    StaticAnalysis,
    FullRegression,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCommandIR {
    pub stage: ValidationStage,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairPlanDisposition {
    Ready,
    CapabilityGap,
    InvalidEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairPlanIR {
    pub schema: String,
    pub disposition: RepairPlanDisposition,
    pub diagnostic_families: Vec<DiagnosticFamily>,
    pub ordered_atoms: Vec<RepairAtom>,
    pub dependencies: Vec<RepairDependency>,
    pub validation_matrix: Vec<ValidationCommandIR>,
    pub missing_evidence: Vec<String>,
    pub forbidden_operations: Vec<String>,
    pub max_competing_hypotheses: usize,
    pub max_revision_rounds: usize,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub reference_solution_imports: u64,
}

/// Classify a public diagnostic into a coarse causal family. The result only
/// selects an inspection atom; it never selects a patch or source replacement.
pub fn classify_public_diagnostic(diagnostic: &str) -> DiagnosticFamily {
    let normalized = diagnostic.to_ascii_lowercase();
    let contains_any = |needles: &[&str]| needles.iter().any(|needle| normalized.contains(needle));
    if contains_any(&["deadlock", "data race", "race detector", "thread sanitizer"]) {
        DiagnosticFamily::RaceOrDeadlock
    } else if contains_any(&["timed out", "timeout", "liveness", "hang detected"]) {
        DiagnosticFamily::TimeoutOrLiveness
    } else if contains_any(&[
        "modulenotfounderror",
        "importerror",
        "cannot find module",
        "unresolved import",
        "no required module provides",
    ]) {
        DiagnosticFamily::ImportOrModule
    } else if contains_any(&[
        "resourcewarning",
        "resource leak",
        "use after close",
        "already closed",
        "file descriptor",
    ]) {
        DiagnosticFamily::ResourceLifecycle
    } else if contains_any(&[
        "schema",
        "serialize",
        "deserialize",
        "invalid json",
        "protocol",
        "wire format",
    ]) {
        DiagnosticFamily::ProtocolOrSchema
    } else if contains_any(&[
        "nondetermin",
        "unstable order",
        "unexpected order",
        "not sorted",
    ]) {
        DiagnosticFamily::OrderingOrDeterminism
    } else if contains_any(&[
        "unexpected keyword",
        "missing required positional",
        "signature mismatch",
        "breaking change",
        "attributeerror",
    ]) {
        DiagnosticFamily::ApiCompatibility
    } else if contains_any(&[
        "regression threshold",
        "performance regression",
        "benchmark slower",
        "allocation regression",
    ]) {
        DiagnosticFamily::PerformanceRegression
    } else if contains_any(&[
        "typeerror",
        "type mismatch",
        "mismatched types",
        "cannot borrow",
        "cannot assign",
        "compile error",
    ]) {
        DiagnosticFamily::CompilationOrType
    } else if contains_any(&[
        "assertionerror",
        "assertion failed",
        "expected:",
        "expected ",
    ]) {
        DiagnosticFamily::AssertionContract
    } else if contains_any(&["traceback", "exception", "panic", "fatal error"]) {
        DiagnosticFamily::ExceptionOrPanic
    } else {
        DiagnosticFamily::Unknown
    }
}

fn validation_matrix(language: RepositoryLanguage) -> Vec<ValidationCommandIR> {
    let command = |stage, program: &str, args: &[&str]| ValidationCommandIR {
        stage,
        program: program.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
    };
    match language {
        RepositoryLanguage::Rust => vec![
            command(ValidationStage::Targeted, "cargo", &["test", "--lib"]),
            command(
                ValidationStage::StaticAnalysis,
                "cargo",
                &["clippy", "--all-targets", "--", "-D", "warnings"],
            ),
            command(ValidationStage::FullRegression, "cargo", &["test"]),
        ],
        RepositoryLanguage::Python => vec![
            command(
                ValidationStage::Targeted,
                "python",
                &["-m", "pytest", "-q", "--maxfail=1"],
            ),
            command(
                ValidationStage::StaticAnalysis,
                "python",
                &["-m", "compileall", "-q", "."],
            ),
            command(
                ValidationStage::FullRegression,
                "python",
                &["-m", "pytest", "-q"],
            ),
        ],
        RepositoryLanguage::TypeScript => vec![
            command(
                ValidationStage::Targeted,
                "npm",
                &["test", "--", "--runInBand"],
            ),
            command(ValidationStage::StaticAnalysis, "npx", &["tsc", "--noEmit"]),
            command(ValidationStage::FullRegression, "npm", &["test"]),
        ],
        RepositoryLanguage::JavaScript => vec![
            command(
                ValidationStage::Targeted,
                "npm",
                &["test", "--", "--runInBand"],
            ),
            command(
                ValidationStage::StaticAnalysis,
                "npm",
                &["run", "lint", "--if-present"],
            ),
            command(ValidationStage::FullRegression, "npm", &["test"]),
        ],
        RepositoryLanguage::Go => vec![
            command(ValidationStage::Targeted, "go", &["test", "./..."]),
            command(ValidationStage::StaticAnalysis, "go", &["vet", "./..."]),
            command(
                ValidationStage::FullRegression,
                "go",
                &["test", "-race", "./..."],
            ),
        ],
        RepositoryLanguage::Unknown => Vec::new(),
    }
}

fn diagnostic_atom(family: DiagnosticFamily) -> Option<RepairAtom> {
    match family {
        DiagnosticFamily::CompilationOrType => Some(RepairAtom::CheckTypeAndOwnershipConstraints),
        DiagnosticFamily::ImportOrModule => Some(RepairAtom::InspectDependencyBoundary),
        DiagnosticFamily::AssertionContract | DiagnosticFamily::ExceptionOrPanic => None,
        DiagnosticFamily::TimeoutOrLiveness | DiagnosticFamily::RaceOrDeadlock => {
            Some(RepairAtom::CheckConcurrencyAndLiveness)
        }
        DiagnosticFamily::ResourceLifecycle => Some(RepairAtom::CheckResourceLifecycle),
        DiagnosticFamily::ProtocolOrSchema => {
            Some(RepairAtom::CheckProtocolAndSerializationBoundary)
        }
        DiagnosticFamily::OrderingOrDeterminism => Some(RepairAtom::CheckOrderingAndDeterminism),
        DiagnosticFamily::ApiCompatibility => Some(RepairAtom::CheckApiAndDataCompatibility),
        DiagnosticFamily::PerformanceRegression => Some(RepairAtom::MeasureBeforeOptimizing),
        DiagnosticFamily::Unknown => None,
    }
}

fn push_unique(atoms: &mut Vec<RepairAtom>, atom: RepairAtom) {
    if !atoms.contains(&atom) {
        atoms.push(atom);
    }
}

/// Compile public observations into a bounded repair plan.
///
/// A failing executable observation and a public behavioral contract are both
/// mandatory. This prevents issue prose or diagnostic strings from becoming a
/// direct patch oracle.
pub fn plan_repository_repair(task: &RepositoryTaskIR) -> RepairPlanIR {
    let mut missing_evidence = Vec::new();
    let valid_schema = task.schema == REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA;
    let valid_observations = !task.observations.is_empty()
        && task.observations.len() <= 128
        && task.observations.iter().all(RepositoryObservationIR::valid);
    if !valid_schema {
        missing_evidence.push("CANONICAL_SCHEMA".to_string());
    }
    if !valid_observations {
        missing_evidence.push("VALID_CONTENT_ADDRESSED_OBSERVATIONS".to_string());
    }
    if !task.language.supported() {
        missing_evidence.push("SUPPORTED_LANGUAGE_BACKEND".to_string());
    }

    let has_reproduction = task.observations.iter().any(|observation| {
        observation.reproducible
            && matches!(
                observation.kind,
                EvidenceKind::CompilerOrTypeDiagnostic
                    | EvidenceKind::FailingPublicTest
                    | EvidenceKind::RuntimeTrace
            )
    });
    let has_contract = task.observations.iter().any(|observation| {
        matches!(
            observation.kind,
            EvidenceKind::FailingPublicTest
                | EvidenceKind::PassingRegressionTest
                | EvidenceKind::PublicContract
        )
    });
    let has_source = task
        .observations
        .iter()
        .any(|observation| observation.kind == EvidenceKind::SourceObservation);
    if !has_reproduction {
        missing_evidence.push("REPRODUCIBLE_FAILURE".to_string());
    }
    if !has_contract {
        missing_evidence.push("PUBLIC_BEHAVIORAL_CONTRACT".to_string());
    }
    if !has_source {
        missing_evidence.push("SOURCE_OBSERVATION".to_string());
    }

    let diagnostic_families = task
        .observations
        .iter()
        .map(|observation| observation.diagnostic_family)
        .filter(|family| *family != DiagnosticFamily::Unknown)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if diagnostic_families.is_empty() {
        missing_evidence.push("TYPED_DIAGNOSTIC_FAMILY".to_string());
    }

    let disposition = if !valid_schema || !valid_observations {
        RepairPlanDisposition::InvalidEvidence
    } else if !missing_evidence.is_empty() {
        RepairPlanDisposition::CapabilityGap
    } else {
        RepairPlanDisposition::Ready
    };

    let mut ordered_atoms = Vec::new();
    if disposition == RepairPlanDisposition::Ready {
        ordered_atoms.extend([
            RepairAtom::ReproduceFailure,
            RepairAtom::ExtractPublicContract,
            RepairAtom::LocateOwningSymbol,
            RepairAtom::SliceCallAndDataFlow,
            RepairAtom::FormCompetingHypotheses,
        ]);
        for family in &diagnostic_families {
            if let Some(atom) = diagnostic_atom(*family) {
                push_unique(&mut ordered_atoms, atom);
            }
        }
        if task.preserve_public_api || task.preserve_data_compatibility {
            push_unique(&mut ordered_atoms, RepairAtom::CheckApiAndDataCompatibility);
        }
        push_unique(&mut ordered_atoms, RepairAtom::SelectInformationGainProbe);
        push_unique(&mut ordered_atoms, RepairAtom::ConstructMinimalEdit);
        push_unique(&mut ordered_atoms, RepairAtom::RunTargetedVerifier);
        push_unique(&mut ordered_atoms, RepairAtom::RunRegressionMatrix);
        push_unique(&mut ordered_atoms, RepairAtom::ReviseFromCounterexample);
        push_unique(&mut ordered_atoms, RepairAtom::PreserveExactRollback);
        ordered_atoms.truncate(MAX_ACTIVE_REPAIR_ATOMS);
    }
    let dependencies = ordered_atoms
        .windows(2)
        .map(|pair| RepairDependency {
            prerequisite: pair[0],
            dependent: pair[1],
        })
        .collect();

    RepairPlanIR {
        schema: REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA.to_string(),
        disposition,
        diagnostic_families,
        ordered_atoms,
        dependencies,
        validation_matrix: if disposition == RepairPlanDisposition::Ready {
            validation_matrix(task.language)
        } else {
            Vec::new()
        },
        missing_evidence,
        forbidden_operations: [
            "ISSUE_TEXT_TO_PATCH_SHORTCUT",
            "TASK_OR_REPOSITORY_ID_ROUTING",
            "REFERENCE_SOLUTION_IMPORT",
            "UNVERIFIED_SOURCE_INSTALL",
        ]
        .into_iter()
        .chain((!task.allow_dependency_changes).then_some("DEPENDENCY_MANIFEST_MUTATION"))
        .map(str::to_string)
        .collect(),
        max_competing_hypotheses: MAX_COMPETING_HYPOTHESES,
        max_revision_rounds: MAX_REVISION_ROUNDS,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        reference_solution_imports: 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryKnowledgeCanaryReceipt {
    pub schema: String,
    pub pass: bool,
    pub fresh_synthetic_tasks: usize,
    pub ready_plans: usize,
    pub required_atom_checks: usize,
    pub required_atom_passes: usize,
    pub supported_languages: Vec<RepositoryLanguage>,
    pub exercised_diagnostic_families: Vec<DiagnosticFamily>,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub reference_solution_imports: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub official_benchmark_score_claimed: bool,
}

/// Run a deterministic cross-product canary over fresh synthetic task IRs.
/// These are planner tests, not SWE-bench or DeepSWE score claims.
pub fn run_repository_knowledge_canary() -> RepositoryKnowledgeCanaryReceipt {
    let languages = vec![
        RepositoryLanguage::Rust,
        RepositoryLanguage::Python,
        RepositoryLanguage::TypeScript,
        RepositoryLanguage::JavaScript,
        RepositoryLanguage::Go,
    ];
    let families = vec![
        DiagnosticFamily::CompilationOrType,
        DiagnosticFamily::ImportOrModule,
        DiagnosticFamily::TimeoutOrLiveness,
        DiagnosticFamily::ResourceLifecycle,
        DiagnosticFamily::ProtocolOrSchema,
        DiagnosticFamily::OrderingOrDeterminism,
        DiagnosticFamily::ApiCompatibility,
        DiagnosticFamily::PerformanceRegression,
    ];
    let mut ready_plans = 0;
    let mut required_atom_checks = 0;
    let mut required_atom_passes = 0;
    for (language_index, language) in languages.iter().enumerate() {
        for (family_index, family) in families.iter().enumerate() {
            let seed = language_index * families.len() + family_index + 1;
            let hash = format!("{seed:064x}");
            let task = RepositoryTaskIR {
                schema: REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA.to_string(),
                language: *language,
                observations: vec![
                    RepositoryObservationIR {
                        kind: EvidenceKind::FailingPublicTest,
                        diagnostic_family: *family,
                        evidence_sha256: hash.clone(),
                        target_symbols: vec!["public_owner".to_string()],
                        reproducible: true,
                    },
                    RepositoryObservationIR {
                        kind: EvidenceKind::SourceObservation,
                        diagnostic_family: *family,
                        evidence_sha256: format!("{:064x}", seed + 4_096),
                        target_symbols: vec!["public_owner".to_string()],
                        reproducible: false,
                    },
                ],
                preserve_public_api: true,
                preserve_data_compatibility: true,
                allow_dependency_changes: false,
            };
            let plan = plan_repository_repair(&task);
            ready_plans += usize::from(plan.disposition == RepairPlanDisposition::Ready);
            required_atom_checks += 1;
            let family_atom_present =
                diagnostic_atom(*family).is_none_or(|atom| plan.ordered_atoms.contains(&atom));
            if family_atom_present
                && plan
                    .ordered_atoms
                    .contains(&RepairAtom::ConstructMinimalEdit)
                && plan
                    .ordered_atoms
                    .contains(&RepairAtom::RunRegressionMatrix)
                && plan.validation_matrix.len() == 3
            {
                required_atom_passes += 1;
            }
        }
    }
    let fresh_synthetic_tasks = languages.len() * families.len();
    RepositoryKnowledgeCanaryReceipt {
        schema: REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA.to_string(),
        pass: ready_plans == fresh_synthetic_tasks && required_atom_passes == required_atom_checks,
        fresh_synthetic_tasks,
        ready_plans,
        required_atom_checks,
        required_atom_passes,
        supported_languages: languages,
        exercised_diagnostic_families: families,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        reference_solution_imports: 0,
        external_llm_calls: 0,
        network_reads: 0,
        official_benchmark_score_claimed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        kind: EvidenceKind,
        family: DiagnosticFamily,
        seed: u64,
    ) -> RepositoryObservationIR {
        RepositoryObservationIR {
            kind,
            diagnostic_family: family,
            evidence_sha256: format!("{seed:064x}"),
            target_symbols: vec!["owner".to_string()],
            reproducible: matches!(
                kind,
                EvidenceKind::FailingPublicTest
                    | EvidenceKind::CompilerOrTypeDiagnostic
                    | EvidenceKind::RuntimeTrace
            ),
        }
    }

    fn task(language: RepositoryLanguage, family: DiagnosticFamily) -> RepositoryTaskIR {
        RepositoryTaskIR {
            schema: REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA.to_string(),
            language,
            observations: vec![
                observation(EvidenceKind::FailingPublicTest, family, 1),
                observation(EvidenceKind::SourceObservation, family, 2),
            ],
            preserve_public_api: true,
            preserve_data_compatibility: true,
            allow_dependency_changes: false,
        }
    }

    #[test]
    fn cross_language_canary_is_complete_and_does_not_claim_benchmark_score() {
        let receipt = run_repository_knowledge_canary();
        assert!(receipt.pass);
        assert_eq!(receipt.fresh_synthetic_tasks, 40);
        assert_eq!(receipt.supported_languages.len(), 5);
        assert_eq!(receipt.reference_solution_imports, 0);
        assert!(!receipt.official_benchmark_score_claimed);
    }

    #[test]
    fn issue_prose_without_reproduction_or_source_fails_closed() {
        let mut input = task(
            RepositoryLanguage::Python,
            DiagnosticFamily::AssertionContract,
        );
        input.observations = vec![observation(
            EvidenceKind::IssueStatement,
            DiagnosticFamily::AssertionContract,
            9,
        )];
        let plan = plan_repository_repair(&input);
        assert_eq!(plan.disposition, RepairPlanDisposition::CapabilityGap);
        assert!(plan.ordered_atoms.is_empty());
        assert!(plan
            .missing_evidence
            .contains(&"REPRODUCIBLE_FAILURE".to_string()));
        assert!(plan
            .missing_evidence
            .contains(&"PUBLIC_BEHAVIORAL_CONTRACT".to_string()));
        assert!(plan
            .missing_evidence
            .contains(&"SOURCE_OBSERVATION".to_string()));
    }

    #[test]
    fn invalid_content_address_is_rejected() {
        let mut input = task(
            RepositoryLanguage::Rust,
            DiagnosticFamily::CompilationOrType,
        );
        input.observations[0].evidence_sha256 = "not-a-hash".to_string();
        let plan = plan_repository_repair(&input);
        assert_eq!(plan.disposition, RepairPlanDisposition::InvalidEvidence);
    }

    #[test]
    fn diagnostic_text_selects_inspection_family_not_patch_content() {
        assert_eq!(
            classify_public_diagnostic("ModuleNotFoundError: no module named codec"),
            DiagnosticFamily::ImportOrModule
        );
        assert_eq!(
            classify_public_diagnostic("thread sanitizer: data race"),
            DiagnosticFamily::RaceOrDeadlock
        );
        assert_eq!(
            classify_public_diagnostic("unrecognized diagnostic"),
            DiagnosticFamily::Unknown
        );
    }

    #[test]
    fn diagnostic_families_activate_composable_atomic_knowledge() {
        let cases = [
            (
                DiagnosticFamily::CompilationOrType,
                RepairAtom::CheckTypeAndOwnershipConstraints,
            ),
            (
                DiagnosticFamily::ImportOrModule,
                RepairAtom::InspectDependencyBoundary,
            ),
            (
                DiagnosticFamily::TimeoutOrLiveness,
                RepairAtom::CheckConcurrencyAndLiveness,
            ),
            (
                DiagnosticFamily::ResourceLifecycle,
                RepairAtom::CheckResourceLifecycle,
            ),
            (
                DiagnosticFamily::ProtocolOrSchema,
                RepairAtom::CheckProtocolAndSerializationBoundary,
            ),
            (
                DiagnosticFamily::OrderingOrDeterminism,
                RepairAtom::CheckOrderingAndDeterminism,
            ),
            (
                DiagnosticFamily::ApiCompatibility,
                RepairAtom::CheckApiAndDataCompatibility,
            ),
            (
                DiagnosticFamily::PerformanceRegression,
                RepairAtom::MeasureBeforeOptimizing,
            ),
        ];
        for (family, required_atom) in cases {
            let plan = plan_repository_repair(&task(RepositoryLanguage::Go, family));
            assert_eq!(plan.disposition, RepairPlanDisposition::Ready);
            assert!(plan.ordered_atoms.contains(&required_atom));
            assert!(plan.ordered_atoms.len() <= MAX_ACTIVE_REPAIR_ATOMS);
            assert_eq!(plan.dependencies.len() + 1, plan.ordered_atoms.len());
        }
    }

    #[test]
    fn validation_matrix_is_language_specific_but_reasoning_atoms_are_shared() {
        let rust = plan_repository_repair(&task(
            RepositoryLanguage::Rust,
            DiagnosticFamily::OrderingOrDeterminism,
        ));
        let go = plan_repository_repair(&task(
            RepositoryLanguage::Go,
            DiagnosticFamily::OrderingOrDeterminism,
        ));
        assert_eq!(rust.ordered_atoms, go.ordered_atoms);
        assert_ne!(rust.validation_matrix, go.validation_matrix);
        assert_eq!(rust.validation_matrix[0].program, "cargo");
        assert_eq!(go.validation_matrix[0].program, "go");
    }

    #[test]
    fn unknown_language_and_diagnostic_are_capability_gaps() {
        let plan = plan_repository_repair(&task(
            RepositoryLanguage::Unknown,
            DiagnosticFamily::Unknown,
        ));
        assert_eq!(plan.disposition, RepairPlanDisposition::CapabilityGap);
        assert!(plan
            .missing_evidence
            .contains(&"SUPPORTED_LANGUAGE_BACKEND".to_string()));
        assert!(plan
            .missing_evidence
            .contains(&"TYPED_DIAGNOSTIC_FAMILY".to_string()));
    }
}
