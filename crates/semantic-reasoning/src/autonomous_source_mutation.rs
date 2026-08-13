//! Autonomous installation of core-generated source improvements.
//!
//! This module grants the core write authority over one explicitly configured
//! source root. A candidate is bound to the exact predecessor bytes, installed
//! atomically, compiled and regression-tested locally, and rolled back on any
//! failure. Successful builds are staged for the persistent launcher to swap
//! after the running supervisor exits.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use crate::compiler_guided_repair::{discover_compiler_guided_repairs, CompilerGuidedRepairPolicy};
use crate::generalized_self_application::{
    derive_dynamic_weakness, feedback_priority, synthesize_generalized_change,
    validate_change_binding, validation_counterexample, GeneralizedChangeIR,
    ValidationCounterexampleIR, ValidationPhase, WeaknessEvidenceKind,
};
use crate::grammar_repair_synthesis::discover_grammar_repairs_for_generation_with_priors;
use crate::self_repair_contract::sha256;
use crate::sem5::typed_mechanism::{
    load_authorized_typed_mechanism_operators, typed_mechanism_improvement_operator_from_receipt,
    typed_mechanism_operator_authority_directory, typed_mechanism_operator_directory,
    validate_typed_mechanism_improvement_operator, validate_typed_mechanism_operator_authority,
    validate_typed_mechanism_synthesis_receipt, TypedMechanismImprovementOperatorIR,
    TypedMechanismOperatorAuthorityReceiptIR, TypedMechanismSynthesisReceiptIR,
    INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA, MAX_ACTIVE_TYPED_MECHANISM_OPERATORS,
};
use crate::structural_source_repair::{
    execute_structural_repair, synthesize_structural_repair, SourceEditAtom,
    StructuralRepairProgram, VerificationObligation,
};

pub const AUTONOMOUS_SOURCE_MUTATION_SCHEMA: &str = "B_CORE_AUTONOMOUS_SOURCE_MUTATION_1";
pub const SELF_UPDATE_HANDOFF_FILE: &str = "SELF_UPDATE_READY.json";
pub const SOURCE_REPAIR_LEARNING_SCHEMA: &str = "B_CORE_SOURCE_REPAIR_LEARNING_1";
pub const IMPROVEMENT_OPERATOR_MEMORY_SCHEMA: &str = "B_CORE_IMPROVEMENT_OPERATOR_MEMORY_1";
pub const MAX_IMPROVEMENT_OPERATOR_GRAPH_NODES: usize = 8;
const MAX_COMPETING_SOURCE_PROPOSALS: usize = 3;
const MAX_TYPED_OPERATOR_RECONCILIATION_RECEIPTS: usize = 64;
// Revision 51 publishes only operators that own a machine-executable source
// synthesis payload; typed-program execution profiles remain useful ranking
// evidence but cannot masquerade as callable repair knowledge.
// Revision 50 makes repository repair attempts capability-addressed and
// carries verifier-falsified candidate hashes into successor synthesis.
// Revision 49 removes diagnostic opportunity-family identity from atomic
// composition authority; exact source/edit compatibility remains authoritative.
// Revision 48 bounds consumed runtime staging generations while preserving a
// pending handoff and its immediate verified predecessor.
// Revision 47 separates exact authority existence from the bounded active
// operator window and deduplicates repeated authority receipts.
// Generator identity remains diagnostic evidence only.
pub const SOURCE_REPAIR_ENGINE_REVISION: u64 = 51;
pub const MAX_RETAINED_CONSUMED_RUNTIME_STAGING_GENERATIONS: usize = 2;
const KNOWN_REMAINDER_PREDICTED_VALUE: u16 = 35;
const MAX_REPOSITORY_REPAIR_FAMILY_FILES: usize = 16;
const KNOWN_REMAINDER_STRATEGIES: [&str; 4] = [
    "TYPED_IS_MULTIPLE_OF",
    "PARENTHESIZED_IS_MULTIPLE_OF",
    "CHECKED_REMAINDER_MATCH",
    "EUCLIDEAN_REMAINDER_COMPARISON",
];

fn default_source_repair_attempts() -> u8 {
    4
}

fn is_default_source_repair_attempts(value: &u8) -> bool {
    *value == default_source_repair_attempts()
}

fn default_minimum_predicted_value() -> u16 {
    60
}

fn is_default_minimum_predicted_value(value: &u16) -> bool {
    *value == default_minimum_predicted_value()
}

fn default_compiler_repair_discovery() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourceMutationPolicy {
    pub enabled: bool,
    pub source_root: PathBuf,
    pub cargo_executable: PathBuf,
    pub build_target_dir: PathBuf,
    pub runtime_bin_dir: PathBuf,
    pub auto_discover_known_transformations: bool,
    #[serde(
        default = "default_compiler_repair_discovery",
        skip_serializing_if = "is_true"
    )]
    pub auto_discover_compiler_repairs: bool,
    #[serde(
        default = "default_compiler_repair_discovery",
        skip_serializing_if = "is_true"
    )]
    pub auto_synthesize_grammar_repairs: bool,
    pub max_candidate_bytes: u64,
    pub max_installations: u64,
    pub validation_timeout_ms: u64,
    #[serde(
        default = "default_source_repair_attempts",
        skip_serializing_if = "is_default_source_repair_attempts"
    )]
    pub max_attempts_per_problem: u8,
    #[serde(
        default = "default_minimum_predicted_value",
        skip_serializing_if = "is_default_minimum_predicted_value"
    )]
    pub minimum_predicted_value: u16,
}

impl Default for AutonomousSourceMutationPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            source_root: PathBuf::new(),
            cargo_executable: PathBuf::new(),
            build_target_dir: PathBuf::new(),
            runtime_bin_dir: PathBuf::new(),
            auto_discover_known_transformations: false,
            auto_discover_compiler_repairs: default_compiler_repair_discovery(),
            auto_synthesize_grammar_repairs: default_compiler_repair_discovery(),
            max_candidate_bytes: 2 * 1024 * 1024,
            max_installations: 64,
            validation_timeout_ms: 15 * 60 * 1_000,
            max_attempts_per_problem: default_source_repair_attempts(),
            minimum_predicted_value: default_minimum_predicted_value(),
        }
    }
}

impl AutonomousSourceMutationPolicy {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePatchFamilyMember {
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub structural_repair_program: StructuralRepairProgram,
    pub public_examples_observed: usize,
    pub public_examples_evaluated: usize,
    pub public_examples_satisfied: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeOpportunityKind {
    Defect,
    CapabilityGap,
    EfficiencyOpportunity,
    RobustnessOpportunity,
    #[default]
    ResearchHypothesis,
}

pub fn source_opportunity_family_id(
    kind: ChangeOpportunityKind,
    stable_family_basis: &str,
) -> String {
    sha256(
        format!(
            "B_CORE_CHANGE_OPPORTUNITY_FAMILY_1:{kind:?}:{}",
            stable_family_basis.trim().to_ascii_uppercase()
        )
        .as_bytes(),
    )
}

fn opportunity_binding_valid(request: &AutonomousSourcePatchRequest) -> bool {
    request.opportunity_family_id.len() == 64
        && request
            .opportunity_family_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}

fn validate_typed_mechanism_recipe_binding(
    request: &AutonomousSourcePatchRequest,
) -> Result<(), String> {
    match (
        &request.typed_mechanism_operator_recipe,
        &request.typed_mechanism_synthesis_receipt,
        &request.typed_mechanism_materialized_syntax_sha256,
        &request.typed_mechanism_materialized_syntax_source,
        &request.typed_mechanism_materialized_edit,
    ) {
        (None, None, None, None, None) => {
            if request.typed_mechanism_selected_operator_id.is_some()
                || request.typed_mechanism_candidates_enumerated != 0
                || request.typed_mechanism_preferred_operator_attempts != 0
            {
                return Err("TYPED_MECHANISM_RECIPE_ORPHAN_ACCOUNTING".to_string());
            }
            Ok(())
        }
        (
            Some(recipe),
            Some(synthesis),
            Some(syntax_sha256),
            Some(syntax_source),
            Some(materialized_edit),
        ) => {
            validate_typed_mechanism_synthesis_receipt(synthesis)?;
            validate_typed_mechanism_improvement_operator(recipe)?;
            let expected_recipe = typed_mechanism_improvement_operator_from_receipt(
                synthesis,
                synthesis.receipt_sha256.clone(),
            )?;
            let synthesis_receipt_sha256 = request
                .solution_strategy
                .split(':')
                .nth(2)
                .filter(|hash| hash.len() == 64)
                .ok_or_else(|| "TYPED_MECHANISM_RECIPE_STRATEGY_BINDING".to_string())?;
            let exact_replace = matches!(
                materialized_edit,
                SourceEditAtom::Replace { replacement, .. } if replacement == syntax_source
            );
            if expected_recipe != *recipe
                || synthesis.receipt_sha256 != synthesis_receipt_sha256
                || synthesis.template.complete_expression_source != *syntax_source
                || request.typed_mechanism_selected_operator_id != synthesis.selected_operator_id
                || request.typed_mechanism_candidates_enumerated != synthesis.candidates_enumerated
                || request.typed_mechanism_preferred_operator_attempts
                    != synthesis.preferred_operator_attempts
                || syntax_sha256.len() != 64
                || sha256(syntax_source.as_bytes()) != *syntax_sha256
                || !exact_replace
                || request.structural_repair_program.is_none()
            {
                return Err("TYPED_MECHANISM_RECIPE_BINDING_MISMATCH".to_string());
            }
            Ok(())
        }
        _ => Err("TYPED_MECHANISM_RECIPE_PARTIAL_BINDING".to_string()),
    }
}

fn validate_typed_mechanism_source_materialization(
    request: &AutonomousSourcePatchRequest,
    predecessor_source: &str,
) -> Result<(), String> {
    let Some(edit) = &request.typed_mechanism_materialized_edit else {
        return Ok(());
    };
    let materialized = crate::structural_source_repair::apply_edit_atom(predecessor_source, edit)
        .map_err(|error| format!("TYPED_MECHANISM_SOURCE_EDIT_REPLAY:{error}"))?;
    if materialized != request.candidate_source
        || sha256(materialized.as_bytes()) != request.candidate_sha256
    {
        return Err("TYPED_MECHANISM_SOURCE_MATERIALIZATION_MISMATCH".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourcePatchRequest {
    pub schema: String,
    pub patch_id: String,
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub transformation: String,
    pub consequence_predictions: Vec<String>,
    pub predicted_value: u16,
    pub source_generation: u64,
    pub core_generated: bool,
    pub core_self_approved: bool,
    #[serde(default)]
    pub solution_strategy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structural_repair_program: Option<StructuralRepairProgram>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change: Option<GeneralizedChangeIR>,
    #[serde(default)]
    pub additional_family_members: Vec<SourcePatchFamilyMember>,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub improvement_operator_invocation: Option<ImprovementOperatorInvocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub improvement_operator_execution: Option<ImprovementOperatorExecution>,
    /// Canonical typed expression recipe. This has no reuse authority until
    /// the installed patch passes format, Clippy, public tests, release build,
    /// exact-postimage, and workspace-stability gates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_operator_recipe: Option<TypedMechanismImprovementOperatorIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_synthesis_receipt: Option<TypedMechanismSynthesisReceiptIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_materialized_syntax_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_materialized_syntax_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_materialized_edit: Option<SourceEditAtom>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_selected_operator_id: Option<String>,
    #[serde(default)]
    pub typed_mechanism_candidates_enumerated: usize,
    #[serde(default)]
    pub typed_mechanism_preferred_operator_attempts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceDiscoveryDisposition {
    Candidate,
    Disabled,
    BelowValueThreshold,
    NoApplicableTransformation,
}

impl SourceDiscoveryDisposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::Candidate => "CANDIDATE",
            Self::Disabled => "DISABLED",
            Self::BelowValueThreshold => "BELOW_VALUE_THRESHOLD",
            Self::NoApplicableTransformation => "NO_APPLICABLE_TRANSFORMATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDiscoveryResult {
    pub disposition: SourceDiscoveryDisposition,
    pub candidate: Option<AutonomousSourcePatchRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepairAttempt {
    pub attempt_number: u8,
    pub source_generation: u64,
    /// Zero denotes an attempt written before per-attempt engine identity was
    /// part of the learning contract.
    #[serde(default)]
    pub source_engine_revision: u64,
    pub solution_strategy: String,
    pub candidate_sha256: String,
    pub succeeded: bool,
    pub receipt_sha256: String,
    pub diagnostic_sha256: String,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub structural_repair_program_sha256: Option<String>,
    #[serde(default)]
    pub edit_atom_kinds: Vec<String>,
    #[serde(default)]
    pub structural_postcondition_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_counterexample: Option<ValidationCounterexampleIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change_sha256: Option<String>,
    #[serde(default)]
    pub derived_from_counterexample_ids: Vec<String>,
    #[serde(default = "one_family_member")]
    pub family_member_count: usize,
    #[serde(default)]
    pub family_structural_repair_program_sha256: Vec<String>,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weakness_evidence_kind: Option<WeaknessEvidenceKind>,
    #[serde(default)]
    pub validation_duration_ms: u64,
    #[serde(default)]
    pub invoked_operator_ids: Vec<String>,
    #[serde(default)]
    pub executed_operator_id: Option<String>,
    #[serde(default)]
    pub improvement_operator_execution_sha256: Option<String>,
    #[serde(default)]
    pub operator_priority_adjustment: i32,
    #[serde(default)]
    pub operator_cross_family_successes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedSuccessfulRepair {
    pub learned_at_generation: u64,
    pub solution_strategy: String,
    #[serde(default)]
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub validation_output_sha256: String,
    pub release_build_output_sha256: String,
    pub attempts_required: u8,
    #[serde(default)]
    pub structural_repair_program_sha256: Option<String>,
    #[serde(default)]
    pub edit_atom_kinds: Vec<String>,
    #[serde(default)]
    pub structural_postcondition_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change_sha256: Option<String>,
    #[serde(default)]
    pub derived_from_counterexample_ids: Vec<String>,
    #[serde(default = "one_family_member")]
    pub family_member_count: usize,
    #[serde(default)]
    pub family_structural_repair_program_sha256: Vec<String>,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weakness_evidence_kind: Option<WeaknessEvidenceKind>,
    #[serde(default)]
    pub validation_duration_ms: u64,
}

fn one_family_member() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepairLearningRecord {
    pub schema: String,
    pub problem_id: String,
    pub relative_path: PathBuf,
    pub transformation: String,
    pub status: String,
    pub cycle_started_generation: u64,
    #[serde(default)]
    pub cycle_started_engine_revision: u64,
    /// Index of the first physical receipt belonging to the current causal
    /// candidate cycle. Older receipts remain available for operator learning
    /// without being misreported as retries of a newly synthesized artifact.
    #[serde(default)]
    pub cycle_attempt_start_index: usize,
    pub eligible_after_generation: Option<u64>,
    pub attempts: Vec<SourceRepairAttempt>,
    pub learned_success: Option<LearnedSuccessfulRepair>,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorIR {
    pub schema: String,
    pub operator_id: String,
    pub weakness_evidence_kind: WeaknessEvidenceKind,
    pub generator_kind: ImprovementOperatorGeneratorKind,
    /// Machine-consumable synthesis payload owned by this operator.  A
    /// profile without this field may rank or validate a freshly synthesized
    /// StructuralRepairProgram, but it cannot create a patch from source and
    /// therefore must not be published as executable knowledge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_payload: Option<ExecutableImprovementOperatorPayloadIR>,
    pub solution_strategy_family: String,
    pub edit_atom_kinds: Vec<String>,
    pub structural_postcondition_class: String,
    pub validation_contract: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImprovementOperatorGeneratorKind {
    KnownStructuralRewrite,
    CompilerSuggestedEdit,
    TypedGrammarComposition,
    ProgramIrLowering,
    LearnedSelfHealingLowering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnownStructuralRewriteIR {
    TypedIsMultipleOf,
    ParenthesizedIsMultipleOf,
    CheckedRemainderMatch,
    EuclideanRemainderComparison,
}

impl KnownStructuralRewriteIR {
    fn from_strategy(strategy: &str) -> Option<Self> {
        match strategy {
            "TYPED_IS_MULTIPLE_OF" => Some(Self::TypedIsMultipleOf),
            "PARENTHESIZED_IS_MULTIPLE_OF" => Some(Self::ParenthesizedIsMultipleOf),
            "CHECKED_REMAINDER_MATCH" => Some(Self::CheckedRemainderMatch),
            "EUCLIDEAN_REMAINDER_COMPARISON" => Some(Self::EuclideanRemainderComparison),
            _ => None,
        }
    }

    fn strategy_index(self) -> usize {
        match self {
            Self::TypedIsMultipleOf => 0,
            Self::ParenthesizedIsMultipleOf => 1,
            Self::CheckedRemainderMatch => 2,
            Self::EuclideanRemainderComparison => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "payload_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutableImprovementOperatorPayloadIR {
    KnownStructuralRewrite { rewrite: KnownStructuralRewriteIR },
}

impl ImprovementOperatorIR {
    pub fn can_synthesize_from_source(&self) -> bool {
        self.executable_payload.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorProfile {
    pub operator: ImprovementOperatorIR,
    pub attempts: u64,
    pub successful_uses: u64,
    pub rollbacks: u64,
    pub repository_guided_attempts: u64,
    pub repository_guided_successful_uses: u64,
    pub cumulative_validation_duration_ms: u64,
    pub attempted_opportunity_kinds: BTreeSet<ChangeOpportunityKind>,
    pub attempted_family_ids: BTreeSet<String>,
    pub successful_family_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorMemory {
    pub schema: String,
    pub profiles: Vec<ImprovementOperatorProfile>,
    pub total_attempts: u64,
    pub total_successful_uses: u64,
    pub repository_guided_attempts: u64,
    pub repository_guided_successful_uses: u64,
    pub productive_cross_family_transfers: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorInvocation {
    pub schema: String,
    pub matched_operator_ids: Vec<String>,
    pub priority_adjustment: i32,
    pub prior_attempts: u64,
    pub prior_successful_uses: u64,
    pub cross_family_successes: usize,
    pub executable_generator_kind: Option<ImprovementOperatorGeneratorKind>,
    pub applicability_satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorExecution {
    pub schema: String,
    pub operator_id: String,
    pub generator_kind: ImprovementOperatorGeneratorKind,
    pub applicable: bool,
    pub candidate_source: Option<String>,
    pub execution_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorBehavioralCanaryReceipt {
    pub schema: String,
    pub context_sha256: String,
    pub operator: ImprovementOperatorIR,
    pub structural_repair_program_sha256: String,
    pub candidate_sha256: String,
    pub cases_executed: usize,
    pub cases_passed: usize,
    pub exact_candidate_observed: bool,
    pub wrong_predecessor_rejected: bool,
    pub tampered_target_rejected: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorGraphIR {
    pub schema: String,
    pub graph_id: String,
    pub operator_ids: Vec<String>,
    pub transported_type: String,
    pub join_postconditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorGraphNodeProgram {
    pub operator: ImprovementOperatorIR,
    pub predecessor_source: String,
    pub structural_repair_program: StructuralRepairProgram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementOperatorGraphCanaryReceipt {
    pub schema: String,
    pub context_sha256: String,
    pub graph: ImprovementOperatorGraphIR,
    pub node_receipt_sha256s: Vec<String>,
    pub cases_executed: usize,
    pub cases_passed: usize,
    pub parallel_nodes_executed: bool,
    pub exact_postimages_observed: bool,
    pub negative_controls_rejected: bool,
    pub canonical_join_observed: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalCommandReceipt {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub cargo_incremental: bool,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    #[serde(default)]
    pub output_sha256: String,
    #[serde(default)]
    pub diagnostic_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeUpdateHandoff {
    pub schema: String,
    pub patch_id: String,
    pub staged_supervisor: PathBuf,
    pub staged_verifier: PathBuf,
    pub runtime_supervisor: PathBuf,
    pub runtime_verifier: PathBuf,
    pub source_receipt: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourcePatchReceipt {
    pub schema: String,
    pub patch_id: String,
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub core_generated: bool,
    pub core_self_approved: bool,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
    pub installed: bool,
    pub rolled_back: bool,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub format_check: Option<LocalCommandReceipt>,
    #[serde(default)]
    pub compile_check: Option<LocalCommandReceipt>,
    pub validation: LocalCommandReceipt,
    pub release_build: Option<LocalCommandReceipt>,
    pub runtime_update_staged: bool,
    pub rollback_source: PathBuf,
    #[serde(default)]
    pub workspace_fingerprint_before: String,
    #[serde(default)]
    pub workspace_fingerprint_after: String,
    #[serde(default)]
    pub workspace_stable_during_validation: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMutationStagingCleanup {
    pub consumed_generations_scanned: usize,
    pub generations_retained: usize,
    pub generations_removed: usize,
    pub bytes_removed: u64,
    pub unverified_generations_skipped: usize,
    pub pending_handoff_preserved: bool,
}

pub fn validate_policy(policy: &AutonomousSourceMutationPolicy) -> Result<(), String> {
    if !policy.enabled {
        return Ok(());
    }
    if !policy.source_root.is_absolute() || !policy.source_root.is_dir() {
        return Err("SOURCE_MUTATION_ROOT_INVALID".to_string());
    }
    if !policy.cargo_executable.is_absolute() || !policy.cargo_executable.is_file() {
        return Err("SOURCE_MUTATION_CARGO_INVALID".to_string());
    }
    if !policy.build_target_dir.is_absolute() || !policy.runtime_bin_dir.is_absolute() {
        return Err("SOURCE_MUTATION_BUILD_OR_RUNTIME_ROOT_INVALID".to_string());
    }
    if policy.max_candidate_bytes == 0
        || policy.max_installations == 0
        || policy.validation_timeout_ms < 1_000
        || !(3..=4).contains(&policy.max_attempts_per_problem)
        || policy.minimum_predicted_value > 100
    {
        return Err("SOURCE_MUTATION_BOUND_INVALID".to_string());
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("SOURCE_MUTATION_PARENT_MISSING:{}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("SOURCE_MUTATION_MKDIR:{}:{error}", parent.display()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("SOURCE_MUTATION_CREATE:{}:{error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("SOURCE_MUTATION_WRITE:{}:{error}", path.display()))
}

fn write_immutable_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SOURCE_MUTATION_JSON:{error}"))?;
    write_new_file(path, &bytes)
}

fn write_mutable_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_JSON:{error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "SOURCE_REPAIR_LEARNING_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("SOURCE_REPAIR_LEARNING_MKDIR:{error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_OPEN:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_WRITE:{error}"))
}

fn typed_operator_json_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_JSON:{error}"))
}

fn persist_installed_typed_mechanism_operator(
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
    receipt: &AutonomousSourcePatchReceipt,
) -> Result<(), String> {
    let Some(recipe) = &request.typed_mechanism_operator_recipe else {
        return Ok(());
    };
    validate_typed_mechanism_recipe_binding(request)?;
    if !receipt.installed
        || receipt.rolled_back
        || receipt.failure_reason.is_some()
        || !receipt
            .format_check
            .as_ref()
            .is_some_and(|check| check.success)
        || !receipt
            .compile_check
            .as_ref()
            .is_some_and(|check| check.success)
        || !receipt.validation.success
        || !receipt
            .release_build
            .as_ref()
            .is_some_and(|build| build.success)
        || !receipt.workspace_stable_during_validation
        || receipt.receipt_sha256.len() != 64
        || receipt.candidate_sha256 != request.candidate_sha256
    {
        return Err("INSTALLED_TYPED_OPERATOR_WITHOUT_COMPLETE_VALIDATION".to_string());
    }

    let mut operator = recipe.clone();
    operator.evidence_sha256 = receipt.validation.output_sha256.clone();
    validate_typed_mechanism_improvement_operator(&operator)?;
    let operator_directory = typed_mechanism_operator_directory(state_dir);
    let authority_directory = typed_mechanism_operator_authority_directory(state_dir);
    fs::create_dir_all(&operator_directory)
        .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_DIRECTORY:{error}"))?;
    fs::create_dir_all(&authority_directory)
        .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_AUTHORITY_DIRECTORY:{error}"))?;
    let operator_path = operator_directory.join(format!("{}.json", operator.operator_id));
    if operator_path.exists() {
        let bytes = fs::read(&operator_path)
            .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_READ:{error}"))?;
        let stored: TypedMechanismImprovementOperatorIR = serde_json::from_slice(&bytes)
            .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_PARSE:{error}"))?;
        validate_typed_mechanism_improvement_operator(&stored)?;
        let mut stored_identity = stored.clone();
        stored_identity.evidence_sha256.clear();
        let mut requested_identity = operator.clone();
        requested_identity.evidence_sha256.clear();
        if stored_identity != requested_identity {
            return Err("INSTALLED_TYPED_OPERATOR_REPOSITORY_COLLISION".to_string());
        }
        // Preserve immutable first evidence. A new authority receipt below
        // records the latest verified generation without rewriting history.
        operator = stored;
    } else {
        write_immutable_json(&operator_path, &operator)?;
    }

    let repair_id = sha256(
        format!(
            "INSTALLED_TYPED_SOURCE_REPAIR_1:{}:{}:{}",
            request.patch_id, request.predecessor_sha256, request.candidate_sha256
        )
        .as_bytes(),
    );
    let authority_id = sha256(
        format!(
            "INSTALLED_TYPED_OPERATOR_AUTHORITY_1:{}:{}:{}:{}",
            operator.operator_id, repair_id, receipt.receipt_sha256, operator.evidence_sha256
        )
        .as_bytes(),
    );
    let mut authority = TypedMechanismOperatorAuthorityReceiptIR {
        schema: INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA.to_string(),
        authority_id: authority_id.clone(),
        operator_id: operator.operator_id.clone(),
        operator_sha256: typed_operator_json_sha256(&operator)?,
        repair_id,
        repair_receipt_sha256: receipt.receipt_sha256.clone(),
        sandbox_output_sha256: operator.evidence_sha256.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        sandbox_verified: true,
        sandbox_cleaned: true,
        authoritative_scope_stable: true,
        candidate_installed: true,
        authoritative_source_write_events: 1,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
        promotion_generation: request.source_generation,
        receipt_sha256: String::new(),
    };
    authority.receipt_sha256 = typed_operator_json_sha256(&authority)?;
    validate_typed_mechanism_operator_authority(&authority)?;
    let authority_path = authority_directory.join(format!("{authority_id}.json"));
    if authority_path.exists() {
        let bytes = fs::read(&authority_path)
            .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_AUTHORITY_READ:{error}"))?;
        let stored: TypedMechanismOperatorAuthorityReceiptIR = serde_json::from_slice(&bytes)
            .map_err(|error| format!("INSTALLED_TYPED_OPERATOR_AUTHORITY_PARSE:{error}"))?;
        if stored != authority {
            return Err("INSTALLED_TYPED_OPERATOR_AUTHORITY_COLLISION".to_string());
        }
    } else {
        write_immutable_json(&authority_path, &authority)?;
    }
    Ok(())
}

fn reconcile_installed_typed_mechanism_operators(state_dir: &Path) -> Result<usize, String> {
    let mutation_directory = state_dir.join("source_mutations");
    if !mutation_directory.is_dir() {
        return Ok(0);
    }
    let mut mutations = fs::read_dir(&mutation_directory)
        .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_ENTRY:{error}"))?
        .into_iter()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            (modified, entry.path())
        })
        .collect::<Vec<_>>();
    mutations.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    mutations.truncate(MAX_TYPED_OPERATOR_RECONCILIATION_RECEIPTS);
    let mut reconciled = 0_usize;
    for (_, mutation) in mutations {
        let request_path = mutation.join("request.json");
        let receipt_path = mutation.join("receipt.json");
        if !request_path.is_file() || !receipt_path.is_file() {
            continue;
        }
        let request_bytes = fs::read(&request_path)
            .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_REQUEST_READ:{error}"))?;
        let request: AutonomousSourcePatchRequest = serde_json::from_slice(&request_bytes)
            .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_REQUEST_PARSE:{error}"))?;
        if request.typed_mechanism_operator_recipe.is_none() {
            continue;
        }
        if request.typed_mechanism_synthesis_receipt.is_none()
            || request.typed_mechanism_materialized_edit.is_none()
        {
            // Historical proposal-only bindings remain readable evidence but
            // cannot cross the stronger revision-29 authority boundary.
            continue;
        }
        let receipt_bytes = fs::read(&receipt_path)
            .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_RECEIPT_READ:{error}"))?;
        let receipt: AutonomousSourcePatchReceipt = serde_json::from_slice(&receipt_bytes)
            .map_err(|error| format!("TYPED_OPERATOR_RECONCILE_RECEIPT_PARSE:{error}"))?;
        if receipt.installed {
            persist_installed_typed_mechanism_operator(state_dir, &request, &receipt)?;
            reconciled = reconciled.saturating_add(1);
        }
    }
    Ok(reconciled)
}

fn repair_problem_id(request: &AutonomousSourcePatchRequest) -> String {
    repair_problem_id_for(&request.relative_path, &request.transformation)
}

fn repair_learning_path(state_dir: &Path, problem_id: &str) -> PathBuf {
    state_dir
        .join("source_repair_knowledge")
        .join(format!("{problem_id}.json"))
}

fn load_repair_learning(
    state_dir: &Path,
    problem_id: &str,
) -> Result<Option<SourceRepairLearningRecord>, String> {
    let path = repair_learning_path(state_dir, problem_id);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| format!("SOURCE_REPAIR_LEARNING_READ:{error}"))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_PARSE:{error}"))
}

pub fn source_patch_failure_is_transient(reason: Option<&str>) -> bool {
    matches!(
        reason,
        Some(
            "CONCURRENT_WORKSPACE_CHANGE_DURING_VALIDATION"
                | "TARGET_CHANGED_DURING_VALIDATION"
                | "SOURCE_UPDATE_ALREADY_STAGED"
        )
    )
}

fn source_repair_attempt_is_causal(attempt: &SourceRepairAttempt) -> bool {
    !source_patch_failure_is_transient(attempt.failure_reason.as_deref())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Operator dispatch was introduced after structurally replayed source repairs
/// were already being installed and rolled back. Those older attempts contain
/// the exact program hash, edit algebra, postcondition count, candidate hash,
/// and immutable validation receipt, but cannot contain a dispatcher-generated
/// operator id. Treat that bounded pre-revision shape as executable bootstrap
/// evidence instead of leaving the callable operator repository permanently
/// empty. New-revision attempts must carry the full causal dispatcher binding.
fn legacy_attempt_has_executable_operator_evidence(attempt: &SourceRepairAttempt) -> bool {
    attempt.source_engine_revision == 0
        && attempt.executed_operator_id.is_none()
        && attempt.improvement_operator_execution_sha256.is_none()
        && attempt
            .structural_repair_program_sha256
            .as_deref()
            .is_some_and(is_sha256_hex)
        && is_sha256_hex(&attempt.candidate_sha256)
        && is_sha256_hex(&attempt.receipt_sha256)
        && is_sha256_hex(&attempt.diagnostic_sha256)
        && !attempt.edit_atom_kinds.is_empty()
        && attempt.edit_atom_kinds.iter().all(|kind| {
            matches!(
                kind.as_str(),
                "REPLACE" | "INSERT" | "DELETE" | "MOVE" | "ATOMIC_MULTI_EDIT"
            )
        })
        && attempt.structural_postcondition_count > 0
        && attempt.family_member_count > 0
}

fn active_cycle_attempts(
    record: &SourceRepairLearningRecord,
    source_generation: u64,
) -> Vec<&SourceRepairAttempt> {
    if (record.status == "ADMITTED_FAILURE"
        && record
            .eligible_after_generation
            .is_some_and(|eligible| source_generation >= eligible))
        || (record.status != "LEARNED_SUCCESS"
            && record.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION)
    {
        Vec::new()
    } else {
        record
            .attempts
            .iter()
            .skip(record.cycle_attempt_start_index.min(record.attempts.len()))
            .filter(|attempt| source_repair_attempt_is_causal(attempt))
            .collect()
    }
}

fn collect_edit_atom_kinds(edit: &SourceEditAtom, kinds: &mut Vec<String>) {
    let kind = match edit {
        SourceEditAtom::Replace { .. } => "REPLACE",
        SourceEditAtom::Insert { .. } => "INSERT",
        SourceEditAtom::Delete { .. } => "DELETE",
        SourceEditAtom::Move { .. } => "MOVE",
        SourceEditAtom::AtomicMultiEdit { edits } => {
            kinds.push("ATOMIC_MULTI_EDIT".to_string());
            for nested in edits {
                collect_edit_atom_kinds(nested, kinds);
            }
            return;
        }
    };
    kinds.push(kind.to_string());
}

fn structural_program_learning_features(
    request: &AutonomousSourcePatchRequest,
) -> Result<(Option<String>, Vec<String>, usize), String> {
    let Some(program) = &request.structural_repair_program else {
        return Ok((None, Vec::new(), 0));
    };
    let encoded = serde_json::to_vec(program)
        .map_err(|error| format!("STRUCTURAL_REPAIR_PROGRAM_SERIALIZE:{error}"))?;
    let mut edit_atom_kinds = Vec::new();
    collect_edit_atom_kinds(&program.edit, &mut edit_atom_kinds);
    Ok((
        Some(sha256(&encoded)),
        edit_atom_kinds,
        program.postconditions.len(),
    ))
}

fn generalized_change_learning_features(
    request: &AutonomousSourcePatchRequest,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(change) = &request.generalized_change else {
        return Ok((None, Vec::new()));
    };
    let encoded = serde_json::to_vec(change)
        .map_err(|error| format!("GENERALIZED_CHANGE_SERIALIZE:{error}"))?;
    Ok((
        Some(sha256(&encoded)),
        change.derived_from_counterexample_ids.clone(),
    ))
}

fn normalized_solution_strategy_family(solution_strategy: &str) -> String {
    normalized_hash_suffixed_family(solution_strategy).to_string()
}

fn normalized_hash_suffixed_family(value: &str) -> &str {
    value
        .rsplit_once(':')
        .filter(|(_, suffix)| {
            suffix.len() >= 12 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .map(|(family, _)| family)
        .unwrap_or(value)
}

fn normalized_repair_transformation(value: &str) -> &str {
    if value.starts_with("COMPILER_OBSERVATION") {
        normalized_hash_suffixed_family(value)
    } else {
        value
    }
}

fn structural_postcondition_class(count: usize) -> &'static str {
    match count {
        0 => "NONE",
        1 => "ONE",
        2..=3 => "FEW",
        _ => "MANY",
    }
}

fn improvement_operator_generator_kind(
    transformation: &str,
    solution_strategy: &str,
) -> ImprovementOperatorGeneratorKind {
    if solution_strategy == "EMIT_TYPED_RUST_AND_ACTIVATE_CALLABLE" {
        ImprovementOperatorGeneratorKind::ProgramIrLowering
    } else if solution_strategy.starts_with("COMPILER_SUGGESTION") {
        ImprovementOperatorGeneratorKind::CompilerSuggestedEdit
    } else if solution_strategy.starts_with("GRAMMAR_COMPOSITION:") {
        ImprovementOperatorGeneratorKind::TypedGrammarComposition
    } else if KNOWN_REMAINDER_STRATEGIES.contains(&solution_strategy) {
        ImprovementOperatorGeneratorKind::KnownStructuralRewrite
    } else if transformation.starts_with("LEARNED_SELF_HEALING::") {
        ImprovementOperatorGeneratorKind::LearnedSelfHealingLowering
    } else {
        ImprovementOperatorGeneratorKind::KnownStructuralRewrite
    }
}

fn inferred_weakness_evidence_kind(transformation: &str) -> WeaknessEvidenceKind {
    if transformation.contains("PUBLIC_EXAMPLE_CONTRADICTED") {
        WeaknessEvidenceKind::PublicBehaviorContradiction
    } else if transformation.contains("AST_GRAMMAR_HOLE") {
        WeaknessEvidenceKind::ExplicitCodeHole
    } else if transformation.starts_with("COMPILER_") || transformation.contains(":clippy::") {
        WeaknessEvidenceKind::CompilerDiagnostic
    } else {
        WeaknessEvidenceKind::StructuralSourceSmell
    }
}

fn improvement_operator_ir_from_features(
    weakness_evidence_kind: WeaknessEvidenceKind,
    transformation: &str,
    solution_strategy: &str,
    edit_atom_kinds: &[String],
    structural_postcondition_count: usize,
) -> Result<ImprovementOperatorIR, String> {
    let mut normalized_edit_atom_kinds = edit_atom_kinds.to_vec();
    normalized_edit_atom_kinds.sort();
    normalized_edit_atom_kinds.dedup();
    let generator_kind = improvement_operator_generator_kind(transformation, solution_strategy);
    let solution_strategy_family = normalized_solution_strategy_family(solution_strategy);
    let executable_payload =
        if generator_kind == ImprovementOperatorGeneratorKind::KnownStructuralRewrite {
            KnownStructuralRewriteIR::from_strategy(&solution_strategy_family).map(|rewrite| {
                ExecutableImprovementOperatorPayloadIR::KnownStructuralRewrite { rewrite }
            })
        } else {
            None
        };
    let mut operator = ImprovementOperatorIR {
        schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
        operator_id: String::new(),
        weakness_evidence_kind,
        generator_kind,
        executable_payload,
        solution_strategy_family,
        edit_atom_kinds: normalized_edit_atom_kinds,
        structural_postcondition_class: structural_postcondition_class(
            structural_postcondition_count,
        )
        .to_string(),
        validation_contract: vec![
            "STRUCTURAL_REPLAY".to_string(),
            "FORMAT".to_string(),
            "COMPILE_CLIPPY".to_string(),
            "PUBLIC_REGRESSION".to_string(),
            "RELEASE_BUILD".to_string(),
            "WORKSPACE_INTEGRITY".to_string(),
        ],
    };
    let encoded = serde_json::to_vec(&operator)
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_SERIALIZE:{error}"))?;
    operator.operator_id = sha256(&encoded);
    Ok(operator)
}

fn improvement_operator_ir_for_program(
    weakness_evidence_kind: WeaknessEvidenceKind,
    transformation: &str,
    solution_strategy: &str,
    program: &StructuralRepairProgram,
) -> Result<ImprovementOperatorIR, String> {
    let mut edit_atom_kinds = Vec::new();
    collect_edit_atom_kinds(&program.edit, &mut edit_atom_kinds);
    improvement_operator_ir_from_features(
        weakness_evidence_kind,
        transformation,
        solution_strategy,
        &edit_atom_kinds,
        program.postconditions.len(),
    )
}

pub(crate) fn source_patch_validation_critical_path_ms(
    receipt: &AutonomousSourcePatchReceipt,
) -> u64 {
    fn distinct_duration<'a>(commands: impl IntoIterator<Item = &'a LocalCommandReceipt>) -> u64 {
        let mut seen = BTreeSet::new();
        let mut duration_ms = 0_u64;
        for command in commands {
            let identity = format!(
                "{}:{}:{}:{}",
                command.program,
                command.args.join("\u{1f}"),
                command.output_sha256,
                command.duration_ms
            );
            if seen.insert(identity) {
                duration_ms = duration_ms.saturating_add(command.duration_ms);
            }
        }
        duration_ms
    }

    let format_duration_ms = distinct_duration(receipt.format_check.iter());
    let compile_lane_duration_ms = distinct_duration(receipt.compile_check.iter());
    let test_lane_duration_ms = distinct_duration(std::iter::once(&receipt.validation));
    let release_lane_duration_ms = distinct_duration(receipt.release_build.iter());

    // Formatting is the cheap mutation gate. After it passes, Clippy, tests,
    // and the runtime artifact build execute concurrently in separate Cargo
    // target directories, so wall time is the slowest lane, not their sum.
    format_duration_ms.saturating_add(
        compile_lane_duration_ms
            .max(test_lane_duration_ms)
            .max(release_lane_duration_ms),
    )
}

pub fn derive_improvement_operator_memory(
    state_dir: &Path,
) -> Result<ImprovementOperatorMemory, String> {
    let knowledge_dir = state_dir.join("source_repair_knowledge");
    if !knowledge_dir.is_dir() {
        return Ok(ImprovementOperatorMemory {
            schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
            profiles: Vec::new(),
            total_attempts: 0,
            total_successful_uses: 0,
            repository_guided_attempts: 0,
            repository_guided_successful_uses: 0,
            productive_cross_family_transfers: 0,
        });
    }
    let mut paths = fs::read_dir(&knowledge_dir)
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_MEMORY_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_MEMORY_ENTRY:{error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut profiles = BTreeMap::<String, ImprovementOperatorProfile>::new();
    for path in paths {
        let bytes =
            fs::read(&path).map_err(|error| format!("IMPROVEMENT_OPERATOR_MEMORY_READ:{error}"))?;
        let record: SourceRepairLearningRecord = serde_json::from_slice(&bytes)
            .map_err(|error| format!("IMPROVEMENT_OPERATOR_MEMORY_PARSE:{error}"))?;
        for attempt in &record.attempts {
            if !source_repair_attempt_is_causal(attempt)
                || attempt.structural_repair_program_sha256.is_none()
                || attempt.edit_atom_kinds.is_empty()
            {
                continue;
            }
            let dispatcher_bound = attempt.executed_operator_id.is_some()
                && attempt.improvement_operator_execution_sha256.is_some();
            let legacy_executable_evidence =
                legacy_attempt_has_executable_operator_evidence(attempt);
            if !dispatcher_bound && !legacy_executable_evidence {
                continue;
            }
            let evidence_kind = attempt
                .weakness_evidence_kind
                .unwrap_or_else(|| inferred_weakness_evidence_kind(&record.transformation));
            let operator = improvement_operator_ir_from_features(
                evidence_kind,
                &record.transformation,
                &attempt.solution_strategy,
                &attempt.edit_atom_kinds,
                attempt.structural_postcondition_count,
            )?;
            if dispatcher_bound
                && attempt.executed_operator_id.as_deref() != Some(operator.operator_id.as_str())
            {
                continue;
            }
            let operator_id = operator.operator_id.clone();
            let profile =
                profiles
                    .entry(operator_id.clone())
                    .or_insert_with(|| ImprovementOperatorProfile {
                        operator,
                        attempts: 0,
                        successful_uses: 0,
                        rollbacks: 0,
                        repository_guided_attempts: 0,
                        repository_guided_successful_uses: 0,
                        cumulative_validation_duration_ms: 0,
                        attempted_opportunity_kinds: BTreeSet::new(),
                        attempted_family_ids: BTreeSet::new(),
                        successful_family_ids: BTreeSet::new(),
                    });
            profile.attempts = profile.attempts.saturating_add(1);
            let repository_guided = attempt
                .invoked_operator_ids
                .iter()
                .any(|invoked| invoked == &operator_id);
            if repository_guided {
                profile.repository_guided_attempts =
                    profile.repository_guided_attempts.saturating_add(1);
            }
            profile.cumulative_validation_duration_ms = profile
                .cumulative_validation_duration_ms
                .saturating_add(attempt.validation_duration_ms);
            profile
                .attempted_opportunity_kinds
                .insert(attempt.opportunity_kind);
            if !attempt.opportunity_family_id.is_empty() {
                profile
                    .attempted_family_ids
                    .insert(attempt.opportunity_family_id.clone());
            }
            if attempt.succeeded {
                profile.successful_uses = profile.successful_uses.saturating_add(1);
                if repository_guided {
                    profile.repository_guided_successful_uses =
                        profile.repository_guided_successful_uses.saturating_add(1);
                }
                if !attempt.opportunity_family_id.is_empty() {
                    profile
                        .successful_family_ids
                        .insert(attempt.opportunity_family_id.clone());
                }
            } else {
                profile.rollbacks = profile.rollbacks.saturating_add(1);
            }
        }
    }
    let profiles = profiles.into_values().collect::<Vec<_>>();
    Ok(ImprovementOperatorMemory {
        schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
        total_attempts: profiles.iter().map(|profile| profile.attempts).sum(),
        total_successful_uses: profiles.iter().map(|profile| profile.successful_uses).sum(),
        repository_guided_attempts: profiles
            .iter()
            .map(|profile| profile.repository_guided_attempts)
            .sum(),
        repository_guided_successful_uses: profiles
            .iter()
            .map(|profile| profile.repository_guided_successful_uses)
            .sum(),
        productive_cross_family_transfers: profiles
            .iter()
            .map(|profile| {
                profile
                    .successful_family_ids
                    .len()
                    .saturating_sub(1)
                    .min(u64::MAX as usize) as u64
            })
            .sum(),
        profiles,
    })
}

fn improvement_operator_repository_path(state_dir: &Path, operator_id: &str) -> PathBuf {
    state_dir
        .join("improvement_operator_repository")
        .join("operators")
        .join(format!("{operator_id}.json"))
}

fn validate_improvement_operator_id(operator: &ImprovementOperatorIR) -> Result<(), String> {
    match &operator.executable_payload {
        Some(ExecutableImprovementOperatorPayloadIR::KnownStructuralRewrite { rewrite })
            if operator.generator_kind
                != ImprovementOperatorGeneratorKind::KnownStructuralRewrite
                || KnownStructuralRewriteIR::from_strategy(&operator.solution_strategy_family)
                    != Some(*rewrite) =>
        {
            return Err("IMPROVEMENT_OPERATOR_EXECUTABLE_PAYLOAD_MISMATCH".to_string());
        }
        Some(ExecutableImprovementOperatorPayloadIR::KnownStructuralRewrite { .. }) => {}
        None => {}
    }
    let mut identity = operator.clone();
    identity.operator_id.clear();
    let encoded = serde_json::to_vec(&identity)
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_ID_SERIALIZE:{error}"))?;
    if sha256(&encoded) != operator.operator_id {
        return Err("IMPROVEMENT_OPERATOR_ID_MISMATCH".to_string());
    }
    Ok(())
}

pub fn refresh_improvement_operator_repository(
    state_dir: &Path,
) -> Result<ImprovementOperatorMemory, String> {
    let memory = derive_improvement_operator_memory(state_dir)?;
    let mut active_operator_ids = BTreeSet::new();
    for profile in &memory.profiles {
        if profile.successful_uses == 0 || !profile.operator.can_synthesize_from_source() {
            continue;
        }
        active_operator_ids.insert(profile.operator.operator_id.clone());
        validate_improvement_operator_id(&profile.operator)?;
        let path = improvement_operator_repository_path(state_dir, &profile.operator.operator_id);
        if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("IMPROVEMENT_OPERATOR_REPOSITORY_READ:{error}"))?;
            let stored: ImprovementOperatorIR = serde_json::from_slice(&bytes)
                .map_err(|error| format!("IMPROVEMENT_OPERATOR_REPOSITORY_PARSE:{error}"))?;
            validate_improvement_operator_id(&stored)?;
            if stored != profile.operator {
                return Err("IMPROVEMENT_OPERATOR_REPOSITORY_COLLISION".to_string());
            }
        } else {
            write_immutable_json(&path, &profile.operator)?;
        }
    }
    let operator_dir = state_dir
        .join("improvement_operator_repository")
        .join("operators");
    if operator_dir.is_dir() {
        for entry in fs::read_dir(&operator_dir)
            .map_err(|error| format!("IMPROVEMENT_OPERATOR_REPOSITORY_READ_DIR:{error}"))?
        {
            let path = entry
                .map_err(|error| format!("IMPROVEMENT_OPERATOR_REPOSITORY_ENTRY:{error}"))?
                .path();
            let Some(operator_id) = path.file_stem().and_then(OsStr::to_str) else {
                continue;
            };
            if path.extension().and_then(OsStr::to_str) == Some("json")
                && !active_operator_ids.contains(operator_id)
            {
                fs::remove_file(&path).map_err(|error| {
                    format!("IMPROVEMENT_OPERATOR_REPOSITORY_STALE_REMOVE:{error}")
                })?;
            }
        }
    }
    Ok(memory)
}

pub fn execute_improvement_operator_on_source(
    operator: &ImprovementOperatorIR,
    source: &str,
) -> Result<ImprovementOperatorExecution, String> {
    validate_improvement_operator_id(operator)?;
    match &operator.executable_payload {
        Some(ExecutableImprovementOperatorPayloadIR::KnownStructuralRewrite { rewrite }) => {
            let candidate_source =
                rewrite_first_known_improvement(source, rewrite.strategy_index());
            Ok(ImprovementOperatorExecution {
                schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
                operator_id: operator.operator_id.clone(),
                generator_kind: operator.generator_kind,
                applicable: candidate_source.is_some(),
                candidate_source,
                execution_reason: "EXECUTED_BOUND_STRUCTURAL_REWRITE_PAYLOAD".to_string(),
            })
        }
        None => Ok(ImprovementOperatorExecution {
            schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
            operator_id: operator.operator_id.clone(),
            generator_kind: operator.generator_kind,
            applicable: false,
            candidate_source: None,
            execution_reason: "NO_EXECUTABLE_SOURCE_SYNTHESIS_PAYLOAD".to_string(),
        }),
    }
}

/// Executes a typed structural repair program through an ImprovementOperatorIR.
///
/// The source-only entry point above remains useful for operators that own a
/// complete repository-independent rewrite. Compiler, grammar, ProgramIR, and
/// learned self-healing operators instead consume a typed repair program. This
/// entry point binds that program's edit algebra and postcondition class to the
/// operator identity before replaying it against the exact predecessor.
pub fn execute_improvement_operator_program_on_source(
    operator: &ImprovementOperatorIR,
    source: &str,
    program: &StructuralRepairProgram,
) -> Result<ImprovementOperatorExecution, String> {
    validate_improvement_operator_id(operator)?;
    let mut observed_edit_atom_kinds = Vec::new();
    collect_edit_atom_kinds(&program.edit, &mut observed_edit_atom_kinds);
    observed_edit_atom_kinds.sort();
    observed_edit_atom_kinds.dedup();
    if observed_edit_atom_kinds != operator.edit_atom_kinds {
        return Err("IMPROVEMENT_OPERATOR_EDIT_ALGEBRA_MISMATCH".to_string());
    }
    if structural_postcondition_class(program.postconditions.len())
        != operator.structural_postcondition_class
    {
        return Err("IMPROVEMENT_OPERATOR_POSTCONDITION_CLASS_MISMATCH".to_string());
    }
    if !operator
        .validation_contract
        .iter()
        .any(|obligation| obligation == "STRUCTURAL_REPLAY")
    {
        return Err("IMPROVEMENT_OPERATOR_STRUCTURAL_REPLAY_NOT_REQUIRED".to_string());
    }
    let replay = execute_structural_repair(program, source)
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_STRUCTURAL_REPLAY:{error}"))?;
    let applicable = replay.structurally_verified && replay.exact_target_observed;
    Ok(ImprovementOperatorExecution {
        schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
        operator_id: operator.operator_id.clone(),
        generator_kind: operator.generator_kind,
        applicable,
        candidate_source: applicable.then_some(replay.candidate_source),
        execution_reason: if applicable {
            "EXECUTED_TYPED_STRUCTURAL_PROGRAM".to_string()
        } else {
            "TYPED_STRUCTURAL_PROGRAM_SELF_FALSIFIED".to_string()
        },
    })
}

/// Selects a previously successful operator when one exactly matches, or uses
/// the freshly derived operator as the bootstrap executable. In both cases the
/// returned invocation and execution are causally bound to the same typed
/// program and predecessor source.
pub fn invoke_and_execute_improvement_operator(
    memory: &ImprovementOperatorMemory,
    weakness_evidence_kind: WeaknessEvidenceKind,
    transformation: &str,
    solution_strategy: &str,
    program: &StructuralRepairProgram,
    opportunity_family_id: &str,
    predecessor_source: &str,
) -> Result<(ImprovementOperatorInvocation, ImprovementOperatorExecution), String> {
    let requested = improvement_operator_ir_for_program(
        weakness_evidence_kind,
        transformation,
        solution_strategy,
        program,
    )?;
    let invocation = invoke_improvement_operator_repository(
        memory,
        weakness_evidence_kind,
        transformation,
        solution_strategy,
        program,
        opportunity_family_id,
    )?;
    let operator = if let Some(operator_id) = invocation.matched_operator_ids.first() {
        memory
            .profiles
            .iter()
            .find(|profile| &profile.operator.operator_id == operator_id)
            .map(|profile| &profile.operator)
            .ok_or_else(|| "IMPROVEMENT_OPERATOR_PROFILE_MISSING".to_string())?
    } else {
        &requested
    };
    let execution =
        execute_improvement_operator_program_on_source(operator, predecessor_source, program)?;
    Ok((invocation, execution))
}

fn improvement_operator_canary_scenario(selector: usize) -> (&'static str, &'static str) {
    match selector % 5 {
        0 => (
            "pub fn alpha() -> i32 { 1 }\n",
            "pub fn alpha() -> i32 { 2 }\n",
        ),
        1 => (
            "pub fn alpha() -> i32 { 1 }\n",
            "pub const LIMIT: i32 = 2;\npub fn alpha() -> i32 { 1 }\n",
        ),
        2 => (
            "pub const LIMIT: i32 = 2;\npub fn alpha() -> i32 { 1 }\n",
            "pub fn alpha() -> i32 { 1 }\n",
        ),
        3 => (
            "pub fn alpha() -> i32 { 1 }\npub fn beta() -> i32 { 2 }\n",
            "pub fn beta() -> i32 { 2 }\npub fn alpha() -> i32 { 1 }\n",
        ),
        _ => (
            "pub fn alpha() -> i32 { 1 }\npub fn beta() -> i32 { 2 }\n",
            "pub fn alpha() -> i32 { 3 }\npub fn beta() -> i32 { 4 }\n",
        ),
    }
}

fn improvement_operator_canary_identity(
    selector: usize,
) -> (WeaknessEvidenceKind, &'static str, &'static str) {
    match (selector / 5) % 5 {
        0 => (
            WeaknessEvidenceKind::StructuralSourceSmell,
            "CANARY_STRUCTURAL_REWRITE",
            "STRUCTURAL_AST_REWRITE",
        ),
        1 => (
            WeaknessEvidenceKind::CompilerDiagnostic,
            "COMPILER_CANARY_DIAGNOSTIC",
            "COMPILER_SUGGESTION:CANONICAL_TYPED_EDIT",
        ),
        2 => (
            WeaknessEvidenceKind::ExplicitCodeHole,
            "AST_GRAMMAR_HOLE:CANARY",
            "GRAMMAR_COMPOSITION:CANONICAL_TYPED_EDIT",
        ),
        3 => (
            WeaknessEvidenceKind::StructuralSourceSmell,
            "SEM5_PROGRAM_IR_TO_ACTIVE_RUNTIME_CALLABLE",
            "EMIT_TYPED_RUST_AND_ACTIVATE_CALLABLE",
        ),
        _ => (
            WeaknessEvidenceKind::PublicBehaviorContradiction,
            "LEARNED_SELF_HEALING::CANARY",
            "LEARNED_COMPOSITION",
        ),
    }
}

#[derive(Debug, Clone)]
struct ImprovementOperatorCanaryCase {
    receipt: ImprovementOperatorBehavioralCanaryReceipt,
    program: StructuralRepairProgram,
    predecessor: String,
    target: String,
}

fn improvement_operator_canary_case(
    selector: usize,
    context_sha256: &str,
) -> Result<ImprovementOperatorCanaryCase, String> {
    let (predecessor, target) = improvement_operator_canary_scenario(selector);
    let (evidence_kind, transformation, solution_strategy) =
        improvement_operator_canary_identity(selector);
    let program = synthesize_structural_repair("canary.rs", predecessor, target)?;
    let operator = improvement_operator_ir_for_program(
        evidence_kind,
        transformation,
        solution_strategy,
        &program,
    )?;
    let execution =
        execute_improvement_operator_program_on_source(&operator, predecessor, &program)?;
    let exact_candidate_observed = execution.applicable
        && execution.candidate_source.as_deref() == Some(target)
        && sha256(target.as_bytes()) == program.target_source_sha256;
    let wrong_predecessor_rejected =
        execute_improvement_operator_program_on_source(&operator, target, &program).is_err();
    let mut tampered = program.clone();
    tampered.target_source_sha256 = sha256(b"different target");
    let tampered_target_rejected =
        execute_improvement_operator_program_on_source(&operator, predecessor, &tampered)
            .is_ok_and(|result| !result.applicable && result.candidate_source.is_none());
    let cases_executed = 3;
    let cases_passed = [
        exact_candidate_observed,
        wrong_predecessor_rejected,
        tampered_target_rejected,
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let structural_repair_program_sha256 = sha256(
        &serde_json::to_vec(&program)
            .map_err(|error| format!("IMPROVEMENT_OPERATOR_CANARY_PROGRAM_JSON:{error}"))?,
    );
    let mut receipt = ImprovementOperatorBehavioralCanaryReceipt {
        schema: "B_CORE_IMPROVEMENT_OPERATOR_BEHAVIORAL_CANARY_1".to_string(),
        context_sha256: context_sha256.to_string(),
        operator,
        structural_repair_program_sha256,
        candidate_sha256: sha256(target.as_bytes()),
        cases_executed,
        cases_passed,
        exact_candidate_observed,
        wrong_predecessor_rejected,
        tampered_target_rejected,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(
        &serde_json::to_vec(&receipt)
            .map_err(|error| format!("IMPROVEMENT_OPERATOR_CANARY_RECEIPT_JSON:{error}"))?,
    );
    Ok(ImprovementOperatorCanaryCase {
        receipt,
        program,
        predecessor: predecessor.to_string(),
        target: target.to_string(),
    })
}

/// Runs one deterministic, context-selected operator over executable Rust AST
/// states and checks both its postimage and two negative cases. The returned
/// artifact identity is the generalized operator, not the scenario source.
pub fn execute_improvement_operator_behavioral_canary(
    context_sha256: &str,
) -> Result<ImprovementOperatorBehavioralCanaryReceipt, String> {
    if context_sha256.len() != 64 || !context_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("IMPROVEMENT_OPERATOR_CANARY_CONTEXT_INVALID".to_string());
    }
    let selector = usize::from_str_radix(&context_sha256[..8], 16)
        .map_err(|error| format!("IMPROVEMENT_OPERATOR_CANARY_SELECTOR:{error}"))?;
    Ok(improvement_operator_canary_case(selector, context_sha256)?.receipt)
}

fn improvement_operator_canary_case_for_id(
    operator_id: &str,
    context_sha256: &str,
) -> Result<ImprovementOperatorCanaryCase, String> {
    for selector in 0..25 {
        let selector_context = format!("{selector:08x}{}", &context_sha256[8..]);
        let candidate = improvement_operator_canary_case(selector, &selector_context)?;
        if candidate.receipt.operator.operator_id == operator_id {
            return Ok(candidate);
        }
    }
    Err("IMPROVEMENT_OPERATOR_GRAPH_NODE_NOT_EXECUTABLE".to_string())
}

pub fn improvement_operator_graph_id_for_nodes(operator_ids: &[String]) -> Result<String, String> {
    if !(2..=MAX_IMPROVEMENT_OPERATOR_GRAPH_NODES).contains(&operator_ids.len())
        || operator_ids.iter().any(|operator_id| {
            operator_id.len() != 64 || !operator_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_NODE_IDS_INVALID".to_string());
    }
    let mut canonical_ids = operator_ids.to_vec();
    canonical_ids.sort();
    canonical_ids.dedup();
    if canonical_ids.len() != operator_ids.len() {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_NODE_IDS_INVALID".to_string());
    }
    Ok(sha256(
        format!(
            "B_CORE_IMPROVEMENT_OPERATOR_GRAPH_1:{}:RUST_SOURCE_SHARD:PARALLEL_JOIN:STRUCTURAL_REPLAY",
            canonical_ids.join(":")
        )
        .as_bytes(),
    ))
}

pub fn improvement_operator_graph_id(
    left_operator_id: &str,
    right_operator_id: &str,
) -> Result<String, String> {
    improvement_operator_graph_id_for_nodes(&[
        left_operator_id.to_string(),
        right_operator_id.to_string(),
    ])
}

pub fn compose_improvement_operator_graph(
    operators: &[ImprovementOperatorIR],
) -> Result<ImprovementOperatorGraphIR, String> {
    let operator_ids = operators
        .iter()
        .map(|operator| {
            validate_improvement_operator_id(operator)?;
            Ok(operator.operator_id.clone())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let Some(first) = operators.first() else {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_NODE_IDS_INVALID".to_string());
    };
    if operators.iter().any(|operator| {
        operator.validation_contract != first.validation_contract
            || !operator
                .validation_contract
                .iter()
                .any(|obligation| obligation == "STRUCTURAL_REPLAY")
    }) {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_POSTCONDITION_INCOMPATIBLE".to_string());
    }
    let mut canonical_ids = operator_ids;
    canonical_ids.sort();
    let graph_id = improvement_operator_graph_id_for_nodes(&canonical_ids)?;
    Ok(ImprovementOperatorGraphIR {
        schema: "B_CORE_IMPROVEMENT_OPERATOR_GRAPH_1".to_string(),
        graph_id,
        operator_ids: canonical_ids,
        transported_type: "RUST_SOURCE_SHARD".to_string(),
        join_postconditions: vec![
            "ALL_NODE_POSTIMAGES_EXACT".to_string(),
            "ALL_NEGATIVE_CONTROLS_REJECTED".to_string(),
            "CANONICAL_CONTENT_ADDRESSED_JOIN".to_string(),
        ],
    })
}

/// Executes a graph of typed operators over independent source shards. This
/// is the callable repository path used by the canary and by real repair
/// families; graph nodes are not proposal strings.
pub fn execute_improvement_operator_graph_on_sources(
    graph: &ImprovementOperatorGraphIR,
    nodes: &[ImprovementOperatorGraphNodeProgram],
) -> Result<Vec<ImprovementOperatorExecution>, String> {
    let operators = nodes
        .iter()
        .map(|node| node.operator.clone())
        .collect::<Vec<_>>();
    if compose_improvement_operator_graph(&operators)? != *graph {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_BINDING_MISMATCH".to_string());
    }
    let mut ordered_nodes = nodes.iter().collect::<Vec<_>>();
    ordered_nodes.sort_by(|left, right| left.operator.operator_id.cmp(&right.operator.operator_id));
    thread::scope(|scope| {
        let handles = ordered_nodes
            .iter()
            .map(|node| {
                scope.spawn(|| {
                    execute_improvement_operator_program_on_source(
                        &node.operator,
                        &node.predecessor_source,
                        &node.structural_repair_program,
                    )
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "IMPROVEMENT_OPERATOR_GRAPH_NODE_PANICKED".to_string())?
            })
            .collect::<Result<Vec<_>, String>>()
    })
}

/// Executes 2..=8 independently applicable, behaviorally verified source
/// operators as one bounded graph. Nodes run concurrently over disjoint source
/// shards; the join is content-addressed and accepts only exact postimages.
pub fn execute_improvement_operator_graph_family_behavioral_canary(
    operator_ids: &[String],
    context_sha256: &str,
) -> Result<ImprovementOperatorGraphCanaryReceipt, String> {
    if context_sha256.len() != 64 || !context_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_CONTEXT_INVALID".to_string());
    }
    improvement_operator_graph_id_for_nodes(operator_ids)?;
    let mut cases = operator_ids
        .iter()
        .map(|operator_id| improvement_operator_canary_case_for_id(operator_id, context_sha256))
        .collect::<Result<Vec<_>, _>>()?;
    cases.sort_by(|left, right| {
        left.receipt
            .operator
            .operator_id
            .cmp(&right.receipt.operator.operator_id)
    });
    let nodes = cases
        .iter()
        .map(|case| ImprovementOperatorGraphNodeProgram {
            operator: case.receipt.operator.clone(),
            predecessor_source: case.predecessor.clone(),
            structural_repair_program: case.program.clone(),
        })
        .collect::<Vec<_>>();
    let operators = nodes
        .iter()
        .map(|node| node.operator.clone())
        .collect::<Vec<_>>();
    let graph = compose_improvement_operator_graph(&operators)?;
    let executions = execute_improvement_operator_graph_on_sources(&graph, &nodes)?;
    if executions.len() != cases.len() {
        return Err("IMPROVEMENT_OPERATOR_GRAPH_EXECUTION_COUNT_MISMATCH".to_string());
    }
    let exact_postimages_observed = cases.iter().zip(&executions).all(|(case, execution)| {
        execution.applicable
            && execution.candidate_source.as_deref() == Some(case.target.as_str())
            && execution.operator_id == case.receipt.operator.operator_id
    });
    let negative_controls_rejected = cases.iter().all(|case| {
        case.receipt.wrong_predecessor_rejected && case.receipt.tampered_target_rejected
    });
    let mut reversed_ids = graph.operator_ids.clone();
    reversed_ids.reverse();
    let canonical_join_observed =
        improvement_operator_graph_id_for_nodes(&reversed_ids)? == graph.graph_id;
    let cases_executed = 4;
    let cases_passed = [
        exact_postimages_observed,
        negative_controls_rejected,
        canonical_join_observed,
        executions
            .iter()
            .map(|execution| execution.operator_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == executions.len(),
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let mut node_receipt_sha256s = cases
        .into_iter()
        .map(|case| case.receipt.receipt_sha256)
        .collect::<Vec<_>>();
    node_receipt_sha256s.sort();
    let mut receipt = ImprovementOperatorGraphCanaryReceipt {
        schema: "B_CORE_IMPROVEMENT_OPERATOR_GRAPH_CANARY_1".to_string(),
        context_sha256: context_sha256.to_string(),
        graph,
        node_receipt_sha256s,
        cases_executed,
        cases_passed,
        parallel_nodes_executed: true,
        exact_postimages_observed,
        negative_controls_rejected,
        canonical_join_observed,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(
        &serde_json::to_vec(&receipt)
            .map_err(|error| format!("IMPROVEMENT_OPERATOR_GRAPH_RECEIPT_JSON:{error}"))?,
    );
    Ok(receipt)
}

pub fn execute_improvement_operator_graph_behavioral_canary(
    left_operator_id: &str,
    right_operator_id: &str,
    context_sha256: &str,
) -> Result<ImprovementOperatorGraphCanaryReceipt, String> {
    execute_improvement_operator_graph_family_behavioral_canary(
        &[left_operator_id.to_string(), right_operator_id.to_string()],
        context_sha256,
    )
}

fn improvement_operator_transfer_priority(
    memory: &ImprovementOperatorMemory,
    operator: &ImprovementOperatorIR,
    opportunity_family_id: &str,
) -> i32 {
    let Some(profile) = memory
        .profiles
        .iter()
        .find(|profile| profile.operator.operator_id == operator.operator_id)
    else {
        return 0;
    };
    let attempts = profile.attempts.max(1);
    let successes = profile.successful_uses;
    let success_signal = successes
        .saturating_mul(60)
        .checked_div(attempts)
        .unwrap_or(0)
        .min(i32::MAX as u64) as i32
        - 30;
    let rollback_penalty = profile.rollbacks.saturating_mul(4).min(20) as i32;
    let transferred_successes = profile
        .successful_family_ids
        .iter()
        .filter(|family| family.as_str() != opportunity_family_id)
        .count();
    let transfer_signal = if transferred_successes == 0 {
        0
    } else {
        20 + i32::try_from(transferred_successes.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(5)
            .min(15)
    };
    let validation_cost_penalty = if profile.successful_uses == 0
        && profile.cumulative_validation_duration_ms > 600_000
    {
        10
    } else if profile.successful_uses == 0 && profile.cumulative_validation_duration_ms > 300_000 {
        5
    } else {
        0
    };
    success_signal
        .saturating_add(transfer_signal)
        .saturating_sub(rollback_penalty)
        .saturating_sub(validation_cost_penalty)
}

pub fn invoke_improvement_operator_repository(
    memory: &ImprovementOperatorMemory,
    weakness_evidence_kind: WeaknessEvidenceKind,
    transformation: &str,
    solution_strategy: &str,
    program: &StructuralRepairProgram,
    opportunity_family_id: &str,
) -> Result<ImprovementOperatorInvocation, String> {
    let requested = improvement_operator_ir_for_program(
        weakness_evidence_kind,
        transformation,
        solution_strategy,
        program,
    )?;
    let matching = memory
        .profiles
        .iter()
        .filter(|profile| {
            profile.successful_uses > 0 && profile.operator.operator_id == requested.operator_id
        })
        .collect::<Vec<_>>();
    let prior_attempts = matching.iter().map(|profile| profile.attempts).sum();
    let prior_successful_uses = matching.iter().map(|profile| profile.successful_uses).sum();
    let cross_family_successes = matching
        .iter()
        .flat_map(|profile| profile.successful_family_ids.iter())
        .filter(|family| family.as_str() != opportunity_family_id)
        .collect::<BTreeSet<_>>()
        .len();
    let priority_adjustment = matching
        .iter()
        .map(|_| improvement_operator_transfer_priority(memory, &requested, opportunity_family_id))
        .max()
        .unwrap_or(0);
    Ok(ImprovementOperatorInvocation {
        schema: IMPROVEMENT_OPERATOR_MEMORY_SCHEMA.to_string(),
        matched_operator_ids: matching
            .iter()
            .map(|profile| profile.operator.operator_id.clone())
            .collect(),
        priority_adjustment,
        prior_attempts,
        prior_successful_uses,
        cross_family_successes,
        executable_generator_kind: matching
            .first()
            .map(|profile| profile.operator.generator_kind),
        applicability_satisfied: !matching.is_empty(),
    })
}

fn counterexample_from_receipt(
    request: &AutonomousSourcePatchRequest,
    receipt: &AutonomousSourcePatchReceipt,
) -> Option<ValidationCounterexampleIR> {
    if receipt.installed {
        return None;
    }
    let reason = receipt
        .failure_reason
        .as_deref()
        .unwrap_or("UNKNOWN_FAILURE");
    if source_patch_failure_is_transient(Some(reason)) {
        return None;
    }
    let (phase, command) = if reason == "FORMAT_CHECK_FAILED" {
        (ValidationPhase::Format, receipt.format_check.as_ref())
    } else if matches!(reason, "COMPILE_CHECK_FAILED" | "CLIPPY_CHECK_FAILED") {
        (ValidationPhase::Compile, receipt.compile_check.as_ref())
    } else if reason == "REGRESSION_VALIDATION_FAILED" {
        (
            ValidationPhase::PublicObservation,
            Some(&receipt.validation),
        )
    } else if reason == "RELEASE_BUILD_FAILED" {
        (
            ValidationPhase::ReleaseBuild,
            receipt.release_build.as_ref(),
        )
    } else {
        (ValidationPhase::Infrastructure, Some(&receipt.validation))
    };
    let diagnostic_sha256 = command
        .map(|value| value.output_sha256.as_str())
        .unwrap_or("");
    let diagnostic_tail = command
        .map(|value| value.diagnostic_tail.as_str())
        .unwrap_or("");
    Some(validation_counterexample(
        request.source_generation,
        phase,
        reason,
        diagnostic_sha256,
        diagnostic_tail,
        if request.solution_strategy.is_empty() {
            &request.transformation
        } else {
            &request.solution_strategy
        },
        &request.candidate_sha256,
    ))
}

fn prior_counterexamples(
    state_dir: &Path,
    relative_path: &Path,
    transformation: &str,
) -> Result<Vec<ValidationCounterexampleIR>, String> {
    if !transformation.starts_with("COMPILER_OBSERVATION") {
        let problem_id = repair_problem_id_for(relative_path, transformation);
        return Ok(load_repair_learning(state_dir, &problem_id)?
            .map(|record| {
                record
                    .attempts
                    .into_iter()
                    .filter(source_repair_attempt_is_causal)
                    .filter_map(|attempt| attempt.validation_counterexample)
                    .collect()
            })
            .unwrap_or_default());
    }
    let knowledge_root = state_dir.join("source_repair_knowledge");
    if !knowledge_root.exists() {
        return Ok(Vec::new());
    }
    let family = normalized_repair_transformation(transformation);
    let mut counterexamples = BTreeMap::new();
    let entries = fs::read_dir(&knowledge_root)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_LIST:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_ENTRY:{error}"))?;
    for entry in entries {
        if entry.path().extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }
        let bytes = fs::read(entry.path())
            .map_err(|error| format!("SOURCE_REPAIR_LEARNING_READ:{error}"))?;
        let record: SourceRepairLearningRecord = serde_json::from_slice(&bytes)
            .map_err(|error| format!("SOURCE_REPAIR_LEARNING_PARSE:{error}"))?;
        if record.relative_path != relative_path
            || normalized_repair_transformation(&record.transformation) != family
        {
            continue;
        }
        for counterexample in record
            .attempts
            .into_iter()
            .filter(source_repair_attempt_is_causal)
            .filter_map(|attempt| attempt.validation_counterexample)
        {
            counterexamples.insert(counterexample.counterexample_id.clone(), counterexample);
        }
    }
    Ok(counterexamples.into_values().collect())
}

#[allow(clippy::too_many_arguments)]
fn generalized_change_for_candidate(
    state_dir: &Path,
    source_generation: u64,
    relative_path: &Path,
    transformation: &str,
    solution_strategy: &str,
    predecessor_sha256: &str,
    candidate_sha256: &str,
    evidence_kind: WeaknessEvidenceKind,
    evidence_sha256: &str,
    observed_mechanism: &str,
    consequence_predictions: &[String],
    program: &StructuralRepairProgram,
) -> Result<GeneralizedChangeIR, String> {
    let prior = prior_counterexamples(state_dir, relative_path, transformation)?;
    let weakness = derive_dynamic_weakness(
        source_generation,
        relative_path,
        transformation,
        evidence_kind,
        evidence_sha256,
        observed_mechanism,
        consequence_predictions.to_vec(),
        prior
            .iter()
            .map(|counterexample| counterexample.counterexample_id.clone())
            .collect(),
    );
    synthesize_generalized_change(
        &weakness,
        solution_strategy,
        predecessor_sha256,
        candidate_sha256,
        program,
    )
}

fn record_source_repair_outcome(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
    receipt: &AutonomousSourcePatchReceipt,
) -> Result<SourceRepairLearningRecord, String> {
    if receipt.opportunity_kind != request.opportunity_kind
        || receipt.opportunity_family_id != request.opportunity_family_id
    {
        return Err("SOURCE_REPAIR_RECEIPT_OPPORTUNITY_BINDING_MISMATCH".to_string());
    }
    let problem_id = repair_problem_id(request);
    let mut record = load_repair_learning(state_dir, &problem_id)?.unwrap_or_else(|| {
        SourceRepairLearningRecord {
            schema: SOURCE_REPAIR_LEARNING_SCHEMA.to_string(),
            problem_id: problem_id.clone(),
            relative_path: request.relative_path.clone(),
            transformation: request.transformation.clone(),
            status: "RETRYING".to_string(),
            cycle_started_generation: request.source_generation,
            cycle_started_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
            cycle_attempt_start_index: 0,
            eligible_after_generation: None,
            attempts: Vec::new(),
            learned_success: None,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
        }
    });
    if record.opportunity_family_id.is_empty() {
        record.opportunity_kind = request.opportunity_kind;
        record.opportunity_family_id = request.opportunity_family_id.clone();
    }
    if record.opportunity_kind != request.opportunity_kind
        || record.opportunity_family_id != request.opportunity_family_id
    {
        return Err("SOURCE_REPAIR_OPPORTUNITY_BINDING_MISMATCH".to_string());
    }
    let starts_new_successor_cycle = record.status == "LEARNED_SUCCESS"
        && (record.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION
            || record.learned_success.as_ref().is_none_or(|learned| {
                learned.predecessor_sha256 != request.predecessor_sha256
                    || learned.candidate_sha256 != request.candidate_sha256
            }));
    let reopens_failed_or_old_cycle = record.status != "LEARNED_SUCCESS"
        && ((record.status == "ADMITTED_FAILURE"
            && record
                .eligible_after_generation
                .is_some_and(|eligible| request.source_generation >= eligible))
            || record.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION);
    if starts_new_successor_cycle || reopens_failed_or_old_cycle {
        record.status = "RETRYING".to_string();
        record.cycle_started_generation = request.source_generation;
        record.cycle_started_engine_revision = SOURCE_REPAIR_ENGINE_REVISION;
        record.cycle_attempt_start_index = record.attempts.len();
        record.eligible_after_generation = None;
        record.learned_success = None;
    }
    let attempt_number = active_cycle_attempts(&record, request.source_generation)
        .len()
        .saturating_add(1)
        .min(u8::MAX as usize) as u8;
    let transient_failure = source_patch_failure_is_transient(receipt.failure_reason.as_deref());
    let solution_strategy = if request.solution_strategy.is_empty() {
        request.transformation.clone()
    } else {
        request.solution_strategy.clone()
    };
    let (structural_repair_program_sha256, edit_atom_kinds, structural_postcondition_count) =
        structural_program_learning_features(request)?;
    let (generalized_change_sha256, derived_from_counterexample_ids) =
        generalized_change_learning_features(request)?;
    let family_structural_repair_program_sha256 = request
        .additional_family_members
        .iter()
        .map(|member| {
            serde_json::to_vec(&member.structural_repair_program)
                .map(|bytes| sha256(&bytes))
                .map_err(|error| format!("SOURCE_REPAIR_FAMILY_PROGRAM_JSON:{error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let family_member_count = request.additional_family_members.len() + 1;
    let validation_counterexample = counterexample_from_receipt(request, receipt);
    let weakness_evidence_kind = request
        .generalized_change
        .as_ref()
        .map(|change| change.weakness_evidence_kind);
    let validation_duration_ms = source_patch_validation_critical_path_ms(receipt);
    let improvement_operator_execution_sha256 = request
        .improvement_operator_execution
        .as_ref()
        .map(|execution| {
            serde_json::to_vec(execution)
                .map(|bytes| sha256(&bytes))
                .map_err(|error| format!("IMPROVEMENT_OPERATOR_EXECUTION_JSON:{error}"))
        })
        .transpose()?;
    record.attempts.push(SourceRepairAttempt {
        attempt_number,
        source_generation: request.source_generation,
        source_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
        solution_strategy: solution_strategy.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        succeeded: receipt.installed,
        receipt_sha256: receipt.receipt_sha256.clone(),
        diagnostic_sha256: receipt.validation.output_sha256.clone(),
        failure_reason: receipt.failure_reason.clone(),
        structural_repair_program_sha256: structural_repair_program_sha256.clone(),
        edit_atom_kinds: edit_atom_kinds.clone(),
        structural_postcondition_count,
        validation_counterexample,
        generalized_change_sha256: generalized_change_sha256.clone(),
        derived_from_counterexample_ids: derived_from_counterexample_ids.clone(),
        family_member_count,
        family_structural_repair_program_sha256: family_structural_repair_program_sha256.clone(),
        opportunity_kind: request.opportunity_kind,
        opportunity_family_id: request.opportunity_family_id.clone(),
        weakness_evidence_kind,
        validation_duration_ms,
        invoked_operator_ids: request
            .improvement_operator_invocation
            .as_ref()
            .map(|invocation| invocation.matched_operator_ids.clone())
            .unwrap_or_default(),
        executed_operator_id: request
            .improvement_operator_execution
            .as_ref()
            .map(|execution| execution.operator_id.clone()),
        improvement_operator_execution_sha256,
        operator_priority_adjustment: request
            .improvement_operator_invocation
            .as_ref()
            .map(|invocation| invocation.priority_adjustment)
            .unwrap_or_default(),
        operator_cross_family_successes: request
            .improvement_operator_invocation
            .as_ref()
            .map(|invocation| invocation.cross_family_successes)
            .unwrap_or_default(),
    });
    if receipt.installed {
        record.status = "LEARNED_SUCCESS".to_string();
        record.eligible_after_generation = None;
        record.learned_success = Some(LearnedSuccessfulRepair {
            learned_at_generation: request.source_generation,
            solution_strategy,
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            validation_output_sha256: receipt.validation.output_sha256.clone(),
            release_build_output_sha256: receipt
                .release_build
                .as_ref()
                .map(|build| build.output_sha256.clone())
                .unwrap_or_default(),
            attempts_required: attempt_number,
            structural_repair_program_sha256,
            edit_atom_kinds,
            structural_postcondition_count,
            generalized_change_sha256,
            derived_from_counterexample_ids,
            family_member_count,
            family_structural_repair_program_sha256,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            weakness_evidence_kind,
            validation_duration_ms,
        });
    } else if transient_failure {
        // The candidate has not been falsified. Preserve the receipt for audit,
        // but do not consume a repair strategy or the bounded causal-attempt
        // budget merely because another process changed the worktree.
        record.status = "RETRYING".to_string();
        record.eligible_after_generation = None;
    } else if attempt_number >= policy.max_attempts_per_problem {
        record.status = "ADMITTED_FAILURE".to_string();
        record.eligible_after_generation = Some(request.source_generation.saturating_add(1));
        record.learned_success = None;
    } else {
        record.status = "RETRYING".to_string();
    }
    write_mutable_json(&repair_learning_path(state_dir, &problem_id), &record)?;
    Ok(record)
}

fn normalized_target(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err("SOURCE_MUTATION_RELATIVE_PATH_INVALID".to_string());
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("SOURCE_MUTATION_ROOT_CANONICALIZE:{error}"))?;
    let target = root.join(relative);
    let canonical_target = fs::canonicalize(&target)
        .map_err(|error| format!("SOURCE_MUTATION_TARGET_CANONICALIZE:{error}"))?;
    if !canonical_target.starts_with(&canonical_root) || !canonical_target.is_file() {
        return Err("SOURCE_MUTATION_TARGET_OUTSIDE_ROOT".to_string());
    }
    if fs::symlink_metadata(&canonical_target)
        .map_err(|error| format!("SOURCE_MUTATION_TARGET_METADATA:{error}"))?
        .file_type()
        .is_symlink()
    {
        return Err("SOURCE_MUTATION_SYMLINK_FORBIDDEN".to_string());
    }
    Ok(canonical_target)
}

fn structural_file_id(relative: &Path) -> String {
    relative.to_string_lossy().replace('\\', "/")
}

fn workspace_semantic_fingerprint_impl(
    root: &Path,
    excluded_target: Option<&Path>,
) -> Result<String, String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_ROOT:{error}"))?;
    let canonical_target = excluded_target
        .map(fs::canonicalize)
        .transpose()
        .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_TARGET:{error}"))?;
    let mut pending = vec![canonical_root.clone()];
    let mut entries = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)
            .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_READ_DIR:{error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_ENTRY:{error}"))?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let file_type = child
                .file_type()
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_TYPE:{error}"))?;
            let path = child.path();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if !excluded_directory(&path) {
                    pending.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&path)
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_CANONICAL:{error}"))?;
            let file_name = canonical.file_name().and_then(OsStr::to_str).unwrap_or("");
            if canonical_target.as_ref() == Some(&canonical)
                || file_name.contains(".bcore-rollback")
                || file_name.contains(".bcore-candidate")
            {
                continue;
            }
            let relative = canonical
                .strip_prefix(&canonical_root)
                .map_err(|_| "SOURCE_MUTATION_FINGERPRINT_OUTSIDE_ROOT".to_string())?;
            let bytes = fs::read(&canonical)
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_READ:{error}"))?;
            entries.push(format!(
                "{}:{}:{}",
                relative.display(),
                bytes.len(),
                sha256(&bytes)
            ));
        }
    }
    entries.sort();
    Ok(sha256(entries.join("\n").as_bytes()))
}

fn workspace_semantic_fingerprint(root: &Path, excluded_target: &Path) -> Result<String, String> {
    workspace_semantic_fingerprint_impl(root, Some(excluded_target))
}

pub(crate) fn full_workspace_semantic_fingerprint(root: &Path) -> Result<String, String> {
    workspace_semantic_fingerprint_impl(root, None)
}

pub(crate) fn runtime_core_feature_available(source_root: &Path) -> bool {
    [
        source_root.join("crates/semantic-reasoning/Cargo.toml"),
        source_root.join("Cargo.toml"),
    ]
    .into_iter()
    .filter_map(|path| fs::read_to_string(path).ok())
    .any(|manifest| {
        manifest.lines().any(|line| {
            let line = line.trim();
            line.starts_with("runtime-core") && line.contains('=')
        })
    })
}

pub(crate) fn runtime_core_relative_path(relative_path: &Path) -> bool {
    let path = relative_path.to_string_lossy().replace('\\', "/");
    let Some(source_path) = path.strip_prefix("crates/semantic-reasoning/src/") else {
        return false;
    };
    if source_path == "lib.rs"
        || matches!(
            source_path,
            "autonomous_self_inspection.rs"
                | "autonomous_source_mutation.rs"
                | "code_graft.rs"
                | "compiler_guided_repair.rs"
                | "fullstack_ops_knowledge.rs"
                | "generalized_self_application.rs"
                | "generated_sem5_capability.rs"
                | "generative_growth.rs"
                | "grammar_repair_synthesis.rs"
                | "growth_supervisor.rs"
                | "integrated_development.rs"
                | "self_healing_execution.rs"
                | "self_healing_pipeline.rs"
                | "self_repair_contract.rs"
                | "source_bound_causal_frontend.rs"
                | "source_bound_causal_main.rs"
                | "structural_source_repair.rs"
        )
    {
        return true;
    }
    matches!(
        source_path,
        "sem5/mod.rs"
            | "sem5/emitter.rs"
            | "sem5/ir.rs"
            | "sem5/learner.rs"
            | "sem5/model.rs"
            | "sem5/tasks.rs"
            | "sem20/engine.rs"
            | "sem21/engine.rs"
            | "sem22/engine.rs"
            | "sem23/engine.rs"
            | "sem24/engine.rs"
            | "sem25/engine.rs"
            | "sem26/engine.rs"
            | "sem27/engine.rs"
    )
}

fn request_targets_runtime_core(request: &AutonomousSourcePatchRequest) -> bool {
    runtime_core_relative_path(&request.relative_path)
        && request
            .additional_family_members
            .iter()
            .all(|member| runtime_core_relative_path(&member.relative_path))
}

fn package_name_from_manifest(manifest: &Path) -> Result<String, String> {
    let source = fs::read_to_string(manifest).map_err(|error| {
        format!(
            "SOURCE_MUTATION_MANIFEST_READ:{}:{error}",
            manifest.display()
        )
    })?;
    let mut in_package = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "name" {
            let name = value.trim().trim_matches(['\'', '"']);
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }
    Err(format!(
        "SOURCE_MUTATION_PACKAGE_NAME_MISSING:{}",
        manifest.display()
    ))
}

fn workspace_package_for_relative_path(
    source_root: &Path,
    relative_path: &Path,
) -> Result<String, String> {
    let canonical_root = fs::canonicalize(source_root)
        .map_err(|error| format!("SOURCE_MUTATION_PACKAGE_ROOT:{error}"))?;
    let target = normalized_target(source_root, relative_path)?;
    let mut directory = target
        .parent()
        .ok_or_else(|| "SOURCE_MUTATION_PACKAGE_PARENT_MISSING".to_string())?;
    loop {
        let manifest = directory.join("Cargo.toml");
        if manifest.is_file() {
            return package_name_from_manifest(&manifest);
        }
        if directory == canonical_root {
            break;
        }
        directory = directory
            .parent()
            .ok_or_else(|| "SOURCE_MUTATION_PACKAGE_OUTSIDE_ROOT".to_string())?;
        if !directory.starts_with(&canonical_root) {
            break;
        }
    }
    Err(format!(
        "SOURCE_MUTATION_PACKAGE_NOT_FOUND:{}",
        relative_path.display()
    ))
}

fn request_workspace_packages(
    source_root: &Path,
    request: &AutonomousSourcePatchRequest,
) -> Result<Vec<String>, String> {
    let mut packages = BTreeSet::new();
    packages.insert(workspace_package_for_relative_path(
        source_root,
        &request.relative_path,
    )?);
    for member in &request.additional_family_members {
        packages.insert(workspace_package_for_relative_path(
            source_root,
            &member.relative_path,
        )?);
    }
    Ok(packages.into_iter().collect())
}

fn append_package_selection<'a>(args: &mut Vec<&'a str>, packages: &'a [String]) {
    for package in packages {
        args.extend(["-p", package.as_str()]);
    }
}

fn append_runtime_core_feature_args(
    source_root: &Path,
    targets_runtime_core: bool,
    args: &mut Vec<&str>,
) {
    if targets_runtime_core && runtime_core_feature_available(source_root) {
        args.extend(["--no-default-features", "--features", "runtime-core"]);
    }
}

pub(crate) fn command_receipt(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    target_dir: &Path,
    timeout_ms: u64,
    diagnostic_path: &Path,
) -> Result<LocalCommandReceipt, String> {
    command_receipt_with_incremental_and_jobs(
        program,
        args,
        cwd,
        target_dir,
        timeout_ms,
        diagnostic_path,
        true,
        None,
    )
}

pub(crate) fn command_receipt_with_incremental(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    target_dir: &Path,
    timeout_ms: u64,
    diagnostic_path: &Path,
    cargo_incremental: bool,
) -> Result<LocalCommandReceipt, String> {
    command_receipt_with_incremental_and_jobs(
        program,
        args,
        cwd,
        target_dir,
        timeout_ms,
        diagnostic_path,
        cargo_incremental,
        None,
    )
}

fn command_receipt_with_cargo_jobs(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    target_dir: &Path,
    timeout_ms: u64,
    diagnostic_path: &Path,
    cargo_build_jobs: usize,
) -> Result<LocalCommandReceipt, String> {
    command_receipt_with_incremental_and_jobs(
        program,
        args,
        cwd,
        target_dir,
        timeout_ms,
        diagnostic_path,
        true,
        Some(cargo_build_jobs.max(1)),
    )
}

#[allow(clippy::too_many_arguments)]
fn command_receipt_with_incremental_and_jobs(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    target_dir: &Path,
    timeout_ms: u64,
    diagnostic_path: &Path,
    cargo_incremental: bool,
    cargo_build_jobs: Option<usize>,
) -> Result<LocalCommandReceipt, String> {
    let started = Instant::now();
    let diagnostic = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(diagnostic_path)
        .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_CREATE:{error}"))?;
    let diagnostic_error = diagnostic
        .try_clone()
        .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_CLONE:{error}"))?;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .env("CARGO_TARGET_DIR", target_dir)
        .env(
            "CARGO_INCREMENTAL",
            if cargo_incremental { "1" } else { "0" },
        )
        .env("CARGO_NET_OFFLINE", "true")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PIP_NO_INDEX", "1")
        .env("UV_OFFLINE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(diagnostic))
        .stderr(Stdio::from(diagnostic_error));
    if let Some(cargo_build_jobs) = cargo_build_jobs {
        command.env("CARGO_BUILD_JOBS", cargo_build_jobs.to_string());
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("SOURCE_MUTATION_COMMAND_SPAWN:{error}"))?;
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("SOURCE_MUTATION_COMMAND_WAIT:{error}"))?
        {
            let output = fs::read(diagnostic_path)
                .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_READ:{error}"))?;
            let tail_start = output.len().saturating_sub(4_096);
            return Ok(LocalCommandReceipt {
                program: program.display().to_string(),
                args: args.iter().map(|value| (*value).to_string()).collect(),
                cargo_incremental,
                exit_code: status.code(),
                success: status.success(),
                timed_out: false,
                duration_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                output_sha256: sha256(&output),
                diagnostic_tail: String::from_utf8_lossy(&output[tail_start..]).to_string(),
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let status = child.wait().ok();
            let output = fs::read(diagnostic_path)
                .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_READ:{error}"))?;
            let tail_start = output.len().saturating_sub(4_096);
            return Ok(LocalCommandReceipt {
                program: program.display().to_string(),
                args: args.iter().map(|value| (*value).to_string()).collect(),
                cargo_incremental,
                exit_code: status.and_then(|value| value.code()),
                success: false,
                timed_out: true,
                duration_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                output_sha256: sha256(&output),
                diagnostic_tail: String::from_utf8_lossy(&output[tail_start..]).to_string(),
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn join_command_lane(
    handle: thread::JoinHandle<Result<LocalCommandReceipt, String>>,
    lane: &str,
) -> Result<LocalCommandReceipt, String> {
    handle
        .join()
        .map_err(|_| format!("SOURCE_MUTATION_{lane}_LANE_PANICKED"))?
}

fn cargo_jobs_per_lane(active_lanes: usize) -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .checked_div(active_lanes.max(1))
        .unwrap_or(1)
        .max(1)
}

fn restore_target(target: &Path, rollback_sibling: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_file(target)
            .map_err(|error| format!("SOURCE_MUTATION_REMOVE_FAILED_TARGET:{error}"))?;
    }
    fs::rename(rollback_sibling, target)
        .map_err(|error| format!("SOURCE_MUTATION_ROLLBACK_RENAME:{error}"))
}

struct PreparedFamilyMember {
    target: PathBuf,
    rollback_sibling: PathBuf,
    candidate_sibling: PathBuf,
    predecessor: Vec<u8>,
}

fn request_weakness_evidence_kind(request: &AutonomousSourcePatchRequest) -> WeaknessEvidenceKind {
    request
        .generalized_change
        .as_ref()
        .map(|change| change.weakness_evidence_kind)
        .unwrap_or_else(|| inferred_weakness_evidence_kind(&request.transformation))
}

fn validate_improvement_operator_execution_binding(
    request: &AutonomousSourcePatchRequest,
    predecessor_source: &str,
) -> Result<(), String> {
    let Some(program) = &request.structural_repair_program else {
        if request.improvement_operator_invocation.is_some()
            || request.improvement_operator_execution.is_some()
        {
            return Err("IMPROVEMENT_OPERATOR_PROGRAM_MISSING".to_string());
        }
        return Ok(());
    };
    let invocation = request
        .improvement_operator_invocation
        .as_ref()
        .ok_or_else(|| "IMPROVEMENT_OPERATOR_INVOCATION_MISSING".to_string())?;
    let execution = request
        .improvement_operator_execution
        .as_ref()
        .ok_or_else(|| "IMPROVEMENT_OPERATOR_EXECUTION_MISSING".to_string())?;
    let strategy = if request.solution_strategy.is_empty() {
        request.transformation.as_str()
    } else {
        request.solution_strategy.as_str()
    };
    let expected = improvement_operator_ir_for_program(
        request_weakness_evidence_kind(request),
        &request.transformation,
        strategy,
        program,
    )?;
    if invocation.schema != IMPROVEMENT_OPERATOR_MEMORY_SCHEMA
        || invocation.applicability_satisfied == invocation.matched_operator_ids.is_empty()
        || invocation
            .matched_operator_ids
            .iter()
            .any(|operator_id| operator_id != &expected.operator_id)
        || (invocation.applicability_satisfied
            && invocation.executable_generator_kind != Some(expected.generator_kind))
    {
        return Err("IMPROVEMENT_OPERATOR_INVOCATION_BINDING_MISMATCH".to_string());
    }
    let observed =
        execute_improvement_operator_program_on_source(&expected, predecessor_source, program)?;
    if execution != &observed
        || !execution.applicable
        || execution.candidate_source.as_deref() != Some(request.candidate_source.as_str())
        || execution.operator_id != expected.operator_id
    {
        return Err("IMPROVEMENT_OPERATOR_EXECUTION_BINDING_MISMATCH".to_string());
    }
    Ok(())
}

fn prepare_family_members(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
) -> Result<Vec<PreparedFamilyMember>, String> {
    if request.additional_family_members.is_empty() {
        return Ok(Vec::new());
    }
    validate_policy(policy)?;
    if !policy.enabled
        || request.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA
        || !request.core_generated
        || !request.core_self_approved
        || request.patch_id.is_empty()
        || request.predicted_value < policy.minimum_predicted_value
        || request.predicted_value > 100
        || sha256(request.candidate_source.as_bytes()) != request.candidate_sha256
        || !opportunity_binding_valid(request)
    {
        return Err("SOURCE_MUTATION_REQUEST_INVALID".to_string());
    }
    if request.additional_family_members.len() + 1 > MAX_REPOSITORY_REPAIR_FAMILY_FILES {
        return Err("SOURCE_MUTATION_FAMILY_TOO_LARGE".to_string());
    }
    let total_candidate_bytes = request
        .additional_family_members
        .iter()
        .try_fold(request.candidate_source.len() as u64, |total, member| {
            total.checked_add(member.candidate_source.len() as u64)
        })
        .ok_or_else(|| "SOURCE_MUTATION_FAMILY_BYTES_OVERFLOW".to_string())?;
    if total_candidate_bytes > policy.max_candidate_bytes {
        return Err("SOURCE_MUTATION_FAMILY_BYTES_EXCEEDED".to_string());
    }

    let primary_target = normalized_target(&policy.source_root, &request.relative_path)?;
    let mut unique_targets = BTreeSet::from([primary_target]);
    let mut prepared = Vec::new();
    for (index, member) in request.additional_family_members.iter().enumerate() {
        if member.public_examples_observed == 0
            || member.public_examples_evaluated != member.public_examples_observed
            || member.public_examples_satisfied != member.public_examples_observed
            || sha256(member.candidate_source.as_bytes()) != member.candidate_sha256
        {
            return Err("SOURCE_MUTATION_FAMILY_MEMBER_EVIDENCE_INVALID".to_string());
        }
        let target = normalized_target(&policy.source_root, &member.relative_path)?;
        if !unique_targets.insert(target.clone()) {
            return Err("SOURCE_MUTATION_FAMILY_DUPLICATE_TARGET".to_string());
        }
        let predecessor = fs::read(&target)
            .map_err(|error| format!("SOURCE_MUTATION_FAMILY_PREDECESSOR_READ:{error}"))?;
        if sha256(&predecessor) != member.predecessor_sha256 {
            return Err("SOURCE_MUTATION_FAMILY_PREDECESSOR_MISMATCH".to_string());
        }
        let predecessor_source = std::str::from_utf8(&predecessor)
            .map_err(|_| "SOURCE_MUTATION_FAMILY_PREDECESSOR_NOT_UTF8".to_string())?;
        if member.structural_repair_program.file_id != structural_file_id(&member.relative_path) {
            return Err("SOURCE_MUTATION_FAMILY_FILE_ID_MISMATCH".to_string());
        }
        let execution =
            execute_structural_repair(&member.structural_repair_program, predecessor_source)
                .map_err(|error| format!("SOURCE_MUTATION_FAMILY_STRUCTURAL_REPLAY:{error}"))?;
        if !execution.structurally_verified
            || execution.candidate_source != member.candidate_source
            || execution.candidate_snapshot.source_sha256 != member.candidate_sha256
        {
            return Err("SOURCE_MUTATION_FAMILY_STRUCTURAL_REPLAY_MISMATCH".to_string());
        }
        let strategy = if request.solution_strategy.is_empty() {
            request.transformation.as_str()
        } else {
            request.solution_strategy.as_str()
        };
        let operator = improvement_operator_ir_for_program(
            request_weakness_evidence_kind(request),
            &request.transformation,
            strategy,
            &member.structural_repair_program,
        )?;
        let operator_execution = execute_improvement_operator_program_on_source(
            &operator,
            predecessor_source,
            &member.structural_repair_program,
        )?;
        if !operator_execution.applicable
            || operator_execution.candidate_source.as_deref()
                != Some(member.candidate_source.as_str())
        {
            return Err("SOURCE_MUTATION_FAMILY_OPERATOR_EXECUTION_MISMATCH".to_string());
        }
        let file_name = target
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("source");
        let rollback_sibling = target.with_file_name(format!(
            ".{file_name}.{}.family-{index}.bcore-rollback",
            request.patch_id
        ));
        let candidate_sibling = target.with_file_name(format!(
            ".{file_name}.{}.family-{index}.bcore-candidate",
            request.patch_id
        ));
        if rollback_sibling.exists() || candidate_sibling.exists() {
            return Err("SOURCE_MUTATION_FAMILY_SIBLING_COLLISION".to_string());
        }
        prepared.push(PreparedFamilyMember {
            target,
            rollback_sibling,
            candidate_sibling,
            predecessor,
        });
    }
    let mutation_root = state_dir.join("source_mutations").join(&request.patch_id);
    fs::create_dir_all(&mutation_root)
        .map_err(|error| format!("SOURCE_MUTATION_RECEIPT_DIR:{error}"))?;
    for (index, (prepared_member, request_member)) in prepared
        .iter()
        .zip(&request.additional_family_members)
        .enumerate()
    {
        let rollback_source = mutation_root.join(format!("family-{index}.predecessor.source"));
        if !rollback_source.exists() {
            write_new_file(&rollback_source, &prepared_member.predecessor)?;
        }
        if let Err(error) = write_new_file(
            &prepared_member.candidate_sibling,
            request_member.candidate_source.as_bytes(),
        ) {
            for prior in &prepared {
                if prior.candidate_sibling.exists() {
                    let _ = fs::remove_file(&prior.candidate_sibling);
                }
            }
            return Err(error);
        }
    }
    Ok(prepared)
}

fn restore_family_members(members: &[PreparedFamilyMember]) -> Result<(), String> {
    let mut failures = Vec::new();
    for member in members {
        if member.rollback_sibling.exists() {
            if let Err(error) = restore_target(&member.target, &member.rollback_sibling) {
                failures.push(error);
            }
        }
        if member.candidate_sibling.exists() {
            if let Err(error) = fs::remove_file(&member.candidate_sibling) {
                failures.push(format!("SOURCE_MUTATION_FAMILY_CANDIDATE_CLEANUP:{error}"));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(";"))
    }
}

fn activate_family_members(members: &[PreparedFamilyMember]) -> Result<(), String> {
    for member in members {
        if let Err(error) = fs::rename(&member.target, &member.rollback_sibling) {
            let restore = restore_family_members(members);
            return Err(format!(
                "SOURCE_MUTATION_FAMILY_PREDECESSOR_RENAME:{error}:RESTORE:{restore:?}"
            ));
        }
    }
    for member in members {
        if let Err(error) = fs::rename(&member.candidate_sibling, &member.target) {
            let restore = restore_family_members(members);
            return Err(format!(
                "SOURCE_MUTATION_FAMILY_CANDIDATE_RENAME:{error}:RESTORE:{restore:?}"
            ));
        }
    }
    Ok(())
}

fn receipt_hash(receipt: &AutonomousSourcePatchReceipt) -> Result<String, String> {
    let mut clone = receipt.clone();
    clone.receipt_sha256.clear();
    serde_json::to_vec(&clone)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("SOURCE_MUTATION_RECEIPT_JSON:{error}"))
}

#[derive(Debug)]
struct ConsumedRuntimeStagingGeneration {
    modified: SystemTime,
    mutation_id: String,
    staging: PathBuf,
    bytes: u64,
}

fn verified_consumed_runtime_staging(
    canonical_mutation_root: &Path,
    mutation: &Path,
) -> Result<Option<ConsumedRuntimeStagingGeneration>, String> {
    let mutation_type = fs::symlink_metadata(mutation)
        .map_err(|error| format!("SOURCE_STAGING_MUTATION_METADATA:{error}"))?
        .file_type();
    if mutation_type.is_symlink() || !mutation_type.is_dir() {
        return Err("SOURCE_STAGING_MUTATION_TYPE_INVALID".to_string());
    }
    let canonical_mutation = fs::canonicalize(mutation)
        .map_err(|error| format!("SOURCE_STAGING_MUTATION_CANONICALIZE:{error}"))?;
    if canonical_mutation.parent() != Some(canonical_mutation_root) {
        return Err("SOURCE_STAGING_MUTATION_OUTSIDE_ROOT".to_string());
    }
    let staging = canonical_mutation.join("staging");
    if !staging.exists() {
        return Ok(None);
    }
    let staging_type = fs::symlink_metadata(&staging)
        .map_err(|error| format!("SOURCE_STAGING_METADATA:{error}"))?
        .file_type();
    if staging_type.is_symlink() || !staging_type.is_dir() {
        return Err("SOURCE_STAGING_TYPE_INVALID".to_string());
    }
    let canonical_staging = fs::canonicalize(&staging)
        .map_err(|error| format!("SOURCE_STAGING_CANONICALIZE:{error}"))?;
    if canonical_staging.parent() != Some(canonical_mutation.as_path()) {
        return Err("SOURCE_STAGING_OUTSIDE_MUTATION".to_string());
    }
    let mutation_id = canonical_mutation
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "SOURCE_STAGING_MUTATION_ID_INVALID".to_string())?
        .to_string();

    let receipt_path = canonical_mutation.join("receipt.json");
    let receipt_bytes =
        fs::read(&receipt_path).map_err(|error| format!("SOURCE_STAGING_RECEIPT_READ:{error}"))?;
    let receipt: AutonomousSourcePatchReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("SOURCE_STAGING_RECEIPT_PARSE:{error}"))?;
    if receipt.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA
        || receipt.patch_id != mutation_id
        || !receipt.core_generated
        || !receipt.core_self_approved
        || !receipt.installed
        || receipt.rolled_back
        || receipt.failure_reason.is_some()
        || !receipt.validation.success
        || !receipt
            .release_build
            .as_ref()
            .is_some_and(|command| command.success)
        || !receipt.runtime_update_staged
        || !receipt.workspace_stable_during_validation
        || receipt.receipt_sha256 != receipt_hash(&receipt)?
    {
        return Err("SOURCE_STAGING_RECEIPT_NOT_CONSUMED_AUTHORITY".to_string());
    }

    let expected_names = BTreeSet::from([
        "b-core-growth-supervisor.exe".to_string(),
        "b-core-growth-verifier.exe".to_string(),
    ]);
    let mut observed_names = BTreeSet::new();
    let mut bytes = 0_u64;
    let entries = fs::read_dir(&canonical_staging)
        .map_err(|error| format!("SOURCE_STAGING_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_STAGING_ENTRY:{error}"))?;
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("SOURCE_STAGING_FILE_TYPE:{error}"))?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err("SOURCE_STAGING_NON_REGULAR_ARTIFACT".to_string());
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| "SOURCE_STAGING_NON_UTF8_ARTIFACT".to_string())?
            .to_string();
        observed_names.insert(name);
        bytes = bytes.saturating_add(
            entry
                .metadata()
                .map_err(|error| format!("SOURCE_STAGING_FILE_METADATA:{error}"))?
                .len(),
        );
    }
    if observed_names != expected_names {
        return Err("SOURCE_STAGING_ARTIFACT_SET_INVALID".to_string());
    }
    let modified = fs::metadata(&canonical_staging)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    Ok(Some(ConsumedRuntimeStagingGeneration {
        modified,
        mutation_id,
        staging: canonical_staging,
        bytes,
    }))
}

fn pending_runtime_staging(
    state_dir: &Path,
    canonical_mutation_root: &Path,
) -> Result<Option<PathBuf>, String> {
    let handoff_path = state_dir.join("control").join(SELF_UPDATE_HANDOFF_FILE);
    if !handoff_path.exists() {
        return Ok(None);
    }
    if fs::symlink_metadata(&handoff_path)
        .map_err(|error| format!("SOURCE_STAGING_HANDOFF_METADATA:{error}"))?
        .file_type()
        .is_symlink()
    {
        return Err("SOURCE_STAGING_HANDOFF_SYMLINK_FORBIDDEN".to_string());
    }
    let bytes =
        fs::read(&handoff_path).map_err(|error| format!("SOURCE_STAGING_HANDOFF_READ:{error}"))?;
    let handoff: RuntimeUpdateHandoff = serde_json::from_slice(&bytes)
        .map_err(|error| format!("SOURCE_STAGING_HANDOFF_PARSE:{error}"))?;
    if handoff.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA {
        return Err("SOURCE_STAGING_HANDOFF_SCHEMA_INVALID".to_string());
    }
    let supervisor = fs::canonicalize(&handoff.staged_supervisor)
        .map_err(|error| format!("SOURCE_STAGING_HANDOFF_SUPERVISOR:{error}"))?;
    let verifier = fs::canonicalize(&handoff.staged_verifier)
        .map_err(|error| format!("SOURCE_STAGING_HANDOFF_VERIFIER:{error}"))?;
    if supervisor.file_name().and_then(OsStr::to_str) != Some("b-core-growth-supervisor.exe")
        || verifier.file_name().and_then(OsStr::to_str) != Some("b-core-growth-verifier.exe")
        || supervisor.parent() != verifier.parent()
    {
        return Err("SOURCE_STAGING_HANDOFF_ARTIFACTS_INVALID".to_string());
    }
    let staging = supervisor
        .parent()
        .ok_or_else(|| "SOURCE_STAGING_HANDOFF_PARENT_MISSING".to_string())?
        .to_path_buf();
    if staging.file_name().and_then(OsStr::to_str) != Some("staging")
        || !staging.starts_with(canonical_mutation_root)
        || staging.parent().and_then(Path::parent) != Some(canonical_mutation_root)
    {
        return Err("SOURCE_STAGING_HANDOFF_OUTSIDE_ROOT".to_string());
    }
    let source_receipt = fs::canonicalize(&handoff.source_receipt)
        .map_err(|error| format!("SOURCE_STAGING_HANDOFF_RECEIPT:{error}"))?;
    if source_receipt != staging.parent().unwrap().join("receipt.json") {
        return Err("SOURCE_STAGING_HANDOFF_RECEIPT_BINDING_INVALID".to_string());
    }
    if handoff.patch_id
        != staging
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .unwrap_or("")
    {
        return Err("SOURCE_STAGING_HANDOFF_PATCH_BINDING_INVALID".to_string());
    }
    verified_consumed_runtime_staging(canonical_mutation_root, staging.parent().unwrap())?
        .ok_or_else(|| "SOURCE_STAGING_HANDOFF_GENERATION_MISSING".to_string())?;
    Ok(Some(staging))
}

/// Removes only verified, already-consumed runtime staging copies. Immutable
/// requests, receipts, predecessor sources, operator knowledge and any pending
/// update handoff remain untouched. At rest this retains the current and
/// immediate predecessor staging generations; while a handoff is pending it
/// retains that generation plus one verified predecessor.
pub fn cleanup_consumed_source_mutation_staging(
    state_dir: &Path,
) -> Result<SourceMutationStagingCleanup, String> {
    let mutation_root = state_dir.join("source_mutations");
    if !mutation_root.exists() {
        return Ok(SourceMutationStagingCleanup::default());
    }
    let root_type = fs::symlink_metadata(&mutation_root)
        .map_err(|error| format!("SOURCE_STAGING_ROOT_METADATA:{error}"))?
        .file_type();
    if root_type.is_symlink() || !root_type.is_dir() {
        return Err("SOURCE_STAGING_ROOT_INVALID".to_string());
    }
    let canonical_mutation_root = fs::canonicalize(&mutation_root)
        .map_err(|error| format!("SOURCE_STAGING_ROOT_CANONICALIZE:{error}"))?;
    let pending = pending_runtime_staging(state_dir, &canonical_mutation_root)?;
    let mut cleanup = SourceMutationStagingCleanup {
        pending_handoff_preserved: pending.is_some(),
        ..SourceMutationStagingCleanup::default()
    };
    let entries = fs::read_dir(&canonical_mutation_root)
        .map_err(|error| format!("SOURCE_STAGING_ROOT_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_STAGING_ROOT_ENTRY:{error}"))?;
    let mut generations = Vec::new();
    for entry in entries {
        if !entry.path().join("staging").exists() {
            continue;
        }
        match verified_consumed_runtime_staging(&canonical_mutation_root, &entry.path()) {
            Ok(Some(generation)) => generations.push(generation),
            Ok(None) => {}
            Err(_) => {
                cleanup.unverified_generations_skipped =
                    cleanup.unverified_generations_skipped.saturating_add(1);
            }
        }
    }
    generations.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.mutation_id.cmp(&right.mutation_id))
    });
    cleanup.consumed_generations_scanned = generations.len();
    let retained_consumed_limit = if pending.is_some() {
        1
    } else {
        MAX_RETAINED_CONSUMED_RUNTIME_STAGING_GENERATIONS
    };
    let mut consumed_retained = 0_usize;
    for generation in generations {
        if pending.as_ref() == Some(&generation.staging) {
            cleanup.generations_retained = cleanup.generations_retained.saturating_add(1);
            continue;
        }
        if consumed_retained < retained_consumed_limit {
            consumed_retained = consumed_retained.saturating_add(1);
            cleanup.generations_retained = cleanup.generations_retained.saturating_add(1);
            continue;
        }
        fs::remove_dir_all(&generation.staging).map_err(|error| {
            format!(
                "SOURCE_STAGING_REMOVE:{}:{error}",
                generation.staging.display()
            )
        })?;
        cleanup.generations_removed = cleanup.generations_removed.saturating_add(1);
        cleanup.bytes_removed = cleanup.bytes_removed.saturating_add(generation.bytes);
    }
    Ok(cleanup)
}

fn install_primary_and_stage_source_patch(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
) -> Result<AutonomousSourcePatchReceipt, String> {
    validate_policy(policy)?;
    if !policy.enabled
        || request.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA
        || !request.core_generated
        || !request.core_self_approved
        || request.patch_id.is_empty()
        || request.predicted_value < policy.minimum_predicted_value
        || request.predicted_value > 100
        || request.candidate_source.len() as u64 > policy.max_candidate_bytes
        || sha256(request.candidate_source.as_bytes()) != request.candidate_sha256
        || !opportunity_binding_valid(request)
    {
        return Err("SOURCE_MUTATION_REQUEST_INVALID".to_string());
    }
    validate_typed_mechanism_recipe_binding(request)?;
    let handoff_path = state_dir.join("control").join(SELF_UPDATE_HANDOFF_FILE);
    if handoff_path.exists() {
        return Err("SOURCE_UPDATE_ALREADY_STAGED".to_string());
    }
    let target = normalized_target(&policy.source_root, &request.relative_path)?;
    let target_packages = request_workspace_packages(&policy.source_root, request)?;
    let predecessor =
        fs::read(&target).map_err(|error| format!("SOURCE_MUTATION_PREDECESSOR_READ:{error}"))?;
    if sha256(&predecessor) != request.predecessor_sha256 {
        return Err("SOURCE_MUTATION_PREDECESSOR_MISMATCH".to_string());
    }
    let predecessor_source = std::str::from_utf8(&predecessor)
        .map_err(|_| "TYPED_MECHANISM_PREDECESSOR_NOT_UTF8".to_string())?;
    validate_typed_mechanism_source_materialization(request, predecessor_source)?;
    if let Some(program) = &request.structural_repair_program {
        if program.file_id != structural_file_id(&request.relative_path) {
            return Err("STRUCTURAL_REPAIR_FILE_ID_MISMATCH".to_string());
        }
        let execution = execute_structural_repair(program, predecessor_source)
            .map_err(|error| format!("STRUCTURAL_REPAIR_REPLAY:{error}"))?;
        if !execution.structurally_verified
            || execution.candidate_source != request.candidate_source
            || execution.candidate_snapshot.source_sha256 != request.candidate_sha256
        {
            return Err("STRUCTURAL_REPAIR_REPLAY_MISMATCH".to_string());
        }
    }
    validate_improvement_operator_execution_binding(request, predecessor_source)?;
    if let Some(change) = &request.generalized_change {
        let program = request
            .structural_repair_program
            .as_ref()
            .ok_or_else(|| "GENERALIZED_CHANGE_STRUCTURAL_PROGRAM_MISSING".to_string())?;
        validate_change_binding(
            change,
            &request.relative_path,
            &request.transformation,
            if request.solution_strategy.is_empty() {
                &request.transformation
            } else {
                &request.solution_strategy
            },
            &request.predecessor_sha256,
            &request.candidate_sha256,
            program,
        )?;
    }
    let workspace_fingerprint_before =
        workspace_semantic_fingerprint(&policy.source_root, &target)?;

    let mutation_root = state_dir.join("source_mutations").join(&request.patch_id);
    fs::create_dir_all(&mutation_root)
        .map_err(|error| format!("SOURCE_MUTATION_RECEIPT_DIR:{error}"))?;
    let request_path = mutation_root.join("request.json");
    if !request_path.exists() {
        write_immutable_json(&request_path, request)?;
    }
    let rollback_source = mutation_root.join("predecessor.source");
    if !rollback_source.exists() {
        write_new_file(&rollback_source, &predecessor)?;
    }
    let rollback_sibling = target.with_file_name(format!(
        ".{}.{}.bcore-rollback",
        target
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("source"),
        request.patch_id
    ));
    let candidate_sibling = target.with_file_name(format!(
        ".{}.{}.bcore-candidate",
        target
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("source"),
        request.patch_id
    ));
    if rollback_sibling.exists() || candidate_sibling.exists() {
        return Err("SOURCE_MUTATION_SIBLING_COLLISION".to_string());
    }
    write_new_file(&candidate_sibling, request.candidate_source.as_bytes())?;
    fs::rename(&target, &rollback_sibling)
        .map_err(|error| format!("SOURCE_MUTATION_PREDECESSOR_RENAME:{error}"))?;
    if let Err(error) = fs::rename(&candidate_sibling, &target) {
        let _ = fs::rename(&rollback_sibling, &target);
        return Err(format!("SOURCE_MUTATION_CANDIDATE_RENAME:{error}"));
    }

    let mut format_args = vec!["fmt"];
    append_package_selection(&mut format_args, &target_packages);
    format_args.extend(["--", "--check"]);
    let format_check = match command_receipt(
        &policy.cargo_executable,
        &format_args,
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("format-check.log"),
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !format_check.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed: false,
            rolled_back: true,
            failure_reason: Some("FORMAT_CHECK_FAILED".to_string()),
            format_check: Some(format_check.clone()),
            compile_check: None,
            validation: format_check,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    // Once the cheap formatting gate has passed, validation is a three-lane
    // DAG. Clippy, behavioral tests, and the runtime artifact build have no
    // semantic dependency on one another. Fixed lane target directories avoid
    // Cargo's global build-directory lock without creating per-attempt caches
    // or unbounded folders. Results are joined in a fixed order below; source
    // installation and rollback remain a single serial transaction.
    let validation_lane_jobs = cargo_jobs_per_lane(3);
    let release_target_dir = policy.build_target_dir.join("bcore-runtime-release-lane");
    let release_program = policy.cargo_executable.clone();
    let release_source_root = policy.source_root.clone();
    let release_timeout_ms = policy.validation_timeout_ms;
    let release_log = mutation_root.join("release-build.log");
    let release_lane_target = release_target_dir.clone();
    let release_lane_jobs = validation_lane_jobs;
    let mut release_args = vec!["build", "-p", "semantic-reasoning"];
    append_runtime_core_feature_args(&policy.source_root, true, &mut release_args);
    release_args.extend([
        "--release",
        "--bin",
        "b-core-growth-supervisor",
        "--bin",
        "b-core-growth-verifier",
    ]);
    let release_handle = thread::spawn(move || {
        command_receipt_with_cargo_jobs(
            &release_program,
            &release_args,
            &release_source_root,
            &release_lane_target,
            release_timeout_ms,
            &release_log,
            release_lane_jobs,
        )
    });

    let test_target_dir = policy.build_target_dir.join("bcore-source-test-lane");
    let test_program = policy.cargo_executable.clone();
    let test_source_root = policy.source_root.clone();
    let test_timeout_ms = policy.validation_timeout_ms;
    let test_log = mutation_root.join("test.log");
    let test_lane_target = test_target_dir.clone();
    let test_target_packages = target_packages.clone();
    let test_targets_runtime_core = request_targets_runtime_core(request);
    let test_lane_jobs = validation_lane_jobs;
    let validation_handle = thread::spawn(move || {
        let mut validation_args = vec!["test"];
        append_package_selection(&mut validation_args, &test_target_packages);
        validation_args.push("--lib");
        append_runtime_core_feature_args(
            &test_source_root,
            test_targets_runtime_core,
            &mut validation_args,
        );
        validation_args.push("--quiet");
        command_receipt_with_cargo_jobs(
            &test_program,
            &validation_args,
            &test_source_root,
            &test_lane_target,
            test_timeout_ms,
            &test_log,
            test_lane_jobs,
        )
    });

    // Clippy includes the compiler check and closes the gap that previously
    // allowed generated code with a known lint defect (for example an empty
    // else branch) to be installed and rediscovered as a new repair later.
    let mut compile_args = vec!["clippy"];
    append_package_selection(&mut compile_args, &target_packages);
    compile_args.push("--lib");
    append_runtime_core_feature_args(
        &policy.source_root,
        request_targets_runtime_core(request),
        &mut compile_args,
    );
    compile_args.extend(["--quiet", "--locked", "--", "-D", "warnings"]);
    let compile_check = match command_receipt_with_cargo_jobs(
        &policy.cargo_executable,
        &compile_args,
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("compile-check.log"),
        validation_lane_jobs,
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            let _ = join_command_lane(validation_handle, "REGRESSION_VALIDATION");
            let _ = join_command_lane(release_handle, "RUNTIME_RELEASE");
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !compile_check.success {
        let validation = join_command_lane(validation_handle, "REGRESSION_VALIDATION")
            .unwrap_or_else(|_| compile_check.clone());
        let release_build = join_command_lane(release_handle, "RUNTIME_RELEASE").ok();
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed: false,
            rolled_back: true,
            failure_reason: Some("CLIPPY_CHECK_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check.clone()),
            validation,
            release_build,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let validation = match join_command_lane(validation_handle, "REGRESSION_VALIDATION") {
        Ok(receipt) => receipt,
        Err(error) => {
            let _ = join_command_lane(release_handle, "RUNTIME_RELEASE");
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !validation.success {
        let release_build = join_command_lane(release_handle, "RUNTIME_RELEASE").ok();
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed: false,
            rolled_back: true,
            failure_reason: Some("REGRESSION_VALIDATION_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let release_build = match join_command_lane(release_handle, "RUNTIME_RELEASE") {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !release_build.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed: false,
            rolled_back: true,
            failure_reason: Some("RELEASE_BUILD_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build: Some(release_build),
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let workspace_fingerprint_after = workspace_semantic_fingerprint(&policy.source_root, &target)?;
    let target_still_exact_candidate = fs::read(&target)
        .map(|bytes| sha256(&bytes) == request.candidate_sha256)
        .unwrap_or(false);
    let workspace_stable_during_validation =
        workspace_fingerprint_before == workspace_fingerprint_after;
    if !workspace_stable_during_validation || !target_still_exact_candidate {
        restore_target(&target, &rollback_sibling)?;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed: false,
            rolled_back: true,
            failure_reason: Some(if target_still_exact_candidate {
                "CONCURRENT_WORKSPACE_CHANGE_DURING_VALIDATION".to_string()
            } else {
                "TARGET_CHANGED_DURING_VALIDATION".to_string()
            }),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build: Some(release_build),
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let built_supervisor = release_target_dir
        .join("release")
        .join("b-core-growth-supervisor.exe");
    let built_verifier = release_target_dir
        .join("release")
        .join("b-core-growth-verifier.exe");
    if !built_supervisor.is_file() || !built_verifier.is_file() {
        restore_target(&target, &rollback_sibling)?;
        return Err("SOURCE_MUTATION_RELEASE_ARTIFACT_MISSING".to_string());
    }
    let staging = mutation_root.join("staging");
    fs::create_dir_all(&staging).map_err(|error| format!("SOURCE_MUTATION_STAGING_DIR:{error}"))?;
    let staged_supervisor = staging.join("b-core-growth-supervisor.exe");
    let staged_verifier = staging.join("b-core-growth-verifier.exe");
    fs::copy(&built_supervisor, &staged_supervisor)
        .map_err(|error| format!("SOURCE_MUTATION_STAGE_SUPERVISOR:{error}"))?;
    fs::copy(&built_verifier, &staged_verifier)
        .map_err(|error| format!("SOURCE_MUTATION_STAGE_VERIFIER:{error}"))?;

    let receipt_path = mutation_root.join("receipt.json");
    let mut receipt = AutonomousSourcePatchReceipt {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id: request.patch_id.clone(),
        relative_path: request.relative_path.clone(),
        predecessor_sha256: request.predecessor_sha256.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        core_generated: true,
        core_self_approved: true,
        opportunity_kind: request.opportunity_kind,
        opportunity_family_id: request.opportunity_family_id.clone(),
        installed: true,
        rolled_back: false,
        failure_reason: None,
        format_check: Some(format_check),
        compile_check: Some(compile_check),
        validation,
        release_build: Some(release_build),
        runtime_update_staged: true,
        rollback_source,
        workspace_fingerprint_before,
        workspace_fingerprint_after,
        workspace_stable_during_validation,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_hash(&receipt)?;
    write_immutable_json(&receipt_path, &receipt)?;
    record_source_repair_outcome(policy, state_dir, request, &receipt)?;
    // Validation and installation are already committed at this point. A
    // transient cleanup failure must not be reclassified as a failed repair;
    // the authoritative predecessor copy remains in `rollback_source`.
    let _ = fs::remove_file(&rollback_sibling);

    fs::create_dir_all(&policy.runtime_bin_dir)
        .map_err(|error| format!("SOURCE_MUTATION_RUNTIME_DIR:{error}"))?;
    let handoff = RuntimeUpdateHandoff {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id: request.patch_id.clone(),
        staged_supervisor,
        staged_verifier,
        runtime_supervisor: policy.runtime_bin_dir.join("b-core-growth-supervisor.exe"),
        runtime_verifier: policy.runtime_bin_dir.join("b-core-growth-verifier.exe"),
        source_receipt: receipt_path,
    };
    write_immutable_json(&handoff_path, &handoff)?;
    // Promotion is deliberately after the durable source receipt and runtime
    // handoff. If a filesystem interruption occurs here, the installed repair
    // remains recoverable and the immutable request/receipt pair can be
    // reconciled on the next discovery cycle.
    persist_installed_typed_mechanism_operator(state_dir, request, &receipt)?;
    Ok(receipt)
}

pub fn install_and_stage_source_patch(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
) -> Result<AutonomousSourcePatchReceipt, String> {
    if state_dir
        .join("control")
        .join(SELF_UPDATE_HANDOFF_FILE)
        .exists()
    {
        return Err("SOURCE_UPDATE_ALREADY_STAGED".to_string());
    }
    let family_members = prepare_family_members(policy, state_dir, request)?;
    if family_members.is_empty() {
        return install_primary_and_stage_source_patch(policy, state_dir, request);
    }
    activate_family_members(&family_members)?;
    match install_primary_and_stage_source_patch(policy, state_dir, request) {
        Ok(receipt) if receipt.installed => {
            for member in &family_members {
                // Cleanup is outside the repair transaction's semantic commit
                // boundary. The durable predecessor copies in the mutation
                // root make any undeleted sibling redundant and recoverable.
                let _ = fs::remove_file(&member.rollback_sibling);
            }
            Ok(receipt)
        }
        Ok(receipt) => {
            restore_family_members(&family_members)?;
            Ok(receipt)
        }
        Err(error) => {
            restore_family_members(&family_members)
                .map_err(|restore| format!("{error}:SOURCE_MUTATION_FAMILY_RESTORE:{restore}"))?;
            Err(error)
        }
    }
}

fn excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            [".git", "target", "vendor", "reports", "artifacts"]
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
        })
}

fn rust_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("SOURCE_DISCOVERY_READ_DIR:{}:{error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("SOURCE_DISCOVERY_ENTRY:{error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("SOURCE_DISCOVERY_FILE_TYPE:{error}"))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() && !excluded_directory(&path) {
                pending.push(path);
            } else if file_type.is_file() && path.extension().and_then(OsStr::to_str) == Some("rs")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn expression_start(prefix: &str) -> Option<usize> {
    let bytes = prefix.as_bytes();
    let mut index = bytes.len();
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    let end = index;
    let mut depth = 0_i32;
    while index > 0 {
        index -= 1;
        match bytes[index] {
            b')' => depth += 1,
            b'(' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let previous = index.checked_sub(1).map(|value| bytes[value]);
                    if !previous.is_some_and(|value| {
                        value.is_ascii_alphanumeric() || matches!(value, b'_' | b'.')
                    }) {
                        return Some(index);
                    }
                }
            }
            b'(' if depth == 0 => return Some(index + 1),
            b'=' if depth == 0 => {
                return Some(if bytes.get(index + 1) == Some(&b'>') {
                    index + 2
                } else {
                    index + 1
                })
            }
            b';' | b',' | b'{' | b'[' | b'!' | b'&' | b'|' if depth == 0 => return Some(index + 1),
            _ => {}
        }
    }
    (end > 0).then_some(0)
}

fn rewrite_remainder_predicate(line: &str, strategy_index: usize) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with("#")
        || line.contains('"')
        || line.contains("assert")
    {
        return None;
    }
    let modulo = line.find(" % ")?;
    let right_start = modulo + 3;
    let tail = &line[right_start..];
    let (divisor, negated, comparison_len) = if let Some(position) = tail.find(" == 0") {
        (&tail[..position], false, position + 5)
    } else if let Some(position) = tail.find(" != 0") {
        (&tail[..position], true, position + 5)
    } else {
        return None;
    };
    if divisor.is_empty()
        || divisor.chars().any(char::is_whitespace)
        || !divisor
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    let left_boundary = expression_start(&line[..modulo])?;
    let leading_whitespace = line[left_boundary..modulo]
        .bytes()
        .take_while(u8::is_ascii_whitespace)
        .count();
    let left_start = left_boundary + leading_whitespace;
    let expression = line[left_start..modulo].trim();
    if expression.is_empty() {
        return None;
    }
    let positive = match strategy_index {
        0 => format!("{expression}.is_multiple_of({divisor})"),
        1 => format!("({expression}).is_multiple_of({divisor})"),
        2 => format!("matches!(({expression}).checked_rem({divisor}), Some(0))"),
        3 => format!("({expression}).rem_euclid({divisor}) == 0"),
        _ => return None,
    };
    let replacement = if negated {
        format!("!({positive})")
    } else {
        positive
    };
    let mut result = String::with_capacity(line.len() + 16);
    result.push_str(&line[..left_start]);
    result.push_str(&replacement);
    result.push_str(&line[right_start + comparison_len..]);
    Some(result)
}

fn rewrite_first_known_improvement(source: &str, strategy_index: usize) -> Option<String> {
    let mut output = String::with_capacity(source.len() + 32);
    let mut changed = false;
    let mut test_module_reached = false;
    for line in source.split_inclusive('\n') {
        if line.trim() == "#[cfg(test)]" {
            test_module_reached = true;
        }
        if !changed && !test_module_reached {
            if let Some(rewritten) = rewrite_remainder_predicate(line, strategy_index) {
                output.push_str(&rewritten);
                changed = true;
                continue;
            }
        }
        output.push_str(line);
    }
    changed.then_some(output)
}

fn repair_problem_id_for(relative_path: &Path, transformation: &str) -> String {
    sha256(
        format!(
            "{}:{}",
            relative_path.to_string_lossy().replace('\\', "/"),
            normalized_repair_transformation(transformation)
        )
        .as_bytes(),
    )
}

fn repair_strategy_is_available(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    relative_path: &Path,
    transformation: &str,
    solution_strategy: &str,
    source_artifact: (&str, &str),
    source_generation: u64,
) -> Result<bool, String> {
    let (predecessor_sha256, candidate_sha256) = source_artifact;
    let problem_id = repair_problem_id_for(relative_path, transformation);
    let record = load_repair_learning(state_dir, &problem_id)?;
    if record.as_ref().is_some_and(|knowledge| {
        (knowledge.status == "LEARNED_SUCCESS"
            && knowledge.cycle_started_engine_revision >= SOURCE_REPAIR_ENGINE_REVISION
            && knowledge.learned_success.as_ref().is_some_and(|success| {
                success.predecessor_sha256 == predecessor_sha256
                    || success.candidate_sha256 == candidate_sha256
            }))
            || (knowledge.status == "ADMITTED_FAILURE"
                && knowledge
                    .eligible_after_generation
                    .is_some_and(|eligible| source_generation < eligible)
                && knowledge.cycle_started_engine_revision >= SOURCE_REPAIR_ENGINE_REVISION)
    }) {
        return Ok(false);
    }
    let successor_candidate = record.as_ref().is_some_and(|knowledge| {
        knowledge.status == "LEARNED_SUCCESS"
            && (knowledge.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION
                || knowledge.learned_success.as_ref().is_some_and(|success| {
                    success.predecessor_sha256 != predecessor_sha256
                        && success.candidate_sha256 != candidate_sha256
                }))
    });
    let attempted = if successor_candidate {
        Vec::new()
    } else {
        record
            .as_ref()
            .map(|knowledge| active_cycle_attempts(knowledge, source_generation))
            .unwrap_or_default()
    };
    Ok(
        attempted.len() < usize::from(policy.max_attempts_per_problem)
            && !attempted
                .iter()
                .any(|attempt| attempt.solution_strategy == solution_strategy),
    )
}

fn compiler_opportunity_metadata(transformation: &str) -> (ChangeOpportunityKind, String) {
    let stable_family_basis = normalized_hash_suffixed_family(transformation);
    let kind = if stable_family_basis.contains(":clippy::") {
        ChangeOpportunityKind::RobustnessOpportunity
    } else {
        ChangeOpportunityKind::Defect
    };
    (
        kind,
        source_opportunity_family_id(kind, stable_family_basis),
    )
}

fn grammar_opportunity_metadata(
    transformation: &str,
    _repair_family: &str,
) -> (ChangeOpportunityKind, String) {
    let kind = if transformation.contains("PUBLIC_EXAMPLE_CONTRADICTED") {
        ChangeOpportunityKind::Defect
    } else {
        ChangeOpportunityKind::CapabilityGap
    };
    // Bind the family to the observed opportunity, not the current candidate
    // expression. Counterexample-guided revision may change the latter while
    // it is still solving the same hole or contradiction.
    let stable_family_basis = transformation
        .rsplit_once(':')
        .filter(|(_, suffix)| {
            suffix.len() >= 12 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .map(|(basis, _)| basis)
        .unwrap_or(transformation);
    (
        kind,
        source_opportunity_family_id(kind, stable_family_basis),
    )
}

fn compiler_guided_request(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
    operator_memory: &ImprovementOperatorMemory,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    if !policy.auto_discover_compiler_repairs {
        return Ok(None);
    }
    let diagnostic_policy = CompilerGuidedRepairPolicy {
        source_root: &policy.source_root,
        cargo_executable: &policy.cargo_executable,
        build_target_dir: &policy.build_target_dir,
        state_dir,
        timeout_ms: policy.validation_timeout_ms,
        max_candidate_bytes: policy.max_candidate_bytes,
    };
    let mut ranked = Vec::new();
    for candidate in discover_compiler_guided_repairs(&diagnostic_policy)? {
        let (_, opportunity_family_id) = compiler_opportunity_metadata(&candidate.transformation);
        let invocation = invoke_improvement_operator_repository(
            operator_memory,
            WeaknessEvidenceKind::CompilerDiagnostic,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.structural_repair_program,
            &opportunity_family_id,
        )?;
        ranked.push((invocation.priority_adjustment, invocation, candidate));
    }
    ranked.sort_by_key(|(priority, _, candidate)| {
        (
            std::cmp::Reverse(*priority),
            std::cmp::Reverse(candidate.predicted_value),
            candidate.relative_path.clone(),
            candidate.transformation.clone(),
            candidate.solution_strategy.clone(),
        )
    });
    for (_, invocation, candidate) in ranked {
        if candidate.predicted_value < policy.minimum_predicted_value
            || !repair_strategy_is_available(
                policy,
                state_dir,
                &candidate.relative_path,
                &candidate.transformation,
                &candidate.solution_strategy,
                (&candidate.predecessor_sha256, &candidate.candidate_sha256),
                source_generation,
            )?
        {
            continue;
        }
        let problem_id = repair_problem_id_for(&candidate.relative_path, &candidate.transformation);
        let patch_id = format!(
            "SELF-{}",
            &sha256(
                format!(
                    "{}:{}:{}:{}",
                    problem_id,
                    source_generation,
                    candidate.solution_strategy,
                    candidate.candidate_sha256
                )
                .as_bytes()
            )[..24]
        );
        if state_dir
            .join("source_mutations")
            .join(&patch_id)
            .join("receipt.json")
            .exists()
        {
            continue;
        }
        let generalized_change = generalized_change_for_candidate(
            state_dir,
            source_generation,
            &candidate.relative_path,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.predecessor_sha256,
            &candidate.candidate_sha256,
            WeaknessEvidenceKind::CompilerDiagnostic,
            &candidate.public_observation_sha256,
            "current compiler or clippy observation exposes a source-level weakness",
            &candidate.consequence_predictions,
            &candidate.structural_repair_program,
        )?;
        let (opportunity_kind, opportunity_family_id) =
            compiler_opportunity_metadata(&candidate.transformation);
        let predecessor_source =
            fs::read_to_string(policy.source_root.join(&candidate.relative_path))
                .map_err(|error| format!("COMPILER_REPAIR_PREDECESSOR_READ:{error}"))?;
        if sha256(predecessor_source.as_bytes()) != candidate.predecessor_sha256 {
            return Err("COMPILER_REPAIR_PREDECESSOR_DIVERGED".to_string());
        }
        let (bound_invocation, operator_execution) = invoke_and_execute_improvement_operator(
            operator_memory,
            WeaknessEvidenceKind::CompilerDiagnostic,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.structural_repair_program,
            &opportunity_family_id,
            &predecessor_source,
        )?;
        if bound_invocation != invocation
            || !operator_execution.applicable
            || operator_execution.candidate_source.as_deref()
                != Some(candidate.candidate_source.as_str())
        {
            return Err("COMPILER_REPAIR_OPERATOR_EXECUTION_DIVERGED".to_string());
        }
        return Ok(Some(AutonomousSourcePatchRequest {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id,
            relative_path: candidate.relative_path,
            predecessor_sha256: candidate.predecessor_sha256,
            candidate_source: candidate.candidate_source,
            candidate_sha256: candidate.candidate_sha256,
            transformation: candidate.transformation,
            consequence_predictions: candidate.consequence_predictions,
            predicted_value: candidate.predicted_value,
            source_generation,
            core_generated: true,
            core_self_approved: true,
            solution_strategy: candidate.solution_strategy,
            structural_repair_program: Some(candidate.structural_repair_program),
            generalized_change: Some(generalized_change),
            additional_family_members: Vec::new(),
            opportunity_kind,
            opportunity_family_id,
            improvement_operator_invocation: Some(invocation),
            improvement_operator_execution: Some(operator_execution),
            typed_mechanism_operator_recipe: None,
            typed_mechanism_synthesis_receipt: None,
            typed_mechanism_materialized_syntax_sha256: None,
            typed_mechanism_materialized_syntax_source: None,
            typed_mechanism_materialized_edit: None,
            typed_mechanism_selected_operator_id: None,
            typed_mechanism_candidates_enumerated: 0,
            typed_mechanism_preferred_operator_attempts: 0,
        }));
    }
    Ok(None)
}

fn public_example_priority(observed: usize, evaluated: usize, satisfied: usize) -> i32 {
    if observed == 0 {
        return 0;
    }
    if evaluated == 0 {
        return -25;
    }
    if evaluated == observed && satisfied == observed {
        return 200;
    }
    let supported = satisfied
        .saturating_mul(100)
        .checked_div(evaluated)
        .unwrap_or(0)
        .min(i32::MAX as usize) as i32;
    let unevaluated_penalty = observed
        .saturating_sub(evaluated)
        .saturating_mul(20)
        .min(i32::MAX as usize) as i32;
    supported.saturating_sub(unevaluated_penalty)
}

fn grammar_synthesized_request(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
    operator_memory: &ImprovementOperatorMemory,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    if !policy.auto_synthesize_grammar_repairs {
        return Ok(None);
    }
    reconcile_installed_typed_mechanism_operators(state_dir)?;
    let typed_operator_priors =
        load_authorized_typed_mechanism_operators(state_dir, MAX_ACTIVE_TYPED_MECHANISM_OPERATORS)?;
    let mut ranked = Vec::new();
    for candidate in discover_grammar_repairs_for_generation_with_priors(
        &policy.source_root,
        policy.max_candidate_bytes,
        source_generation,
        &typed_operator_priors,
    )? {
        let public_behavior_contradiction = candidate
            .transformation
            .contains("PUBLIC_EXAMPLE_CONTRADICTED_");
        let evidence_kind = if public_behavior_contradiction {
            WeaknessEvidenceKind::PublicBehaviorContradiction
        } else {
            WeaknessEvidenceKind::ExplicitCodeHole
        };
        let (_, opportunity_family_id) =
            grammar_opportunity_metadata(&candidate.transformation, &candidate.repair_family);
        let invocation = invoke_improvement_operator_repository(
            operator_memory,
            evidence_kind,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.structural_repair_program,
            &opportunity_family_id,
        )?;
        let counterexamples = prior_counterexamples(
            state_dir,
            &candidate.relative_path,
            &candidate.transformation,
        )?;
        let priority = i32::from(candidate.predicted_value)
            + feedback_priority(&candidate.solution_strategy, &counterexamples)
            + i32::try_from(candidate.family_member_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(25)
            + public_example_priority(
                candidate.public_examples_observed,
                candidate.public_examples_evaluated,
                candidate.public_examples_satisfied,
            )
            + invocation.priority_adjustment;
        ranked.push((priority, invocation, candidate));
    }
    ranked.sort_by_key(|(priority, _, candidate)| {
        (
            std::cmp::Reverse(*priority),
            candidate.relative_path.clone(),
            candidate.transformation.clone(),
            candidate.solution_strategy.clone(),
        )
    });
    for (_, invocation, candidate) in ranked {
        if candidate.predicted_value < policy.minimum_predicted_value
            || !repair_strategy_is_available(
                policy,
                state_dir,
                &candidate.relative_path,
                &candidate.transformation,
                &candidate.solution_strategy,
                (&candidate.predecessor_sha256, &candidate.candidate_sha256),
                source_generation,
            )?
        {
            continue;
        }
        let problem_id = repair_problem_id_for(&candidate.relative_path, &candidate.transformation);
        let patch_id = format!(
            "SELF-{}",
            &sha256(
                format!(
                    "{}:{}:{}:{}",
                    problem_id,
                    source_generation,
                    candidate.solution_strategy,
                    candidate.candidate_sha256
                )
                .as_bytes()
            )[..24]
        );
        if state_dir
            .join("source_mutations")
            .join(&patch_id)
            .join("receipt.json")
            .exists()
        {
            continue;
        }
        let evidence_sha256 = sha256(
            format!(
                "{}:{}:{}",
                candidate.relative_path.display(),
                candidate.transformation,
                candidate.predecessor_sha256
            )
            .as_bytes(),
        );
        let public_behavior_contradiction = candidate
            .transformation
            .contains("PUBLIC_EXAMPLE_CONTRADICTED_");
        let generalized_change = generalized_change_for_candidate(
            state_dir,
            source_generation,
            &candidate.relative_path,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.predecessor_sha256,
            &candidate.candidate_sha256,
            if public_behavior_contradiction {
                WeaknessEvidenceKind::PublicBehaviorContradiction
            } else {
                WeaknessEvidenceKind::ExplicitCodeHole
            },
            &evidence_sha256,
            if public_behavior_contradiction {
                "repository-visible public examples contradict the current typed implementation behavior"
            } else {
                "current Rust AST contains an executable todo or unimplemented hole"
            },
            &candidate.consequence_predictions,
            &candidate.structural_repair_program,
        )?;
        let additional_family_members = candidate
            .additional_family_files
            .iter()
            .map(|member| SourcePatchFamilyMember {
                relative_path: member.relative_path.clone(),
                predecessor_sha256: member.predecessor_sha256.clone(),
                candidate_source: member.candidate_source.clone(),
                candidate_sha256: member.candidate_sha256.clone(),
                structural_repair_program: member.structural_repair_program.clone(),
                public_examples_observed: member.public_examples_observed,
                public_examples_evaluated: member.public_examples_evaluated,
                public_examples_satisfied: member.public_examples_satisfied,
            })
            .collect();
        let (opportunity_kind, opportunity_family_id) =
            grammar_opportunity_metadata(&candidate.transformation, &candidate.repair_family);
        let evidence_kind = if public_behavior_contradiction {
            WeaknessEvidenceKind::PublicBehaviorContradiction
        } else {
            WeaknessEvidenceKind::ExplicitCodeHole
        };
        let predecessor_source =
            fs::read_to_string(policy.source_root.join(&candidate.relative_path))
                .map_err(|error| format!("GRAMMAR_REPAIR_PREDECESSOR_READ:{error}"))?;
        if sha256(predecessor_source.as_bytes()) != candidate.predecessor_sha256 {
            return Err("GRAMMAR_REPAIR_PREDECESSOR_DIVERGED".to_string());
        }
        let (bound_invocation, operator_execution) = invoke_and_execute_improvement_operator(
            operator_memory,
            evidence_kind,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.structural_repair_program,
            &opportunity_family_id,
            &predecessor_source,
        )?;
        if bound_invocation != invocation
            || !operator_execution.applicable
            || operator_execution.candidate_source.as_deref()
                != Some(candidate.candidate_source.as_str())
        {
            return Err("GRAMMAR_REPAIR_OPERATOR_EXECUTION_DIVERGED".to_string());
        }
        return Ok(Some(AutonomousSourcePatchRequest {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id,
            relative_path: candidate.relative_path,
            predecessor_sha256: candidate.predecessor_sha256,
            candidate_source: candidate.candidate_source,
            candidate_sha256: candidate.candidate_sha256,
            transformation: candidate.transformation,
            consequence_predictions: candidate.consequence_predictions,
            predicted_value: candidate.predicted_value,
            source_generation,
            core_generated: true,
            core_self_approved: true,
            solution_strategy: candidate.solution_strategy,
            structural_repair_program: Some(candidate.structural_repair_program),
            generalized_change: Some(generalized_change),
            additional_family_members,
            opportunity_kind,
            opportunity_family_id,
            improvement_operator_invocation: Some(invocation),
            improvement_operator_execution: Some(operator_execution),
            typed_mechanism_operator_recipe: candidate.typed_mechanism_operator_recipe,
            typed_mechanism_synthesis_receipt: candidate.typed_mechanism_synthesis_receipt,
            typed_mechanism_materialized_syntax_sha256: candidate.materialized_syntax_sha256,
            typed_mechanism_materialized_syntax_source: candidate.materialized_syntax_source,
            typed_mechanism_materialized_edit: candidate.typed_mechanism_materialized_edit,
            typed_mechanism_selected_operator_id: candidate.typed_mechanism_selected_operator_id,
            typed_mechanism_candidates_enumerated: candidate.typed_mechanism_candidates_enumerated,
            typed_mechanism_preferred_operator_attempts: candidate
                .typed_mechanism_preferred_operator_attempts,
        }));
    }
    Ok(None)
}

fn discover_source_improvement_lane(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
    operator_memory: &ImprovementOperatorMemory,
) -> Result<SourceDiscoveryResult, String> {
    // Search the bounded AST/public-example grammar before launching a Cargo
    // observation for a new source fingerprint. Grammar candidates need no
    // compiler process and still pass the exact same structural replay and
    // compile/public-regression installation gate.
    if let Some(candidate) =
        grammar_synthesized_request(policy, state_dir, source_generation, operator_memory)?
    {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Candidate,
            candidate: Some(candidate),
        });
    }
    if let Some(candidate) =
        compiler_guided_request(policy, state_dir, source_generation, operator_memory)?
    {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Candidate,
            candidate: Some(candidate),
        });
    }
    if !policy.auto_discover_known_transformations {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::NoApplicableTransformation,
            candidate: None,
        });
    }
    for path in rust_source_files(&policy.source_root)? {
        let bytes = fs::read(&path)
            .map_err(|error| format!("SOURCE_DISCOVERY_READ:{}:{error}", path.display()))?;
        if bytes.len() as u64 > policy.max_candidate_bytes {
            continue;
        }
        let Ok(source) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let relative_path = path
            .strip_prefix(&policy.source_root)
            .map_err(|_| "SOURCE_DISCOVERY_PATH_OUTSIDE_ROOT".to_string())?
            .to_path_buf();
        let predecessor_sha256 = sha256(&bytes);
        let transformation =
            "MANUAL_REMAINDER_PREDICATE_TO_TYPED_DIVISIBILITY_PREDICATE".to_string();
        let problem_id = repair_problem_id_for(&relative_path, &transformation);
        let record = load_repair_learning(state_dir, &problem_id)?;
        let attempted = record
            .as_ref()
            .map(|knowledge| active_cycle_attempts(knowledge, source_generation))
            .unwrap_or_default();
        if attempted.len() >= usize::from(policy.max_attempts_per_problem) {
            continue;
        }
        let opportunity_family_id = source_opportunity_family_id(
            ChangeOpportunityKind::EfficiencyOpportunity,
            &transformation,
        );
        let mut ranked_known_candidates = Vec::new();
        for (strategy_index, solution_strategy) in KNOWN_REMAINDER_STRATEGIES
            .iter()
            .enumerate()
            .take(usize::from(policy.max_attempts_per_problem))
        {
            let Some(candidate_source) = rewrite_first_known_improvement(source, strategy_index)
            else {
                continue;
            };
            let candidate_sha256 = sha256(candidate_source.as_bytes());
            if attempted
                .iter()
                .any(|attempt| attempt.solution_strategy == *solution_strategy)
                || !repair_strategy_is_available(
                    policy,
                    state_dir,
                    &relative_path,
                    &transformation,
                    solution_strategy,
                    (&predecessor_sha256, &candidate_sha256),
                    source_generation,
                )?
            {
                continue;
            }
            // A value gate is evidence about an applicable candidate, not a
            // substitute for applicability discovery. Returning early before
            // this point falsely reported a blocked improvement in every
            // repository, including those with no matching predicate.
            if KNOWN_REMAINDER_PREDICTED_VALUE < policy.minimum_predicted_value {
                return Ok(SourceDiscoveryResult {
                    disposition: SourceDiscoveryDisposition::BelowValueThreshold,
                    candidate: None,
                });
            }
            let structural_repair_program = match synthesize_structural_repair(
                &structural_file_id(&relative_path),
                source,
                &candidate_source,
            ) {
                Ok(program) => program,
                Err(_) => continue,
            };
            let patch_id = format!(
                "SELF-{}",
                &sha256(
                    format!(
                        "{}:{}:{}:{}",
                        problem_id, source_generation, solution_strategy, candidate_sha256
                    )
                    .as_bytes()
                )[..24]
            );
            if state_dir
                .join("source_mutations")
                .join(&patch_id)
                .join("receipt.json")
                .exists()
            {
                continue;
            }
            let (invocation, operator_execution) = invoke_and_execute_improvement_operator(
                operator_memory,
                WeaknessEvidenceKind::StructuralSourceSmell,
                &transformation,
                solution_strategy,
                &structural_repair_program,
                &opportunity_family_id,
                source,
            )?;
            if !operator_execution.applicable
                || operator_execution.candidate_source.as_deref() != Some(candidate_source.as_str())
            {
                return Err("IMPROVEMENT_OPERATOR_EXECUTION_DIVERGED".to_string());
            }
            ranked_known_candidates.push((
                invocation.priority_adjustment,
                strategy_index,
                (*solution_strategy).to_string(),
                candidate_source,
                structural_repair_program,
                candidate_sha256,
                patch_id,
                invocation,
                operator_execution,
            ));
        }
        ranked_known_candidates.sort_by_key(|(priority, strategy_index, ..)| {
            (std::cmp::Reverse(*priority), *strategy_index)
        });
        if let Some((
            _,
            _,
            solution_strategy,
            candidate_source,
            structural_repair_program,
            candidate_sha256,
            patch_id,
            invocation,
            operator_execution,
        )) = ranked_known_candidates.into_iter().next()
        {
            let consequence_predictions = vec![
                "preserve parity/divisibility semantics".to_string(),
                "replace a manual predicate using a distinct bounded repair strategy".to_string(),
                "retain only a method that passes format, regression, and release build gates"
                    .to_string(),
            ];
            let evidence_sha256 = sha256(
                format!(
                    "{}:{}:{}",
                    relative_path.display(),
                    transformation,
                    predecessor_sha256
                )
                .as_bytes(),
            );
            let generalized_change = generalized_change_for_candidate(
                state_dir,
                source_generation,
                &relative_path,
                &transformation,
                &solution_strategy,
                &predecessor_sha256,
                &candidate_sha256,
                WeaknessEvidenceKind::StructuralSourceSmell,
                &evidence_sha256,
                "current source contains a mechanically recognized redundant predicate form",
                &consequence_predictions,
                &structural_repair_program,
            )?;
            return Ok(SourceDiscoveryResult {
                disposition: SourceDiscoveryDisposition::Candidate,
                candidate: Some(AutonomousSourcePatchRequest {
                    schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
                    patch_id,
                    relative_path,
                    predecessor_sha256,
                    candidate_source,
                    candidate_sha256,
                    transformation,
                    consequence_predictions,
                    predicted_value: KNOWN_REMAINDER_PREDICTED_VALUE,
                    source_generation,
                    core_generated: true,
                    core_self_approved: true,
                    solution_strategy,
                    structural_repair_program: Some(structural_repair_program),
                    generalized_change: Some(generalized_change),
                    additional_family_members: Vec::new(),
                    opportunity_kind: ChangeOpportunityKind::EfficiencyOpportunity,
                    opportunity_family_id,
                    improvement_operator_invocation: Some(invocation),
                    improvement_operator_execution: Some(operator_execution),
                    typed_mechanism_operator_recipe: None,
                    typed_mechanism_synthesis_receipt: None,
                    typed_mechanism_materialized_syntax_sha256: None,
                    typed_mechanism_materialized_syntax_source: None,
                    typed_mechanism_materialized_edit: None,
                    typed_mechanism_selected_operator_id: None,
                    typed_mechanism_candidates_enumerated: 0,
                    typed_mechanism_preferred_operator_attempts: 0,
                }),
            });
        }
    }
    Ok(SourceDiscoveryResult {
        disposition: SourceDiscoveryDisposition::NoApplicableTransformation,
        candidate: None,
    })
}

fn source_discovery_lane_policy(
    policy: &AutonomousSourceMutationPolicy,
    known_transformations: bool,
    compiler_repairs: bool,
    grammar_repairs: bool,
) -> AutonomousSourceMutationPolicy {
    let mut lane = policy.clone();
    lane.auto_discover_known_transformations = known_transformations;
    lane.auto_discover_compiler_repairs = compiler_repairs;
    lane.auto_synthesize_grammar_repairs = grammar_repairs;
    lane
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceProposalOrigin {
    Grammar,
    Compiler,
    KnownTransformation,
}

impl SourceProposalOrigin {
    fn label(self) -> &'static str {
        match self {
            Self::Grammar => "GRAMMAR",
            Self::Compiler => "COMPILER",
            Self::KnownTransformation => "KNOWN_TRANSFORMATION",
        }
    }
}

#[derive(Debug)]
struct RankedSourceProposal {
    request: AutonomousSourcePatchRequest,
}

fn source_proposal_score(proposal: &RankedSourceProposal) -> i32 {
    i32::from(proposal.request.predicted_value)
        .saturating_add(
            proposal
                .request
                .improvement_operator_invocation
                .as_ref()
                .map_or(0, |invocation| invocation.priority_adjustment),
        )
        .saturating_add(
            i32::try_from(proposal.request.additional_family_members.len().min(8))
                .unwrap_or(0)
                .saturating_mul(2),
        )
}

fn validate_source_proposal_kernel_input(
    policy: &AutonomousSourceMutationPolicy,
    request: &AutonomousSourcePatchRequest,
) -> Result<String, String> {
    if request.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA
        || !request.core_generated
        || !request.core_self_approved
        || request.patch_id.is_empty()
        || request.predicted_value > 100
        || request.candidate_source.len() as u64 > policy.max_candidate_bytes
        || sha256(request.candidate_source.as_bytes()) != request.candidate_sha256
        || !opportunity_binding_valid(request)
    {
        return Err("SOURCE_PROPOSAL_ENVELOPE_INVALID".to_string());
    }
    let target = normalized_target(&policy.source_root, &request.relative_path)?;
    let predecessor = fs::read_to_string(&target)
        .map_err(|error| format!("SOURCE_PROPOSAL_PREDECESSOR_READ:{error}"))?;
    if sha256(predecessor.as_bytes()) != request.predecessor_sha256 {
        return Err("SOURCE_PROPOSAL_PREDECESSOR_MISMATCH".to_string());
    }
    let program = request
        .structural_repair_program
        .as_ref()
        .ok_or_else(|| "SOURCE_PROPOSAL_STRUCTURAL_PROGRAM_MISSING".to_string())?;
    if program.file_id != structural_file_id(&request.relative_path)
        || ![
            VerificationObligation::StructuralPostconditions,
            VerificationObligation::SourceCompile,
            VerificationObligation::PublicObservation,
        ]
        .iter()
        .all(|obligation| program.verification_obligations.contains(obligation))
    {
        return Err("SOURCE_PROPOSAL_OPERATION_CLOSURE_INCOMPLETE".to_string());
    }
    let replay = execute_structural_repair(program, &predecessor)
        .map_err(|error| format!("SOURCE_PROPOSAL_STRUCTURAL_REPLAY:{error}"))?;
    if !replay.structurally_verified
        || !replay.exact_target_observed
        || replay.candidate_source != request.candidate_source
        || replay.candidate_snapshot.source_sha256 != request.candidate_sha256
        || syn::parse_file(&request.candidate_source).is_err()
    {
        return Err("SOURCE_PROPOSAL_MATERIALIZATION_INVALID".to_string());
    }
    validate_typed_mechanism_recipe_binding(request)?;
    validate_typed_mechanism_source_materialization(request, &predecessor)?;
    validate_improvement_operator_execution_binding(request, &predecessor)?;
    if let Some(change) = &request.generalized_change {
        validate_change_binding(
            change,
            &request.relative_path,
            &request.transformation,
            if request.solution_strategy.is_empty() {
                &request.transformation
            } else {
                &request.solution_strategy
            },
            &request.predecessor_sha256,
            &request.candidate_sha256,
            program,
        )?;
    }
    Ok(predecessor)
}

fn proposal_is_atomic_composition_compatible(
    primary: &AutonomousSourcePatchRequest,
    candidate: &AutonomousSourcePatchRequest,
) -> bool {
    primary.relative_path == candidate.relative_path
        && primary.predecessor_sha256 == candidate.predecessor_sha256
        && primary.source_generation == candidate.source_generation
        && primary.additional_family_members.is_empty()
        && candidate.additional_family_members.is_empty()
}

fn compose_ranked_source_proposals(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
    operator_memory: &ImprovementOperatorMemory,
    ranked: &[RankedSourceProposal],
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    let Some(primary) = ranked.first() else {
        return Ok(None);
    };
    let predecessor = validate_source_proposal_kernel_input(policy, &primary.request)?;
    let primary_program = primary
        .request
        .structural_repair_program
        .as_ref()
        .ok_or_else(|| "SOURCE_PROPOSAL_STRUCTURAL_PROGRAM_MISSING".to_string())?;
    let mut selected = vec![&primary.request];
    let mut edits = vec![primary_program.edit.clone()];
    let mut combined_source = primary.request.candidate_source.clone();
    for proposal in ranked.iter().skip(1) {
        if !proposal_is_atomic_composition_compatible(&primary.request, &proposal.request) {
            continue;
        }
        let Some(program) = &proposal.request.structural_repair_program else {
            continue;
        };
        let mut trial_edits = edits.clone();
        trial_edits.push(program.edit.clone());
        let trial = SourceEditAtom::AtomicMultiEdit {
            edits: trial_edits.clone(),
        };
        let Ok(materialized) =
            crate::structural_source_repair::apply_edit_atom(&predecessor, &trial)
        else {
            continue;
        };
        if syn::parse_file(&materialized).is_err() {
            continue;
        }
        edits = trial_edits;
        combined_source = materialized;
        selected.push(&proposal.request);
    }
    if selected.len() < 2 {
        return Ok(None);
    }
    let selected_identity = selected
        .iter()
        .map(|request| request.patch_id.as_str())
        .collect::<Vec<_>>()
        .join(":");
    let identity_sha256 = sha256(selected_identity.as_bytes());
    let transformation = format!("COMPOSED_SOURCE_PROPOSAL_KERNEL:{}", &identity_sha256[..24]);
    let solution_strategy = format!("BOUND_ATOMIC_MULTI_EDIT:{}", &identity_sha256[..24]);
    let opportunity_kind = selected
        .iter()
        .map(|request| request.opportunity_kind)
        .min()
        .unwrap_or(primary.request.opportunity_kind);
    let opportunity_family_id = source_opportunity_family_id(opportunity_kind, &transformation);
    let candidate_sha256 = sha256(combined_source.as_bytes());
    let mut structural_repair_program = synthesize_structural_repair(
        &structural_file_id(&primary.request.relative_path),
        &predecessor,
        &combined_source,
    )?;
    // Preserve the proposal-level atomic decomposition even if a generic text
    // differ would represent the same postimage as one broad Replace. The
    // target-derived postconditions remain authoritative and the combined edit
    // is replayed immediately against the exact predecessor.
    structural_repair_program.edit = SourceEditAtom::AtomicMultiEdit { edits };
    let composite_replay = execute_structural_repair(&structural_repair_program, &predecessor)?;
    if !composite_replay.structurally_verified
        || !composite_replay.exact_target_observed
        || composite_replay.candidate_source != combined_source
    {
        return Err("SOURCE_PROPOSAL_COMPOSITE_SELF_FALSIFIED".to_string());
    }
    let (invocation, operator_execution) = invoke_and_execute_improvement_operator(
        operator_memory,
        request_weakness_evidence_kind(&primary.request),
        &transformation,
        &solution_strategy,
        &structural_repair_program,
        &opportunity_family_id,
        &predecessor,
    )?;
    if !operator_execution.applicable
        || operator_execution.candidate_source.as_deref() != Some(combined_source.as_str())
    {
        return Err("SOURCE_PROPOSAL_COMPOSITE_OPERATOR_DIVERGED".to_string());
    }
    let evidence_sha256 = sha256(
        format!(
            "{}:{}:{}",
            primary.request.relative_path.display(),
            primary.request.predecessor_sha256,
            identity_sha256
        )
        .as_bytes(),
    );
    let mut consequence_predictions = selected
        .iter()
        .flat_map(|request| request.consequence_predictions.iter().cloned())
        .collect::<BTreeSet<_>>();
    consequence_predictions
        .insert("non-conflicting source proposals install as one atomic transaction".to_string());
    let consequence_predictions = consequence_predictions.into_iter().collect::<Vec<_>>();
    let generalized_change = generalized_change_for_candidate(
        state_dir,
        source_generation,
        &primary.request.relative_path,
        &transformation,
        &solution_strategy,
        &primary.request.predecessor_sha256,
        &candidate_sha256,
        request_weakness_evidence_kind(&primary.request),
        &evidence_sha256,
        "multiple independently generated proposals bind to disjoint source operations",
        &consequence_predictions,
        &structural_repair_program,
    )?;
    let patch_id = format!(
        "SELF-COMPOSED-{}",
        &sha256(format!("{identity_sha256}:{source_generation}:{candidate_sha256}").as_bytes())
            [..24]
    );
    Ok(Some(AutonomousSourcePatchRequest {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id,
        relative_path: primary.request.relative_path.clone(),
        predecessor_sha256: primary.request.predecessor_sha256.clone(),
        candidate_source: combined_source,
        candidate_sha256,
        transformation,
        consequence_predictions,
        predicted_value: selected
            .iter()
            .map(|request| request.predicted_value)
            .max()
            .unwrap_or(0)
            .saturating_add(u16::try_from(selected.len().saturating_sub(1) * 5).unwrap_or(0))
            .min(100),
        source_generation,
        core_generated: true,
        core_self_approved: true,
        solution_strategy,
        structural_repair_program: Some(structural_repair_program),
        generalized_change: Some(generalized_change),
        additional_family_members: Vec::new(),
        opportunity_kind,
        opportunity_family_id,
        improvement_operator_invocation: Some(invocation),
        improvement_operator_execution: Some(operator_execution),
        typed_mechanism_operator_recipe: None,
        typed_mechanism_synthesis_receipt: None,
        typed_mechanism_materialized_syntax_sha256: None,
        typed_mechanism_materialized_syntax_source: None,
        typed_mechanism_materialized_edit: None,
        typed_mechanism_selected_operator_id: None,
        typed_mechanism_candidates_enumerated: 0,
        typed_mechanism_preferred_operator_attempts: 0,
    }))
}

fn select_source_discovery_proposals(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
    operator_memory: &ImprovementOperatorMemory,
    results: Vec<(SourceProposalOrigin, SourceDiscoveryResult)>,
) -> Result<SourceDiscoveryResult, String> {
    let below_value_threshold = results
        .iter()
        .any(|(_, result)| result.disposition == SourceDiscoveryDisposition::BelowValueThreshold);
    let mut ranked = Vec::new();
    let mut rejected = Vec::new();
    for (origin, result) in results {
        let Some(request) = result.candidate else {
            continue;
        };
        match validate_source_proposal_kernel_input(policy, &request) {
            Ok(_) => ranked.push(RankedSourceProposal { request }),
            Err(error) => rejected.push(format!("{}:{error}", origin.label())),
        }
    }
    ranked.sort_by_key(|proposal| {
        (
            std::cmp::Reverse(source_proposal_score(proposal)),
            std::cmp::Reverse(proposal.request.predicted_value),
            proposal.request.relative_path.clone(),
            proposal.request.transformation.clone(),
            proposal.request.patch_id.clone(),
            proposal.request.candidate_sha256.clone(),
        )
    });
    ranked.truncate(MAX_COMPETING_SOURCE_PROPOSALS);
    if ranked.is_empty() {
        if !rejected.is_empty() {
            return Err(format!(
                "SOURCE_PROPOSAL_KERNEL_ALL_REJECTED:{}",
                sha256(rejected.join(":").as_bytes())
            ));
        }
        return Ok(SourceDiscoveryResult {
            disposition: if below_value_threshold {
                SourceDiscoveryDisposition::BelowValueThreshold
            } else {
                SourceDiscoveryDisposition::NoApplicableTransformation
            },
            candidate: None,
        });
    }
    let candidate = compose_ranked_source_proposals(
        policy,
        state_dir,
        source_generation,
        operator_memory,
        &ranked,
    )?
    .or_else(|| ranked.into_iter().next().map(|proposal| proposal.request));
    Ok(SourceDiscoveryResult {
        disposition: SourceDiscoveryDisposition::Candidate,
        candidate,
    })
}

pub fn discover_known_source_improvement_detailed(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<SourceDiscoveryResult, String> {
    validate_policy(policy)?;
    if !policy.enabled
        || (!policy.auto_discover_known_transformations
            && !policy.auto_discover_compiler_repairs
            && !policy.auto_synthesize_grammar_repairs)
    {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Disabled,
            candidate: None,
        });
    }

    // Materialize the read-only operator snapshot before fan-out. Each lane
    // may then inspect the same repository without racing to create operator
    // files. Grammar synthesis, compiler observation, and known-transform
    // scanning have no result dependency and therefore form a bounded DAG.
    let operator_memory = refresh_improvement_operator_repository(state_dir)?;
    let grammar_policy =
        source_discovery_lane_policy(policy, false, false, policy.auto_synthesize_grammar_repairs);
    let compiler_policy =
        source_discovery_lane_policy(policy, false, policy.auto_discover_compiler_repairs, false);
    let known_policy = source_discovery_lane_policy(
        policy,
        policy.auto_discover_known_transformations,
        false,
        false,
    );

    let (grammar, compiler, known) = thread::scope(|scope| {
        let grammar_handle = policy.auto_synthesize_grammar_repairs.then(|| {
            scope.spawn(|| {
                discover_source_improvement_lane(
                    &grammar_policy,
                    state_dir,
                    source_generation,
                    &operator_memory,
                )
            })
        });
        let compiler_handle = policy.auto_discover_compiler_repairs.then(|| {
            scope.spawn(|| {
                discover_source_improvement_lane(
                    &compiler_policy,
                    state_dir,
                    source_generation,
                    &operator_memory,
                )
            })
        });
        let known_handle = policy.auto_discover_known_transformations.then(|| {
            scope.spawn(|| {
                discover_source_improvement_lane(
                    &known_policy,
                    state_dir,
                    source_generation,
                    &operator_memory,
                )
            })
        });
        let join = |handle: Option<
            thread::ScopedJoinHandle<'_, Result<SourceDiscoveryResult, String>>,
        >,
                    lane: &str|
         -> Result<Option<SourceDiscoveryResult>, String> {
            handle
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| format!("SOURCE_DISCOVERY_{lane}_LANE_PANICKED"))?
                })
                .transpose()
        };
        Ok::<_, String>((
            join(grammar_handle, "GRAMMAR")?,
            join(compiler_handle, "COMPILER")?,
            join(known_handle, "KNOWN_TRANSFORMATION")?,
        ))
    })?;

    // Every generator is proposal-only. A single typed kernel validates exact
    // source binding, operation closure, syntax, and structural replay before
    // it ranks at most three competitors. Disjoint edits for the same observed
    // opportunity may become one AtomicMultiEdit; installation and behavioral
    // authority remain in the unchanged verifier path.
    select_source_discovery_proposals(
        policy,
        state_dir,
        source_generation,
        &operator_memory,
        [
            grammar.map(|result| (SourceProposalOrigin::Grammar, result)),
            compiler.map(|result| (SourceProposalOrigin::Compiler, result)),
            known.map(|result| (SourceProposalOrigin::KnownTransformation, result)),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn discover_known_source_improvement(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    Ok(discover_known_source_improvement_detailed(policy, state_dir, source_generation)?.candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_core_path_classifier_never_fast_validates_historical_campaign_code() {
        for path in [
            "crates/semantic-reasoning/src/growth_supervisor.rs",
            "crates/semantic-reasoning/src/grammar_repair_synthesis.rs",
            "crates/semantic-reasoning/src/source_bound_causal_frontend.rs",
            "crates/semantic-reasoning/src/sem5/emitter.rs",
            "crates/semantic-reasoning/src/sem27/engine.rs",
        ] {
            assert!(runtime_core_relative_path(Path::new(path)), "{path}");
        }
        for path in [
            "crates/semantic-reasoning/src/sem12/mod.rs",
            "crates/semantic-reasoning/src/sem5/experiment.rs",
            "crates/semantic-reasoning/src/sem36/engine.rs",
            "research/sem27/frozen.json",
        ] {
            assert!(!runtime_core_relative_path(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn validation_package_is_resolved_from_the_changed_source_manifest() {
        let (root, _) = fixture("target-package-resolution");
        let nested = root.join("crates/worker");
        fs::create_dir_all(nested.join("src")).unwrap();
        fs::write(
            nested.join("Cargo.toml"),
            "[package]\nname = \"worker-core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(nested.join("src/lib.rs"), "pub fn work() {}\n").unwrap();

        assert_eq!(
            workspace_package_for_relative_path(&root, Path::new("src/lib.rs")).unwrap(),
            "semantic-reasoning"
        );
        assert_eq!(
            workspace_package_for_relative_path(&root, Path::new("crates/worker/src/lib.rs"))
                .unwrap(),
            "worker-core"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_validation_lanes_share_the_available_parallel_budget() {
        let available = thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        let per_lane = cargo_jobs_per_lane(3);
        assert!(per_lane >= 1);
        assert!(per_lane.saturating_mul(3) <= available.max(3));
    }

    fn synthetic_receipt(
        request: &AutonomousSourcePatchRequest,
        installed: bool,
    ) -> AutonomousSourcePatchReceipt {
        let output: &[u8] = if installed { b"pass" } else { b"failure" };
        let command = LocalCommandReceipt {
            program: "cargo".to_string(),
            args: vec!["test".to_string()],
            cargo_incremental: false,
            exit_code: Some(if installed { 0 } else { 101 }),
            success: installed,
            timed_out: false,
            duration_ms: 1,
            output_sha256: sha256(output),
            diagnostic_tail: String::from_utf8_lossy(output).to_string(),
        };
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: request.opportunity_kind,
            opportunity_family_id: request.opportunity_family_id.clone(),
            installed,
            rolled_back: !installed,
            failure_reason: (!installed).then(|| "SYNTHETIC_FAILURE".to_string()),
            format_check: Some(command.clone()),
            compile_check: Some(command.clone()),
            validation: command.clone(),
            release_build: installed.then_some(command),
            runtime_update_staged: installed,
            rollback_source: PathBuf::from("predecessor.source"),
            workspace_fingerprint_before: "a".repeat(64),
            workspace_fingerprint_after: "a".repeat(64),
            workspace_stable_during_validation: true,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt).unwrap();
        receipt
    }

    fn staged_synthetic_generation(
        state: &Path,
        request: &AutonomousSourcePatchRequest,
        mutation_id: &str,
    ) -> (PathBuf, PathBuf) {
        let mutation = state.join("source_mutations").join(mutation_id);
        let staging = mutation.join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("b-core-growth-supervisor.exe"), mutation_id).unwrap();
        fs::write(staging.join("b-core-growth-verifier.exe"), mutation_id).unwrap();
        let mut receipt = synthetic_receipt(request, true);
        receipt.patch_id = mutation_id.to_string();
        receipt.receipt_sha256 = receipt_hash(&receipt).unwrap();
        let receipt_path = mutation.join("receipt.json");
        write_immutable_json(&receipt_path, &receipt).unwrap();
        (staging, receipt_path)
    }

    #[test]
    fn consumed_runtime_staging_retains_only_current_and_predecessor() {
        let (root, policy) = fixture("bounded-runtime-staging");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 1)
            .unwrap()
            .expect("repair request");
        for index in 0..4 {
            staged_synthetic_generation(&state, &request, &format!("generation-{index}"));
        }

        let cleanup = cleanup_consumed_source_mutation_staging(&state).unwrap();
        assert_eq!(cleanup.consumed_generations_scanned, 4);
        assert_eq!(cleanup.generations_retained, 2);
        assert_eq!(cleanup.generations_removed, 2);
        assert!(cleanup.bytes_removed > 0);
        assert_eq!(cleanup.unverified_generations_skipped, 0);
        assert!(!cleanup.pending_handoff_preserved);
        let mutations = fs::read_dir(state.join("source_mutations"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            mutations
                .iter()
                .filter(|entry| entry.path().join("staging").is_dir())
                .count(),
            2
        );
        assert!(mutations
            .iter()
            .all(|entry| entry.path().join("receipt.json").is_file()));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn pending_runtime_handoff_and_one_verified_predecessor_are_preserved() {
        let (root, policy) = fixture("pending-runtime-staging");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 1)
            .unwrap()
            .expect("repair request");
        let mut staged = Vec::new();
        for index in 0..4 {
            staged.push(staged_synthetic_generation(
                &state,
                &request,
                &format!("pending-generation-{index}"),
            ));
        }
        let (pending_staging, pending_receipt) = &staged[0];
        fs::create_dir_all(state.join("control")).unwrap();
        let handoff = RuntimeUpdateHandoff {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: "pending-generation-0".to_string(),
            staged_supervisor: pending_staging.join("b-core-growth-supervisor.exe"),
            staged_verifier: pending_staging.join("b-core-growth-verifier.exe"),
            runtime_supervisor: policy.runtime_bin_dir.join("b-core-growth-supervisor.exe"),
            runtime_verifier: policy.runtime_bin_dir.join("b-core-growth-verifier.exe"),
            source_receipt: pending_receipt.clone(),
        };
        write_immutable_json(
            &state.join("control").join(SELF_UPDATE_HANDOFF_FILE),
            &handoff,
        )
        .unwrap();

        let cleanup = cleanup_consumed_source_mutation_staging(&state).unwrap();
        assert!(cleanup.pending_handoff_preserved);
        assert_eq!(cleanup.generations_retained, 2);
        assert_eq!(cleanup.generations_removed, 2);
        assert!(pending_staging.is_dir());
        assert!(pending_receipt.is_file());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn unverified_runtime_staging_is_never_deleted() {
        let (root, policy) = fixture("unverified-runtime-staging");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 1)
            .unwrap()
            .expect("repair request");
        for index in 0..3 {
            staged_synthetic_generation(&state, &request, &format!("verified-generation-{index}"));
        }
        let (unverified_staging, receipt_path) =
            staged_synthetic_generation(&state, &request, "unverified-generation");
        let mut receipt: AutonomousSourcePatchReceipt =
            serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt.patch_id = "different-generation".to_string();
        receipt.receipt_sha256 = receipt_hash(&receipt).unwrap();
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        fs::write(unverified_staging.join("operator-note.txt"), "preserve").unwrap();

        let cleanup = cleanup_consumed_source_mutation_staging(&state).unwrap();
        assert_eq!(cleanup.consumed_generations_scanned, 3);
        assert_eq!(cleanup.generations_removed, 1);
        assert_eq!(cleanup.unverified_generations_skipped, 1);
        assert!(unverified_staging.is_dir());
        assert!(unverified_staging.join("operator-note.txt").is_file());
        assert!(receipt_path.is_file());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    fn cargo_path() -> PathBuf {
        let candidate = std::env::var_os("CARGO")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cargo.exe"));
        if candidate.is_absolute() {
            candidate
        } else {
            std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .map(|home| home.join("bin").join(&candidate))
                .filter(|path| path.is_file())
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
                        .join(".cargo")
                        .join("bin")
                        .join(candidate)
                })
        }
    }

    fn fixture(label: &str) -> (PathBuf, AutonomousSourceMutationPolicy) {
        let root = std::env::temp_dir().join(format!(
            "b-core-source-mutation-{label}-{}-{}",
            std::process::id(),
            crate::self_repair_contract::sha256(label.as_bytes())
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='semantic-reasoning'\nversion='0.1.0'\nedition='2021'\n\n[lib]\npath='src/lib.rs'\n\n[[bin]]\nname='b-core-growth-supervisor'\npath='src/growth_supervisor_main.rs'\n\n[[bin]]\nname='b-core-growth-verifier'\npath='src/growth_verifier_main.rs'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"semantic-reasoning\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn even(value: u32) -> bool {\n    value % 2 == 0\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn even_works() {\n        assert!(super::even(2));\n    }\n}\n",
        )
        .unwrap();
        fs::write(root.join("src/growth_supervisor_main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("src/growth_verifier_main.rs"), "fn main() {}\n").unwrap();
        let policy = AutonomousSourceMutationPolicy {
            enabled: true,
            source_root: root.clone(),
            cargo_executable: cargo_path(),
            build_target_dir: root.join("target"),
            runtime_bin_dir: root.join("runtime"),
            auto_discover_known_transformations: true,
            auto_discover_compiler_repairs: false,
            auto_synthesize_grammar_repairs: false,
            max_candidate_bytes: 1024 * 1024,
            max_installations: 4,
            validation_timeout_ms: 120_000,
            max_attempts_per_problem: 4,
            minimum_predicted_value: 0,
        };
        (root, policy)
    }

    fn external_state(root: &Path) -> PathBuf {
        let state = root.with_file_name(format!(
            "{}-state",
            root.file_name().and_then(OsStr::to_str).unwrap_or("b-core")
        ));
        if state.exists() {
            fs::remove_dir_all(&state).unwrap();
        }
        state
    }

    fn proposal_request_fixture(
        policy: &AutonomousSourceMutationPolicy,
        state_dir: &Path,
        operator_memory: &ImprovementOperatorMemory,
        patch_id: &str,
        candidate_source: &str,
        predicted_value: u16,
        opportunity_family_id: &str,
    ) -> AutonomousSourcePatchRequest {
        let relative_path = PathBuf::from("src/lib.rs");
        let predecessor = fs::read_to_string(policy.source_root.join(&relative_path)).unwrap();
        let predecessor_sha256 = sha256(predecessor.as_bytes());
        let candidate_sha256 = sha256(candidate_source.as_bytes());
        let transformation = format!("PROPOSAL_FIXTURE_TRANSFORMATION:{patch_id}");
        let solution_strategy = format!("PROPOSAL_FIXTURE_STRATEGY:{patch_id}");
        let structural_repair_program = synthesize_structural_repair(
            &structural_file_id(&relative_path),
            &predecessor,
            candidate_source,
        )
        .unwrap();
        let (invocation, operator_execution) = invoke_and_execute_improvement_operator(
            operator_memory,
            WeaknessEvidenceKind::PublicBehaviorContradiction,
            &transformation,
            &solution_strategy,
            &structural_repair_program,
            opportunity_family_id,
            &predecessor,
        )
        .unwrap();
        let consequence_predictions = vec!["preserve unrelated public functions".to_string()];
        let generalized_change = generalized_change_for_candidate(
            state_dir,
            7,
            &relative_path,
            &transformation,
            &solution_strategy,
            &predecessor_sha256,
            &candidate_sha256,
            WeaknessEvidenceKind::PublicBehaviorContradiction,
            &sha256(format!("evidence:{patch_id}").as_bytes()),
            "a public observation contradicts the current expression",
            &consequence_predictions,
            &structural_repair_program,
        )
        .unwrap();
        AutonomousSourcePatchRequest {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: patch_id.to_string(),
            relative_path,
            predecessor_sha256,
            candidate_source: candidate_source.to_string(),
            candidate_sha256,
            transformation,
            consequence_predictions,
            predicted_value,
            source_generation: 7,
            core_generated: true,
            core_self_approved: true,
            solution_strategy,
            structural_repair_program: Some(structural_repair_program),
            generalized_change: Some(generalized_change),
            additional_family_members: Vec::new(),
            opportunity_kind: ChangeOpportunityKind::Defect,
            opportunity_family_id: opportunity_family_id.to_string(),
            improvement_operator_invocation: Some(invocation),
            improvement_operator_execution: Some(operator_execution),
            typed_mechanism_operator_recipe: None,
            typed_mechanism_synthesis_receipt: None,
            typed_mechanism_materialized_syntax_sha256: None,
            typed_mechanism_materialized_syntax_source: None,
            typed_mechanism_materialized_edit: None,
            typed_mechanism_selected_operator_id: None,
            typed_mechanism_candidates_enumerated: 0,
            typed_mechanism_preferred_operator_attempts: 0,
        }
    }

    fn rebind_request_operator_execution(
        request: &mut AutonomousSourcePatchRequest,
        state: &Path,
        predecessor_source: &str,
    ) {
        let program = request
            .structural_repair_program
            .as_ref()
            .expect("typed structural program");
        let memory = refresh_improvement_operator_repository(state).unwrap();
        let (invocation, execution) = invoke_and_execute_improvement_operator(
            &memory,
            request_weakness_evidence_kind(request),
            &request.transformation,
            &request.solution_strategy,
            program,
            &request.opportunity_family_id,
            predecessor_source,
        )
        .unwrap();
        request.improvement_operator_invocation = Some(invocation);
        request.improvement_operator_execution = Some(execution);
    }

    #[test]
    fn source_proposal_kernel_composes_disjoint_generator_edits_atomically() {
        let (root, policy) = fixture("proposal-kernel-compose");
        let predecessor = "pub fn left() -> i32 { 1 }\npub fn right() -> i32 { 10 }\n";
        fs::write(root.join("src/lib.rs"), predecessor).unwrap();
        let state = root.join("state");
        fs::create_dir_all(&state).unwrap();
        let operator_memory = refresh_improvement_operator_repository(&state).unwrap();
        let grammar_family =
            source_opportunity_family_id(ChangeOpportunityKind::Defect, "LEFT_CONTRACT");
        let compiler_family =
            source_opportunity_family_id(ChangeOpportunityKind::Defect, "RIGHT_CONTRACT");
        let conflicting_family =
            source_opportunity_family_id(ChangeOpportunityKind::Defect, "THIRD_CONTRACT");
        let grammar = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "GRAMMAR-LEFT",
            "pub fn left() -> i32 { 2 }\npub fn right() -> i32 { 10 }\n",
            90,
            &grammar_family,
        );
        let compiler = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "COMPILER-RIGHT",
            "pub fn left() -> i32 { 1 }\npub fn right() -> i32 { 20 }\n",
            85,
            &compiler_family,
        );
        let conflicting = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "KNOWN-CONFLICT",
            "pub fn left() -> i32 { 3 }\npub fn right() -> i32 { 10 }\n",
            80,
            &conflicting_family,
        );

        let selected = select_source_discovery_proposals(
            &policy,
            &state,
            7,
            &operator_memory,
            vec![
                (
                    SourceProposalOrigin::Grammar,
                    SourceDiscoveryResult {
                        disposition: SourceDiscoveryDisposition::Candidate,
                        candidate: Some(grammar),
                    },
                ),
                (
                    SourceProposalOrigin::Compiler,
                    SourceDiscoveryResult {
                        disposition: SourceDiscoveryDisposition::Candidate,
                        candidate: Some(compiler),
                    },
                ),
                (
                    SourceProposalOrigin::KnownTransformation,
                    SourceDiscoveryResult {
                        disposition: SourceDiscoveryDisposition::Candidate,
                        candidate: Some(conflicting),
                    },
                ),
            ],
        )
        .unwrap()
        .candidate
        .expect("composed proposal");

        assert!(selected
            .transformation
            .starts_with("COMPOSED_SOURCE_PROPOSAL_KERNEL:"));
        assert_ne!(selected.opportunity_family_id, grammar_family);
        assert_ne!(selected.opportunity_family_id, compiler_family);
        assert_eq!(
            selected.opportunity_family_id,
            source_opportunity_family_id(selected.opportunity_kind, &selected.transformation)
        );
        assert_eq!(
            selected.candidate_source,
            "pub fn left() -> i32 { 2 }\npub fn right() -> i32 { 20 }\n"
        );
        assert!(matches!(
            selected
                .structural_repair_program
                .as_ref()
                .expect("composite structural program")
                .edit,
            SourceEditAtom::AtomicMultiEdit { ref edits } if edits.len() == 2
        ));
        validate_source_proposal_kernel_input(&policy, &selected).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_proposal_kernel_bounds_competitors_and_rejects_conflicting_composition() {
        let (root, policy) = fixture("proposal-kernel-bound");
        let predecessor = "pub fn first() -> i32 { 1 }\npub fn second() -> i32 { 2 }\n";
        fs::write(root.join("src/lib.rs"), predecessor).unwrap();
        let state = root.join("state");
        fs::create_dir_all(&state).unwrap();
        let operator_memory = refresh_improvement_operator_repository(&state).unwrap();
        let family = source_opportunity_family_id(ChangeOpportunityKind::Defect, "BOUND_CONTRACT");
        let candidates = [("TOP", 4, 95), ("CONFLICT-A", 5, 90), ("CONFLICT-B", 6, 85)]
            .into_iter()
            .map(|(id, value, score)| {
                proposal_request_fixture(
                    &policy,
                    &state,
                    &operator_memory,
                    id,
                    &format!(
                        "pub fn first() -> i32 {{ {value} }}\npub fn second() -> i32 {{ 2 }}\n"
                    ),
                    score,
                    &family,
                )
            })
            .collect::<Vec<_>>();
        let fourth_disjoint = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "FOURTH-DISJOINT",
            "pub fn first() -> i32 { 1 }\npub fn second() -> i32 { 9 }\n",
            10,
            &family,
        );
        let mut results = candidates
            .into_iter()
            .map(|candidate| {
                (
                    SourceProposalOrigin::Grammar,
                    SourceDiscoveryResult {
                        disposition: SourceDiscoveryDisposition::Candidate,
                        candidate: Some(candidate),
                    },
                )
            })
            .collect::<Vec<_>>();
        results.push((
            SourceProposalOrigin::Compiler,
            SourceDiscoveryResult {
                disposition: SourceDiscoveryDisposition::Candidate,
                candidate: Some(fourth_disjoint),
            },
        ));

        let selected =
            select_source_discovery_proposals(&policy, &state, 7, &operator_memory, results)
                .unwrap()
                .candidate
                .expect("highest-ranked proposal");

        assert_eq!(selected.patch_id, "TOP");
        assert_eq!(
            selected.candidate_source,
            "pub fn first() -> i32 { 4 }\npub fn second() -> i32 { 2 }\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_proposal_kernel_ties_are_generator_origin_neutral() {
        let (root, policy) = fixture("proposal-kernel-origin-neutral");
        let predecessor = "pub fn value() -> i32 { 1 }\n";
        fs::write(root.join("src/lib.rs"), predecessor).unwrap();
        let state = root.join("state");
        fs::create_dir_all(&state).unwrap();
        let operator_memory = refresh_improvement_operator_repository(&state).unwrap();
        let family = source_opportunity_family_id(ChangeOpportunityKind::Defect, "TIED_CONTRACT");
        let candidate_a = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "A-CANDIDATE",
            "pub fn value() -> i32 { 2 }\n",
            90,
            &family,
        );
        let candidate_z = proposal_request_fixture(
            &policy,
            &state,
            &operator_memory,
            "Z-CANDIDATE",
            "pub fn value() -> i32 { 3 }\n",
            90,
            &family,
        );
        let ranked_a = RankedSourceProposal {
            request: candidate_a.clone(),
        };
        let ranked_z = RankedSourceProposal {
            request: candidate_z.clone(),
        };
        assert_eq!(
            source_proposal_score(&ranked_a),
            source_proposal_score(&ranked_z)
        );

        let select = |results| {
            select_source_discovery_proposals(&policy, &state, 7, &operator_memory, results)
                .unwrap()
                .candidate
                .expect("tied candidate")
                .patch_id
        };
        let first = select(vec![
            (
                SourceProposalOrigin::KnownTransformation,
                SourceDiscoveryResult {
                    disposition: SourceDiscoveryDisposition::Candidate,
                    candidate: Some(candidate_a.clone()),
                },
            ),
            (
                SourceProposalOrigin::Grammar,
                SourceDiscoveryResult {
                    disposition: SourceDiscoveryDisposition::Candidate,
                    candidate: Some(candidate_z.clone()),
                },
            ),
        ]);
        let second = select(vec![
            (
                SourceProposalOrigin::Compiler,
                SourceDiscoveryResult {
                    disposition: SourceDiscoveryDisposition::Candidate,
                    candidate: Some(candidate_z),
                },
            ),
            (
                SourceProposalOrigin::Grammar,
                SourceDiscoveryResult {
                    disposition: SourceDiscoveryDisposition::Candidate,
                    candidate: Some(candidate_a),
                },
            ),
        ]);
        assert_eq!(first, "A-CANDIDATE");
        assert_eq!(second, first);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parallel_validation_duration_tracks_the_critical_path() {
        let command = |program: &str, duration_ms: u64| LocalCommandReceipt {
            program: program.to_string(),
            args: Vec::new(),
            cargo_incremental: true,
            exit_code: Some(0),
            success: true,
            timed_out: false,
            duration_ms,
            output_sha256: program.to_string(),
            diagnostic_tail: String::new(),
        };
        let receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: "parallel-duration".to_string(),
            relative_path: PathBuf::from("src/lib.rs"),
            predecessor_sha256: "predecessor".to_string(),
            candidate_sha256: "candidate".to_string(),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: ChangeOpportunityKind::EfficiencyOpportunity,
            opportunity_family_id: "family".to_string(),
            installed: true,
            rolled_back: false,
            failure_reason: None,
            format_check: Some(command("fmt", 10)),
            compile_check: Some(command("clippy", 100)),
            validation: command("test", 200),
            release_build: Some(command("release", 500)),
            runtime_update_staged: true,
            rollback_source: PathBuf::from("rollback.rs"),
            workspace_fingerprint_before: "stable".to_string(),
            workspace_fingerprint_after: "stable".to_string(),
            workspace_stable_during_validation: true,
            receipt_sha256: "receipt".to_string(),
        };

        assert_eq!(source_patch_validation_critical_path_ms(&receipt), 510);
    }

    #[test]
    fn known_improvement_is_predicted_without_touching_tests_or_strings() {
        let source = "pub fn even(value: u32) -> bool { value % 2 == 0 }\n#[cfg(test)]\nmod tests { const TEXT: &str = \"x % 2 == 0\"; }\n";
        let rewritten = rewrite_first_known_improvement(source, 0).expect("candidate");
        assert!(rewritten.contains("value.is_multiple_of(2)"));
        assert!(rewritten.contains("\"x % 2 == 0\""));

        let conditional = "let scope = if ordinal % 5 == 0 { 1 } else { 2 };\n";
        let rewritten = rewrite_first_known_improvement(conditional, 0).expect("conditional");
        assert!(rewritten.contains("= if ordinal.is_multiple_of(5)"));

        let match_arm = "            Self::Even => value % 2 == 0,\n";
        for strategy in 0..KNOWN_REMAINDER_STRATEGIES.len() {
            let rewritten =
                rewrite_first_known_improvement(match_arm, strategy).expect("match arm");
            assert!(rewritten.contains("Self::Even =>"));
            assert!(!rewritten.contains("=(>"));
            assert!(!rewritten.contains("=matches"));
        }
    }

    #[test]
    fn traversal_and_absolute_targets_are_rejected() {
        let root = std::env::temp_dir();
        assert!(normalized_target(&root, Path::new("..\\escape.rs")).is_err());
        assert!(normalized_target(&root, Path::new("C:\\escape.rs")).is_err());
    }

    #[test]
    fn opportunity_family_identity_is_stable_across_candidate_attempts() {
        let (first_kind, first_family) = compiler_opportunity_metadata(
            "COMPILER_OBSERVATION:clippy::needless_else:aaaaaaaaaaaa",
        );
        let (second_kind, second_family) = compiler_opportunity_metadata(
            "COMPILER_OBSERVATION:clippy::needless_else:bbbbbbbbbbbb",
        );
        assert_eq!(first_kind, ChangeOpportunityKind::RobustnessOpportunity);
        assert_eq!(first_kind, second_kind);
        assert_eq!(first_family, second_family);

        let (hole_kind, hole_family) =
            grammar_opportunity_metadata("AST_GRAMMAR_HOLE:TODO:1", "TYPED_I32_EXPRESSION");
        let (defect_kind, defect_family) = grammar_opportunity_metadata(
            "AST_GRAMMAR_HOLE:PUBLIC_EXAMPLE_CONTRADICTED_STUB:1",
            "TYPED_I32_EXPRESSION",
        );
        assert_eq!(hole_kind, ChangeOpportunityKind::CapabilityGap);
        assert_eq!(defect_kind, ChangeOpportunityKind::Defect);
        assert_ne!(hole_family, defect_family);

        let (_, add_attempt) = grammar_opportunity_metadata(
            "AST_GRAMMAR_HOLE:TODO:1234567890abcdef",
            "TODO:BINARY_ADD",
        );
        let (_, multiply_attempt) = grammar_opportunity_metadata(
            "AST_GRAMMAR_HOLE:TODO:1234567890abcdef",
            "TODO:BINARY_MULTIPLY",
        );
        assert_eq!(add_attempt, multiply_attempt);
    }

    #[test]
    fn typed_improvement_operator_canary_executes_and_rejects_counterexamples() {
        let mut operator_ids = BTreeSet::new();
        for selector in 0_u32..25 {
            let context = format!("{selector:08x}{}", "0".repeat(56));
            let receipt = execute_improvement_operator_behavioral_canary(&context).unwrap();
            assert_eq!(receipt.cases_executed, 3);
            assert_eq!(receipt.cases_passed, 3);
            assert!(receipt.exact_candidate_observed);
            assert!(receipt.wrong_predecessor_rejected);
            assert!(receipt.tampered_target_rejected);
            assert_eq!(receipt.operator.operator_id.len(), 64);
            operator_ids.insert(receipt.operator.operator_id);
        }
        assert_eq!(operator_ids.len(), 20);
    }

    #[test]
    fn compatible_improvement_operators_execute_as_parallel_graph() {
        let context = "a".repeat(64);
        let left =
            execute_improvement_operator_behavioral_canary(&format!("{:08x}{}", 0, "0".repeat(56)))
                .unwrap();
        let right =
            execute_improvement_operator_behavioral_canary(&format!("{:08x}{}", 1, "0".repeat(56)))
                .unwrap();
        let receipt = execute_improvement_operator_graph_behavioral_canary(
            &left.operator.operator_id,
            &right.operator.operator_id,
            &context,
        )
        .unwrap();
        assert_eq!(receipt.cases_executed, 4);
        assert_eq!(receipt.cases_passed, 4);
        assert!(receipt.parallel_nodes_executed);
        assert!(receipt.exact_postimages_observed);
        assert!(receipt.negative_controls_rejected);
        assert!(receipt.canonical_join_observed);
        assert_eq!(receipt.graph.operator_ids.len(), 2);
        assert_eq!(
            improvement_operator_graph_id(&right.operator.operator_id, &left.operator.operator_id)
                .unwrap(),
            receipt.graph.graph_id
        );

        let third =
            execute_improvement_operator_behavioral_canary(&format!("{:08x}{}", 2, "0".repeat(56)))
                .unwrap();
        let family = execute_improvement_operator_graph_family_behavioral_canary(
            &[
                left.operator.operator_id,
                right.operator.operator_id,
                third.operator.operator_id,
            ],
            &context,
        )
        .unwrap();
        assert_eq!(family.graph.operator_ids.len(), 3);
        assert_eq!(family.cases_passed, family.cases_executed);
        assert!(family.parallel_nodes_executed);
    }

    #[test]
    fn default_retry_bound_is_backward_compatible_with_frozen_configs() {
        let policy = AutonomousSourceMutationPolicy::default();
        let serialized = serde_json::to_value(&policy).unwrap();
        assert!(serialized.get("max_attempts_per_problem").is_none());
        assert!(serialized.get("minimum_predicted_value").is_none());
        assert!(serialized.get("auto_discover_compiler_repairs").is_none());
        assert!(serialized.get("auto_synthesize_grammar_repairs").is_none());
        let restored: AutonomousSourceMutationPolicy = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.max_attempts_per_problem, 4);
        assert_eq!(restored.minimum_predicted_value, 60);
        assert!(restored.auto_discover_compiler_repairs);
        assert!(restored.auto_synthesize_grammar_repairs);
    }

    #[test]
    fn pending_runtime_handoff_is_not_a_repair_counterexample() {
        assert!(source_patch_failure_is_transient(Some(
            "SOURCE_UPDATE_ALREADY_STAGED"
        )));
    }

    #[test]
    fn low_value_cosmetic_discovery_is_skipped_before_validation() {
        let (root, mut policy) = fixture("utility-gate");
        policy.minimum_predicted_value = 60;
        let state = external_state(&root);
        let discovery = discover_known_source_improvement_detailed(&policy, &state, 1).unwrap();
        assert_eq!(
            discovery.disposition,
            SourceDiscoveryDisposition::BelowValueThreshold
        );
        assert!(discovery.candidate.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn value_gate_does_not_invent_an_inapplicable_source_opportunity() {
        let (root, mut policy) = fixture("utility-gate-applicability");
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = false;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn stable() -> bool { true }\n",
        )
        .unwrap();
        let state = external_state(&root);

        let discovery = discover_known_source_improvement_detailed(&policy, &state, 1).unwrap();

        assert_eq!(
            discovery.disposition,
            SourceDiscoveryDisposition::NoApplicableTransformation
        );
        assert!(discovery.candidate.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_fingerprint_detects_non_target_changes() {
        let (root, _) = fixture("workspace-fingerprint");
        let target = root.join("src/lib.rs");
        let before = workspace_semantic_fingerprint(&root, &target).unwrap();
        fs::write(root.join("src/concurrent.rs"), "pub fn changed() {}\n").unwrap();
        let after = workspace_semantic_fingerprint(&root, &target).unwrap();
        assert_ne!(before, after);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compiler_observation_autonomously_finds_and_repairs_a_fresh_defect() {
        let (root, mut policy) = fixture("compiler-guided-defect");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    1;\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 4)
            .unwrap()
            .expect("compiler-guided repair candidate");

        assert!(request.transformation.starts_with("COMPILER_OBSERVATION:"));
        assert_eq!(request.opportunity_kind, ChangeOpportunityKind::Defect);
        assert_eq!(request.opportunity_family_id.len(), 64);
        assert!(request
            .solution_strategy
            .starts_with("COMPILER_SUGGESTION:"));
        assert!(request.structural_repair_program.is_some());
        assert_eq!(
            request.candidate_source,
            "pub fn value() -> i32 {\n    1\n}\n"
        );
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        assert_eq!(
            receipt
                .compile_check
                .as_ref()
                .and_then(|check| check.args.first())
                .map(String::as_str),
            Some("clippy")
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            request.candidate_source
        );
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned compiler repair");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert_eq!(learned.opportunity_kind, ChangeOpportunityKind::Defect);
        assert_eq!(learned.opportunity_family_id, request.opportunity_family_id);
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| !success.edit_atom_kinds.is_empty()));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn compiler_family_utility_survives_operator_reranking() {
        let (root, mut policy) = fixture("compiler-family-ranking");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = true;
        policy.auto_synthesize_grammar_repairs = false;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn merge(left: String, right: String) -> String {\n    let first = left.clone();\n    let second = right.clone();\n    first + &second\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 4)
            .unwrap()
            .expect("one atomic performance family");

        assert!(request
            .transformation
            .starts_with("COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:"));
        assert!(request
            .solution_strategy
            .starts_with("COMPILER_SUGGESTION_FAMILY:MachineApplicable:2:"));
        assert!(matches!(
            request.structural_repair_program.as_ref().map(|program| &program.edit),
            Some(SourceEditAtom::AtomicMultiEdit { edits }) if edits.len() == 2
        ));
        assert!(!request.candidate_source.contains(".clone()"));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn grammar_atoms_compose_and_validate_new_code_without_a_gold_patch() {
        let (root, mut policy) = fixture("grammar-composition-defect");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn add_works() {\n        assert_eq!(super::add(2, 3), 5);\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 5)
            .unwrap()
            .expect("grammar-composed repair candidate");

        assert!(request.transformation.starts_with("AST_GRAMMAR_HOLE:TODO:"));
        assert_eq!(
            request.opportunity_kind,
            ChangeOpportunityKind::CapabilityGap
        );
        assert!(request
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:BINARY_ADD"));
        assert!(request.candidate_source.contains("    left + right\n"));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned grammar composition");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert_eq!(
            learned.opportunity_kind,
            ChangeOpportunityKind::CapabilityGap
        );
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| success.solution_strategy.contains("BINARY_ADD")));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn installed_rust_typed_repair_promotes_and_accelerates_the_next_repository() {
        let (root, mut policy) = fixture("installed-rust-typed-operator");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn count(values: &[i64]) -> i64 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn count_examples() {\n        assert_eq!(super::count(&[1, 2]), 2);\n        assert_eq!(super::count(&[7]), 1);\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let first = discover_known_source_improvement(&policy, &state, 20)
            .unwrap()
            .expect("cold typed Rust repair");
        let operator_id = first
            .typed_mechanism_operator_recipe
            .as_ref()
            .map(|operator| operator.operator_id.clone())
            .expect("canonical typed recipe");
        assert_eq!(first.typed_mechanism_selected_operator_id, None);
        assert!(first.typed_mechanism_candidates_enumerated > 1);
        let first_enumerated = first.typed_mechanism_candidates_enumerated;
        let predecessor_source = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        validate_typed_mechanism_recipe_binding(&first).unwrap();
        validate_typed_mechanism_source_materialization(&first, &predecessor_source).unwrap();
        let mut tampered_binding = first.clone();
        tampered_binding.typed_mechanism_materialized_syntax_source = Some("0i64".to_string());
        assert_eq!(
            validate_typed_mechanism_recipe_binding(&tampered_binding),
            Err("TYPED_MECHANISM_RECIPE_BINDING_MISMATCH".to_string())
        );
        let mut forged_receipt = first.clone();
        forged_receipt
            .typed_mechanism_synthesis_receipt
            .as_mut()
            .unwrap()
            .candidates_falsified += 1;
        assert!(validate_typed_mechanism_recipe_binding(&forged_receipt)
            .unwrap_err()
            .starts_with("TYPED_MECHANISM_"));
        let mut shifted_edit = first.clone();
        let SourceEditAtom::Replace { range, .. } = shifted_edit
            .typed_mechanism_materialized_edit
            .as_mut()
            .unwrap()
        else {
            panic!("typed materialization must be one exact replacement")
        };
        range.start += 1;
        validate_typed_mechanism_recipe_binding(&shifted_edit).unwrap();
        assert_eq!(
            validate_typed_mechanism_source_materialization(&shifted_edit, &predecessor_source),
            Err(
                "TYPED_MECHANISM_SOURCE_EDIT_REPLAY:REPLACE_PRECONDITION_HASH_MISMATCH".to_string()
            )
        );
        let receipt = install_and_stage_source_patch(&policy, &state, &first).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        let authorized =
            load_authorized_typed_mechanism_operators(&state, MAX_ACTIVE_TYPED_MECHANISM_OPERATORS)
                .unwrap();
        assert!(authorized
            .iter()
            .any(|operator| operator.operator_id == operator_id));

        fs::write(
            root.join("src/lib.rs"),
            "pub fn count(values: &[i64]) -> i64 {\n    values.len() as i64\n}\n\npub fn amount(payload: &[i64]) -> i64 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn amount_examples() {\n        assert_eq!(super::amount(&[4, 5, 6]), 3);\n        assert_eq!(super::amount(&[8]), 1);\n    }\n}\n",
        )
        .unwrap();
        let second = discover_known_source_improvement(&policy, &state, 21)
            .unwrap()
            .expect("warm renamed typed Rust repair");
        assert_eq!(
            second.typed_mechanism_selected_operator_id.as_deref(),
            Some(operator_id.as_str())
        );
        assert_eq!(second.typed_mechanism_preferred_operator_attempts, 1);
        assert!(second.typed_mechanism_candidates_enumerated < first_enumerated);
        assert!(second.candidate_source.contains("payload"));
        assert!(second.candidate_source.contains(".len() as i64"));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn typed_call_chain_is_installed_after_public_examples_falsify_primitives() {
        let (root, mut policy) = fixture("grammar-call-chain-repair");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn widen(value: i32) -> i64 {\n    i64::from(value) * i64::from(value)\n}\n\npub fn compare(wide: i64, limit: i32) -> bool {\n    wide > i64::from(limit)\n}\n\npub fn decide(raw: i32, limit: i32) -> bool {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn decides_after_widening() {\n        assert_eq!(super::decide(3, 5), true);\n        assert_eq!(super::decide(2, 5), false);\n        assert_eq!(super::decide(-3, 5), true);\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 6)
            .unwrap()
            .expect("typed call-chain candidate");

        assert!(request
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:EXISTING_CALL_CHAIN"));
        assert!(request
            .candidate_source
            .contains("    compare(widen(raw), limit)\n"));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned typed call-chain composition");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| success.solution_strategy.contains("EXISTING_CALL_CHAIN")));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn typed_string_atoms_compose_and_install_concatenation() {
        let (root, mut policy) = fixture("grammar-string-concat-repair");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn join(left: &str, right: &str) -> String {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn joins() {\n        assert_eq!(super::join(\"a\", \"b\"), \"ab\");\n        assert_eq!(super::join(\"left\", \"right\"), \"leftright\");\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 7)
            .unwrap()
            .expect("typed string concatenation candidate");

        assert!(request
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:STRING_CONCAT"));
        assert!(request
            .candidate_source
            .contains("    format!(\"{}{}\", left, right)\n"));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned string composition");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| success.solution_strategy.contains("STRING_CONCAT")));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn contradicted_stub_is_repaired_from_external_public_examples() {
        let (root, mut policy) = fixture("grammar-external-stub-repair");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 {\n    0\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/add.rs"),
            "#[test]\nfn add_works() {\n    assert_eq!(semantic_reasoning::add(2, 3), 5);\n    assert_eq!(semantic_reasoning::add(-2, 3), 1);\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 5)
            .unwrap()
            .expect("public-behavior repair candidate");

        assert!(request
            .transformation
            .starts_with("AST_GRAMMAR_HOLE:PUBLIC_EXAMPLE_CONTRADICTED_STUB:"));
        assert_eq!(request.opportunity_kind, ChangeOpportunityKind::Defect);
        assert!(request
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:BINARY_ADD"));
        assert!(request
            .generalized_change
            .as_ref()
            .is_some_and(|change| change.weakness_evidence_kind
                == WeaknessEvidenceKind::PublicBehaviorContradiction));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("left + right"));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn public_counterexamples_drive_bounded_grammar_revision_until_success() {
        let (root, mut policy) = fixture("grammar-counterexample-revision");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        policy.max_attempts_per_problem = 4;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combine_works() {\n        let observed = super::combine(3, 4);\n        if observed != 12 {\n            panic!(\"assertion `left == right` failed\\n  left: {observed}\\n right: 12\");\n        }\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);
        let mut strategies = Vec::new();
        let mut final_receipt = None;

        for _ in 0..4 {
            let request = discover_known_source_improvement(&policy, &state, 6)
                .unwrap()
                .expect("next grammar hypothesis");
            strategies.push(request.solution_strategy.clone());
            let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
            if receipt.installed {
                final_receipt = Some((request, receipt));
                break;
            }
            assert!(receipt.rolled_back);
            assert_eq!(
                fs::read_to_string(root.join("src/lib.rs")).unwrap(),
                "pub fn combine(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combine_works() {\n        let observed = super::combine(3, 4);\n        if observed != 12 {\n            panic!(\"assertion `left == right` failed\\n  left: {observed}\\n right: 12\");\n        }\n    }\n}\n"
            );
        }

        let (request, receipt) =
            final_receipt.expect("feedback-ranked grammar composition succeeds");
        assert!(receipt.validation.success);
        assert_eq!(strategies.len(), 2);
        assert!(strategies[0].contains("BINARY_ADD"));
        assert!(strategies[1].contains("BINARY_MULTIPLY"));
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("counterexample-guided learning record");
        let success = learned
            .learned_success
            .expect("learned successful composition");
        assert_eq!(success.attempts_required, 2);
        assert!(success.solution_strategy.contains("BINARY_MULTIPLY"));
        assert_eq!(learned.attempts.len(), 2);
        let first_counterexample = learned.attempts[0]
            .validation_counterexample
            .as_ref()
            .expect("public failure becomes a structured counterexample");
        assert_eq!(
            first_counterexample.numeric_relation,
            Some(crate::generalized_self_application::NumericRelation::ExpectedGreaterThanObserved)
        );
        assert!(learned.attempts[1]
            .derived_from_counterexample_ids
            .contains(&first_counterexample.counterexample_id));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn repository_repair_family_installs_and_validates_as_one_transaction() {
        let (root, mut policy) = fixture("repository-family-install");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            r#"pub mod other;
pub fn add_pair(left: i32, right: i32) -> i32 {
    left - right
}
#[cfg(test)]
mod tests {
    #[test]
    fn example() {
        assert_eq!(super::add_pair(2, 3), 5);
    }
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("src/other.rs"),
            r#"pub fn add_more(left: i32, right: i32) -> i32 {
    left - right
}
#[cfg(test)]
mod tests {
    #[test]
    fn example() {
        assert_eq!(super::add_more(4, 6), 10);
    }
}
"#,
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 9)
            .unwrap()
            .expect("repository family repair");
        assert!(request
            .transformation
            .starts_with("AST_GRAMMAR_REPOSITORY_FAMILY:"));
        assert_eq!(request.additional_family_members.len(), 1);

        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();

        assert!(receipt.installed);
        assert!(receipt.validation.success);
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("left + right"));
        assert!(fs::read_to_string(root.join("src/other.rs"))
            .unwrap()
            .contains("left + right"));
        assert!(fs::read_dir(root.join("src")).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("bcore-")));
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .and_then(|record| record.learned_success)
            .expect("repository family success is learned");
        assert_eq!(learned.family_member_count, 2);
        assert_eq!(learned.family_structural_repair_program_sha256.len(), 1);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn failed_repository_family_restores_every_member() {
        let (root, mut policy) = fixture("repository-family-rollback");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        let primary_source = "pub mod other;\npub fn add_pair(left: i32, right: i32) -> i32 {\n    left - right\n}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn example() {\n        assert_eq!(super::add_pair(2, 3), 5);\n    }\n}\n";
        let additional_source = "pub fn add_more(left: i32, right: i32) -> i32 {\n    left - right\n}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn example() {\n        assert_eq!(super::add_more(4, 6), 10);\n    }\n}\n";
        fs::write(root.join("src/lib.rs"), primary_source).unwrap();
        fs::write(root.join("src/other.rs"), additional_source).unwrap();
        let state = external_state(&root);
        let mut request = discover_known_source_improvement(&policy, &state, 10)
            .unwrap()
            .expect("repository family repair");
        assert_eq!(request.additional_family_members.len(), 1);
        request.patch_id.push_str("-invalid");
        request.candidate_source = primary_source.replace("left - right", "left * right");
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());
        request.structural_repair_program = Some(
            synthesize_structural_repair("src/lib.rs", primary_source, &request.candidate_source)
                .unwrap(),
        );
        request.generalized_change = None;
        rebind_request_operator_execution(&mut request, &state, primary_source);

        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();

        assert!(!receipt.installed);
        assert!(receipt.rolled_back);
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            primary_source
        );
        assert_eq!(
            fs::read_to_string(root.join("src/other.rs")).unwrap(),
            additional_source
        );
        assert!(fs::read_dir(root.join("src")).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("bcore-")));

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn generated_lint_defect_is_rejected_before_installation() {
        let (root, policy) = fixture("generated-lint-gate");
        let state = external_state(&root);
        let predecessor = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("base request");
        request.patch_id.push_str("-lint");
        request.candidate_source = r#"pub fn even(value: u32) -> bool {
    let mut observed = false;
    if value.is_multiple_of(2) {
        observed = true;
    } else {
    }
    observed
}

#[cfg(test)]
mod tests {
    #[test]
    fn even_works() {
        assert!(super::even(2));
    }
}
"#
        .to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());
        request.structural_repair_program = Some(
            synthesize_structural_repair("src/lib.rs", &predecessor, &request.candidate_source)
                .unwrap(),
        );
        request.generalized_change = None;
        rebind_request_operator_execution(&mut request, &state, &predecessor);

        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();

        assert!(!receipt.installed);
        assert!(receipt.rolled_back);
        assert_eq!(
            receipt.failure_reason.as_deref(),
            Some("CLIPPY_CHECK_FAILED")
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            predecessor
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn core_can_install_validate_and_stage_its_own_source_patch() {
        let (root, policy) = fixture("install");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        assert!(request.core_self_approved);
        assert!(request
            .structural_repair_program
            .as_ref()
            .is_some_and(|program| program.file_id == "src/lib.rs"));
        assert!(request.generalized_change.as_ref().is_some_and(|change| {
            !change.fixed_toggle_patch
                && !change.one_generation_only
                && change.source_generation == 3
        }));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(!receipt.rolled_back);
        assert!(receipt.validation.success);
        assert!(receipt
            .release_build
            .as_ref()
            .is_some_and(|value| value.success));
        assert!(receipt.runtime_update_staged);
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("value.is_multiple_of(2)"));
        assert!(state
            .join("control")
            .join(SELF_UPDATE_HANDOFF_FILE)
            .is_file());
        let knowledge = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("success knowledge");
        assert_eq!(knowledge.status, "LEARNED_SUCCESS");
        assert_eq!(knowledge.attempts.len(), 1);
        let learned = knowledge.learned_success.unwrap();
        assert_eq!(learned.solution_strategy, "TYPED_IS_MULTIPLE_OF");
        assert!(learned.structural_repair_program_sha256.is_some());
        assert!(!learned.edit_atom_kinds.is_empty());
        assert!(learned.structural_postcondition_count > 0);
        assert!(learned.generalized_change_sha256.is_some());
        let incremental = root.join("target/debug/incremental");
        assert!(incremental.is_dir());
        assert!(fs::read_dir(&incremental).unwrap().next().is_some());
        assert!(receipt
            .compile_check
            .as_ref()
            .is_some_and(|command| command.cargo_incremental));
        assert!(receipt.validation.cargo_incremental);
        assert!(receipt
            .release_build
            .as_ref()
            .is_some_and(|command| command.cargo_incremental));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn malformed_patch_without_typed_program_is_rejected_before_write() {
        let (root, policy) = fixture("rollback");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        request.patch_id.push_str("-invalid");
        request.candidate_source = "pub fn broken( {\n".to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());
        request.structural_repair_program = None;
        request.generalized_change = None;
        let error = install_and_stage_source_patch(&policy, &state, &request).unwrap_err();
        assert!(error.contains("IMPROVEMENT_OPERATOR_PROGRAM_MISSING"));
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state
            .join("control")
            .join(SELF_UPDATE_HANDOFF_FILE)
            .exists());
        assert!(load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .is_none());
        fs::remove_dir_all(root).unwrap();
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
    }

    #[test]
    fn tampered_candidate_cannot_bypass_structural_program_replay() {
        let (root, policy) = fixture("structural-replay");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        request.candidate_source = "pub fn even(_: u32) -> bool { false }\n".to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());

        let error = install_and_stage_source_patch(&policy, &state, &request).unwrap_err();

        assert!(error.contains("STRUCTURAL_REPAIR_REPLAY_MISMATCH"));
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state.join("source_mutations").exists());
        fs::remove_dir_all(root).unwrap();
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
    }

    #[test]
    fn tampered_generalized_change_cannot_bypass_source_binding() {
        let (root, policy) = fixture("generalized-change-tamper");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 11)
            .unwrap()
            .expect("generalized change");
        request
            .generalized_change
            .as_mut()
            .expect("change")
            .solution_strategy = "FIXED_SEM9_TOGGLE_REPLAY".to_string();

        let error = install_and_stage_source_patch(&policy, &state, &request).unwrap_err();
        assert_eq!(error, "GENERALIZED_CHANGE_REQUEST_BINDING_FAILURE");
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state.join("source_mutations").exists());
        fs::remove_dir_all(root).unwrap();
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
    }

    #[test]
    fn four_failed_solutions_are_admitted_then_reopened_after_growth() {
        let (root, policy) = fixture("bounded-retry");
        let state = external_state(&root);
        let mut problem_id = String::new();
        for expected_attempts in 1..=4 {
            let request = discover_known_source_improvement(&policy, &state, 7)
                .unwrap()
                .expect("bounded solution");
            problem_id = repair_problem_id(&request);
            let receipt = synthetic_receipt(&request, false);
            let knowledge =
                record_source_repair_outcome(&policy, &state, &request, &receipt).unwrap();
            assert_eq!(knowledge.attempts.len(), expected_attempts);
        }
        let admitted = load_repair_learning(&state, &problem_id)
            .unwrap()
            .expect("admitted failure");
        assert_eq!(admitted.status, "ADMITTED_FAILURE");
        assert_eq!(admitted.eligible_after_generation, Some(8));
        assert!(discover_known_source_improvement(&policy, &state, 7)
            .unwrap()
            .is_none());

        let retry = discover_known_source_improvement(&policy, &state, 8)
            .unwrap()
            .expect("reopened after growth");
        let success = synthetic_receipt(&retry, true);
        let learned = record_source_repair_outcome(&policy, &state, &retry, &success).unwrap();
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert_eq!(learned.attempts.len(), 5);
        assert_eq!(learned.cycle_attempt_start_index, 4);
        assert_eq!(learned.learned_success.unwrap().attempts_required, 1);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn workspace_contention_is_a_deferred_attempt_not_a_learned_counterexample() {
        let (root, policy) = fixture("transient-workspace-contention");
        let state = external_state(&root);
        let mut problem_id = String::new();

        for physical_attempt in 1..=6 {
            let request = discover_known_source_improvement(&policy, &state, 7)
                .unwrap()
                .expect("same strategy remains retryable after workspace contention");
            problem_id = repair_problem_id(&request);
            let mut receipt = synthetic_receipt(&request, false);
            receipt.failure_reason =
                Some("CONCURRENT_WORKSPACE_CHANGE_DURING_VALIDATION".to_string());
            receipt.receipt_sha256 = receipt_hash(&receipt).unwrap();

            assert!(counterexample_from_receipt(&request, &receipt).is_none());
            let knowledge =
                record_source_repair_outcome(&policy, &state, &request, &receipt).unwrap();
            assert_eq!(knowledge.status, "RETRYING");
            assert_eq!(knowledge.attempts.len(), physical_attempt);
            assert!(active_cycle_attempts(&knowledge, 7).is_empty());
        }

        let knowledge = load_repair_learning(&state, &problem_id)
            .unwrap()
            .expect("deferred attempts remain auditable");
        assert!(knowledge
            .attempts
            .iter()
            .all(|attempt| attempt.validation_counterexample.is_none()));
        let memory = derive_improvement_operator_memory(&state).unwrap();
        assert_eq!(memory.total_attempts, 0);
        assert_eq!(memory.total_successful_uses, 0);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn distinct_successor_artifact_starts_a_fresh_cycle_without_erasing_operator_history() {
        let (root, policy) = fixture("successor-artifact-cycle");
        let state = external_state(&root);
        let first = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("first candidate");
        let first_receipt = synthetic_receipt(&first, true);
        let first_record =
            record_source_repair_outcome(&policy, &state, &first, &first_receipt).unwrap();
        assert_eq!(first_record.cycle_attempt_start_index, 0);
        assert_eq!(first_record.learned_success.unwrap().attempts_required, 1);

        let mut successor = first.clone();
        successor.source_generation = 13;
        successor.patch_id.push_str("-successor");
        successor.candidate_sha256 = "9".repeat(64);
        let successor_receipt = synthetic_receipt(&successor, true);
        let successor_record =
            record_source_repair_outcome(&policy, &state, &successor, &successor_receipt).unwrap();

        assert_eq!(successor_record.status, "LEARNED_SUCCESS");
        assert_eq!(successor_record.attempts.len(), 2);
        assert_eq!(successor_record.cycle_attempt_start_index, 1);
        assert_eq!(active_cycle_attempts(&successor_record, 13).len(), 1);
        assert_eq!(
            successor_record.learned_success.unwrap().attempts_required,
            1
        );
        let memory = derive_improvement_operator_memory(&state).unwrap();
        assert_eq!(memory.total_attempts, 2);
        assert_eq!(memory.total_successful_uses, 2);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn successful_structural_operator_is_stored_invoked_and_strengthened_by_transfer() {
        let (root, policy) = fixture("callable-improvement-operator");
        let state = external_state(&root);
        let first = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("first structural candidate");
        let first_receipt = synthetic_receipt(&first, true);
        record_source_repair_outcome(&policy, &state, &first, &first_receipt).unwrap();

        let mut second = first.clone();
        second.patch_id.push_str("-second-family");
        second.transformation.push_str(":SECOND_FAMILY");
        second.opportunity_family_id = source_opportunity_family_id(
            ChangeOpportunityKind::EfficiencyOpportunity,
            &second.transformation,
        );
        let second_receipt = synthetic_receipt(&second, true);
        record_source_repair_outcome(&policy, &state, &second, &second_receipt).unwrap();

        let memory = refresh_improvement_operator_repository(&state).unwrap();
        assert_eq!(memory.total_successful_uses, 2);
        assert_eq!(memory.productive_cross_family_transfers, 1);
        let profile = memory
            .profiles
            .iter()
            .find(|profile| profile.successful_uses == 2)
            .expect("cross-family operator profile");
        assert_eq!(profile.successful_family_ids.len(), 2);
        let stored = improvement_operator_repository_path(&state, &profile.operator.operator_id);
        assert!(stored.is_file());
        assert_eq!(
            profile.operator.generator_kind,
            ImprovementOperatorGeneratorKind::KnownStructuralRewrite
        );
        let executable = execute_improvement_operator_on_source(
            &profile.operator,
            "pub fn even(value: u32) -> bool { value % 2 == 0 }\n",
        )
        .unwrap();
        assert!(executable.applicable);
        assert!(executable
            .candidate_source
            .as_deref()
            .is_some_and(|source| source.contains("value.is_multiple_of(2)")));

        let new_family = source_opportunity_family_id(
            ChangeOpportunityKind::EfficiencyOpportunity,
            "THIRD_FAMILY",
        );
        let invocation = invoke_improvement_operator_repository(
            &memory,
            first
                .generalized_change
                .as_ref()
                .expect("generalized change")
                .weakness_evidence_kind,
            &first.transformation,
            &first.solution_strategy,
            first
                .structural_repair_program
                .as_ref()
                .expect("structural program"),
            &new_family,
        )
        .unwrap();
        assert_eq!(invocation.matched_operator_ids.len(), 1);
        assert_eq!(invocation.prior_successful_uses, 2);
        assert_eq!(invocation.cross_family_successes, 2);
        assert!(invocation.priority_adjustment > 0);

        fs::write(
            root.join("src/other.rs"),
            "pub fn other_even(value: u32) -> bool { value % 2 == 0 }\n",
        )
        .unwrap();
        let transferred = discover_known_source_improvement(&policy, &state, 13)
            .unwrap()
            .expect("repository-guided candidate on a fresh family");
        let bound_invocation = transferred
            .improvement_operator_invocation
            .as_ref()
            .expect("candidate is causally bound to repository invocation");
        assert_eq!(transferred.relative_path, PathBuf::from("src/other.rs"));
        assert_eq!(transferred.solution_strategy, first.solution_strategy);
        assert_eq!(bound_invocation.matched_operator_ids.len(), 1);
        assert_eq!(bound_invocation.prior_successful_uses, 2);
        assert!(bound_invocation.priority_adjustment > 0);
        let mut transferred_outcome = transferred.clone();
        transferred_outcome.transformation.push_str(":THIRD_FAMILY");
        transferred_outcome.opportunity_family_id = source_opportunity_family_id(
            ChangeOpportunityKind::EfficiencyOpportunity,
            &transferred_outcome.transformation,
        );
        let transferred_receipt = synthetic_receipt(&transferred_outcome, true);
        record_source_repair_outcome(&policy, &state, &transferred_outcome, &transferred_receipt)
            .unwrap();
        let causally_updated = refresh_improvement_operator_repository(&state).unwrap();
        assert_eq!(causally_updated.repository_guided_attempts, 1);
        assert_eq!(causally_updated.repository_guided_successful_uses, 1);
        assert_eq!(causally_updated.productive_cross_family_transfers, 2);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn program_execution_profile_is_not_published_as_source_synthesis_knowledge() {
        let (root, policy) = fixture("non-synthesizing-operator-profile");
        let state = external_state(&root);
        let mut request = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("typed structural candidate");
        request.transformation =
            "COMPILER_OBSERVATION_FAMILY:clippy::manual_is_multiple_of:fresh".to_string();
        request.solution_strategy =
            "COMPILER_SUGGESTION_FAMILY:MachineApplicable:1:fresh".to_string();
        request.opportunity_family_id = source_opportunity_family_id(
            ChangeOpportunityKind::RobustnessOpportunity,
            &request.transformation,
        );
        let predecessor = fs::read_to_string(root.join(&request.relative_path)).unwrap();
        rebind_request_operator_execution(&mut request, &state, &predecessor);
        let receipt = synthetic_receipt(&request, true);
        record_source_repair_outcome(&policy, &state, &request, &receipt).unwrap();

        let memory = refresh_improvement_operator_repository(&state).unwrap();
        let profile = memory.profiles.first().expect("execution profile");
        assert_eq!(
            profile.operator.generator_kind,
            ImprovementOperatorGeneratorKind::CompilerSuggestedEdit
        );
        assert!(!profile.operator.can_synthesize_from_source());
        assert!(
            !improvement_operator_repository_path(&state, &profile.operator.operator_id).exists()
        );

        let source_only =
            execute_improvement_operator_on_source(&profile.operator, &predecessor).unwrap();
        assert!(!source_only.applicable);
        assert_eq!(
            source_only.execution_reason,
            "NO_EXECUTABLE_SOURCE_SYNTHESIS_PAYLOAD"
        );
        let program_execution = execute_improvement_operator_program_on_source(
            &profile.operator,
            &predecessor,
            request
                .structural_repair_program
                .as_ref()
                .expect("typed structural program"),
        )
        .unwrap();
        assert!(program_execution.applicable);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn pre_dispatch_verified_program_bootstraps_operator_repository_once() {
        let (root, policy) = fixture("legacy-operator-bootstrap");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("structural candidate");
        let receipt = synthetic_receipt(&request, true);
        let mut record = record_source_repair_outcome(&policy, &state, &request, &receipt).unwrap();
        assert!(record.attempts[0].executed_operator_id.is_some());

        record.cycle_started_engine_revision = SOURCE_REPAIR_ENGINE_REVISION - 1;
        record.attempts[0].source_engine_revision = 0;
        record.attempts[0].executed_operator_id = None;
        record.attempts[0].improvement_operator_execution_sha256 = None;
        record.attempts[0].invoked_operator_ids.clear();
        write_mutable_json(&repair_learning_path(&state, &record.problem_id), &record).unwrap();

        let bootstrapped = refresh_improvement_operator_repository(&state).unwrap();
        assert_eq!(bootstrapped.total_attempts, 1);
        assert_eq!(bootstrapped.total_successful_uses, 1);
        assert_eq!(bootstrapped.repository_guided_attempts, 0);
        let profile = &bootstrapped.profiles[0];
        assert!(
            improvement_operator_repository_path(&state, &profile.operator.operator_id).is_file()
        );
        let execution = execute_improvement_operator_on_source(
            &profile.operator,
            "pub fn renamed(value: u32) -> bool { value % 2 == 0 }\n",
        )
        .unwrap();
        assert!(execution.applicable);
        assert!(execution
            .candidate_source
            .as_deref()
            .is_some_and(|source| source.contains("value.is_multiple_of(2)")));

        // Absence of dispatcher binding is accepted only for bounded records
        // that provably predate the dispatcher contract.
        record.attempts[0].source_engine_revision = SOURCE_REPAIR_ENGINE_REVISION;
        write_mutable_json(&repair_learning_path(&state, &record.problem_id), &record).unwrap();
        let current_revision = derive_improvement_operator_memory(&state).unwrap();
        assert_eq!(current_revision.total_attempts, 0);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn changing_compiler_hash_keeps_counterexample_and_allows_a_new_postimage() {
        let (root, policy) = fixture("stable-compiler-family-identity");
        let state = external_state(&root);
        let mut failed = discover_known_source_improvement(&policy, &state, 12)
            .unwrap()
            .expect("structural candidate");
        failed.transformation =
            "COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:aaaaaaaaaaaa".to_string();
        failed.solution_strategy =
            "COMPILER_SUGGESTION_FAMILY:MachineApplicable:7:aaaaaaaaaaaa".to_string();
        failed.opportunity_kind = ChangeOpportunityKind::RobustnessOpportunity;
        failed.opportunity_family_id = source_opportunity_family_id(
            failed.opportunity_kind,
            normalized_hash_suffixed_family(&failed.transformation),
        );
        let failed_receipt = synthetic_receipt(&failed, false);
        record_source_repair_outcome(&policy, &state, &failed, &failed_receipt).unwrap();

        let successor_transformation =
            "COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:bbbbbbbbbbbb";
        assert_eq!(
            repair_problem_id_for(&failed.relative_path, &failed.transformation),
            repair_problem_id_for(&failed.relative_path, successor_transformation)
        );
        let inherited =
            prior_counterexamples(&state, &failed.relative_path, successor_transformation).unwrap();
        assert_eq!(inherited.len(), 1);

        let mut succeeded = failed.clone();
        succeeded.transformation = successor_transformation.to_string();
        succeeded.patch_id.push_str("-success");
        let success_receipt = synthetic_receipt(&succeeded, true);
        record_source_repair_outcome(&policy, &state, &succeeded, &success_receipt).unwrap();
        assert!(!repair_strategy_is_available(
            &policy,
            &state,
            &succeeded.relative_path,
            "COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:cccccccccccc",
            &succeeded.solution_strategy,
            (&succeeded.predecessor_sha256, &succeeded.candidate_sha256),
            13,
        )
        .unwrap());
        assert!(repair_strategy_is_available(
            &policy,
            &state,
            &succeeded.relative_path,
            "COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:dddddddddddd",
            &succeeded.solution_strategy,
            (&"e".repeat(64), &"f".repeat(64)),
            13,
        )
        .unwrap());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }
}
