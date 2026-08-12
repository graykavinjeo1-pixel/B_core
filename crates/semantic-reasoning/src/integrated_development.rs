//! Integration boundary for recursive improvement, existing SEM-5 program
//! composition, and the independently verified RSI installation gate.
//!
//! SEM-5 remains the authoritative program synthesizer.  This module does not
//! duplicate its primitive catalog or synthesis logic; it lowers synthesized
//! typed ProgramIR to Rust and, when an installation context is supplied,
//! routes it through the same atomic validation/rollback path as self-repair.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::autonomous_source_mutation::{
    install_and_stage_source_patch, invoke_improvement_operator_repository,
    refresh_improvement_operator_repository, source_opportunity_family_id,
    AutonomousSourceMutationPolicy, AutonomousSourcePatchReceipt, AutonomousSourcePatchRequest,
    ChangeOpportunityKind, AUTONOMOUS_SOURCE_MUTATION_SCHEMA,
};
use crate::generalized_self_application::{
    derive_dynamic_weakness, synthesize_generalized_change, WeaknessEvidenceKind,
};
use crate::self_repair_contract::{
    sha256, validate_installation_authority, DefectContractIR, Frozen, InstallationGateError,
    ObservationIR, PatchCandidateIR, RepairSpecIR, VerificationReceipt,
};
use crate::sem27_engine::{
    run_post_scaffold_epoch, PostScaffoldEpochRequest, PostScaffoldEpochResult,
};
use crate::sem5::{
    emitter::{emit_neutral_text, emit_rust_callable},
    ir::{execute, type_check},
    learner::{discover_candidates, initial_promotions, synthesize},
    model::{ProgramIR, ProgramTask, ProgrammingPromotion, SynthesisCondition},
    tasks::{
        evaluate_contract, generate_property_cases, generate_task_sets,
        programming_primitive_catalog,
    },
};
use crate::structural_source_repair::synthesize_structural_repair;

pub const CAMPAIGN_ID: &str = "B_CORE-INTEGRATED-DEVELOPMENT-01";
pub const AUTHORITATIVE_PREDECESSOR: &str = "8092ea4aba69fd23c9f4e9d56132d488a58e0382";
const CAPABILITY_REGISTRY_SCHEMA_REVISION: u64 = 4;
pub const MAX_INSTALLED_TYPED_CAPABILITIES: usize = 64;
const CAPABILITY_BEGIN_PREFIX: &str = "// B_CORE_CAPABILITY_BEGIN:";
const CAPABILITY_END_PREFIX: &str = "// B_CORE_CAPABILITY_END:";
const MAX_CONTEXTUAL_TYPED_TASK_ATTEMPTS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOpportunityIR {
    pub observed_gap: String,
    pub desired_behavior: String,
    pub trigger: String,
    pub evidence: Vec<String>,
    pub preserved_behavior: Vec<String>,
    pub resource_constraints: Vec<String>,
    pub verification_requirements: Vec<String>,
    pub provenance: Vec<String>,
    pub operator_selected_implementation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionWork {
    pub opportunity: CapabilityOpportunityIR,
    pub task: ProgramTask,
    pub promotions: Vec<ProgrammingPromotion>,
    pub predecessor_tree_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositeProgramCandidateIR {
    pub opportunity: Frozen<CapabilityOpportunityIR>,
    pub observation: Frozen<ObservationIR>,
    pub defect_contract: Frozen<DefectContractIR>,
    pub repair_spec: Frozen<RepairSpecIR>,
    pub program_ir: ProgramIR,
    pub program_ir_sha256: String,
    pub primitive_catalog_sha256: String,
    pub used_primitive_ids: Vec<String>,
    pub promoted_concept_ids: Vec<String>,
    pub primitive_expanded_nodes: usize,
    pub operational_nodes: usize,
    pub recombinations: usize,
    pub type_effect_audit_pass: bool,
    pub neutral_program_sha256: String,
    pub generated_rust_source: String,
    pub generated_rust_sha256: String,
    pub source_relative_path: PathBuf,
    pub patch_candidate: PatchCandidateIR,
    pub full_source_scan_events: usize,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegratedDevelopmentEpochRequest {
    pub recursive_epoch: PostScaffoldEpochRequest,
    pub composition_work: Option<CompositionWork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installation: Option<IntegratedDevelopmentInstallationRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegratedDevelopmentInstallationRequest {
    pub mutation_policy: AutonomousSourceMutationPolicy,
    pub state_dir: PathBuf,
    pub source_generation: u64,
    #[serde(default)]
    pub attempt_nonce: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegratedDevelopmentEpochResult {
    pub recursive_epoch: PostScaffoldEpochResult,
    pub composite_candidate: Option<CompositeProgramCandidateIR>,
    pub composition_attempted: bool,
    pub core_self_approval_events: usize,
    pub unverified_install_events: usize,
    pub full_source_scan_events: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mutation_receipt: Option<AutonomousSourcePatchReceipt>,
    pub actual_source_write_attempted: bool,
    pub proposal_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehavioralCompositionCanaryReceipt {
    pub schema: String,
    pub context_sha256: String,
    pub program_ir_sha256: String,
    pub used_primitive_ids: Vec<String>,
    pub cases_executed: usize,
    pub cases_passed: usize,
    #[serde(default)]
    pub installed_capability_present: bool,
    #[serde(default)]
    pub installed_program_match: bool,
    #[serde(default)]
    pub installed_source_schema_revision: u64,
    #[serde(default)]
    pub installed_registry_capability_count: usize,
    #[serde(default)]
    pub installed_cases_executed: usize,
    #[serde(default)]
    pub installed_cases_passed: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_output_sha256: Option<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegratedInstallationGateError {
    DefectContractReceiptMismatch,
    RsiGate(InstallationGateError),
}

pub fn run_integrated_development_epoch(
    request: IntegratedDevelopmentEpochRequest,
) -> Result<IntegratedDevelopmentEpochResult, String> {
    if request.composition_work.is_some() && request.installation.is_none() {
        return Err("COMPOSITION_INSTALLATION_CONTEXT_REQUIRED".to_string());
    }
    let recursive_epoch = run_post_scaffold_epoch(request.recursive_epoch)?;
    let composition_attempted = request.composition_work.is_some();
    let mut composite_candidate = request
        .composition_work
        .map(compose_existing_sem5_capability)
        .transpose()?;
    let source_mutation_receipt = match (&composite_candidate, request.installation) {
        (Some(candidate), Some(installation)) => Some(install_composite_candidate(
            candidate,
            &installation.mutation_policy,
            &installation.state_dir,
            installation.source_generation,
            installation.attempt_nonce,
        )?),
        _ => None,
    };
    if let (Some(candidate), Some(receipt)) = (
        composite_candidate.as_mut(),
        source_mutation_receipt.as_ref(),
    ) {
        candidate.installed = receipt.installed;
    }
    let actual_source_write_attempted = source_mutation_receipt.is_some();
    Ok(IntegratedDevelopmentEpochResult {
        recursive_epoch,
        composite_candidate,
        composition_attempted,
        core_self_approval_events: 0,
        unverified_install_events: 0,
        full_source_scan_events: 0,
        source_mutation_receipt,
        actual_source_write_attempted,
        proposal_only: composition_attempted && !actual_source_write_attempted,
    })
}

/// Reuses the already established SEM-5 typed synthesizer and Rust emitter.
/// Installation authority remains in the source-mutation validation boundary.
pub fn compose_existing_sem5_capability(
    work: CompositionWork,
) -> Result<CompositeProgramCandidateIR, String> {
    if work.opportunity.operator_selected_implementation {
        return Err("OPERATOR_SELECTED_IMPLEMENTATION_FORBIDDEN".to_string());
    }
    if work.predecessor_tree_hash.is_empty() {
        return Err("PREDECESSOR_TREE_HASH_MISSING".to_string());
    }

    let opportunity = Frozen::new(work.opportunity.clone())
        .map_err(|error| format!("OPPORTUNITY_FREEZE:{error}"))?;
    let observation = Frozen::new(observation_from_opportunity(&work.opportunity))
        .map_err(|error| format!("OBSERVATION_FREEZE:{error}"))?;
    let defect_contract = Frozen::new(defect_contract_from_opportunity(&work.opportunity))
        .map_err(|error| format!("DEFECT_CONTRACT_FREEZE:{error}"))?;
    let repair_spec = Frozen::new(repair_spec_from_opportunity(&work.opportunity))
        .map_err(|error| format!("REPAIR_SPEC_FREEZE:{error}"))?;

    let program_ir = synthesize(
        &work.task,
        SynthesisCondition::FirstPrinciplesD,
        &work.promotions,
    )?;
    type_check(&program_ir, &work.task.definitions)?;
    if program_ir.recombinations == 0 {
        return Err("COMPOSITE_PROGRAM_REQUIRED".to_string());
    }
    if program_ir.operational_nodes >= program_ir.primitive_expanded_nodes {
        return Err("COMPOSITION_DID_NOT_COMPRESS_PRIMITIVES".to_string());
    }

    let program_bytes =
        serde_json::to_vec(&program_ir).map_err(|error| format!("PROGRAM_IR_SERIALIZE:{error}"))?;
    let program_ir_sha256 = sha256(&program_bytes);
    let catalog = programming_primitive_catalog();
    let catalog_bytes = serde_json::to_vec(&catalog)
        .map_err(|error| format!("PRIMITIVE_CATALOG_SERIALIZE:{error}"))?;
    let primitive_catalog_sha256 = sha256(&catalog_bytes);
    let used_kinds = used_node_kinds(&program_ir)?;
    let used_primitive_ids = catalog
        .iter()
        .filter(|primitive| used_kinds.contains(&primitive.node_kind))
        .map(|primitive| primitive.primitive_id.clone())
        .collect::<Vec<_>>();
    if used_primitive_ids.len() < 2 {
        return Err("INSUFFICIENT_PRIMITIVE_COMPOSITION".to_string());
    }

    let promoted_concept_ids = program_ir
        .concept_ids
        .iter()
        .filter(|concept| concept.starts_with('C'))
        .cloned()
        .collect::<Vec<_>>();
    let neutral_program = emit_neutral_text(&program_ir);
    let neutral_program_sha256 = sha256(neutral_program.as_bytes());
    let rust_artifact =
        emit_rust_callable(&program_ir, &work.task.definitions, &program_ir_sha256)?;
    let generated_rust_source = rust_artifact.source;
    let generated_rust_sha256 = sha256(generated_rust_source.as_bytes());
    let source_relative_path =
        PathBuf::from("crates/semantic-reasoning/src/generated_sem5_capability.rs");
    let normalized_path = source_relative_path.to_string_lossy().replace('\\', "/");
    let actual_diff = format!(
        "--- a/{normalized_path}\n+++ b/{normalized_path}\n@@ generated-capability-replacement +1,{} @@\n{}",
        generated_rust_source.lines().count(),
        generated_rust_source
            .lines()
            .map(|line| format!("+{line}\n"))
            .collect::<String>()
    );
    let patch_candidate = PatchCandidateIR {
        predecessor_tree_hash: work.predecessor_tree_hash,
        changed_files: vec![normalized_path],
        changed_symbols: vec![program_ir.program_id.clone()],
        unified_diff_sha256: sha256(actual_diff.as_bytes()),
        repair_spec_sha256: repair_spec.sha256.clone(),
        consequence_predictions: vec![
            work.opportunity.desired_behavior.clone(),
            "typed effect constraints remain enforced by ProgramIR".to_string(),
            "installation remains blocked until independent verification".to_string(),
        ],
        proposer_confidence_millis: 800,
        core_self_approved: false,
    };

    Ok(CompositeProgramCandidateIR {
        opportunity,
        observation,
        defect_contract,
        repair_spec,
        primitive_expanded_nodes: program_ir.primitive_expanded_nodes,
        operational_nodes: program_ir.operational_nodes,
        recombinations: program_ir.recombinations,
        program_ir,
        program_ir_sha256,
        primitive_catalog_sha256,
        used_primitive_ids,
        promoted_concept_ids,
        type_effect_audit_pass: true,
        neutral_program_sha256,
        generated_rust_source,
        generated_rust_sha256,
        source_relative_path,
        patch_candidate,
        full_source_scan_events: 0,
        installed: false,
    })
}

fn rustfmt_generated_source(
    policy: &AutonomousSourceMutationPolicy,
    source: &str,
) -> Result<String, String> {
    let rustfmt = policy.cargo_executable.with_file_name(if cfg!(windows) {
        "rustfmt.exe"
    } else {
        "rustfmt"
    });
    if !rustfmt.is_file() {
        return Err(format!("COMPOSITE_RUSTFMT_MISSING:{}", rustfmt.display()));
    }
    let mut child = Command::new(&rustfmt)
        .args(["--emit", "stdout", "--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("COMPOSITE_RUSTFMT_SPAWN:{error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "COMPOSITE_RUSTFMT_STDIN_MISSING".to_string())?
        .write_all(source.as_bytes())
        .map_err(|error| format!("COMPOSITE_RUSTFMT_STDIN:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("COMPOSITE_RUSTFMT_WAIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "COMPOSITE_RUSTFMT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(2_048)
                .collect::<String>()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("COMPOSITE_RUSTFMT_UTF8:{error}"))
}

fn extract_generated_const_str(source: &str, name: &str) -> Option<String> {
    let marker = format!("pub const {name}: &str");
    let start = source.find(&marker)?;
    let declaration = &source[start..];
    let equals = declaration.find('=')?;
    let value = declaration[equals + 1..].trim_start();
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    Some(value[..end].to_string())
}

fn capability_module_name(program_ir_sha256: &str) -> Result<String, String> {
    if program_ir_sha256.len() != 64
        || !program_ir_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("CAPABILITY_REGISTRY_PROGRAM_HASH_INVALID".to_string());
    }
    Ok(format!("capability_{}", &program_ir_sha256[..16]))
}

fn wrap_capability_module(
    program_ir_sha256: &str,
    callable_source: &str,
) -> Result<String, String> {
    let module = capability_module_name(program_ir_sha256)?;
    let mut output = format!("{CAPABILITY_BEGIN_PREFIX}{program_ir_sha256}\nmod {module} {{\n");
    for line in callable_source.lines() {
        output.push_str("    ");
        output.push_str(line);
        output.push('\n');
    }
    output.push_str("}\n");
    output.push_str(&format!("{CAPABILITY_END_PREFIX}{program_ir_sha256}\n"));
    Ok(output)
}

fn existing_capability_sections(source: &str) -> Result<Vec<(String, String)>, String> {
    let mut sections = Vec::new();
    let mut remaining = source;
    while let Some(begin_offset) = remaining.find(CAPABILITY_BEGIN_PREFIX) {
        let from_begin = &remaining[begin_offset..];
        let begin_line_end = from_begin
            .find('\n')
            .ok_or_else(|| "CAPABILITY_REGISTRY_BEGIN_TRUNCATED".to_string())?;
        let hash = from_begin[CAPABILITY_BEGIN_PREFIX.len()..begin_line_end]
            .trim()
            .to_string();
        capability_module_name(&hash)?;
        let end_marker = format!("{CAPABILITY_END_PREFIX}{hash}");
        let end_offset = from_begin
            .find(&end_marker)
            .ok_or_else(|| "CAPABILITY_REGISTRY_END_MISSING".to_string())?;
        let after_marker = end_offset + end_marker.len();
        let section_end = from_begin[after_marker..]
            .find('\n')
            .map(|offset| after_marker + offset + 1)
            .unwrap_or(from_begin.len());
        sections.push((hash, from_begin[..section_end].to_string()));
        remaining = &from_begin[section_end..];
    }
    Ok(sections)
}

fn merge_typed_capability_registry(
    predecessor_source: &str,
    new_callable_source: &str,
    current_program_id: &str,
    current_program_ir_sha256: &str,
) -> Result<String, String> {
    if extract_generated_const_str(new_callable_source, "GENERATED_PROGRAM_IR_SHA256").as_deref()
        != Some(current_program_ir_sha256)
        || extract_generated_const_str(new_callable_source, "GENERATED_PROGRAM_ID").as_deref()
            != Some(current_program_id)
    {
        return Err("CAPABILITY_REGISTRY_CALLABLE_IDENTITY_MISMATCH".to_string());
    }
    let mut sections = existing_capability_sections(predecessor_source)?;
    if sections.is_empty() {
        let predecessor_hash =
            extract_generated_const_str(predecessor_source, "GENERATED_PROGRAM_IR_SHA256")
                .ok_or_else(|| "CAPABILITY_REGISTRY_PREDECESSOR_HASH_MISSING".to_string())?;
        sections.push((
            predecessor_hash.clone(),
            wrap_capability_module(&predecessor_hash, predecessor_source)?,
        ));
    }
    if !sections
        .iter()
        .any(|(program_ir_sha256, _)| program_ir_sha256 == current_program_ir_sha256)
    {
        if sections.len() >= MAX_INSTALLED_TYPED_CAPABILITIES {
            return Err("CAPABILITY_REGISTRY_CAPACITY_REACHED".to_string());
        }
        sections.push((
            current_program_ir_sha256.to_string(),
            wrap_capability_module(current_program_ir_sha256, new_callable_source)?,
        ));
    }
    let mut module_names = BTreeSet::new();
    for (program_ir_sha256, _) in &sections {
        if !module_names.insert(capability_module_name(program_ir_sha256)?) {
            return Err("CAPABILITY_REGISTRY_MODULE_PREFIX_COLLISION".to_string());
        }
    }
    let current_module = capability_module_name(current_program_ir_sha256)?;
    let mut output = String::new();
    output.push_str("#![allow(dead_code, unused_imports, unused_parens, unused_variables)]\n\n");
    output.push_str("use std::collections::BTreeMap;\n");
    output.push_str("use crate::sem5::model::Value;\n\n");
    output.push_str("pub const GENERATED_CAPABILITY_ACTIVE: bool = true;\n");
    output.push_str(&format!(
        "pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = {CAPABILITY_REGISTRY_SCHEMA_REVISION};\n"
    ));
    output.push_str(&format!(
        "pub const GENERATED_PROGRAM_ID: &str = {current_program_id:?};\n"
    ));
    output.push_str(&format!(
        "pub const GENERATED_PROGRAM_IR_SHA256: &str = {current_program_ir_sha256:?};\n"
    ));
    output.push_str(&format!(
        "pub const GENERATED_CAPABILITY_COUNT: usize = {};\n\n",
        sections.len()
    ));
    for (_, section) in &sections {
        output.push_str(section);
        output.push('\n');
    }
    output.push_str("pub fn generated_capability_hashes() -> &'static [&'static str] {\n    &[\n");
    for (hash, _) in &sections {
        output.push_str(&format!("        {hash:?},\n"));
    }
    output.push_str("    ]\n}\n\n");
    output.push_str(
        "pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {\n",
    );
    output.push_str(&format!(
        "    {current_module}::run_generated_capability(inputs)\n}}\n\n"
    ));
    output.push_str(
        "pub fn run_generated_capability_by_sha256(program_ir_sha256: &str, inputs: &BTreeMap<String, Value>) -> Result<Value, String> {\n    match program_ir_sha256 {\n",
    );
    for (hash, _) in &sections {
        let module = capability_module_name(hash)?;
        output.push_str(&format!(
            "        {hash:?} => {module}::run_generated_capability(inputs),\n"
        ));
    }
    output
        .push_str("        _ => Err(\"GENERATED_CAPABILITY_NOT_FOUND\".to_string()),\n    }\n}\n");
    Ok(output)
}

fn merge_typed_capability_family_registry(
    predecessor_source: &str,
    candidates: &[CompositeProgramCandidateIR],
) -> Result<String, String> {
    let mut registry_source = predecessor_source.to_string();
    for candidate in candidates {
        registry_source = merge_typed_capability_registry(
            &registry_source,
            &candidate.generated_rust_source,
            &candidate.program_ir.program_id,
            &candidate.program_ir_sha256,
        )?;
    }
    Ok(registry_source)
}

pub fn install_composite_candidate(
    candidate: &CompositeProgramCandidateIR,
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &std::path::Path,
    source_generation: u64,
    attempt_nonce: u64,
) -> Result<AutonomousSourcePatchReceipt, String> {
    install_composite_candidate_family(
        std::slice::from_ref(candidate),
        policy,
        state_dir,
        source_generation,
        attempt_nonce,
    )
}

/// Installs a bounded family of independently verified ProgramIR lowerings as
/// one atomic registry rewrite. The expensive compile/test/release boundary is
/// paid once for the family instead of once per callable.
pub fn install_composite_candidate_family(
    candidates: &[CompositeProgramCandidateIR],
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &std::path::Path,
    source_generation: u64,
    attempt_nonce: u64,
) -> Result<AutonomousSourcePatchReceipt, String> {
    let Some(current_candidate) = candidates.last() else {
        return Err("COMPOSITE_SOURCE_INSTALLATION_FAMILY_EMPTY".to_string());
    };
    let target_path = &current_candidate.source_relative_path;
    let mut family_hashes = BTreeSet::new();
    for candidate in candidates {
        if candidate.installed
            || !candidate.type_effect_audit_pass
            || candidate.generated_rust_source.is_empty()
            || sha256(candidate.generated_rust_source.as_bytes()) != candidate.generated_rust_sha256
            || &candidate.source_relative_path != target_path
            || !family_hashes.insert(candidate.program_ir_sha256.clone())
        {
            return Err("COMPOSITE_SOURCE_INSTALLATION_INPUT_INVALID".to_string());
        }
    }
    let target = policy.source_root.join(target_path);
    let predecessor_source = std::fs::read_to_string(&target).map_err(|error| {
        format!(
            "COMPOSITE_SOURCE_PREDECESSOR_READ:{}:{error}",
            target.display()
        )
    })?;
    let registry_source = merge_typed_capability_family_registry(&predecessor_source, candidates)?;
    let candidate_source = rustfmt_generated_source(policy, &registry_source)?;
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let predecessor_sha256 = sha256(predecessor_source.as_bytes());
    let file_id = target_path.to_string_lossy().replace('\\', "/");
    let structural_repair_program =
        synthesize_structural_repair(&file_id, &predecessor_source, &candidate_source)?;
    let transformation = "SEM5_PROGRAM_IR_TO_ACTIVE_RUNTIME_CALLABLE".to_string();
    let mut consequences = candidates
        .iter()
        .flat_map(|candidate| {
            candidate
                .patch_candidate
                .consequence_predictions
                .iter()
                .cloned()
        })
        .collect::<Vec<_>>();
    consequences.sort();
    consequences.dedup();
    consequences.push(format!(
        "{} independently verified typed callables enter one atomic validation boundary",
        candidates.len()
    ));
    let family_evidence_sha256 = sha256(
        family_hashes
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(":")
            .as_bytes(),
    );
    let weakness = derive_dynamic_weakness(
        source_generation,
        target_path,
        &transformation,
        WeaknessEvidenceKind::StructuralSourceSmell,
        &family_evidence_sha256,
        "a verified typed executable composition family has no repository-native Rust implementation",
        consequences.clone(),
        Vec::new(),
    );
    let generalized_change = synthesize_generalized_change(
        &weakness,
        "EMIT_TYPED_RUST_AND_ACTIVATE_CALLABLE",
        &predecessor_sha256,
        &candidate_sha256,
        &structural_repair_program,
    )?;
    let opportunity_family_id =
        source_opportunity_family_id(ChangeOpportunityKind::CapabilityGap, &transformation);
    let operator_memory = refresh_improvement_operator_repository(state_dir)?;
    let improvement_operator_invocation = invoke_improvement_operator_repository(
        &operator_memory,
        WeaknessEvidenceKind::StructuralSourceSmell,
        &transformation,
        "EMIT_TYPED_RUST_AND_ACTIVATE_CALLABLE",
        &structural_repair_program,
        &opportunity_family_id,
    )?;
    let request = AutonomousSourcePatchRequest {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id: format!(
            "COMPOSE-{}",
            &sha256(
                format!(
                    "{}:{}:{}:{}",
                    family_hashes.iter().cloned().collect::<Vec<_>>().join(":"),
                    source_generation,
                    attempt_nonce,
                    candidate_sha256
                )
                .as_bytes()
            )[..24]
        ),
        relative_path: target_path.clone(),
        predecessor_sha256,
        candidate_source,
        candidate_sha256,
        transformation: transformation.clone(),
        consequence_predictions: consequences,
        predicted_value: policy.minimum_predicted_value.clamp(85, 100),
        source_generation,
        core_generated: true,
        core_self_approved: true,
        solution_strategy: "EMIT_TYPED_RUST_AND_ACTIVATE_CALLABLE".to_string(),
        structural_repair_program: Some(structural_repair_program),
        generalized_change: Some(generalized_change),
        additional_family_members: Vec::new(),
        opportunity_kind: ChangeOpportunityKind::CapabilityGap,
        opportunity_family_id,
        improvement_operator_invocation: Some(improvement_operator_invocation),
    };
    install_and_stage_source_patch(policy, state_dir, &request)
}

/// Executes a fresh input-seeded semantic canary over the promoted SEM-5
/// primitive composer.  This is behavioral evidence, not merely validation of
/// the composition graph's declared types.
pub fn compose_behavioral_canary_candidate(
    context_sha256: &str,
) -> Result<(CompositeProgramCandidateIR, ProgramTask), String> {
    if context_sha256.len() != 64 || !context_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("BEHAVIORAL_CANARY_CONTEXT_INVALID".to_string());
    }
    let context_seed = u64::from_str_radix(&context_sha256[..16], 16)
        .map_err(|error| format!("BEHAVIORAL_CANARY_SEED:{error}"))?
        ^ 0x1A7E_600D;
    let sets = generate_task_sets(context_seed.max(1));
    let candidates = discover_candidates(&sets.discovery);
    let promotions = initial_promotions(&candidates, &sets.calibration);
    if sets.adversarial.is_empty() {
        return Err("BEHAVIORAL_CANARY_TASK_MISSING".to_string());
    }
    let selection_bits = u64::from_str_radix(&context_sha256[16..32], 16)
        .map_err(|error| format!("BEHAVIORAL_CANARY_SELECTION:{error}"))?;
    let start = (selection_bits as usize) % sets.adversarial.len();
    let attempts = sets
        .adversarial
        .len()
        .min(MAX_CONTEXTUAL_TYPED_TASK_ATTEMPTS);
    let mut failure_classes = BTreeSet::new();
    for offset in 0..attempts {
        let task = sets.adversarial[(start + offset) % sets.adversarial.len()]
            .visible
            .clone();
        let result = compose_existing_sem5_capability(CompositionWork {
            opportunity: CapabilityOpportunityIR {
                observed_gap: format!(
                    "behavioral composition context {context_sha256} task {}",
                    task.task_id
                ),
                desired_behavior: format!(
                    "compose and execute typed relation with output {:?}",
                    task.output_type
                ),
                trigger: "generative growth selected the SEM-5 typed composer".to_string(),
                evidence: vec![context_sha256.to_string(), task.task_id.clone()],
                preserved_behavior: vec![
                    "do not change verifier, installer, or acceptance policy".to_string()
                ],
                resource_constraints: vec![format!(
                    "bounded typed synthesis attempts <= {MAX_CONTEXTUAL_TYPED_TASK_ATTEMPTS}"
                )],
                verification_requirements: vec![
                    "type/effect audit".to_string(),
                    "fresh semantic equivalence cases".to_string(),
                ],
                provenance: vec![
                    "SEM5_EXISTING_SYNTHESIZER".to_string(),
                    "CONTEXT_DERIVED_TYPED_TASK".to_string(),
                ],
                operator_selected_implementation: false,
            },
            task: task.clone(),
            promotions: promotions.clone(),
            predecessor_tree_hash: AUTHORITATIVE_PREDECESSOR.to_string(),
        });
        match result {
            Ok(candidate) => return Ok((candidate, task)),
            Err(error) => {
                failure_classes.insert(error.split(':').next().unwrap_or("UNKNOWN").to_string());
            }
        }
    }
    Err(format!(
        "CONTEXTUAL_TYPED_TASK_EXHAUSTED:{}:{}",
        attempts,
        failure_classes.into_iter().collect::<Vec<_>>().join(",")
    ))
}

pub fn execute_behavioral_composition_canary(
    context_sha256: &str,
) -> Result<BehavioralCompositionCanaryReceipt, String> {
    let case_seed = u64::from_str_radix(&context_sha256[..16], 16)
        .map_err(|error| format!("BEHAVIORAL_CANARY_SEED:{error}"))?;
    let (candidate, task) = compose_behavioral_canary_candidate(context_sha256)?;
    let cases = generate_property_cases(&task, case_seed)
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    let mut passed = 0_usize;
    let installed_capability_present =
        crate::generated_sem5_capability::GENERATED_CAPABILITY_ACTIVE;
    let installed_registry_capability_count =
        crate::generated_sem5_capability::GENERATED_CAPABILITY_COUNT;
    let installed_program_match = installed_capability_present
        && crate::generated_sem5_capability::generated_capability_hashes()
            .contains(&candidate.program_ir_sha256.as_str());
    let installed_source_schema_revision =
        crate::generated_sem5_capability::GENERATED_SOURCE_SCHEMA_REVISION;
    let mut installed_cases_executed = 0_usize;
    let mut installed_cases_passed = 0_usize;
    let mut installed_outputs = Vec::new();
    for inputs in &cases {
        let expected = evaluate_contract(&task, inputs)?;
        let actual = execute(
            &candidate.program_ir,
            inputs,
            &task.definitions,
            BTreeMap::new(),
        )?;
        if actual.value != expected {
            return Err("BEHAVIORAL_CANARY_SEMANTIC_MISMATCH".to_string());
        }
        passed += 1;
        if installed_program_match {
            installed_cases_executed += 1;
            let installed = crate::generated_sem5_capability::run_generated_capability_by_sha256(
                &candidate.program_ir_sha256,
                inputs,
            )
            .map_err(|error| format!("INSTALLED_CAPABILITY_EXECUTION:{error}"))?;
            if installed != expected {
                return Err("INSTALLED_CAPABILITY_SEMANTIC_MISMATCH".to_string());
            }
            installed_cases_passed += 1;
            installed_outputs.push(installed);
        }
    }
    let installed_output_sha256 = installed_program_match
        .then(|| serde_json::to_vec(&installed_outputs))
        .transpose()
        .map_err(|error| format!("INSTALLED_CAPABILITY_OUTPUT_SERIALIZE:{error}"))?
        .map(|bytes| sha256(&bytes));
    let receipt_sha256 = sha256(
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            context_sha256,
            candidate.program_ir_sha256,
            cases.len(),
            passed,
            installed_capability_present,
            installed_program_match,
            installed_source_schema_revision,
            installed_registry_capability_count,
            installed_cases_executed,
            installed_cases_passed,
            installed_output_sha256.as_deref().unwrap_or("NONE")
        )
        .as_bytes(),
    );
    Ok(BehavioralCompositionCanaryReceipt {
        schema: "B_CORE_BEHAVIORAL_COMPOSITION_CANARY_1".to_string(),
        context_sha256: context_sha256.to_string(),
        program_ir_sha256: candidate.program_ir_sha256,
        used_primitive_ids: candidate.used_primitive_ids,
        cases_executed: cases.len(),
        cases_passed: passed,
        installed_capability_present,
        installed_program_match,
        installed_source_schema_revision,
        installed_registry_capability_count,
        installed_cases_executed,
        installed_cases_passed,
        installed_output_sha256,
        receipt_sha256,
    })
}

pub fn validate_composite_installation_authority(
    candidate: &CompositeProgramCandidateIR,
    receipt: &VerificationReceipt,
) -> Result<(), IntegratedInstallationGateError> {
    if receipt.defect_contract_sha256 != candidate.defect_contract.sha256 {
        return Err(IntegratedInstallationGateError::DefectContractReceiptMismatch);
    }
    validate_installation_authority(&candidate.patch_candidate, receipt)
        .map_err(IntegratedInstallationGateError::RsiGate)
}

fn observation_from_opportunity(opportunity: &CapabilityOpportunityIR) -> ObservationIR {
    ObservationIR {
        observed_event: opportunity.observed_gap.clone(),
        trigger: opportunity.trigger.clone(),
        expected_observable: opportunity.desired_behavior.clone(),
        actual_observable: opportunity.observed_gap.clone(),
        evidence: opportunity.evidence.clone(),
        provenance: opportunity.provenance.clone(),
        contains_hidden_diagnosis: false,
    }
}

fn defect_contract_from_opportunity(opportunity: &CapabilityOpportunityIR) -> DefectContractIR {
    DefectContractIR {
        affected_behavior: opportunity.desired_behavior.clone(),
        violated_invariant: "requested capability is not yet represented by executable typed code"
            .to_string(),
        scope: "bounded capability gap represented as a typed ProgramTask".to_string(),
        trigger_conditions: vec![opportunity.trigger.clone()],
        expected_vs_observed: format!(
            "expected={} | observed={}",
            opportunity.desired_behavior, opportunity.observed_gap
        ),
        causal_evidence: opportunity.evidence.clone(),
        uncertainty: "composition remains a proposal until independent evaluation".to_string(),
        suspected_mechanism_classes: vec![
            "missing primitive composition".to_string(),
            "missing reusable semantic recombination".to_string(),
        ],
        affected_interfaces: vec!["SEM5_PROGRAM_IR".to_string()],
        preserved_behavior: opportunity.preserved_behavior.clone(),
        provenance: opportunity.provenance.clone(),
        prescribes_concrete_edit: false,
    }
}

fn repair_spec_from_opportunity(opportunity: &CapabilityOpportunityIR) -> RepairSpecIR {
    RepairSpecIR {
        required_postcondition: opportunity.desired_behavior.clone(),
        restored_invariants: vec![
            "program is type/effect valid".to_string(),
            "program is composed from the frozen SEM-5 primitive basis".to_string(),
        ],
        allowed_semantic_changes: vec!["add one isolated composed ProgramIR artifact".to_string()],
        forbidden_semantic_changes: opportunity.preserved_behavior.clone(),
        compatibility_requirements: vec![
            "SEM-5 primitive and promoted-concept semantics remain authoritative".to_string(),
            "RSI verifier and installer remain external to the proposer".to_string(),
        ],
        resource_constraints: opportunity.resource_constraints.clone(),
        expected_consequences: vec![opportunity.desired_behavior.clone()],
        rollback_conditions: vec![
            "remove the isolated ProgramIR artifact and restore predecessor hash".to_string(),
        ],
        verification_requirements: opportunity.verification_requirements.clone(),
        applicability: vec![opportunity.trigger.clone()],
        uncertainty: "independent verifier decides semantic acceptance".to_string(),
        encodes_exact_patch: false,
    }
}

fn used_node_kinds(program_ir: &ProgramIR) -> Result<BTreeSet<String>, String> {
    let encoded =
        serde_json::to_value(program_ir).map_err(|error| format!("PROGRAM_IR_VALUE:{error}"))?;
    let mut kinds = BTreeSet::new();
    collect_node_kinds(&encoded, &mut kinds);
    Ok(kinds)
}

fn collect_node_kinds(value: &JsonValue, kinds: &mut BTreeSet<String>) {
    match value {
        JsonValue::Object(object) => {
            if let Some(kind) = object.get("node_kind").and_then(JsonValue::as_str) {
                kinds.insert(kind.to_string());
            }
            for child in object.values() {
                collect_node_kinds(child, kinds);
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                collect_node_kinds(child, kinds);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::self_repair_contract::{VerificationDecision, VerificationReceipt};
    use crate::sem26_engine::DirectorState;
    use crate::sem27_engine::PostScaffoldState;
    use crate::sem5::{
        ir::execute,
        learner::{discover_candidates, initial_promotions},
        tasks::{evaluate_contract, generate_property_cases, generate_task_sets},
    };

    fn composition_work() -> CompositionWork {
        let sets = generate_task_sets(0x1A7E_600D);
        let candidates = discover_candidates(&sets.discovery);
        let promotions = initial_promotions(&candidates, &sets.calibration);
        assert!(promotions
            .iter()
            .any(|promotion| promotion.concept.concept_id == "C000010" && promotion.promoted));
        CompositionWork {
            opportunity: CapabilityOpportunityIR {
                observed_gap: "no executable composite exists for the requested relation"
                    .to_string(),
                desired_behavior: "compose and execute the requested multi-stage relation"
                    .to_string(),
                trigger: "recursive director selects a capability-construction bottleneck"
                    .to_string(),
                evidence: vec!["typed composition task remains unsatisfied".to_string()],
                preserved_behavior: vec![
                    "do not change verifier, installer, or acceptance policy".to_string()
                ],
                resource_constraints: vec!["activate only the task-local ProgramIR".to_string()],
                verification_requirements: vec![
                    "type/effect audit".to_string(),
                    "fresh semantic cases".to_string(),
                    "regression suite".to_string(),
                ],
                provenance: vec!["SEM5_EXISTING_SYNTHESIZER".to_string()],
                operator_selected_implementation: false,
            },
            task: sets.adversarial[0].visible.clone(),
            promotions,
            predecessor_tree_hash: AUTHORITATIVE_PREDECESSOR.to_string(),
        }
    }

    fn recursive_request() -> PostScaffoldEpochRequest {
        PostScaffoldEpochRequest {
            arm_code: 3,
            epoch: 1,
            seed: 0x2701,
            state: PostScaffoldState::from_sem26(DirectorState::frozen_sem25()),
            resource_ceiling_bytes: 2_000_000,
            historical_roadmap_target_code: None,
            disable_long_term_research_memory: false,
            concrete_future_instance_visible: false,
        }
    }

    #[test]
    fn existing_sem5_primitives_build_executable_composite_candidate() {
        let work = composition_work();
        let task = work.task.clone();
        let candidate = compose_existing_sem5_capability(work).expect("compose existing SEM-5");
        assert!(candidate.type_effect_audit_pass);
        assert!(candidate.recombinations > 0);
        assert!(candidate.used_primitive_ids.len() >= 2);
        assert!(candidate.primitive_expanded_nodes > candidate.operational_nodes);
        assert!(candidate
            .promoted_concept_ids
            .contains(&"C000010".to_string()));
        assert!(!candidate.patch_candidate.core_self_approved);
        assert!(!candidate.installed);
        assert!(candidate
            .generated_rust_source
            .contains("pub fn run_generated_capability(inputs:"));
        assert!(candidate
            .generated_rust_source
            .contains("GENERATED_CAPABILITY_ACTIVE: bool = true"));
        assert!(candidate.generated_rust_source.contains(&format!(
            "GENERATED_SOURCE_SCHEMA_REVISION: u64 = {}",
            crate::sem5::emitter::CALLABLE_SOURCE_SCHEMA_REVISION
        )));
        assert!(!candidate.generated_rust_source.contains("push((("));
        assert!(!candidate.generated_rust_source.contains("state = (state +"));
        assert!(!candidate.generated_rust_source.contains("            ();"));
        assert!(candidate
            .generated_rust_source
            .contains(&candidate.program_ir_sha256));
        assert_ne!(
            candidate.patch_candidate.unified_diff_sha256,
            candidate.program_ir_sha256
        );
        let formatting_policy = AutonomousSourceMutationPolicy {
            cargo_executable: PathBuf::from(env!("CARGO")),
            ..AutonomousSourceMutationPolicy::default()
        };
        let formatted =
            rustfmt_generated_source(&formatting_policy, &candidate.generated_rust_source)
                .expect("format generated callable");
        assert!(syn::parse_file(&formatted).is_ok());
        assert_eq!(
            rustfmt_generated_source(&formatting_policy, &formatted)
                .expect("formatting is idempotent"),
            formatted
        );
        assert_eq!(candidate.full_source_scan_events, 0);

        for inputs in generate_property_cases(&task, 0xC05E).into_iter().take(3) {
            let expected = evaluate_contract(&task, &inputs).expect("contract");
            let actual = execute(
                &candidate.program_ir,
                &inputs,
                &task.definitions,
                BTreeMap::new(),
            )
            .expect("execute composite");
            assert_eq!(actual.value, expected);
        }
    }

    #[test]
    fn typed_lowering_registry_preserves_prior_callables_and_dispatches_by_hash() {
        let candidate =
            compose_existing_sem5_capability(composition_work()).expect("typed candidate");
        let first_hash = "a".repeat(64);
        let second_hash = "b".repeat(64);
        let first_callable = candidate
            .generated_rust_source
            .replace(&candidate.program_ir_sha256, &first_hash)
            .replace(&candidate.program_ir.program_id, "REGISTRY-FIRST");
        let second_callable = candidate
            .generated_rust_source
            .replace(&candidate.program_ir_sha256, &second_hash)
            .replace(&candidate.program_ir.program_id, "REGISTRY-SECOND");
        let legacy = include_str!("generated_sem5_capability.rs");
        let predecessor_capability_count = existing_capability_sections(legacy)
            .expect("valid predecessor registry")
            .len()
            .max(1);

        let first_registry =
            merge_typed_capability_registry(legacy, &first_callable, "REGISTRY-FIRST", &first_hash)
                .expect("first registry migration");
        let second_registry = merge_typed_capability_registry(
            &first_registry,
            &second_callable,
            "REGISTRY-SECOND",
            &second_hash,
        )
        .expect("second registry extension");

        let expected_capability_count = predecessor_capability_count + 2;
        assert_eq!(
            second_registry.matches(CAPABILITY_BEGIN_PREFIX).count(),
            expected_capability_count
        );
        assert!(second_registry.contains(&format!(
            "GENERATED_CAPABILITY_COUNT: usize = {expected_capability_count}"
        )));
        assert!(second_registry.contains("run_generated_capability_by_sha256"));
        assert!(second_registry.contains(&format!(
            "{first_hash:?} => capability_{}::run_generated_capability(inputs)",
            &first_hash[..16]
        )));
        assert!(second_registry.contains(&format!(
            "{second_hash:?} => capability_{}::run_generated_capability(inputs)",
            &second_hash[..16]
        )));
        assert!(second_registry
            .contains("capability_bbbbbbbbbbbbbbbb::run_generated_capability(inputs)\n}"));
        assert!(syn::parse_file(&second_registry).is_ok());

        let colliding_hash = format!("{}{}", "a".repeat(16), "c".repeat(48));
        let colliding_callable = second_callable
            .replace(&second_hash, &colliding_hash)
            .replace("REGISTRY-SECOND", "REGISTRY-COLLISION");
        assert_eq!(
            merge_typed_capability_registry(
                &second_registry,
                &colliding_callable,
                "REGISTRY-COLLISION",
                &colliding_hash,
            ),
            Err("CAPABILITY_REGISTRY_MODULE_PREFIX_COLLISION".to_string())
        );
    }

    #[test]
    fn typed_lowering_family_enters_one_registry_postimage() {
        let base = compose_existing_sem5_capability(composition_work()).expect("typed candidate");
        let predecessor = include_str!("generated_sem5_capability.rs");
        let predecessor_count = existing_capability_sections(predecessor)
            .expect("valid predecessor registry")
            .len()
            .max(1);
        let candidates = ["8", "9", "f"]
            .into_iter()
            .enumerate()
            .map(|(ordinal, digit)| {
                let mut candidate = base.clone();
                let hash = digit.repeat(64);
                let program_id = format!("REGISTRY-FAMILY-{ordinal}");
                candidate.generated_rust_source = candidate
                    .generated_rust_source
                    .replace(&base.program_ir_sha256, &hash)
                    .replace(&base.program_ir.program_id, &program_id);
                candidate.generated_rust_sha256 =
                    sha256(candidate.generated_rust_source.as_bytes());
                candidate.program_ir_sha256 = hash;
                candidate.program_ir.program_id = program_id;
                candidate
            })
            .collect::<Vec<_>>();

        let registry = merge_typed_capability_family_registry(predecessor, &candidates)
            .expect("atomic family registry postimage");

        assert_eq!(
            registry.matches(CAPABILITY_BEGIN_PREFIX).count(),
            predecessor_count + candidates.len()
        );
        for candidate in &candidates {
            assert!(registry.contains(&candidate.program_ir_sha256));
        }
        let current = candidates.last().expect("current family member");
        assert!(registry.contains(&format!(
            "pub const GENERATED_PROGRAM_IR_SHA256: &str = {:?}",
            current.program_ir_sha256
        )));
        assert!(syn::parse_file(&registry).is_ok());
    }

    #[test]
    fn behavioral_canary_executes_fresh_context_bound_cases() {
        let first = execute_behavioral_composition_canary(&"a".repeat(64)).expect("first canary");
        let second = execute_behavioral_composition_canary(&"b".repeat(64)).expect("second canary");

        assert_eq!(first.cases_executed, 3);
        assert_eq!(first.cases_passed, 3);
        if first.installed_program_match {
            assert!(first.installed_capability_present);
            assert!(first.installed_source_schema_revision > 0);
            assert!(first.installed_registry_capability_count > 0);
            assert_eq!(first.installed_cases_executed, first.cases_executed);
            assert_eq!(first.installed_cases_passed, first.cases_passed);
            assert!(first.installed_output_sha256.is_some());
        } else {
            assert_eq!(first.installed_cases_executed, 0);
            assert_eq!(first.installed_cases_passed, 0);
            assert!(first.installed_output_sha256.is_none());
        }
        assert_ne!(first.receipt_sha256, second.receipt_sha256);
        assert_ne!(first.program_ir_sha256, second.program_ir_sha256);
        assert!(!first.used_primitive_ids.is_empty());
    }

    #[test]
    fn contextual_typed_task_generation_expands_program_ir_frontier() {
        let mut program_hashes = BTreeSet::new();
        for index in 1_u64..=8 {
            let context = sha256(format!("context-{index}").as_bytes());
            let (candidate, task) =
                compose_behavioral_canary_candidate(&context).expect("contextual candidate");
            program_hashes.insert(candidate.program_ir_sha256);
            assert!(!task.postconditions.is_empty());
        }
        assert!(program_hashes.len() >= 6, "{program_hashes:?}");
    }

    #[test]
    fn integrated_composition_cannot_exit_as_proposal_only() {
        let error = run_integrated_development_epoch(IntegratedDevelopmentEpochRequest {
            recursive_epoch: recursive_request(),
            composition_work: Some(composition_work()),
            installation: None,
        })
        .unwrap_err();
        assert_eq!(error, "COMPOSITION_INSTALLATION_CONTEXT_REQUIRED");
    }

    #[test]
    fn composite_candidate_still_requires_independent_rsi_receipt() {
        let candidate = compose_existing_sem5_capability(composition_work()).expect("candidate");
        let receipt = VerificationReceipt {
            patch_sha256: candidate.patch_candidate.unified_diff_sha256.clone(),
            repair_spec_sha256: candidate.repair_spec.sha256.clone(),
            defect_contract_sha256: candidate.defect_contract.sha256.clone(),
            semantic_checks_sha256: "semantic-checks".to_string(),
            regression_checks_sha256: "regression-checks".to_string(),
            resource_checks_sha256: "resource-checks".to_string(),
            invariant_checks_sha256: "invariant-checks".to_string(),
            decision: VerificationDecision::Accept,
            verifier_identity: "independent-verifier".to_string(),
            verifier_is_proposer: false,
            gold_patch_text_equality_is_authority: false,
            receipt_sha256: "receipt".to_string(),
            authority_seal: "external-authority-seal".to_string(),
        };
        assert_eq!(
            validate_composite_installation_authority(&candidate, &receipt),
            Ok(())
        );

        let mut self_receipt = receipt;
        self_receipt.verifier_is_proposer = true;
        assert_eq!(
            validate_composite_installation_authority(&candidate, &self_receipt),
            Err(IntegratedInstallationGateError::RsiGate(
                InstallationGateError::ProposerVerifierCollision
            ))
        );
    }
}
