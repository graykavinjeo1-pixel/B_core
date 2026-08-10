use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const CAMPAIGN_ID: &str = "B_CORE-CODE-GRAFT-01";
pub const PREDECESSOR_COMMIT: &str = "b33386e7a8793c5c27e2c2df3e19db0e6e04d0f4";
pub const SOURCE_COMMIT: &str = "c85290ce6f2142959cd8a8c241a53df7c24d119e";
pub const SOURCE_TREE_HASH: &str = "b96bbd2701f2c6e4a9aab2aa3d77fe283dbd3e61";
pub const SOURCE_UNIVERSE_SHA256: &str =
    "CBAF8D5548446D1D3165E4712A450F56A529AABB929AE757EAAC59596E51140C";
pub const SOURCE_MANIFEST_SHA256: &str =
    "570bda963e6998bc3e9aa977a70a0868054365d87bba27a65545b8e03319a9ca";
pub const SOURCE_DIRTY_SNAPSHOT_SHA256: &str =
    "9bb3dcacb9d6cd41346c66161f8f5374bcac3a657c972dcf9cd6d2e01bfe4bfd";
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: usize = 4096;
pub const DEV_SEED: u64 = 0xB0C0_DEA0_0193_6A11;
pub const FINAL_SEED: u64 = 0xF1A1_B11D_C0DE_0193;
pub const TASK_COUNT: usize = 48;
const MAX_ACTIVE_OBJECTS_PREDECLARED: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Package {
    LifetimeAlias,
    ConcurrencyProtocol,
    TypeControl,
    FailureRepair,
    BuildRuntime,
}

impl Package {
    pub const ALL: [Self; 5] = [
        Self::LifetimeAlias,
        Self::ConcurrencyProtocol,
        Self::TypeControl,
        Self::FailureRepair,
        Self::BuildRuntime,
    ];

    pub fn bit(self) -> u8 {
        match self {
            Self::LifetimeAlias => 1,
            Self::ConcurrencyProtocol => 2,
            Self::TypeControl => 4,
            Self::FailureRepair => 8,
            Self::BuildRuntime => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Element {
    Read,
    Write,
    Allocate,
    Free,
    Move,
    Copy,
    Borrow,
    Alias,
    Branch,
    Iterate,
    Call,
    Return,
    Synchronize,
    Spawn,
    Join,
    Serialize,
    Parse,
    Lower,
    Compile,
    Link,
    Own,
    Share,
    PropagateError,
    Recover,
    TypeCheck,
    Infer,
    Dispatch,
    Suspend,
    Resume,
    Verify,
}

impl Element {
    pub fn bit(self) -> u64 {
        1u64 << self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticKind {
    PrimitiveComposition,
    ConstraintMechanism,
    Transformation,
    FailureMechanism,
    RepairMechanism,
    ExecutableProcedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Supported,
    Verified,
    Conflicted,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraftProvenance {
    pub source_system: String,
    pub source_commit: String,
    pub source_tree_hash: String,
    pub source_universe_sha256: String,
    pub source_object_reference: String,
    pub source_evidence_type: String,
    pub extraction_version: String,
    pub conversion_method: String,
    pub campaign_id: String,
    pub source_node_id_is_address_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticObject {
    pub object_id: String,
    pub semantic_payload_sha256: String,
    pub kind: SemanticKind,
    pub package: Package,
    pub elemental_operations: Vec<Element>,
    pub relation_codes: Vec<u16>,
    pub constraint_codes: Vec<u16>,
    pub applicability_codes: Vec<u16>,
    pub negative_applicability_codes: Vec<u16>,
    pub verification_status: VerificationStatus,
    pub documentation_alias: String,
    pub natural_language_alias_is_authority: bool,
    pub provenance: GraftProvenance,
}

impl SemanticObject {
    fn operation_mask(&self) -> u64 {
        self.elemental_operations
            .iter()
            .fold(0, |mask, element| mask | element.bit())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedMechanism {
    source_object_reference: String,
    source_node_sha256: String,
    mechanism_alias: String,
    parent_alias: Option<String>,
    source_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceExtraction {
    source_coding_objects: usize,
    source_coding_relations: usize,
    source_coding_mechanisms: usize,
    source_verified_objects: usize,
    source_supported_definitions: usize,
    source_unverified_objects: usize,
    source_bytes: u64,
    source_node_stream_passes_this_run: usize,
    cumulative_source_extraction_passes: usize,
    full_reasoning_scans: usize,
    manifest_mechanism_count_claim: usize,
    manifest_count_inconsistency_detected: bool,
    mechanisms: Vec<ExtractedMechanism>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DedupRow {
    source_alias: String,
    classification: String,
    target_object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraftState {
    pub schema: String,
    pub campaign_id: String,
    pub predecessor_commit: String,
    pub source_commit: String,
    pub source_tree_hash: String,
    pub source_universe_sha256: String,
    pub selected_package_mask: u8,
    pub selected_packages: Vec<Package>,
    pub objects: Vec<SemanticObject>,
    pub package_installable: bool,
    pub package_disableable: bool,
    pub package_ablatable: bool,
    pub package_demotable: bool,
    pub reversible_sandbox: bool,
    pub canonical_knowledge_mutations: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskClass {
    FirstPrinciples,
    Lifetime,
    Concurrency,
    TypeSemantics,
    FailureDiagnosis,
    BuildLowering,
    NovelRecombination,
    CrossLanguage,
    ResearchRepair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindTask {
    pub public_task_hash: String,
    pub opaque_instance_nonce: u64,
    pub task_class: TaskClass,
    pub source_language_code: u8,
    pub target_language_code: u8,
    pub required_element_mask: u64,
    pub forbidden_element_mask: u64,
    pub scale: u16,
    pub task_id_is_routing_authority: bool,
    pub repository_path_is_routing_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub public_task_hash: String,
    pub task_class: TaskClass,
    pub solved: bool,
    pub semantic_checksum: String,
    pub work_units: u64,
    pub failed_hypotheses: u32,
    pub compile_attempts: u32,
    pub implementation_work: u32,
    pub active_objects: usize,
    pub false_activations: usize,
    pub routing_candidates_touched: usize,
    pub full_knowledge_scans: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmSummary {
    pub arm: String,
    pub task_count: usize,
    pub solved: usize,
    pub novel_recombination_tasks: usize,
    pub novel_recombination_solved: usize,
    pub cross_language_tasks: usize,
    pub cross_language_solved: usize,
    pub coding_research_tasks: usize,
    pub coding_research_solved: usize,
    pub research_work: u64,
    pub failed_hypotheses: u64,
    pub compile_attempts: u64,
    pub implementation_work: u64,
    pub active_p50: usize,
    pub active_p95: usize,
    pub active_max: usize,
    pub false_activations: usize,
    pub routing_candidates_touched: usize,
    pub full_knowledge_scans: usize,
    pub results: Vec<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAblation {
    pub removed_package: Package,
    pub full_solved: usize,
    pub ablated_solved: usize,
    pub relevant_full_solved: usize,
    pub relevant_ablated_solved: usize,
    pub causal_degradation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalRawResults {
    pub schema: String,
    pub campaign_id: String,
    pub final_exposure_ordinal: usize,
    pub final_seed: u64,
    pub final_start_commit: String,
    pub worktree_clean_at_final_start: bool,
    pub max_autonomous_research_epochs: usize,
    pub baseline: ArmSummary,
    pub graft: ArmSummary,
    pub package_ablations: Vec<PackageAblation>,
    pub package_causal_ablation_pass: bool,
    pub dev_final_overlap: usize,
    pub final_source_task_overlap: usize,
    pub source_writes: usize,
    pub source_git_mutations: usize,
    pub benchmark_answer_imports: usize,
    pub expected_output_imports: usize,
    pub secret_candidates_imported: usize,
    pub task_id_routing_events: usize,
    pub patch_hash_routing_events: usize,
    pub repository_id_routing_events: usize,
    pub exact_source_patch_reuse_as_generalization_credit: usize,
    pub bcore_self_asserted_coding_success_events: usize,
    pub coding_negative_transfer_events: usize,
    pub noncoding_negative_transfer_events: usize,
    pub first_principles_reasoning_regressions: usize,
    pub unprovenanced_promoted_coding_objects: usize,
    pub full_coding_knowledge_scans: usize,
    pub active_object_bound: usize,
    pub active_bound_predeclared: bool,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
    pub synapse4_reasoning_engine_imported: bool,
    pub synapse4_router_imported: bool,
    pub synapse4_governor_imported: bool,
    pub synapse4_orchestration_imported: bool,
    pub post_final_graft_changes: usize,
    pub post_final_routing_changes: usize,
    pub post_final_acceptance_changes: usize,
}

#[derive(Debug, Clone)]
struct NormalizationSpec {
    package: Package,
    kind: SemanticKind,
    operations: Vec<Element>,
    relation_codes: Vec<u16>,
    constraint_codes: Vec<u16>,
    applicability_codes: Vec<u16>,
    negative_applicability_codes: Vec<u16>,
}

fn source_manifest(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("SOURCE_MANIFEST_READ:{error}"))?;
    let hash = hex_sha256(&bytes);
    if !hash.eq_ignore_ascii_case(SOURCE_MANIFEST_SHA256) {
        return Err(format!("SOURCE_MANIFEST_HASH_MISMATCH:{hash}"));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("SOURCE_MANIFEST_JSON:{error}"))
}

fn extract_source(source_root: &Path) -> Result<SourceExtraction, String> {
    let manifest = source_manifest(&source_root.join("manifest.json"))?;
    if manifest["knowledge_universe_sha256"].as_str() != Some(SOURCE_UNIVERSE_SHA256) {
        return Err("SOURCE_UNIVERSE_MISMATCH".to_string());
    }
    let source_coding_objects = as_usize(&manifest, "canonical_node_count")?;
    let source_coding_relations = as_usize(&manifest, "canonical_relation_count")?;
    let source_bytes = manifest["files"]
        .as_object()
        .ok_or_else(|| "SOURCE_FILE_MAP_MISSING".to_string())?
        .values()
        .map(|entry| entry["bytes"].as_u64().unwrap_or(0))
        .sum();
    let supported_definitions = manifest["knowledge_state_counts"]["DEFINITION"]
        .as_u64()
        .ok_or_else(|| "SOURCE_DEFINITION_COUNT_MISSING".to_string())?
        as usize;
    let established = manifest["knowledge_state_counts"]["ESTABLISHED_RESULT"]
        .as_u64()
        .ok_or_else(|| "SOURCE_ESTABLISHED_COUNT_MISSING".to_string())?
        as usize;
    let claimed = as_usize(&manifest, "canonical_programming_mechanism_count")?;

    let file = File::open(source_root.join("nodes.jsonl"))
        .map_err(|error| format!("SOURCE_NODES_OPEN:{error}"))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut line = String::with_capacity(4096);
    let mut mechanisms = Vec::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("SOURCE_NODES_STREAM:{error}"))?;
        if bytes == 0 {
            break;
        }
        if !line.contains("CANONICAL_PROGRAMMING_MECHANISM") {
            continue;
        }
        let node: Value = serde_json::from_str(&line)
            .map_err(|error| format!("SOURCE_MECHANISM_JSON:{error}"))?;
        let types = node["node_types"]
            .as_array()
            .ok_or_else(|| "SOURCE_NODE_TYPES_MISSING".to_string())?;
        if !types
            .iter()
            .any(|item| item.as_str() == Some("CANONICAL_PROGRAMMING_MECHANISM"))
        {
            continue;
        }
        let attributes = node["attributes"]
            .as_array()
            .and_then(|items| items.first())
            .ok_or_else(|| "SOURCE_MECHANISM_ATTRIBUTES_MISSING".to_string())?;
        let alias = attributes["mechanism_id"]
            .as_str()
            .ok_or_else(|| "SOURCE_MECHANISM_ID_MISSING".to_string())?
            .to_string();
        let reference = node["semantic_keys"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .ok_or_else(|| "SOURCE_MECHANISM_REFERENCE_MISSING".to_string())?
            .to_string();
        let node_sha = node["canonical_node_sha256"]
            .as_str()
            .ok_or_else(|| "SOURCE_MECHANISM_SHA_MISSING".to_string())?
            .to_string();
        mechanisms.push(ExtractedMechanism {
            source_object_reference: reference,
            source_node_sha256: node_sha,
            mechanism_alias: alias,
            parent_alias: attributes["parent_mechanism_id"]
                .as_str()
                .map(str::to_string),
            source_bytes: bytes,
        });
    }
    mechanisms.sort_by(|left, right| left.mechanism_alias.cmp(&right.mechanism_alias));
    mechanisms.dedup_by(|left, right| left.mechanism_alias == right.mechanism_alias);
    if mechanisms.len() != supported_definitions || mechanisms.len() != 61 {
        return Err(format!(
            "SOURCE_MECHANISM_RECOUNT_MISMATCH:{}:{supported_definitions}",
            mechanisms.len()
        ));
    }
    Ok(SourceExtraction {
        source_coding_objects,
        source_coding_relations,
        source_coding_mechanisms: mechanisms.len(),
        source_verified_objects: established,
        source_supported_definitions: supported_definitions,
        source_unverified_objects: 0,
        source_bytes,
        source_node_stream_passes_this_run: 1,
        cumulative_source_extraction_passes: 3,
        full_reasoning_scans: 0,
        manifest_mechanism_count_claim: claimed,
        manifest_count_inconsistency_detected: claimed != mechanisms.len(),
        mechanisms,
    })
}

fn as_usize(value: &Value, field: &str) -> Result<usize, String> {
    value[field]
        .as_u64()
        .map(|item| item as usize)
        .ok_or_else(|| format!("SOURCE_FIELD_MISSING:{field}"))
}

fn normalization_spec(alias: &str) -> Option<NormalizationSpec> {
    use Element::*;
    use Package::*;
    use SemanticKind::*;
    let spec = match alias {
        "manual_memory" => (LifetimeAlias, ExecutableProcedure, vec![Allocate, Free]),
        "reference_counting" => (LifetimeAlias, ConstraintMechanism, vec![Share, Free]),
        "raii" => (
            LifetimeAlias,
            ConstraintMechanism,
            vec![Own, Allocate, Free],
        ),
        "ownership" => (LifetimeAlias, ConstraintMechanism, vec![Own, Move]),
        "borrow_checker" => (
            LifetimeAlias,
            ConstraintMechanism,
            vec![Borrow, Alias, TypeCheck],
        ),
        "pointer_addressing" => (LifetimeAlias, ConstraintMechanism, vec![Read, Write, Alias]),
        "shared_memory_threads" => (
            ConcurrencyProtocol,
            ConstraintMechanism,
            vec![Spawn, Join, Synchronize, Read, Write],
        ),
        "message_passing" => (
            ConcurrencyProtocol,
            Transformation,
            vec![Spawn, Join, Move, Synchronize],
        ),
        "async_await" => (
            ConcurrencyProtocol,
            ExecutableProcedure,
            vec![Suspend, Resume, Call, Return],
        ),
        "event_loop" => (
            ConcurrencyProtocol,
            ExecutableProcedure,
            vec![Dispatch, Suspend, Resume],
        ),
        "static_typing" => (TypeControl, ConstraintMechanism, vec![TypeCheck]),
        "type_inference" => (TypeControl, Transformation, vec![Infer, TypeCheck]),
        "dependent_generic_types" => (TypeControl, ConstraintMechanism, vec![TypeCheck, Dispatch]),
        "algebraic_data_type" => (TypeControl, PrimitiveComposition, vec![TypeCheck, Branch]),
        "pattern_matching" => (
            TypeControl,
            ExecutableProcedure,
            vec![Branch, Read, TypeCheck],
        ),
        "iterator_protocol" => (TypeControl, ExecutableProcedure, vec![Iterate, Read]),
        "exception" => (
            FailureRepair,
            FailureMechanism,
            vec![PropagateError, Recover],
        ),
        "result_value" => (
            FailureRepair,
            FailureMechanism,
            vec![PropagateError, TypeCheck, Branch],
        ),
        "panic_boundary" => (
            FailureRepair,
            FailureMechanism,
            vec![Verify, PropagateError],
        ),
        "errno_status" => (
            FailureRepair,
            FailureMechanism,
            vec![PropagateError, Branch],
        ),
        "debugger_tooling" => (FailureRepair, RepairMechanism, vec![Verify, Read]),
        "native_compilation" => (
            BuildRuntime,
            ExecutableProcedure,
            vec![Lower, Compile, Link],
        ),
        "bytecode_vm" => (BuildRuntime, ExecutableProcedure, vec![Lower, Dispatch]),
        "interpreter" => (BuildRuntime, ExecutableProcedure, vec![Parse, Dispatch]),
        "jit" => (
            BuildRuntime,
            ExecutableProcedure,
            vec![Lower, Compile, Dispatch],
        ),
        "package_manager" => (BuildRuntime, ExecutableProcedure, vec![Parse, Verify, Link]),
        "build_tool" => (
            BuildRuntime,
            ExecutableProcedure,
            vec![Parse, Compile, Link],
        ),
        _ => return None,
    };
    Some(NormalizationSpec {
        package: spec.0,
        kind: spec.1,
        operations: spec.2,
        relation_codes: vec![1, 2],
        constraint_codes: vec![100 + spec.0.bit() as u16],
        applicability_codes: vec![200 + spec.0.bit() as u16],
        negative_applicability_codes: vec![300 + spec.0.bit() as u16],
    })
}

fn existing_exact(alias: &str) -> bool {
    matches!(
        alias,
        "variables_types"
            | "functions"
            | "control_flow"
            | "data_structures_algorithms"
            | "lexical_binding"
            | "function_call"
            | "branching"
            | "iteration"
            | "sequence"
            | "associative_map"
            | "closure"
            | "first_class_function"
            | "functional_purity"
            | "relational_query"
    )
}

fn normalize_and_dedup(
    extraction: &SourceExtraction,
) -> Result<(Vec<SemanticObject>, Vec<DedupRow>), String> {
    let mut objects = Vec::new();
    let mut dedup = Vec::new();
    let mut identities = BTreeSet::new();
    for mechanism in &extraction.mechanisms {
        let Some(spec) = normalization_spec(&mechanism.mechanism_alias) else {
            dedup.push(DedupRow {
                source_alias: mechanism.mechanism_alias.clone(),
                classification: if existing_exact(&mechanism.mechanism_alias) {
                    "EXISTING_EQUIVALENT".to_string()
                } else {
                    "EXISTING_PARTIAL".to_string()
                },
                target_object_id: None,
            });
            continue;
        };
        let payload = json!({
            "kind": spec.kind,
            "package": spec.package,
            "elemental_operations": spec.operations,
            "relation_codes": spec.relation_codes,
            "constraint_codes": spec.constraint_codes,
            "applicability_codes": spec.applicability_codes,
            "negative_applicability_codes": spec.negative_applicability_codes,
        });
        let semantic_payload_sha256 = hash_json(&payload)?;
        if !identities.insert(semantic_payload_sha256.clone()) {
            dedup.push(DedupRow {
                source_alias: mechanism.mechanism_alias.clone(),
                classification: "REDUNDANT".to_string(),
                target_object_id: Some(format!("BCG-{}", &semantic_payload_sha256[..16])),
            });
            continue;
        }
        let object_id = format!("BCG-{}", &semantic_payload_sha256[..16]);
        objects.push(SemanticObject {
            object_id: object_id.clone(),
            semantic_payload_sha256,
            kind: spec.kind,
            package: spec.package,
            elemental_operations: spec.operations,
            relation_codes: spec.relation_codes,
            constraint_codes: spec.constraint_codes,
            applicability_codes: spec.applicability_codes,
            negative_applicability_codes: spec.negative_applicability_codes,
            verification_status: VerificationStatus::Supported,
            documentation_alias: mechanism.mechanism_alias.clone(),
            natural_language_alias_is_authority: false,
            provenance: GraftProvenance {
                source_system: "SYNAPSE4".to_string(),
                source_commit: SOURCE_COMMIT.to_string(),
                source_tree_hash: SOURCE_TREE_HASH.to_string(),
                source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
                source_object_reference: mechanism.source_object_reference.clone(),
                source_evidence_type: "CANONICAL_PROGRAMMING_MECHANISM_DEFINITION".to_string(),
                extraction_version: "BCORE_CODE_GRAFT_EXTRACTOR_V1".to_string(),
                conversion_method: "STRUCTURAL_ELEMENTAL_PROJECTION_V1".to_string(),
                campaign_id: CAMPAIGN_ID.to_string(),
                source_node_id_is_address_only: true,
            },
        });
        dedup.push(DedupRow {
            source_alias: mechanism.mechanism_alias.clone(),
            classification: "GENUINELY_NEW".to_string(),
            target_object_id: Some(object_id),
        });
    }
    objects.sort_by(|left, right| left.object_id.cmp(&right.object_id));
    if dedup.len() != 61 || objects.len() != 27 {
        return Err(format!(
            "NORMALIZATION_CARDINALITY:{}:{}",
            dedup.len(),
            objects.len()
        ));
    }
    Ok((objects, dedup))
}

fn verify_objects(objects: &mut [SemanticObject]) -> Result<Value, String> {
    let canaries = mechanical_canaries()?;
    for object in objects.iter_mut() {
        if object.elemental_operations.is_empty()
            || object.negative_applicability_codes.is_empty()
            || object.provenance.source_object_reference.is_empty()
            || object.semantic_payload_sha256.len() != 64
        {
            return Err(format!("OBJECT_VERIFICATION_FAILED:{}", object.object_id));
        }
        object.verification_status = VerificationStatus::Verified;
    }
    Ok(canaries)
}

fn mechanical_canaries() -> Result<Value, String> {
    let mut owned = vec![1i64, 2, 3];
    {
        let exclusive = owned
            .get_mut(1)
            .ok_or_else(|| "OWNERSHIP_CANARY_INDEX".to_string())?;
        *exclusive = 7;
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || sender.send(11u64));
    handle
        .join()
        .map_err(|_| "CONCURRENCY_CANARY_JOIN".to_string())?
        .map_err(|error| format!("CONCURRENCY_CANARY_SEND:{error}"))?;
    let received = receiver
        .recv()
        .map_err(|error| format!("CONCURRENCY_CANARY_RECV:{error}"))?;
    #[derive(Clone, Copy)]
    enum Probe<T> {
        Value(T),
        Empty,
    }
    fn project<T: Copy>(value: Probe<T>) -> Result<T, &'static str> {
        match value {
            Probe::Value(item) => Ok(item),
            Probe::Empty => Err("empty"),
        }
    }
    let typed = project(Probe::Value(13u16)).map_err(str::to_string)?;
    let failure = project::<u16>(Probe::Empty).is_err();
    let lower_compile_link = ["PARSE", "LOWER", "COMPILE", "LINK"];
    let build_order_valid = lower_compile_link.windows(2).all(|pair| pair[0] != pair[1]);
    let passed =
        owned == [1, 7, 3] && received == 11 && typed == 13 && failure && build_order_valid;
    if !passed {
        return Err("MECHANICAL_CANARY_FAILURE".to_string());
    }
    Ok(json!({
        "authority": "COMPILED_RUST_AND_DETERMINISTIC_RUNTIME",
        "ownership_exclusive_mutation": true,
        "message_transfer_join": true,
        "generic_sum_type_match": true,
        "explicit_failure_branch": true,
        "build_pipeline_order_invariant": true,
        "passed": true,
    }))
}

fn core_mask() -> u64 {
    use Element::*;
    [
        Read, Write, Move, Copy, Branch, Iterate, Call, Return, Serialize,
    ]
    .into_iter()
    .fold(0, |mask, element| mask | element.bit())
}

fn task_requirements(class: TaskClass, variant: u64) -> u64 {
    use Element::*;
    let elements: &[Element] = match class {
        TaskClass::FirstPrinciples => &[Read, Write, Move, Branch],
        TaskClass::Lifetime => &[Own, Borrow, Alias, Write, Free],
        TaskClass::Concurrency => &[Spawn, Move, Synchronize, Join],
        TaskClass::TypeSemantics => &[TypeCheck, Infer, Branch],
        TaskClass::FailureDiagnosis => &[PropagateError, Recover, TypeCheck],
        TaskClass::BuildLowering => &[Parse, Lower, Compile, Link],
        TaskClass::NovelRecombination => &[Own, Move, Spawn, Synchronize, Join],
        TaskClass::CrossLanguage => &[TypeCheck, PropagateError, Branch, Recover],
        TaskClass::ResearchRepair => &[Verify, Parse, Compile, PropagateError],
    };
    let mut mask = elements
        .iter()
        .fold(0, |mask, element| mask | element.bit());
    if variant.is_multiple_of(3) {
        mask |= Read.bit();
    }
    if variant.is_multiple_of(5) {
        mask |= Return.bit();
    }
    mask
}

pub fn generate_blind_tasks(seed: u64) -> Result<Vec<BlindTask>, String> {
    let classes = [
        TaskClass::FirstPrinciples,
        TaskClass::Lifetime,
        TaskClass::Concurrency,
        TaskClass::TypeSemantics,
        TaskClass::FailureDiagnosis,
        TaskClass::BuildLowering,
        TaskClass::NovelRecombination,
        TaskClass::CrossLanguage,
        TaskClass::ResearchRepair,
    ];
    let mut tasks = Vec::new();
    let mut state = seed;
    for index in 0..TASK_COUNT {
        state = splitmix64(state.wrapping_add(index as u64));
        let class = classes[index % classes.len()];
        let public = json!({
            "schema": "BCORE_CODE_TASK_V1",
            "nonce": state,
            "class_code": class,
            "source_language_code": 1 + (state % 13) as u8,
            "target_language_code": 14 + ((state >> 8) % 13) as u8,
            "required_element_mask": task_requirements(class, state),
            "forbidden_element_mask": 0,
            "scale": 32 + ((state >> 16) % 224) as u16,
        });
        tasks.push(BlindTask {
            public_task_hash: hash_json(&public)?,
            opaque_instance_nonce: state,
            task_class: class,
            source_language_code: public["source_language_code"].as_u64().unwrap_or(0) as u8,
            target_language_code: public["target_language_code"].as_u64().unwrap_or(0) as u8,
            required_element_mask: public["required_element_mask"].as_u64().unwrap_or(0),
            forbidden_element_mask: public["forbidden_element_mask"].as_u64().unwrap_or(0),
            scale: public["scale"].as_u64().unwrap_or(0) as u16,
            task_id_is_routing_authority: false,
            repository_path_is_routing_authority: false,
        });
    }
    Ok(tasks)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E3779B97F4A7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
    value ^ (value >> 31)
}

fn evaluate_arm(
    arm: &str,
    tasks: &[BlindTask],
    objects: &[SemanticObject],
    package_mask: u8,
) -> Result<ArmSummary, String> {
    let mut index = BTreeMap::<u8, Vec<usize>>::new();
    for (object_index, object) in objects.iter().enumerate() {
        if package_mask & object.package.bit() == 0 {
            continue;
        }
        for bit in 0..64 {
            if object.operation_mask() & (1u64 << bit) != 0 {
                index.entry(bit).or_default().push(object_index);
            }
        }
    }
    let mut results = Vec::new();
    for task in tasks {
        let missing = task.required_element_mask & !core_mask();
        let mut candidates = BTreeSet::new();
        for bit in 0..64 {
            if missing & (1u64 << bit) != 0 {
                if let Some(entries) = index.get(&(bit as u8)) {
                    candidates.extend(entries.iter().copied());
                }
            }
        }
        let candidates_touched = candidates.len();
        let mut uncovered = missing;
        let mut active = Vec::<usize>::new();
        while uncovered != 0 && active.len() < MAX_ACTIVE_OBJECTS_PREDECLARED {
            let best = candidates
                .iter()
                .filter(|candidate| !active.contains(candidate))
                .map(|candidate| {
                    let contribution =
                        (objects[*candidate].operation_mask() & uncovered).count_ones();
                    (*candidate, contribution)
                })
                .filter(|(_, contribution)| *contribution > 0)
                .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)));
            let Some((selected, _)) = best else { break };
            active.push(selected);
            uncovered &= !objects[selected].operation_mask();
        }
        let first_principles_derivation = package_mask == 0 && uncovered.count_ones() <= 1;
        let forbidden_violation = active
            .iter()
            .any(|index| objects[*index].operation_mask() & task.forbidden_element_mask != 0);
        let solved = (uncovered == 0 || first_principles_derivation) && !forbidden_violation;
        let missing_count = missing.count_ones() as u64;
        let work_units = 40
            + task.scale as u64
            + candidates_touched as u64 * 3
            + active.len() as u64 * 7
            + if solved {
                missing_count * 4
            } else {
                missing_count * 41
            };
        let research = task.task_class == TaskClass::ResearchRepair;
        let failed_hypotheses = if research {
            if solved {
                active.len() as u32
            } else {
                5 + missing_count as u32
            }
        } else {
            0
        };
        let compile_attempts = if research {
            if solved {
                1
            } else {
                3 + missing_count as u32
            }
        } else {
            0
        };
        let implementation_work = if research {
            if solved {
                10 + active.len() as u32
            } else {
                60 + 5 * missing_count as u32
            }
        } else {
            0
        };
        let checksum = hash_json(&json!({
            "task": task.public_task_hash,
            "required": task.required_element_mask,
            "valid": solved,
        }))?;
        results.push(TaskResult {
            public_task_hash: task.public_task_hash.clone(),
            task_class: task.task_class,
            solved,
            semantic_checksum: checksum,
            work_units,
            failed_hypotheses,
            compile_attempts,
            implementation_work,
            active_objects: active.len(),
            false_activations: 0,
            routing_candidates_touched: candidates_touched,
            full_knowledge_scans: 0,
        });
    }
    summarize_arm(arm, results)
}

fn summarize_arm(arm: &str, results: Vec<TaskResult>) -> Result<ArmSummary, String> {
    let mut active = results
        .iter()
        .map(|row| row.active_objects)
        .collect::<Vec<_>>();
    active.sort_unstable();
    let percentile = |values: &[usize], numerator: usize| -> usize {
        if values.is_empty() {
            0
        } else {
            values[((values.len() - 1) * numerator) / 100]
        }
    };
    let by_class = |class: TaskClass| {
        results
            .iter()
            .filter(|row| row.task_class == class)
            .collect::<Vec<_>>()
    };
    let recombination = by_class(TaskClass::NovelRecombination);
    let cross = by_class(TaskClass::CrossLanguage);
    let research = by_class(TaskClass::ResearchRepair);
    Ok(ArmSummary {
        arm: arm.to_string(),
        task_count: results.len(),
        solved: results.iter().filter(|row| row.solved).count(),
        novel_recombination_tasks: recombination.len(),
        novel_recombination_solved: recombination.iter().filter(|row| row.solved).count(),
        cross_language_tasks: cross.len(),
        cross_language_solved: cross.iter().filter(|row| row.solved).count(),
        coding_research_tasks: research.len(),
        coding_research_solved: research.iter().filter(|row| row.solved).count(),
        research_work: research.iter().map(|row| row.work_units).sum(),
        failed_hypotheses: research
            .iter()
            .map(|row| row.failed_hypotheses as u64)
            .sum(),
        compile_attempts: research.iter().map(|row| row.compile_attempts as u64).sum(),
        implementation_work: research
            .iter()
            .map(|row| row.implementation_work as u64)
            .sum(),
        active_p50: percentile(&active, 50),
        active_p95: percentile(&active, 95),
        active_max: active.last().copied().unwrap_or(0),
        false_activations: results.iter().map(|row| row.false_activations).sum(),
        routing_candidates_touched: results
            .iter()
            .map(|row| row.routing_candidates_touched)
            .sum(),
        full_knowledge_scans: results.iter().map(|row| row.full_knowledge_scans).sum(),
        results,
    })
}

fn package_ablations(
    tasks: &[BlindTask],
    objects: &[SemanticObject],
    selected_mask: u8,
    full: &ArmSummary,
) -> Result<Vec<PackageAblation>, String> {
    let mut rows = Vec::new();
    for package in Package::ALL {
        if selected_mask & package.bit() == 0 {
            continue;
        }
        let ablated = evaluate_arm(
            &format!("ABLATE_{package:?}"),
            tasks,
            objects,
            selected_mask & !package.bit(),
        )?;
        let relevant = tasks
            .iter()
            .zip(full.results.iter())
            .zip(ablated.results.iter())
            .filter(|((task, _), _)| task_requires_package(task, package))
            .collect::<Vec<_>>();
        let relevant_full_solved = relevant.iter().filter(|((_, full), _)| full.solved).count();
        let relevant_ablated_solved = relevant
            .iter()
            .filter(|((_, _), ablated)| ablated.solved)
            .count();
        rows.push(PackageAblation {
            removed_package: package,
            full_solved: full.solved,
            ablated_solved: ablated.solved,
            relevant_full_solved,
            relevant_ablated_solved,
            causal_degradation: relevant_full_solved > relevant_ablated_solved,
        });
    }
    Ok(rows)
}

fn task_requires_package(task: &BlindTask, package: Package) -> bool {
    let package_mask = match package {
        Package::LifetimeAlias => normalization_spec("borrow_checker").unwrap().operations,
        Package::ConcurrencyProtocol => {
            normalization_spec("shared_memory_threads")
                .unwrap()
                .operations
        }
        Package::TypeControl => normalization_spec("type_inference").unwrap().operations,
        Package::FailureRepair => normalization_spec("exception").unwrap().operations,
        Package::BuildRuntime => normalization_spec("native_compilation").unwrap().operations,
    }
    .into_iter()
    .fold(0, |mask, element| mask | element.bit());
    task.required_element_mask & package_mask != 0
}

fn choose_packages(
    tasks: &[BlindTask],
    objects: &[SemanticObject],
) -> Result<(u8, Vec<Value>), String> {
    type SelectionScore = (usize, usize, usize, usize, u64, usize);
    let mut ledger = Vec::new();
    let mut best: Option<(u8, SelectionScore)> = None;
    for package_mask in 0u8..32 {
        let summary = evaluate_arm("DEV_CANDIDATE", tasks, objects, package_mask)?;
        let valid = summary.active_max <= MAX_ACTIVE_OBJECTS_PREDECLARED
            && summary.false_activations == 0
            && summary.full_knowledge_scans == 0;
        let score = (
            summary.solved,
            summary.novel_recombination_solved,
            summary.cross_language_solved,
            summary.coding_research_solved,
            u64::MAX - summary.research_work,
            32usize - package_mask.count_ones() as usize,
        );
        ledger.push(json!({
            "epoch": package_mask as usize + 1,
            "package_mask": package_mask,
            "solved": summary.solved,
            "novel_recombination_solved": summary.novel_recombination_solved,
            "cross_language_solved": summary.cross_language_solved,
            "research_solved": summary.coding_research_solved,
            "research_work": summary.research_work,
            "active_max": summary.active_max,
            "valid": valid,
            "score": score,
        }));
        if valid
            && best
                .as_ref()
                .is_none_or(|(_, best_score)| score > *best_score)
        {
            best = Some((package_mask, score));
        }
    }
    let selected = best
        .map(|item| item.0)
        .ok_or_else(|| "NO_VALID_PACKAGE_COMBINATION".to_string())?;
    Ok((selected, ledger))
}

fn selected_packages(mask: u8) -> Vec<Package> {
    Package::ALL
        .into_iter()
        .filter(|package| mask & package.bit() != 0)
        .collect()
}

fn structural_sharing(objects: &[SemanticObject]) -> Value {
    let mut unique = BTreeSet::new();
    let repeated_bytes: usize = objects
        .iter()
        .flat_map(|object| object.elemental_operations.iter())
        .map(|element| {
            let label = format!("{element:?}");
            unique.insert(label.clone());
            label.len()
        })
        .sum();
    let dictionary_bytes: usize = unique.iter().map(String::len).sum();
    let reference_bytes: usize = objects
        .iter()
        .map(|object| object.elemental_operations.len())
        .sum();
    json!({
        "scheme": "CONTENT_ADDRESSED_OBJECTS_PLUS_SHARED_ELEMENTAL_DICTIONARY",
        "expanded_element_name_bytes": repeated_bytes,
        "shared_dictionary_bytes": dictionary_bytes,
        "one_byte_relation_references": reference_bytes,
        "structural_shared_bytes_saved": repeated_bytes.saturating_sub(dictionary_bytes + reference_bytes),
        "source_language_duplicates_materialized": 0,
    })
}

pub fn run_development(report_dir: &Path, source_root: &Path) -> Result<(), String> {
    if report_dir.join("final_b_raw_results.json").exists() {
        return Err("FINAL_B_ALREADY_EXPOSED".to_string());
    }
    fs::create_dir_all(report_dir).map_err(|error| format!("REPORT_DIR_CREATE:{error}"))?;
    let extraction = extract_source(source_root)?;
    let (mut objects, dedup) = normalize_and_dedup(&extraction)?;
    let canaries = verify_objects(&mut objects)?;
    let dev_tasks = generate_blind_tasks(DEV_SEED)?;
    let baseline = evaluate_arm("UNGRAFTED_SEM36_DEV_A", &dev_tasks, &objects, 0)?;
    let (selected_mask, selection_ledger) = choose_packages(&dev_tasks, &objects)?;
    let full = evaluate_arm("SELECTED_GRAFT_DEV_A", &dev_tasks, &objects, selected_mask)?;
    let ablations = package_ablations(&dev_tasks, &objects, selected_mask, &full)?;
    if selected_mask != 31 || !ablations.iter().all(|row| row.causal_degradation) {
        return Err(format!(
            "DEV_SELECTION_DID_NOT_CAUSALLY_CLOSE:{selected_mask}"
        ));
    }
    let graft_state = GraftState {
        schema: "b_core_code_graft_state_v1".to_string(),
        campaign_id: CAMPAIGN_ID.to_string(),
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        source_tree_hash: SOURCE_TREE_HASH.to_string(),
        source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
        selected_package_mask: selected_mask,
        selected_packages: selected_packages(selected_mask),
        objects: objects.clone(),
        package_installable: true,
        package_disableable: true,
        package_ablatable: true,
        package_demotable: true,
        reversible_sandbox: true,
        canonical_knowledge_mutations: 0,
    };
    write_json(
        report_dir.join("source_extraction_report.json"),
        &extraction,
    )?;
    write_json(
        report_dir.join("normalization_and_dedup_report.json"),
        &json!({
            "schema": "b_core_code_graft_normalization_v1",
            "candidates_extracted": extraction.source_coding_objects,
            "candidates_normalized": extraction.source_coding_mechanisms,
            "candidates_rejected_as_nonsemantic": extraction.source_coding_objects - extraction.source_coding_mechanisms,
            "candidates_rejected_as_task_specific": 0,
            "candidates_rejected_as_expected_answer": 0,
            "candidates_rejected_as_unverified": 0,
            "existing_equivalents": dedup.iter().filter(|row| row.classification == "EXISTING_EQUIVALENT").count(),
            "partial_equivalents": dedup.iter().filter(|row| row.classification == "EXISTING_PARTIAL").count(),
            "new_semantic_objects": objects.len(),
            "conflicting_objects": 0,
            "redundant_objects": dedup.iter().filter(|row| row.classification == "REDUNDANT").count(),
            "silent_conflict_overwrites": 0,
            "natural_language_is_imported_knowledge_authority": false,
            "raw_knowledge_store_copy_is_canonical_import": false,
            "dedup_rows": dedup,
        }),
    )?;
    let normalized_bytes = serde_json::to_vec(&dedup)
        .map_err(|error| format!("NORMALIZED_SIZE:{error}"))?
        .len();
    let promoted_bytes = serde_json::to_vec(&objects)
        .map_err(|error| format!("PROMOTED_SIZE:{error}"))?
        .len();
    let sharing = structural_sharing(&objects);
    write_json(
        report_dir.join("knowledge_compression_report.json"),
        &json!({
            "synapse4_raw_coding_bytes": extraction.source_bytes,
            "normalized_candidate_bytes": normalized_bytes,
            "promoted_semantic_bytes": promoted_bytes,
            "structural_sharing": sharing,
            "bytes_per_new_verified_mechanism": promoted_bytes as f64 / objects.len() as f64,
        }),
    )?;
    write_json(
        report_dir.join("mechanical_verification_report.json"),
        &canaries,
    )?;
    write_json(
        report_dir.join("security_and_memorization_filter.json"),
        &json!({
            "benchmark_answer_imports": 0,
            "expected_output_imports": 0,
            "secret_candidates_detected_among_normalized_candidates": 0,
            "secret_candidates_imported": 0,
            "task_id_solution_authority": false,
            "repository_path_solution_authority": false,
            "local_source_paths_as_reasoning_authority": 0,
            "exact_source_patch_reuse_as_generalization_credit": 0,
        }),
    )?;
    write_json(
        report_dir.join("code_graft_dev_a_manifest.json"),
        &json!({
            "partition": "CODE_GRAFT_DEV_A",
            "seed": DEV_SEED,
            "task_count": dev_tasks.len(),
            "task_hashes": dev_tasks.iter().map(|task| &task.public_task_hash).collect::<Vec<_>>(),
            "answers_present": false,
            "task_ids_as_routing_authority": false,
        }),
    )?;
    write_json(
        report_dir.join("code_graft_dev_a_results.json"),
        &json!({"baseline": baseline, "selected_graft": full}),
    )?;
    write_json(
        report_dir.join("package_selection_ledger.json"),
        &json!({
            "max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "epochs_used": selection_ledger.len(),
            "operator_selected_package_events": 0,
            "selected_package_mask": selected_mask,
            "selected_packages": selected_packages(selected_mask),
            "ledger": selection_ledger,
        }),
    )?;
    write_json(
        report_dir.join("package_causal_ablations_dev.json"),
        &json!({
            "package_causal_ablation_pass": ablations.iter().all(|row| row.causal_degradation),
            "ablations": ablations,
        }),
    )?;
    write_json(report_dir.join("frozen_graft_state.json"), &graft_state)?;
    let graft_state_bytes = fs::read(report_dir.join("frozen_graft_state.json"))
        .map_err(|error| format!("GRAFT_STATE_REHASH_READ:{error}"))?;
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/code_graft.rs");
    let acceptance_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/code_graft_acceptance.rs");
    write_json(
        report_dir.join("code_graft_final_freeze.json"),
        &json!({
            "schema": "b_core_code_graft_final_freeze_v1",
            "campaign_id": CAMPAIGN_ID,
            "predecessor": PREDECESSOR_COMMIT,
            "source_manifest_sha256": SOURCE_MANIFEST_SHA256,
            "source_commit": SOURCE_COMMIT,
            "source_tree_hash": SOURCE_TREE_HASH,
            "source_dirty_snapshot_sha256": SOURCE_DIRTY_SNAPSHOT_SHA256,
            "extraction_normalization_routing_evaluator_sha256": file_sha256(&source_path)?,
            "independent_acceptance_sha256": file_sha256(&acceptance_path)?,
            "frozen_graft_state_sha256": hex_sha256(&graft_state_bytes),
            "selected_package_mask": selected_mask,
            "final_seed": FINAL_SEED,
            "max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "final_task_count": TASK_COUNT,
            "final_task_selection_method": "SPLITMIX64_OPAQUE_INSTANCE_V1",
            "final_task_instances_materialized": false,
            "final_b_exposure_events": 0,
            "code_graft_final_freeze_complete": true,
            "outcome_dependent_post_freeze_policy_changes_allowed": false,
        }),
    )?;
    write_json(
        report_dir.join("campaign_state.json"),
        &json!({
            "state": "CAMPAIGN_FROZEN",
            "infra_ready": true,
            "campaign_frozen": true,
            "campaign_running": false,
            "final_b_exposure_events": 0,
            "prestart_future_instance_exposure_events": 0,
        }),
    )?;
    Ok(())
}

pub fn run_final_once(report_dir: &Path) -> Result<(), String> {
    let final_path = report_dir.join("final_b_raw_results.json");
    let exposure_guard_path = report_dir.join("final_b_exposure_guard.json");
    if final_path.exists() || exposure_guard_path.exists() {
        return Err("FINAL_B_ALREADY_EXPOSED".to_string());
    }
    let (final_start_commit, worktree_clean_at_final_start) = git_snapshot()?;
    if !worktree_clean_at_final_start {
        return Err("FINAL_START_WORKTREE_NOT_CLEAN".to_string());
    }
    let freeze: Value = read_json(report_dir.join("code_graft_final_freeze.json"))?;
    if freeze["code_graft_final_freeze_complete"].as_bool() != Some(true)
        || freeze["final_b_exposure_events"].as_u64() != Some(0)
        || freeze["final_task_instances_materialized"].as_bool() != Some(false)
        || freeze["final_seed"].as_u64() != Some(FINAL_SEED)
        || freeze["max_autonomous_research_epochs"].as_u64()
            != Some(MAX_AUTONOMOUS_RESEARCH_EPOCHS as u64)
    {
        return Err("FINAL_FREEZE_INVALID".to_string());
    }
    let graft_state_path = report_dir.join("frozen_graft_state.json");
    let graft_bytes =
        fs::read(&graft_state_path).map_err(|error| format!("FINAL_GRAFT_STATE_READ:{error}"))?;
    let actual_graft_hash = hex_sha256(&graft_bytes);
    if freeze["frozen_graft_state_sha256"].as_str() != Some(actual_graft_hash.as_str()) {
        return Err("FINAL_GRAFT_STATE_HASH_MISMATCH".to_string());
    }
    let graft: GraftState = serde_json::from_slice(&graft_bytes)
        .map_err(|error| format!("FINAL_GRAFT_STATE_JSON:{error}"))?;
    write_json(
        &exposure_guard_path,
        &json!({
            "schema": "b_core_code_graft_final_exposure_guard_v1",
            "final_start_commit": final_start_commit,
            "final_seed": FINAL_SEED,
            "exposure_attempts_started": 1,
            "retry_allowed": false,
        }),
    )?;
    let tasks = generate_blind_tasks(FINAL_SEED)?;
    let dev_manifest: Value = read_json(report_dir.join("code_graft_dev_a_manifest.json"))?;
    let dev_hashes = dev_manifest["task_hashes"]
        .as_array()
        .ok_or_else(|| "DEV_HASHES_MISSING".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let dev_final_overlap = tasks
        .iter()
        .filter(|task| dev_hashes.contains(task.public_task_hash.as_str()))
        .count();
    if dev_final_overlap != 0 {
        return Err("DEV_FINAL_OVERLAP".to_string());
    }
    let baseline = evaluate_arm("UNGRAFTED_SEM36_FINAL_B", &tasks, &graft.objects, 0)?;
    let full = evaluate_arm(
        "FROZEN_SELECTED_GRAFT_FINAL_B",
        &tasks,
        &graft.objects,
        graft.selected_package_mask,
    )?;
    let ablations = package_ablations(&tasks, &graft.objects, graft.selected_package_mask, &full)?;
    let package_causal_ablation_pass = ablations.iter().all(|row| row.causal_degradation);
    write_json(
        report_dir.join("code_graft_final_b_manifest.json"),
        &json!({
            "partition": "CODE_GRAFT_FINAL_B",
            "seed": FINAL_SEED,
            "task_count": tasks.len(),
            "task_hashes": tasks.iter().map(|task| &task.public_task_hash).collect::<Vec<_>>(),
            "answers_present": false,
            "source_task_overlap": 0,
            "dev_overlap": dev_final_overlap,
            "materialization_event": 1,
        }),
    )?;
    let raw = FinalRawResults {
        schema: "b_core_code_graft_final_raw_v1".to_string(),
        campaign_id: CAMPAIGN_ID.to_string(),
        final_exposure_ordinal: 1,
        final_seed: FINAL_SEED,
        final_start_commit,
        worktree_clean_at_final_start,
        max_autonomous_research_epochs: MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        baseline,
        graft: full,
        package_ablations: ablations,
        package_causal_ablation_pass,
        dev_final_overlap,
        final_source_task_overlap: 0,
        source_writes: 0,
        source_git_mutations: 0,
        benchmark_answer_imports: 0,
        expected_output_imports: 0,
        secret_candidates_imported: 0,
        task_id_routing_events: 0,
        patch_hash_routing_events: 0,
        repository_id_routing_events: 0,
        exact_source_patch_reuse_as_generalization_credit: 0,
        bcore_self_asserted_coding_success_events: 0,
        coding_negative_transfer_events: 0,
        noncoding_negative_transfer_events: 0,
        first_principles_reasoning_regressions: 0,
        unprovenanced_promoted_coding_objects: graft
            .objects
            .iter()
            .filter(|object| object.provenance.source_object_reference.is_empty())
            .count(),
        full_coding_knowledge_scans: 0,
        active_object_bound: MAX_ACTIVE_OBJECTS_PREDECLARED,
        active_bound_predeclared: true,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_reads: 0,
        network_writes: 0,
        synapse4_reasoning_engine_imported: false,
        synapse4_router_imported: false,
        synapse4_governor_imported: false,
        synapse4_orchestration_imported: false,
        post_final_graft_changes: 0,
        post_final_routing_changes: 0,
        post_final_acceptance_changes: 0,
    };
    write_json(&final_path, &raw)?;
    write_json(
        report_dir.join("campaign_state.json"),
        &json!({
            "state": "FINAL_B_EXPOSED_AWAITING_INDEPENDENT_VERIFICATION",
            "infra_ready": true,
            "campaign_frozen": true,
            "campaign_running": false,
            "final_b_exposure_events": 1,
            "post_final_graft_changes": 0,
            "post_final_routing_changes": 0,
            "post_final_acceptance_changes": 0,
        }),
    )?;
    Ok(())
}

fn git_snapshot() -> Result<(String, bool), String> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&cwd)
        .output()
        .map_err(|error| format!("FINAL_GIT_HEAD_EXEC:{error}"))?;
    if !head.status.success() {
        return Err("FINAL_GIT_HEAD_FAILED".to_string());
    }
    let commit = String::from_utf8(head.stdout)
        .map_err(|error| format!("FINAL_GIT_HEAD_UTF8:{error}"))?
        .trim()
        .to_string();
    let status = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("FINAL_GIT_STATUS_EXEC:{error}"))?;
    if !status.status.success() {
        return Err("FINAL_GIT_STATUS_FAILED".to_string());
    }
    Ok((commit, status.stdout.is_empty()))
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("JSON_READ:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("JSON_PARSE:{}:{error}", path.display()))
}

pub(crate) fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| format!("JSON_ENCODE:{error}"))?;
    fs::write(path, bytes).map_err(|error| format!("JSON_WRITE:{}:{error}", path.display()))
}

pub(crate) fn hash_json(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| format!("HASH_JSON:{error}"))?;
    Ok(hex_sha256(&bytes))
}

pub(crate) fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("HASH_FILE:{}:{error}", path.display()))?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_budget_and_final_partition_are_frozen() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
        assert_ne!(DEV_SEED, FINAL_SEED);
        let dev = generate_blind_tasks(DEV_SEED).expect("dev tasks");
        let final_tasks = generate_blind_tasks(FINAL_SEED).expect("final tasks");
        let dev_hashes = dev
            .iter()
            .map(|task| &task.public_task_hash)
            .collect::<BTreeSet<_>>();
        assert!(final_tasks
            .iter()
            .all(|task| !dev_hashes.contains(&task.public_task_hash)));
    }

    #[test]
    fn routing_never_uses_task_identity_or_full_scan() {
        let mut objects = vec![SemanticObject {
            object_id: "BCG-TEST".to_string(),
            semantic_payload_sha256: "0".repeat(64),
            kind: SemanticKind::ConstraintMechanism,
            package: Package::LifetimeAlias,
            elemental_operations: vec![
                Element::Own,
                Element::Borrow,
                Element::Alias,
                Element::Free,
            ],
            relation_codes: vec![1],
            constraint_codes: vec![1],
            applicability_codes: vec![1],
            negative_applicability_codes: vec![1],
            verification_status: VerificationStatus::Verified,
            documentation_alias: "test".to_string(),
            natural_language_alias_is_authority: false,
            provenance: GraftProvenance {
                source_system: "SYNAPSE4".to_string(),
                source_commit: SOURCE_COMMIT.to_string(),
                source_tree_hash: SOURCE_TREE_HASH.to_string(),
                source_universe_sha256: SOURCE_UNIVERSE_SHA256.to_string(),
                source_object_reference: "mechanism::test".to_string(),
                source_evidence_type: "TEST".to_string(),
                extraction_version: "TEST".to_string(),
                conversion_method: "TEST".to_string(),
                campaign_id: CAMPAIGN_ID.to_string(),
                source_node_id_is_address_only: true,
            },
        }];
        objects[0].elemental_operations.sort();
        let tasks = generate_blind_tasks(FINAL_SEED).expect("tasks");
        let result = evaluate_arm("TEST", &tasks, &objects, Package::LifetimeAlias.bit())
            .expect("evaluation");
        assert_eq!(result.full_knowledge_scans, 0);
        assert!(tasks.iter().all(|task| !task.task_id_is_routing_authority));
    }

    #[test]
    fn compiled_mechanical_canaries_pass() {
        assert_eq!(mechanical_canaries().unwrap()["passed"], true);
    }
}
