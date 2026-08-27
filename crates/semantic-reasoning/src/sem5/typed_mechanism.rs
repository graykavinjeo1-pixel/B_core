//! Name-independent lowering from a typed mechanism goal to executable SEM-5
//! syntax.  This is deliberately a compiler boundary: the goal supplies typed
//! operand roles and an expression graph, while this module resolves those
//! roles to concrete repository expressions, checks types/effects, parses the
//! resulting Rust syntax, and falsifies it against public observations.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::bounded_parallel::{
    map_ordered_batched as parallel_map_ordered_batched, worker_count_for,
};
use crate::self_repair_contract::sha256;

use super::ir::eval_scalar;
use super::model::{
    ApiDefinition, BinaryOperator, BindingSpec, DataSplit, Effect, ProgramTask, ProgramType,
    RelationSpec, ScalarExpression, StringTransformOperator, UnaryOperator, Value,
};
use super::tasks::evaluate_contract;

pub const TYPED_MECHANISM_GOAL_SCHEMA: &str = "B_CORE_TYPED_MECHANISM_GOAL_1";
pub const TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA: &str = "B_CORE_TYPED_MECHANISM_SYNTHESIS_GOAL_1";
pub const CONCRETE_SYNTAX_TEMPLATE_SCHEMA: &str = "B_CORE_CONCRETE_SYNTAX_TEMPLATE_1";
pub const SOURCE_BOUND_OPERATOR_AUTHORITY_SCHEMA: &str = "B_CORE_SOURCE_BOUND_OPERATOR_AUTHORITY_1";
pub const INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA: &str =
    "B_CORE_INSTALLED_TYPED_OPERATOR_AUTHORITY_1";
pub const NATIVE_TYPED_OPERATOR_GENESIS_SCHEMA: &str = "B_CORE_NATIVE_TYPED_OPERATOR_GENESIS_1";
pub const MAX_ACTIVE_TYPED_MECHANISM_OPERATORS: usize = 256;
const MAX_MECHANISM_OPERANDS: usize = 32;
const MAX_MECHANISM_EXPRESSION_NODES: usize = 256;
const MAX_MECHANISM_OBSERVATIONS: usize = 64;
const MAX_SYNTHESIS_CANDIDATES: usize = 1_024;
const MAX_SYNTHESIS_DEPTH: usize = 3;
const MAX_IDENTIFIABILITY_PROBES: usize = 64;
const MAX_IDENTIFIABILITY_HYPOTHESES: usize = 64;
const MAX_SOURCE_SEED_EXPRESSIONS: usize = 64;
const TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceOperandIR {
    /// Stable semantic role used by the mechanism graph.
    pub role: String,
    /// Concrete Rust expression selected from the repository AST.
    pub source: String,
    pub value_type: ProgramType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "syntax_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypedSyntaxExpressionIR {
    Operand {
        role: String,
    },
    IntLiteral {
        value: i64,
    },
    BoolLiteral {
        value: bool,
    },
    StringLiteral {
        value: String,
    },
    Unary {
        operator: UnaryOperator,
        input: Box<TypedSyntaxExpressionIR>,
    },
    StringTransform {
        operator: StringTransformOperator,
        input: Box<TypedSyntaxExpressionIR>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<TypedSyntaxExpressionIR>,
        right: Box<TypedSyntaxExpressionIR>,
    },
    Length {
        input: Box<TypedSyntaxExpressionIR>,
    },
    Index {
        collection: Box<TypedSyntaxExpressionIR>,
        index: Box<TypedSyntaxExpressionIR>,
    },
    Call {
        api_token: String,
        arguments: Vec<TypedSyntaxExpressionIR>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismObservationIR {
    pub operands: BTreeMap<String, Value>,
    pub expected_postimage: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismGoalIR {
    pub schema: String,
    pub goal_id: String,
    pub split: DataSplit,
    pub operands: Vec<SourceOperandIR>,
    pub output_type: ProgramType,
    pub condition: Option<TypedSyntaxExpressionIR>,
    pub postimage: TypedSyntaxExpressionIR,
    pub otherwise: Option<TypedSyntaxExpressionIR>,
    pub definitions: Vec<ApiDefinition>,
    pub allowed_effects: Vec<Effect>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub public_observations: Vec<TypedMechanismObservationIR>,
    pub provenance: Vec<String>,
}

/// A synthesis goal deliberately omits condition/postimage syntax.  Those
/// expressions must be discovered from the typed operands, API signatures,
/// and public observations rather than selected by a task-name switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismSynthesisGoalIR {
    pub schema: String,
    pub goal_id: String,
    pub split: DataSplit,
    pub operands: Vec<SourceOperandIR>,
    pub output_type: ProgramType,
    pub definitions: Vec<ApiDefinition>,
    pub allowed_effects: Vec<Effect>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub public_observations: Vec<TypedMechanismObservationIR>,
    pub require_conditional: bool,
    pub max_expression_depth: usize,
    pub max_candidates: usize,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismSynthesisReceiptIR {
    pub schema: String,
    pub goal_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthesis_request: Option<TypedMechanismSynthesisGoalIR>,
    pub candidates_enumerated: usize,
    pub candidates_falsified: usize,
    pub counterexample_guided_selection: bool,
    pub conditional_synthesized: bool,
    pub winning_expression_nodes: usize,
    #[serde(default)]
    pub preferred_operator_attempts: usize,
    #[serde(default)]
    pub preferred_operator_selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_operator_id: Option<String>,
    #[serde(default)]
    pub attempted_operator_ids: Vec<String>,
    #[serde(default)]
    pub rejected_operator_ids: Vec<String>,
    #[serde(default)]
    pub parallel_operator_evaluation: bool,
    pub winning_goal: TypedMechanismGoalIR,
    pub template: ConcreteSyntaxTemplateIR,
    pub receipt_sha256: String,
}

/// A name-independent, content-addressed expression recipe retained from a
/// previously falsified and execution-verified typed repair. Operand roles
/// are canonical ARG_0..ARG_N positions so the recipe can be transported to a
/// fresh repository without retaining source identifiers or patch text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismImprovementOperatorIR {
    pub schema: String,
    pub operator_id: String,
    pub operand_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub condition: Option<TypedSyntaxExpressionIR>,
    pub postimage: TypedSyntaxExpressionIR,
    pub otherwise: Option<TypedSyntaxExpressionIR>,
    pub validation_contract: Vec<String>,
    pub evidence_sha256: String,
}

/// A derived operator graph proposal and the public behavior contract that
/// independently falsifies it. Component authority permits exploration only;
/// the derived operator must still pass the normal verifier and promotion
/// boundary before it can enter the authorized repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismOperatorCompositionIR {
    pub schema: String,
    pub goal: TypedMechanismSynthesisGoalIR,
    pub operator_proposal: TypedMechanismImprovementOperatorIR,
    pub producer_operator_id: String,
    pub consumer_operator_id: String,
    pub wire_index: usize,
}

/// A product-owned grammar primitive proposed by B_Core itself.  It is not an
/// authority receipt: the ordinary fresh-source, hidden-case, campaign, and
/// promotion boundaries must still accept it before it can be executed from
/// the improvement-operator repository.  Keeping genesis in the compiler
/// removes any runtime or packaging dependency on an external knowledge tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTypedOperatorGenesisIR {
    pub schema: String,
    pub primitive_id: String,
    pub goal: TypedMechanismSynthesisGoalIR,
    pub operator_proposal: TypedMechanismImprovementOperatorIR,
}

/// Immutable execution authority for a reusable typed expression operator.
/// The historical directory names remain stable, but the operator itself is
/// language-neutral and may be consumed by both Python and Rust source
/// frontends after an isolated-sandbox or installed-repair receipt is
/// validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismOperatorAuthorityReceiptIR {
    pub schema: String,
    pub authority_id: String,
    pub operator_id: String,
    pub operator_sha256: String,
    pub repair_id: String,
    pub repair_receipt_sha256: String,
    pub sandbox_output_sha256: String,
    pub candidate_sha256: String,
    pub sandbox_verified: bool,
    pub sandbox_cleaned: bool,
    pub authoritative_scope_stable: bool,
    pub candidate_installed: bool,
    pub authoritative_source_write_events: u64,
    pub codex_calls: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    #[serde(default, skip_serializing_if = "u64_is_zero")]
    pub promotion_generation: u64,
    pub receipt_sha256: String,
}

/// The only write contract accepted by the typed-operator authority store.
/// Producers may discover operators in different subsystems, but none of
/// them owns repository writes or authority construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismOperatorPromotionEvidenceIR {
    pub repair_id: String,
    pub repair_receipt_sha256: String,
    pub execution_output_sha256: String,
    pub candidate_sha256: String,
    pub sandbox_verified: bool,
    pub sandbox_cleaned: bool,
    pub authoritative_scope_stable: bool,
    pub candidate_installed: bool,
    pub authoritative_source_write_events: u64,
    pub codex_calls: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub promotion_generation: u64,
}

fn u64_is_zero(value: &u64) -> bool {
    *value == 0
}

fn json_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("TYPED_OPERATOR_JSON:{error}"))
}

pub fn typed_mechanism_operator_directory(state_dir: &Path) -> PathBuf {
    state_dir
        .join("improvement_operator_repository")
        .join("source_bound_operators")
}

pub fn typed_mechanism_operator_authority_directory(state_dir: &Path) -> PathBuf {
    state_dir
        .join("improvement_operator_repository")
        .join("source_bound_authority")
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("TYPED_OPERATOR_REPOSITORY_JSON:{error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            format!(
                "TYPED_OPERATOR_REPOSITORY_CREATE:{}:{error}",
                path.display()
            )
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("TYPED_OPERATOR_REPOSITORY_WRITE:{}:{error}", path.display()))
}

fn operator_identity_without_evidence(
    operator: &TypedMechanismImprovementOperatorIR,
) -> TypedMechanismImprovementOperatorIR {
    let mut identity = operator.clone();
    identity.evidence_sha256.clear();
    identity
}

/// Atomically persists a reusable typed operator and constructs its immutable
/// execution authority. This is deliberately shared by sandbox-verified
/// generative programs and installed source repairs so proposal producers do
/// not compete for repository or authority ownership.
pub fn persist_authorized_typed_mechanism_operator(
    state_dir: &Path,
    requested_operator: &TypedMechanismImprovementOperatorIR,
    evidence: &TypedMechanismOperatorPromotionEvidenceIR,
) -> Result<TypedMechanismOperatorAuthorityReceiptIR, String> {
    validate_typed_mechanism_improvement_operator(requested_operator)?;
    let sandbox_authority =
        !evidence.candidate_installed && evidence.authoritative_source_write_events == 0;
    let installed_authority =
        evidence.candidate_installed && evidence.authoritative_source_write_events == 1;
    if (!sandbox_authority && !installed_authority)
        || !evidence.sandbox_verified
        || !evidence.sandbox_cleaned
        || !evidence.authoritative_scope_stable
        || evidence.repair_id.len() != 64
        || evidence.repair_receipt_sha256.len() != 64
        || evidence.execution_output_sha256.len() != 64
        || evidence.candidate_sha256.len() != 64
        || evidence.codex_calls != 0
        || evidence.external_llm_calls != 0
        || evidence.network_reads != 0
        || evidence.network_writes != 0
        || requested_operator.evidence_sha256 != evidence.execution_output_sha256
    {
        return Err("TYPED_OPERATOR_PROMOTION_EVIDENCE_INVALID".to_string());
    }

    let operator_directory = typed_mechanism_operator_directory(state_dir);
    let authority_directory = typed_mechanism_operator_authority_directory(state_dir);
    fs::create_dir_all(&operator_directory)
        .map_err(|error| format!("TYPED_OPERATOR_REPOSITORY_DIRECTORY:{error}"))?;
    fs::create_dir_all(&authority_directory)
        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_DIRECTORY:{error}"))?;
    let operator_path = operator_directory.join(format!("{}.json", requested_operator.operator_id));
    let operator =
        if operator_path.exists() {
            let bytes = fs::read(&operator_path)
                .map_err(|error| format!("TYPED_OPERATOR_REPOSITORY_READ:{error}"))?;
            let stored: TypedMechanismImprovementOperatorIR = serde_json::from_slice(&bytes)
                .map_err(|error| format!("TYPED_OPERATOR_REPOSITORY_PARSE:{error}"))?;
            validate_typed_mechanism_improvement_operator(&stored)?;
            if operator_identity_without_evidence(&stored)
                != operator_identity_without_evidence(requested_operator)
            {
                return Err("TYPED_OPERATOR_REPOSITORY_COLLISION".to_string());
            }
            if stored.evidence_sha256 != evidence.execution_output_sha256 {
                // Immutable first evidence remains authoritative. If it already
                // has a valid authority, this promotion is an idempotent replay.
                let authorized = load_authorized_typed_mechanism_operators(state_dir, usize::MAX)?
                    .iter()
                    .any(|operator| operator.operator_id == stored.operator_id);
                if authorized {
                    let mut paths = fs::read_dir(&authority_directory)
                        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_READ_DIR:{error}"))?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_ENTRY:{error}"))?
                        .into_iter()
                        .map(|entry| entry.path())
                        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
                        .collect::<Vec<_>>();
                    paths.sort();
                    for path in paths {
                        let authority: TypedMechanismOperatorAuthorityReceiptIR =
                            serde_json::from_slice(&fs::read(&path).map_err(|error| {
                                format!("TYPED_OPERATOR_AUTHORITY_READ:{error}")
                            })?)
                            .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_PARSE:{error}"))?;
                        validate_typed_mechanism_operator_authority(&authority)?;
                        if authority.operator_id == stored.operator_id {
                            return Ok(authority);
                        }
                    }
                }
                return Err("TYPED_OPERATOR_UNAUTHORIZED_FIRST_EVIDENCE".to_string());
            }
            stored
        } else {
            write_new_json(&operator_path, requested_operator)?;
            requested_operator.clone()
        };

    let operator_sha256 = json_sha256(&operator)?;
    let mut existing_authorities = fs::read_dir(&authority_directory)
        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_ENTRY:{error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
        .collect::<Vec<_>>();
    existing_authorities.sort();
    for path in existing_authorities {
        let existing: TypedMechanismOperatorAuthorityReceiptIR = serde_json::from_slice(
            &fs::read(&path).map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_READ:{error}"))?,
        )
        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_PARSE:{error}"))?;
        validate_typed_mechanism_operator_authority(&existing)?;
        if existing.operator_id == operator.operator_id
            && existing.operator_sha256 == operator_sha256
            && existing.sandbox_output_sha256 == operator.evidence_sha256
        {
            return Ok(existing);
        }
    }
    let authority_prefix = if installed_authority {
        "INSTALLED_TYPED_OPERATOR_AUTHORITY_1"
    } else {
        "SOURCE_BOUND_OPERATOR_AUTHORITY_1"
    };
    let authority_id = sha256(
        format!(
            "{authority_prefix}:{}:{}:{}:{}",
            operator.operator_id,
            evidence.repair_id,
            evidence.repair_receipt_sha256,
            operator.evidence_sha256
        )
        .as_bytes(),
    );
    let mut authority = TypedMechanismOperatorAuthorityReceiptIR {
        schema: if installed_authority {
            INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA.to_string()
        } else {
            SOURCE_BOUND_OPERATOR_AUTHORITY_SCHEMA.to_string()
        },
        authority_id: authority_id.clone(),
        operator_id: operator.operator_id.clone(),
        operator_sha256,
        repair_id: evidence.repair_id.clone(),
        repair_receipt_sha256: evidence.repair_receipt_sha256.clone(),
        sandbox_output_sha256: operator.evidence_sha256,
        candidate_sha256: evidence.candidate_sha256.clone(),
        sandbox_verified: evidence.sandbox_verified,
        sandbox_cleaned: evidence.sandbox_cleaned,
        authoritative_scope_stable: evidence.authoritative_scope_stable,
        candidate_installed: evidence.candidate_installed,
        authoritative_source_write_events: evidence.authoritative_source_write_events,
        codex_calls: evidence.codex_calls,
        external_llm_calls: evidence.external_llm_calls,
        network_reads: evidence.network_reads,
        network_writes: evidence.network_writes,
        promotion_generation: evidence.promotion_generation,
        receipt_sha256: String::new(),
    };
    authority.receipt_sha256 = json_sha256(&authority)?;
    validate_typed_mechanism_operator_authority(&authority)?;
    let authority_path = authority_directory.join(format!("{authority_id}.json"));
    if authority_path.exists() {
        let stored: TypedMechanismOperatorAuthorityReceiptIR = serde_json::from_slice(
            &fs::read(&authority_path)
                .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_READ:{error}"))?,
        )
        .map_err(|error| format!("TYPED_OPERATOR_AUTHORITY_PARSE:{error}"))?;
        if stored != authority {
            return Err("TYPED_OPERATOR_AUTHORITY_COLLISION".to_string());
        }
    } else {
        write_new_json(&authority_path, &authority)?;
    }
    Ok(authority)
}

pub fn validate_typed_mechanism_operator_authority(
    authority: &TypedMechanismOperatorAuthorityReceiptIR,
) -> Result<(), String> {
    let authority_prefix = match authority.schema.as_str() {
        SOURCE_BOUND_OPERATOR_AUTHORITY_SCHEMA
            if !authority.candidate_installed
                && authority.authoritative_source_write_events == 0 =>
        {
            "SOURCE_BOUND_OPERATOR_AUTHORITY_1"
        }
        INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA
            if authority.candidate_installed
                && authority.authoritative_source_write_events == 1 =>
        {
            "INSTALLED_TYPED_OPERATOR_AUTHORITY_1"
        }
        _ => return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_ENVELOPE".to_string()),
    };
    if authority.authority_id.len() != 64
        || authority.operator_id.len() != 64
        || authority.operator_sha256.len() != 64
        || authority.repair_id.len() != 64
        || authority.repair_receipt_sha256.len() != 64
        || authority.sandbox_output_sha256.len() != 64
        || authority.candidate_sha256.len() != 64
        || !authority.sandbox_verified
        || !authority.sandbox_cleaned
        || !authority.authoritative_scope_stable
        || authority.codex_calls != 0
        || authority.external_llm_calls != 0
        || authority.network_reads != 0
        || authority.network_writes != 0
    {
        return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_ENVELOPE".to_string());
    }
    let expected_authority_id = sha256(
        format!(
            "{authority_prefix}:{}:{}:{}:{}",
            authority.operator_id,
            authority.repair_id,
            authority.repair_receipt_sha256,
            authority.sandbox_output_sha256
        )
        .as_bytes(),
    );
    if authority.authority_id != expected_authority_id {
        return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_ID_MISMATCH".to_string());
    }
    let mut identity = authority.clone();
    identity.receipt_sha256.clear();
    if json_sha256(&identity)? != authority.receipt_sha256 {
        return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_HASH_MISMATCH".to_string());
    }
    Ok(())
}

pub fn select_bounded_typed_mechanism_operator_ids(
    operator_ids: impl IntoIterator<Item = String>,
    latest_verified_generation: &BTreeMap<String, u64>,
    limit: usize,
) -> Vec<String> {
    let mut selected = operator_ids.into_iter().collect::<Vec<_>>();
    selected.sort();
    selected.dedup();
    selected.sort_by(|left, right| {
        latest_verified_generation
            .get(right)
            .copied()
            .unwrap_or_default()
            .cmp(
                &latest_verified_generation
                    .get(left)
                    .copied()
                    .unwrap_or_default(),
            )
            .then_with(|| left.cmp(right))
    });
    selected.truncate(limit);
    selected
}

/// Load only operators backed by an immutable execution-authority receipt.
/// Missing repositories are an empty snapshot; malformed authority is a hard
/// failure rather than an untrusted fallback.
pub fn load_authorized_typed_mechanism_operators(
    state_dir: &Path,
    limit: usize,
) -> Result<Vec<TypedMechanismImprovementOperatorIR>, String> {
    let directory = typed_mechanism_operator_directory(state_dir);
    let authority_directory = typed_mechanism_operator_authority_directory(state_dir);
    if !directory.is_dir() || !authority_directory.is_dir() || limit == 0 {
        return Ok(Vec::new());
    }
    let mut authority_paths = fs::read_dir(&authority_directory)
        .map_err(|error| format!("SOURCE_BOUND_OPERATOR_AUTHORITY_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_BOUND_OPERATOR_AUTHORITY_ENTRY:{error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
        .collect::<Vec<_>>();
    authority_paths.sort();
    let mut authorized_operator_evidence = BTreeSet::new();
    let mut latest_verified_generation = BTreeMap::<String, u64>::new();
    for path in authority_paths {
        let bytes = fs::read(&path)
            .map_err(|error| format!("SOURCE_BOUND_OPERATOR_AUTHORITY_READ:{error}"))?;
        let authority: TypedMechanismOperatorAuthorityReceiptIR = serde_json::from_slice(&bytes)
            .map_err(|error| format!("SOURCE_BOUND_OPERATOR_AUTHORITY_JSON:{error}"))?;
        validate_typed_mechanism_operator_authority(&authority)?;
        if path.file_stem().and_then(OsStr::to_str) != Some(authority.authority_id.as_str()) {
            return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_PATH_ID_MISMATCH".to_string());
        }
        latest_verified_generation
            .entry(authority.operator_id.clone())
            .and_modify(|generation| {
                *generation = (*generation).max(authority.promotion_generation)
            })
            .or_insert(authority.promotion_generation);
        authorized_operator_evidence.insert((
            authority.operator_id,
            authority.operator_sha256,
            authority.sandbox_output_sha256,
        ));
    }

    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("SOURCE_BOUND_OPERATOR_REPOSITORY_READ_DIR:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_BOUND_OPERATOR_REPOSITORY_ENTRY:{error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut operators = BTreeMap::new();
    for path in paths {
        let bytes = fs::read(&path)
            .map_err(|error| format!("SOURCE_BOUND_OPERATOR_REPOSITORY_READ:{error}"))?;
        let operator: TypedMechanismImprovementOperatorIR = serde_json::from_slice(&bytes)
            .map_err(|error| format!("SOURCE_BOUND_OPERATOR_REPOSITORY_JSON:{error}"))?;
        validate_typed_mechanism_improvement_operator(&operator)?;
        if path.file_stem().and_then(OsStr::to_str) != Some(operator.operator_id.as_str()) {
            return Err("SOURCE_BOUND_OPERATOR_REPOSITORY_PATH_ID_MISMATCH".to_string());
        }
        let operator_sha256 = json_sha256(&operator)?;
        if authorized_operator_evidence.contains(&(
            operator.operator_id.clone(),
            operator_sha256,
            operator.evidence_sha256.clone(),
        )) {
            operators.insert(operator.operator_id.clone(), operator);
        }
    }
    let selected_ids = select_bounded_typed_mechanism_operator_ids(
        operators.keys().cloned(),
        &latest_verified_generation,
        limit.min(MAX_ACTIVE_TYPED_MECHANISM_OPERATORS),
    );
    Ok(selected_ids
        .into_iter()
        .filter_map(|operator_id| operators.remove(&operator_id))
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcreteSyntaxTemplateIR {
    pub schema: String,
    pub goal_id: String,
    pub source_operands: Vec<SourceOperandIR>,
    pub condition_source: Option<String>,
    pub postimage_source: String,
    pub otherwise_source: Option<String>,
    pub complete_expression_source: String,
    pub canonical_compilable_source: String,
    pub program_task: ProgramTask,
    pub expression_nodes: usize,
    pub recombinations: usize,
    pub public_observations_checked: usize,
    pub public_observations_passed: usize,
    pub syntax_parse_pass: bool,
    pub type_effect_check_pass: bool,
    pub syntax_sha256: String,
}

/// Compile an abstract typed mechanism into both transplantable repository
/// syntax and an executable SEM-5 task.  No goal/task name participates in
/// expression selection; only operand roles, types, operators, and API
/// signatures are consulted.
pub fn lower_typed_mechanism_goal(
    goal: &TypedMechanismGoalIR,
) -> Result<ConcreteSyntaxTemplateIR, String> {
    validate_goal_envelope(goal)?;

    let operand_types = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.source.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_indices = goal
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let definitions = goal
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();

    let mut effects = BTreeSet::new();
    let condition_type = goal
        .condition
        .as_ref()
        .map(|condition| {
            infer_expression_type(condition, &operand_types, &definitions, &mut effects)
        })
        .transpose()?;
    if condition_type
        .as_ref()
        .is_some_and(|kind| *kind != ProgramType::Bool)
    {
        return Err("TYPED_MECHANISM_CONDITION_NOT_BOOL".to_string());
    }
    let postimage_type =
        infer_expression_type(&goal.postimage, &operand_types, &definitions, &mut effects)?;
    if postimage_type != goal.output_type {
        return Err(format!(
            "TYPED_MECHANISM_POSTIMAGE_TYPE:{postimage_type:?}:{:?}",
            goal.output_type
        ));
    }
    let otherwise_type = goal
        .otherwise
        .as_ref()
        .map(|otherwise| {
            infer_expression_type(otherwise, &operand_types, &definitions, &mut effects)
        })
        .transpose()?;
    match (&goal.condition, &goal.otherwise, otherwise_type) {
        (Some(_), Some(_), Some(kind)) if kind == goal.output_type => {}
        (Some(_), Some(_), Some(kind)) => {
            return Err(format!(
                "TYPED_MECHANISM_OTHERWISE_TYPE:{kind:?}:{:?}",
                goal.output_type
            ));
        }
        (Some(_), None, _) => return Err("TYPED_MECHANISM_OTHERWISE_MISSING".to_string()),
        (None, Some(_), _) => {
            return Err("TYPED_MECHANISM_OTHERWISE_WITHOUT_CONDITION".to_string());
        }
        (None, None, _) => {}
        _ => return Err("TYPED_MECHANISM_OTHERWISE_INVALID".to_string()),
    }
    if !effects
        .iter()
        .all(|effect| goal.allowed_effects.contains(effect))
    {
        return Err("TYPED_MECHANISM_EFFECT_NOT_ALLOWED".to_string());
    }

    let condition_source = goal
        .condition
        .as_ref()
        .map(|condition| emit_expression(condition, &operand_sources, &operand_types, &definitions))
        .transpose()?;
    let postimage_source = emit_expression(
        &goal.postimage,
        &operand_sources,
        &operand_types,
        &definitions,
    )?;
    let otherwise_source = goal
        .otherwise
        .as_ref()
        .map(|otherwise| emit_expression(otherwise, &operand_sources, &operand_types, &definitions))
        .transpose()?;
    let complete_expression_source = complete_expression(
        condition_source.as_deref(),
        &postimage_source,
        otherwise_source.as_deref(),
    )?;
    syn::parse_str::<syn::Expr>(&complete_expression_source)
        .map_err(|error| format!("TYPED_MECHANISM_SOURCE_PARSE:{error}"))?;

    let canonical_sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.role.clone()))
        .collect::<BTreeMap<_, _>>();
    let canonical_condition = goal
        .condition
        .as_ref()
        .map(|condition| {
            emit_expression(condition, &canonical_sources, &operand_types, &definitions)
        })
        .transpose()?;
    let canonical_postimage = emit_expression(
        &goal.postimage,
        &canonical_sources,
        &operand_types,
        &definitions,
    )?;
    let canonical_otherwise = goal
        .otherwise
        .as_ref()
        .map(|otherwise| {
            emit_expression(otherwise, &canonical_sources, &operand_types, &definitions)
        })
        .transpose()?;
    let canonical_expression = complete_expression(
        canonical_condition.as_deref(),
        &canonical_postimage,
        canonical_otherwise.as_deref(),
    )?;
    let parameters = goal
        .operands
        .iter()
        .map(|operand| format!("{}: {}", operand.role, rust_type(&operand.value_type)))
        .collect::<Vec<_>>()
        .join(", ");
    let canonical_compilable_source = format!(
        "fn __b_core_typed_mechanism({parameters}) -> {} {{ {canonical_expression} }}",
        rust_type(&goal.output_type)
    );
    syn::parse_file(&canonical_compilable_source)
        .map_err(|error| format!("TYPED_MECHANISM_TEMPLATE_PARSE:{error}"))?;

    let condition = goal
        .condition
        .as_ref()
        .map(|condition| lower_expression(condition, &operand_indices))
        .transpose()?;
    let postimage = lower_expression(&goal.postimage, &operand_indices)?;
    let otherwise = goal
        .otherwise
        .as_ref()
        .map(|otherwise| lower_expression(otherwise, &operand_indices))
        .transpose()?;
    let inputs = goal
        .operands
        .iter()
        .map(|operand| BindingSpec {
            name: operand.role.clone(),
            value_type: operand.value_type.clone(),
            mutable: false,
        })
        .collect::<Vec<_>>();
    let program_task = ProgramTask {
        task_id: goal.goal_id.clone(),
        split: goal.split,
        inputs,
        output_type: goal.output_type.clone(),
        relation: RelationSpec::Mechanism {
            condition,
            postimage,
            otherwise,
        },
        definitions: goal.definitions.clone(),
        allowed_effects: goal.allowed_effects.clone(),
        preconditions: goal.preconditions.clone(),
        postconditions: goal.postconditions.clone(),
        invariants: goal.invariants.clone(),
        demonstrations: goal
            .public_observations
            .iter()
            .map(|observation| {
                goal.operands
                    .iter()
                    .filter_map(|operand| observation.operands.get(&operand.role).cloned())
                    .collect()
            })
            .collect(),
        provenance: goal
            .provenance
            .iter()
            .cloned()
            .chain(["TYPED_MECHANISM_TO_CONCRETE_SYNTAX".to_string()])
            .collect(),
    };

    for (index, observation) in goal.public_observations.iter().enumerate() {
        validate_observation_bindings(goal, observation)?;
        let actual = evaluate_contract(&program_task, &observation.operands)
            .map_err(|error| format!("TYPED_MECHANISM_OBSERVATION_EXECUTE:{index}:{error}"))?;
        if actual != observation.expected_postimage {
            return Err(format!("TYPED_MECHANISM_COUNTEREXAMPLE:{index}"));
        }
    }

    let expression_nodes = goal
        .condition
        .iter()
        .map(expression_nodes)
        .sum::<usize>()
        .saturating_add(expression_nodes(&goal.postimage))
        .saturating_add(goal.otherwise.iter().map(expression_nodes).sum::<usize>());
    let recombinations = expression_nodes.saturating_sub(1);
    let syntax_sha256 = sha256(
        serde_json::to_vec(&(
            CONCRETE_SYNTAX_TEMPLATE_SCHEMA,
            &goal.goal_id,
            &goal.operands,
            &complete_expression_source,
            &program_task,
        ))
        .map_err(|error| format!("TYPED_MECHANISM_TEMPLATE_SERIALIZE:{error}"))?
        .as_slice(),
    );

    Ok(ConcreteSyntaxTemplateIR {
        schema: CONCRETE_SYNTAX_TEMPLATE_SCHEMA.to_string(),
        goal_id: goal.goal_id.clone(),
        source_operands: goal.operands.clone(),
        condition_source,
        postimage_source,
        otherwise_source,
        complete_expression_source,
        canonical_compilable_source,
        program_task,
        expression_nodes,
        recombinations,
        public_observations_checked: goal.public_observations.len(),
        public_observations_passed: goal.public_observations.len(),
        syntax_parse_pass: true,
        type_effect_check_pass: true,
        syntax_sha256,
    })
}

#[derive(Debug, Clone)]
struct EnumeratedExpression {
    expression: TypedSyntaxExpressionIR,
    value_type: ProgramType,
    outputs: Vec<Value>,
    nodes: usize,
    canonical_key: String,
}

type ConditionalBranchIndex = BTreeMap<Vec<bool>, (Vec<usize>, Vec<usize>)>;

#[derive(Debug, Clone)]
struct TransportedImprovementOperator {
    operator_id: String,
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
}

fn remap_expression_roles(
    expression: &TypedSyntaxExpressionIR,
    role_map: &BTreeMap<String, String>,
) -> Result<TypedSyntaxExpressionIR, String> {
    Ok(match expression {
        TypedSyntaxExpressionIR::Operand { role } => TypedSyntaxExpressionIR::Operand {
            role: role_map
                .get(role)
                .cloned()
                .ok_or_else(|| format!("TYPED_MECHANISM_PRIOR_ROLE_MISSING:{role}"))?,
        },
        TypedSyntaxExpressionIR::IntLiteral { value } => {
            TypedSyntaxExpressionIR::IntLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            TypedSyntaxExpressionIR::BoolLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::StringLiteral { value } => {
            TypedSyntaxExpressionIR::StringLiteral {
                value: value.clone(),
            }
        }
        TypedSyntaxExpressionIR::Unary { operator, input } => TypedSyntaxExpressionIR::Unary {
            operator: *operator,
            input: Box::new(remap_expression_roles(input, role_map)?),
        },
        TypedSyntaxExpressionIR::StringTransform { operator, input } => {
            TypedSyntaxExpressionIR::StringTransform {
                operator: *operator,
                input: Box::new(remap_expression_roles(input, role_map)?),
            }
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => TypedSyntaxExpressionIR::Binary {
            operator: *operator,
            left: Box::new(remap_expression_roles(left, role_map)?),
            right: Box::new(remap_expression_roles(right, role_map)?),
        },
        TypedSyntaxExpressionIR::Length { input } => TypedSyntaxExpressionIR::Length {
            input: Box::new(remap_expression_roles(input, role_map)?),
        },
        TypedSyntaxExpressionIR::Index { collection, index } => TypedSyntaxExpressionIR::Index {
            collection: Box::new(remap_expression_roles(collection, role_map)?),
            index: Box::new(remap_expression_roles(index, role_map)?),
        },
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => TypedSyntaxExpressionIR::Call {
            api_token: api_token.clone(),
            arguments: arguments
                .iter()
                .map(|argument| remap_expression_roles(argument, role_map))
                .collect::<Result<Vec<_>, _>>()?,
        },
    })
}

pub fn validate_typed_mechanism_improvement_operator(
    prior: &TypedMechanismImprovementOperatorIR,
) -> Result<(), String> {
    if prior.schema != "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1"
        || prior.operator_id.is_empty()
        || prior.operand_types.is_empty()
        || prior.evidence_sha256.len() != 64
        || prior.validation_contract
            != [
                "PUBLIC_OBSERVATION_REPLAY",
                "TYPE_EFFECT_CHECK",
                "SOURCE_BOUND_ATOMIC_MATERIALIZATION",
                "SANDBOX_PUBLIC_REGRESSION",
                "AUTHORITATIVE_SCOPE_STABLE",
            ]
    {
        return Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ENVELOPE".to_string());
    }
    let mut identity = prior.clone();
    identity.operator_id.clear();
    identity.evidence_sha256.clear();
    let encoded = serde_json::to_vec(&identity)
        .map_err(|error| format!("TYPED_MECHANISM_PRIOR_SERIALIZE:{error}"))?;
    if sha256(&encoded) != prior.operator_id {
        return Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ID_MISMATCH".to_string());
    }
    Ok(())
}

fn substitute_operator_expression(
    expression: &TypedSyntaxExpressionIR,
    bindings: &BTreeMap<String, TypedSyntaxExpressionIR>,
) -> Result<TypedSyntaxExpressionIR, String> {
    Ok(match expression {
        TypedSyntaxExpressionIR::Operand { role } => bindings
            .get(role)
            .cloned()
            .ok_or_else(|| format!("TYPED_OPERATOR_COMPOSITION_ROLE_MISSING:{role}"))?,
        TypedSyntaxExpressionIR::IntLiteral { value } => {
            TypedSyntaxExpressionIR::IntLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            TypedSyntaxExpressionIR::BoolLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::StringLiteral { value } => {
            TypedSyntaxExpressionIR::StringLiteral {
                value: value.clone(),
            }
        }
        TypedSyntaxExpressionIR::Unary { operator, input } => TypedSyntaxExpressionIR::Unary {
            operator: *operator,
            input: Box::new(substitute_operator_expression(input, bindings)?),
        },
        TypedSyntaxExpressionIR::StringTransform { operator, input } => {
            TypedSyntaxExpressionIR::StringTransform {
                operator: *operator,
                input: Box::new(substitute_operator_expression(input, bindings)?),
            }
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => TypedSyntaxExpressionIR::Binary {
            operator: *operator,
            left: Box::new(substitute_operator_expression(left, bindings)?),
            right: Box::new(substitute_operator_expression(right, bindings)?),
        },
        TypedSyntaxExpressionIR::Length { input } => TypedSyntaxExpressionIR::Length {
            input: Box::new(substitute_operator_expression(input, bindings)?),
        },
        TypedSyntaxExpressionIR::Index { collection, index } => TypedSyntaxExpressionIR::Index {
            collection: Box::new(substitute_operator_expression(collection, bindings)?),
            index: Box::new(substitute_operator_expression(index, bindings)?),
        },
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => TypedSyntaxExpressionIR::Call {
            api_token: api_token.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_operator_expression(argument, bindings))
                .collect::<Result<Vec<_>, _>>()?,
        },
    })
}

fn evaluate_typed_operator(
    operator: &TypedMechanismImprovementOperatorIR,
    arguments: &[Value],
) -> Result<Value, String> {
    if arguments.len() != operator.operand_types.len()
        || arguments
            .iter()
            .zip(&operator.operand_types)
            .any(|(value, expected)| value.program_type() != *expected)
    {
        return Err("TYPED_OPERATOR_COMPOSITION_ARGUMENT_TYPE".to_string());
    }
    let indices = (0..arguments.len())
        .flat_map(|index| {
            [
                (format!("ARG_{index}"), index),
                (format!("arg_{index}"), index),
            ]
        })
        .collect::<BTreeMap<_, _>>();
    let api_map = BTreeMap::new();
    match (&operator.condition, &operator.otherwise) {
        (Some(condition), Some(otherwise)) => {
            match eval_scalar(&lower_expression(condition, &indices)?, arguments, &api_map)? {
                Value::Bool(true) => eval_scalar(
                    &lower_expression(&operator.postimage, &indices)?,
                    arguments,
                    &api_map,
                ),
                Value::Bool(false) => {
                    eval_scalar(&lower_expression(otherwise, &indices)?, arguments, &api_map)
                }
                _ => Err("TYPED_OPERATOR_COMPOSITION_CONDITION_TYPE".to_string()),
            }
        }
        (None, None) => eval_scalar(
            &lower_expression(&operator.postimage, &indices)?,
            arguments,
            &api_map,
        ),
        _ => Err("TYPED_OPERATOR_COMPOSITION_CONDITIONAL_SHAPE".to_string()),
    }
}

fn typed_operator_probe_value(
    value_type: &ProgramType,
    case: usize,
    index: usize,
) -> Option<Value> {
    match value_type {
        ProgramType::Int => {
            let base = (case as i64).saturating_sub(15);
            let displacement = (index as i64).saturating_mul(3);
            Some(Value::Int(if index > 0 && case.is_multiple_of(index + 2) {
                base
            } else if (case + index).is_multiple_of(2) {
                base.saturating_add(displacement)
            } else {
                base.saturating_sub(displacement)
            }))
        }
        ProgramType::Bool => Some(Value::Bool(((case >> (index % 4)) & 1) == 1)),
        ProgramType::String => Some(Value::String(match (case + index) % 8 {
            0 => String::new(),
            1 => "A".to_string(),
            2 => "xy".to_string(),
            3 => "Rust".to_string(),
            4 => "한글".to_string(),
            5 => "  padded value  ".to_string(),
            6 => "MiXeD Case".to_string(),
            _ => format!("probe_{index}_{case}"),
        })),
        _ => None,
    }
}

fn collect_typed_expression_roles(
    expression: &TypedSyntaxExpressionIR,
    roles: &mut BTreeSet<String>,
) {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => {
            roles.insert(role.clone());
        }
        TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::BoolLiteral { .. }
        | TypedSyntaxExpressionIR::StringLiteral { .. } => {}
        TypedSyntaxExpressionIR::Unary { input, .. }
        | TypedSyntaxExpressionIR::StringTransform { input, .. }
        | TypedSyntaxExpressionIR::Length { input } => {
            collect_typed_expression_roles(input, roles);
        }
        TypedSyntaxExpressionIR::Binary { left, right, .. } => {
            collect_typed_expression_roles(left, roles);
            collect_typed_expression_roles(right, roles);
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            collect_typed_expression_roles(collection, roles);
            collect_typed_expression_roles(index, roles);
        }
        TypedSyntaxExpressionIR::Call { arguments, .. } => {
            for argument in arguments {
                collect_typed_expression_roles(argument, roles);
            }
        }
    }
}

struct CanonicalComposedOperatorInputs {
    operand_types: Vec<ProgramType>,
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
    live_argument_indices: Vec<usize>,
}

fn canonicalize_composed_operator_inputs(
    raw_operand_types: &[ProgramType],
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
) -> Result<CanonicalComposedOperatorInputs, String> {
    let mut roles = BTreeSet::new();
    for expression in condition
        .iter()
        .chain(std::iter::once(&postimage))
        .chain(otherwise.iter())
    {
        collect_typed_expression_roles(expression, &mut roles);
    }
    let mut live_indices = roles
        .iter()
        .map(|role| {
            role.strip_prefix("ARG_")
                .ok_or_else(|| format!("TYPED_OPERATOR_COMPOSITION_NONCANONICAL_ROLE:{role}"))?
                .parse::<usize>()
                .map_err(|_| format!("TYPED_OPERATOR_COMPOSITION_NONCANONICAL_ROLE:{role}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    live_indices.sort_unstable();
    live_indices.dedup();
    if live_indices.is_empty()
        || live_indices
            .iter()
            .any(|index| *index >= raw_operand_types.len())
    {
        return Err("TYPED_OPERATOR_COMPOSITION_LIVE_OPERAND_SET".to_string());
    }
    let role_map = live_indices
        .iter()
        .enumerate()
        .map(|(dense, original)| (format!("ARG_{original}"), format!("ARG_{dense}")))
        .collect::<BTreeMap<_, _>>();
    let operand_types = live_indices
        .iter()
        .map(|index| raw_operand_types[*index].clone())
        .collect();
    let condition = condition
        .as_ref()
        .map(|expression| remap_expression_roles(expression, &role_map))
        .transpose()?;
    let postimage = remap_expression_roles(&postimage, &role_map)?;
    let otherwise = otherwise
        .as_ref()
        .map(|expression| remap_expression_roles(expression, &role_map))
        .transpose()?;
    Ok(CanonicalComposedOperatorInputs {
        operand_types,
        condition,
        postimage,
        otherwise,
        live_argument_indices: live_indices,
    })
}

fn typed_operator_composition_semantic_key(
    operator: &TypedMechanismImprovementOperatorIR,
) -> Result<String, String> {
    let canonical = canonicalize_composed_operator_inputs(
        &operator.operand_types,
        operator.condition.clone(),
        operator.postimage.clone(),
        operator.otherwise.clone(),
    )?;
    json_sha256(&(
        canonical.operand_types,
        operator.output_type.clone(),
        canonical.condition,
        canonical.postimage,
        canonical.otherwise,
        operator.validation_contract.clone(),
    ))
}

fn typed_expression_references_role(
    expression: &TypedSyntaxExpressionIR,
    expected_role: &str,
) -> bool {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => role == expected_role,
        TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::BoolLiteral { .. }
        | TypedSyntaxExpressionIR::StringLiteral { .. } => false,
        TypedSyntaxExpressionIR::Unary { input, .. }
        | TypedSyntaxExpressionIR::StringTransform { input, .. }
        | TypedSyntaxExpressionIR::Length { input } => {
            typed_expression_references_role(input, expected_role)
        }
        TypedSyntaxExpressionIR::Binary { left, right, .. } => {
            typed_expression_references_role(left, expected_role)
                || typed_expression_references_role(right, expected_role)
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            typed_expression_references_role(collection, expected_role)
                || typed_expression_references_role(index, expected_role)
        }
        TypedSyntaxExpressionIR::Call { arguments, .. } => arguments
            .iter()
            .any(|argument| typed_expression_references_role(argument, expected_role)),
    }
}

fn typed_operator_operand_is_structurally_referenced(
    operator: &TypedMechanismImprovementOperatorIR,
    operand_index: usize,
) -> bool {
    let role = format!("ARG_{operand_index}");
    operator
        .condition
        .iter()
        .chain(std::iter::once(&operator.postimage))
        .chain(operator.otherwise.iter())
        .any(|expression| typed_expression_references_role(expression, &role))
}

fn typed_probe_counterfactuals(value: &Value) -> Vec<Value> {
    match value {
        Value::Int(value) => vec![
            Value::Int(value.saturating_add(1)),
            Value::Int(value.saturating_sub(1)),
            Value::Int(0),
        ],
        Value::Bool(value) => vec![Value::Bool(!value)],
        Value::String(value) => vec![
            Value::String(if value.is_empty() {
                "causal_probe".to_string()
            } else {
                String::new()
            }),
            Value::String(format!("{value}#causal_probe")),
        ],
        _ => Vec::new(),
    }
}

/// Requires more than type compatibility before one authorized operator may
/// feed another. The selected consumer operand must occur in executable
/// syntax and changing only that operand must change at least one bounded
/// public outcome. This keeps semantically dead wires from consuming the
/// finite composition frontier or creating observationally ambiguous source
/// synthesis tasks.
fn typed_operator_operand_has_observed_influence(
    operator: &TypedMechanismImprovementOperatorIR,
    operand_index: usize,
) -> Result<bool, String> {
    if !typed_operator_operand_is_structurally_referenced(operator, operand_index) {
        return Ok(false);
    }
    for case in 0..MAX_IDENTIFIABILITY_PROBES {
        let Some(arguments) = operator
            .operand_types
            .iter()
            .enumerate()
            .map(|(index, value_type)| typed_operator_probe_value(value_type, case, index))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(false);
        };
        let baseline = match evaluate_typed_operator(operator, &arguments) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(selected) = arguments.get(operand_index) else {
            return Ok(false);
        };
        for counterfactual in typed_probe_counterfactuals(selected) {
            if counterfactual == *selected {
                continue;
            }
            let mut counterfactual_arguments = arguments.clone();
            counterfactual_arguments[operand_index] = counterfactual;
            if let Ok(output) = evaluate_typed_operator(operator, &counterfactual_arguments) {
                if output != baseline {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Produces fresh, name-independent observations from an already validated
/// typed operator.  Callers use a disjoint case range from the public
/// examples so source lowering can be falsified without exposing those
/// counterexamples to the synthesizer first.
pub fn typed_mechanism_operator_probe_observations(
    operator: &TypedMechanismImprovementOperatorIR,
    case_start: usize,
    case_count: usize,
) -> Result<Vec<TypedMechanismObservationIR>, String> {
    validate_typed_mechanism_improvement_operator(operator)?;
    if case_count == 0 || case_count > 32 {
        return Err("TYPED_OPERATOR_PROBE_CASE_BUDGET".to_string());
    }
    let mut observations = Vec::with_capacity(case_count);
    for case in case_start..case_start.saturating_add(case_count) {
        let arguments = operator
            .operand_types
            .iter()
            .enumerate()
            .map(|(index, value_type)| {
                typed_operator_probe_value(value_type, case, index)
                    .ok_or_else(|| "TYPED_OPERATOR_PROBE_UNSUPPORTED_TYPE".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected_postimage = evaluate_typed_operator(operator, &arguments)?;
        let operands = arguments
            .into_iter()
            .enumerate()
            .map(|(index, value)| (format!("ARG_{index}"), value))
            .collect();
        observations.push(TypedMechanismObservationIR {
            operands,
            expected_postimage,
        });
    }
    Ok(observations)
}

fn composed_operator_goal_from_validated(
    producer: &TypedMechanismImprovementOperatorIR,
    consumer: &TypedMechanismImprovementOperatorIR,
    wire_index: usize,
    consumer_operand_influential: bool,
    excluded_operator_ids: Option<&BTreeSet<String>>,
) -> Result<Option<TypedMechanismOperatorCompositionIR>, String> {
    if producer.operator_id == consumer.operator_id
        || producer.condition.is_some()
        || producer.otherwise.is_some()
        || consumer.operand_types.get(wire_index) != Some(&producer.output_type)
        || !consumer_operand_influential
    {
        return Ok(None);
    }
    let mut raw_operand_types = producer.operand_types.clone();
    raw_operand_types.extend(
        consumer
            .operand_types
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != wire_index)
            .map(|(_, value_type)| value_type.clone()),
    );
    if raw_operand_types.len() > MAX_MECHANISM_OPERANDS
        || raw_operand_types.iter().any(|value_type| {
            !matches!(
                value_type,
                ProgramType::Int | ProgramType::Bool | ProgramType::String
            )
        })
    {
        return Ok(None);
    }
    let producer_bindings = (0..producer.operand_types.len())
        .map(|index| {
            (
                format!("ARG_{index}"),
                TypedSyntaxExpressionIR::Operand {
                    role: format!("ARG_{index}"),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let producer_expression =
        substitute_operator_expression(&producer.postimage, &producer_bindings)?;
    let mut consumer_bindings = BTreeMap::new();
    let mut next_argument = producer.operand_types.len();
    for index in 0..consumer.operand_types.len() {
        let expression = if index == wire_index {
            producer_expression.clone()
        } else {
            let expression = TypedSyntaxExpressionIR::Operand {
                role: format!("ARG_{next_argument}"),
            };
            next_argument += 1;
            expression
        };
        consumer_bindings.insert(format!("ARG_{index}"), expression);
    }
    let condition = consumer
        .condition
        .as_ref()
        .map(|expression| substitute_operator_expression(expression, &consumer_bindings))
        .transpose()?;
    let postimage = substitute_operator_expression(&consumer.postimage, &consumer_bindings)?;
    let otherwise = consumer
        .otherwise
        .as_ref()
        .map(|expression| substitute_operator_expression(expression, &consumer_bindings))
        .transpose()?;
    // Composition is a compiler operation, not argument-list concatenation.
    // Remove producer/consumer inputs that cannot influence the flattened
    // expression and remap the surviving roles densely.  Without this step an
    // identity-like operator recursively accumulated dozens of dead operands,
    // consuming the finite search budget without adding capability.
    let canonical =
        canonicalize_composed_operator_inputs(&raw_operand_types, condition, postimage, otherwise)?;
    let CanonicalComposedOperatorInputs {
        operand_types,
        condition,
        postimage,
        otherwise,
        live_argument_indices,
    } = canonical;
    let mut composed_operator = TypedMechanismImprovementOperatorIR {
        schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
        operator_id: String::new(),
        operand_types: operand_types.clone(),
        output_type: consumer.output_type.clone(),
        condition,
        postimage,
        otherwise,
        validation_contract: producer.validation_contract.clone(),
        evidence_sha256: sha256(
            format!(
                "TYPED_OPERATOR_COMPOSITION_PROBE_1:{}:{}:{wire_index}",
                producer.evidence_sha256, consumer.evidence_sha256
            )
            .as_bytes(),
        ),
    };
    let mut identity = composed_operator.clone();
    identity.evidence_sha256.clear();
    identity.operator_id.clear();
    composed_operator.operator_id = sha256(
        &serde_json::to_vec(&identity)
            .map_err(|error| format!("TYPED_OPERATOR_COMPOSITION_SERIALIZE:{error}"))?,
    );
    validate_typed_mechanism_improvement_operator(&composed_operator)?;
    if excluded_operator_ids
        .is_some_and(|excluded| excluded.contains(&composed_operator.operator_id))
    {
        return Ok(None);
    }

    let mut public_observations = Vec::new();
    for case in 0..32 {
        let Some(raw_arguments) = raw_operand_types
            .iter()
            .enumerate()
            .map(|(index, value_type)| typed_operator_probe_value(value_type, case, index))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let producer_arguments = &raw_arguments[..producer.operand_types.len()];
        let producer_output = match evaluate_typed_operator(producer, producer_arguments) {
            Ok(output) => output,
            Err(_) => continue,
        };
        let mut consumer_arguments = Vec::with_capacity(consumer.operand_types.len());
        let mut remaining = raw_arguments[producer.operand_types.len()..].iter();
        for index in 0..consumer.operand_types.len() {
            consumer_arguments.push(if index == wire_index {
                producer_output.clone()
            } else {
                remaining
                    .next()
                    .cloned()
                    .ok_or_else(|| "TYPED_OPERATOR_COMPOSITION_ARGUMENT_BINDING".to_string())?
            });
        }
        let staged = match evaluate_typed_operator(consumer, &consumer_arguments) {
            Ok(output) => output,
            Err(_) => continue,
        };
        let arguments = live_argument_indices
            .iter()
            .map(|index| raw_arguments[*index].clone())
            .collect::<Vec<_>>();
        let flattened = match evaluate_typed_operator(&composed_operator, &arguments) {
            Ok(output) => output,
            Err(_) => continue,
        };
        if staged != flattened {
            return Err("TYPED_OPERATOR_COMPOSITION_POSTIMAGE_MISMATCH".to_string());
        }
        public_observations.push(TypedMechanismObservationIR {
            operands: arguments
                .into_iter()
                .enumerate()
                .map(|(index, value)| (format!("arg_{index}"), value))
                .collect(),
            expected_postimage: flattened,
        });
    }
    public_observations
        .sort_by_key(|observation| serde_json::to_vec(observation).unwrap_or_default());
    public_observations.dedup();
    let distinct_outputs = public_observations
        .iter()
        .map(|observation| {
            serde_json::to_vec(&observation.expected_postimage)
                .map_err(|error| format!("TYPED_OPERATOR_COMPOSITION_OUTPUT_SERIALIZE:{error}"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if public_observations.len() < 8 || distinct_outputs.len() < 2 {
        return Ok(None);
    }
    let goal_identity = sha256(
        format!(
            "TYPED_OPERATOR_COMPOSITION_GOAL_1:{}:{}:{wire_index}",
            producer.operator_id, consumer.operator_id
        )
        .as_bytes(),
    );
    let goal = TypedMechanismSynthesisGoalIR {
        schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
        goal_id: format!("compound_operator_{}", &goal_identity[..32]),
        split: DataSplit::FreshBlind,
        operands: operand_types
            .into_iter()
            .enumerate()
            .map(|(index, value_type)| SourceOperandIR {
                role: format!("arg_{index}"),
                source: format!("input.arg_{index}"),
                value_type,
            })
            .collect(),
        output_type: consumer.output_type.clone(),
        definitions: Vec::new(),
        allowed_effects: vec![Effect::Pure],
        preconditions: vec![
            "both component operators have immutable execution authority".to_string(),
        ],
        postconditions: vec![
            "the producer postimage is transported into the typed consumer operand".to_string(),
            "the flattened postimage equals staged component execution".to_string(),
        ],
        invariants: vec![
            "operator composition is independent of repository and operand names".to_string(),
            "every public observation is replayed by both staged and flattened execution"
                .to_string(),
        ],
        public_observations,
        require_conditional: composed_operator.condition.is_some(),
        max_expression_depth: MAX_SYNTHESIS_DEPTH,
        max_candidates: MAX_SYNTHESIS_CANDIDATES,
        provenance: vec![
            format!("AUTHORIZED_PRODUCER_OPERATOR:{}", producer.operator_id),
            format!("AUTHORIZED_CONSUMER_OPERATOR:{}", consumer.operator_id),
            format!("TYPED_OPERATOR_WIRE_INDEX:{wire_index}"),
            "COMPOSED_BY_TYPE_EFFECT_KERNEL".to_string(),
        ],
    };
    validate_typed_mechanism_synthesis_goal(&goal)?;
    // Pre-falsify the derived public contract through the same bounded
    // synthesizer used by the production generative path. An ambiguous or too
    // deep composition never becomes a curiosity candidate.
    if synthesize_typed_mechanism_goal_with_priors(&goal, std::slice::from_ref(&composed_operator))
        .is_err()
    {
        return Ok(None);
    }
    Ok(Some(TypedMechanismOperatorCompositionIR {
        schema: "B_CORE_TYPED_MECHANISM_OPERATOR_COMPOSITION_1".to_string(),
        goal,
        operator_proposal: composed_operator,
        producer_operator_id: producer.operator_id.clone(),
        consumer_operator_id: consumer.operator_id.clone(),
        wire_index,
    }))
}

/// Builds bounded higher-arity behavior goals by wiring the output of one
/// already-authorized typed operator into a compatible input of another.
/// Results contain observations from staged-vs-flattened execution and still
/// require the ordinary independent campaign verifier before promotion.
pub fn compose_authorized_typed_operator_programs(
    operators: &[TypedMechanismImprovementOperatorIR],
    limit: usize,
) -> Result<Vec<TypedMechanismOperatorCompositionIR>, String> {
    compose_authorized_typed_operator_programs_excluding(operators, &BTreeSet::new(), limit)
}

/// Searches past already-authorized programs instead of letting the first
/// deterministic page permanently starve later graph edges.  Exclusion is
/// applied immediately after proposal identity construction, before the
/// expensive observation/synthesis proof.
pub fn compose_authorized_typed_operator_programs_excluding(
    operators: &[TypedMechanismImprovementOperatorIR],
    excluded_operator_ids: &BTreeSet<String>,
    limit: usize,
) -> Result<Vec<TypedMechanismOperatorCompositionIR>, String> {
    for operator in operators {
        validate_typed_mechanism_improvement_operator(operator)?;
    }
    // Historical authority remains immutable, but recursive composition must
    // not evaluate semantically identical operators once per obsolete arity.
    // Select the smallest authorized representative for each normalized
    // executable program; provenance continues to point at that real receipt.
    let mut semantic_representatives =
        BTreeMap::<String, &TypedMechanismImprovementOperatorIR>::new();
    for operator in operators {
        let semantic_key = typed_operator_composition_semantic_key(operator)?;
        match semantic_representatives.get(&semantic_key) {
            Some(existing)
                if (existing.operand_types.len(), existing.operator_id.as_str())
                    <= (operator.operand_types.len(), operator.operator_id.as_str()) => {}
            _ => {
                semantic_representatives.insert(semantic_key, operator);
            }
        }
    }
    let operators = semantic_representatives.into_values().collect::<Vec<_>>();
    let mut consumer_influence = BTreeMap::new();
    for consumer in &operators {
        for wire_index in 0..consumer.operand_types.len() {
            consumer_influence.insert(
                (consumer.operator_id.as_str(), wire_index),
                typed_operator_operand_has_observed_influence(consumer, wire_index)?,
            );
        }
    }
    let mut goals = BTreeMap::new();
    for producer in &operators {
        for consumer in &operators {
            for wire_index in 0..consumer.operand_types.len() {
                let consumer_operand_influential = consumer_influence
                    .get(&(consumer.operator_id.as_str(), wire_index))
                    .copied()
                    .unwrap_or(false);
                if let Some(composition) = composed_operator_goal_from_validated(
                    producer,
                    consumer,
                    wire_index,
                    consumer_operand_influential,
                    Some(excluded_operator_ids),
                )? {
                    goals
                        .entry(composition.goal.goal_id.clone())
                        .or_insert(composition);
                }
                if goals.len() >= limit.min(32) {
                    return Ok(goals.into_values().collect());
                }
            }
        }
    }
    Ok(goals.into_values().collect())
}

pub fn compose_authorized_typed_operator_goals(
    operators: &[TypedMechanismImprovementOperatorIR],
    limit: usize,
) -> Result<Vec<TypedMechanismSynthesisGoalIR>, String> {
    Ok(
        compose_authorized_typed_operator_programs(operators, limit)?
            .into_iter()
            .map(|composition| composition.goal)
            .collect(),
    )
}

fn build_native_typed_operator_genesis_programs(
) -> Result<Vec<NativeTypedOperatorGenesisIR>, String> {
    struct PrimitiveSpec {
        id: &'static str,
        operand_types: Vec<ProgramType>,
        output_type: ProgramType,
        postimage: TypedSyntaxExpressionIR,
    }

    let arg = |index: usize| TypedSyntaxExpressionIR::Operand {
        role: format!("ARG_{index}"),
    };
    let unary = |id: &'static str,
                 input_type: ProgramType,
                 output_type: ProgramType,
                 operator: UnaryOperator| PrimitiveSpec {
        id,
        operand_types: vec![input_type],
        output_type,
        postimage: TypedSyntaxExpressionIR::Unary {
            operator,
            input: Box::new(arg(0)),
        },
    };
    let string_transform = |id: &'static str, operator: StringTransformOperator| PrimitiveSpec {
        id,
        operand_types: vec![ProgramType::String],
        output_type: ProgramType::String,
        postimage: TypedSyntaxExpressionIR::StringTransform {
            operator,
            input: Box::new(arg(0)),
        },
    };
    let binary = |id: &'static str,
                  input_type: ProgramType,
                  output_type: ProgramType,
                  operator: BinaryOperator| PrimitiveSpec {
        id,
        operand_types: vec![input_type.clone(), input_type],
        output_type,
        postimage: TypedSyntaxExpressionIR::Binary {
            operator,
            left: Box::new(arg(0)),
            right: Box::new(arg(1)),
        },
    };
    let specs = vec![
        unary(
            "INT_NEGATE",
            ProgramType::Int,
            ProgramType::Int,
            UnaryOperator::Negate,
        ),
        unary(
            "BOOL_NOT",
            ProgramType::Bool,
            ProgramType::Bool,
            UnaryOperator::Not,
        ),
        string_transform("STRING_TRIM", StringTransformOperator::Trim),
        string_transform("STRING_LOWERCASE", StringTransformOperator::Lowercase),
        string_transform("STRING_UPPERCASE", StringTransformOperator::Uppercase),
        binary(
            "INT_ADD",
            ProgramType::Int,
            ProgramType::Int,
            BinaryOperator::Add,
        ),
        binary(
            "INT_SUBTRACT",
            ProgramType::Int,
            ProgramType::Int,
            BinaryOperator::Subtract,
        ),
        binary(
            "INT_MULTIPLY",
            ProgramType::Int,
            ProgramType::Int,
            BinaryOperator::Multiply,
        ),
        binary(
            "INT_EQUAL",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::Equal,
        ),
        binary(
            "INT_NOT_EQUAL",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::NotEqual,
        ),
        binary(
            "INT_LESS_THAN",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::LessThan,
        ),
        binary(
            "INT_LESS_THAN_OR_EQUAL",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::LessThanOrEqual,
        ),
        binary(
            "INT_GREATER_THAN",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::GreaterThan,
        ),
        binary(
            "INT_GREATER_THAN_OR_EQUAL",
            ProgramType::Int,
            ProgramType::Bool,
            BinaryOperator::GreaterThanOrEqual,
        ),
        binary(
            "BOOL_AND",
            ProgramType::Bool,
            ProgramType::Bool,
            BinaryOperator::And,
        ),
        binary(
            "BOOL_OR",
            ProgramType::Bool,
            ProgramType::Bool,
            BinaryOperator::Or,
        ),
        binary(
            "BOOL_EQUAL",
            ProgramType::Bool,
            ProgramType::Bool,
            BinaryOperator::Equal,
        ),
        binary(
            "BOOL_NOT_EQUAL",
            ProgramType::Bool,
            ProgramType::Bool,
            BinaryOperator::NotEqual,
        ),
        binary(
            "STRING_CONCATENATE",
            ProgramType::String,
            ProgramType::String,
            BinaryOperator::Add,
        ),
        binary(
            "STRING_EQUAL",
            ProgramType::String,
            ProgramType::Bool,
            BinaryOperator::Equal,
        ),
        binary(
            "STRING_NOT_EQUAL",
            ProgramType::String,
            ProgramType::Bool,
            BinaryOperator::NotEqual,
        ),
        binary(
            "STRING_LESS_THAN",
            ProgramType::String,
            ProgramType::Bool,
            BinaryOperator::LessThan,
        ),
        binary(
            "STRING_GREATER_THAN",
            ProgramType::String,
            ProgramType::Bool,
            BinaryOperator::GreaterThan,
        ),
    ];

    let validation_contract = vec![
        "PUBLIC_OBSERVATION_REPLAY".to_string(),
        "TYPE_EFFECT_CHECK".to_string(),
        "SOURCE_BOUND_ATOMIC_MATERIALIZATION".to_string(),
        "SANDBOX_PUBLIC_REGRESSION".to_string(),
        "AUTHORITATIVE_SCOPE_STABLE".to_string(),
    ];
    let mut programs = Vec::new();
    for spec in specs {
        let mut operator = TypedMechanismImprovementOperatorIR {
            schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
            operator_id: String::new(),
            operand_types: spec.operand_types,
            output_type: spec.output_type,
            condition: None,
            postimage: spec.postimage,
            otherwise: None,
            validation_contract: validation_contract.clone(),
            evidence_sha256: sha256(
                format!("B_CORE_NATIVE_TYPED_GRAMMAR_1:{}", spec.id).as_bytes(),
            ),
        };
        let mut identity = operator.clone();
        identity.operator_id.clear();
        identity.evidence_sha256.clear();
        operator.operator_id = sha256(
            &serde_json::to_vec(&identity)
                .map_err(|error| format!("NATIVE_TYPED_OPERATOR_SERIALIZE:{error}"))?,
        );
        validate_typed_mechanism_improvement_operator(&operator)?;
        let mut all_operands_influential = true;
        for index in 0..operator.operand_types.len() {
            if !typed_operator_operand_has_observed_influence(&operator, index)? {
                all_operands_influential = false;
                break;
            }
        }
        if !all_operands_influential {
            continue;
        }
        let mut public_observations =
            typed_mechanism_operator_probe_observations(&operator, 0, 32)?
                .into_iter()
                .map(|observation| TypedMechanismObservationIR {
                    operands: observation
                        .operands
                        .into_iter()
                        .map(|(role, value)| (role.to_ascii_lowercase(), value))
                        .collect(),
                    expected_postimage: observation.expected_postimage,
                })
                .collect::<Vec<_>>();
        public_observations
            .sort_by_key(|observation| serde_json::to_vec(observation).unwrap_or_default());
        public_observations.dedup();
        let distinct_outputs = public_observations
            .iter()
            .map(|observation| json_sha256(&observation.expected_postimage))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if distinct_outputs.len() < 2 {
            continue;
        }
        let goal = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: format!("native_operator_{}", &operator.operator_id[..32]),
            split: DataSplit::FreshBlind,
            operands: operator
                .operand_types
                .iter()
                .enumerate()
                .map(|(index, value_type)| SourceOperandIR {
                    role: format!("arg_{index}"),
                    source: format!("input.arg_{index}"),
                    value_type: value_type.clone(),
                })
                .collect(),
            output_type: operator.output_type.clone(),
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: vec!["typed operands satisfy the native grammar signature".to_string()],
            postconditions: vec![
                "the materialized source implements the proposed typed postimage".to_string(),
            ],
            invariants: vec![
                "genesis uses only B_Core-owned typed execution semantics".to_string(),
                "proposal authority remains absent until hidden-source verification".to_string(),
            ],
            public_observations,
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: MAX_SYNTHESIS_CANDIDATES,
            provenance: vec![
                format!("B_CORE_NATIVE_TYPED_PRIMITIVE:{}", spec.id),
                "B_CORE_NATIVE_TYPED_GRAMMAR_GENESIS".to_string(),
                "EXTERNAL_KNOWLEDGE_RUNTIME_DEPENDENCY:false".to_string(),
            ],
        };
        validate_typed_mechanism_synthesis_goal(&goal)?;
        let receipt =
            synthesize_typed_mechanism_goal_with_priors(&goal, std::slice::from_ref(&operator))?;
        if receipt.selected_operator_id.as_deref() != Some(operator.operator_id.as_str()) {
            continue;
        }
        programs.push(NativeTypedOperatorGenesisIR {
            schema: NATIVE_TYPED_OPERATOR_GENESIS_SCHEMA.to_string(),
            primitive_id: spec.id.to_string(),
            goal,
            operator_proposal: operator,
        });
    }
    Ok(programs)
}

/// Produces a bounded set of name-independent grammar programs from B_Core's
/// own typed execution semantics.  These are product bootstrap hypotheses,
/// not copied answers and not trusted knowledge: every item is replayed over
/// public observations, held-out by the source curriculum, and promoted only
/// through the normal independent authority boundary.  The immutable native
/// grammar is built once per process; state-specific authority filtering stays
/// cheap on every supervisor scan.
pub fn native_typed_operator_genesis_programs(
    excluded_operator_ids: &BTreeSet<String>,
    limit: usize,
) -> Result<Vec<NativeTypedOperatorGenesisIR>, String> {
    static NATIVE_PROGRAMS: OnceLock<Result<Vec<NativeTypedOperatorGenesisIR>, String>> =
        OnceLock::new();
    Ok(NATIVE_PROGRAMS
        .get_or_init(build_native_typed_operator_genesis_programs)
        .clone()?
        .into_iter()
        .filter(|program| !excluded_operator_ids.contains(&program.operator_proposal.operator_id))
        .take(limit.min(32))
        .collect())
}

pub fn typed_mechanism_improvement_operator_from_receipt(
    receipt: &TypedMechanismSynthesisReceiptIR,
    evidence_sha256: String,
) -> Result<TypedMechanismImprovementOperatorIR, String> {
    validate_typed_mechanism_synthesis_receipt(receipt)?;
    if evidence_sha256.len() != 64 || receipt.winning_goal.operands.is_empty() {
        return Err("TYPED_MECHANISM_PRIOR_EVIDENCE".to_string());
    }
    let role_map = receipt
        .winning_goal
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), format!("ARG_{index}")))
        .collect::<BTreeMap<_, _>>();
    if role_map.len() != receipt.winning_goal.operands.len() {
        return Err("TYPED_MECHANISM_PRIOR_DUPLICATE_ROLE".to_string());
    }
    let mut prior = TypedMechanismImprovementOperatorIR {
        schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
        operator_id: String::new(),
        operand_types: receipt
            .winning_goal
            .operands
            .iter()
            .map(|operand| operand.value_type.clone())
            .collect(),
        output_type: receipt.winning_goal.output_type.clone(),
        condition: receipt
            .winning_goal
            .condition
            .as_ref()
            .map(|condition| remap_expression_roles(condition, &role_map))
            .transpose()?,
        postimage: remap_expression_roles(&receipt.winning_goal.postimage, &role_map)?,
        otherwise: receipt
            .winning_goal
            .otherwise
            .as_ref()
            .map(|otherwise| remap_expression_roles(otherwise, &role_map))
            .transpose()?,
        validation_contract: vec![
            "PUBLIC_OBSERVATION_REPLAY".to_string(),
            "TYPE_EFFECT_CHECK".to_string(),
            "SOURCE_BOUND_ATOMIC_MATERIALIZATION".to_string(),
            "SANDBOX_PUBLIC_REGRESSION".to_string(),
            "AUTHORITATIVE_SCOPE_STABLE".to_string(),
        ],
        evidence_sha256,
    };
    let mut identity = prior.clone();
    identity.evidence_sha256.clear();
    let encoded = serde_json::to_vec(&identity)
        .map_err(|error| format!("TYPED_MECHANISM_PRIOR_SERIALIZE:{error}"))?;
    prior.operator_id = sha256(&encoded);
    validate_typed_mechanism_improvement_operator(&prior)?;
    Ok(prior)
}

fn transport_prior_to_request(
    prior: &TypedMechanismImprovementOperatorIR,
    request: &TypedMechanismSynthesisGoalIR,
) -> Result<
    (
        Option<TypedSyntaxExpressionIR>,
        TypedSyntaxExpressionIR,
        Option<TypedSyntaxExpressionIR>,
    ),
    String,
> {
    validate_typed_mechanism_improvement_operator(prior)?;
    let request_types = request
        .operands
        .iter()
        .map(|operand| operand.value_type.clone())
        .collect::<Vec<_>>();
    if prior.operand_types != request_types || prior.output_type != request.output_type {
        return Err("TYPED_MECHANISM_PRIOR_TYPE_SHAPE_MISMATCH".to_string());
    }
    let role_map = request
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (format!("ARG_{index}"), operand.role.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok((
        prior
            .condition
            .as_ref()
            .map(|condition| remap_expression_roles(condition, &role_map))
            .transpose()?,
        remap_expression_roles(&prior.postimage, &role_map)?,
        prior
            .otherwise
            .as_ref()
            .map(|otherwise| remap_expression_roles(otherwise, &role_map))
            .transpose()?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn evaluate_prior_expression(
    expression: &TypedSyntaxExpressionIR,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
    allowed_effects: &[Effect],
) -> Result<(ProgramType, Vec<Value>), String> {
    let mut effects = BTreeSet::new();
    let value_type = infer_expression_type(expression, operand_types, definitions, &mut effects)?;
    if !effects
        .iter()
        .all(|effect| allowed_effects.contains(effect))
    {
        return Err("TYPED_MECHANISM_PRIOR_EFFECT_FORBIDDEN".to_string());
    }
    let scalar = lower_expression(expression, operand_indices)?;
    let outputs = observation_arguments
        .iter()
        .map(|arguments| eval_scalar(&scalar, arguments, api_map))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((value_type, outputs))
}

#[allow(clippy::too_many_arguments)]
fn prior_matches_public_observations(
    request: &TypedMechanismSynthesisGoalIR,
    condition: &Option<TypedSyntaxExpressionIR>,
    postimage: &TypedSyntaxExpressionIR,
    otherwise: &Option<TypedSyntaxExpressionIR>,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
) -> Result<bool, String> {
    let (postimage_type, postimage_outputs) = evaluate_prior_expression(
        postimage,
        operand_types,
        operand_indices,
        definitions,
        api_map,
        observation_arguments,
        &request.allowed_effects,
    )?;
    if postimage_type != request.output_type {
        return Ok(false);
    }
    let expected = request
        .public_observations
        .iter()
        .map(|observation| observation.expected_postimage.clone())
        .collect::<Vec<_>>();
    match (condition, otherwise) {
        (None, None) => Ok(!request.require_conditional && postimage_outputs == expected),
        (Some(condition), Some(otherwise)) => {
            let (condition_type, condition_outputs) = evaluate_prior_expression(
                condition,
                operand_types,
                operand_indices,
                definitions,
                api_map,
                observation_arguments,
                &request.allowed_effects,
            )?;
            let (otherwise_type, otherwise_outputs) = evaluate_prior_expression(
                otherwise,
                operand_types,
                operand_indices,
                definitions,
                api_map,
                observation_arguments,
                &request.allowed_effects,
            )?;
            if condition_type != ProgramType::Bool || otherwise_type != request.output_type {
                return Ok(false);
            }
            let outputs = condition_outputs
                .iter()
                .zip(postimage_outputs.iter().zip(&otherwise_outputs))
                .map(|(condition, (then_value, else_value))| match condition {
                    Value::Bool(true) => Ok(then_value.clone()),
                    Value::Bool(false) => Ok(else_value.clone()),
                    _ => Err("TYPED_MECHANISM_PRIOR_CONDITION_NOT_BOOL".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(outputs == expected)
        }
        _ => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_synthesis_receipt(
    request: &TypedMechanismSynthesisGoalIR,
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
    enumerated: usize,
    candidates_falsified: usize,
    attempted_operator_ids: Vec<String>,
    rejected_operator_ids: Vec<String>,
    selected_operator_id: Option<String>,
    parallel_operator_evaluation: bool,
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    let preferred_operator_attempts = attempted_operator_ids.len();
    let conditional_synthesized = condition.is_some();
    let winning_goal = TypedMechanismGoalIR {
        schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
        goal_id: request.goal_id.clone(),
        split: request.split,
        operands: request.operands.clone(),
        output_type: request.output_type.clone(),
        condition,
        postimage,
        otherwise,
        definitions: request.definitions.clone(),
        allowed_effects: request.allowed_effects.clone(),
        preconditions: request.preconditions.clone(),
        postconditions: request.postconditions.clone(),
        invariants: request.invariants.clone(),
        public_observations: request.public_observations.clone(),
        provenance: request
            .provenance
            .iter()
            .cloned()
            .chain([if selected_operator_id.is_some() {
                "CONTENT_ADDRESSED_OPERATOR_REUSE".to_string()
            } else {
                "BOUNDED_TYPED_GRAMMAR_SYNTHESIS".to_string()
            }])
            .collect(),
    };
    let template = lower_typed_mechanism_goal(&winning_goal)?;
    let receipt_sha256 = typed_mechanism_synthesis_receipt_hash(
        request,
        &winning_goal,
        &template,
        enumerated,
        candidates_falsified,
        preferred_operator_attempts,
        &selected_operator_id,
        &attempted_operator_ids,
        &rejected_operator_ids,
        parallel_operator_evaluation,
    )?;
    Ok(TypedMechanismSynthesisReceiptIR {
        schema: "B_CORE_TYPED_MECHANISM_SYNTHESIS_RECEIPT_1".to_string(),
        goal_id: request.goal_id.clone(),
        synthesis_request: Some(request.clone()),
        candidates_enumerated: enumerated,
        candidates_falsified,
        counterexample_guided_selection: candidates_falsified > 0,
        conditional_synthesized,
        winning_expression_nodes: template.expression_nodes,
        preferred_operator_attempts,
        preferred_operator_selected: selected_operator_id.is_some(),
        selected_operator_id,
        attempted_operator_ids,
        rejected_operator_ids,
        parallel_operator_evaluation,
        winning_goal,
        template,
        receipt_sha256,
    })
}

#[allow(clippy::too_many_arguments)]
fn typed_mechanism_synthesis_receipt_hash(
    request: &TypedMechanismSynthesisGoalIR,
    winning_goal: &TypedMechanismGoalIR,
    template: &ConcreteSyntaxTemplateIR,
    enumerated: usize,
    candidates_falsified: usize,
    preferred_operator_attempts: usize,
    selected_operator_id: &Option<String>,
    attempted_operator_ids: &[String],
    rejected_operator_ids: &[String],
    parallel_operator_evaluation: bool,
) -> Result<String, String> {
    Ok(sha256(
        serde_json::to_vec(&(
            TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
            request,
            winning_goal,
            &template.syntax_sha256,
            enumerated,
            candidates_falsified,
            preferred_operator_attempts,
            selected_operator_id,
            attempted_operator_ids,
            rejected_operator_ids,
            parallel_operator_evaluation,
        ))
        .map_err(|error| format!("TYPED_MECHANISM_RECEIPT_SERIALIZE:{error}"))?
        .as_slice(),
    ))
}

pub fn validate_typed_mechanism_synthesis_receipt(
    receipt: &TypedMechanismSynthesisReceiptIR,
) -> Result<(), String> {
    let request = receipt
        .synthesis_request
        .as_ref()
        .ok_or_else(|| "TYPED_MECHANISM_RECEIPT_REQUEST_MISSING".to_string())?;
    validate_typed_mechanism_synthesis_goal(request)?;
    let goal = &receipt.winning_goal;
    let selected = receipt.selected_operator_id.is_some();
    let expected_provenance = request
        .provenance
        .iter()
        .cloned()
        .chain([if selected {
            "CONTENT_ADDRESSED_OPERATOR_REUSE".to_string()
        } else {
            "BOUNDED_TYPED_GRAMMAR_SYNTHESIS".to_string()
        }])
        .collect::<Vec<_>>();
    if receipt.schema != "B_CORE_TYPED_MECHANISM_SYNTHESIS_RECEIPT_1"
        || receipt.goal_id != request.goal_id
        || goal.schema != TYPED_MECHANISM_GOAL_SCHEMA
        || goal.goal_id != request.goal_id
        || goal.split != request.split
        || goal.operands != request.operands
        || goal.output_type != request.output_type
        || goal.definitions != request.definitions
        || goal.allowed_effects != request.allowed_effects
        || goal.preconditions != request.preconditions
        || goal.postconditions != request.postconditions
        || goal.invariants != request.invariants
        || goal.public_observations != request.public_observations
        || goal.provenance != expected_provenance
    {
        return Err("TYPED_MECHANISM_RECEIPT_GOAL_BINDING".to_string());
    }
    let rebuilt_template = lower_typed_mechanism_goal(goal)?;
    if rebuilt_template != receipt.template
        || receipt.winning_expression_nodes != rebuilt_template.expression_nodes
        || receipt.conditional_synthesized != goal.condition.is_some()
        || goal.condition.is_some() != goal.otherwise.is_some()
    {
        return Err("TYPED_MECHANISM_RECEIPT_TEMPLATE_BINDING".to_string());
    }
    let expected_workers = worker_count_for(
        receipt.attempted_operator_ids.len(),
        TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER,
    );
    if receipt.candidates_enumerated == 0
        || receipt.counterexample_guided_selection != (receipt.candidates_falsified > 0)
        || receipt.preferred_operator_attempts != receipt.attempted_operator_ids.len()
        || receipt.preferred_operator_selected != selected
        || receipt.parallel_operator_evaluation != (expected_workers > 1)
        || receipt
            .selected_operator_id
            .as_ref()
            .is_some_and(|operator| !receipt.attempted_operator_ids.contains(operator))
        || receipt.rejected_operator_ids.iter().any(|operator| {
            !receipt.attempted_operator_ids.contains(operator)
                || receipt.selected_operator_id.as_ref() == Some(operator)
        })
    {
        return Err("TYPED_MECHANISM_RECEIPT_ACCOUNTING".to_string());
    }
    let expected_hash = typed_mechanism_synthesis_receipt_hash(
        request,
        goal,
        &rebuilt_template,
        receipt.candidates_enumerated,
        receipt.candidates_falsified,
        receipt.preferred_operator_attempts,
        &receipt.selected_operator_id,
        &receipt.attempted_operator_ids,
        &receipt.rejected_operator_ids,
        receipt.parallel_operator_evaluation,
    )?;
    if receipt.receipt_sha256 != expected_hash {
        return Err("TYPED_MECHANISM_RECEIPT_HASH".to_string());
    }
    Ok(())
}

/// Enumerate a bounded, typed expression grammar and use public
/// counterexamples to select either one postimage or a guarded pair of
/// postimages.  Search order is deterministic and independent of goal names.
pub fn synthesize_typed_mechanism_goal(
    request: &TypedMechanismSynthesisGoalIR,
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    synthesize_typed_mechanism_goal_with_priors(request, &[])
}

pub fn synthesize_typed_mechanism_goal_with_priors(
    request: &TypedMechanismSynthesisGoalIR,
    priors: &[TypedMechanismImprovementOperatorIR],
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    synthesize_typed_mechanism_goal_with_source_seeds_and_priors(request, &[], priors)
}

/// Adds bounded expressions extracted from the exact repository source to the
/// common typed grammar. Source seeds are hypotheses only: they are type/effect
/// checked, replayed against every public observation, and compete under the
/// same minimality/identifiability rules as generated expressions.
pub fn synthesize_typed_mechanism_goal_with_source_seeds_and_priors(
    request: &TypedMechanismSynthesisGoalIR,
    source_seeds: &[TypedSyntaxExpressionIR],
    priors: &[TypedMechanismImprovementOperatorIR],
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    validate_typed_mechanism_synthesis_goal(request)?;
    if source_seeds.len() > MAX_SOURCE_SEED_EXPRESSIONS
        || source_seeds
            .iter()
            .any(|expression| expression_nodes(expression) > MAX_MECHANISM_EXPRESSION_NODES)
    {
        return Err("TYPED_MECHANISM_SOURCE_SEED_BUDGET".to_string());
    }
    let max_depth = request.max_expression_depth.clamp(1, MAX_SYNTHESIS_DEPTH);
    let max_candidates = request.max_candidates.clamp(16, MAX_SYNTHESIS_CANDIDATES);
    let operand_types = request
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_indices = request
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let definitions = request
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let api_map = request
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let observation_arguments = request
        .public_observations
        .iter()
        .map(|observation| {
            request
                .operands
                .iter()
                .map(|operand| {
                    observation
                        .operands
                        .get(&operand.role)
                        .cloned()
                        .ok_or_else(|| {
                            format!("TYPED_MECHANISM_OBSERVATION_MISSING:{}", operand.role)
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut applicable_operators = Vec::new();
    let mut seen_operator_ids = BTreeSet::new();
    for prior in priors
        .iter()
        .filter(|prior| seen_operator_ids.insert(prior.operator_id.clone()))
    {
        validate_typed_mechanism_improvement_operator(prior)?;
        let transported = match transport_prior_to_request(prior, request) {
            Ok(transported) => transported,
            Err(error) if error == "TYPED_MECHANISM_PRIOR_TYPE_SHAPE_MISMATCH" => continue,
            Err(error) => return Err(error),
        };
        applicable_operators.push(TransportedImprovementOperator {
            operator_id: prior.operator_id.clone(),
            condition: transported.0,
            postimage: transported.1,
            otherwise: transported.2,
        });
    }
    let operator_worker_count = worker_count_for(
        applicable_operators.len(),
        TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER,
    );
    let operator_matches = parallel_map_ordered_batched(
        &applicable_operators,
        "TYPED_OPERATOR_PUBLIC_REPLAY",
        TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER,
        |operator| {
            Ok(prior_matches_public_observations(
                request,
                &operator.condition,
                &operator.postimage,
                &operator.otherwise,
                &operand_types,
                &operand_indices,
                &definitions,
                &api_map,
                &observation_arguments,
            )
            .unwrap_or(false))
        },
    )?;
    let attempted_operator_ids = applicable_operators
        .iter()
        .map(|operator| operator.operator_id.clone())
        .collect::<Vec<_>>();
    if let Some(winner_index) = operator_matches.iter().position(|matches| *matches) {
        let winner = &applicable_operators[winner_index];
        let rejected_operator_ids = applicable_operators
            .iter()
            .zip(&operator_matches)
            .filter(|(_, matches)| !**matches)
            .map(|(operator, _)| operator.operator_id.clone())
            .collect::<Vec<_>>();
        let enumerated = applicable_operators
            .iter()
            .map(|operator| {
                1 + usize::from(operator.condition.is_some())
                    + usize::from(operator.otherwise.is_some())
            })
            .sum();
        return build_synthesis_receipt(
            request,
            winner.condition.clone(),
            winner.postimage.clone(),
            winner.otherwise.clone(),
            enumerated,
            rejected_operator_ids.len(),
            attempted_operator_ids,
            rejected_operator_ids,
            Some(winner.operator_id.clone()),
            operator_worker_count > 1,
        );
    }
    let rejected_operator_ids = attempted_operator_ids.clone();

    let mut expressions = Vec::<EnumeratedExpression>::new();
    let mut seen = BTreeSet::new();
    let mut enumerated = 0_usize;
    let mut evaluation_failures = 0_usize;
    for operand in &request.operands {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::Operand {
                role: operand.role.clone(),
            },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }
    // Keep the literal basis universal. Mining expected outputs as constants
    // can fit a finite observation table while ignoring the source operands,
    // which is precisely the template-selection failure this compiler is
    // intended to remove.
    let int_constants = BTreeSet::from([-1_i64, 0_i64, 1_i64]);
    let bool_constants = BTreeSet::from([false, true]);
    for value in int_constants {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::IntLiteral { value },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }
    for value in bool_constants {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::BoolLiteral { value },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }
    add_enumerated_expression(
        TypedSyntaxExpressionIR::StringLiteral {
            value: String::new(),
        },
        &operand_types,
        &operand_indices,
        &definitions,
        &api_map,
        &observation_arguments,
        &request.allowed_effects,
        max_candidates,
        &mut enumerated,
        &mut evaluation_failures,
        &mut seen,
        &mut expressions,
    )?;

    // Preserve the universal operands/literals above even under a small
    // candidate budget. Source-derived syntax then contributes reusable
    // subexpressions without receiving answer authority or starving basics.
    for expression in source_seeds {
        add_enumerated_expression(
            expression.clone(),
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }

    for _depth in 1..=max_depth {
        if expressions.len() >= max_candidates {
            break;
        }
        let prior = expressions.clone();
        for candidate in &prior {
            for operator in [UnaryOperator::Negate, UnaryOperator::Not] {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Unary {
                        operator,
                        input: Box::new(candidate.expression.clone()),
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
        for candidate in prior
            .iter()
            .filter(|candidate| candidate.value_type == ProgramType::String)
        {
            for operator in [
                StringTransformOperator::Trim,
                StringTransformOperator::Lowercase,
                StringTransformOperator::Uppercase,
            ] {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::StringTransform {
                        operator,
                        input: Box::new(candidate.expression.clone()),
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
        for collection in prior.iter().filter(|candidate| {
            matches!(
                candidate.value_type,
                ProgramType::String
                    | ProgramType::SequenceInt
                    | ProgramType::NestedSequenceInt
                    | ProgramType::Bytes
            )
        }) {
            if expressions.len() >= max_candidates {
                break;
            }
            add_enumerated_expression(
                TypedSyntaxExpressionIR::Length {
                    input: Box::new(collection.expression.clone()),
                },
                &operand_types,
                &operand_indices,
                &definitions,
                &api_map,
                &observation_arguments,
                &request.allowed_effects,
                max_candidates,
                &mut enumerated,
                &mut evaluation_failures,
                &mut seen,
                &mut expressions,
            )?;
            for index in prior
                .iter()
                .filter(|candidate| candidate.value_type == ProgramType::Int)
            {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Index {
                        collection: Box::new(collection.expression.clone()),
                        index: Box::new(index.expression.clone()),
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
        let operators = [
            BinaryOperator::Equal,
            BinaryOperator::NotEqual,
            BinaryOperator::LessThan,
            BinaryOperator::LessThanOrEqual,
            BinaryOperator::GreaterThan,
            BinaryOperator::GreaterThanOrEqual,
            BinaryOperator::Add,
            BinaryOperator::Subtract,
            BinaryOperator::Multiply,
            BinaryOperator::Divide,
            BinaryOperator::Modulo,
            BinaryOperator::And,
            BinaryOperator::Or,
        ];
        'binary: for operator in operators {
            for left in &prior {
                for right in &prior {
                    if expressions.len() >= max_candidates {
                        break 'binary;
                    }
                    add_enumerated_expression(
                        TypedSyntaxExpressionIR::Binary {
                            operator,
                            left: Box::new(left.expression.clone()),
                            right: Box::new(right.expression.clone()),
                        },
                        &operand_types,
                        &operand_indices,
                        &definitions,
                        &api_map,
                        &observation_arguments,
                        &request.allowed_effects,
                        max_candidates,
                        &mut enumerated,
                        &mut evaluation_failures,
                        &mut seen,
                        &mut expressions,
                    )?;
                }
            }
        }
        for definition in &request.definitions {
            if expressions.len() >= max_candidates {
                break;
            }
            let argument_sets = enumerate_api_arguments(definition, &prior, 64);
            for arguments in argument_sets {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Call {
                        api_token: definition.api_token.clone(),
                        arguments,
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
    }

    let expected = request
        .public_observations
        .iter()
        .map(|observation| observation.expected_postimage.clone())
        .collect::<Vec<_>>();
    let mut output_candidates = expressions
        .iter()
        .filter(|candidate| candidate.value_type == request.output_type)
        .cloned()
        .collect::<Vec<_>>();
    output_candidates.sort_by(|left, right| {
        (left.nodes, &left.canonical_key).cmp(&(right.nodes, &right.canonical_key))
    });
    if !request.require_conditional {
        ensure_minimal_exact_hypotheses_identifiable(
            request,
            &output_candidates,
            &expected,
            &observation_arguments,
            &operand_indices,
            &api_map,
        )?;
    }
    let exact = output_candidates
        .iter()
        .find(|candidate| candidate.outputs == expected)
        .cloned();
    let (condition, postimage, otherwise) = if !request.require_conditional {
        if let Some(exact) = exact {
            (None, exact.expression, None)
        } else {
            synthesize_conditional(
                request,
                &expressions,
                &output_candidates,
                &expected,
                &observation_arguments,
                &operand_indices,
                &api_map,
            )?
        }
    } else {
        synthesize_conditional(
            request,
            &expressions,
            &output_candidates,
            &expected,
            &observation_arguments,
            &operand_indices,
            &api_map,
        )?
    };
    let candidates_falsified = output_candidates
        .iter()
        .filter(|candidate| candidate.outputs != expected)
        .count()
        .saturating_add(evaluation_failures)
        .saturating_add(rejected_operator_ids.len());
    build_synthesis_receipt(
        request,
        condition,
        postimage,
        otherwise,
        enumerated,
        candidates_falsified,
        attempted_operator_ids,
        rejected_operator_ids,
        None,
        operator_worker_count > 1,
    )
}

fn ensure_minimal_exact_hypotheses_identifiable(
    request: &TypedMechanismSynthesisGoalIR,
    output_candidates: &[EnumeratedExpression],
    expected: &[Value],
    observation_arguments: &[Vec<Value>],
    operand_indices: &BTreeMap<String, usize>,
    api_map: &BTreeMap<String, &ApiDefinition>,
) -> Result<(), String> {
    let Some(minimum_nodes) = output_candidates
        .iter()
        .filter(|candidate| candidate.outputs == expected)
        .map(|candidate| candidate.nodes)
        .min()
    else {
        return Ok(());
    };
    let minimal = output_candidates
        .iter()
        .filter(|candidate| candidate.nodes == minimum_nodes && candidate.outputs == expected)
        .collect::<Vec<_>>();
    if minimal.len() <= 1 {
        return Ok(());
    }
    let probes = bounded_identifiability_arguments(request, observation_arguments);
    let mut semantic_classes = BTreeSet::new();
    for candidate in &minimal {
        let scalar = lower_expression(&candidate.expression, operand_indices)?;
        let signature = probes
            .iter()
            .map(|arguments| match eval_scalar(&scalar, arguments, api_map) {
                Ok(value) => serde_json::to_string(&("OK", value))
                    .map_err(|error| format!("TYPED_MECHANISM_PROBE_SERIALIZE:{error}")),
                Err(error) => Ok(format!("[\"ERROR\",{error:?}]")),
            })
            .collect::<Result<Vec<_>, String>>()?;
        semantic_classes.insert(
            serde_json::to_string(&signature)
                .map_err(|error| format!("TYPED_MECHANISM_PROBE_SIGNATURE:{error}"))?,
        );
    }
    if semantic_classes.len() > 1 {
        return Err(format!(
            "TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_HYPOTHESES:{}:SEMANTIC_CLASSES:{}:PROBES:{}",
            minimal.len(),
            semantic_classes.len(),
            probes.len()
        ));
    }
    Ok(())
}

fn bounded_identifiability_arguments(
    request: &TypedMechanismSynthesisGoalIR,
    observation_arguments: &[Vec<Value>],
) -> Vec<Vec<Value>> {
    fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T, limit: usize) {
        if values.len() < limit && !values.contains(&value) {
            values.push(value);
        }
    }

    fn domain(value_type: &ProgramType, observed: impl Iterator<Item = Value>) -> Vec<Value> {
        let mut values = Vec::new();
        for value in observed {
            push_unique(&mut values, value, 12);
        }
        match value_type {
            ProgramType::Int => {
                let observed_ints = values
                    .iter()
                    .filter_map(|value| match value {
                        Value::Int(value) => Some(*value),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                for value in [-2_i64, -1, 0, 1, 2] {
                    push_unique(&mut values, Value::Int(value), 12);
                }
                for value in observed_ints {
                    push_unique(&mut values, Value::Int(value.saturating_sub(1)), 12);
                    push_unique(&mut values, Value::Int(value.saturating_add(1)), 12);
                }
            }
            ProgramType::Bool => {
                push_unique(&mut values, Value::Bool(false), 12);
                push_unique(&mut values, Value::Bool(true), 12);
            }
            ProgramType::String => {
                for value in ["", "a", "b", "A", "  a  "] {
                    push_unique(&mut values, Value::String(value.to_string()), 12);
                }
            }
            ProgramType::SequenceInt => {
                for value in [vec![], vec![0], vec![1], vec![-1], vec![0, 1]] {
                    push_unique(&mut values, Value::Sequence(value), 12);
                }
            }
            ProgramType::NestedSequenceInt => {
                for value in [vec![], vec![vec![]], vec![vec![0]], vec![vec![0], vec![1]]] {
                    push_unique(&mut values, Value::NestedSequence(value), 12);
                }
            }
            ProgramType::Bytes => {
                for value in [vec![], vec![0], vec![1], vec![0, 1]] {
                    push_unique(&mut values, Value::Bytes(value), 12);
                }
            }
            ProgramType::Image | ProgramType::Unit => {}
        }
        values
    }

    let mut probes = Vec::new();
    for arguments in observation_arguments {
        push_unique(&mut probes, arguments.clone(), MAX_IDENTIFIABILITY_PROBES);
    }
    let base = observation_arguments[0].clone();
    let domains = request
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| {
            domain(
                &operand.value_type,
                observation_arguments
                    .iter()
                    .filter_map(move |arguments| arguments.get(index).cloned()),
            )
        })
        .collect::<Vec<_>>();
    for (index, values) in domains.iter().enumerate() {
        for value in values {
            let mut arguments = base.clone();
            arguments[index] = value.clone();
            push_unique(&mut probes, arguments, MAX_IDENTIFIABILITY_PROBES);
        }
    }
    let diagonal_steps = domains.iter().map(Vec::len).max().unwrap_or(0);
    for step in 0..diagonal_steps {
        let mut arguments = base.clone();
        for (index, values) in domains.iter().enumerate() {
            if !values.is_empty() {
                arguments[index] = values[(step + index) % values.len()].clone();
            }
        }
        push_unique(&mut probes, arguments, MAX_IDENTIFIABILITY_PROBES);
    }
    'pairs: for left in 0..domains.len() {
        for right in (left + 1)..domains.len() {
            for left_value in domains[left].iter().take(4) {
                for right_value in domains[right].iter().take(4) {
                    let mut arguments = base.clone();
                    arguments[left] = left_value.clone();
                    arguments[right] = right_value.clone();
                    push_unique(&mut probes, arguments, MAX_IDENTIFIABILITY_PROBES);
                    if probes.len() >= MAX_IDENTIFIABILITY_PROBES {
                        break 'pairs;
                    }
                }
            }
        }
    }
    probes
}

#[allow(clippy::too_many_arguments)]
fn add_enumerated_expression(
    expression: TypedSyntaxExpressionIR,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
    allowed_effects: &[Effect],
    max_candidates: usize,
    enumerated: &mut usize,
    evaluation_failures: &mut usize,
    seen: &mut BTreeSet<String>,
    expressions: &mut Vec<EnumeratedExpression>,
) -> Result<(), String> {
    if expressions.len() >= max_candidates {
        return Ok(());
    }
    let canonical_key = serde_json::to_string(&expression)
        .map_err(|error| format!("TYPED_MECHANISM_EXPRESSION_SERIALIZE:{error}"))?;
    if !seen.insert(canonical_key.clone()) {
        return Ok(());
    }
    *enumerated = enumerated.saturating_add(1);
    let mut effects = BTreeSet::new();
    let Ok(value_type) =
        infer_expression_type(&expression, operand_types, definitions, &mut effects)
    else {
        *evaluation_failures = evaluation_failures.saturating_add(1);
        return Ok(());
    };
    if !effects
        .iter()
        .all(|effect| allowed_effects.contains(effect))
    {
        *evaluation_failures = evaluation_failures.saturating_add(1);
        return Ok(());
    }
    let scalar = lower_expression(&expression, operand_indices)?;
    let mut outputs = Vec::with_capacity(observation_arguments.len());
    for arguments in observation_arguments {
        match eval_scalar(&scalar, arguments, api_map) {
            Ok(output) => outputs.push(output),
            Err(_) => {
                *evaluation_failures = evaluation_failures.saturating_add(1);
                return Ok(());
            }
        }
    }
    expressions.push(EnumeratedExpression {
        nodes: expression_nodes(&expression),
        expression,
        value_type,
        outputs,
        canonical_key,
    });
    Ok(())
}

fn enumerate_api_arguments(
    definition: &ApiDefinition,
    expressions: &[EnumeratedExpression],
    limit: usize,
) -> Vec<Vec<TypedSyntaxExpressionIR>> {
    let pools = definition
        .inputs
        .iter()
        .map(|expected| {
            expressions
                .iter()
                .filter(|candidate| &candidate.value_type == expected)
                .take(16)
                .map(|candidate| candidate.expression.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if pools.iter().any(Vec::is_empty) {
        return Vec::new();
    }
    let mut results = vec![Vec::new()];
    for pool in pools {
        let mut next = Vec::new();
        for prefix in &results {
            for expression in &pool {
                if next.len() >= limit {
                    break;
                }
                let mut arguments = prefix.clone();
                arguments.push(expression.clone());
                next.push(arguments);
            }
            if next.len() >= limit {
                break;
            }
        }
        results = next;
    }
    results
}

fn synthesize_conditional(
    request: &TypedMechanismSynthesisGoalIR,
    expressions: &[EnumeratedExpression],
    output_candidates: &[EnumeratedExpression],
    expected: &[Value],
    observation_arguments: &[Vec<Value>],
    operand_indices: &BTreeMap<String, usize>,
    api_map: &BTreeMap<String, &ApiDefinition>,
) -> Result<
    (
        Option<TypedSyntaxExpressionIR>,
        TypedSyntaxExpressionIR,
        Option<TypedSyntaxExpressionIR>,
    ),
    String,
> {
    let mut branch_index = ConditionalBranchIndex::new();
    let mut best: Option<(
        usize,
        String,
        TypedSyntaxExpressionIR,
        TypedSyntaxExpressionIR,
        TypedSyntaxExpressionIR,
    )> = None;
    for condition in expressions
        .iter()
        .filter(|candidate| candidate.value_type == ProgramType::Bool)
    {
        let mask = condition
            .outputs
            .iter()
            .map(|value| match value {
                Value::Bool(value) => Ok(*value),
                _ => Err("TYPED_MECHANISM_CONDITION_EVALUATION_TYPE".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if mask.iter().all(|value| *value) || mask.iter().all(|value| !*value) {
            continue;
        }
        let (true_indices, false_indices) = branch_index
            .entry(mask.clone())
            .or_insert_with(|| conditional_branch_indices(expected, &mask, output_candidates));
        let (Some(true_index), Some(false_index)) = (
            true_indices.first().copied(),
            false_indices.first().copied(),
        ) else {
            continue;
        };
        let true_branch = &output_candidates[true_index];
        let false_branch = &output_candidates[false_index];
        let nodes = condition
            .nodes
            .saturating_add(true_branch.nodes)
            .saturating_add(false_branch.nodes);
        let key = format!(
            "{}|{}|{}",
            condition.canonical_key, true_branch.canonical_key, false_branch.canonical_key
        );
        if best
            .as_ref()
            .is_none_or(|(best_nodes, best_key, ..)| (nodes, &key) < (*best_nodes, best_key))
        {
            best = Some((
                nodes,
                key,
                condition.expression.clone(),
                true_branch.expression.clone(),
                false_branch.expression.clone(),
            ));
        }
    }
    let Some((best_nodes, _, condition, postimage, otherwise)) = best else {
        return Err("TYPED_MECHANISM_SYNTHESIS_EXHAUSTED".to_string());
    };
    ensure_minimal_conditional_hypotheses_identifiable(
        request,
        expressions,
        output_candidates,
        best_nodes,
        observation_arguments,
        operand_indices,
        api_map,
        &branch_index,
    )?;
    Ok((Some(condition), postimage, Some(otherwise)))
}

#[allow(clippy::too_many_arguments)]
fn ensure_minimal_conditional_hypotheses_identifiable(
    request: &TypedMechanismSynthesisGoalIR,
    expressions: &[EnumeratedExpression],
    output_candidates: &[EnumeratedExpression],
    best_nodes: usize,
    observation_arguments: &[Vec<Value>],
    operand_indices: &BTreeMap<String, usize>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    branch_index: &ConditionalBranchIndex,
) -> Result<(), String> {
    let probes = bounded_identifiability_arguments(request, observation_arguments);
    let mut semantic_classes = BTreeSet::new();
    let mut hypotheses = 0_usize;
    for condition in expressions
        .iter()
        .filter(|candidate| candidate.value_type == ProgramType::Bool)
    {
        let mask = condition
            .outputs
            .iter()
            .map(|value| match value {
                Value::Bool(value) => Ok(*value),
                _ => Err("TYPED_MECHANISM_CONDITION_EVALUATION_TYPE".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if mask.iter().all(|value| *value) || mask.iter().all(|value| !*value) {
            continue;
        }
        let Some((true_indices, false_indices)) = branch_index.get(&mask) else {
            return Err("TYPED_MECHANISM_CONDITIONAL_BRANCH_INDEX_MISSING".to_string());
        };
        for true_index in true_indices {
            let true_branch = &output_candidates[*true_index];
            let Some(required_false_nodes) = best_nodes
                .checked_sub(condition.nodes)
                .and_then(|remaining| remaining.checked_sub(true_branch.nodes))
            else {
                continue;
            };
            for false_index in false_indices {
                let false_branch = &output_candidates[*false_index];
                if false_branch.nodes != required_false_nodes {
                    continue;
                }
                if condition
                    .nodes
                    .saturating_add(true_branch.nodes)
                    .saturating_add(false_branch.nodes)
                    != best_nodes
                {
                    continue;
                }
                hypotheses = hypotheses.saturating_add(1);
                if hypotheses > MAX_IDENTIFIABILITY_HYPOTHESES {
                    return Err(format!(
                        "TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_CONDITIONAL_HYPOTHESIS_BUDGET:{}:PROBES:{}",
                        hypotheses,
                        probes.len()
                    ));
                }
                semantic_classes.insert(conditional_probe_signature(
                    condition,
                    true_branch,
                    false_branch,
                    &probes,
                    operand_indices,
                    api_map,
                )?);
                if semantic_classes.len() > 1 {
                    return Err(format!(
                        "TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_CONDITIONAL_HYPOTHESES:{}:SEMANTIC_CLASSES:{}:PROBES:{}",
                        hypotheses,
                        semantic_classes.len(),
                        probes.len()
                    ));
                }
            }
        }
    }
    Ok(())
}

fn conditional_probe_signature(
    condition: &EnumeratedExpression,
    true_branch: &EnumeratedExpression,
    false_branch: &EnumeratedExpression,
    probes: &[Vec<Value>],
    operand_indices: &BTreeMap<String, usize>,
    api_map: &BTreeMap<String, &ApiDefinition>,
) -> Result<String, String> {
    let condition = lower_expression(&condition.expression, operand_indices)?;
    let true_branch = lower_expression(&true_branch.expression, operand_indices)?;
    let false_branch = lower_expression(&false_branch.expression, operand_indices)?;
    let signature = probes
        .iter()
        .map(|arguments| {
            let outcome = match eval_scalar(&condition, arguments, api_map) {
                Ok(Value::Bool(true)) => eval_scalar(&true_branch, arguments, api_map),
                Ok(Value::Bool(false)) => eval_scalar(&false_branch, arguments, api_map),
                Ok(_) => Err("TYPED_MECHANISM_CONDITION_EVALUATION_TYPE".to_string()),
                Err(error) => Err(error),
            };
            match outcome {
                Ok(value) => serde_json::to_string(&("OK", value))
                    .map_err(|error| format!("TYPED_MECHANISM_PROBE_SERIALIZE:{error}")),
                Err(error) => Ok(format!("[\"ERROR\",{error:?}]")),
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_string(&signature)
        .map_err(|error| format!("TYPED_MECHANISM_PROBE_SIGNATURE:{error}"))
}

fn conditional_branch_indices(
    expected: &[Value],
    mask: &[bool],
    output_candidates: &[EnumeratedExpression],
) -> (Vec<usize>, Vec<usize>) {
    let mut true_indices = Vec::new();
    let mut false_indices = Vec::new();
    for (index, candidate) in output_candidates.iter().enumerate() {
        if masked_values_match(&candidate.outputs, expected, mask, true) {
            true_indices.push(index);
        }
        if masked_values_match(&candidate.outputs, expected, mask, false) {
            false_indices.push(index);
        }
    }
    (true_indices, false_indices)
}

fn masked_values_match(
    values: &[Value],
    expected: &[Value],
    mask: &[bool],
    selected: bool,
) -> bool {
    values.len() == expected.len()
        && values.len() == mask.len()
        && values
            .iter()
            .zip(expected)
            .zip(mask)
            .filter(|(_, include)| **include == selected)
            .all(|((value, expected), _)| value == expected)
}

pub fn validate_typed_mechanism_synthesis_goal(
    request: &TypedMechanismSynthesisGoalIR,
) -> Result<(), String> {
    if request.schema != TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA {
        return Err("TYPED_MECHANISM_SYNTHESIS_SCHEMA".to_string());
    }
    if request.goal_id.is_empty() {
        return Err("TYPED_MECHANISM_GOAL_ID_EMPTY".to_string());
    }
    if request.operands.is_empty() || request.operands.len() > MAX_MECHANISM_OPERANDS {
        return Err("TYPED_MECHANISM_OPERAND_BUDGET".to_string());
    }
    if request.public_observations.is_empty()
        || request.public_observations.len() > MAX_MECHANISM_OBSERVATIONS
    {
        return Err("TYPED_MECHANISM_OBSERVATION_BUDGET".to_string());
    }
    if request.max_expression_depth == 0 || request.max_candidates < 16 {
        return Err("TYPED_MECHANISM_SYNTHESIS_BUDGET".to_string());
    }
    let validation_goal = TypedMechanismGoalIR {
        schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
        goal_id: request.goal_id.clone(),
        split: request.split,
        operands: request.operands.clone(),
        output_type: request.output_type.clone(),
        condition: None,
        postimage: TypedSyntaxExpressionIR::Operand {
            role: request.operands[0].role.clone(),
        },
        otherwise: None,
        definitions: request.definitions.clone(),
        allowed_effects: request.allowed_effects.clone(),
        preconditions: request.preconditions.clone(),
        postconditions: request.postconditions.clone(),
        invariants: request.invariants.clone(),
        public_observations: Vec::new(),
        provenance: request.provenance.clone(),
    };
    validate_goal_envelope(&validation_goal)?;
    for observation in &request.public_observations {
        validate_observation_bindings(&validation_goal, observation)?;
    }
    Ok(())
}

fn validate_goal_envelope(goal: &TypedMechanismGoalIR) -> Result<(), String> {
    if goal.schema != TYPED_MECHANISM_GOAL_SCHEMA {
        return Err("TYPED_MECHANISM_SCHEMA".to_string());
    }
    if goal.goal_id.is_empty() {
        return Err("TYPED_MECHANISM_GOAL_ID_EMPTY".to_string());
    }
    if goal.operands.is_empty() || goal.operands.len() > MAX_MECHANISM_OPERANDS {
        return Err("TYPED_MECHANISM_OPERAND_BUDGET".to_string());
    }
    if goal.public_observations.len() > MAX_MECHANISM_OBSERVATIONS {
        return Err("TYPED_MECHANISM_OBSERVATION_BUDGET".to_string());
    }
    let node_count = goal
        .condition
        .iter()
        .map(expression_nodes)
        .sum::<usize>()
        .saturating_add(expression_nodes(&goal.postimage))
        .saturating_add(goal.otherwise.iter().map(expression_nodes).sum::<usize>());
    if node_count == 0 || node_count > MAX_MECHANISM_EXPRESSION_NODES {
        return Err("TYPED_MECHANISM_EXPRESSION_BUDGET".to_string());
    }
    let mut roles = BTreeSet::new();
    for operand in &goal.operands {
        syn::parse_str::<syn::Ident>(&operand.role)
            .map_err(|_| format!("TYPED_MECHANISM_ROLE_NOT_IDENTIFIER:{}", operand.role))?;
        syn::parse_str::<syn::Expr>(&operand.source)
            .map_err(|error| format!("TYPED_MECHANISM_OPERAND_SOURCE:{}:{error}", operand.role))?;
        if !roles.insert(operand.role.clone()) {
            return Err(format!("TYPED_MECHANISM_DUPLICATE_ROLE:{}", operand.role));
        }
    }
    let mut api_tokens = BTreeSet::new();
    for definition in &goal.definitions {
        syn::parse_str::<syn::Path>(&definition.api_token).map_err(|error| {
            format!("TYPED_MECHANISM_API_PATH:{}:{error}", definition.api_token)
        })?;
        if !api_tokens.insert(definition.api_token.clone()) {
            return Err(format!(
                "TYPED_MECHANISM_DUPLICATE_API:{}",
                definition.api_token
            ));
        }
    }
    Ok(())
}

fn infer_expression_type(
    expression: &TypedSyntaxExpressionIR,
    operands: &BTreeMap<String, ProgramType>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    effects: &mut BTreeSet<Effect>,
) -> Result<ProgramType, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => operands
            .get(role)
            .cloned()
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { .. } => Ok(ProgramType::Int),
        TypedSyntaxExpressionIR::BoolLiteral { .. } => Ok(ProgramType::Bool),
        TypedSyntaxExpressionIR::StringLiteral { .. } => Ok(ProgramType::String),
        TypedSyntaxExpressionIR::Unary { operator, input } => {
            let input_type = infer_expression_type(input, operands, definitions, effects)?;
            match (operator, input_type) {
                (UnaryOperator::Negate, ProgramType::Int) => Ok(ProgramType::Int),
                (UnaryOperator::Not, ProgramType::Bool) => Ok(ProgramType::Bool),
                _ => Err("TYPED_MECHANISM_UNARY_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::StringTransform { input, .. } => {
            let input_type = infer_expression_type(input, operands, definitions, effects)?;
            if input_type == ProgramType::String {
                Ok(ProgramType::String)
            } else {
                Err("TYPED_MECHANISM_STRING_TRANSFORM_TYPE".to_string())
            }
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => {
            let left_type = infer_expression_type(left, operands, definitions, effects)?;
            let right_type = infer_expression_type(right, operands, definitions, effects)?;
            use BinaryOperator as Op;
            match (operator, left_type, right_type) {
                (
                    Op::Add | Op::Subtract | Op::Multiply | Op::Divide | Op::Modulo,
                    ProgramType::Int,
                    ProgramType::Int,
                ) => Ok(ProgramType::Int),
                (Op::Add, ProgramType::String, ProgramType::String) => Ok(ProgramType::String),
                (
                    Op::LessThan | Op::LessThanOrEqual | Op::GreaterThan | Op::GreaterThanOrEqual,
                    left,
                    right,
                ) if left == right && matches!(left, ProgramType::Int | ProgramType::String) => {
                    Ok(ProgramType::Bool)
                }
                (Op::Equal | Op::NotEqual, left, right)
                    if left == right
                        && matches!(
                            left,
                            ProgramType::Int | ProgramType::Bool | ProgramType::String
                        ) =>
                {
                    Ok(ProgramType::Bool)
                }
                (Op::And | Op::Or, ProgramType::Bool, ProgramType::Bool) => Ok(ProgramType::Bool),
                _ => Err("TYPED_MECHANISM_BINARY_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Length { input } => {
            let input_type = infer_expression_type(input, operands, definitions, effects)?;
            if matches!(
                input_type,
                ProgramType::String
                    | ProgramType::SequenceInt
                    | ProgramType::NestedSequenceInt
                    | ProgramType::Bytes
            ) {
                Ok(ProgramType::Int)
            } else {
                Err("TYPED_MECHANISM_LENGTH_SOURCE_TYPE".to_string())
            }
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            let collection_type =
                infer_expression_type(collection, operands, definitions, effects)?;
            let index_type = infer_expression_type(index, operands, definitions, effects)?;
            if index_type != ProgramType::Int {
                return Err("TYPED_MECHANISM_INDEX_NOT_INT".to_string());
            }
            match collection_type {
                ProgramType::SequenceInt | ProgramType::Bytes => Ok(ProgramType::Int),
                ProgramType::NestedSequenceInt => Ok(ProgramType::SequenceInt),
                ProgramType::String => Ok(ProgramType::String),
                _ => Err("TYPED_MECHANISM_INDEX_SOURCE_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => {
            let definition = definitions
                .get(api_token)
                .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_API:{api_token}"))?;
            if arguments.len() != definition.inputs.len() {
                return Err(format!("TYPED_MECHANISM_API_ARITY:{api_token}"));
            }
            for (argument, expected) in arguments.iter().zip(&definition.inputs) {
                let actual = infer_expression_type(argument, operands, definitions, effects)?;
                if &actual != expected {
                    return Err(format!("TYPED_MECHANISM_API_INPUT_TYPE:{api_token}"));
                }
            }
            effects.insert(definition.effect.clone());
            Ok(definition.output.clone())
        }
    }
}

fn emit_expression(
    expression: &TypedSyntaxExpressionIR,
    sources: &BTreeMap<String, String>,
    operands: &BTreeMap<String, ProgramType>,
    definitions: &BTreeMap<String, &ApiDefinition>,
) -> Result<String, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => sources
            .get(role)
            .cloned()
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { value } => Ok(format!("{value}i64")),
        TypedSyntaxExpressionIR::BoolLiteral { value } => Ok(value.to_string()),
        TypedSyntaxExpressionIR::StringLiteral { value } => serde_json::to_string(value)
            .map(|literal| format!("{literal}.to_string()"))
            .map_err(|error| format!("TYPED_MECHANISM_STRING_LITERAL_SERIALIZE:{error}")),
        TypedSyntaxExpressionIR::Unary { operator, input } => {
            let input = emit_expression(input, sources, operands, definitions)?;
            Ok(match operator {
                UnaryOperator::Negate => format!("({input}).saturating_neg()"),
                UnaryOperator::Not => format!("(!{input})"),
            })
        }
        TypedSyntaxExpressionIR::StringTransform { operator, input } => {
            let receiver = emit_postfix_receiver(input, sources, operands, definitions)?;
            Ok(match operator {
                StringTransformOperator::Trim => format!("{receiver}.trim().to_string()"),
                StringTransformOperator::Lowercase => format!("{receiver}.to_lowercase()"),
                StringTransformOperator::Uppercase => format!("{receiver}.to_uppercase()"),
            })
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => {
            let left_expression = emit_expression(left, sources, operands, definitions)?;
            let right_expression = emit_expression(right, sources, operands, definitions)?;
            let mut effects = BTreeSet::new();
            let left_type = infer_expression_type(left, operands, definitions, &mut effects)?;
            let right_type = infer_expression_type(right, operands, definitions, &mut effects)?;
            if *operator == BinaryOperator::Add
                && left_type == ProgramType::String
                && right_type == ProgramType::String
            {
                Ok(format!(
                    "format!(\"{{}}{{}}\", {left_expression}, {right_expression})"
                ))
            } else if left_type == ProgramType::Int && right_type == ProgramType::Int {
                Ok(match operator {
                    BinaryOperator::Add => {
                        format!("({left_expression}).saturating_add({right_expression})")
                    }
                    BinaryOperator::Subtract => {
                        format!("({left_expression}).saturating_sub({right_expression})")
                    }
                    BinaryOperator::Multiply => {
                        format!("({left_expression}).saturating_mul({right_expression})")
                    }
                    BinaryOperator::Divide => {
                        format!("({left_expression}).saturating_div({right_expression})")
                    }
                    BinaryOperator::Modulo => {
                        format!("({left_expression}).wrapping_rem({right_expression})")
                    }
                    _ => format!(
                        "({left_expression} {} {right_expression})",
                        binary_token(*operator)
                    ),
                })
            } else {
                Ok(format!(
                    "({left_expression} {} {right_expression})",
                    binary_token(*operator)
                ))
            }
        }
        TypedSyntaxExpressionIR::Length { input } => {
            let mut effects = BTreeSet::new();
            let input_type = infer_expression_type(input, operands, definitions, &mut effects)?;
            let receiver = emit_postfix_receiver(input, sources, operands, definitions)?;
            if input_type == ProgramType::String {
                Ok(format!("{receiver}.chars().count() as i64"))
            } else {
                Ok(format!("{receiver}.len() as i64"))
            }
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            let mut effects = BTreeSet::new();
            let collection_type =
                infer_expression_type(collection, operands, definitions, &mut effects)?;
            let access = format!(
                "{}[{} as usize]",
                emit_postfix_receiver(collection, sources, operands, definitions)?,
                emit_expression(index, sources, operands, definitions)?
            );
            match collection_type {
                ProgramType::SequenceInt => Ok(access),
                ProgramType::NestedSequenceInt => Ok(format!("{access}.clone()")),
                ProgramType::Bytes => Ok(format!("i64::from({access})")),
                ProgramType::String => Ok(format!(
                    "{}.chars().nth({} as usize).expect(\"typed string index\").to_string()",
                    emit_postfix_receiver(collection, sources, operands, definitions)?,
                    emit_expression(index, sources, operands, definitions)?
                )),
                _ => Err("TYPED_MECHANISM_INDEX_SOURCE_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(format!(
            "{api_token}({})",
            arguments
                .iter()
                .map(|argument| emit_expression(argument, sources, operands, definitions))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
    }
}

fn emit_postfix_receiver(
    expression: &TypedSyntaxExpressionIR,
    sources: &BTreeMap<String, String>,
    operands: &BTreeMap<String, ProgramType>,
    definitions: &BTreeMap<String, &ApiDefinition>,
) -> Result<String, String> {
    let emitted = emit_expression(expression, sources, operands, definitions)?;
    Ok(match expression {
        TypedSyntaxExpressionIR::Operand { .. }
        | TypedSyntaxExpressionIR::Call { .. }
        | TypedSyntaxExpressionIR::Index { .. } => emitted,
        _ => format!("({emitted})"),
    })
}

fn lower_expression(
    expression: &TypedSyntaxExpressionIR,
    indices: &BTreeMap<String, usize>,
) -> Result<ScalarExpression, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => indices
            .get(role)
            .copied()
            .map(|index| ScalarExpression::Argument { index })
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { value } => {
            Ok(ScalarExpression::Constant { value: *value })
        }
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            Ok(ScalarExpression::BoolConstant { value: *value })
        }
        TypedSyntaxExpressionIR::StringLiteral { value } => Ok(ScalarExpression::StringConstant {
            value: value.clone(),
        }),
        TypedSyntaxExpressionIR::Unary { operator, input } => Ok(ScalarExpression::Unary {
            operator: *operator,
            input: Box::new(lower_expression(input, indices)?),
        }),
        TypedSyntaxExpressionIR::StringTransform { operator, input } => {
            Ok(ScalarExpression::StringTransform {
                operator: *operator,
                input: Box::new(lower_expression(input, indices)?),
            })
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => Ok(ScalarExpression::Binary {
            operator: *operator,
            left: Box::new(lower_expression(left, indices)?),
            right: Box::new(lower_expression(right, indices)?),
        }),
        TypedSyntaxExpressionIR::Length { input } => Ok(ScalarExpression::Length {
            input: Box::new(lower_expression(input, indices)?),
        }),
        TypedSyntaxExpressionIR::Index { collection, index } => Ok(ScalarExpression::Index {
            collection: Box::new(lower_expression(collection, indices)?),
            index: Box::new(lower_expression(index, indices)?),
        }),
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(ScalarExpression::OpaqueCall {
            api_token: api_token.clone(),
            args: arguments
                .iter()
                .map(|argument| lower_expression(argument, indices))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn validate_observation_bindings(
    goal: &TypedMechanismGoalIR,
    observation: &TypedMechanismObservationIR,
) -> Result<(), String> {
    if observation.operands.len() != goal.operands.len() {
        return Err("TYPED_MECHANISM_OBSERVATION_BINDING_COUNT".to_string());
    }
    for operand in &goal.operands {
        let value = observation
            .operands
            .get(&operand.role)
            .ok_or_else(|| format!("TYPED_MECHANISM_OBSERVATION_MISSING:{}", operand.role))?;
        if value.program_type() != operand.value_type {
            return Err(format!("TYPED_MECHANISM_OBSERVATION_TYPE:{}", operand.role));
        }
    }
    if observation.expected_postimage.program_type() != goal.output_type {
        return Err("TYPED_MECHANISM_OBSERVATION_OUTPUT_TYPE".to_string());
    }
    Ok(())
}

fn complete_expression(
    condition: Option<&str>,
    postimage: &str,
    otherwise: Option<&str>,
) -> Result<String, String> {
    match (condition, otherwise) {
        (Some(condition), Some(otherwise)) => Ok(format!(
            "if {condition} {{ {postimage} }} else {{ {otherwise} }}"
        )),
        (None, None) => Ok(postimage.to_string()),
        _ => Err("TYPED_MECHANISM_CONDITION_POSTIMAGE_SHAPE".to_string()),
    }
}

fn expression_nodes(expression: &TypedSyntaxExpressionIR) -> usize {
    match expression {
        TypedSyntaxExpressionIR::Operand { .. }
        | TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::BoolLiteral { .. }
        | TypedSyntaxExpressionIR::StringLiteral { .. } => 1,
        TypedSyntaxExpressionIR::Unary { input, .. } => 1 + expression_nodes(input),
        TypedSyntaxExpressionIR::StringTransform { input, .. } => 1 + expression_nodes(input),
        TypedSyntaxExpressionIR::Binary { left, right, .. } => {
            1 + expression_nodes(left) + expression_nodes(right)
        }
        TypedSyntaxExpressionIR::Length { input } => 1 + expression_nodes(input),
        TypedSyntaxExpressionIR::Index { collection, index } => {
            1 + expression_nodes(collection) + expression_nodes(index)
        }
        TypedSyntaxExpressionIR::Call { arguments, .. } => {
            1 + arguments.iter().map(expression_nodes).sum::<usize>()
        }
    }
}

fn binary_token(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
    }
}

fn rust_type(value_type: &ProgramType) -> &'static str {
    match value_type {
        ProgramType::Int => "i64",
        ProgramType::Bool => "bool",
        ProgramType::String => "String",
        ProgramType::SequenceInt => "Vec<i64>",
        ProgramType::NestedSequenceInt => "Vec<Vec<i64>>",
        ProgramType::Bytes => "Vec<u8>",
        ProgramType::Image => "Sem5Image",
        ProgramType::Unit => "()",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operand(role: &str, source: &str, value_type: ProgramType) -> SourceOperandIR {
        SourceOperandIR {
            role: role.to_string(),
            source: source.to_string(),
            value_type,
        }
    }

    #[test]
    fn source_bound_seed_extends_bounded_composition_without_answer_authority() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "source_seed_composition".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left", "left", ProgramType::Int),
                operand("right", "right", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: [(5, 2), (4, 1), (3, -2), (-4, 1)]
                .into_iter()
                .map(|(left, right)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("left".to_string(), Value::Int(left)),
                        ("right".to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Int((left + right) * (left - right)),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 1,
            max_candidates: 256,
            provenance: vec!["SOURCE_SEED_CANARY".to_string()],
        };
        let left = TypedSyntaxExpressionIR::Operand {
            role: "left".to_string(),
        };
        let right = TypedSyntaxExpressionIR::Operand {
            role: "right".to_string(),
        };
        let seed = TypedSyntaxExpressionIR::Binary {
            operator: BinaryOperator::Multiply,
            left: Box::new(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            }),
            right: Box::new(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Subtract,
                left: Box::new(left),
                right: Box::new(right),
            }),
        };

        assert!(synthesize_typed_mechanism_goal(&request).is_err());
        let receipt = synthesize_typed_mechanism_goal_with_source_seeds_and_priors(
            &request,
            std::slice::from_ref(&seed),
            &[],
        )
        .unwrap();
        assert_eq!(receipt.winning_goal.postimage, seed);
        assert!(receipt.selected_operator_id.is_none());
        validate_typed_mechanism_synthesis_receipt(&receipt).unwrap();
    }

    fn role(name: &str) -> TypedSyntaxExpressionIR {
        TypedSyntaxExpressionIR::Operand {
            role: name.to_string(),
        }
    }

    fn assert_rust_source_compiles_and_runs(goal_id: &str, source: String) {
        let workspace = std::env::temp_dir().join(format!(
            "b-core-string-template-{}-{}",
            std::process::id(),
            goal_id
        ));
        if workspace.exists() {
            std::fs::remove_dir_all(&workspace).unwrap();
        }
        std::fs::create_dir(&workspace).unwrap();
        let source_path = workspace.join("program.rs");
        let executable = workspace.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        std::fs::write(&source_path, source).unwrap();
        let compile = std::process::Command::new("rustc")
            .current_dir(&workspace)
            .args(["--edition=2021", "-C", "opt-level=0", "-C", "debuginfo=0"])
            .arg(&source_path)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        let runtime = compile
            .status
            .success()
            .then(|| std::process::Command::new(&executable).output().unwrap());
        let compile_stderr = String::from_utf8_lossy(&compile.stderr).into_owned();
        let compiled = compile.status.success();
        let runtime_valid = runtime
            .as_ref()
            .is_some_and(|output| output.status.success());
        std::fs::remove_dir_all(&workspace).unwrap();
        assert!(
            compiled && runtime_valid && !workspace.exists(),
            "compile={compiled} runtime={runtime_valid} cleanup={} stderr={compile_stderr}",
            !workspace.exists()
        );
    }

    fn assert_template_compiles_and_runs(template: &ConcreteSyntaxTemplateIR, invocation: &str) {
        let source = format!(
            "{}\nfn main() {{ let _value = __b_core_typed_mechanism({invocation}); }}\n",
            template.canonical_compilable_source
        );
        assert_rust_source_compiles_and_runs(&template.goal_id, source);
    }

    #[test]
    fn string_ir_compiles_for_concat_unicode_length_and_unicode_index() {
        let cases = [
            (
                "string_concat",
                vec![
                    operand("left", "left", ProgramType::String),
                    operand("right", "right", ProgramType::String),
                ],
                ProgramType::String,
                TypedSyntaxExpressionIR::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(role("left")),
                    right: Box::new(role("right")),
                },
                BTreeMap::from([
                    ("left".to_string(), Value::String("한".to_string())),
                    ("right".to_string(), Value::String("글".to_string())),
                ]),
                Value::String("한글".to_string()),
                "\"한\".to_string(), \"글\".to_string()",
            ),
            (
                "string_unicode_length",
                vec![operand("value", "value", ProgramType::String)],
                ProgramType::Int,
                TypedSyntaxExpressionIR::Length {
                    input: Box::new(role("value")),
                },
                BTreeMap::from([("value".to_string(), Value::String("한글".to_string()))]),
                Value::Int(2),
                "\"한글\".to_string()",
            ),
            (
                "string_unicode_index",
                vec![
                    operand("value", "value", ProgramType::String),
                    operand("position", "position", ProgramType::Int),
                ],
                ProgramType::String,
                TypedSyntaxExpressionIR::Index {
                    collection: Box::new(role("value")),
                    index: Box::new(role("position")),
                },
                BTreeMap::from([
                    ("value".to_string(), Value::String("한글".to_string())),
                    ("position".to_string(), Value::Int(1)),
                ]),
                Value::String("글".to_string()),
                "\"한글\".to_string(), 1i64",
            ),
        ];
        for (goal_id, operands, output_type, postimage, values, expected, invocation) in cases {
            let goal = TypedMechanismGoalIR {
                schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
                goal_id: goal_id.to_string(),
                split: DataSplit::FreshBlind,
                operands,
                output_type,
                condition: None,
                postimage,
                otherwise: None,
                definitions: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                invariants: Vec::new(),
                public_observations: vec![TypedMechanismObservationIR {
                    operands: values,
                    expected_postimage: expected,
                }],
                provenance: vec!["CROSS_LANGUAGE_STRING_CANARY".to_string()],
            };
            let template = lower_typed_mechanism_goal(&goal).unwrap();
            assert_eq!(template.public_observations_passed, 1);
            assert_template_compiles_and_runs(&template, invocation);
            let program = crate::sem5::learner::synthesize(
                &template.program_task,
                crate::sem5::model::SynthesisCondition::PrimitiveA,
                &[],
            )
            .unwrap();
            let artifact = crate::sem5::emitter::emit_rust(
                &program,
                &goal.definitions,
                &goal.public_observations[0].operands,
            )
            .unwrap();
            assert_rust_source_compiles_and_runs(
                &format!("{}-program-ir", goal.goal_id),
                artifact.source,
            );
        }
    }

    #[test]
    fn string_transform_ir_compiles_and_executes_across_all_lowerings() {
        let cases = [
            (
                "string_trim",
                StringTransformOperator::Trim,
                "  Alpha  ",
                "Alpha",
            ),
            (
                "string_lowercase",
                StringTransformOperator::Lowercase,
                "MiXeD",
                "mixed",
            ),
            (
                "string_uppercase",
                StringTransformOperator::Uppercase,
                "Alpha",
                "ALPHA",
            ),
        ];
        for (goal_id, operator, input, expected) in cases {
            let goal = TypedMechanismGoalIR {
                schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
                goal_id: goal_id.to_string(),
                split: DataSplit::FreshBlind,
                operands: vec![operand("value", "value", ProgramType::String)],
                output_type: ProgramType::String,
                condition: None,
                postimage: TypedSyntaxExpressionIR::StringTransform {
                    operator,
                    input: Box::new(role("value")),
                },
                otherwise: None,
                definitions: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                invariants: Vec::new(),
                public_observations: vec![TypedMechanismObservationIR {
                    operands: BTreeMap::from([(
                        "value".to_string(),
                        Value::String(input.to_string()),
                    )]),
                    expected_postimage: Value::String(expected.to_string()),
                }],
                provenance: vec!["STRING_TRANSFORM_CROSS_LOWERING_CANARY".to_string()],
            };
            let template = lower_typed_mechanism_goal(&goal).unwrap();
            assert_eq!(template.public_observations_passed, 1);
            assert_template_compiles_and_runs(&template, &format!("{input:?}.to_string()"));
            let program = crate::sem5::learner::synthesize(
                &template.program_task,
                crate::sem5::model::SynthesisCondition::PrimitiveA,
                &[],
            )
            .unwrap();
            let artifact = crate::sem5::emitter::emit_rust(
                &program,
                &goal.definitions,
                &goal.public_observations[0].operands,
            )
            .unwrap();
            assert_rust_source_compiles_and_runs(&format!("{goal_id}-program-ir"), artifact.source);
        }
    }

    #[test]
    fn verified_string_transform_operator_transfers_to_renamed_role() {
        let request = |goal_id: &str, value_role: &str| TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand(value_role, value_role, ProgramType::String)],
            output_type: ProgramType::String,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: vec![
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([(
                        value_role.to_string(),
                        Value::String("  Alpha ".to_string()),
                    )]),
                    expected_postimage: Value::String("Alpha".to_string()),
                },
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([(
                        value_role.to_string(),
                        Value::String("\tBeta\n".to_string()),
                    )]),
                    expected_postimage: Value::String("Beta".to_string()),
                },
            ],
            require_conditional: false,
            max_expression_depth: 1,
            max_candidates: 64,
            provenance: vec!["STRING_TRANSFORM_OPERATOR_TRANSFER_CANARY".to_string()],
        };
        let learned = synthesize_typed_mechanism_goal(&request("learn_trim", "raw")).unwrap();
        assert!(matches!(
            learned.winning_goal.postimage,
            TypedSyntaxExpressionIR::StringTransform {
                operator: StringTransformOperator::Trim,
                ..
            }
        ));
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&learned, "b".repeat(64)).unwrap();
        let renamed = synthesize_typed_mechanism_goal_with_priors(
            &request("reuse_trim", "payload"),
            std::slice::from_ref(&operator),
        )
        .unwrap();
        assert!(renamed.preferred_operator_selected);
        assert_eq!(renamed.preferred_operator_attempts, 1);
        assert_eq!(renamed.candidates_enumerated, 1);
        assert_eq!(renamed.selected_operator_id, Some(operator.operator_id));
    }

    #[test]
    fn bounded_relational_family_synthesizes_compiles_and_transfers() {
        let observations_for = |operator: BinaryOperator,
                                left_role: &str,
                                right_role: &str|
         -> Vec<TypedMechanismObservationIR> {
            let cases = match operator {
                BinaryOperator::NotEqual => [
                    (1, 2, true),
                    (2, 1, true),
                    (2, 2, false),
                    (7, 7, false),
                    (5, 9, true),
                ],
                BinaryOperator::LessThanOrEqual => [
                    (1, 2, true),
                    (5, 5, true),
                    (7, 3, false),
                    (-2, -1, true),
                    (4, 2, false),
                ],
                BinaryOperator::GreaterThanOrEqual => [
                    (2, 1, true),
                    (5, 5, true),
                    (3, 7, false),
                    (-1, -2, true),
                    (2, 4, false),
                ],
                _ => unreachable!(),
            };
            cases
                .into_iter()
                .map(|(left, right, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        (left_role.to_string(), Value::Int(left)),
                        (right_role.to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Bool(expected),
                })
                .collect()
        };
        let request = |goal_id: &str,
                       operator: BinaryOperator,
                       left_role: &str,
                       right_role: &str| TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand(left_role, left_role, ProgramType::Int),
                operand(right_role, right_role, ProgramType::Int),
            ],
            output_type: ProgramType::Bool,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: observations_for(operator, left_role, right_role),
            require_conditional: false,
            max_expression_depth: 1,
            max_candidates: 1_024,
            provenance: vec!["RELATIONAL_OPERATOR_FAMILY_CANARY".to_string()],
        };

        let mut learned_less_or_equal = None;
        for (goal_id, operator) in [
            ("not_equal", BinaryOperator::NotEqual),
            ("less_than_or_equal", BinaryOperator::LessThanOrEqual),
            ("greater_than_or_equal", BinaryOperator::GreaterThanOrEqual),
        ] {
            let receipt =
                synthesize_typed_mechanism_goal(&request(goal_id, operator, "value", "boundary"))
                    .unwrap();
            let relation_matches = match (&receipt.winning_goal.postimage, operator) {
                (
                    TypedSyntaxExpressionIR::Binary {
                        operator: BinaryOperator::NotEqual,
                        ..
                    },
                    BinaryOperator::NotEqual,
                ) => true,
                (
                    TypedSyntaxExpressionIR::Binary {
                        operator: selected,
                        left,
                        right,
                    },
                    expected,
                ) if matches!(
                    (selected, expected),
                    (
                        BinaryOperator::LessThanOrEqual,
                        BinaryOperator::LessThanOrEqual
                    ) | (
                        BinaryOperator::GreaterThanOrEqual,
                        BinaryOperator::GreaterThanOrEqual
                    )
                ) =>
                {
                    matches!(
                        (left.as_ref(), right.as_ref()),
                        (
                            TypedSyntaxExpressionIR::Operand { role: left },
                            TypedSyntaxExpressionIR::Operand { role: right }
                        ) if left == "value" && right == "boundary"
                    )
                }
                (
                    TypedSyntaxExpressionIR::Binary {
                        operator: selected,
                        left,
                        right,
                    },
                    expected,
                ) if matches!(
                    (selected, expected),
                    (
                        BinaryOperator::GreaterThanOrEqual,
                        BinaryOperator::LessThanOrEqual
                    ) | (
                        BinaryOperator::LessThanOrEqual,
                        BinaryOperator::GreaterThanOrEqual
                    )
                ) =>
                {
                    matches!(
                        (left.as_ref(), right.as_ref()),
                        (
                            TypedSyntaxExpressionIR::Operand { role: left },
                            TypedSyntaxExpressionIR::Operand { role: right }
                        ) if left == "boundary" && right == "value"
                    )
                }
                _ => false,
            };
            assert!(
                relation_matches,
                "expected={operator:?} selected={:?}",
                receipt.winning_goal.postimage
            );
            assert!(
                receipt
                    .template
                    .canonical_compilable_source
                    .contains(" != ")
                    || receipt
                        .template
                        .canonical_compilable_source
                        .contains(" <= ")
                    || receipt
                        .template
                        .canonical_compilable_source
                        .contains(" >= ")
            );
            assert_template_compiles_and_runs(&receipt.template, "1i64, 2i64");
            let program = crate::sem5::learner::synthesize(
                &receipt.template.program_task,
                crate::sem5::model::SynthesisCondition::PrimitiveA,
                &[],
            )
            .unwrap();
            let artifact = crate::sem5::emitter::emit_rust(
                &program,
                &[],
                &receipt.winning_goal.public_observations[0].operands,
            )
            .unwrap();
            assert_rust_source_compiles_and_runs(&format!("{goal_id}-program-ir"), artifact.source);
            if operator == BinaryOperator::LessThanOrEqual {
                learned_less_or_equal = Some(receipt);
            }
        }

        let learned = learned_less_or_equal.unwrap();
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&learned, "c".repeat(64)).unwrap();
        let renamed = synthesize_typed_mechanism_goal_with_priors(
            &request(
                "renamed_less_than_or_equal",
                BinaryOperator::LessThanOrEqual,
                "candidate",
                "ceiling",
            ),
            std::slice::from_ref(&operator),
        )
        .unwrap();
        assert!(renamed.preferred_operator_selected);
        assert_eq!(renamed.preferred_operator_attempts, 1);
        assert_eq!(renamed.candidates_enumerated, 1);
        assert_eq!(renamed.selected_operator_id, Some(operator.operator_id));
    }

    #[test]
    fn observationally_ambiguous_minimal_hypotheses_fail_closed() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "sparse_at_least".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("value", "value", ProgramType::Int),
                operand("floor", "floor", ProgramType::Int),
            ],
            output_type: ProgramType::Bool,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: [(2, 1, true), (2, 2, true), (1, 2, false)]
                .into_iter()
                .map(|(value, floor, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("value".to_string(), Value::Int(value)),
                        ("floor".to_string(), Value::Int(floor)),
                    ]),
                    expected_postimage: Value::Bool(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 1,
            max_candidates: 1_024,
            provenance: vec!["IDENTIFIABILITY_NEGATIVE_CANARY".to_string()],
        };
        let error = synthesize_typed_mechanism_goal(&request).unwrap_err();
        assert!(
            error
                .starts_with("TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_HYPOTHESES:"),
            "{error}"
        );
        assert!(error.contains(":SEMANTIC_CLASSES:"));
        assert!(error.contains(":PROBES:"));
    }

    #[test]
    fn observationally_ambiguous_conditional_hypotheses_fail_closed() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "sparse_conditional_boundary".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("value", "value", ProgramType::Int),
                operand("floor", "floor", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: [(2, 1, 1), (2, 2, 1), (1, 2, -1)]
                .into_iter()
                .map(|(value, floor, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("value".to_string(), Value::Int(value)),
                        ("floor".to_string(), Value::Int(floor)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: true,
            max_expression_depth: 1,
            max_candidates: 1_024,
            provenance: vec!["CONDITIONAL_IDENTIFIABILITY_NEGATIVE_CANARY".to_string()],
        };
        let error = synthesize_typed_mechanism_goal(&request).unwrap_err();
        assert!(
            error.starts_with(
                "TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_CONDITIONAL_HYPOTHESES:"
            ),
            "{error}"
        );
        assert!(error.contains(":SEMANTIC_CLASSES:"));
        assert!(error.contains(":PROBES:"));
    }

    #[test]
    fn verified_string_operator_transfers_to_renamed_roles_before_enumeration() {
        let request = |goal_id: &str, left: &str, right: &str| TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand(left, left, ProgramType::String),
                operand(right, right, ProgramType::String),
            ],
            output_type: ProgramType::String,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: vec![
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        (left.to_string(), Value::String("ab".to_string())),
                        (right.to_string(), Value::String("cd".to_string())),
                    ]),
                    expected_postimage: Value::String("abcd".to_string()),
                },
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        (left.to_string(), Value::String("x".to_string())),
                        (right.to_string(), Value::String("yz".to_string())),
                    ]),
                    expected_postimage: Value::String("xyz".to_string()),
                },
            ],
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["STRING_OPERATOR_TRANSFER_CANARY".to_string()],
        };
        let learned = synthesize_typed_mechanism_goal(&request("learn", "left", "right")).unwrap();
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&learned, "a".repeat(64)).unwrap();
        let renamed = synthesize_typed_mechanism_goal_with_priors(
            &request("reuse", "prefix", "suffix"),
            std::slice::from_ref(&operator),
        )
        .unwrap();
        assert!(renamed.preferred_operator_selected);
        assert_eq!(renamed.preferred_operator_attempts, 1);
        assert_eq!(renamed.selected_operator_id, Some(operator.operator_id));
        assert_eq!(renamed.candidates_enumerated, 1);
        assert!(renamed
            .template
            .canonical_compilable_source
            .contains("format!"));
    }

    #[test]
    fn typed_goal_lowers_renamed_repository_operands_to_concrete_guarded_postimage() {
        let observations = [(7, 3, 5, 10), (2, 3, 5, -1), (5, 4, 5, 1)]
            .into_iter()
            .map(
                |(current, delta, threshold, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("current".to_string(), Value::Int(current)),
                        ("delta".to_string(), Value::Int(delta)),
                        ("threshold".to_string(), Value::Int(threshold)),
                    ]),
                    expected_postimage: Value::Int(expected),
                },
            )
            .collect();
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "rename_independent_guarded_update".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("current", "self.runtime.accumulator", ProgramType::Int),
                operand("delta", "event.payload.delta", ProgramType::Int),
                operand("threshold", "limits.activation_floor", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            condition: Some(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::GreaterThan,
                left: Box::new(role("current")),
                right: Box::new(role("threshold")),
            }),
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(role("current")),
                right: Box::new(role("delta")),
            },
            otherwise: Some(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Subtract,
                left: Box::new(role("current")),
                right: Box::new(role("delta")),
            }),
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: vec!["operands are valid at the source site".to_string()],
            postconditions: vec!["guard selects exactly one postimage".to_string()],
            invariants: vec!["unselected branch has no effect".to_string()],
            public_observations: observations,
            provenance: vec!["RENAMED_REPOSITORY_CANARY".to_string()],
        };

        let template = lower_typed_mechanism_goal(&goal).expect("lower typed mechanism");
        assert!(template.syntax_parse_pass);
        assert!(template.type_effect_check_pass);
        assert_eq!(template.public_observations_checked, 3);
        assert_eq!(template.public_observations_passed, 3);
        assert!(template
            .complete_expression_source
            .contains("self.runtime.accumulator"));
        assert!(template
            .complete_expression_source
            .contains("event.payload.delta"));
        assert!(matches!(
            template.program_task.relation,
            RelationSpec::Mechanism { .. }
        ));
    }

    #[test]
    fn type_mismatch_is_rejected_before_source_installation() {
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "invalid_types".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("enabled", "state.enabled", ProgramType::Bool)],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Unary {
                operator: UnaryOperator::Negate,
                input: Box::new(role("enabled")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&goal),
            Err("TYPED_MECHANISM_UNARY_TYPE".to_string())
        );

        let string_transform_goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "invalid_string_transform_type".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("count", "state.count", ProgramType::Int)],
            output_type: ProgramType::String,
            condition: None,
            postimage: TypedSyntaxExpressionIR::StringTransform {
                operator: StringTransformOperator::Trim,
                input: Box::new(role("count")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&string_transform_goal),
            Err("TYPED_MECHANISM_STRING_TRANSFORM_TYPE".to_string())
        );

        let invalid_ordered_comparison = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "invalid_ordered_bool_comparison".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left", "left", ProgramType::Bool),
                operand("right", "right", ProgramType::Bool),
            ],
            output_type: ProgramType::Bool,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::LessThanOrEqual,
                left: Box::new(role("left")),
                right: Box::new(role("right")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&invalid_ordered_comparison),
            Err("TYPED_MECHANISM_BINARY_TYPE".to_string())
        );
    }

    #[test]
    fn collection_index_fails_closed_on_non_integer_and_out_of_bounds_indices() {
        let base_goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "collection_index_contract".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("items", "state.items", ProgramType::SequenceInt),
                operand("position", "request.position", ProgramType::Bool),
            ],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Index {
                collection: Box::new(role("items")),
                index: Box::new(role("position")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&base_goal),
            Err("TYPED_MECHANISM_INDEX_NOT_INT".to_string())
        );

        let mut out_of_bounds_goal = base_goal;
        out_of_bounds_goal.operands[1].value_type = ProgramType::Int;
        out_of_bounds_goal.public_observations = vec![TypedMechanismObservationIR {
            operands: BTreeMap::from([
                ("items".to_string(), Value::Sequence(Vec::new())),
                ("position".to_string(), Value::Int(0)),
            ]),
            expected_postimage: Value::Int(0),
        }];
        assert_eq!(
            lower_typed_mechanism_goal(&out_of_bounds_goal),
            Err("TYPED_MECHANISM_OBSERVATION_EXECUTE:0:INDEX_OUT_OF_BOUNDS".to_string())
        );
    }

    #[test]
    fn authorized_operator_snapshot_is_shared_without_weakening_authority() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "length_operator_seed".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("values", "values", ProgramType::SequenceInt)],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: vec![
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([("values".to_string(), Value::Sequence(vec![1, 2]))]),
                    expected_postimage: Value::Int(2),
                },
                TypedMechanismObservationIR {
                    operands: BTreeMap::from([("values".to_string(), Value::Sequence(vec![7]))]),
                    expected_postimage: Value::Int(1),
                },
            ],
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: Vec::new(),
        };
        let synthesis = synthesize_typed_mechanism_goal(&request).unwrap();
        let evidence_sha256 = "a".repeat(64);
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&synthesis, evidence_sha256.clone())
                .unwrap();
        let state_dir = std::env::temp_dir().join(format!(
            "b-core-authorized-typed-operators-{}",
            std::process::id()
        ));
        if state_dir.exists() {
            std::fs::remove_dir_all(&state_dir).unwrap();
        }
        let operator_directory = typed_mechanism_operator_directory(&state_dir);
        let authority_directory = typed_mechanism_operator_authority_directory(&state_dir);
        std::fs::create_dir_all(&operator_directory).unwrap();
        std::fs::create_dir_all(&authority_directory).unwrap();
        std::fs::write(
            operator_directory.join(format!("{}.json", operator.operator_id)),
            serde_json::to_vec_pretty(&operator).unwrap(),
        )
        .unwrap();

        let repair_id = "b".repeat(64);
        let repair_receipt_sha256 = "c".repeat(64);
        let authority_id = sha256(
            format!(
                "SOURCE_BOUND_OPERATOR_AUTHORITY_1:{}:{}:{}:{}",
                operator.operator_id, repair_id, repair_receipt_sha256, evidence_sha256
            )
            .as_bytes(),
        );
        let mut authority = TypedMechanismOperatorAuthorityReceiptIR {
            schema: SOURCE_BOUND_OPERATOR_AUTHORITY_SCHEMA.to_string(),
            authority_id: authority_id.clone(),
            operator_id: operator.operator_id.clone(),
            operator_sha256: json_sha256(&operator).unwrap(),
            repair_id,
            repair_receipt_sha256,
            sandbox_output_sha256: evidence_sha256,
            candidate_sha256: "d".repeat(64),
            sandbox_verified: true,
            sandbox_cleaned: true,
            authoritative_scope_stable: true,
            candidate_installed: false,
            authoritative_source_write_events: 0,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
            promotion_generation: 7,
            receipt_sha256: String::new(),
        };
        authority.receipt_sha256 = json_sha256(&authority).unwrap();
        std::fs::write(
            authority_directory.join(format!("{authority_id}.json")),
            serde_json::to_vec_pretty(&authority).unwrap(),
        )
        .unwrap();

        assert_eq!(
            load_authorized_typed_mechanism_operators(&state_dir, 256).unwrap(),
            vec![operator]
        );
        let mut tampered = authority;
        tampered.promotion_generation += 1;
        assert_eq!(
            validate_typed_mechanism_operator_authority(&tampered),
            Err("SOURCE_BOUND_OPERATOR_AUTHORITY_HASH_MISMATCH".to_string())
        );
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn counterexample_rejects_semantically_wrong_postimage() {
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "counterexample".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("left", "node.left", ProgramType::Int)],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(role("left")),
                right: Box::new(TypedSyntaxExpressionIR::IntLiteral { value: 1 }),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: vec![TypedMechanismObservationIR {
                operands: BTreeMap::from([("left".to_string(), Value::Int(4))]),
                expected_postimage: Value::Int(9),
            }],
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&goal),
            Err("TYPED_MECHANISM_COUNTEREXAMPLE:0".to_string())
        );
    }

    #[test]
    fn observations_synthesize_postimage_without_a_solution_template() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "unknown_repository_addition".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left_role", "graph.nodes[source].weight", ProgramType::Int),
                operand("right_role", "delta.payload", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["matches all public postimages".to_string()],
            invariants: Vec::new(),
            public_observations: [(2, 3, 5), (-4, 9, 5), (11, -8, 3)]
                .into_iter()
                .map(|(left, right, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("left_role".to_string(), Value::Int(left)),
                        ("right_role".to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["PUBLIC_OBSERVATION_ONLY".to_string()],
        };

        let receipt =
            synthesize_typed_mechanism_goal(&request).expect("synthesize typed expression");
        assert!(receipt.counterexample_guided_selection);
        assert!(!receipt.conditional_synthesized);
        assert!(receipt.candidates_falsified > 0);
        assert!(receipt.template.postimage_source.contains("saturating_add"));
        assert!(receipt
            .template
            .postimage_source
            .contains("graph.nodes[source].weight"));
        assert_eq!(receipt.template.public_observations_passed, 3);
    }

    #[test]
    fn successful_expression_operator_transfers_by_typed_role_and_short_circuits_search() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "first_repository".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left", "left", ProgramType::Int),
                operand("right", "right", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: [(2, 3, 5), (4, 7, 11)]
                .into_iter()
                .map(|(left, right, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("left".to_string(), Value::Int(left)),
                        ("right".to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: Vec::new(),
        };
        let learned = synthesize_typed_mechanism_goal(&request).unwrap();
        validate_typed_mechanism_synthesis_receipt(&learned).unwrap();
        let mut missing_request = learned.clone();
        missing_request.synthesis_request = None;
        assert_eq!(
            typed_mechanism_improvement_operator_from_receipt(&missing_request, "a".repeat(64)),
            Err("TYPED_MECHANISM_RECEIPT_REQUEST_MISSING".to_string())
        );
        let mut forged_goal = learned.clone();
        forged_goal.winning_goal.postimage = TypedSyntaxExpressionIR::IntLiteral { value: 0 };
        assert_eq!(
            validate_typed_mechanism_synthesis_receipt(&forged_goal),
            Err("TYPED_MECHANISM_COUNTEREXAMPLE:0".to_string())
        );
        let mut forged_accounting = learned.clone();
        forged_accounting.counterexample_guided_selection =
            !forged_accounting.counterexample_guided_selection;
        assert_eq!(
            validate_typed_mechanism_synthesis_receipt(&forged_accounting),
            Err("TYPED_MECHANISM_RECEIPT_ACCOUNTING".to_string())
        );
        let mut forged_hash = learned.clone();
        forged_hash.receipt_sha256 = "0".repeat(64);
        assert_eq!(
            validate_typed_mechanism_synthesis_receipt(&forged_hash),
            Err("TYPED_MECHANISM_RECEIPT_HASH".to_string())
        );
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&learned, "a".repeat(64)).unwrap();
        let renamed = TypedMechanismSynthesisGoalIR {
            goal_id: "renamed_repository".to_string(),
            operands: vec![
                operand("alpha", "payload.alpha", ProgramType::Int),
                operand("beta", "payload.beta", ProgramType::Int),
            ],
            public_observations: [(8, 5, 13), (-4, 9, 5)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..request
        };
        let transferred =
            synthesize_typed_mechanism_goal_with_priors(&renamed, std::slice::from_ref(&operator))
                .unwrap();
        assert!(transferred.preferred_operator_selected);
        assert_eq!(
            transferred.selected_operator_id.as_deref(),
            Some(operator.operator_id.as_str())
        );
        assert_eq!(transferred.candidates_enumerated, 1);
        assert!(transferred.candidates_enumerated < learned.candidates_enumerated);
        assert!(transferred
            .template
            .postimage_source
            .contains("payload.alpha"));
        assert!(transferred
            .template
            .postimage_source
            .contains("payload.beta"));

        let multiplication_seed = TypedMechanismSynthesisGoalIR {
            goal_id: "multiplication_seed".to_string(),
            public_observations: [(3, 4, 12), (-2, 5, -10)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..renamed.clone()
        };
        let multiplication_receipt = synthesize_typed_mechanism_goal(&multiplication_seed).unwrap();
        let multiplication_operator = typed_mechanism_improvement_operator_from_receipt(
            &multiplication_receipt,
            "b".repeat(64),
        )
        .unwrap();
        let collision = synthesize_typed_mechanism_goal_with_priors(
            &renamed,
            &[multiplication_operator.clone(), operator.clone()],
        )
        .unwrap();
        assert!(!collision.parallel_operator_evaluation);
        assert_eq!(collision.preferred_operator_attempts, 2);
        assert_eq!(
            collision.selected_operator_id.as_deref(),
            Some(operator.operator_id.as_str())
        );
        assert_eq!(
            collision.rejected_operator_ids,
            [multiplication_operator.operator_id]
        );

        let counterexample = TypedMechanismSynthesisGoalIR {
            goal_id: "operator_counterexample".to_string(),
            public_observations: [(3, 4, 12), (-2, 5, -10)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..renamed
        };
        let revised = synthesize_typed_mechanism_goal_with_priors(
            &counterexample,
            std::slice::from_ref(&operator),
        )
        .unwrap();
        assert!(!revised.preferred_operator_selected);
        assert_eq!(revised.preferred_operator_attempts, 1);
        assert!(revised.template.postimage_source.contains("saturating_mul"));

        let mut tampered = operator;
        tampered.postimage = TypedSyntaxExpressionIR::IntLiteral { value: 0 };
        assert_eq!(
            validate_typed_mechanism_improvement_operator(&tampered),
            Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ID_MISMATCH".to_string())
        );
    }

    #[test]
    fn authorized_operators_compose_into_a_new_behaviorally_falsified_goal() {
        fn binary_operator(
            goal_id: &str,
            samples: &[(i64, i64, i64)],
            evidence: char,
        ) -> TypedMechanismImprovementOperatorIR {
            let request = TypedMechanismSynthesisGoalIR {
                schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
                goal_id: goal_id.to_string(),
                split: DataSplit::FreshBlind,
                operands: vec![
                    operand("left", "input.left", ProgramType::Int),
                    operand("right", "input.right", ProgramType::Int),
                ],
                output_type: ProgramType::Int,
                definitions: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                invariants: Vec::new(),
                public_observations: samples
                    .iter()
                    .map(|(left, right, expected)| TypedMechanismObservationIR {
                        operands: BTreeMap::from([
                            ("left".to_string(), Value::Int(*left)),
                            ("right".to_string(), Value::Int(*right)),
                        ]),
                        expected_postimage: Value::Int(*expected),
                    })
                    .collect(),
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
                provenance: vec!["AUTHORIZED_COMPONENT_TEST".to_string()],
            };
            let receipt = synthesize_typed_mechanism_goal(&request).unwrap();
            typed_mechanism_improvement_operator_from_receipt(
                &receipt,
                evidence.to_string().repeat(64),
            )
            .unwrap()
        }

        let addition = binary_operator("operator_add", &[(2, 3, 5), (-4, 9, 5)], 'a');
        let multiplication = binary_operator("operator_multiply", &[(2, 3, 6), (-4, 2, -8)], 'b');
        let programs = compose_authorized_typed_operator_programs(
            &[addition.clone(), multiplication.clone()],
            8,
        )
        .unwrap();

        assert!(!programs.is_empty());
        let program = programs
            .iter()
            .find(|program| {
                program.goal.provenance.contains(&format!(
                    "AUTHORIZED_PRODUCER_OPERATOR:{}",
                    addition.operator_id
                )) && program.goal.provenance.contains(&format!(
                    "AUTHORIZED_CONSUMER_OPERATOR:{}",
                    multiplication.operator_id
                ))
            })
            .expect("addition output should wire into multiplication");
        let goal = &program.goal;
        assert!(goal.goal_id.starts_with("compound_operator_"));
        assert_eq!(goal.operands.len(), 3);
        assert!(goal.public_observations.len() >= 8);
        validate_typed_mechanism_synthesis_goal(goal).unwrap();
        let receipt = synthesize_typed_mechanism_goal_with_priors(
            goal,
            std::slice::from_ref(&program.operator_proposal),
        )
        .unwrap();
        assert_eq!(
            receipt.selected_operator_id.as_deref(),
            Some(program.operator_proposal.operator_id.as_str())
        );
        validate_typed_mechanism_synthesis_receipt(&receipt).unwrap();

        assert!(compose_authorized_typed_operator_goals(&[addition], 8)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn native_operator_genesis_is_standalone_typed_and_behaviorally_falsified() {
        let programs = native_typed_operator_genesis_programs(&BTreeSet::new(), 32).unwrap();
        assert!(programs.len() >= 16);
        assert!(programs
            .iter()
            .any(|program| program.primitive_id == "INT_ADD"));
        assert!(programs
            .iter()
            .any(|program| program.primitive_id == "STRING_TRIM"));
        assert!(programs.iter().all(|program| {
            program.schema == NATIVE_TYPED_OPERATOR_GENESIS_SCHEMA
                && program.goal.public_observations.len() >= 2
                && !serde_json::to_string(program)
                    .unwrap()
                    .to_ascii_uppercase()
                    .contains("SYNAPSE")
        }));
        let operator_ids = programs
            .iter()
            .map(|program| program.operator_proposal.operator_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(operator_ids.len(), programs.len());
        for program in &programs {
            validate_typed_mechanism_synthesis_goal(&program.goal).unwrap();
            validate_typed_mechanism_improvement_operator(&program.operator_proposal).unwrap();
            let receipt = synthesize_typed_mechanism_goal_with_priors(
                &program.goal,
                std::slice::from_ref(&program.operator_proposal),
            )
            .unwrap();
            assert_eq!(
                receipt.selected_operator_id.as_deref(),
                Some(program.operator_proposal.operator_id.as_str())
            );
        }

        let excluded = native_typed_operator_genesis_programs(&BTreeSet::new(), 1)
            .unwrap()
            .pop()
            .unwrap();
        let remaining = native_typed_operator_genesis_programs(
            &BTreeSet::from([excluded.operator_proposal.operator_id.clone()]),
            1,
        )
        .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_ne!(
            remaining[0].operator_proposal.operator_id,
            excluded.operator_proposal.operator_id
        );
    }

    #[test]
    fn composition_prunes_dead_operands_and_searches_past_authorized_page() {
        fn seal(
            operand_types: Vec<ProgramType>,
            output_type: ProgramType,
            postimage: TypedSyntaxExpressionIR,
            evidence: char,
        ) -> TypedMechanismImprovementOperatorIR {
            let mut operator = TypedMechanismImprovementOperatorIR {
                schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
                operator_id: String::new(),
                operand_types,
                output_type,
                condition: None,
                postimage,
                otherwise: None,
                validation_contract: vec![
                    "PUBLIC_OBSERVATION_REPLAY".to_string(),
                    "TYPE_EFFECT_CHECK".to_string(),
                    "SOURCE_BOUND_ATOMIC_MATERIALIZATION".to_string(),
                    "SANDBOX_PUBLIC_REGRESSION".to_string(),
                    "AUTHORITATIVE_SCOPE_STABLE".to_string(),
                ],
                evidence_sha256: evidence.to_string().repeat(64),
            };
            let mut identity = operator.clone();
            identity.evidence_sha256.clear();
            operator.operator_id = sha256(&serde_json::to_vec(&identity).unwrap());
            validate_typed_mechanism_improvement_operator(&operator).unwrap();
            operator
        }

        let bloated_identity = seal(
            vec![
                ProgramType::Int,
                ProgramType::String,
                ProgramType::Bool,
                ProgramType::Int,
            ],
            ProgramType::Int,
            role("ARG_0"),
            'a',
        );
        let multiply = seal(
            vec![ProgramType::Int, ProgramType::Int],
            ProgramType::Int,
            TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Multiply,
                left: Box::new(role("ARG_0")),
                right: Box::new(role("ARG_1")),
            },
            'b',
        );
        let compact_identity = seal(vec![ProgramType::Int], ProgramType::Int, role("ARG_0"), 'c');
        assert_eq!(
            typed_operator_composition_semantic_key(&bloated_identity).unwrap(),
            typed_operator_composition_semantic_key(&compact_identity).unwrap()
        );
        let compact =
            composed_operator_goal_from_validated(&bloated_identity, &multiply, 0, true, None)
                .unwrap()
                .unwrap();
        assert_eq!(compact.operator_proposal.operand_types.len(), 2);
        assert_eq!(compact.goal.operands.len(), 2);

        let without_duplicate = compose_authorized_typed_operator_programs(
            &[compact_identity.clone(), multiply.clone()],
            8,
        )
        .unwrap()
        .into_iter()
        .map(|program| program.operator_proposal.operator_id)
        .collect::<BTreeSet<_>>();
        let with_duplicate = compose_authorized_typed_operator_programs(
            &[bloated_identity.clone(), compact_identity, multiply.clone()],
            8,
        )
        .unwrap()
        .into_iter()
        .map(|program| program.operator_proposal.operator_id)
        .collect::<BTreeSet<_>>();
        assert_eq!(without_duplicate, with_duplicate);

        let first = compose_authorized_typed_operator_programs(
            &[bloated_identity.clone(), multiply.clone()],
            1,
        )
        .unwrap();
        assert_eq!(first.len(), 1);
        let excluded = BTreeSet::from([first[0].operator_proposal.operator_id.clone()]);
        let next = compose_authorized_typed_operator_programs_excluding(
            &[bloated_identity, multiply],
            &excluded,
            1,
        )
        .unwrap();
        assert_eq!(next.len(), 1);
        assert_ne!(
            first[0].operator_proposal.operator_id,
            next[0].operator_proposal.operator_id
        );
    }

    #[test]
    fn operator_composition_rejects_structurally_dead_and_behaviorally_masked_wires() {
        fn seal(
            mut operator: TypedMechanismImprovementOperatorIR,
        ) -> TypedMechanismImprovementOperatorIR {
            operator.operator_id.clear();
            let mut identity = operator.clone();
            identity.evidence_sha256.clear();
            operator.operator_id = sha256(&serde_json::to_vec(&identity).unwrap());
            validate_typed_mechanism_improvement_operator(&operator).unwrap();
            operator
        }

        let validation_contract = vec![
            "PUBLIC_OBSERVATION_REPLAY".to_string(),
            "TYPE_EFFECT_CHECK".to_string(),
            "SOURCE_BOUND_ATOMIC_MATERIALIZATION".to_string(),
            "SANDBOX_PUBLIC_REGRESSION".to_string(),
            "AUTHORITATIVE_SCOPE_STABLE".to_string(),
        ];
        let producer = seal(TypedMechanismImprovementOperatorIR {
            schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
            operator_id: String::new(),
            operand_types: vec![ProgramType::Int],
            output_type: ProgramType::Int,
            condition: None,
            postimage: role("ARG_0"),
            otherwise: None,
            validation_contract: validation_contract.clone(),
            evidence_sha256: "a".repeat(64),
        });
        let dead_consumer = seal(TypedMechanismImprovementOperatorIR {
            schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
            operator_id: String::new(),
            operand_types: vec![ProgramType::Int, ProgramType::Int],
            output_type: ProgramType::Int,
            condition: None,
            postimage: role("ARG_0"),
            otherwise: None,
            validation_contract: validation_contract.clone(),
            evidence_sha256: "b".repeat(64),
        });
        assert!(typed_operator_operand_has_observed_influence(&dead_consumer, 0).unwrap());
        assert!(!typed_operator_operand_is_structurally_referenced(
            &dead_consumer,
            1
        ));
        assert!(!typed_operator_operand_has_observed_influence(&dead_consumer, 1).unwrap());
        assert!(
            composed_operator_goal_from_validated(&producer, &dead_consumer, 0, true, None)
                .unwrap()
                .is_some()
        );
        assert!(
            composed_operator_goal_from_validated(&producer, &dead_consumer, 1, false, None)
                .unwrap()
                .is_none()
        );

        let masked_consumer = seal(TypedMechanismImprovementOperatorIR {
            schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
            operator_id: String::new(),
            operand_types: vec![ProgramType::Int],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Subtract,
                left: Box::new(role("ARG_0")),
                right: Box::new(role("ARG_0")),
            },
            otherwise: None,
            validation_contract,
            evidence_sha256: "c".repeat(64),
        });
        assert!(typed_operator_operand_is_structurally_referenced(
            &masked_consumer,
            0
        ));
        assert!(!typed_operator_operand_has_observed_influence(&masked_consumer, 0).unwrap());
        assert!(
            composed_operator_goal_from_validated(&producer, &masked_consumer, 0, false, None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn observations_synthesize_condition_and_two_postimages() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "unknown_absolute_delta".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("new_value", "sample.current", ProgramType::Int),
                operand("old_value", "sample.previous", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["result is an unsigned distance".to_string()],
            invariants: Vec::new(),
            public_observations: [(9, 4, 5), (3, 8, 5), (-2, -9, 7), (-8, -3, 5)]
                .into_iter()
                .map(
                    |(new_value, old_value, expected)| TypedMechanismObservationIR {
                        operands: BTreeMap::from([
                            ("new_value".to_string(), Value::Int(new_value)),
                            ("old_value".to_string(), Value::Int(old_value)),
                        ]),
                        expected_postimage: Value::Int(expected),
                    },
                )
                .collect(),
            require_conditional: true,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["PUBLIC_OBSERVATION_ONLY".to_string()],
        };

        let receipt =
            synthesize_typed_mechanism_goal(&request).expect("synthesize guarded typed expression");
        assert!(receipt.conditional_synthesized);
        assert!(receipt.template.condition_source.is_some());
        assert!(receipt.template.otherwise_source.is_some());
        assert!(receipt
            .template
            .complete_expression_source
            .starts_with("if "));
        assert_eq!(receipt.template.public_observations_passed, 4);
    }
}
