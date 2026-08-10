//! Bounded self-healing coordination with an explicit operator-teaching path.
//!
//! The core may audit, diagnose, learn a generalized repair schema, and emit a
//! proposal. It still cannot approve or install its own patch. Operator repairs
//! are useful as teaching evidence only after independent verification and
//! successful replay on fresh, non-identical transfer scenarios.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::{
    sha256, validate_installation_authority, DefectContractIR, Frozen, ObservationIR,
    PatchCandidateIR, RepairSpecIR, VerificationDecision, VerificationReceipt,
};

pub const PIPELINE_SCHEMA: &str = "B_CORE_SELF_HEALING_PIPELINE_1";
pub const DEFAULT_MAX_CORE_ATTEMPTS: u8 = 3;
pub const MIN_FRESH_TRANSFER_CASES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SurfaceAuthority {
    Compiled,
    Tooling,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSurface {
    pub package: String,
    pub target: String,
    pub target_kind: String,
    pub source_path: String,
    pub authority: SurfaceAuthority,
}

impl ModuleSurface {
    fn key(&self) -> String {
        format!("{}::{}::{}", self.package, self.target_kind, self.target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProbeStatus {
    Pass,
    PassAfterInfrastructureRetry,
    Fail,
    Timeout,
    NotRun,
}

impl ProbeStatus {
    fn is_effective_pass(self) -> bool {
        matches!(self, Self::Pass | Self::PassAfterInfrastructureRetry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureOrigin {
    SourceDefect,
    ToolchainTransient,
    ResourceOrTimeout,
    Unknown,
}

pub fn classify_failure_origin(output: &str, status: ProbeStatus) -> FailureOrigin {
    let lowered = output.to_ascii_lowercase();
    if status == ProbeStatus::Timeout
        || lowered.contains("out of memory")
        || lowered.contains("resource temporarily unavailable")
    {
        FailureOrigin::ResourceOrTimeout
    } else if lowered.contains("internal compiler error")
        || lowered.contains("query stack during panic")
    {
        FailureOrigin::ToolchainTransient
    } else if lowered.contains("error[")
        || lowered.contains("test result: failed")
        || lowered.contains("warning:")
    {
        FailureOrigin::SourceDefect
    } else {
        FailureOrigin::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthProbeReceipt {
    pub surface_key: String,
    pub probe_kind: String,
    pub command_sha256: String,
    pub output_sha256: String,
    pub status: ProbeStatus,
    pub duration_ms: u64,
    pub bounded_timeout_ms: u64,
    pub externally_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleAuditSummary {
    pub inventory_count: usize,
    pub compiled_surface_count: usize,
    pub tooling_surface_count: usize,
    pub quarantined_surface_count: usize,
    pub probed_compiled_surface_count: usize,
    pub passing_compiled_surface_count: usize,
    pub failing_surface_keys: Vec<String>,
    pub timed_out_surface_keys: Vec<String>,
    pub missing_compiled_surface_keys: Vec<String>,
    pub compiled_coverage_complete: bool,
    pub all_compiled_probes_pass: bool,
}

/// Coverage is separate from success: an unprobed target cannot be counted as
/// passing, and quarantined source cannot silently inflate compiled coverage.
pub fn summarize_module_audit(
    inventory: &[ModuleSurface],
    probes: &[HealthProbeReceipt],
) -> ModuleAuditSummary {
    let mut by_surface: BTreeMap<&str, Vec<&HealthProbeReceipt>> = BTreeMap::new();
    for probe in probes {
        by_surface
            .entry(probe.surface_key.as_str())
            .or_default()
            .push(probe);
    }

    let mut compiled_surface_count = 0;
    let mut tooling_surface_count = 0;
    let mut quarantined_surface_count = 0;
    let mut probed_compiled_surface_count = 0;
    let mut passing_compiled_surface_count = 0;
    let mut failing = BTreeSet::new();
    let mut timed_out = BTreeSet::new();
    let mut missing = BTreeSet::new();

    for surface in inventory {
        match surface.authority {
            SurfaceAuthority::Compiled => {
                compiled_surface_count += 1;
                let key = surface.key();
                let receipts = by_surface.get(key.as_str()).cloned().unwrap_or_default();
                if receipts.is_empty()
                    || receipts
                        .iter()
                        .all(|probe| probe.status == ProbeStatus::NotRun)
                {
                    missing.insert(key);
                    continue;
                }
                probed_compiled_surface_count += 1;
                if receipts
                    .iter()
                    .any(|probe| probe.status == ProbeStatus::Fail)
                {
                    failing.insert(key.clone());
                }
                if receipts
                    .iter()
                    .any(|probe| probe.status == ProbeStatus::Timeout)
                {
                    timed_out.insert(key);
                }
                if receipts
                    .iter()
                    .all(|probe| probe.status.is_effective_pass())
                {
                    passing_compiled_surface_count += 1;
                }
            }
            SurfaceAuthority::Tooling => tooling_surface_count += 1,
            SurfaceAuthority::Quarantined => quarantined_surface_count += 1,
        }
    }

    let compiled_coverage_complete =
        missing.is_empty() && probed_compiled_surface_count == compiled_surface_count;
    let all_compiled_probes_pass = compiled_coverage_complete
        && failing.is_empty()
        && timed_out.is_empty()
        && passing_compiled_surface_count == compiled_surface_count;
    ModuleAuditSummary {
        inventory_count: inventory.len(),
        compiled_surface_count,
        tooling_surface_count,
        quarantined_surface_count,
        probed_compiled_surface_count,
        passing_compiled_surface_count,
        failing_surface_keys: failing.into_iter().collect(),
        timed_out_surface_keys: timed_out.into_iter().collect(),
        missing_compiled_surface_keys: missing.into_iter().collect(),
        compiled_coverage_complete,
        all_compiled_probes_pass,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DefectClass {
    ManualRemainderPredicate,
    BoundaryRelation,
    BooleanComposition,
    NondeterministicSelection,
    ResourceRegression,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairAttemptStatus {
    CandidateProposed,
    CapabilityGap,
    NoApplicableLesson,
    BoundedAttemptExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairLessonIR {
    pub lesson_id: String,
    pub defect_class: DefectClass,
    pub diagnostic_cues: Vec<String>,
    pub restored_invariants: Vec<String>,
    pub transformation_schema: String,
    pub applicability: Vec<String>,
    pub non_applicability: Vec<String>,
    pub regression_obligations: Vec<String>,
    pub composition_lesson: RepairCompositionLessonIR,
    pub exact_patch_data_present: bool,
    pub exact_repository_identity_present: bool,
    pub exact_task_identity_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairPrimitiveIR {
    pub primitive_id: String,
    pub implementation_anchor: String,
    pub input_type: String,
    pub output_type: String,
    pub semantic_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionEdgeIR {
    pub from_primitive_id: String,
    pub to_primitive_id: String,
    pub transported_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairCompositionLessonIR {
    pub composition_id: String,
    pub primitives: Vec<RepairPrimitiveIR>,
    pub edges: Vec<CompositionEdgeIR>,
    pub execution_order: Vec<String>,
    pub required_semantic_roles: Vec<String>,
    pub applicability: Vec<String>,
    pub non_applicability: Vec<String>,
    pub exact_source_fragment_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionValidationError {
    ExactSourceFragment,
    InsufficientPrimitiveComposition,
    DuplicatePrimitive,
    MissingPrimitiveReference,
    TypeTransportMismatch,
    ExecutionOrderMismatch,
    MissingSemanticRole,
}

/// Type-checks the generalized assembly recipe. This learns which existing
/// code capabilities must be connected, not the source text of one repair.
pub fn validate_composition_lesson(
    composition: &RepairCompositionLessonIR,
) -> Result<(), CompositionValidationError> {
    if composition.exact_source_fragment_present {
        return Err(CompositionValidationError::ExactSourceFragment);
    }
    if composition.primitives.len() < 3 || composition.edges.len() < 2 {
        return Err(CompositionValidationError::InsufficientPrimitiveComposition);
    }
    let mut primitives = BTreeMap::new();
    for primitive in &composition.primitives {
        if primitives
            .insert(primitive.primitive_id.as_str(), primitive)
            .is_some()
        {
            return Err(CompositionValidationError::DuplicatePrimitive);
        }
    }
    let order = composition
        .execution_order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    if order.len() != primitives.len() || primitives.keys().any(|id| !order.contains_key(id)) {
        return Err(CompositionValidationError::ExecutionOrderMismatch);
    }
    for edge in &composition.edges {
        let Some(from) = primitives.get(edge.from_primitive_id.as_str()) else {
            return Err(CompositionValidationError::MissingPrimitiveReference);
        };
        let Some(to) = primitives.get(edge.to_primitive_id.as_str()) else {
            return Err(CompositionValidationError::MissingPrimitiveReference);
        };
        if from.output_type != edge.transported_type || to.input_type != edge.transported_type {
            return Err(CompositionValidationError::TypeTransportMismatch);
        }
        if order[from.primitive_id.as_str()] >= order[to.primitive_id.as_str()] {
            return Err(CompositionValidationError::ExecutionOrderMismatch);
        }
    }
    let roles = composition
        .primitives
        .iter()
        .map(|primitive| primitive.semantic_role.as_str())
        .collect::<BTreeSet<_>>();
    if composition
        .required_semantic_roles
        .iter()
        .any(|role| !roles.contains(role.as_str()))
    {
        return Err(CompositionValidationError::MissingSemanticRole);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTeachingReceipt {
    pub operator_identity: String,
    pub scenario_sha256: String,
    pub defect_contract_sha256: String,
    pub patch_candidate: PatchCandidateIR,
    pub verification_receipt: VerificationReceipt,
    pub operator_patch_installed_in_isolation: bool,
    pub post_install_regression_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshTransferReceipt {
    pub scenario_sha256: String,
    pub source_shape_sha256: String,
    pub core_generated: bool,
    pub patch_candidate: PatchCandidateIR,
    pub verification_receipt: VerificationReceipt,
    pub regression_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotedRepairLesson {
    pub lesson: Frozen<RepairLessonIR>,
    pub teaching_scenario_sha256: String,
    pub fresh_transfer_scenario_sha256: Vec<String>,
    pub independent_transfer_verifications: usize,
    pub exact_patch_lookup_events: usize,
    pub task_identity_routing_events: usize,
    pub repository_identity_routing_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RepairLearningMemory {
    pub promoted_lessons: Vec<PromotedRepairLesson>,
    pub rejected_lesson_sha256: Vec<String>,
    pub operator_teaching_events: usize,
    pub successful_fresh_transfer_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LessonPromotionError {
    LessonContainsExactSolutionAuthority,
    InvalidCompositionLesson,
    TeachingScenarioMismatch,
    TeachingRepairNotVerified,
    TeachingRepairNotInstalledAndRegated,
    InsufficientFreshTransfer,
    ReusedTeachingScenario,
    DuplicateTransferScenario,
    TransferNotCoreGenerated,
    TransferRepairNotVerified,
    TransferRegressionFailed,
}

pub fn promote_operator_lesson(
    lesson: RepairLessonIR,
    teaching: &OperatorTeachingReceipt,
    transfers: &[FreshTransferReceipt],
) -> Result<PromotedRepairLesson, LessonPromotionError> {
    if lesson.exact_patch_data_present
        || lesson.exact_repository_identity_present
        || lesson.exact_task_identity_present
    {
        return Err(LessonPromotionError::LessonContainsExactSolutionAuthority);
    }
    validate_composition_lesson(&lesson.composition_lesson)
        .map_err(|_| LessonPromotionError::InvalidCompositionLesson)?;
    if teaching.scenario_sha256.is_empty() || teaching.operator_identity.is_empty() {
        return Err(LessonPromotionError::TeachingScenarioMismatch);
    }
    if teaching.verification_receipt.defect_contract_sha256 != teaching.defect_contract_sha256
        || validate_installation_authority(
            &teaching.patch_candidate,
            &teaching.verification_receipt,
        )
        .is_err()
    {
        return Err(LessonPromotionError::TeachingRepairNotVerified);
    }
    if !teaching.operator_patch_installed_in_isolation || !teaching.post_install_regression_passed {
        return Err(LessonPromotionError::TeachingRepairNotInstalledAndRegated);
    }
    if transfers.len() < MIN_FRESH_TRANSFER_CASES {
        return Err(LessonPromotionError::InsufficientFreshTransfer);
    }

    let mut scenario_hashes = BTreeSet::new();
    for transfer in transfers {
        if transfer.scenario_sha256 == teaching.scenario_sha256 {
            return Err(LessonPromotionError::ReusedTeachingScenario);
        }
        if !scenario_hashes.insert(transfer.scenario_sha256.clone()) {
            return Err(LessonPromotionError::DuplicateTransferScenario);
        }
        if !transfer.core_generated {
            return Err(LessonPromotionError::TransferNotCoreGenerated);
        }
        if validate_installation_authority(
            &transfer.patch_candidate,
            &transfer.verification_receipt,
        )
        .is_err()
            || transfer.verification_receipt.decision != VerificationDecision::Accept
        {
            return Err(LessonPromotionError::TransferRepairNotVerified);
        }
        if !transfer.regression_passed {
            return Err(LessonPromotionError::TransferRegressionFailed);
        }
    }

    let lesson = Frozen::new(lesson)
        .map_err(|_| LessonPromotionError::LessonContainsExactSolutionAuthority)?;
    Ok(PromotedRepairLesson {
        lesson,
        teaching_scenario_sha256: teaching.scenario_sha256.clone(),
        fresh_transfer_scenario_sha256: scenario_hashes.into_iter().collect(),
        independent_transfer_verifications: transfers.len(),
        exact_patch_lookup_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreRepairRequest {
    pub scenario_sha256: String,
    pub logical_file_id: String,
    pub predecessor_tree_hash: String,
    pub defect_class: DefectClass,
    pub observation: Frozen<ObservationIR>,
    pub defect_contract: Frozen<DefectContractIR>,
    pub repair_spec: Frozen<RepairSpecIR>,
    pub source_text: String,
    pub attempt: u8,
    pub max_attempts: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreRepairAttempt {
    pub status: RepairAttemptStatus,
    pub matched_lesson_sha256: Option<String>,
    pub candidate_source: Option<String>,
    pub candidate_diff: Option<String>,
    pub patch_candidate: Option<PatchCandidateIR>,
    pub changed_line_count: usize,
    pub activated_file_count: usize,
    pub activated_composition_id: Option<String>,
    pub activated_primitive_ids: Vec<String>,
    pub primitive_recombinations: usize,
    pub core_self_approval_events: usize,
    pub exact_patch_lookup_events: usize,
    pub task_identity_routing_events: usize,
    pub repository_identity_routing_events: usize,
    pub capability_gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfHealingRunnerRequest {
    pub request: CoreRepairRequest,
    pub memory: RepairLearningMemory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfHealingRunnerResult {
    pub schema: String,
    pub request_sha256: String,
    pub memory_sha256: String,
    pub attempt: CoreRepairAttempt,
    pub original_source_write_events: usize,
    pub core_self_approval_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairLearningPromotionRequest {
    pub memory: RepairLearningMemory,
    pub lesson: RepairLessonIR,
    pub teaching: OperatorTeachingReceipt,
    pub transfers: Vec<FreshTransferReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairLearningPromotionResult {
    pub schema: String,
    pub promoted_lesson_sha256: String,
    pub composition_id: String,
    pub primitive_ids: Vec<String>,
    pub recombinations: usize,
    pub memory: RepairLearningMemory,
    pub exact_patch_lookup_events: usize,
    pub task_identity_routing_events: usize,
    pub repository_identity_routing_events: usize,
}

pub fn ingest_operator_teaching(
    request: RepairLearningPromotionRequest,
) -> Result<RepairLearningPromotionResult, String> {
    let promoted = promote_operator_lesson(request.lesson, &request.teaching, &request.transfers)
        .map_err(|error| format!("LESSON_PROMOTION:{error:?}"))?;
    if request
        .memory
        .promoted_lessons
        .iter()
        .any(|existing| existing.lesson.sha256 == promoted.lesson.sha256)
    {
        return Err("DUPLICATE_PROMOTED_LESSON".to_string());
    }
    let promoted_lesson_sha256 = promoted.lesson.sha256.clone();
    let composition_id = promoted
        .lesson
        .value
        .composition_lesson
        .composition_id
        .clone();
    let primitive_ids = promoted
        .lesson
        .value
        .composition_lesson
        .execution_order
        .clone();
    let recombinations = promoted.lesson.value.composition_lesson.edges.len();
    let mut memory = request.memory;
    memory.operator_teaching_events = memory.operator_teaching_events.saturating_add(1);
    memory.successful_fresh_transfer_events = memory
        .successful_fresh_transfer_events
        .saturating_add(request.transfers.len());
    memory.promoted_lessons.push(promoted);
    Ok(RepairLearningPromotionResult {
        schema: PIPELINE_SCHEMA.to_string(),
        promoted_lesson_sha256,
        composition_id,
        primitive_ids,
        recombinations,
        memory,
        exact_patch_lookup_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
    })
}

pub fn run_self_healing_request(
    runner_request: SelfHealingRunnerRequest,
) -> Result<SelfHealingRunnerResult, String> {
    let request_sha256 = sha256(
        &serde_json::to_vec(&runner_request.request)
            .map_err(|error| format!("REQUEST_SERIALIZE:{error}"))?,
    );
    let memory_sha256 = sha256(
        &serde_json::to_vec(&runner_request.memory)
            .map_err(|error| format!("MEMORY_SERIALIZE:{error}"))?,
    );
    let attempt = attempt_core_repair(&runner_request.request, &runner_request.memory);
    Ok(SelfHealingRunnerResult {
        schema: PIPELINE_SCHEMA.to_string(),
        request_sha256,
        memory_sha256,
        core_self_approval_events: attempt.core_self_approval_events,
        attempt,
        original_source_write_events: 0,
    })
}

pub fn attempt_core_repair(
    request: &CoreRepairRequest,
    memory: &RepairLearningMemory,
) -> CoreRepairAttempt {
    if request.attempt >= request.max_attempts.max(1) {
        return empty_attempt(
            RepairAttemptStatus::BoundedAttemptExhausted,
            "BOUNDED_CORE_ATTEMPT_BUDGET_EXHAUSTED",
        );
    }
    if !request.observation.integrity_valid()
        || !request.defect_contract.integrity_valid()
        || !request.repair_spec.integrity_valid()
    {
        return empty_attempt(
            RepairAttemptStatus::CapabilityGap,
            "FROZEN_INPUT_INTEGRITY_FAILURE",
        );
    }
    let Some(lesson) = memory
        .promoted_lessons
        .iter()
        .find(|candidate| candidate.lesson.value.defect_class == request.defect_class)
    else {
        return empty_attempt(
            RepairAttemptStatus::NoApplicableLesson,
            "NO_PROMOTED_GENERALIZED_LESSON",
        );
    };

    let transformed = match request.defect_class {
        DefectClass::ManualRemainderPredicate => {
            rewrite_manual_remainder_predicates(&request.source_text)
        }
        _ => None,
    };
    let Some((candidate_source, changed_lines)) = transformed else {
        return empty_attempt(
            RepairAttemptStatus::CapabilityGap,
            "LESSON_MATCHED_BUT_TRANSFORMATION_UNSUPPORTED_OR_NOT_APPLICABLE",
        );
    };
    let candidate_diff = sparse_line_diff(&request.source_text, &candidate_source);
    let patch_sha256 = sha256(candidate_diff.as_bytes());
    let patch_candidate = PatchCandidateIR {
        predecessor_tree_hash: request.predecessor_tree_hash.clone(),
        changed_files: vec![request.logical_file_id.clone()],
        changed_symbols: vec![format!("learned::{:?}", request.defect_class)],
        unified_diff_sha256: patch_sha256,
        repair_spec_sha256: request.repair_spec.sha256.clone(),
        consequence_predictions: request.repair_spec.value.expected_consequences.clone(),
        proposer_confidence_millis: 850,
        core_self_approved: false,
    };
    CoreRepairAttempt {
        status: RepairAttemptStatus::CandidateProposed,
        matched_lesson_sha256: Some(lesson.lesson.sha256.clone()),
        candidate_source: Some(candidate_source),
        candidate_diff: Some(candidate_diff),
        patch_candidate: Some(patch_candidate),
        changed_line_count: changed_lines,
        activated_file_count: 1,
        activated_composition_id: Some(
            lesson
                .lesson
                .value
                .composition_lesson
                .composition_id
                .clone(),
        ),
        activated_primitive_ids: lesson
            .lesson
            .value
            .composition_lesson
            .execution_order
            .clone(),
        primitive_recombinations: lesson.lesson.value.composition_lesson.edges.len(),
        core_self_approval_events: 0,
        exact_patch_lookup_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        capability_gap: None,
    }
}

fn empty_attempt(status: RepairAttemptStatus, reason: &str) -> CoreRepairAttempt {
    CoreRepairAttempt {
        status,
        matched_lesson_sha256: None,
        candidate_source: None,
        candidate_diff: None,
        patch_candidate: None,
        changed_line_count: 0,
        activated_file_count: 0,
        activated_composition_id: None,
        activated_primitive_ids: vec![],
        primitive_recombinations: 0,
        core_self_approval_events: 0,
        exact_patch_lookup_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        capability_gap: Some(reason.to_string()),
    }
}

/// Learned schema: `value % divisor == 0` becomes
/// `value.is_multiple_of(divisor)`; `!= 0` becomes its negation. Only a
/// same-line, zero-comparison form is handled. Ambiguous syntax is left alone.
fn rewrite_manual_remainder_predicates(source: &str) -> Option<(String, usize)> {
    let mut changed = 0;
    let mut rewritten = Vec::new();
    for line in source.split_inclusive('\n') {
        let mut current = line.to_string();
        while let Some(next) = rewrite_one_remainder_predicate(&current) {
            if next == current {
                break;
            }
            current = next;
            changed += 1;
        }
        rewritten.push(current);
    }
    if !source.ends_with('\n') && source.is_empty() {
        rewritten.push(String::new());
    }
    (changed > 0).then(|| (rewritten.concat(), changed))
}

fn rewrite_one_remainder_predicate(line: &str) -> Option<String> {
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
    let left_end = modulo;
    let left_start = expression_start(&line[..left_end])?;
    let expression = line[left_start..left_end].trim();
    if expression.is_empty() {
        return None;
    }
    let replacement = if negated {
        format!("!{expression}.is_multiple_of({divisor})")
    } else {
        format!("{expression}.is_multiple_of({divisor})")
    };
    let mut result = String::with_capacity(line.len() + 16);
    result.push_str(&line[..left_start]);
    result.push_str(&replacement);
    result.push_str(&line[right_start + comparison_len..]);
    Some(result)
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
            b'=' | b';' | b',' | b'{' | b'[' | b'!' | b'&' | b'|' if depth == 0 => {
                return Some(index + 1)
            }
            _ => {}
        }
    }
    (end > 0).then_some(0)
}

fn sparse_line_diff(before: &str, after: &str) -> String {
    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    let mut diff = String::from("--- before\n+++ after\n");
    for index in 0..before_lines.len().max(after_lines.len()) {
        let old = before_lines.get(index).copied().unwrap_or("");
        let new = after_lines.get(index).copied().unwrap_or("");
        if old != new {
            diff.push_str(&format!("@@ line {} @@\n-{old}\n+{new}\n", index + 1));
        }
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(target: &str, authority: SurfaceAuthority) -> ModuleSurface {
        ModuleSurface {
            package: "semantic-reasoning".into(),
            target: target.into(),
            target_kind: "lib".into(),
            source_path: format!("src/{target}.rs"),
            authority,
        }
    }

    fn probe(target: &str, status: ProbeStatus) -> HealthProbeReceipt {
        HealthProbeReceipt {
            surface_key: format!("semantic-reasoning::lib::{target}"),
            probe_kind: "cargo-test".into(),
            command_sha256: "command".into(),
            output_sha256: "output".into(),
            status,
            duration_ms: 1,
            bounded_timeout_ms: 100,
            externally_observed: true,
        }
    }

    fn lesson() -> RepairLessonIR {
        let primitives = vec![
            RepairPrimitiveIR {
                primitive_id: "PREDICATE_LOCATOR".into(),
                implementation_anchor: "self_healing_pipeline::rewrite_one_remainder_predicate"
                    .into(),
                input_type: "FrozenRepairContext".into(),
                output_type: "PredicateSpan".into(),
                semantic_role: "LOCALIZE".into(),
            },
            RepairPrimitiveIR {
                primitive_id: "EXPRESSION_BOUNDARY".into(),
                implementation_anchor: "self_healing_pipeline::expression_start".into(),
                input_type: "PredicateSpan".into(),
                output_type: "BoundedPredicate".into(),
                semantic_role: "PRESERVE_BOUNDARY".into(),
            },
            RepairPrimitiveIR {
                primitive_id: "DIVISIBILITY_REWRITE".into(),
                implementation_anchor: "self_healing_pipeline::rewrite_manual_remainder_predicates"
                    .into(),
                input_type: "BoundedPredicate".into(),
                output_type: "RewrittenSource".into(),
                semantic_role: "TRANSFORM".into(),
            },
            RepairPrimitiveIR {
                primitive_id: "SPARSE_DIFF".into(),
                implementation_anchor: "self_healing_pipeline::sparse_line_diff".into(),
                input_type: "RewrittenSource".into(),
                output_type: "SparseDiff".into(),
                semantic_role: "PACKAGE".into(),
            },
            RepairPrimitiveIR {
                primitive_id: "RSI_PATCH_CANDIDATE".into(),
                implementation_anchor: "self_repair_contract::PatchCandidateIR".into(),
                input_type: "SparseDiff".into(),
                output_type: "PatchCandidateIR".into(),
                semantic_role: "PREDICT".into(),
            },
        ];
        let edges = primitives
            .windows(2)
            .map(|pair| CompositionEdgeIR {
                from_primitive_id: pair[0].primitive_id.clone(),
                to_primitive_id: pair[1].primitive_id.clone(),
                transported_type: pair[0].output_type.clone(),
            })
            .collect::<Vec<_>>();
        RepairLessonIR {
            lesson_id: "lesson-manual-remainder-predicate-v1".into(),
            defect_class: DefectClass::ManualRemainderPredicate,
            diagnostic_cues: vec!["clippy::manual_is_multiple_of".into()],
            restored_invariants: vec!["canonical divisibility predicate".into()],
            transformation_schema: "x % n == 0 => x.is_multiple_of(n)".into(),
            applicability: vec!["integer remainder compared with zero".into()],
            non_applicability: vec!["non-zero remainder comparison".into()],
            regression_obligations: vec!["cargo test".into(), "clippy -D warnings".into()],
            composition_lesson: RepairCompositionLessonIR {
                composition_id: "COMPOSE_REMAINDER_CANONICALIZATION_V1".into(),
                execution_order: primitives
                    .iter()
                    .map(|primitive| primitive.primitive_id.clone())
                    .collect(),
                primitives,
                edges,
                required_semantic_roles: vec![
                    "LOCALIZE".into(),
                    "PRESERVE_BOUNDARY".into(),
                    "TRANSFORM".into(),
                    "PACKAGE".into(),
                    "PREDICT".into(),
                ],
                applicability: vec!["typed integer zero-remainder predicate".into()],
                non_applicability: vec!["non-zero remainder or overloaded operator".into()],
                exact_source_fragment_present: false,
            },
            exact_patch_data_present: false,
            exact_repository_identity_present: false,
            exact_task_identity_present: false,
        }
    }

    fn frozen_request(source: &str) -> CoreRepairRequest {
        CoreRepairRequest {
            scenario_sha256: sha256(source.as_bytes()),
            logical_file_id: "fresh/module.rs".into(),
            predecessor_tree_hash: "predecessor".into(),
            defect_class: DefectClass::ManualRemainderPredicate,
            observation: Frozen::new(ObservationIR {
                observed_event: "lint failure".into(),
                trigger: "clippy".into(),
                expected_observable: "canonical divisibility predicate".into(),
                actual_observable: "manual remainder predicate".into(),
                evidence: vec!["manual_is_multiple_of".into()],
                provenance: vec!["fresh probe".into()],
                contains_hidden_diagnosis: false,
            })
            .expect("observation"),
            defect_contract: Frozen::new(DefectContractIR {
                affected_behavior: "integer predicate".into(),
                violated_invariant: "warning-free canonical form".into(),
                scope: "one activated file".into(),
                trigger_conditions: vec!["zero remainder comparison".into()],
                expected_vs_observed: "canonical vs manual".into(),
                causal_evidence: vec!["clippy".into()],
                uncertainty: "bounded".into(),
                suspected_mechanism_classes: vec!["lint modernization".into()],
                affected_interfaces: vec!["integer predicate".into()],
                preserved_behavior: vec!["truth table".into()],
                provenance: vec!["fresh probe".into()],
                prescribes_concrete_edit: false,
            })
            .expect("contract"),
            repair_spec: Frozen::new(RepairSpecIR {
                required_postcondition: "warning absent".into(),
                restored_invariants: vec!["truth table preserved".into()],
                allowed_semantic_changes: vec!["canonical syntax only".into()],
                forbidden_semantic_changes: vec!["test changes".into()],
                compatibility_requirements: vec!["same boolean result".into()],
                resource_constraints: vec!["one file".into()],
                expected_consequences: vec!["clippy warning removed".into()],
                rollback_conditions: vec!["regression".into()],
                verification_requirements: vec!["truth table".into()],
                applicability: vec!["zero remainder comparison".into()],
                uncertainty: "bounded".into(),
                encodes_exact_patch: false,
            })
            .expect("spec"),
            source_text: source.into(),
            attempt: 0,
            max_attempts: DEFAULT_MAX_CORE_ATTEMPTS,
        }
    }

    fn candidate(patch_hash: &str, spec_hash: &str) -> PatchCandidateIR {
        PatchCandidateIR {
            predecessor_tree_hash: "predecessor".into(),
            changed_files: vec!["isolated.rs".into()],
            changed_symbols: vec!["predicate".into()],
            unified_diff_sha256: patch_hash.into(),
            repair_spec_sha256: spec_hash.into(),
            consequence_predictions: vec!["warning absent".into()],
            proposer_confidence_millis: 900,
            core_self_approved: false,
        }
    }

    fn verification(
        patch: &PatchCandidateIR,
        defect_contract_hash: &str,
        scenario: &str,
    ) -> VerificationReceipt {
        VerificationReceipt {
            patch_sha256: patch.unified_diff_sha256.clone(),
            repair_spec_sha256: patch.repair_spec_sha256.clone(),
            defect_contract_sha256: defect_contract_hash.into(),
            semantic_checks_sha256: format!("semantic-{scenario}"),
            regression_checks_sha256: format!("regression-{scenario}"),
            resource_checks_sha256: format!("resource-{scenario}"),
            invariant_checks_sha256: format!("invariant-{scenario}"),
            decision: VerificationDecision::Accept,
            verifier_identity: "independent-verifier".into(),
            verifier_is_proposer: false,
            gold_patch_text_equality_is_authority: false,
            receipt_sha256: format!("receipt-{scenario}"),
            authority_seal: format!("seal-{scenario}"),
        }
    }

    fn promoted_memory() -> RepairLearningMemory {
        let request = frozen_request("fn teaching(x: usize) -> bool { x % 2 == 0 }\n");
        let teaching_patch = candidate("operator-patch", &request.repair_spec.sha256);
        let teaching = OperatorTeachingReceipt {
            operator_identity: "codex-operator-teacher".into(),
            scenario_sha256: "teaching-scenario".into(),
            defect_contract_sha256: request.defect_contract.sha256.clone(),
            verification_receipt: verification(
                &teaching_patch,
                &request.defect_contract.sha256,
                "teaching",
            ),
            patch_candidate: teaching_patch,
            operator_patch_installed_in_isolation: true,
            post_install_regression_passed: true,
        };
        let transfers = ["fresh-a", "fresh-b"]
            .into_iter()
            .map(|scenario| {
                let patch = candidate(&format!("patch-{scenario}"), &request.repair_spec.sha256);
                FreshTransferReceipt {
                    scenario_sha256: scenario.into(),
                    source_shape_sha256: format!("shape-{scenario}"),
                    core_generated: true,
                    verification_receipt: verification(
                        &patch,
                        &request.defect_contract.sha256,
                        scenario,
                    ),
                    patch_candidate: patch,
                    regression_passed: true,
                }
            })
            .collect::<Vec<_>>();
        RepairLearningMemory {
            promoted_lessons: vec![
                promote_operator_lesson(lesson(), &teaching, &transfers).expect("promote lesson")
            ],
            rejected_lesson_sha256: vec![],
            operator_teaching_events: 1,
            successful_fresh_transfer_events: transfers.len(),
        }
    }

    #[test]
    fn compiled_coverage_does_not_count_quarantined_or_missing_as_pass() {
        let inventory = vec![
            surface("compiled-a", SurfaceAuthority::Compiled),
            surface("compiled-b", SurfaceAuthority::Compiled),
            surface("legacy", SurfaceAuthority::Quarantined),
        ];
        let summary = summarize_module_audit(&inventory, &[probe("compiled-a", ProbeStatus::Pass)]);
        assert_eq!(summary.compiled_surface_count, 2);
        assert_eq!(summary.quarantined_surface_count, 1);
        assert!(!summary.compiled_coverage_complete);
        assert!(!summary.all_compiled_probes_pass);
        assert_eq!(summary.missing_compiled_surface_keys.len(), 1);
    }

    #[test]
    fn compiler_ice_is_not_mislearned_as_a_source_defect() {
        let output = "error: internal compiler error: Res::Err; query stack during panic";
        assert_eq!(
            classify_failure_origin(output, ProbeStatus::Fail),
            FailureOrigin::ToolchainTransient
        );
        assert_eq!(
            classify_failure_origin("error[E0308]: mismatched types", ProbeStatus::Fail),
            FailureOrigin::SourceDefect
        );
    }

    #[test]
    fn core_records_capability_gap_before_operator_teaching() {
        let request = frozen_request("fn fresh(x: usize) -> bool { x % 7 == 0 }\n");
        let result = attempt_core_repair(&request, &RepairLearningMemory::default());
        assert_eq!(result.status, RepairAttemptStatus::NoApplicableLesson);
        assert_eq!(
            result.capability_gap.as_deref(),
            Some("NO_PROMOTED_GENERALIZED_LESSON")
        );
        assert!(result.patch_candidate.is_none());
    }

    #[test]
    fn exact_operator_patch_cannot_be_promoted_as_learning() {
        let mut contaminated = lesson();
        contaminated.exact_patch_data_present = true;
        let request = frozen_request("fn teaching(x: usize) -> bool { x % 2 == 0 }\n");
        let patch = candidate("operator-patch", &request.repair_spec.sha256);
        let teaching = OperatorTeachingReceipt {
            operator_identity: "operator".into(),
            scenario_sha256: "teaching".into(),
            defect_contract_sha256: request.defect_contract.sha256.clone(),
            verification_receipt: verification(&patch, &request.defect_contract.sha256, "teaching"),
            patch_candidate: patch,
            operator_patch_installed_in_isolation: true,
            post_install_regression_passed: true,
        };
        assert_eq!(
            promote_operator_lesson(contaminated, &teaching, &[]),
            Err(LessonPromotionError::LessonContainsExactSolutionAuthority)
        );
    }

    #[test]
    fn learned_rule_repairs_fresh_shapes_without_identity_routing() {
        let memory = promoted_memory();
        let cases = [
            (
                "fn alpha(sorted: &[u8]) -> bool { sorted.len() % 2 == 0 }\n",
                "sorted.len().is_multiple_of(2)",
            ),
            (
                "fn beta(a: usize, b: usize) -> bool { (a + b) % 3 != 0 }\n",
                "!(a + b).is_multiple_of(3)",
            ),
            (
                "fn gamma(rng: &mut Rng) -> bool { rng.next_u64() % 5 == 0 }\n",
                "rng.next_u64().is_multiple_of(5)",
            ),
        ];
        for (source, expected) in cases {
            let result = attempt_core_repair(&frozen_request(source), &memory);
            assert_eq!(result.status, RepairAttemptStatus::CandidateProposed);
            assert!(result
                .candidate_source
                .as_deref()
                .is_some_and(|candidate| candidate.contains(expected)));
            assert_eq!(result.core_self_approval_events, 0);
            assert_eq!(result.exact_patch_lookup_events, 0);
            assert_eq!(result.task_identity_routing_events, 0);
            assert_eq!(result.repository_identity_routing_events, 0);
            assert_eq!(
                result.activated_composition_id.as_deref(),
                Some("COMPOSE_REMAINDER_CANONICALIZATION_V1")
            );
            assert_eq!(result.activated_primitive_ids.len(), 5);
            assert_eq!(result.primitive_recombinations, 4);
        }
    }

    #[test]
    fn composition_lesson_is_typed_and_rejects_broken_transport() {
        let valid = lesson().composition_lesson;
        assert_eq!(validate_composition_lesson(&valid), Ok(()));
        let mut broken = valid;
        broken.edges[1].transported_type = "WrongType".into();
        assert_eq!(
            validate_composition_lesson(&broken),
            Err(CompositionValidationError::TypeTransportMismatch)
        );
    }

    #[test]
    fn learned_rule_abstains_outside_its_applicability() {
        let memory = promoted_memory();
        let source = "fn remainder(x: usize) -> bool { x % 4 == 1 }\n";
        let result = attempt_core_repair(&frozen_request(source), &memory);
        assert_eq!(result.status, RepairAttemptStatus::CapabilityGap);
        assert_eq!(result.candidate_source, None);
    }

    #[test]
    fn runner_emits_proposal_without_original_write_or_self_approval() {
        let request = frozen_request("fn delta(x: usize) -> bool { x % 11 == 0 }\n");
        let result = run_self_healing_request(SelfHealingRunnerRequest {
            request,
            memory: promoted_memory(),
        })
        .expect("runner");
        assert_eq!(result.schema, PIPELINE_SCHEMA);
        assert_eq!(
            result.attempt.status,
            RepairAttemptStatus::CandidateProposed
        );
        assert_eq!(result.original_source_write_events, 0);
        assert_eq!(result.core_self_approval_events, 0);
        assert!(!result.request_sha256.is_empty());
        assert!(!result.memory_sha256.is_empty());
    }

    #[test]
    fn operator_teaching_ingestion_persists_the_typed_composition_recipe() {
        let existing = promoted_memory();
        let promoted = existing.promoted_lessons[0].clone();
        let request = frozen_request("fn teaching(x: usize) -> bool { x % 2 == 0 }\n");
        let patch = candidate("second-operator-patch", &request.repair_spec.sha256);
        let teaching = OperatorTeachingReceipt {
            operator_identity: "codex-operator-teacher".into(),
            scenario_sha256: "second-teaching-scenario".into(),
            defect_contract_sha256: request.defect_contract.sha256.clone(),
            verification_receipt: verification(
                &patch,
                &request.defect_contract.sha256,
                "second-teaching",
            ),
            patch_candidate: patch,
            operator_patch_installed_in_isolation: true,
            post_install_regression_passed: true,
        };
        let transfers = ["second-fresh-a", "second-fresh-b"]
            .into_iter()
            .map(|scenario| {
                let patch = candidate(
                    &format!("second-patch-{scenario}"),
                    &request.repair_spec.sha256,
                );
                FreshTransferReceipt {
                    scenario_sha256: scenario.into(),
                    source_shape_sha256: format!("second-shape-{scenario}"),
                    core_generated: true,
                    verification_receipt: verification(
                        &patch,
                        &request.defect_contract.sha256,
                        scenario,
                    ),
                    patch_candidate: patch,
                    regression_passed: true,
                }
            })
            .collect::<Vec<_>>();
        let mut second_lesson = promoted.lesson.value;
        second_lesson.lesson_id = "lesson-manual-remainder-predicate-v2".into();
        second_lesson.composition_lesson.composition_id =
            "COMPOSE_REMAINDER_CANONICALIZATION_V2".into();
        let result = ingest_operator_teaching(RepairLearningPromotionRequest {
            memory: RepairLearningMemory::default(),
            lesson: second_lesson,
            teaching,
            transfers,
        })
        .expect("teaching ingestion");
        assert_eq!(result.memory.operator_teaching_events, 1);
        assert_eq!(result.memory.successful_fresh_transfer_events, 2);
        assert_eq!(result.primitive_ids.len(), 5);
        assert_eq!(result.recombinations, 4);
        assert_eq!(result.exact_patch_lookup_events, 0);
    }
}
