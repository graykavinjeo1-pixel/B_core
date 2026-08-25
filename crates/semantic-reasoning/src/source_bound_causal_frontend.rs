//! Source-bound causal alternatives for repository-native repair synthesis.
//!
//! The typed composition kernel cannot repair a repository when the language
//! frontend drops the concrete operand, guard, postimage, or public-symbol
//! owner before synthesis.  This module makes that transport explicit.  It
//! parses Python without importing or executing the target, preserves the
//! exact public symbol and its same-file call closure, feeds only typed public
//! observations into the existing SEM-5 kernel, and materializes the winning
//! expression through the same atomic edit primitive used by Rust repair.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use quote::ToTokens;
use serde::{Deserialize, Serialize};

use crate::bounded_parallel::{
    map_ordered_batched_by as parallel_map_ordered_batched_by, worker_count_for,
};
use crate::self_repair_contract::sha256;
use crate::sem5::model::{
    BinaryOperator, DataSplit, Effect, ProgramType, StringTransformOperator, UnaryOperator,
};
use crate::sem5::typed_mechanism::{
    lower_typed_mechanism_goal, synthesize_typed_mechanism_goal_with_source_seeds_and_priors,
    typed_mechanism_improvement_operator_from_receipt, validate_typed_mechanism_synthesis_receipt,
    ConcreteSyntaxTemplateIR, SourceOperandIR, TypedMechanismGoalIR,
    TypedMechanismImprovementOperatorIR, TypedMechanismObservationIR,
    TypedMechanismSynthesisGoalIR, TypedMechanismSynthesisReceiptIR, TypedSyntaxExpressionIR,
    TYPED_MECHANISM_GOAL_SCHEMA, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
};
use crate::source_proposal_kernel::{
    compose_source_edit_proposals, rank_source_proposals, SourceEditProposalIR,
    SourceProposalCompositionRequirementIR, SourceProposalKernelInput,
    SourceProposalRankingEvidenceIR,
};
use crate::structural_source_repair::{
    apply_edit_atom, synthesize_structural_repair, ByteRange, SourceEditAtom,
    StructuralRepairProgram,
};

pub const SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_REQUEST_1";
pub const SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_RECEIPT_1";
pub const SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA: &str =
    "B_CORE_SOURCE_BOUND_REPOSITORY_DISCOVERY_1";
pub const SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA: &str =
    "B_CORE_SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_1";
pub const CALL_IDENTITY_PREDICATE_REFINEMENT_SCHEMA: &str =
    "B_CORE_CALL_IDENTITY_PREDICATE_REFINEMENT_1";
pub const PREDICATE_REFINEMENT_LOWERING_RECEIPT_SCHEMA: &str =
    "B_CORE_PREDICATE_REFINEMENT_LOWERING_RECEIPT_1";
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_TEST_SOURCES: usize = 64;
const MAX_TEST_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_CAUSAL_ALTERNATIVES: usize = 32;
const MAX_DEPENDENCY_CLOSURE: usize = 64;
const MAX_SOURCE_BOUND_PATCH_VARIANTS: usize = 64;
const MAX_CANDIDATE_VALIDATION_BATCH_ITEMS: usize = 16;
const MAX_CANDIDATE_VALIDATION_BATCH_BYTES: usize = 16 * 1024 * 1024;
const CAUSAL_ALTERNATIVE_ITEMS_PER_WORKER: usize = 4;
const PYTHON_HOST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalFrontendFailureKind {
    PublicInformationInsufficient,
    ConflictingSourceBoundEdits,
    UnsupportedLanguageSyntax,
}

impl CausalFrontendFailureKind {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::PublicInformationInsufficient => "PUBLIC_INFORMATION_INSUFFICIENT",
            Self::ConflictingSourceBoundEdits => "CONFLICTING_SOURCE_BOUND_EDITS",
            Self::UnsupportedLanguageSyntax => "UNSUPPORTED_LANGUAGE_SYNTAX",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalFrontendFailure {
    pub kind: CausalFrontendFailureKind,
    pub detail: String,
}

impl CausalFrontendFailure {
    pub fn public(detail: impl Into<String>) -> Self {
        Self {
            kind: CausalFrontendFailureKind::PublicInformationInsufficient,
            detail: detail.into(),
        }
    }

    fn conflict(detail: impl Into<String>) -> Self {
        Self {
            kind: CausalFrontendFailureKind::ConflictingSourceBoundEdits,
            detail: detail.into(),
        }
    }

    fn unsupported(detail: impl Into<String>) -> Self {
        Self {
            kind: CausalFrontendFailureKind::UnsupportedLanguageSyntax,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for CausalFrontendFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.kind.as_code(), self.detail)
    }
}

impl std::error::Error for CausalFrontendFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceLanguageBackend {
    RustSyn,
    PythonAst,
}

/// One typed equality relation in a call-identity predicate. Sources are exact
/// repository AST projections (field access or a zero-argument accessor), not
/// names inferred by this lowering. The diagnostic/frontend owns the semantic
/// role assignment; this compiler verifies that all three relations preserve
/// the same two receiver roots before it emits an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateRelationBindingIR {
    pub left_source: String,
    pub right_source: String,
    pub value_type: ProgramType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallIdentityPredicateRefinementIR {
    pub schema: String,
    pub source_relative_path: PathBuf,
    pub source: String,
    pub predicate_range: ByteRange,
    pub identity: PredicateRelationBindingIR,
    pub receiver: PredicateRelationBindingIR,
    pub owner: PredicateRelationBindingIR,
    /// Required only for the Python backend. Rust never launches a language
    /// host and leaves this field empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_executable: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateRefinementLoweringReceiptIR {
    pub schema: String,
    pub language_backend: SourceLanguageBackend,
    pub typed_goal: TypedMechanismGoalIR,
    pub concrete_template: ConcreteSyntaxTemplateIR,
    pub materialized_patch: MaterializedSourceBoundPatchIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_structural_repair_program: Option<StructuralRepairProgram>,
    pub receiver_root_relation_preserved: bool,
    pub owner_root_relation_preserved: bool,
    pub original_identity_predicate_replaced: bool,
    pub receipt_sha256: String,
}

pub fn language_backend_for_path(
    path: &Path,
) -> Result<SourceLanguageBackend, CausalFrontendFailure> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Ok(SourceLanguageBackend::RustSyn),
        Some("py") => Ok(SourceLanguageBackend::PythonAst),
        extension => Err(CausalFrontendFailure::unsupported(format!(
            "SOURCE_EXTENSION:{}",
            extension.unwrap_or("NONE")
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundCausalAlternativeIR {
    pub alternative_id: String,
    /// Exact repository-native qualified symbol, e.g. `Rational.normalize`.
    pub public_symbol: String,
    pub public_observations: Vec<TypedMechanismObservationIR>,
    #[serde(default)]
    pub allowed_effects: Vec<Effect>,
    #[serde(default)]
    pub require_conditional: bool,
    #[serde(default = "default_expression_depth")]
    pub max_expression_depth: usize,
    #[serde(default = "default_candidate_budget")]
    pub max_candidates: usize,
}

fn default_expression_depth() -> usize {
    2
}

fn default_candidate_budget() -> usize {
    1_024
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundCausalRequestIR {
    pub schema: String,
    pub source_relative_path: PathBuf,
    pub source: String,
    pub python_executable: PathBuf,
    pub alternatives: Vec<SourceBoundCausalAlternativeIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryTestSourceIR {
    pub relative_path: PathBuf,
    pub source: String,
}

/// Product entry point for deriving causal alternatives from repository
/// source and public tests. `target_symbols` is normally populated from a
/// failing test diagnostic; when empty, only statically contradicted or
/// explicit-hole functions are eligible, preventing speculative rewrites of
/// already-correct code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundRepositoryDiscoveryRequestIR {
    pub schema: String,
    pub source_relative_path: PathBuf,
    pub source: String,
    pub test_sources: Vec<RepositoryTestSourceIR>,
    pub python_executable: PathBuf,
    #[serde(default)]
    pub target_symbols: Vec<String>,
    #[serde(default)]
    pub allowed_effects: Vec<Effect>,
    #[serde(default = "default_expression_depth")]
    pub max_expression_depth: usize,
    #[serde(default = "default_candidate_budget")]
    pub max_candidates: usize,
}

/// Practical repository entry point. The caller supplies paths, not copied
/// source text or a preselected answer. The frontend reads only bounded,
/// non-symlink files inside the canonical repository root, derives public
/// observations from the actual tests, and then uses the same typed synthesis
/// and atomic lowering path as the in-memory API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundRepositoryPathDiscoveryRequestIR {
    pub schema: String,
    pub repository_root: PathBuf,
    pub source_relative_path: PathBuf,
    pub test_relative_paths: Vec<PathBuf>,
    pub python_executable: PathBuf,
    #[serde(default)]
    pub target_symbols: Vec<String>,
    #[serde(default)]
    pub allowed_effects: Vec<Effect>,
    #[serde(default = "default_expression_depth")]
    pub max_expression_depth: usize,
    #[serde(default = "default_candidate_budget")]
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalCutBranch {
    Unconditional,
    Then,
    Else,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundCausalCutIR {
    pub branch: CausalCutBranch,
    pub condition_source: Option<String>,
    pub condition_range: Option<ByteRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_template: Option<TypedSyntaxExpressionIR>,
    pub postimage_source: String,
    pub postimage_range: ByteRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postimage_template: Option<TypedSyntaxExpressionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundFunctionTemplateIR {
    pub qualified_symbol: String,
    pub owner: String,
    pub is_async: bool,
    pub operands: Vec<SourceOperandIR>,
    pub output_type: ProgramType,
    pub operand_type_evidence: BTreeMap<String, String>,
    pub output_type_evidence: String,
    pub effects: Vec<String>,
    pub direct_dependencies: Vec<String>,
    pub execution_dependency_closure: Vec<String>,
    #[serde(default)]
    pub external_callers: Vec<String>,
    pub cuts: Vec<SourceBoundCausalCutIR>,
    pub source_template_sha256: String,
}

fn source_bound_function_template_hash(
    template: &SourceBoundFunctionTemplateIR,
) -> Result<String, CausalFrontendFailure> {
    serde_json::to_vec(&(
        &template.qualified_symbol,
        &template.operands,
        &template.output_type,
        &template.execution_dependency_closure,
        &template.external_callers,
        &template.cuts,
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|error| CausalFrontendFailure::public(format!("TEMPLATE_HASH:{error}")))
}

fn validate_source_bound_function_template(
    source: &str,
    template: &SourceBoundFunctionTemplateIR,
) -> Result<(), CausalFrontendFailure> {
    if template.qualified_symbol.is_empty()
        || template.owner != qualified_symbol_owner(&template.qualified_symbol)
        || template.operands.is_empty()
        || template.cuts.is_empty()
        || !template_closure_is_preserved(template)
    {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_TEMPLATE_STRUCTURAL_BINDING",
        ));
    }
    let mut roles = BTreeSet::new();
    for operand in &template.operands {
        if operand.role.is_empty()
            || operand.source.is_empty()
            || !roles.insert(operand.role.as_str())
            || !template.operand_type_evidence.contains_key(&operand.role)
        {
            return Err(CausalFrontendFailure::conflict(
                "SOURCE_TEMPLATE_OPERAND_BINDING",
            ));
        }
    }
    for cut in &template.cuts {
        let postimage = source
            .get(cut.postimage_range.start..cut.postimage_range.end)
            .ok_or_else(|| CausalFrontendFailure::conflict("SOURCE_TEMPLATE_POSTIMAGE_RANGE"))?;
        if postimage != cut.postimage_source {
            return Err(CausalFrontendFailure::conflict(
                "SOURCE_TEMPLATE_POSTIMAGE_ORIGIN",
            ));
        }
        match (&cut.condition_source, cut.condition_range) {
            (None, None) if cut.branch == CausalCutBranch::Unconditional => {}
            (Some(condition_source), Some(range))
                if cut.branch != CausalCutBranch::Unconditional =>
            {
                let condition = source.get(range.start..range.end).ok_or_else(|| {
                    CausalFrontendFailure::conflict("SOURCE_TEMPLATE_CONDITION_RANGE")
                })?;
                if condition != condition_source {
                    return Err(CausalFrontendFailure::conflict(
                        "SOURCE_TEMPLATE_CONDITION_ORIGIN",
                    ));
                }
            }
            _ => {
                return Err(CausalFrontendFailure::conflict(
                    "SOURCE_TEMPLATE_CONDITION_BINDING",
                ))
            }
        }
    }
    if template.source_template_sha256 != source_bound_function_template_hash(template)? {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_TEMPLATE_HASH_BINDING",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedSourceBoundPatchIR {
    pub predecessor_sha256: String,
    pub edit: SourceEditAtom,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub candidate_replay_sha256: String,
    pub candidate_materialization_is_one_to_one: bool,
}

/// A predecessor-bound edit program for a patch variant.  Variant search may
/// create dozens of combinations for the same source file, so retaining the
/// entire postimage in every combination would make memory scale with
/// `source_bytes * variants`.  The authoritative postimage is reconstructed
/// from this program and accepted only when both frozen hashes replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayableSourceBoundPatchIR {
    pub predecessor_sha256: String,
    pub edit: SourceEditAtom,
    pub candidate_sha256: String,
    pub candidate_replay_sha256: String,
    pub candidate_materialization_is_one_to_one: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PredicateProjectionShape {
    root: String,
    selector: String,
}

fn transparent_rust_expression(expression: &syn::Expr) -> &syn::Expr {
    match expression {
        syn::Expr::Group(group) => transparent_rust_expression(&group.expr),
        syn::Expr::Paren(paren) => transparent_rust_expression(&paren.expr),
        _ => expression,
    }
}

fn rust_expression_key(expression: &syn::Expr) -> String {
    transparent_rust_expression(expression)
        .to_token_stream()
        .to_string()
}

fn rust_projection_shape(source: &str) -> Result<PredicateProjectionShape, CausalFrontendFailure> {
    let expression = syn::parse_str::<syn::Expr>(source).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("RUST_PREDICATE_PROJECTION_PARSE:{error}"))
    })?;
    match transparent_rust_expression(&expression) {
        syn::Expr::Field(field) => {
            let syn::Member::Named(member) = &field.member else {
                return Err(CausalFrontendFailure::unsupported(
                    "RUST_TUPLE_FIELD_IS_NOT_SEMANTIC_PROJECTION",
                ));
            };
            Ok(PredicateProjectionShape {
                root: rust_expression_key(&field.base),
                selector: format!("FIELD:{member}"),
            })
        }
        syn::Expr::MethodCall(call) if call.args.is_empty() && call.turbofish.is_none() => {
            Ok(PredicateProjectionShape {
                root: rust_expression_key(&call.receiver),
                selector: format!("METHOD:{}", call.method),
            })
        }
        _ => Err(CausalFrontendFailure::unsupported(
            "RUST_PREDICATE_OPERAND_NOT_FIELD_OR_ZERO_ARG_ACCESSOR",
        )),
    }
}

fn validate_rust_identity_predicate(
    predicate: &str,
    identity: &PredicateRelationBindingIR,
) -> Result<(), CausalFrontendFailure> {
    let predicate = syn::parse_str::<syn::Expr>(predicate).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("RUST_IDENTITY_PREDICATE_PARSE:{error}"))
    })?;
    let left = syn::parse_str::<syn::Expr>(&identity.left_source).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("RUST_IDENTITY_LEFT_PARSE:{error}"))
    })?;
    let right = syn::parse_str::<syn::Expr>(&identity.right_source).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("RUST_IDENTITY_RIGHT_PARSE:{error}"))
    })?;
    let syn::Expr::Binary(binary) = transparent_rust_expression(&predicate) else {
        return Err(CausalFrontendFailure::conflict(
            "ORIGINAL_PREDICATE_NOT_IDENTITY_EQUALITY",
        ));
    };
    if !matches!(binary.op, syn::BinOp::Eq(_)) {
        return Err(CausalFrontendFailure::conflict(
            "ORIGINAL_PREDICATE_NOT_IDENTITY_EQUALITY",
        ));
    }
    let observed_left = rust_expression_key(&binary.left);
    let observed_right = rust_expression_key(&binary.right);
    let expected_left = rust_expression_key(&left);
    let expected_right = rust_expression_key(&right);
    if !((observed_left == expected_left && observed_right == expected_right)
        || (observed_left == expected_right && observed_right == expected_left))
    {
        return Err(CausalFrontendFailure::conflict(
            "ORIGINAL_IDENTITY_OPERANDS_NOT_EXACTLY_BOUND",
        ));
    }
    Ok(())
}

fn validate_projection_relation_contract(
    shapes: &[PredicateProjectionShape],
) -> Result<(), CausalFrontendFailure> {
    if shapes.len() != 6 {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_PROJECTION_ARITY",
        ));
    }
    let left_root = &shapes[0].root;
    let right_root = &shapes[1].root;
    if left_root == right_root
        || shapes[2].root != *left_root
        || shapes[4].root != *left_root
        || shapes[3].root != *right_root
        || shapes[5].root != *right_root
    {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_RECEIVER_OWNER_ROOT_RELATION",
        ));
    }
    if shapes[0].selector != shapes[1].selector
        || shapes[2].selector != shapes[3].selector
        || shapes[4].selector != shapes[5].selector
    {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_RELATION_SELECTOR_MISMATCH",
        ));
    }
    let selectors = [
        &shapes[0].selector,
        &shapes[2].selector,
        &shapes[4].selector,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if selectors.len() != 3 {
        return Err(CausalFrontendFailure::conflict(
            "IDENTITY_RECEIVER_OWNER_ROLES_NOT_DISTINCT",
        ));
    }
    Ok(())
}

const PYTHON_PREDICATE_REFINEMENT_VALIDATOR: &str = r#"
import ast, json, sys

request = json.load(sys.stdin)

def fail(detail):
    print(json.dumps({"ok": False, "detail": detail}, ensure_ascii=False))
    raise SystemExit(0)

def parse_expression(source, label):
    try:
        return ast.parse(source, mode="eval").body
    except Exception as error:
        fail(label + ":" + str(error))

def projection(node):
    if isinstance(node, ast.Attribute):
        return {
            "root": ast.dump(node.value, annotate_fields=True, include_attributes=False),
            "selector": "FIELD:" + node.attr,
        }
    if (isinstance(node, ast.Call) and not node.args and not node.keywords
            and isinstance(node.func, ast.Attribute)):
        return {
            "root": ast.dump(node.func.value, annotate_fields=True, include_attributes=False),
            "selector": "METHOD:" + node.func.attr,
        }
    fail("PYTHON_PREDICATE_OPERAND_NOT_ATTRIBUTE_OR_ZERO_ARG_ACCESSOR")

projections = [parse_expression(value, "PYTHON_PROJECTION_PARSE") for value in request["projections"]]
predicate = parse_expression(request["predicate"], "PYTHON_IDENTITY_PREDICATE_PARSE")
if not (isinstance(predicate, ast.Compare) and len(predicate.ops) == 1
        and isinstance(predicate.ops[0], ast.Eq) and len(predicate.comparators) == 1):
    fail("ORIGINAL_PREDICATE_NOT_IDENTITY_EQUALITY")
observed = [
    ast.dump(predicate.left, annotate_fields=True, include_attributes=False),
    ast.dump(predicate.comparators[0], annotate_fields=True, include_attributes=False),
]
expected = [
    ast.dump(projections[0], annotate_fields=True, include_attributes=False),
    ast.dump(projections[1], annotate_fields=True, include_attributes=False),
]
if not (observed == expected or observed == list(reversed(expected))):
    fail("ORIGINAL_IDENTITY_OPERANDS_NOT_EXACTLY_BOUND")
try:
    ast.parse(request["candidate_source"], mode="exec")
except Exception as error:
    fail("PYTHON_REFINED_CANDIDATE_PARSE:" + str(error))
print(json.dumps({"ok": True, "detail": "", "shapes": [projection(node) for node in projections]}, ensure_ascii=False))
"#;

#[derive(Debug, Deserialize)]
struct PythonPredicateRefinementValidation {
    ok: bool,
    detail: String,
    #[serde(default)]
    shapes: Vec<PredicateProjectionShape>,
}

fn python_refinement_shapes(
    executable: &Path,
    predicate: &str,
    projections: &[String],
    candidate_source: &str,
) -> Result<Vec<PredicateProjectionShape>, CausalFrontendFailure> {
    let input = serde_json::to_vec(&serde_json::json!({
        "predicate": predicate,
        "projections": projections,
        "candidate_source": candidate_source,
    }))
    .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_REFINEMENT_INPUT:{error}")))?;
    let stdout = run_python_json_host(executable, PYTHON_PREDICATE_REFINEMENT_VALIDATOR, &input)?;
    let validation: PythonPredicateRefinementValidation =
        serde_json::from_slice(&stdout).map_err(|error| {
            CausalFrontendFailure::unsupported(format!("PYTHON_REFINEMENT_OUTPUT:{error}"))
        })?;
    if !validation.ok {
        return Err(CausalFrontendFailure::conflict(format!(
            "PYTHON_PREDICATE_REFINEMENT:{}",
            validation.detail
        )));
    }
    Ok(validation.shapes)
}

fn equality(left: &str, right: &str) -> TypedSyntaxExpressionIR {
    TypedSyntaxExpressionIR::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(TypedSyntaxExpressionIR::Operand {
            role: left.to_string(),
        }),
        right: Box::new(TypedSyntaxExpressionIR::Operand {
            role: right.to_string(),
        }),
    }
}

fn conjunction(
    left: TypedSyntaxExpressionIR,
    right: TypedSyntaxExpressionIR,
) -> TypedSyntaxExpressionIR {
    TypedSyntaxExpressionIR::Binary {
        operator: BinaryOperator::And,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn predicate_refinement_goal(
    request: &CallIdentityPredicateRefinementIR,
) -> Result<TypedMechanismGoalIR, CausalFrontendFailure> {
    let supported = |kind: &ProgramType| {
        matches!(
            kind,
            ProgramType::Int | ProgramType::Bool | ProgramType::String
        )
    };
    if !supported(&request.identity.value_type)
        || !supported(&request.receiver.value_type)
        || !supported(&request.owner.value_type)
    {
        return Err(CausalFrontendFailure::unsupported(
            "PREDICATE_EQUALITY_TYPE_NOT_SUPPORTED",
        ));
    }
    let roles = [
        (
            "LEFT_IDENTITY",
            &request.identity.left_source,
            &request.identity.value_type,
        ),
        (
            "RIGHT_IDENTITY",
            &request.identity.right_source,
            &request.identity.value_type,
        ),
        (
            "LEFT_RECEIVER",
            &request.receiver.left_source,
            &request.receiver.value_type,
        ),
        (
            "RIGHT_RECEIVER",
            &request.receiver.right_source,
            &request.receiver.value_type,
        ),
        (
            "LEFT_OWNER",
            &request.owner.left_source,
            &request.owner.value_type,
        ),
        (
            "RIGHT_OWNER",
            &request.owner.right_source,
            &request.owner.value_type,
        ),
    ];
    let operands = roles
        .into_iter()
        .map(|(role, source, value_type)| SourceOperandIR {
            role: role.to_string(),
            source: source.clone(),
            value_type: value_type.clone(),
        })
        .collect::<Vec<_>>();
    let postimage = conjunction(
        conjunction(
            equality("LEFT_IDENTITY", "RIGHT_IDENTITY"),
            equality("LEFT_RECEIVER", "RIGHT_RECEIVER"),
        ),
        equality("LEFT_OWNER", "RIGHT_OWNER"),
    );
    let goal_id = format!(
        "CALL_IDENTITY_PREDICATE_REFINEMENT:{}",
        &sha256(
            serde_json::to_vec(&(
                &request.source_relative_path,
                request.predicate_range,
                &request.identity,
                &request.receiver,
                &request.owner,
            ))
            .map_err(|error| {
                CausalFrontendFailure::public(format!("PREDICATE_REFINEMENT_GOAL_HASH:{error}"))
            })?
            .as_slice()
        )[..16]
    );
    Ok(TypedMechanismGoalIR {
        schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
        goal_id,
        split: DataSplit::Discovery,
        operands,
        output_type: ProgramType::Bool,
        condition: None,
        postimage,
        otherwise: None,
        definitions: Vec::new(),
        allowed_effects: vec![Effect::Pure],
        preconditions: vec![
            "CURRENT_PREDICATE_IS_EXACT_IDENTITY_EQUALITY".to_string(),
            "IDENTITY_RECEIVER_OWNER_SHARE_LEFT_AND_RIGHT_RECEIVER_ROOTS".to_string(),
        ],
        postconditions: vec![
            "IDENTITY_RELATION_PRESERVED".to_string(),
            "RECEIVER_RELATION_REQUIRED".to_string(),
            "OWNER_RELATION_REQUIRED".to_string(),
        ],
        invariants: vec![
            "NO_RECEIVER_OR_OWNER_CROSS_BINDING".to_string(),
            "ONE_TYPED_GOAL_ONE_MATERIALIZED_SOURCE_EDIT".to_string(),
        ],
        public_observations: Vec::new(),
        provenance: vec![
            "SOURCE_BOUND_CALL_IDENTITY_CAUSAL_DIAGNOSIS".to_string(),
            "COMMON_TYPED_COMPOSITION_KERNEL".to_string(),
        ],
    })
}

fn predicate_refinement_receipt_hash(
    receipt: &PredicateRefinementLoweringReceiptIR,
) -> Result<String, CausalFrontendFailure> {
    let mut canonical = receipt.clone();
    canonical.receipt_sha256.clear();
    serde_json::to_vec(&canonical)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| {
            CausalFrontendFailure::public(format!("PREDICATE_REFINEMENT_RECEIPT:{error}"))
        })
}

/// Replays the lowering receipt without trusting its success booleans. The
/// canonical verifier can therefore distinguish a typed operation that was
/// actually materialized from a diagnostic label that merely claims repair.
pub fn validate_predicate_refinement_lowering_receipt(
    receipt: &PredicateRefinementLoweringReceiptIR,
    predecessor_source: &str,
) -> Result<(), CausalFrontendFailure> {
    if receipt.schema != PREDICATE_REFINEMENT_LOWERING_RECEIPT_SCHEMA
        || !receipt.receiver_root_relation_preserved
        || !receipt.owner_root_relation_preserved
        || !receipt.original_identity_predicate_replaced
    {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_RECEIPT_CONTRACT",
        ));
    }
    if receipt.receipt_sha256 != predicate_refinement_receipt_hash(receipt)? {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_RECEIPT_HASH",
        ));
    }
    if sha256(predecessor_source.as_bytes()) != receipt.materialized_patch.predecessor_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_PREDECESSOR_HASH",
        ));
    }
    let canonical_template = lower_typed_mechanism_goal(&receipt.typed_goal).map_err(|error| {
        CausalFrontendFailure::unsupported(format!(
            "PREDICATE_REFINEMENT_RECEIPT_TYPED_REPLAY:{error}"
        ))
    })?;
    if canonical_template != receipt.concrete_template {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_TEMPLATE_REPLAY",
        ));
    }
    let replay =
        apply_edit_atom(predecessor_source, &receipt.materialized_patch.edit).map_err(|error| {
            CausalFrontendFailure::conflict(format!(
                "PREDICATE_REFINEMENT_RECEIPT_EDIT_REPLAY:{error}"
            ))
        })?;
    if replay != receipt.materialized_patch.candidate_source
        || sha256(replay.as_bytes()) != receipt.materialized_patch.candidate_sha256
        || receipt.materialized_patch.candidate_replay_sha256
            != receipt.materialized_patch.candidate_sha256
        || !receipt
            .materialized_patch
            .candidate_materialization_is_one_to_one
    {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_POSTIMAGE_REPLAY",
        ));
    }
    match (
        receipt.language_backend,
        &receipt.rust_structural_repair_program,
    ) {
        (SourceLanguageBackend::RustSyn, Some(program)) => {
            if apply_edit_atom(predecessor_source, &program.edit).as_deref() != Ok(replay.as_str())
                || program.predecessor_source_sha256
                    != receipt.materialized_patch.predecessor_sha256
                || program.target_source_sha256 != receipt.materialized_patch.candidate_sha256
            {
                return Err(CausalFrontendFailure::conflict(
                    "PREDICATE_REFINEMENT_STRUCTURAL_PROGRAM_REPLAY",
                ));
            }
        }
        (SourceLanguageBackend::PythonAst, None) => {}
        _ => {
            return Err(CausalFrontendFailure::conflict(
                "PREDICATE_REFINEMENT_BACKEND_PROGRAM_SHAPE",
            ))
        }
    }
    Ok(())
}

/// Lowers a diagnosed call-identity underconstraint through the existing
/// typed mechanism compiler, then installs the emitted predicate into the
/// exact predecessor range using the common source edit algebra. This is a
/// compiler entry point only: compile/tests/verifier still own acceptance.
pub fn lower_call_identity_predicate_refinement(
    request: &CallIdentityPredicateRefinementIR,
) -> Result<PredicateRefinementLoweringReceiptIR, CausalFrontendFailure> {
    if request.schema != CALL_IDENTITY_PREDICATE_REFINEMENT_SCHEMA {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_SCHEMA",
        ));
    }
    if request.source.len() > MAX_SOURCE_BYTES {
        return Err(CausalFrontendFailure::public(
            "PREDICATE_REFINEMENT_SOURCE_BUDGET",
        ));
    }
    if request
        .source_relative_path
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_TEST_TARGET_BLOCKED",
        ));
    }
    let original_predicate = request
        .source
        .get(request.predicate_range.start..request.predicate_range.end)
        .ok_or_else(|| CausalFrontendFailure::conflict("PREDICATE_REFINEMENT_RANGE"))?;
    let backend = language_backend_for_path(&request.source_relative_path)?;
    let goal = predicate_refinement_goal(request)?;
    let concrete_template = lower_typed_mechanism_goal(&goal).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("PREDICATE_REFINEMENT_TYPED_LOWERING:{error}"))
    })?;
    let sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.source.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_types = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let replacement = match backend {
        SourceLanguageBackend::RustSyn => concrete_template.complete_expression_source.clone(),
        SourceLanguageBackend::PythonAst => {
            python_expression(&goal.postimage, &sources, &operand_types)?
        }
    };
    let edit = replacement_edit(&request.source, request.predicate_range, replacement)?;
    let candidate_source = apply_edit_atom(&request.source, &edit).map_err(|error| {
        CausalFrontendFailure::conflict(format!("PREDICATE_REFINEMENT_EDIT_REPLAY:{error}"))
    })?;
    let projection_sources = goal
        .operands
        .iter()
        .map(|operand| operand.source.clone())
        .collect::<Vec<_>>();
    let shapes = match backend {
        SourceLanguageBackend::RustSyn => {
            validate_rust_identity_predicate(original_predicate, &request.identity)?;
            syn::parse_file(&candidate_source).map_err(|error| {
                CausalFrontendFailure::unsupported(format!("RUST_REFINED_CANDIDATE_PARSE:{error}"))
            })?;
            projection_sources
                .iter()
                .map(|source| rust_projection_shape(source))
                .collect::<Result<Vec<_>, _>>()?
        }
        SourceLanguageBackend::PythonAst => {
            let executable = request.python_executable.as_deref().ok_or_else(|| {
                CausalFrontendFailure::public("PYTHON_EXECUTABLE_REQUIRED_FOR_REFINEMENT")
            })?;
            python_refinement_shapes(
                executable,
                original_predicate,
                &projection_sources,
                &candidate_source,
            )?
        }
    };
    validate_projection_relation_contract(&shapes)?;
    let replay = apply_edit_atom(&request.source, &edit).map_err(|error| {
        CausalFrontendFailure::conflict(format!("PREDICATE_REFINEMENT_SECOND_REPLAY:{error}"))
    })?;
    if replay != candidate_source {
        return Err(CausalFrontendFailure::conflict(
            "PREDICATE_REFINEMENT_NOT_ONE_TO_ONE",
        ));
    }
    let predecessor_sha256 = sha256(request.source.as_bytes());
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let materialized_patch = MaterializedSourceBoundPatchIR {
        predecessor_sha256,
        edit: edit.clone(),
        candidate_source: candidate_source.clone(),
        candidate_sha256,
        candidate_replay_sha256: sha256(replay.as_bytes()),
        candidate_materialization_is_one_to_one: true,
    };
    let rust_structural_repair_program = if backend == SourceLanguageBackend::RustSyn {
        let mut program = synthesize_structural_repair(
            &request
                .source_relative_path
                .to_string_lossy()
                .replace('\\', "/"),
            &request.source,
            &candidate_source,
        )
        .map_err(|error| {
            CausalFrontendFailure::conflict(format!("PREDICATE_REFINEMENT_PROGRAM:{error}"))
        })?;
        program.edit = edit;
        Some(program)
    } else {
        None
    };
    let mut receipt = PredicateRefinementLoweringReceiptIR {
        schema: PREDICATE_REFINEMENT_LOWERING_RECEIPT_SCHEMA.to_string(),
        language_backend: backend,
        typed_goal: goal,
        concrete_template,
        materialized_patch,
        rust_structural_repair_program,
        receiver_root_relation_preserved: true,
        owner_root_relation_preserved: true,
        original_identity_predicate_replaced: true,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = predicate_refinement_receipt_hash(&receipt)?;
    validate_predicate_refinement_lowering_receipt(&receipt, &request.source)?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundAlternativeReceiptIR {
    pub alternative_id: String,
    pub requested_public_symbol: String,
    pub function_template: SourceBoundFunctionTemplateIR,
    pub synthesis: TypedMechanismSynthesisReceiptIR,
    pub replayable_patch: ReplayableSourceBoundPatchIR,
    #[serde(default)]
    pub closure_candidates: Vec<SourceBoundClosureCandidateReceiptIR>,
    #[serde(default)]
    pub closure_candidate_rejections: Vec<SourceBoundClosureCandidateRejectionIR>,
    #[serde(default)]
    pub candidate_validation_processes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceBoundDeclarationOperation {
    Insert,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundDeclarationTemplateIR {
    pub qualified_owner: String,
    pub attribute: String,
    pub value_source: String,
    pub operation: SourceBoundDeclarationOperation,
    pub edit_range: ByteRange,
    pub edit_source: String,
    pub public_evidence_sha256: String,
    pub source_template_sha256: String,
}

fn source_bound_declaration_template_hash(
    source: &str,
    template: &SourceBoundDeclarationTemplateIR,
) -> Result<String, CausalFrontendFailure> {
    serde_json::to_vec(&(
        sha256(source.as_bytes()),
        &template.qualified_owner,
        &template.attribute,
        &template.value_source,
        &template.operation,
        template.edit_range,
        &template.edit_source,
        &template.public_evidence_sha256,
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|error| CausalFrontendFailure::public(format!("DECLARATION_TEMPLATE_HASH:{error}")))
}

fn validate_source_bound_declaration_template(
    source: &str,
    template: &SourceBoundDeclarationTemplateIR,
) -> Result<(), CausalFrontendFailure> {
    let mut attribute_characters = template.attribute.chars();
    let attribute_is_identifier = attribute_characters
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic())
        && attribute_characters.all(|character| character == '_' || character.is_alphanumeric());
    let expected_statement = format!("{} = {}", template.attribute, template.value_source);
    let edit_binding_valid = match template.operation {
        SourceBoundDeclarationOperation::Insert => {
            template.edit_range.start == template.edit_range.end
                && template.edit_source.trim() == expected_statement
        }
        SourceBoundDeclarationOperation::Replace => {
            template.edit_range.start < template.edit_range.end
                && template.edit_source == template.value_source
        }
    };
    if template.qualified_owner.is_empty()
        || template.qualified_owner.split('.').any(str::is_empty)
        || !attribute_is_identifier
        || template.value_source.is_empty()
        || !edit_binding_valid
        || template.edit_range.end > source.len()
        || !source.is_char_boundary(template.edit_range.start)
        || !source.is_char_boundary(template.edit_range.end)
        || template.public_evidence_sha256.len() != 64
    {
        return Err(CausalFrontendFailure::conflict(
            "DECLARATION_TEMPLATE_STRUCTURAL_BINDING",
        ));
    }
    if template.source_template_sha256 != source_bound_declaration_template_hash(source, template)?
    {
        return Err(CausalFrontendFailure::conflict(
            "DECLARATION_TEMPLATE_HASH_BINDING",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundDeclarationAlternativeReceiptIR {
    pub alternative_id: String,
    pub requested_public_symbol: String,
    pub declaration_template: SourceBoundDeclarationTemplateIR,
    pub replayable_patch: ReplayableSourceBoundPatchIR,
    pub candidate_validation_processes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundClosureCandidateReceiptIR {
    pub closure_ordinal: usize,
    pub public_operand_bindings: BTreeMap<String, String>,
    pub function_template: SourceBoundFunctionTemplateIR,
    pub synthesis: TypedMechanismSynthesisReceiptIR,
    pub replayable_patch: ReplayableSourceBoundPatchIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundClosureCandidateRejectionIR {
    pub closure_ordinal: usize,
    pub qualified_symbol: String,
    pub failure_kind: CausalFrontendFailureKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundPatchVariantIR {
    pub variant_id: String,
    /// Zero selects the public-symbol template; N selects closure candidate
    /// N-1 for the corresponding causal alternative.
    pub selected_candidate_indices: Vec<usize>,
    pub selected_template_symbols: Vec<String>,
    pub replayable_patch: ReplayableSourceBoundPatchIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundCausalReceiptIR {
    pub schema: String,
    pub source_relative_path: PathBuf,
    pub language_backend: SourceLanguageBackend,
    pub predecessor_sha256: String,
    pub alternatives: Vec<SourceBoundAlternativeReceiptIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declaration_alternatives: Vec<SourceBoundDeclarationAlternativeReceiptIR>,
    #[serde(default)]
    pub patch_variants: Vec<SourceBoundPatchVariantIR>,
    #[serde(default)]
    pub alternative_worker_count: usize,
    pub public_symbol_owner_preserved: bool,
    pub execution_dependency_closure_preserved: bool,
    pub single_and_multi_edit_share_atomic_path: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Deserialize)]
struct PythonHostResponse {
    ok: bool,
    #[serde(default)]
    failure: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    definitions: Vec<PythonFunctionDefinition>,
}

#[derive(Debug, Deserialize)]
struct PythonObservationDiscoveryResponse {
    ok: bool,
    #[serde(default)]
    failure: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    alternatives: Vec<PythonDiscoveredAlternative>,
    #[serde(default)]
    declarations: Vec<PythonDiscoveredDeclaration>,
}

#[derive(Debug, Deserialize)]
struct PythonDiscoveredDeclaration {
    qualified_owner: String,
    attribute: String,
    value_source: String,
    operation: SourceBoundDeclarationOperation,
    edit_start: usize,
    edit_end: usize,
    edit_source: String,
    evidence: Vec<String>,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct PythonCandidateBatchResponse {
    ok: bool,
    #[serde(default)]
    failure: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    results: Vec<PythonCandidateValidationResult>,
}

#[derive(Debug, Deserialize)]
struct PythonCandidateValidationResult {
    ok: bool,
    #[serde(default)]
    failure: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PythonDiscoveredAlternative {
    qualified_symbol: String,
    reason: String,
    observations: Vec<TypedMechanismObservationIR>,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonFunctionDefinition {
    qualified_symbol: String,
    owner: String,
    is_async: bool,
    operands: Vec<PythonOperand>,
    return_annotation: String,
    effects: Vec<String>,
    direct_dependencies: Vec<String>,
    execution_dependency_closure: Vec<String>,
    #[serde(default)]
    external_callers: Vec<String>,
    cuts: Vec<PythonCut>,
    #[serde(default)]
    closure_templates: Vec<PythonClosureTemplateDefinition>,
    #[serde(default)]
    closure_rejections: Vec<PythonClosureTemplateRejection>,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonClosureTemplateDefinition {
    qualified_symbol: String,
    owner: String,
    is_async: bool,
    operands: Vec<PythonOperand>,
    return_annotation: String,
    effects: Vec<String>,
    direct_dependencies: Vec<String>,
    execution_dependency_closure: Vec<String>,
    external_callers: Vec<String>,
    cuts: Vec<PythonCut>,
    public_operand_bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonClosureTemplateRejection {
    qualified_symbol: String,
    failure: String,
    detail: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonOperand {
    name: String,
    annotation: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonCut {
    branch: String,
    condition_source: Option<String>,
    condition_start: Option<usize>,
    condition_end: Option<usize>,
    #[serde(default)]
    condition_template: Option<TypedSyntaxExpressionIR>,
    postimage_source: String,
    postimage_start: usize,
    postimage_end: usize,
    #[serde(default)]
    postimage_template: Option<TypedSyntaxExpressionIR>,
}

const PYTHON_AST_HOST: &str = r#"
import ast, json, sys

def emit_failure(kind, detail):
    json.dump({"ok": False, "failure": kind, "detail": detail}, sys.stdout, ensure_ascii=False)
    raise SystemExit(0)

request = json.load(sys.stdin)
source = request.get("source")
symbols = request.get("symbols")
max_closure = int(request.get("max_closure", 64))
if not isinstance(source, str) or not isinstance(symbols, list) or not symbols:
    emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "PYTHON_HOST_INPUT")
try:
    tree = ast.parse(source, filename="<b-core-source-bound>", type_comments=True)
    # Compile the AST to bytecode without executing it.  `ast.parse` alone
    # does not exercise every context validation performed by the compiler.
    compile(tree, "<b-core-source-bound>", "exec")
except (SyntaxError, ValueError, TypeError) as error:
    emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_PARSE:" + str(error))

line_starts = [0]
for line in source.splitlines(keepends=True):
    line_starts.append(line_starts[-1] + len(line.encode("utf-8")))
source_bytes = source.encode("utf-8")

def byte_offset(node, end=False):
    line = getattr(node, "end_lineno" if end else "lineno", None)
    column = getattr(node, "end_col_offset" if end else "col_offset", None)
    if line is None or column is None or line < 1 or line >= len(line_starts):
        emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_AST_SPAN_MISSING")
    return line_starts[line - 1] + column

def source_segment(node):
    start, end = byte_offset(node), byte_offset(node, True)
    try:
        return source_bytes[start:end].decode("utf-8")
    except UnicodeDecodeError:
        emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_AST_UTF8_SPAN")

def annotation(node):
    if node is None:
        return ""
    try:
        return ast.unparse(node)
    except Exception as error:
        emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_ANNOTATION:" + str(error))

definitions = {}

def register_function(node, prefix):
    qualified = ".".join(prefix + [node.name])
    owner = ".".join(prefix)
    parameters = []
    all_positional = list(node.args.posonlyargs) + list(node.args.args)
    for argument in all_positional + list(node.args.kwonlyargs):
        if argument.arg in ("self", "cls"):
            continue
        parameters.append({"name": argument.arg, "annotation": annotation(argument.annotation)})
    unsupported_reasons = []
    if node.args.vararg is not None or node.args.kwarg is not None:
        unsupported_reasons.append("PYTHON_VARIADIC_PUBLIC_SYMBOL:" + qualified)
    unsupported = (ast.Yield, ast.YieldFrom, ast.NamedExpr)
    if any(isinstance(item, unsupported) for item in owned_walk(node)):
        unsupported_reasons.append("PYTHON_UNSUPPORTED_NODE:" + qualified)
    effects = set()
    for item in owned_walk(node):
        if isinstance(item, ast.Await): effects.add("AWAIT")
        if isinstance(item, ast.Raise): effects.add("RAISE")
        if isinstance(item, (ast.Assign, ast.AnnAssign, ast.AugAssign, ast.Delete)): effects.add("LOCAL_MUTATION")
        if isinstance(item, (ast.For, ast.AsyncFor, ast.While)): effects.add("LOOP")
        try_nodes = (ast.Try,) + ((ast.TryStar,) if hasattr(ast, "TryStar") else ())
        if isinstance(item, try_nodes): effects.add("EXCEPTION_FLOW")
        if isinstance(item, (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)): effects.add("COMPREHENSION")
    definitions[qualified] = {
        "node": node, "qualified_symbol": qualified, "owner": owner,
        "is_async": isinstance(node, ast.AsyncFunctionDef), "operands": parameters,
        "return_annotation": annotation(node.returns), "effects": sorted(effects),
        "unsupported_reasons": unsupported_reasons,
    }

def owned_walk(node):
    # Calls/effects inside a nested definition belong to that definition, not
    # to its lexical parent.  `ast.walk(function)` silently merges the two and
    # invents dependency edges that never execute when the parent is called.
    pending = list(ast.iter_child_nodes(node))
    while pending:
        current = pending.pop()
        if isinstance(current, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Lambda)):
            continue
        yield current
        pending.extend(ast.iter_child_nodes(current))

def collect(items, prefix):
    for item in items:
        if isinstance(item, ast.ClassDef):
            collect(item.body, prefix + [item.name])
        elif isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
            register_function(item, prefix)
            nested = [child for child in item.body if isinstance(child, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))]
            collect(nested, prefix + [item.name])

collect(tree.body, [])

def dotted_name(node):
    if isinstance(node, ast.Name): return node.id
    if isinstance(node, ast.Attribute):
        base = dotted_name(node.value)
        return (base + "." if base else "") + node.attr
    return ""

short_index = {}
for qualified in definitions:
    short_index.setdefault(qualified.rsplit(".", 1)[-1], []).append(qualified)

def resolve_call(caller, call):
    raw = dotted_name(call.func)
    if not raw: return None
    owner = definitions[caller]["owner"]
    if raw.startswith("self.") or raw.startswith("cls."):
        candidate = owner + "." + raw.split(".", 1)[1] if owner else raw.split(".", 1)[1]
        return candidate if candidate in definitions else None
    if raw in definitions: return raw
    candidate = owner + "." + raw if owner else raw
    if candidate in definitions: return candidate
    matches = short_index.get(raw, [])
    return matches[0] if len(matches) == 1 else None

for qualified, definition in definitions.items():
    dependencies = set()
    dependency_bindings = {}
    ambiguous_bindings = set()
    caller_operands = {operand["name"] for operand in definition["operands"]}
    for item in owned_walk(definition["node"]):
        if isinstance(item, ast.Call):
            resolved = resolve_call(qualified, item)
            if resolved and resolved != qualified:
                dependencies.add(resolved)
                callee_operands = [operand["name"] for operand in definitions[resolved]["operands"]]
                bound = {}
                valid = not any(isinstance(argument, ast.Starred) for argument in item.args)
                if valid and len(item.args) <= len(callee_operands):
                    for operand, argument in zip(callee_operands, item.args):
                        if isinstance(argument, ast.Name) and argument.id in caller_operands:
                            bound[operand] = argument.id
                        else:
                            valid = False
                    for keyword in item.keywords:
                        if (keyword.arg not in callee_operands or keyword.arg in bound
                                or not isinstance(keyword.value, ast.Name)
                                or keyword.value.id not in caller_operands):
                            valid = False
                            break
                        bound[keyword.arg] = keyword.value.id
                else:
                    valid = False
                if not valid or set(bound) != set(callee_operands):
                    ambiguous_bindings.add(resolved)
                elif resolved in dependency_bindings and dependency_bindings[resolved] != bound:
                    ambiguous_bindings.add(resolved)
                else:
                    dependency_bindings[resolved] = bound
    definition["direct_dependencies"] = sorted(dependencies)
    definition["dependency_bindings"] = dependency_bindings
    definition["ambiguous_bindings"] = ambiguous_bindings

reverse_callers = {qualified: set() for qualified in definitions}
for caller, definition in definitions.items():
    for dependency in definition["direct_dependencies"]:
        reverse_callers[dependency].add(caller)

def typed_template(node, operands, depth=0):
    # This is a bounded syntax frontend, not an evaluator. Unsupported source
    # expressions simply provide no seed and never weaken normal synthesis.
    if node is None or depth > 32:
        return None
    if isinstance(node, ast.Name) and node.id in operands:
        return {"syntax_kind": "OPERAND", "role": node.id}
    if isinstance(node, ast.Constant):
        if isinstance(node.value, bool):
            return {"syntax_kind": "BOOL_LITERAL", "value": node.value}
        if isinstance(node.value, int) and -(2**63) <= node.value < 2**63:
            return {"syntax_kind": "INT_LITERAL", "value": node.value}
        return None
    if isinstance(node, ast.UnaryOp):
        operators = {ast.USub: "NEGATE", ast.Not: "NOT"}
        operator = operators.get(type(node.op))
        input_ = typed_template(node.operand, operands, depth + 1)
        if operator and input_:
            return {"syntax_kind": "UNARY", "operator": operator, "input": input_}
        return None
    if isinstance(node, ast.BinOp):
        operators = {
            ast.Add: "ADD", ast.Sub: "SUBTRACT", ast.Mult: "MULTIPLY",
        }
        operator = operators.get(type(node.op))
        left = typed_template(node.left, operands, depth + 1)
        right = typed_template(node.right, operands, depth + 1)
        if operator and left and right:
            return {"syntax_kind": "BINARY", "operator": operator, "left": left, "right": right}
        return None
    if isinstance(node, ast.BoolOp) and len(node.values) >= 2:
        operator = "AND" if isinstance(node.op, ast.And) else "OR" if isinstance(node.op, ast.Or) else None
        values = [typed_template(value, operands, depth + 1) for value in node.values]
        if operator and all(values):
            expression = values[0]
            for value in values[1:]:
                expression = {"syntax_kind": "BINARY", "operator": operator, "left": expression, "right": value}
            return expression
        return None
    if isinstance(node, ast.Compare) and len(node.ops) == 1 and len(node.comparators) == 1:
        operators = {
            ast.Eq: "EQUAL", ast.NotEq: "NOT_EQUAL", ast.Lt: "LESS_THAN",
            ast.LtE: "LESS_THAN_OR_EQUAL", ast.Gt: "GREATER_THAN",
            ast.GtE: "GREATER_THAN_OR_EQUAL",
        }
        operator = operators.get(type(node.ops[0]))
        left = typed_template(node.left, operands, depth + 1)
        right = typed_template(node.comparators[0], operands, depth + 1)
        if operator and left and right:
            return {"syntax_kind": "BINARY", "operator": operator, "left": left, "right": right}
        return None
    if isinstance(node, ast.Call) and not node.keywords:
        if isinstance(node.func, ast.Name) and node.func.id == "len" and len(node.args) == 1:
            input_ = typed_template(node.args[0], operands, depth + 1)
            return {"syntax_kind": "LENGTH", "input": input_} if input_ else None
        if isinstance(node.func, ast.Attribute) and not node.args:
            operators = {"strip": "TRIM", "lower": "LOWERCASE", "upper": "UPPERCASE"}
            operator = operators.get(node.func.attr)
            input_ = typed_template(node.func.value, operands, depth + 1)
            if operator and input_:
                return {"syntax_kind": "STRING_TRANSFORM", "operator": operator, "input": input_}
        return None
    if isinstance(node, ast.Subscript):
        collection = typed_template(node.value, operands, depth + 1)
        index = typed_template(node.slice, operands, depth + 1)
        if collection and index:
            return {"syntax_kind": "INDEX", "collection": collection, "index": index}
    return None

def cuts_for(definition):
    cuts = []
    operand_names = {operand["name"] for operand in definition["operands"]}
    def visit_statements(statements, guard=None, branch="UNCONDITIONAL"):
        for statement in statements:
            if isinstance(statement, ast.Return) and statement.value is not None:
                cuts.append({
                    "branch": branch,
                    "condition_source": source_segment(guard) if guard is not None else None,
                    "condition_start": byte_offset(guard) if guard is not None else None,
                    "condition_end": byte_offset(guard, True) if guard is not None else None,
                    "condition_template": typed_template(guard, operand_names),
                    "postimage_source": source_segment(statement.value),
                    "postimage_start": byte_offset(statement.value),
                    "postimage_end": byte_offset(statement.value, True),
                    "postimage_template": typed_template(statement.value, operand_names),
                })
            elif isinstance(statement, ast.If):
                visit_statements(statement.body, statement.test, "THEN")
                visit_statements(statement.orelse, statement.test, "ELSE")
            elif isinstance(statement, (ast.For, ast.AsyncFor, ast.While)):
                visit_statements(statement.body, guard, branch)
                visit_statements(statement.orelse, guard, branch)
            elif isinstance(statement, (ast.With, ast.AsyncWith)):
                visit_statements(statement.body, guard, branch)
            elif isinstance(statement, (ast.Try,) + ((ast.TryStar,) if hasattr(ast, "TryStar") else ())):
                visit_statements(statement.body, guard, branch)
                for handler in statement.handlers: visit_statements(handler.body, guard, branch)
                visit_statements(statement.orelse, guard, branch)
                visit_statements(statement.finalbody, guard, branch)
    visit_statements(definition["node"].body)
    return cuts

def dependency_closure(start, requested):
    closure, pending, seen = [], [start], set()
    while pending:
        current = pending.pop(0)
        if current in seen: continue
        seen.add(current); closure.append(current)
        if len(closure) > max_closure:
            emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "DEPENDENCY_CLOSURE_BUDGET:" + requested)
        unsupported_reasons = definitions[current]["unsupported_reasons"]
        if unsupported_reasons:
            emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", unsupported_reasons[0])
        pending.extend(dependency for dependency in definitions[current]["direct_dependencies"] if dependency not in seen)
    return closure

selected = []
for requested in symbols:
    if requested not in definitions:
        emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "EXACT_PUBLIC_SYMBOL_NOT_FOUND:" + str(requested))
    closure = dependency_closure(requested, requested)
    definition = definitions[requested]
    cuts = cuts_for(definition)
    if not cuts:
        emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "PUBLIC_SYMBOL_POSTIMAGE_MISSING:" + requested)
    initial_bindings = {operand["name"]: operand["name"] for operand in definition["operands"]}
    binding_states = {requested: {tuple(sorted(initial_bindings.items()))}}
    binding_pending = [(requested, initial_bindings)]
    while binding_pending:
        current, current_bindings = binding_pending.pop(0)
        for dependency in definitions[current]["direct_dependencies"]:
            if dependency in definitions[current]["ambiguous_bindings"]:
                continue
            local_bindings = definitions[current]["dependency_bindings"].get(dependency)
            if local_bindings is None:
                continue
            propagated = {}
            valid = True
            for dependency_operand, current_operand in local_bindings.items():
                public_operand = current_bindings.get(current_operand)
                if public_operand is None:
                    valid = False
                    break
                propagated[dependency_operand] = public_operand
            if not valid:
                continue
            signature = tuple(sorted(propagated.items()))
            states = binding_states.setdefault(dependency, set())
            if signature not in states:
                states.add(signature)
                if sum(len(values) for values in binding_states.values()) > max_closure * max_closure:
                    emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "DEPENDENCY_BINDING_STATE_BUDGET:" + requested)
                binding_pending.append((dependency, propagated))
    public_bindings = {
        symbol: dict(next(iter(states)))
        for symbol, states in binding_states.items()
        if len(states) == 1
    }
    closure_templates = []
    closure_rejections = []
    for closure_ordinal, closure_symbol in enumerate(closure[1:], start=1):
        closure_definition = definitions[closure_symbol]
        closure_bindings = public_bindings.get(closure_symbol)
        closure_cuts = cuts_for(closure_definition)
        if closure_bindings is None:
            closure_rejections.append({
                "qualified_symbol": closure_symbol,
                "failure": "PUBLIC_INFORMATION_INSUFFICIENT",
                "detail": "DEPENDENCY_OPERAND_BINDING_AMBIGUOUS:" + closure_symbol,
            })
            continue
        if not closure_cuts:
            closure_rejections.append({
                "qualified_symbol": closure_symbol,
                "failure": "PUBLIC_INFORMATION_INSUFFICIENT",
                "detail": "DEPENDENCY_POSTIMAGE_MISSING:" + closure_symbol,
            })
            continue
        closure_templates.append({
            "qualified_symbol": closure_symbol, "owner": closure_definition["owner"],
            "is_async": closure_definition["is_async"], "operands": closure_definition["operands"],
            "return_annotation": closure_definition["return_annotation"], "effects": closure_definition["effects"],
            "direct_dependencies": closure_definition["direct_dependencies"],
            "execution_dependency_closure": dependency_closure(closure_symbol, requested), "cuts": closure_cuts,
            "external_callers": sorted(reverse_callers[closure_symbol] - set(closure)),
            "public_operand_bindings": closure_bindings,
        })
    selected.append({
        "qualified_symbol": requested, "owner": definition["owner"],
        "is_async": definition["is_async"], "operands": definition["operands"],
        "return_annotation": definition["return_annotation"], "effects": definition["effects"],
        "direct_dependencies": definition["direct_dependencies"],
        "execution_dependency_closure": closure, "cuts": cuts,
        "external_callers": sorted(reverse_callers[requested] - set(closure)),
        "closure_templates": closure_templates,
        "closure_rejections": closure_rejections,
    })

json.dump({"ok": True, "definitions": selected}, sys.stdout, ensure_ascii=False)
"#;

const PYTHON_CANDIDATE_BATCH_HOST: &str = r#"
import ast, json, sys

def fail(kind, detail):
    json.dump({"ok": False, "failure": kind, "detail": detail}, sys.stdout, ensure_ascii=False)
    raise SystemExit(0)

request = json.load(sys.stdin)
candidates = request.get("candidates")
if not isinstance(candidates, list) or not candidates:
    fail("PUBLIC_INFORMATION_INSUFFICIENT", "CANDIDATE_BATCH_INPUT")

def qualified_functions(tree):
    found = set()
    def collect(items, prefix):
        for item in items:
            if isinstance(item, ast.ClassDef):
                collect(item.body, prefix + [item.name])
            elif isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                qualified = ".".join(prefix + [item.name])
                found.add(qualified)
                nested = [child for child in item.body if isinstance(child, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))]
                collect(nested, prefix + [item.name])
    collect(tree.body, [])
    return found

def qualified_class_attributes(tree):
    found = {}
    def collect(items, prefix):
        for item in items:
            if not isinstance(item, ast.ClassDef):
                continue
            qualified = ".".join(prefix + [item.name])
            for statement in item.body:
                if isinstance(statement, ast.Assign):
                    for target in statement.targets:
                        if isinstance(target, ast.Name): found[(qualified, target.id)] = statement.value
                elif isinstance(statement, ast.AnnAssign) and isinstance(statement.target, ast.Name) and statement.value is not None:
                    found[(qualified, statement.target.id)] = statement.value
            collect(item.body, prefix + [item.name])
    collect(tree.body, [])
    return found

results = []
for ordinal, candidate in enumerate(candidates):
    source = candidate.get("source") if isinstance(candidate, dict) else None
    public_symbol = candidate.get("public_symbol") if isinstance(candidate, dict) else None
    declaration_owner = candidate.get("declaration_owner") if isinstance(candidate, dict) else None
    declaration_attribute = candidate.get("declaration_attribute") if isinstance(candidate, dict) else None
    declaration_value_source = candidate.get("declaration_value_source") if isinstance(candidate, dict) else None
    function_binding = isinstance(public_symbol, str) and bool(public_symbol)
    declaration_binding = isinstance(declaration_owner, str) and bool(declaration_owner) and isinstance(declaration_attribute, str) and bool(declaration_attribute) and isinstance(declaration_value_source, str) and bool(declaration_value_source)
    if not isinstance(source, str) or function_binding == declaration_binding:
        results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "CANDIDATE_INPUT:" + str(ordinal)})
        continue
    try:
        tree = ast.parse(source, filename="<b-core-candidate-" + str(ordinal) + ">", type_comments=True)
        compile(tree, "<b-core-candidate-" + str(ordinal) + ">", "exec")
    except (SyntaxError, ValueError, TypeError) as error:
        results.append({"ok": False, "failure": "UNSUPPORTED_LANGUAGE_SYNTAX", "detail": "CANDIDATE_PARSE:" + str(error)})
        continue
    if function_binding and public_symbol not in qualified_functions(tree):
        results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "MATERIALIZED_PUBLIC_SYMBOL_IDENTITY_LOST:" + public_symbol})
        continue
    if declaration_binding:
        declarations = qualified_class_attributes(tree)
        key = (declaration_owner, declaration_attribute)
        if key not in declarations:
            results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "MATERIALIZED_CLASS_DECLARATION_IDENTITY_LOST:" + declaration_owner + "." + declaration_attribute})
            continue
        try:
            observed = ast.literal_eval(declarations[key])
            expected = ast.literal_eval(declaration_value_source)
        except (ValueError, TypeError, SyntaxError) as error:
            results.append({"ok": False, "failure": "UNSUPPORTED_LANGUAGE_SYNTAX", "detail": "MATERIALIZED_CLASS_DECLARATION_LITERAL:" + str(error)})
            continue
        if type(observed) is not type(expected) or observed != expected:
            results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "MATERIALIZED_CLASS_DECLARATION_POSTIMAGE_MISMATCH:" + declaration_owner + "." + declaration_attribute})
            continue
    results.append({"ok": True})

json.dump({"ok": True, "results": results}, sys.stdout, ensure_ascii=False)
"#;

const PYTHON_PUBLIC_OBSERVATION_HOST: &str = r#"
import ast, json, sys

def fail(kind, detail):
    json.dump({"ok": False, "failure": kind, "detail": detail}, sys.stdout, ensure_ascii=False)
    raise SystemExit(0)

request = json.load(sys.stdin)
source = request.get("source")
tests = request.get("tests")
targets = set(request.get("target_symbols") or [])
if not isinstance(source, str) or not isinstance(tests, list) or not tests:
    fail("PUBLIC_INFORMATION_INSUFFICIENT", "REPOSITORY_DISCOVERY_INPUT")
try:
    source_tree = ast.parse(source, filename="<b-core-implementation>", type_comments=True)
    compile(source_tree, "<b-core-implementation>", "exec")
except (SyntaxError, ValueError, TypeError) as error:
    fail("UNSUPPORTED_LANGUAGE_SYNTAX", "IMPLEMENTATION_PARSE:" + str(error))

line_starts = [0]
for line in source.splitlines(keepends=True):
    line_starts.append(line_starts[-1] + len(line.encode("utf-8")))
source_bytes = source.encode("utf-8")
newline = "\r\n" if "\r\n" in source else "\n"
def byte_offset(node, end=False):
    line = getattr(node, "end_lineno" if end else "lineno", None)
    column = getattr(node, "end_col_offset" if end else "col_offset", None)
    if line is None or column is None or line < 1 or line >= len(line_starts):
        fail("UNSUPPORTED_LANGUAGE_SYNTAX", "DECLARATION_AST_SPAN_MISSING")
    return line_starts[line - 1] + column

definitions = {}
classes = {}
def collect(items, prefix):
    for item in items:
        if isinstance(item, ast.ClassDef):
            qualified = ".".join(prefix + [item.name])
            attributes = {}
            for statement in item.body:
                if isinstance(statement, ast.Assign):
                    for target in statement.targets:
                        if isinstance(target, ast.Name): attributes[target.id] = statement.value
                elif isinstance(statement, ast.AnnAssign) and isinstance(statement.target, ast.Name):
                    attributes[statement.target.id] = statement.value
            classes[qualified] = {"node": item, "attributes": attributes}
            collect(item.body, prefix + [item.name])
        elif isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
            qualified = ".".join(prefix + [item.name])
            positional = list(item.args.posonlyargs) + list(item.args.args)
            roles = [argument.arg for argument in positional if argument.arg not in ("self", "cls")]
            if item.args.vararg is None and item.args.kwarg is None and len(roles) == len(set(roles)):
                definitions[qualified] = {"node": item, "roles": roles}
collect(source_tree.body, [])

short_index = {}
for qualified in definitions:
    short_index.setdefault(qualified.rsplit(".", 1)[-1], []).append(qualified)
class_short_index = {}
for qualified in classes:
    class_short_index.setdefault(qualified.rsplit(".", 1)[-1], []).append(qualified)

def dotted(node):
    if isinstance(node, ast.Name): return node.id
    if isinstance(node, ast.Attribute):
        base = dotted(node.value)
        return (base + "." if base else "") + node.attr
    return ""

def resolve(call):
    raw = dotted(call.func)
    exact = [qualified for qualified in definitions if raw == qualified or raw.endswith("." + qualified)]
    if len(exact) == 1: return exact[0]
    matches = short_index.get(raw.rsplit(".", 1)[-1], [])
    return matches[0] if len(matches) == 1 else None

def resolve_class(raw):
    exact = [qualified for qualified in classes if raw == qualified or raw.endswith("." + qualified)]
    if len(exact) == 1: return exact[0]
    matches = class_short_index.get(raw.rsplit(".", 1)[-1], [])
    return matches[0] if len(matches) == 1 else None

def literal(node):
    if isinstance(node, ast.Constant) and isinstance(node.value, bool): return node.value
    if isinstance(node, ast.Constant) and isinstance(node.value, int): return node.value
    if isinstance(node, ast.Constant) and isinstance(node.value, str): return node.value
    if isinstance(node, ast.Constant) and isinstance(node.value, bytes): return node.value
    if isinstance(node, ast.List):
        values = [literal(element) for element in node.elts]
        return values if all(value is not None for value in values) else None
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
        value = literal(node.operand)
        return -value if isinstance(value, int) and not isinstance(value, bool) else None
    return None

def encoded(value):
    if isinstance(value, bool): return {"value_kind": "BOOL", "value": value}
    if isinstance(value, int): return {"value_kind": "INT", "value": value}
    if isinstance(value, str): return {"value_kind": "STRING", "value": value}
    if isinstance(value, bytes): return {"value_kind": "BYTES", "value": list(value)}
    if isinstance(value, list):
        if all(isinstance(item, int) and not isinstance(item, bool) for item in value):
            return {"value_kind": "SEQUENCE", "value": value}
        if all(isinstance(row, list) and all(isinstance(item, int) and not isinstance(item, bool) for item in row) for row in value):
            return {"value_kind": "NESTED_SEQUENCE", "value": value}
    return None

def call_observation(call, expected):
    qualified = resolve(call)
    if qualified is None: return None
    definition = definitions[qualified]
    roles = definition["roles"]
    if len(call.args) > len(roles) or any(keyword.arg is None for keyword in call.keywords): return None
    values = {}
    for role, argument in zip(roles, call.args):
        value = literal(argument)
        if encoded(value) is None: return None
        values[role] = value
    for keyword in call.keywords:
        if keyword.arg not in roles or keyword.arg in values: return None
        value = literal(keyword.value)
        if encoded(value) is None: return None
        values[keyword.arg] = value
    if set(values) != set(roles) or encoded(expected) is None: return None
    return qualified, values, expected

def declaration_observation(node, expected, test_local_classes):
    if not isinstance(node, ast.Attribute) or encoded(expected) is None: return None
    raw_owner = dotted(node.value)
    if raw_owner in test_local_classes or raw_owner.rsplit(".", 1)[-1] in test_local_classes: return None
    owner = resolve_class(raw_owner)
    if owner is None or not node.attr.isidentifier(): return None
    return owner, node.attr, expected

observations = {}
declaration_observations = {}
for test in tests:
    path, test_source = test.get("relative_path", "<test>"), test.get("source")
    if not isinstance(test_source, str):
        fail("PUBLIC_INFORMATION_INSUFFICIENT", "TEST_SOURCE_MISSING:" + str(path))
    try:
        tree = ast.parse(test_source, filename=str(path), type_comments=True)
        compile(tree, str(path), "exec")
    except (SyntaxError, ValueError, TypeError) as error:
        fail("UNSUPPORTED_LANGUAGE_SYNTAX", "TEST_PARSE:" + str(path) + ":" + str(error))
    test_local_classes = {node.name for node in ast.walk(tree) if isinstance(node, ast.ClassDef)}
    for node in ast.walk(tree):
        observation = None
        declaration = None
        if isinstance(node, ast.Assert):
            test_node = node.test
            if isinstance(test_node, ast.Compare) and len(test_node.ops) == 1 and isinstance(test_node.ops[0], ast.Eq) and len(test_node.comparators) == 1:
                left, right = test_node.left, test_node.comparators[0]
                if isinstance(left, ast.Call): observation = call_observation(left, literal(right))
                elif isinstance(right, ast.Call): observation = call_observation(right, literal(left))
                if observation is None:
                    declaration = declaration_observation(left, literal(right), test_local_classes)
                    if declaration is None: declaration = declaration_observation(right, literal(left), test_local_classes)
            elif isinstance(test_node, ast.Call):
                observation = call_observation(test_node, True)
            elif isinstance(test_node, ast.UnaryOp) and isinstance(test_node.op, ast.Not) and isinstance(test_node.operand, ast.Call):
                observation = call_observation(test_node.operand, False)
        elif isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr in ("assertEqual", "assertEquals") and len(node.args) == 2 and not node.keywords:
            declaration = declaration_observation(node.args[0], literal(node.args[1]), test_local_classes)
            if declaration is None: declaration = declaration_observation(node.args[1], literal(node.args[0]), test_local_classes)
        if observation is not None:
            qualified, values, expected = observation
            key = json.dumps([{role: encoded(value) for role, value in values.items()}, encoded(expected)], sort_keys=True, ensure_ascii=False)
            observations.setdefault(qualified, {})[key] = {"values": values, "expected": expected}
        if declaration is not None:
            owner, attribute, expected = declaration
            evidence = str(path) + ":" + str(getattr(node, "lineno", 0)) + ":" + repr(expected)
            declaration_observations.setdefault((owner, attribute), {}).setdefault(repr(expected), {"value": expected, "evidence": []})["evidence"].append(evidence)

UNKNOWN = object()
def safe_eval(node, environment):
    if isinstance(node, ast.Constant) and isinstance(node.value, (bool, int, str, bytes)): return node.value
    if isinstance(node, ast.List):
        values = [safe_eval(element, environment) for element in node.elts]
        return values if all(value is not UNKNOWN for value in values) else UNKNOWN
    if isinstance(node, ast.Name): return environment.get(node.id, UNKNOWN)
    if isinstance(node, ast.Call) and not node.args and not node.keywords and isinstance(node.func, ast.Attribute):
        value = safe_eval(node.func.value, environment)
        if not isinstance(value, str): return UNKNOWN
        if node.func.attr == "strip": return value.strip()
        if node.func.attr == "lower": return value.lower()
        if node.func.attr == "upper": return value.upper()
        return UNKNOWN
    if isinstance(node, ast.UnaryOp):
        value = safe_eval(node.operand, environment)
        if value is UNKNOWN: return UNKNOWN
        if isinstance(node.op, ast.USub) and isinstance(value, int): return -value
        if isinstance(node.op, ast.Not): return not value
        return UNKNOWN
    if isinstance(node, ast.BinOp):
        left, right = safe_eval(node.left, environment), safe_eval(node.right, environment)
        if left is UNKNOWN or right is UNKNOWN: return UNKNOWN
        try:
            if isinstance(node.op, ast.Add): return left + right
            if isinstance(node.op, ast.Sub): return left - right
            if isinstance(node.op, ast.Mult): return left * right
            if isinstance(node.op, ast.FloorDiv) and right != 0: return left // right
            if isinstance(node.op, ast.Mod) and right != 0: return left % right
        except (ArithmeticError, TypeError): return UNKNOWN
        return UNKNOWN
    if isinstance(node, ast.Compare) and len(node.ops) == 1 and len(node.comparators) == 1:
        left, right = safe_eval(node.left, environment), safe_eval(node.comparators[0], environment)
        if left is UNKNOWN or right is UNKNOWN: return UNKNOWN
        operator = node.ops[0]
        if isinstance(operator, ast.Eq): return left == right
        if isinstance(operator, ast.NotEq): return left != right
        if isinstance(operator, ast.Lt): return left < right
        if isinstance(operator, ast.Gt): return left > right
        if isinstance(operator, ast.LtE): return left <= right
        if isinstance(operator, ast.GtE): return left >= right
        return UNKNOWN
    if isinstance(node, ast.BoolOp):
        values = [safe_eval(value, environment) for value in node.values]
        if any(value is UNKNOWN for value in values): return UNKNOWN
        if isinstance(node.op, ast.And): return all(values)
        if isinstance(node.op, ast.Or): return any(values)
    if isinstance(node, ast.IfExp):
        condition = safe_eval(node.test, environment)
        if condition is UNKNOWN: return UNKNOWN
        return safe_eval(node.body if condition else node.orelse, environment)
    return UNKNOWN

def safe_statements(statements, environment):
    for statement in statements:
        if isinstance(statement, ast.Return) and statement.value is not None:
            return safe_eval(statement.value, environment)
        if isinstance(statement, ast.If):
            condition = safe_eval(statement.test, environment)
            if condition is UNKNOWN: return UNKNOWN
            return safe_statements(statement.body if condition else statement.orelse, environment)
        if isinstance(statement, ast.Pass): return UNKNOWN
        return UNKNOWN
    return UNKNOWN

def explicit_hole(node):
    for statement in node.body:
        if isinstance(statement, ast.Pass): return True
        if isinstance(statement, ast.Raise):
            name = dotted(statement.exc.func) if isinstance(statement.exc, ast.Call) else dotted(statement.exc)
            if name.endswith("NotImplementedError"): return True
    return False

alternatives = []
for qualified in sorted(observations):
    cases = list(observations[qualified].values())
    if len(cases) < 2: continue
    definition = definitions[qualified]
    contradicted = False
    evaluable = 0
    for case in cases:
        actual = safe_statements(definition["node"].body, case["values"])
        if actual is not UNKNOWN:
            evaluable += 1
            if actual != case["expected"]: contradicted = True
    targeted = qualified in targets
    if not (targeted or explicit_hole(definition["node"]) or (evaluable == len(cases) and contradicted)):
        continue
    alternatives.append({
        "qualified_symbol": qualified,
        "reason": "FAILED_DIAGNOSTIC_TARGET" if targeted else ("EXPLICIT_SOURCE_HOLE" if explicit_hole(definition["node"]) else "STATIC_PUBLIC_CONTRADICTION"),
        "observations": [
            {"operands": {role: encoded(value) for role, value in case["values"].items()}, "expected_postimage": encoded(case["expected"])}
            for case in cases
        ],
    })

def declaration_insertion(owner, attribute, expected):
    class_node = classes[owner]["node"]
    body = list(class_node.body)
    if not body: return None
    first = body[0]
    line_start = line_starts[first.lineno - 1]
    prefix = source_bytes[line_start:byte_offset(first)].decode("utf-8")
    if not prefix.isspace(): prefix = " " * (class_node.col_offset + 4)
    is_docstring = isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) and isinstance(first.value.value, str)
    if is_docstring and len(body) > 1:
        anchor = body[1]
        anchor_start = line_starts[anchor.lineno - 1]
        indent = source_bytes[anchor_start:byte_offset(anchor)].decode("utf-8")
        if not indent.isspace(): indent = prefix
        offset = anchor_start
        insertion = indent + attribute + " = " + repr(expected) + newline
    elif is_docstring:
        offset = byte_offset(first, True)
        insertion = newline + prefix + attribute + " = " + repr(expected)
    else:
        offset = line_start
        insertion = prefix + attribute + " = " + repr(expected) + newline
    return offset, insertion

declarations = []
for (owner, attribute), values in sorted(declaration_observations.items()):
    if len(values) != 1:
        fail("CONFLICTING_SOURCE_BOUND_EDITS", "CONFLICTING_PUBLIC_DECLARATION_POSTIMAGES:" + owner + "." + attribute)
    expected = next(iter(values.values()))
    existing_value = classes[owner]["attributes"].get(attribute)
    if attribute in classes[owner]["attributes"] and existing_value is not None:
        current_value = literal(existing_value)
        if encoded(current_value) is None:
            fail("PUBLIC_INFORMATION_INSUFFICIENT", "DYNAMIC_PRODUCT_CLASS_DECLARATION:" + owner + "." + attribute)
        if current_value == expected["value"]: continue
        operation = "REPLACE"
        edit_start = byte_offset(existing_value)
        edit_end = byte_offset(existing_value, True)
        edit_source = repr(expected["value"])
        reason = "INCORRECT_PRODUCT_CLASS_DECLARATION"
    else:
        insertion = declaration_insertion(owner, attribute, expected["value"])
        if insertion is None: continue
        edit_start, edit_source = insertion
        edit_end = edit_start
        operation = "INSERT"
        reason = "MISSING_PRODUCT_CLASS_DECLARATION"
    declarations.append({
        "qualified_owner": owner,
        "attribute": attribute,
        "value_source": repr(expected["value"]),
        "operation": operation,
        "edit_start": edit_start,
        "edit_end": edit_end,
        "edit_source": edit_source,
        "evidence": sorted(set(expected["evidence"])),
        "reason": reason,
    })

if not alternatives and not declarations:
    fail("PUBLIC_INFORMATION_INSUFFICIENT", "NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
json.dump({"ok": True, "alternatives": alternatives, "declarations": declarations}, sys.stdout, ensure_ascii=False)
"#;

fn map_python_type(annotation: &str) -> Option<ProgramType> {
    let normalized = annotation
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    map_normalized_python_type(&normalized)
}

fn map_normalized_python_type(annotation: &str) -> Option<ProgramType> {
    let sequence_inner = [
        "list[",
        "builtins.list[",
        "List[",
        "typing.List[",
        "Sequence[",
        "typing.Sequence[",
    ]
    .into_iter()
    .find_map(|prefix| annotation.strip_prefix(prefix)?.strip_suffix(']'));
    if let Some(inner) = sequence_inner {
        return match map_normalized_python_type(inner)? {
            ProgramType::Int => Some(ProgramType::SequenceInt),
            ProgramType::SequenceInt => Some(ProgramType::NestedSequenceInt),
            _ => None,
        };
    }
    match annotation {
        "int" | "builtins.int" => Some(ProgramType::Int),
        "bool" | "builtins.bool" => Some(ProgramType::Bool),
        "str" | "builtins.str" => Some(ProgramType::String),
        "bytes" | "builtins.bytes" => Some(ProgramType::Bytes),
        "None" | "NoneType" => Some(ProgramType::Unit),
        _ => None,
    }
}

fn uniform_public_operand_type(
    observations: &[TypedMechanismObservationIR],
    roles: &BTreeSet<String>,
    role: &str,
) -> Result<ProgramType, CausalFrontendFailure> {
    let mut observed = None;
    for observation in observations {
        if observation
            .operands
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != *roles
        {
            return Err(CausalFrontendFailure::public(format!(
                "PUBLIC_OPERAND_ROLE_SET_MISMATCH:{role}"
            )));
        }
        let value_type = observation
            .operands
            .get(role)
            .ok_or_else(|| CausalFrontendFailure::public(format!("PUBLIC_OPERAND_MISSING:{role}")))?
            .program_type();
        if observed.as_ref().is_some_and(|prior| prior != &value_type) {
            return Err(CausalFrontendFailure::public(format!(
                "PUBLIC_OPERAND_TYPE_CONFLICT:{role}"
            )));
        }
        observed = Some(value_type);
    }
    observed
        .ok_or_else(|| CausalFrontendFailure::public(format!("PUBLIC_OPERAND_TYPE_MISSING:{role}")))
}

fn uniform_public_output_type(
    observations: &[TypedMechanismObservationIR],
) -> Result<ProgramType, CausalFrontendFailure> {
    let mut observed = None;
    for observation in observations {
        let value_type = observation.expected_postimage.program_type();
        if observed.as_ref().is_some_and(|prior| prior != &value_type) {
            return Err(CausalFrontendFailure::public("PUBLIC_OUTPUT_TYPE_CONFLICT"));
        }
        observed = Some(value_type);
    }
    observed.ok_or_else(|| CausalFrontendFailure::public("PUBLIC_OUTPUT_TYPE_MISSING"))
}

fn bind_declared_or_observed_type(
    annotation: &str,
    observed: ProgramType,
    label: &str,
) -> Result<(ProgramType, String), CausalFrontendFailure> {
    match map_python_type(annotation) {
        Some(declared) if declared != observed => Err(CausalFrontendFailure::public(format!(
            "DECLARED_PUBLIC_TYPE_CONFLICT:{label}:{declared:?}:{observed:?}"
        ))),
        Some(declared) => Ok((declared, "DECLARATION_AND_PUBLIC_OBSERVATION".to_string())),
        None => Ok((observed, "PUBLIC_OBSERVATION".to_string())),
    }
}

fn run_python_host(
    executable: &Path,
    source: &str,
    symbols: &[String],
) -> Result<PythonHostResponse, CausalFrontendFailure> {
    let input = serde_json::to_vec(&serde_json::json!({
        "source": source,
        "symbols": symbols,
        "max_closure": MAX_DEPENDENCY_CLOSURE,
    }))
    .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_INPUT:{error}")))?;
    let stdout = run_python_json_host(executable, PYTHON_AST_HOST, &input)?;
    serde_json::from_slice(&stdout)
        .map_err(|error| CausalFrontendFailure::unsupported(format!("PYTHON_HOST_OUTPUT:{error}")))
}

fn run_python_json_host(
    executable: &Path,
    script: &str,
    input: &[u8],
) -> Result<Vec<u8>, CausalFrontendFailure> {
    if !executable.is_file() {
        return Err(CausalFrontendFailure::public(format!(
            "PYTHON_EXECUTABLE_MISSING:{}",
            executable.display()
        )));
    }
    let mut child = Command::new(executable)
        // `-X utf8` is required on Windows hosts whose redirected stdio
        // inherits a legacy code page.  Without it a valid Korean identifier
        // can enter Python as surrogate-escaped bytes and invalidate the AST
        // byte spans even though the Rust request is valid UTF-8.
        .args(["-X", "utf8", "-I", "-S", "-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_SPAWN:{error}")))?;
    child
        .stdin
        .take()
        .ok_or_else(|| CausalFrontendFailure::public("PYTHON_HOST_STDIN_MISSING"))?
        .write_all(input)
        .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_STDIN:{error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CausalFrontendFailure::public("PYTHON_HOST_STDOUT_MISSING"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CausalFrontendFailure::public("PYTHON_HOST_STDERR_MISSING"))?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut stream = stdout;
        stream.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut stream = stderr;
        stream.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_WAIT:{error}")))?
        {
            break status;
        }
        if started.elapsed() >= PYTHON_HOST_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CausalFrontendFailure::unsupported("PYTHON_HOST_TIMEOUT"));
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| CausalFrontendFailure::public("PYTHON_HOST_STDOUT_PANICKED"))?
        .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_STDOUT:{error}")))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| CausalFrontendFailure::public("PYTHON_HOST_STDERR_PANICKED"))?
        .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_STDERR:{error}")))?;
    if !status.success() {
        return Err(CausalFrontendFailure::unsupported(format!(
            "PYTHON_HOST_EXIT:{}",
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(1_024)
                .collect::<String>()
        )));
    }
    Ok(stdout)
}

enum PythonCandidateBinding<'a> {
    PublicFunction {
        source: &'a str,
        public_symbol: &'a str,
    },
    ClassDeclaration {
        source: &'a str,
        qualified_owner: &'a str,
        attribute: &'a str,
        value_source: &'a str,
    },
}

impl PythonCandidateBinding<'_> {
    fn source(&self) -> &str {
        match self {
            Self::PublicFunction { source, .. } | Self::ClassDeclaration { source, .. } => source,
        }
    }

    fn json(&self) -> serde_json::Value {
        match self {
            Self::PublicFunction {
                source,
                public_symbol,
            } => serde_json::json!({
                "source": source,
                "public_symbol": public_symbol,
            }),
            Self::ClassDeclaration {
                source,
                qualified_owner,
                attribute,
                value_source,
            } => serde_json::json!({
                "source": source,
                "declaration_owner": qualified_owner,
                "declaration_attribute": attribute,
                "declaration_value_source": value_source,
            }),
        }
    }
}

fn validate_python_candidate_bindings(
    executable: &Path,
    candidates: &[PythonCandidateBinding<'_>],
) -> Result<(Vec<Result<(), CausalFrontendFailure>>, usize), CausalFrontendFailure> {
    if candidates.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let mut outcomes = Vec::with_capacity(candidates.len());
    let mut batch_processes = 0_usize;
    let mut start = 0_usize;
    while start < candidates.len() {
        let mut end = start;
        let mut bytes = 0_usize;
        while end < candidates.len() && end - start < MAX_CANDIDATE_VALIDATION_BATCH_ITEMS {
            let next_bytes = candidates[end].source().len();
            if end > start
                && bytes.saturating_add(next_bytes) > MAX_CANDIDATE_VALIDATION_BATCH_BYTES
            {
                break;
            }
            bytes = bytes.saturating_add(next_bytes);
            end += 1;
        }
        let input = serde_json::to_vec(&serde_json::json!({
            "candidates": candidates[start..end]
                .iter()
                .map(PythonCandidateBinding::json)
                .collect::<Vec<_>>(),
        }))
        .map_err(|error| CausalFrontendFailure::public(format!("CANDIDATE_BATCH_INPUT:{error}")))?;
        let stdout = run_python_json_host(executable, PYTHON_CANDIDATE_BATCH_HOST, &input)?;
        batch_processes = batch_processes.saturating_add(1);
        let response: PythonCandidateBatchResponse =
            serde_json::from_slice(&stdout).map_err(|error| {
                CausalFrontendFailure::unsupported(format!("CANDIDATE_BATCH_OUTPUT:{error}"))
            })?;
        if !response.ok {
            return Err(classified_host_failure(
                response.failure.as_deref(),
                response.detail.as_deref(),
            ));
        }
        if response.results.len() != end - start {
            return Err(CausalFrontendFailure::public("CANDIDATE_BATCH_CARDINALITY"));
        }
        outcomes.extend(response.results.into_iter().map(|result| {
            if result.ok {
                Ok(())
            } else {
                Err(classified_host_failure(
                    result.failure.as_deref(),
                    result.detail.as_deref(),
                ))
            }
        }));
        start = end;
    }
    Ok((outcomes, batch_processes))
}

fn validate_python_candidate_batch(
    executable: &Path,
    candidates: &[(&str, &str)],
) -> Result<(Vec<Result<(), CausalFrontendFailure>>, usize), CausalFrontendFailure> {
    let bindings = candidates
        .iter()
        .map(
            |(source, public_symbol)| PythonCandidateBinding::PublicFunction {
                source,
                public_symbol,
            },
        )
        .collect::<Vec<_>>();
    validate_python_candidate_bindings(executable, &bindings)
}

fn validate_python_declaration_candidate_batch(
    executable: &Path,
    candidates: &[(&str, &str, &str, &str)],
) -> Result<(Vec<Result<(), CausalFrontendFailure>>, usize), CausalFrontendFailure> {
    let bindings = candidates
        .iter()
        .map(|(source, qualified_owner, attribute, value_source)| {
            PythonCandidateBinding::ClassDeclaration {
                source,
                qualified_owner,
                attribute,
                value_source,
            }
        })
        .collect::<Vec<_>>();
    validate_python_candidate_bindings(executable, &bindings)
}

fn host_failure(response: &PythonHostResponse) -> Option<CausalFrontendFailure> {
    if response.ok {
        return None;
    }
    Some(classified_host_failure(
        response.failure.as_deref(),
        response.detail.as_deref(),
    ))
}

/// Python reports concrete parser/AST observations only. The Rust kernel owns
/// the public failure ontology and derives it from the bounded observation
/// code; the host's `failure` string is retained solely for mismatch audit.
fn failure_kind_from_host_detail(detail: &str) -> CausalFrontendFailureKind {
    let observation = detail.split(':').next().unwrap_or(detail);
    match observation {
        "CONFLICTING_PUBLIC_DECLARATION_POSTIMAGES" => {
            CausalFrontendFailureKind::ConflictingSourceBoundEdits
        }
        "PYTHON_PARSE"
        | "PYTHON_AST_SPAN_MISSING"
        | "PYTHON_AST_UTF8_SPAN"
        | "PYTHON_ANNOTATION"
        | "PYTHON_VARIADIC_PUBLIC_SYMBOL"
        | "PYTHON_UNSUPPORTED_NODE"
        | "CANDIDATE_PARSE"
        | "MATERIALIZED_CLASS_DECLARATION_LITERAL"
        | "IMPLEMENTATION_PARSE"
        | "DECLARATION_AST_SPAN_MISSING"
        | "TEST_PARSE" => CausalFrontendFailureKind::UnsupportedLanguageSyntax,
        "PYTHON_HOST_INPUT"
        | "DEPENDENCY_CLOSURE_BUDGET"
        | "EXACT_PUBLIC_SYMBOL_NOT_FOUND"
        | "PUBLIC_SYMBOL_POSTIMAGE_MISSING"
        | "DEPENDENCY_BINDING_STATE_BUDGET"
        | "DEPENDENCY_OPERAND_BINDING_AMBIGUOUS"
        | "DEPENDENCY_POSTIMAGE_MISSING"
        | "CANDIDATE_BATCH_INPUT"
        | "CANDIDATE_INPUT"
        | "MATERIALIZED_PUBLIC_SYMBOL_IDENTITY_LOST"
        | "MATERIALIZED_CLASS_DECLARATION_IDENTITY_LOST"
        | "MATERIALIZED_CLASS_DECLARATION_POSTIMAGE_MISMATCH"
        | "REPOSITORY_DISCOVERY_INPUT"
        | "TEST_SOURCE_MISSING"
        | "DYNAMIC_PRODUCT_CLASS_DECLARATION"
        | "NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE" => {
            CausalFrontendFailureKind::PublicInformationInsufficient
        }
        _ => CausalFrontendFailureKind::UnsupportedLanguageSyntax,
    }
}

fn classified_host_failure(failure: Option<&str>, detail: Option<&str>) -> CausalFrontendFailure {
    let detail = detail.unwrap_or("PYTHON_HOST_UNCLASSIFIED").to_string();
    let kind = failure_kind_from_host_detail(&detail);
    let declared_mismatch = failure.is_some_and(|declared| declared != kind.as_code());
    let detail = if declared_mismatch {
        format!(
            "PYTHON_FAILURE_CLASSIFICATION_MISMATCH:declared={}:observation={detail}",
            failure.unwrap_or("MISSING")
        )
    } else {
        detail
    };
    CausalFrontendFailure { kind, detail }
}

/// Discover public literal observations from Python tests, retain only an
/// explicit hole, a statically contradicted implementation, or a symbol bound
/// by a failing diagnostic, then run the exact same source-bound synthesis and
/// atomic materialization path as an explicit causal request.
fn repository_relative_path_valid(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn read_repository_source(
    canonical_root: &Path,
    relative_path: &Path,
    max_bytes: usize,
) -> Result<String, CausalFrontendFailure> {
    if !repository_relative_path_valid(relative_path) {
        return Err(CausalFrontendFailure::public("REPOSITORY_PATH_INVALID"));
    }
    let joined = canonical_root.join(relative_path);
    let metadata = fs::symlink_metadata(&joined).map_err(|error| {
        CausalFrontendFailure::public(format!("REPOSITORY_PATH_METADATA:{error}"))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_PATH_NOT_REGULAR_FILE",
        ));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_PATH_FILE_TOO_LARGE",
        ));
    }
    let canonical = fs::canonicalize(&joined).map_err(|error| {
        CausalFrontendFailure::public(format!("REPOSITORY_PATH_CANONICALIZE:{error}"))
    })?;
    if !canonical.starts_with(canonical_root) {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_PATH_OUTSIDE_ROOT",
        ));
    }
    fs::read_to_string(&canonical).map_err(|error| {
        CausalFrontendFailure::unsupported(format!("REPOSITORY_PATH_UTF8:{error}"))
    })
}

pub fn discover_and_synthesize_python_repository_paths(
    request: &SourceBoundRepositoryPathDiscoveryRequestIR,
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    discover_and_synthesize_python_repository_paths_with_operators(request, &[])
}

pub fn discover_and_synthesize_python_repository_paths_with_operators(
    request: &SourceBoundRepositoryPathDiscoveryRequestIR,
    operators: &[TypedMechanismImprovementOperatorIR],
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    if request.schema != SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA
        || !request.repository_root.is_absolute()
        || !request.repository_root.is_dir()
        || !repository_relative_path_valid(&request.source_relative_path)
        || request.test_relative_paths.is_empty()
        || request.test_relative_paths.len() > MAX_TEST_SOURCES
        || request
            .test_relative_paths
            .iter()
            .any(|path| !repository_relative_path_valid(path))
    {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_PATH_DISCOVERY_ENVELOPE",
        ));
    }
    let canonical_root = fs::canonicalize(&request.repository_root).map_err(|error| {
        CausalFrontendFailure::public(format!("REPOSITORY_ROOT_CANONICALIZE:{error}"))
    })?;
    let source = read_repository_source(
        &canonical_root,
        &request.source_relative_path,
        MAX_SOURCE_BYTES,
    )?;
    let mut remaining_test_bytes = MAX_TEST_SOURCE_BYTES;
    let mut test_sources = Vec::with_capacity(request.test_relative_paths.len());
    for relative_path in &request.test_relative_paths {
        if remaining_test_bytes == 0 {
            return Err(CausalFrontendFailure::public(
                "REPOSITORY_PATH_TEST_SET_INVALID",
            ));
        }
        let source = read_repository_source(&canonical_root, relative_path, remaining_test_bytes)?;
        remaining_test_bytes = remaining_test_bytes.saturating_sub(source.len());
        test_sources.push(RepositoryTestSourceIR {
            relative_path: relative_path.clone(),
            source,
        });
    }
    test_sources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if test_sources
        .windows(2)
        .any(|pair| pair[0].relative_path == pair[1].relative_path)
        || test_sources
            .iter()
            .map(|test| test.source.len())
            .sum::<usize>()
            > MAX_TEST_SOURCE_BYTES
    {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_PATH_TEST_SET_INVALID",
        ));
    }
    discover_and_synthesize_python_repository_with_operators(
        &SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: request.source_relative_path.clone(),
            source,
            test_sources,
            python_executable: request.python_executable.clone(),
            target_symbols: request.target_symbols.clone(),
            allowed_effects: request.allowed_effects.clone(),
            max_expression_depth: request.max_expression_depth,
            max_candidates: request.max_candidates,
        },
        operators,
    )
}

pub fn discover_and_synthesize_python_repository(
    request: &SourceBoundRepositoryDiscoveryRequestIR,
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    discover_and_synthesize_python_repository_with_operators(request, &[])
}

pub fn discover_and_synthesize_python_repository_with_operators(
    request: &SourceBoundRepositoryDiscoveryRequestIR,
    operators: &[TypedMechanismImprovementOperatorIR],
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    if request.schema != SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA
        || request.source.is_empty()
        || request.source.len() > MAX_SOURCE_BYTES
        || request.test_sources.is_empty()
        || request.test_sources.len() > MAX_TEST_SOURCES
        || request
            .test_sources
            .iter()
            .map(|test| test.source.len())
            .try_fold(0_usize, usize::checked_add)
            .is_none_or(|bytes| bytes > MAX_TEST_SOURCE_BYTES)
        || request
            .test_sources
            .iter()
            .any(|test| test.source.is_empty())
    {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_DISCOVERY_ENVELOPE",
        ));
    }
    if language_backend_for_path(&request.source_relative_path)? != SourceLanguageBackend::PythonAst
        || request.test_sources.iter().any(|test| {
            language_backend_for_path(&test.relative_path) != Ok(SourceLanguageBackend::PythonAst)
        })
    {
        return Err(CausalFrontendFailure::unsupported(
            "REPOSITORY_DISCOVERY_REQUIRES_PYTHON_SOURCE_AND_TESTS",
        ));
    }
    let mut target_symbols = request.target_symbols.clone();
    target_symbols.sort();
    target_symbols.dedup();
    let tests = request
        .test_sources
        .iter()
        .map(|test| {
            serde_json::json!({
                "relative_path": test.relative_path.to_string_lossy().replace('\\', "/"),
                "source": test.source,
            })
        })
        .collect::<Vec<_>>();
    let input = serde_json::to_vec(&serde_json::json!({
        "source": request.source,
        "tests": tests,
        "target_symbols": target_symbols,
    }))
    .map_err(|error| CausalFrontendFailure::public(format!("DISCOVERY_HOST_INPUT:{error}")))?;
    let stdout = run_python_json_host(
        &request.python_executable,
        PYTHON_PUBLIC_OBSERVATION_HOST,
        &input,
    )?;
    let response: PythonObservationDiscoveryResponse =
        serde_json::from_slice(&stdout).map_err(|error| {
            CausalFrontendFailure::unsupported(format!("DISCOVERY_HOST_OUTPUT:{error}"))
        })?;
    if !response.ok {
        return Err(classified_host_failure(
            response.failure.as_deref(),
            response.detail.as_deref(),
        ));
    }
    let alternatives = response
        .alternatives
        .into_iter()
        .map(|alternative| {
            let evidence_sha256 = sha256(
                serde_json::to_vec(&alternative.observations)
                    .map_err(|error| {
                        CausalFrontendFailure::public(format!("DISCOVERY_EVIDENCE_HASH:{error}"))
                    })?
                    .as_slice(),
            );
            Ok(SourceBoundCausalAlternativeIR {
                alternative_id: format!(
                    "AUTO:{}:{}:{}",
                    alternative.reason,
                    alternative.qualified_symbol,
                    &evidence_sha256[..16]
                ),
                public_symbol: alternative.qualified_symbol,
                public_observations: alternative.observations,
                allowed_effects: if request.allowed_effects.is_empty() {
                    vec![Effect::Pure]
                } else {
                    request.allowed_effects.clone()
                },
                require_conditional: false,
                max_expression_depth: request.max_expression_depth,
                max_candidates: request.max_candidates,
            })
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    if alternatives
        .len()
        .saturating_add(response.declarations.len())
        > MAX_CAUSAL_ALTERNATIVES
    {
        return Err(CausalFrontendFailure::public(
            "REPOSITORY_DISCOVERY_ALTERNATIVE_BUDGET",
        ));
    }
    let mut declaration_ids = BTreeSet::new();
    let pending_declarations = response
        .declarations
        .into_iter()
        .map(|declaration| {
            let requested_public_symbol =
                format!("{}.{}", declaration.qualified_owner, declaration.attribute);
            if !matches!(
                declaration.reason.as_str(),
                "MISSING_PRODUCT_CLASS_DECLARATION" | "INCORRECT_PRODUCT_CLASS_DECLARATION"
            ) || declaration.evidence.is_empty()
                || !declaration_ids.insert(requested_public_symbol.clone())
            {
                return Err(CausalFrontendFailure::public(
                    "DECLARATION_DISCOVERY_PUBLIC_INFORMATION",
                ));
            }
            let public_evidence_sha256 = sha256(
                serde_json::to_vec(&declaration.evidence)
                    .map_err(|error| {
                        CausalFrontendFailure::public(format!("DECLARATION_EVIDENCE_HASH:{error}"))
                    })?
                    .as_slice(),
            );
            let mut declaration_template = SourceBoundDeclarationTemplateIR {
                qualified_owner: declaration.qualified_owner,
                attribute: declaration.attribute,
                value_source: declaration.value_source,
                operation: declaration.operation,
                edit_range: ByteRange {
                    start: declaration.edit_start,
                    end: declaration.edit_end,
                },
                edit_source: declaration.edit_source,
                public_evidence_sha256: public_evidence_sha256.clone(),
                source_template_sha256: String::new(),
            };
            declaration_template.source_template_sha256 =
                source_bound_declaration_template_hash(&request.source, &declaration_template)?;
            let materialized =
                materialize_python_declaration(&request.source, &declaration_template)?;
            let alternative_id = format!(
                "AUTO:{}:{}:{}",
                declaration.reason,
                requested_public_symbol,
                &public_evidence_sha256[..16]
            );
            Ok((
                SourceBoundDeclarationAlternativeReceiptIR {
                    alternative_id,
                    requested_public_symbol,
                    declaration_template,
                    replayable_patch: ReplayableSourceBoundPatchIR {
                        predecessor_sha256: String::new(),
                        edit: SourceEditAtom::AtomicMultiEdit { edits: Vec::new() },
                        candidate_sha256: String::new(),
                        candidate_replay_sha256: String::new(),
                        candidate_materialization_is_one_to_one: false,
                    },
                    candidate_validation_processes: 0,
                },
                materialized,
            ))
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    let declaration_validation_inputs = pending_declarations
        .iter()
        .map(|(declaration, materialized)| {
            (
                materialized.candidate_source.as_str(),
                declaration.declaration_template.qualified_owner.as_str(),
                declaration.declaration_template.attribute.as_str(),
                declaration.declaration_template.value_source.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let (declaration_validation_outcomes, declaration_validation_processes) =
        validate_python_declaration_candidate_batch(
            &request.python_executable,
            &declaration_validation_inputs,
        )?;
    for outcome in declaration_validation_outcomes {
        outcome?;
    }
    drop(declaration_validation_inputs);
    let declaration_alternatives = pending_declarations
        .into_iter()
        .map(|(mut declaration, materialized)| {
            declaration.replayable_patch = into_replayable_source_bound_patch(materialized);
            declaration.candidate_validation_processes = declaration_validation_processes;
            declaration
        })
        .collect::<Vec<_>>();
    if alternatives.is_empty() && declaration_alternatives.is_empty() {
        return Err(CausalFrontendFailure::public(
            "NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE",
        ));
    }
    let causal_request = SourceBoundCausalRequestIR {
        schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
        source_relative_path: request.source_relative_path.clone(),
        source: request.source.clone(),
        python_executable: request.python_executable.clone(),
        alternatives,
    };
    let mut receipt = if causal_request.alternatives.is_empty() {
        SourceBoundCausalReceiptIR {
            schema: SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA.to_string(),
            source_relative_path: request.source_relative_path.clone(),
            language_backend: SourceLanguageBackend::PythonAst,
            predecessor_sha256: sha256(request.source.as_bytes()),
            alternatives: Vec::new(),
            declaration_alternatives: Vec::new(),
            patch_variants: Vec::new(),
            alternative_worker_count: 0,
            public_symbol_owner_preserved: false,
            execution_dependency_closure_preserved: false,
            single_and_multi_edit_share_atomic_path: false,
            receipt_sha256: String::new(),
        }
    } else {
        analyze_and_synthesize_source_bound_with_operators(&causal_request, operators)?
    };
    receipt.declaration_alternatives = declaration_alternatives;
    receipt.patch_variants = select_source_bound_patch_proposals(
        &receipt.alternatives,
        build_source_bound_patch_variants_with_declarations(
            &request.source,
            &receipt.alternatives,
            &receipt.declaration_alternatives,
        )?,
    )?;
    let (owner, closure, atomic) = source_bound_receipt_claims(&receipt);
    receipt.public_symbol_owner_preserved = owner;
    receipt.execution_dependency_closure_preserved = closure;
    receipt.single_and_multi_edit_share_atomic_path = atomic;
    receipt.receipt_sha256 = source_bound_receipt_hash(&receipt)?;
    validate_source_bound_causal_receipt_with_python(
        &receipt,
        &request.source,
        &request.python_executable,
    )?;
    Ok(receipt)
}

fn convert_python_definition(
    definition: PythonFunctionDefinition,
    observations: &[TypedMechanismObservationIR],
) -> Result<SourceBoundFunctionTemplateIR, CausalFrontendFailure> {
    let roles = definition
        .operands
        .iter()
        .map(|operand| operand.name.clone())
        .collect::<BTreeSet<_>>();
    if roles.is_empty() || roles.len() != definition.operands.len() {
        return Err(CausalFrontendFailure::public(format!(
            "PUBLIC_OPERANDS_MISSING_OR_DUPLICATED:{}",
            definition.qualified_symbol
        )));
    }
    let mut operand_type_evidence = BTreeMap::new();
    let operands = definition
        .operands
        .into_iter()
        .map(|operand| {
            let observed = uniform_public_operand_type(observations, &roles, &operand.name)?;
            let (value_type, evidence) = bind_declared_or_observed_type(
                &operand.annotation,
                observed,
                &format!("{}:{}", definition.qualified_symbol, operand.name),
            )?;
            operand_type_evidence.insert(operand.name.clone(), evidence);
            Ok(SourceOperandIR {
                role: operand.name.clone(),
                source: operand.name,
                value_type,
            })
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    let observed_output = uniform_public_output_type(observations)?;
    let (output_type, output_type_evidence) = bind_declared_or_observed_type(
        &definition.return_annotation,
        observed_output,
        &format!("{}:RETURN", definition.qualified_symbol),
    )?;
    let cuts = definition
        .cuts
        .into_iter()
        .map(|cut| {
            let branch = match cut.branch.as_str() {
                "UNCONDITIONAL" => CausalCutBranch::Unconditional,
                "THEN" => CausalCutBranch::Then,
                "ELSE" => CausalCutBranch::Else,
                other => {
                    return Err(CausalFrontendFailure::unsupported(format!(
                        "PYTHON_CUT_BRANCH:{other}"
                    )))
                }
            };
            let condition_range = match (cut.condition_start, cut.condition_end) {
                (Some(start), Some(end)) if start < end => Some(ByteRange { start, end }),
                (None, None) => None,
                _ => {
                    return Err(CausalFrontendFailure::unsupported(
                        "PYTHON_CONDITION_SPAN_INVALID",
                    ))
                }
            };
            if cut.postimage_start >= cut.postimage_end {
                return Err(CausalFrontendFailure::unsupported(
                    "PYTHON_POSTIMAGE_SPAN_INVALID",
                ));
            }
            Ok(SourceBoundCausalCutIR {
                branch,
                condition_source: cut.condition_source,
                condition_range,
                condition_template: cut.condition_template,
                postimage_source: cut.postimage_source,
                postimage_range: ByteRange {
                    start: cut.postimage_start,
                    end: cut.postimage_end,
                },
                postimage_template: cut.postimage_template,
            })
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    let mut template = SourceBoundFunctionTemplateIR {
        qualified_symbol: definition.qualified_symbol,
        owner: definition.owner,
        is_async: definition.is_async,
        operands,
        output_type,
        operand_type_evidence,
        output_type_evidence,
        effects: definition.effects,
        direct_dependencies: definition.direct_dependencies,
        execution_dependency_closure: definition.execution_dependency_closure,
        external_callers: definition.external_callers,
        cuts,
        source_template_sha256: String::new(),
    };
    template.source_template_sha256 = source_bound_function_template_hash(&template)?;
    Ok(template)
}

fn remap_public_observations_for_closure(
    observations: &[TypedMechanismObservationIR],
    bindings: &BTreeMap<String, String>,
) -> Result<Vec<TypedMechanismObservationIR>, CausalFrontendFailure> {
    if bindings.is_empty() || bindings.values().collect::<BTreeSet<_>>().len() != bindings.len() {
        return Err(CausalFrontendFailure::public(
            "DEPENDENCY_OPERAND_BINDING_AMBIGUOUS",
        ));
    }
    observations
        .iter()
        .map(|observation| {
            let operands = bindings
                .iter()
                .map(|(closure_operand, public_operand)| {
                    observation
                        .operands
                        .get(public_operand)
                        .cloned()
                        .map(|value| (closure_operand.clone(), value))
                        .ok_or_else(|| {
                            CausalFrontendFailure::public(format!(
                                "DEPENDENCY_PUBLIC_OPERAND_MISSING:{public_operand}"
                            ))
                        })
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            Ok(TypedMechanismObservationIR {
                operands,
                expected_postimage: observation.expected_postimage.clone(),
            })
        })
        .collect()
}

fn convert_python_closure_definition(
    definition: PythonClosureTemplateDefinition,
    observations: &[TypedMechanismObservationIR],
) -> Result<SourceBoundFunctionTemplateIR, CausalFrontendFailure> {
    convert_python_definition(
        PythonFunctionDefinition {
            qualified_symbol: definition.qualified_symbol,
            owner: definition.owner,
            is_async: definition.is_async,
            operands: definition.operands,
            return_annotation: definition.return_annotation,
            effects: definition.effects,
            direct_dependencies: definition.direct_dependencies,
            execution_dependency_closure: definition.execution_dependency_closure,
            external_callers: definition.external_callers,
            cuts: definition.cuts,
            closure_templates: Vec::new(),
            closure_rejections: Vec::new(),
        },
        observations,
    )
}

fn python_expression(
    expression: &TypedSyntaxExpressionIR,
    sources: &BTreeMap<String, String>,
    operand_types: &BTreeMap<String, ProgramType>,
) -> Result<String, CausalFrontendFailure> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => sources
            .get(role)
            .map(|source| format!("({source})"))
            .ok_or_else(|| CausalFrontendFailure::public(format!("UNKNOWN_OPERAND_ROLE:{role}"))),
        TypedSyntaxExpressionIR::IntLiteral { value } => Ok(value.to_string()),
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            Ok(if *value { "True" } else { "False" }.to_string())
        }
        TypedSyntaxExpressionIR::Unary { operator, input } => {
            let input = python_expression(input, sources, operand_types)?;
            Ok(match operator {
                UnaryOperator::Negate => format!(
                    "(lambda _b_core_value: min(max(-_b_core_value, -9223372036854775808), 9223372036854775807))({input})"
                ),
                UnaryOperator::Not => format!("(not {input})"),
            })
        }
        TypedSyntaxExpressionIR::StringTransform { operator, input } => Ok(format!(
            "({}).{}()",
            python_expression(input, sources, operand_types)?,
            match operator {
                StringTransformOperator::Trim => "strip",
                StringTransformOperator::Lowercase => "lower",
                StringTransformOperator::Uppercase => "upper",
            }
        )),
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => {
            let left_source = python_expression(left, sources, operand_types)?;
            let right_source = python_expression(right, sources, operand_types)?;
            let integer_operands = python_expression_type(left, operand_types)
                == Some(ProgramType::Int)
                && python_expression_type(right, operand_types) == Some(ProgramType::Int);
            Ok(match operator {
                BinaryOperator::Add if integer_operands => python_saturating_arithmetic(
                    "+",
                    &left_source,
                    &right_source,
                ),
                BinaryOperator::Subtract if integer_operands => python_saturating_arithmetic(
                    "-",
                    &left_source,
                    &right_source,
                ),
                BinaryOperator::Multiply if integer_operands => python_saturating_arithmetic(
                    "*",
                    &left_source,
                    &right_source,
                ),
                BinaryOperator::Divide => format!(
                    "(lambda _b_core_left, _b_core_right: min(max(((abs(_b_core_left) // abs(_b_core_right)) if ((_b_core_left >= 0) == (_b_core_right >= 0)) else -(abs(_b_core_left) // abs(_b_core_right))), -9223372036854775808), 9223372036854775807))({left_source}, {right_source})"
                ),
                BinaryOperator::Modulo => format!(
                    "(lambda _b_core_left, _b_core_right: (-(abs(_b_core_left) % abs(_b_core_right)) if _b_core_left < 0 else (abs(_b_core_left) % abs(_b_core_right))))({left_source}, {right_source})"
                ),
                _ => format!(
                    "({left_source} {} {right_source})",
                    match operator {
                        BinaryOperator::Add => "+",
                        BinaryOperator::Subtract => "-",
                        BinaryOperator::Multiply => "*",
                        BinaryOperator::Divide | BinaryOperator::Modulo => unreachable!(),
                        BinaryOperator::Equal => "==",
                        BinaryOperator::NotEqual => "!=",
                        BinaryOperator::LessThan => "<",
                        BinaryOperator::LessThanOrEqual => "<=",
                        BinaryOperator::GreaterThan => ">",
                        BinaryOperator::GreaterThanOrEqual => ">=",
                        BinaryOperator::And => "and",
                        BinaryOperator::Or => "or",
                    }
                ),
            })
        }
        TypedSyntaxExpressionIR::Length { input } => Ok(format!(
            "len({})",
            python_expression(input, sources, operand_types)?
        )),
        TypedSyntaxExpressionIR::Index { collection, index } => Ok(format!(
            "({})[{}]",
            python_expression(collection, sources, operand_types)?,
            python_expression(index, sources, operand_types)?
        )),
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(format!(
            "{api_token}({})",
            arguments
                .iter()
                .map(|argument| python_expression(argument, sources, operand_types))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
    }
}

fn python_saturating_arithmetic(operator: &str, left: &str, right: &str) -> String {
    format!(
        "(lambda _b_core_left, _b_core_right: min(max((_b_core_left {operator} _b_core_right), -9223372036854775808), 9223372036854775807))({left}, {right})"
    )
}

fn python_expression_type(
    expression: &TypedSyntaxExpressionIR,
    operand_types: &BTreeMap<String, ProgramType>,
) -> Option<ProgramType> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => operand_types.get(role).cloned(),
        TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::Unary {
            operator: UnaryOperator::Negate,
            ..
        }
        | TypedSyntaxExpressionIR::Length { .. } => Some(ProgramType::Int),
        TypedSyntaxExpressionIR::BoolLiteral { .. }
        | TypedSyntaxExpressionIR::Unary {
            operator: UnaryOperator::Not,
            ..
        } => Some(ProgramType::Bool),
        TypedSyntaxExpressionIR::StringTransform { .. } => Some(ProgramType::String),
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => match operator {
            BinaryOperator::Add
                if python_expression_type(left, operand_types) == Some(ProgramType::String)
                    && python_expression_type(right, operand_types)
                        == Some(ProgramType::String) =>
            {
                Some(ProgramType::String)
            }
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo => Some(ProgramType::Int),
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::And
            | BinaryOperator::Or => Some(ProgramType::Bool),
        },
        TypedSyntaxExpressionIR::Index { collection, .. } => {
            match python_expression_type(collection, operand_types)? {
                ProgramType::SequenceInt | ProgramType::Bytes => Some(ProgramType::Int),
                ProgramType::NestedSequenceInt => Some(ProgramType::SequenceInt),
                ProgramType::String => Some(ProgramType::String),
                _ => None,
            }
        }
        TypedSyntaxExpressionIR::Call { .. } => None,
    }
}

fn replacement_edit(
    source: &str,
    range: ByteRange,
    replacement: String,
) -> Result<SourceEditAtom, CausalFrontendFailure> {
    let observed = source
        .get(range.start..range.end)
        .ok_or_else(|| CausalFrontendFailure::unsupported("SOURCE_BOUND_RANGE_INVALID"))?;
    if observed == replacement {
        return Err(CausalFrontendFailure::public("SOURCE_BOUND_EDIT_NO_OP"));
    }
    Ok(SourceEditAtom::Replace {
        range,
        expected_sha256: sha256(observed.as_bytes()),
        replacement,
    })
}

fn materialize_python_synthesis(
    source: &str,
    template: &SourceBoundFunctionTemplateIR,
    synthesis: &TypedMechanismSynthesisReceiptIR,
) -> Result<MaterializedSourceBoundPatchIR, CausalFrontendFailure> {
    let sources = template
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.source.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_types = template
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let goal = &synthesis.winning_goal;
    let condition = goal
        .condition
        .as_ref()
        .map(|expression| python_expression(expression, &sources, &operand_types))
        .transpose()?;
    let postimage = python_expression(&goal.postimage, &sources, &operand_types)?;
    let otherwise = goal
        .otherwise
        .as_ref()
        .map(|expression| python_expression(expression, &sources, &operand_types))
        .transpose()?;
    let mut edits = Vec::new();
    let unconditional = template
        .cuts
        .iter()
        .filter(|cut| cut.branch == CausalCutBranch::Unconditional)
        .collect::<Vec<_>>();
    if unconditional.len() == template.cuts.len() {
        let replacement = match (&condition, &otherwise) {
            (Some(condition), Some(otherwise)) => {
                format!("({postimage} if {condition} else {otherwise})")
            }
            (None, None) => postimage,
            _ => {
                return Err(CausalFrontendFailure::unsupported(
                    "INCOMPLETE_CONDITIONAL_LOWERING",
                ))
            }
        };
        for cut in unconditional {
            edits.push(replacement_edit(
                source,
                cut.postimage_range,
                replacement.clone(),
            )?);
        }
    } else {
        let condition_ranges = template
            .cuts
            .iter()
            .filter_map(|cut| cut.condition_range)
            .collect::<BTreeSet<_>>();
        let then_cuts = template
            .cuts
            .iter()
            .filter(|cut| cut.branch == CausalCutBranch::Then)
            .collect::<Vec<_>>();
        let else_cuts = template
            .cuts
            .iter()
            .filter(|cut| cut.branch == CausalCutBranch::Else)
            .collect::<Vec<_>>();
        let explicit_topology =
            unconditional.is_empty() && !then_cuts.is_empty() && !else_cuts.is_empty();
        let fallthrough_topology = unconditional.len() == 1
            && then_cuts.len() == 1
            && else_cuts.is_empty()
            && then_cuts[0].postimage_range.end <= unconditional[0].postimage_range.start;
        if condition_ranges.len() != 1 || (!explicit_topology && !fallthrough_topology) {
            return Err(CausalFrontendFailure::unsupported(
                "CONDITIONAL_CUT_TOPOLOGY_UNSUPPORTED",
            ));
        }
        match (condition, otherwise) {
            (None, None) => {
                for cut in &template.cuts {
                    edits.push(replacement_edit(
                        source,
                        cut.postimage_range,
                        postimage.clone(),
                    )?);
                }
            }
            (Some(condition), Some(otherwise)) => {
                edits.push(replacement_edit(
                    source,
                    *condition_ranges.iter().next().expect("one range"),
                    condition,
                )?);
                for cut in &template.cuts {
                    let replacement = match cut.branch {
                        CausalCutBranch::Then => postimage.clone(),
                        CausalCutBranch::Else => otherwise.clone(),
                        CausalCutBranch::Unconditional if fallthrough_topology => otherwise.clone(),
                        CausalCutBranch::Unconditional => {
                            return Err(CausalFrontendFailure::unsupported(
                                "CONDITIONAL_CUT_TOPOLOGY_UNSUPPORTED",
                            ))
                        }
                    };
                    edits.push(replacement_edit(source, cut.postimage_range, replacement)?);
                }
            }
            _ => {
                return Err(CausalFrontendFailure::unsupported(
                    "INCOMPLETE_CONDITIONAL_LOWERING",
                ))
            }
        }
    }
    let edit = SourceEditAtom::AtomicMultiEdit { edits };
    let candidate_source = apply_edit_atom(source, &edit).map_err(|error| {
        if error.contains("OVERLAPPING")
            || error.contains("INSIDE_CONSUMED")
            || error.contains("DUPLICATE")
        {
            CausalFrontendFailure::conflict(error)
        } else {
            CausalFrontendFailure::unsupported(error)
        }
    })?;
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let replay = apply_edit_atom(source, &edit).map_err(CausalFrontendFailure::conflict)?;
    let candidate_replay_sha256 = sha256(replay.as_bytes());
    if replay != candidate_source || candidate_replay_sha256 != candidate_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "CANDIDATE_MATERIALIZATION_DIVERGED",
        ));
    }
    Ok(MaterializedSourceBoundPatchIR {
        predecessor_sha256: sha256(source.as_bytes()),
        edit,
        candidate_source,
        candidate_sha256,
        candidate_replay_sha256,
        candidate_materialization_is_one_to_one: true,
    })
}

fn materialize_python_declaration(
    source: &str,
    template: &SourceBoundDeclarationTemplateIR,
) -> Result<MaterializedSourceBoundPatchIR, CausalFrontendFailure> {
    validate_source_bound_declaration_template(source, template)?;
    let declaration_edit = match template.operation {
        SourceBoundDeclarationOperation::Insert => SourceEditAtom::Insert {
            offset: template.edit_range.start,
            content: template.edit_source.clone(),
        },
        SourceBoundDeclarationOperation::Replace => {
            let observed = source
                .get(template.edit_range.start..template.edit_range.end)
                .ok_or_else(|| CausalFrontendFailure::conflict("DECLARATION_REPLACEMENT_RANGE"))?;
            SourceEditAtom::Replace {
                range: template.edit_range,
                expected_sha256: sha256(observed.as_bytes()),
                replacement: template.edit_source.clone(),
            }
        }
    };
    let edit = SourceEditAtom::AtomicMultiEdit {
        edits: vec![declaration_edit],
    };
    let candidate_source = apply_edit_atom(source, &edit).map_err(|error| {
        if error.contains("OVERLAPPING")
            || error.contains("INSIDE_CONSUMED")
            || error.contains("DUPLICATE")
        {
            CausalFrontendFailure::conflict(error)
        } else {
            CausalFrontendFailure::unsupported(error)
        }
    })?;
    if candidate_source == source {
        return Err(CausalFrontendFailure::public(
            "DECLARATION_MATERIALIZATION_NO_OP",
        ));
    }
    let replay = apply_edit_atom(source, &edit).map_err(CausalFrontendFailure::conflict)?;
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let candidate_replay_sha256 = sha256(replay.as_bytes());
    if replay != candidate_source || candidate_replay_sha256 != candidate_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "DECLARATION_MATERIALIZATION_DIVERGED",
        ));
    }
    Ok(MaterializedSourceBoundPatchIR {
        predecessor_sha256: sha256(source.as_bytes()),
        edit,
        candidate_source,
        candidate_sha256,
        candidate_replay_sha256,
        candidate_materialization_is_one_to_one: true,
    })
}

fn push_source_seed_expression(
    expression: &TypedSyntaxExpressionIR,
    seen: &mut BTreeSet<String>,
    output: &mut Vec<TypedSyntaxExpressionIR>,
) -> Result<(), CausalFrontendFailure> {
    if output.len() >= 64 {
        return Ok(());
    }
    let key = serde_json::to_string(expression)
        .map_err(|error| CausalFrontendFailure::public(format!("SOURCE_SEED_JSON:{error}")))?;
    if !seen.insert(key) {
        return Ok(());
    }
    output.push(expression.clone());
    match expression {
        TypedSyntaxExpressionIR::Unary { input, .. }
        | TypedSyntaxExpressionIR::StringTransform { input, .. }
        | TypedSyntaxExpressionIR::Length { input } => {
            push_source_seed_expression(input, seen, output)?;
        }
        TypedSyntaxExpressionIR::Binary { left, right, .. } => {
            push_source_seed_expression(left, seen, output)?;
            push_source_seed_expression(right, seen, output)?;
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            push_source_seed_expression(collection, seen, output)?;
            push_source_seed_expression(index, seen, output)?;
        }
        TypedSyntaxExpressionIR::Call { arguments, .. } => {
            for argument in arguments {
                push_source_seed_expression(argument, seen, output)?;
            }
        }
        TypedSyntaxExpressionIR::Operand { .. }
        | TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::BoolLiteral { .. } => {}
    }
    Ok(())
}

fn source_bound_template_seeds(
    template: &SourceBoundFunctionTemplateIR,
) -> Result<Vec<TypedSyntaxExpressionIR>, CausalFrontendFailure> {
    let mut seeds = Vec::new();
    let mut seen = BTreeSet::new();
    for cut in &template.cuts {
        if let Some(condition) = &cut.condition_template {
            push_source_seed_expression(condition, &mut seen, &mut seeds)?;
        }
        if let Some(postimage) = &cut.postimage_template {
            push_source_seed_expression(postimage, &mut seen, &mut seeds)?;
        }
    }
    Ok(seeds)
}

fn source_seed_set_sha256(
    seeds: &[TypedSyntaxExpressionIR],
) -> Result<String, CausalFrontendFailure> {
    serde_json::to_vec(seeds)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| CausalFrontendFailure::public(format!("SOURCE_SEED_HASH:{error}")))
}

fn validate_synthesis_source_seed_binding(
    template: &SourceBoundFunctionTemplateIR,
    synthesis: &TypedMechanismSynthesisReceiptIR,
) -> Result<(), CausalFrontendFailure> {
    let seeds = source_bound_template_seeds(template)?;
    let expected_template = format!("SOURCE_TEMPLATE_SHA256:{}", template.source_template_sha256);
    let expected_seed_hash = format!("SOURCE_SEED_SET_SHA256:{}", source_seed_set_sha256(&seeds)?);
    let expected_seed_count = format!("SOURCE_SEED_COUNT:{}", seeds.len());
    let request = synthesis
        .synthesis_request
        .as_ref()
        .ok_or_else(|| CausalFrontendFailure::conflict("SOURCE_SEED_SYNTHESIS_REQUEST_MISSING"))?;
    let exact_single_binding = |prefix: &str, expected: &str| {
        let mut observed = request
            .provenance
            .iter()
            .filter(|item| item.starts_with(prefix));
        observed.next().is_some_and(|item| item == expected) && observed.next().is_none()
    };
    if !exact_single_binding("SOURCE_TEMPLATE_SHA256:", &expected_template)
        || !exact_single_binding("SOURCE_SEED_SET_SHA256:", &expected_seed_hash)
        || !exact_single_binding("SOURCE_SEED_COUNT:", &expected_seed_count)
    {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_SEED_SYNTHESIS_BINDING",
        ));
    }
    Ok(())
}

fn synthesize_source_bound_template(
    source: &str,
    alternative: &SourceBoundCausalAlternativeIR,
    goal_id: String,
    template: &SourceBoundFunctionTemplateIR,
    observations: Vec<TypedMechanismObservationIR>,
    operator_type_index: &BTreeMap<String, Vec<TypedMechanismImprovementOperatorIR>>,
) -> Result<
    (
        TypedMechanismSynthesisReceiptIR,
        MaterializedSourceBoundPatchIR,
    ),
    CausalFrontendFailure,
> {
    let source_seeds = source_bound_template_seeds(template)?;
    let source_seed_sha256 = source_seed_set_sha256(&source_seeds)?;
    let synthesis_request = TypedMechanismSynthesisGoalIR {
        schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
        goal_id,
        split: DataSplit::FreshBlind,
        operands: template.operands.clone(),
        output_type: template.output_type.clone(),
        definitions: Vec::new(),
        allowed_effects: if alternative.allowed_effects.is_empty() {
            vec![Effect::Pure]
        } else {
            alternative.allowed_effects.clone()
        },
        preconditions: vec![
            format!(
                "exact public symbol {} is source bound",
                alternative.public_symbol
            ),
            format!(
                "closure template {} is execution bound",
                template.qualified_symbol
            ),
        ],
        postconditions: vec!["satisfy all public postimage observations".to_string()],
        invariants: vec![
            "preserve exact public symbol owner".to_string(),
            "preserve same-file execution dependency closure".to_string(),
        ],
        public_observations: observations,
        require_conditional: alternative.require_conditional,
        max_expression_depth: alternative.max_expression_depth,
        max_candidates: alternative.max_candidates,
        provenance: vec![
            "PYTHON_AST_SOURCE_BOUND_CAUSAL_CUT".to_string(),
            format!("SOURCE_TEMPLATE_SHA256:{}", template.source_template_sha256),
            format!("SOURCE_SEED_COUNT:{}", source_seeds.len()),
            format!("SOURCE_SEED_SET_SHA256:{source_seed_sha256}"),
        ],
    };
    let type_key = serde_json::to_string(&(
        template
            .operands
            .iter()
            .map(|operand| operand.value_type.clone())
            .collect::<Vec<_>>(),
        &template.output_type,
    ))
    .map_err(|error| CausalFrontendFailure::public(format!("SOURCE_TYPE_INDEX:{error}")))?;
    let applicable_operators = operator_type_index
        .get(&type_key)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let synthesis = synthesize_typed_mechanism_goal_with_source_seeds_and_priors(
        &synthesis_request,
        &source_seeds,
        applicable_operators,
    )
    .map_err(|error| {
        let detail = format!("BOUNDED_COMPOSITION:{error}");
        if error.starts_with("TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:") {
            CausalFrontendFailure::public(detail)
        } else {
            CausalFrontendFailure::unsupported(detail)
        }
    })?;
    let materialized_patch = materialize_python_synthesis(source, template, &synthesis)?;
    Ok((synthesis, materialized_patch))
}

fn combine_source_bound_patches(
    source: &str,
    patches: &[&ReplayableSourceBoundPatchIR],
    expected_members: usize,
) -> Result<MaterializedSourceBoundPatchIR, CausalFrontendFailure> {
    let predecessor_sha256 = sha256(source.as_bytes());
    if patches.iter().any(|patch| {
        patch.predecessor_sha256 != predecessor_sha256
            || !patch.candidate_materialization_is_one_to_one
            || patch.candidate_sha256 != patch.candidate_replay_sha256
    }) {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_PROPOSAL_PATCH_AUTHORITY_INVALID",
        ));
    }
    let proposals = patches
        .iter()
        .enumerate()
        .map(|(index, patch)| SourceEditProposalIR {
            proposal_id: format!("{}:{index}", patch.candidate_sha256),
            edit: patch.edit.clone(),
        })
        .collect::<Vec<_>>();
    let requirement = if expected_members > 1 {
        SourceProposalCompositionRequirementIR::RequiredGroup {
            group_id: sha256(
                format!("SOURCE_BOUND_REQUIRED_GROUP:{predecessor_sha256}:{expected_members}")
                    .as_bytes(),
            ),
            expected_members,
        }
    } else {
        SourceProposalCompositionRequirementIR::Independent
    };
    let composed =
        compose_source_edit_proposals(source, &proposals, &requirement).map_err(|error| {
            if error.contains("OVERLAPPING")
                || error.contains("INSIDE_CONSUMED")
                || error.contains("DUPLICATE")
                || error.contains("REPLAY")
                || error.contains("REQUIRED_GROUP")
            {
                CausalFrontendFailure::conflict(error)
            } else {
                CausalFrontendFailure::unsupported(error)
            }
        })?;
    Ok(MaterializedSourceBoundPatchIR {
        predecessor_sha256,
        edit: composed.edit,
        candidate_source: composed.candidate_source,
        candidate_sha256: composed.candidate_sha256.clone(),
        candidate_replay_sha256: composed.candidate_sha256,
        candidate_materialization_is_one_to_one: true,
    })
}

fn split_materialized_source_bound_patch(
    patch: MaterializedSourceBoundPatchIR,
) -> (ReplayableSourceBoundPatchIR, String) {
    (
        ReplayableSourceBoundPatchIR {
            predecessor_sha256: patch.predecessor_sha256,
            edit: patch.edit,
            candidate_sha256: patch.candidate_sha256,
            candidate_replay_sha256: patch.candidate_replay_sha256,
            candidate_materialization_is_one_to_one: patch.candidate_materialization_is_one_to_one,
        },
        patch.candidate_source,
    )
}

fn into_replayable_source_bound_patch(
    patch: MaterializedSourceBoundPatchIR,
) -> ReplayableSourceBoundPatchIR {
    split_materialized_source_bound_patch(patch).0
}

pub fn replay_source_bound_patch(
    source: &str,
    patch: &ReplayableSourceBoundPatchIR,
) -> Result<String, CausalFrontendFailure> {
    if sha256(source.as_bytes()) != patch.predecessor_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "PATCH_VARIANT_PREDECESSOR_DIVERGED",
        ));
    }
    if !patch.candidate_materialization_is_one_to_one
        || patch.candidate_sha256 != patch.candidate_replay_sha256
    {
        return Err(CausalFrontendFailure::conflict(
            "PATCH_VARIANT_REPLAY_AUTHORITY_INVALID",
        ));
    }
    let candidate_source = apply_edit_atom(source, &patch.edit).map_err(|error| {
        if error.contains("OVERLAPPING")
            || error.contains("INSIDE_CONSUMED")
            || error.contains("DUPLICATE")
        {
            CausalFrontendFailure::conflict(error)
        } else {
            CausalFrontendFailure::unsupported(error)
        }
    })?;
    if candidate_source == source || sha256(candidate_source.as_bytes()) != patch.candidate_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "PATCH_VARIANT_REPLAY_DIVERGED",
        ));
    }
    Ok(candidate_source)
}

fn source_bound_receipt_patches(
    receipt: &SourceBoundCausalReceiptIR,
) -> Vec<&ReplayableSourceBoundPatchIR> {
    let mut patches = Vec::new();
    for alternative in &receipt.alternatives {
        patches.push(&alternative.replayable_patch);
        patches.extend(
            alternative
                .closure_candidates
                .iter()
                .map(|candidate| &candidate.replayable_patch),
        );
    }
    patches.extend(
        receipt
            .declaration_alternatives
            .iter()
            .map(|alternative| &alternative.replayable_patch),
    );
    patches.extend(
        receipt
            .patch_variants
            .iter()
            .map(|variant| &variant.replayable_patch),
    );
    patches
}

fn qualified_symbol_owner(symbol: &str) -> &str {
    symbol.rsplit_once('.').map_or("", |(owner, _)| owner)
}

fn template_closure_is_preserved(template: &SourceBoundFunctionTemplateIR) -> bool {
    template.execution_dependency_closure.first() == Some(&template.qualified_symbol)
        && template
            .execution_dependency_closure
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == template.execution_dependency_closure.len()
        && template
            .direct_dependencies
            .iter()
            .all(|dependency| template.execution_dependency_closure.contains(dependency))
}

fn closure_evidence_is_an_exact_partition(alternative: &SourceBoundAlternativeReceiptIR) -> bool {
    let closure = &alternative.function_template.execution_dependency_closure;
    if closure.is_empty() {
        return false;
    }
    let mut covered_ordinals = BTreeSet::from([0_usize]);
    let candidates_are_exact = alternative.closure_candidates.iter().all(|candidate| {
        candidate.closure_ordinal > 0
            && closure.get(candidate.closure_ordinal)
                == Some(&candidate.function_template.qualified_symbol)
            && covered_ordinals.insert(candidate.closure_ordinal)
    });
    let rejections_are_exact = alternative
        .closure_candidate_rejections
        .iter()
        .all(|rejection| {
            rejection.closure_ordinal > 0
                && closure.get(rejection.closure_ordinal) == Some(&rejection.qualified_symbol)
                && covered_ordinals.insert(rejection.closure_ordinal)
        });
    candidates_are_exact && rejections_are_exact && covered_ordinals.len() == closure.len()
}

fn source_bound_receipt_claims(receipt: &SourceBoundCausalReceiptIR) -> (bool, bool, bool) {
    let owner_preserved = receipt.alternatives.iter().all(|alternative| {
        alternative.requested_public_symbol == alternative.function_template.qualified_symbol
            && alternative.function_template.owner
                == qualified_symbol_owner(&alternative.function_template.qualified_symbol)
            && alternative.closure_candidates.iter().all(|candidate| {
                candidate.function_template.owner
                    == qualified_symbol_owner(&candidate.function_template.qualified_symbol)
            })
    }) && receipt.declaration_alternatives.iter().all(|alternative| {
        alternative.requested_public_symbol
            == format!(
                "{}.{}",
                alternative.declaration_template.qualified_owner,
                alternative.declaration_template.attribute
            )
    });
    let closure_preserved = receipt.alternatives.iter().all(|alternative| {
        template_closure_is_preserved(&alternative.function_template)
            && closure_evidence_is_an_exact_partition(alternative)
            && alternative
                .closure_candidates
                .iter()
                .all(|candidate| template_closure_is_preserved(&candidate.function_template))
    });
    let atomic_path = source_bound_receipt_patches(receipt)
        .into_iter()
        .all(|patch| {
            patch.predecessor_sha256 == receipt.predecessor_sha256
                && patch.candidate_materialization_is_one_to_one
                && matches!(patch.edit, SourceEditAtom::AtomicMultiEdit { .. })
        });
    (owner_preserved, closure_preserved, atomic_path)
}

fn source_bound_receipt_hash(
    receipt: &SourceBoundCausalReceiptIR,
) -> Result<String, CausalFrontendFailure> {
    let mut canonical = receipt.clone();
    canonical.receipt_sha256.clear();
    serde_json::to_vec(&canonical)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| CausalFrontendFailure::public(format!("RECEIPT_HASH:{error}")))
}

pub fn validate_source_bound_causal_receipt(
    receipt: &SourceBoundCausalReceiptIR,
    source: &str,
) -> Result<(), CausalFrontendFailure> {
    if receipt.schema != SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA
        || receipt.predecessor_sha256 != sha256(source.as_bytes())
        || receipt
            .alternatives
            .len()
            .saturating_add(receipt.declaration_alternatives.len())
            == 0
        || receipt
            .alternatives
            .len()
            .saturating_add(receipt.declaration_alternatives.len())
            > MAX_CAUSAL_ALTERNATIVES
    {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_BOUND_RECEIPT_PREDECESSOR",
        ));
    }
    let (owner, closure, atomic) = source_bound_receipt_claims(receipt);
    if !owner || receipt.public_symbol_owner_preserved != owner {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_BOUND_RECEIPT_OWNER_CLAIM",
        ));
    }
    if !closure || receipt.execution_dependency_closure_preserved != closure {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_BOUND_RECEIPT_CLOSURE_CLAIM",
        ));
    }
    if !atomic || receipt.single_and_multi_edit_share_atomic_path != atomic {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_BOUND_RECEIPT_ATOMIC_PATH_CLAIM",
        ));
    }
    for alternative in &receipt.alternatives {
        validate_source_bound_function_template(source, &alternative.function_template)?;
        validate_synthesis_source_seed_binding(
            &alternative.function_template,
            &alternative.synthesis,
        )?;
        validate_typed_mechanism_synthesis_receipt(&alternative.synthesis).map_err(|error| {
            CausalFrontendFailure::conflict(format!("SOURCE_BOUND_RECEIPT_OWNER_SYNTHESIS:{error}"))
        })?;
        let expected_owner = materialize_python_synthesis(
            source,
            &alternative.function_template,
            &alternative.synthesis,
        )?;
        if into_replayable_source_bound_patch(expected_owner) != alternative.replayable_patch {
            return Err(CausalFrontendFailure::conflict(
                "SOURCE_BOUND_RECEIPT_OWNER_MATERIALIZATION",
            ));
        }
        for candidate in &alternative.closure_candidates {
            validate_source_bound_function_template(source, &candidate.function_template)?;
            validate_synthesis_source_seed_binding(
                &candidate.function_template,
                &candidate.synthesis,
            )?;
            validate_typed_mechanism_synthesis_receipt(&candidate.synthesis).map_err(|error| {
                CausalFrontendFailure::conflict(format!(
                    "SOURCE_BOUND_RECEIPT_CLOSURE_SYNTHESIS:{}:{error}",
                    candidate.closure_ordinal
                ))
            })?;
            let expected_candidate = materialize_python_synthesis(
                source,
                &candidate.function_template,
                &candidate.synthesis,
            )?;
            if into_replayable_source_bound_patch(expected_candidate) != candidate.replayable_patch
            {
                return Err(CausalFrontendFailure::conflict(
                    "SOURCE_BOUND_RECEIPT_CLOSURE_MATERIALIZATION",
                ));
            }
        }
    }
    for declaration in &receipt.declaration_alternatives {
        validate_source_bound_declaration_template(source, &declaration.declaration_template)?;
        let expected = materialize_python_declaration(source, &declaration.declaration_template)?;
        if into_replayable_source_bound_patch(expected) != declaration.replayable_patch {
            return Err(CausalFrontendFailure::conflict(
                "SOURCE_BOUND_RECEIPT_DECLARATION_MATERIALIZATION",
            ));
        }
    }
    if select_source_bound_patch_proposals(
        &receipt.alternatives,
        build_source_bound_patch_variants_with_declarations(
            source,
            &receipt.alternatives,
            &receipt.declaration_alternatives,
        )?,
    )? != receipt.patch_variants
    {
        return Err(CausalFrontendFailure::conflict(
            "SOURCE_BOUND_RECEIPT_VARIANT_MATERIALIZATION",
        ));
    }
    for patch in source_bound_receipt_patches(receipt) {
        replay_source_bound_patch(source, patch)?;
    }
    if receipt.receipt_sha256 != source_bound_receipt_hash(receipt)? {
        return Err(CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_HASH"));
    }
    Ok(())
}

/// Re-derive every accepted Python owner/closure template from the exact
/// predecessor before consuming its source-seed provenance.  The ordinary
/// validator remains language-host independent for sealed artifact audits;
/// installation and canonical synthesis use this stronger boundary.
pub fn validate_source_bound_causal_receipt_with_python(
    receipt: &SourceBoundCausalReceiptIR,
    source: &str,
    python_executable: &Path,
) -> Result<(), CausalFrontendFailure> {
    let requested_symbols = receipt
        .alternatives
        .iter()
        .map(|alternative| alternative.requested_public_symbol.clone())
        .collect::<Vec<_>>();
    if requested_symbols.is_empty() && receipt.declaration_alternatives.is_empty() {
        return Err(CausalFrontendFailure::public(
            "SOURCE_BOUND_RECEIPT_ALTERNATIVES_EMPTY",
        ));
    }
    if !requested_symbols.is_empty() {
        let response = run_python_host(python_executable, source, &requested_symbols)?;
        if let Some(error) = host_failure(&response) {
            return Err(error);
        }
        if response.definitions.len() != receipt.alternatives.len() {
            return Err(CausalFrontendFailure::conflict(
                "SOURCE_BOUND_RECEIPT_PYTHON_AST_CARDINALITY",
            ));
        }
        for (alternative, definition) in receipt.alternatives.iter().zip(&response.definitions) {
            let owner_request = alternative
                .synthesis
                .synthesis_request
                .as_ref()
                .ok_or_else(|| {
                    CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_OWNER_REQUEST_MISSING")
                })?;
            let derived_owner =
                convert_python_definition(definition.clone(), &owner_request.public_observations)?;
            if derived_owner != alternative.function_template {
                return Err(CausalFrontendFailure::conflict(
                    "SOURCE_BOUND_RECEIPT_PYTHON_AST_DERIVATION",
                ));
            }
            for candidate in &alternative.closure_candidates {
                let Some(derived) = definition.closure_templates.iter().find(|derived| {
                    derived.qualified_symbol == candidate.function_template.qualified_symbol
                }) else {
                    return Err(CausalFrontendFailure::conflict(
                        "SOURCE_BOUND_RECEIPT_PYTHON_CLOSURE_DERIVATION",
                    ));
                };
                if derived.public_operand_bindings != candidate.public_operand_bindings {
                    return Err(CausalFrontendFailure::conflict(
                        "SOURCE_BOUND_RECEIPT_PYTHON_CLOSURE_DERIVATION",
                    ));
                }
                let closure_observations = remap_public_observations_for_closure(
                    &owner_request.public_observations,
                    &derived.public_operand_bindings,
                )?;
                let derived_closure =
                    convert_python_closure_definition(derived.clone(), &closure_observations)?;
                if derived_closure != candidate.function_template {
                    return Err(CausalFrontendFailure::conflict(
                        "SOURCE_BOUND_RECEIPT_PYTHON_CLOSURE_DERIVATION",
                    ));
                }
            }
        }
    }
    let declaration_candidates = receipt
        .declaration_alternatives
        .iter()
        .map(|declaration| {
            Ok((
                replay_source_bound_patch(source, &declaration.replayable_patch)?,
                declaration.declaration_template.qualified_owner.as_str(),
                declaration.declaration_template.attribute.as_str(),
                declaration.declaration_template.value_source.as_str(),
            ))
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    let declaration_inputs = declaration_candidates
        .iter()
        .map(|(candidate, owner, attribute, value_source)| {
            (candidate.as_str(), *owner, *attribute, *value_source)
        })
        .collect::<Vec<_>>();
    let (declaration_outcomes, _) =
        validate_python_declaration_candidate_batch(python_executable, &declaration_inputs)?;
    for outcome in declaration_outcomes {
        outcome?;
    }
    validate_source_bound_causal_receipt(receipt, source)
}

fn build_source_bound_function_patch_variants(
    source: &str,
    alternatives: &[SourceBoundAlternativeReceiptIR],
) -> Result<Vec<SourceBoundPatchVariantIR>, CausalFrontendFailure> {
    let mut selections = vec![Vec::<usize>::new()];
    for alternative in alternatives {
        // Prefer deep dependencies used exclusively by this public closure.
        // The public owner comes before shared helpers so an unrelated caller
        // does not force a predictably failing whole-repository validation.
        let mut choices = alternative
            .closure_candidates
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, candidate)| candidate.function_template.external_callers.is_empty())
            .map(|(index, _)| index + 1)
            .collect::<Vec<_>>();
        choices.push(0);
        choices.extend(
            alternative
                .closure_candidates
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, candidate)| !candidate.function_template.external_callers.is_empty())
                .map(|(index, _)| index + 1),
        );
        let mut expanded = Vec::new();
        for prefix in &selections {
            for choice in &choices {
                let mut selection = prefix.clone();
                selection.push(*choice);
                expanded.push(selection);
                if expanded.len() >= MAX_SOURCE_BOUND_PATCH_VARIANTS {
                    break;
                }
            }
            if expanded.len() >= MAX_SOURCE_BOUND_PATCH_VARIANTS {
                break;
            }
        }
        selections = expanded;
    }
    let public_owner_fallback = vec![0; alternatives.len()];
    if !selections.contains(&public_owner_fallback) {
        if selections.len() >= MAX_SOURCE_BOUND_PATCH_VARIANTS {
            selections.pop();
        }
        selections.push(public_owner_fallback);
    }

    let mut variants = Vec::new();
    let mut candidate_hashes = BTreeSet::new();
    let mut saw_conflict = false;
    for selection in selections {
        let mut patches = Vec::new();
        let mut symbols = Vec::new();
        for (alternative, selected) in alternatives.iter().zip(&selection) {
            if *selected == 0 {
                patches.push(&alternative.replayable_patch);
                symbols.push(alternative.function_template.qualified_symbol.clone());
            } else {
                let candidate = alternative
                    .closure_candidates
                    .get(selected - 1)
                    .ok_or_else(|| {
                        CausalFrontendFailure::public("PATCH_VARIANT_CANDIDATE_INDEX")
                    })?;
                patches.push(&candidate.replayable_patch);
                symbols.push(candidate.function_template.qualified_symbol.clone());
            }
        }
        let materialized_patch =
            match combine_source_bound_patches(source, &patches, alternatives.len()) {
                Ok(patch) => patch,
                Err(error)
                    if error.kind == CausalFrontendFailureKind::ConflictingSourceBoundEdits =>
                {
                    saw_conflict = true;
                    continue;
                }
                Err(error) => return Err(error),
            };
        if !candidate_hashes.insert(materialized_patch.candidate_sha256.clone()) {
            continue;
        }
        let variant_id = sha256(
            serde_json::to_vec(&(&selection, &symbols, &materialized_patch.candidate_sha256))
                .map_err(|error| {
                    CausalFrontendFailure::public(format!("PATCH_VARIANT_HASH:{error}"))
                })?
                .as_slice(),
        );
        variants.push(SourceBoundPatchVariantIR {
            variant_id,
            selected_candidate_indices: selection,
            selected_template_symbols: symbols,
            replayable_patch: into_replayable_source_bound_patch(materialized_patch),
        });
    }
    if variants.is_empty() {
        return Err(if saw_conflict {
            CausalFrontendFailure::conflict("ALL_SOURCE_BOUND_PATCH_VARIANTS_CONFLICT")
        } else {
            CausalFrontendFailure::public("NO_SOURCE_BOUND_PATCH_VARIANTS")
        });
    }
    Ok(variants)
}

fn build_source_bound_patch_variants(
    source: &str,
    alternatives: &[SourceBoundAlternativeReceiptIR],
) -> Result<Vec<SourceBoundPatchVariantIR>, CausalFrontendFailure> {
    build_source_bound_patch_variants_with_declarations(source, alternatives, &[])
}

fn build_source_bound_patch_variants_with_declarations(
    source: &str,
    alternatives: &[SourceBoundAlternativeReceiptIR],
    declarations: &[SourceBoundDeclarationAlternativeReceiptIR],
) -> Result<Vec<SourceBoundPatchVariantIR>, CausalFrontendFailure> {
    if declarations.is_empty() {
        return build_source_bound_function_patch_variants(source, alternatives);
    }
    let declaration_patches = declarations
        .iter()
        .map(|declaration| &declaration.replayable_patch)
        .collect::<Vec<_>>();
    let declaration_symbols = declarations
        .iter()
        .map(|declaration| declaration.requested_public_symbol.clone())
        .collect::<Vec<_>>();
    let function_variants = if alternatives.is_empty() {
        Vec::new()
    } else {
        build_source_bound_function_patch_variants(source, alternatives)?
    };
    let mut variants = Vec::new();
    let mut candidate_hashes = BTreeSet::new();
    if function_variants.is_empty() {
        let materialized =
            combine_source_bound_patches(source, &declaration_patches, declarations.len())?;
        let variant_id = sha256(
            serde_json::to_vec(&(
                Vec::<usize>::new(),
                &declaration_symbols,
                &materialized.candidate_sha256,
            ))
            .map_err(|error| {
                CausalFrontendFailure::public(format!("DECLARATION_VARIANT_HASH:{error}"))
            })?
            .as_slice(),
        );
        variants.push(SourceBoundPatchVariantIR {
            variant_id,
            selected_candidate_indices: Vec::new(),
            selected_template_symbols: declaration_symbols,
            replayable_patch: into_replayable_source_bound_patch(materialized),
        });
        return Ok(variants);
    }
    for function_variant in function_variants {
        let mut patches = Vec::with_capacity(1 + declaration_patches.len());
        patches.push(&function_variant.replayable_patch);
        patches.extend(declaration_patches.iter().copied());
        let materialized = combine_source_bound_patches(
            source,
            &patches,
            1_usize.saturating_add(declarations.len()),
        )?;
        if !candidate_hashes.insert(materialized.candidate_sha256.clone()) {
            continue;
        }
        let mut symbols = function_variant.selected_template_symbols;
        symbols.extend(declaration_symbols.iter().cloned());
        let variant_id = sha256(
            serde_json::to_vec(&(
                &function_variant.selected_candidate_indices,
                &symbols,
                &materialized.candidate_sha256,
            ))
            .map_err(|error| {
                CausalFrontendFailure::public(format!("DECLARATION_VARIANT_HASH:{error}"))
            })?
            .as_slice(),
        );
        variants.push(SourceBoundPatchVariantIR {
            variant_id,
            selected_candidate_indices: function_variant.selected_candidate_indices,
            selected_template_symbols: symbols,
            replayable_patch: into_replayable_source_bound_patch(materialized),
        });
    }
    if variants.is_empty() {
        return Err(CausalFrontendFailure::public(
            "NO_SOURCE_BOUND_PATCH_VARIANTS",
        ));
    }
    Ok(variants)
}

fn source_bound_variant_ranking_evidence(
    alternatives: &[SourceBoundAlternativeReceiptIR],
    variant: &SourceBoundPatchVariantIR,
) -> SourceProposalRankingEvidenceIR {
    let mut evidence = SourceProposalRankingEvidenceIR::default();
    for (alternative, selected) in alternatives.iter().zip(&variant.selected_candidate_indices) {
        if *selected == 0 {
            evidence.public_owner_members = evidence.public_owner_members.saturating_add(1);
        } else if let Some(candidate) = alternative.closure_candidates.get(selected - 1) {
            if candidate.function_template.external_callers.is_empty() {
                evidence.source_local_closure_members =
                    evidence.source_local_closure_members.saturating_add(1);
                evidence.source_local_closure_depth = evidence
                    .source_local_closure_depth
                    .saturating_add(candidate.closure_ordinal.min(16));
            }
        }
    }
    evidence
}

fn select_source_bound_patch_proposals(
    alternatives: &[SourceBoundAlternativeReceiptIR],
    variants: Vec<SourceBoundPatchVariantIR>,
) -> Result<Vec<SourceBoundPatchVariantIR>, CausalFrontendFailure> {
    rank_source_proposals(
        variants
            .into_iter()
            .map(|variant| SourceProposalKernelInput {
                proposal_id: variant.variant_id.clone(),
                candidate_sha256: variant.replayable_patch.candidate_sha256.clone(),
                tie_breaker: variant.selected_template_symbols.join(":"),
                evidence: source_bound_variant_ranking_evidence(alternatives, &variant),
                payload: variant,
            })
            .collect(),
    )
    .map_err(CausalFrontendFailure::conflict)
}

fn validate_request(
    request: &SourceBoundCausalRequestIR,
) -> Result<SourceLanguageBackend, CausalFrontendFailure> {
    if request.schema != SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA
        || request.source.is_empty()
        || request.source.len() > MAX_SOURCE_BYTES
        || request.alternatives.is_empty()
        || request.alternatives.len() > MAX_CAUSAL_ALTERNATIVES
    {
        return Err(CausalFrontendFailure::public(
            "SOURCE_BOUND_REQUEST_ENVELOPE",
        ));
    }
    let mut ids = BTreeSet::new();
    for alternative in &request.alternatives {
        if alternative.alternative_id.is_empty()
            || alternative.public_symbol.is_empty()
            || alternative.public_observations.is_empty()
            || !ids.insert(alternative.alternative_id.clone())
        {
            return Err(CausalFrontendFailure::public(
                "SOURCE_BOUND_ALTERNATIVE_PUBLIC_INFORMATION",
            ));
        }
    }
    language_backend_for_path(&request.source_relative_path)
}

pub fn analyze_and_synthesize_source_bound(
    request: &SourceBoundCausalRequestIR,
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    analyze_and_synthesize_source_bound_with_operators(request, &[])
}

pub fn analyze_and_synthesize_source_bound_with_operators(
    request: &SourceBoundCausalRequestIR,
    operators: &[TypedMechanismImprovementOperatorIR],
) -> Result<SourceBoundCausalReceiptIR, CausalFrontendFailure> {
    let backend = validate_request(request)?;
    if backend == SourceLanguageBackend::RustSyn {
        return Err(CausalFrontendFailure::unsupported(
            "RUST_SOURCE_BOUND_CAUSAL_FRONTEND_USES_ACTIVE_SYN_GRAMMAR_PIPELINE",
        ));
    }
    let symbols = request
        .alternatives
        .iter()
        .map(|alternative| alternative.public_symbol.clone())
        .collect::<Vec<_>>();
    let response = run_python_host(&request.python_executable, &request.source, &symbols)?;
    if let Some(failure) = host_failure(&response) {
        return Err(failure);
    }
    if response.definitions.len() != request.alternatives.len() {
        return Err(CausalFrontendFailure::public(
            "PYTHON_HOST_ALTERNATIVE_CARDINALITY",
        ));
    }
    let definitions = response
        .definitions
        .into_iter()
        .map(|definition| (definition.qualified_symbol.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut operator_type_index =
        BTreeMap::<String, Vec<TypedMechanismImprovementOperatorIR>>::new();
    for operator in operators {
        let key = serde_json::to_string(&(&operator.operand_types, &operator.output_type))
            .map_err(|error| {
                CausalFrontendFailure::public(format!("OPERATOR_TYPE_INDEX:{error}"))
            })?;
        operator_type_index
            .entry(key)
            .or_default()
            .push(operator.clone());
    }
    let alternative_worker_count = worker_count_for(
        request.alternatives.len(),
        CAUSAL_ALTERNATIVE_ITEMS_PER_WORKER,
    );
    let receipts = parallel_map_ordered_batched_by(
        &request.alternatives,
        "SOURCE_BOUND_CAUSAL_ALTERNATIVE",
        CAUSAL_ALTERNATIVE_ITEMS_PER_WORKER,
        |alternative| {
            let definition = definitions.get(&alternative.public_symbol).ok_or_else(|| {
                CausalFrontendFailure::public(format!(
                    "EXACT_PUBLIC_SYMBOL_NOT_RETURNED:{}",
                    alternative.public_symbol
                ))
            })?;
            let closure_definitions = definition.closure_templates.clone();
            let mut closure_candidate_rejections = Vec::new();
            for rejection in &definition.closure_rejections {
                let closure_ordinal = definition
                    .execution_dependency_closure
                    .iter()
                    .position(|symbol| symbol == &rejection.qualified_symbol)
                    .ok_or_else(|| {
                        CausalFrontendFailure::public(
                            "CLOSURE_REJECTION_OUTSIDE_EXECUTION_DEPENDENCY_CLOSURE",
                        )
                    })?;
                let failure_kind = failure_kind_from_host_detail(&rejection.detail);
                let detail = if rejection.failure == failure_kind.as_code() {
                    rejection.detail.clone()
                } else {
                    format!(
                        "PYTHON_FAILURE_CLASSIFICATION_MISMATCH:declared={}:observation={}",
                        rejection.failure, rejection.detail
                    )
                };
                closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                    closure_ordinal,
                    qualified_symbol: rejection.qualified_symbol.clone(),
                    failure_kind,
                    detail,
                });
            }
            let function_template =
                convert_python_definition(definition.clone(), &alternative.public_observations)?;
            if function_template.qualified_symbol != alternative.public_symbol
                || function_template.execution_dependency_closure.first()
                    != Some(&alternative.public_symbol)
            {
                return Err(CausalFrontendFailure::public(
                    "PUBLIC_SYMBOL_OR_CLOSURE_IDENTITY_LOST",
                ));
            }
            let (synthesis, materialized_patch) = synthesize_source_bound_template(
                &request.source,
                alternative,
                alternative.alternative_id.clone(),
                &function_template,
                alternative.public_observations.clone(),
                &operator_type_index,
            )?;
            // The owner solution has already been falsified against the exact
            // public observations. Reuse its name-independent typed recipe as
            // an ephemeral prior for this request's dependency templates. It
            // still has to replay against every remapped observation and gains
            // no persistent authority until repository validation succeeds.
            let owner_operator = typed_mechanism_improvement_operator_from_receipt(
                &synthesis,
                synthesis.receipt_sha256.clone(),
            )
            .map_err(|error| {
                CausalFrontendFailure::public(format!("OWNER_EPHEMERAL_OPERATOR:{error}"))
            })?;
            let owner_type_key = serde_json::to_string(&(
                function_template
                    .operands
                    .iter()
                    .map(|operand| operand.value_type.clone())
                    .collect::<Vec<_>>(),
                &function_template.output_type,
            ))
            .map_err(|error| {
                CausalFrontendFailure::public(format!("OWNER_EPHEMERAL_TYPE_KEY:{error}"))
            })?;
            let mut closure_operator_type_index = operator_type_index.clone();
            let owner_type_operators = closure_operator_type_index
                .entry(owner_type_key)
                .or_default();
            if !owner_type_operators
                .iter()
                .any(|operator| operator.operator_id == owner_operator.operator_id)
            {
                owner_type_operators.insert(0, owner_operator);
            }
            let mut pending_closure_candidates = Vec::new();
            for closure_definition in closure_definitions {
                let closure_symbol = closure_definition.qualified_symbol.clone();
                let bindings = closure_definition.public_operand_bindings.clone();
                let closure_ordinal = function_template
                    .execution_dependency_closure
                    .iter()
                    .position(|symbol| symbol == &closure_symbol)
                    .ok_or_else(|| {
                        CausalFrontendFailure::public(
                            "CLOSURE_TEMPLATE_OUTSIDE_EXECUTION_DEPENDENCY_CLOSURE",
                        )
                    })?;
                let remapped_observations = match remap_public_observations_for_closure(
                    &alternative.public_observations,
                    &bindings,
                ) {
                    Ok(observations) => observations,
                    Err(error) => {
                        closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                            closure_ordinal,
                            qualified_symbol: closure_symbol,
                            failure_kind: error.kind,
                            detail: error.detail,
                        });
                        continue;
                    }
                };
                let closure_template = match convert_python_closure_definition(
                    closure_definition,
                    &remapped_observations,
                ) {
                    Ok(template) => template,
                    Err(error) => {
                        closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                            closure_ordinal,
                            qualified_symbol: closure_symbol,
                            failure_kind: error.kind,
                            detail: error.detail,
                        });
                        continue;
                    }
                };
                let closure_result = synthesize_source_bound_template(
                    &request.source,
                    alternative,
                    format!("{}:CLOSURE:{}", alternative.alternative_id, closure_ordinal),
                    &closure_template,
                    remapped_observations,
                    &closure_operator_type_index,
                );
                let (closure_synthesis, closure_patch) = match closure_result {
                    Ok(result) => result,
                    Err(error) => {
                        closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                            closure_ordinal,
                            qualified_symbol: closure_template.qualified_symbol.clone(),
                            failure_kind: error.kind,
                            detail: error.detail,
                        });
                        continue;
                    }
                };
                let (replayable_patch, candidate_source) =
                    split_materialized_source_bound_patch(closure_patch);
                pending_closure_candidates.push((
                    SourceBoundClosureCandidateReceiptIR {
                        closure_ordinal,
                        public_operand_bindings: bindings,
                        function_template: closure_template,
                        synthesis: closure_synthesis,
                        replayable_patch,
                    },
                    candidate_source,
                ));
            }
            // Every materialization still has to parse, compile, and preserve
            // the exact public symbol.  Validate the owner and closure-local
            // alternatives in bounded batches so one source-bound cut does
            // not spawn one identical Python frontend process per candidate.
            let mut validation_inputs = Vec::with_capacity(1 + pending_closure_candidates.len());
            validation_inputs.push((
                materialized_patch.candidate_source.as_str(),
                alternative.public_symbol.as_str(),
            ));
            validation_inputs.extend(pending_closure_candidates.iter().map(
                |(_, candidate_source)| {
                    (
                        candidate_source.as_str(),
                        alternative.public_symbol.as_str(),
                    )
                },
            ));
            let (validation_outcomes, candidate_validation_processes) =
                validate_python_candidate_batch(&request.python_executable, &validation_inputs)?;
            drop(validation_inputs);
            let mut validation_outcomes = validation_outcomes.into_iter();
            validation_outcomes.next().ok_or_else(|| {
                CausalFrontendFailure::public("CANDIDATE_BATCH_OWNER_OUTCOME_MISSING")
            })??;
            let mut closure_candidates: Vec<SourceBoundClosureCandidateReceiptIR> =
                pending_closure_candidates
                    .into_iter()
                    .zip(validation_outcomes)
                    .filter_map(|((candidate, _candidate_source), outcome)| match outcome {
                        Ok(()) => Some(candidate),
                        Err(error) => {
                            closure_candidate_rejections.push(
                                SourceBoundClosureCandidateRejectionIR {
                                    closure_ordinal: candidate.closure_ordinal,
                                    qualified_symbol: candidate.function_template.qualified_symbol,
                                    failure_kind: error.kind,
                                    detail: error.detail,
                                },
                            );
                            None
                        }
                    })
                    .collect();
            closure_candidates.sort_by_key(|candidate| candidate.closure_ordinal);
            closure_candidate_rejections.sort_by(|left, right| {
                left.closure_ordinal
                    .cmp(&right.closure_ordinal)
                    .then_with(|| left.qualified_symbol.cmp(&right.qualified_symbol))
            });
            Ok(SourceBoundAlternativeReceiptIR {
                alternative_id: alternative.alternative_id.clone(),
                requested_public_symbol: alternative.public_symbol.clone(),
                function_template,
                synthesis,
                replayable_patch: into_replayable_source_bound_patch(materialized_patch),
                closure_candidates,
                closure_candidate_rejections,
                candidate_validation_processes,
            })
        },
        |detail| CausalFrontendFailure::unsupported(format!("PARALLEL_EXECUTOR:{detail}")),
    )?;
    let patch_variants = select_source_bound_patch_proposals(
        &receipts,
        build_source_bound_patch_variants(&request.source, &receipts)?,
    )?;
    let mut receipt = SourceBoundCausalReceiptIR {
        schema: SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA.to_string(),
        source_relative_path: request.source_relative_path.clone(),
        language_backend: backend,
        predecessor_sha256: sha256(request.source.as_bytes()),
        alternatives: receipts,
        declaration_alternatives: Vec::new(),
        patch_variants,
        alternative_worker_count,
        public_symbol_owner_preserved: false,
        execution_dependency_closure_preserved: false,
        single_and_multi_edit_share_atomic_path: false,
        receipt_sha256: String::new(),
    };
    let (owner, closure, atomic) = source_bound_receipt_claims(&receipt);
    receipt.public_symbol_owner_preserved = owner;
    receipt.execution_dependency_closure_preserved = closure;
    receipt.single_and_multi_edit_share_atomic_path = atomic;
    receipt.receipt_sha256 = source_bound_receipt_hash(&receipt)?;
    validate_source_bound_causal_receipt_with_python(
        &receipt,
        &request.source,
        &request.python_executable,
    )?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem5::model::Value;
    use crate::source_proposal_kernel::MAX_SELECTED_SOURCE_PROPOSALS;

    fn python() -> Option<PathBuf> {
        let output = Command::new("python")
            .arg("-c")
            .arg("import sys;print(sys.executable)")
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
    }

    fn repository_fixture(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "b-core-path-discovery-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("product")).unwrap();
        fs::create_dir_all(root.join("checks")).unwrap();
        root
    }

    #[test]
    fn repository_path_api_derives_a_goal_from_fresh_real_files() {
        let Some(python_executable) = python() else {
            return;
        };
        let root = repository_fixture("fresh-names");
        let source_path = PathBuf::from("product/renamed_math.py");
        let test_path = PathBuf::from("checks/test_renamed_math.py");
        fs::write(
            root.join(&source_path),
            "def merge_quantities(alpha: int, beta: int) -> int:\n    return 0\n",
        )
        .unwrap();
        fs::write(
            root.join(&test_path),
            concat!(
                "from product.renamed_math import merge_quantities\n\n",
                "def test_merge_quantities():\n",
                "    assert merge_quantities(2, 3) == 5\n",
                "    assert merge_quantities(-4, 1) == -3\n",
            ),
        )
        .unwrap();
        let receipt = discover_and_synthesize_python_repository_paths(
            &SourceBoundRepositoryPathDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA.to_string(),
                repository_root: root.clone(),
                source_relative_path: source_path,
                test_relative_paths: vec![test_path],
                python_executable,
                target_symbols: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 2,
                max_candidates: 1_024,
            },
        )
        .unwrap();
        assert_eq!(receipt.alternatives.len(), 1);
        assert_eq!(
            receipt.alternatives[0].requested_public_symbol,
            "merge_quantities"
        );
        let candidate = replay_source_bound_patch(
            &fs::read_to_string(root.join("product/renamed_math.py")).unwrap(),
            &receipt.patch_variants[0].replayable_patch,
        )
        .unwrap();
        assert!(candidate.contains("alpha"));
        assert!(candidate.contains("beta"));
        assert!(!candidate.contains("return 0"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn repository_path_api_rejects_targets_outside_the_runtime_root() {
        let Some(python_executable) = python() else {
            return;
        };
        let root = repository_fixture("outside-root");
        let error = discover_and_synthesize_python_repository_paths(
            &SourceBoundRepositoryPathDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA.to_string(),
                repository_root: root.clone(),
                source_relative_path: PathBuf::from("../outside.py"),
                test_relative_paths: vec![PathBuf::from("checks/test_any.py")],
                python_executable,
                target_symbols: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 2,
                max_candidates: 1_024,
            },
        )
        .unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        fs::remove_dir_all(&root).unwrap();
    }

    fn observations(cases: &[(i64, i64, i64)]) -> Vec<TypedMechanismObservationIR> {
        cases
            .iter()
            .map(|(left, right, expected)| TypedMechanismObservationIR {
                operands: BTreeMap::from([
                    ("left".to_string(), Value::Int(*left)),
                    ("right".to_string(), Value::Int(*right)),
                ]),
                expected_postimage: Value::Int(*expected),
            })
            .collect()
    }

    fn relation(left: &str, right: &str) -> PredicateRelationBindingIR {
        PredicateRelationBindingIR {
            left_source: left.to_string(),
            right_source: right.to_string(),
            value_type: ProgramType::String,
        }
    }

    fn refinement_request(
        path: &str,
        source: &str,
        predicate: &str,
        identity: PredicateRelationBindingIR,
        receiver: PredicateRelationBindingIR,
        owner: PredicateRelationBindingIR,
        python_executable: Option<PathBuf>,
    ) -> CallIdentityPredicateRefinementIR {
        let start = source.find(predicate).expect("predicate exists");
        CallIdentityPredicateRefinementIR {
            schema: CALL_IDENTITY_PREDICATE_REFINEMENT_SCHEMA.to_string(),
            source_relative_path: PathBuf::from(path),
            source: source.to_string(),
            predicate_range: ByteRange {
                start,
                end: start + predicate.len(),
            },
            identity,
            receiver,
            owner,
            python_executable,
        }
    }

    #[test]
    fn call_identity_predicate_refinement_lowers_to_exact_rust_ast_edit() {
        let source = r#"pub struct Invocation {
    pub token: String,
    pub endpoint: String,
    pub namespace: String,
}

pub fn same_call(left: &Invocation, right: &Invocation) -> bool {
    left.token == right.token
}
"#;
        let request = refinement_request(
            "src/lib.rs",
            source,
            "left.token == right.token",
            relation("left.token", "right.token"),
            relation("left.endpoint", "right.endpoint"),
            relation("left.namespace", "right.namespace"),
            None,
        );

        let receipt = lower_call_identity_predicate_refinement(&request).unwrap();

        assert_eq!(receipt.language_backend, SourceLanguageBackend::RustSyn);
        assert!(receipt.receiver_root_relation_preserved);
        assert!(receipt.owner_root_relation_preserved);
        assert!(receipt.original_identity_predicate_replaced);
        assert!(receipt
            .materialized_patch
            .candidate_source
            .contains("left.endpoint == right.endpoint"));
        assert!(receipt
            .materialized_patch
            .candidate_source
            .contains("left.namespace == right.namespace"));
        validate_predicate_refinement_lowering_receipt(&receipt, source).unwrap();
        let mut tampered = receipt.clone();
        tampered.owner_root_relation_preserved = false;
        let error = validate_predicate_refinement_lowering_receipt(&tampered, source).unwrap_err();
        assert_eq!(error.detail, "PREDICATE_REFINEMENT_RECEIPT_CONTRACT");
        let program = receipt.rust_structural_repair_program.unwrap();
        assert_eq!(
            apply_edit_atom(source, &program.edit).unwrap(),
            receipt.materialized_patch.candidate_source
        );
        assert_eq!(receipt.concrete_template.source_operands.len(), 6);
        assert!(receipt.concrete_template.type_effect_check_pass);
    }

    #[test]
    fn predicate_refinement_is_name_independent_but_rejects_cross_receiver_binding() {
        let source = "pub fn equivalent(a: &Node, b: &Node, c: &Node) -> bool { a.signature == b.signature }\n";
        let renamed = refinement_request(
            "src/model.rs",
            source,
            "a.signature == b.signature",
            relation("a.signature", "b.signature"),
            relation("a.host", "b.host"),
            relation("a.scope", "b.scope"),
            None,
        );
        let receipt = lower_call_identity_predicate_refinement(&renamed).unwrap();
        assert!(receipt
            .materialized_patch
            .candidate_source
            .contains("a.host == b.host"));

        let forged = refinement_request(
            "src/model.rs",
            source,
            "a.signature == b.signature",
            relation("a.signature", "b.signature"),
            relation("c.host", "b.host"),
            relation("a.scope", "b.scope"),
            None,
        );
        let error = lower_call_identity_predicate_refinement(&forged).unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::ConflictingSourceBoundEdits
        );
        assert_eq!(error.detail, "PREDICATE_RECEIVER_OWNER_ROOT_RELATION");
    }

    #[test]
    fn python_backend_uses_the_same_typed_predicate_goal_and_atomic_edit_path() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "def same_call(left, right):\n    return left.key == right.key\n";
        let request = refinement_request(
            "src/model.py",
            source,
            "left.key == right.key",
            relation("left.key", "right.key"),
            relation("left.receiver", "right.receiver"),
            relation("left.owner", "right.owner"),
            Some(python_executable),
        );

        let receipt = lower_call_identity_predicate_refinement(&request).unwrap();

        assert_eq!(receipt.language_backend, SourceLanguageBackend::PythonAst);
        assert!(receipt.rust_structural_repair_program.is_none());
        assert!(receipt
            .materialized_patch
            .candidate_source
            .contains("(left.receiver) == (right.receiver)"));
        assert!(receipt
            .materialized_patch
            .candidate_source
            .contains(" and "));
        assert_eq!(
            apply_edit_atom(&request.source, &receipt.materialized_patch.edit).unwrap(),
            receipt.materialized_patch.candidate_source
        );
    }

    #[test]
    fn candidate_batch_reports_item_failures_without_reparsing_valid_siblings() {
        let Some(python_executable) = python() else {
            return;
        };
        let valid = "def target() -> int:\n    return 1\n";
        let invalid = "def target(:\n    return 1\n";
        let (outcomes, processes) = validate_python_candidate_batch(
            &python_executable,
            &[(valid, "target"), (invalid, "target")],
        )
        .unwrap();
        assert_eq!(processes, 1);
        assert!(outcomes[0].is_ok());
        let failure = outcomes[1].as_ref().unwrap_err();
        assert_eq!(
            failure.kind,
            CausalFrontendFailureKind::UnsupportedLanguageSyntax
        );
        assert!(failure.detail.starts_with("CANDIDATE_PARSE:"));
    }

    #[test]
    fn rust_kernel_owns_python_host_failure_classification() {
        let parse = classified_host_failure(
            Some("PUBLIC_INFORMATION_INSUFFICIENT"),
            Some("CANDIDATE_PARSE:invalid syntax"),
        );
        assert_eq!(
            parse.kind,
            CausalFrontendFailureKind::UnsupportedLanguageSyntax
        );
        assert!(parse
            .detail
            .starts_with("PYTHON_FAILURE_CLASSIFICATION_MISMATCH:"));

        let missing = classified_host_failure(
            Some("UNSUPPORTED_LANGUAGE_SYNTAX"),
            Some("EXACT_PUBLIC_SYMBOL_NOT_FOUND:pkg.target"),
        );
        assert_eq!(
            missing.kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert!(missing
            .detail
            .starts_with("PYTHON_FAILURE_CLASSIFICATION_MISMATCH:"));
    }

    #[test]
    fn declaration_candidate_batch_requires_the_exact_public_postimage() {
        let Some(python_executable) = python() else {
            return;
        };
        let correct = "class Policy:\n    marker = 'ready'\n";
        let wrong = "class Policy:\n    marker = 'blocked'\n";
        let (outcomes, processes) = validate_python_declaration_candidate_batch(
            &python_executable,
            &[
                (correct, "Policy", "marker", "'ready'"),
                (wrong, "Policy", "marker", "'ready'"),
            ],
        )
        .unwrap();
        assert_eq!(processes, 1);
        assert!(outcomes[0].is_ok());
        assert_eq!(
            outcomes[1].as_ref().unwrap_err(),
            &CausalFrontendFailure::public(
                "MATERIALIZED_CLASS_DECLARATION_POSTIMAGE_MISMATCH:Policy.marker"
            )
        );
    }

    #[test]
    fn independent_causal_alternatives_run_as_a_bounded_ordered_graph() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = (0..5)
            .map(|ordinal| {
                format!("def repair_{ordinal}(left: int, right: int) -> int:\n    return 0\n")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let alternatives = (0..5)
            .map(|ordinal| SourceBoundCausalAlternativeIR {
                alternative_id: format!("alternative-{ordinal}"),
                public_symbol: format!("repair_{ordinal}"),
                public_observations: observations(&[(2, 3, 5), (-4, 9, 5)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
            })
            .collect::<Vec<_>>();
        let receipt = analyze_and_synthesize_source_bound(&SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("repairs.py"),
            source,
            python_executable,
            alternatives,
        })
        .unwrap();
        assert_eq!(
            receipt.alternative_worker_count,
            worker_count_for(5, CAUSAL_ALTERNATIVE_ITEMS_PER_WORKER)
        );
        assert_eq!(
            receipt
                .alternatives
                .iter()
                .map(|alternative| alternative.alternative_id.as_str())
                .collect::<Vec<_>>(),
            [
                "alternative-0",
                "alternative-1",
                "alternative-2",
                "alternative-3",
                "alternative-4"
            ]
        );
        assert!(receipt.alternatives.iter().all(|alternative| {
            matches!(
                alternative.synthesis.winning_goal.postimage,
                TypedSyntaxExpressionIR::Binary {
                    operator: BinaryOperator::Add,
                    ..
                }
            ) && alternative
                .replayable_patch
                .candidate_materialization_is_one_to_one
        }));
    }

    #[test]
    fn repository_collection_literals_reach_typed_operand_repair() {
        assert_eq!(
            map_python_type("typing.List[ List[int] ]"),
            Some(ProgramType::NestedSequenceInt)
        );
        assert_eq!(
            map_python_type("list[typing.Sequence[int]]"),
            Some(ProgramType::NestedSequenceInt)
        );
        assert_eq!(map_python_type("list[list[list[int]]]"), None);
        assert_eq!(map_python_type("list[str]"), None);
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def keep_flat(values: list[int]) -> list[int]:
    return []

def keep_nested(values: list[list[int]]) -> list[list[int]]:
    return []

def keep_bytes(values: bytes) -> bytes:
    return b""
"#;
        let tests = r#"def test_collections():
    assert keep_flat([1, 2]) == [1, 2]
    assert keep_flat([-3, 4]) == [-3, 4]
    assert keep_nested([[1], [2, 3]]) == [[1], [2, 3]]
    assert keep_nested([[4, 5]]) == [[4, 5]]
    assert keep_bytes(b"ab") == b"ab"
    assert keep_bytes(b"xyz") == b"xyz"
"#;
        let receipt =
            discover_and_synthesize_python_repository(&SourceBoundRepositoryDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
                source_relative_path: PathBuf::from("collections.py"),
                source: source.to_string(),
                test_sources: vec![RepositoryTestSourceIR {
                    relative_path: PathBuf::from("tests/test_collections.py"),
                    source: tests.to_string(),
                }],
                python_executable,
                target_symbols: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 2,
                max_candidates: 1_024,
            })
            .unwrap();
        let by_symbol = receipt
            .alternatives
            .iter()
            .map(|alternative| (alternative.requested_public_symbol.as_str(), alternative))
            .collect::<BTreeMap<_, _>>();
        for (symbol, expected_type) in [
            ("keep_flat", ProgramType::SequenceInt),
            ("keep_nested", ProgramType::NestedSequenceInt),
            ("keep_bytes", ProgramType::Bytes),
        ] {
            let alternative = by_symbol.get(symbol).expect("collection alternative");
            assert_eq!(
                alternative.function_template.operands[0].value_type,
                expected_type
            );
            assert_eq!(alternative.function_template.output_type, expected_type);
            assert!(matches!(
                alternative.synthesis.winning_goal.postimage,
                TypedSyntaxExpressionIR::Operand { .. }
            ));
            assert!(
                alternative
                    .replayable_patch
                    .candidate_materialization_is_one_to_one
            );
        }
    }

    #[test]
    fn repository_collections_synthesize_length_and_index_primitives() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def count(values: list[int]) -> int:
    return 0

def first(values: list[int]) -> int:
    return 0

def byte_at(values: bytes, position: int) -> int:
    return 0

def row_at(values: list[list[int]], position: int) -> list[int]:
    return []
"#;
        let tests = r#"def test_collection_primitives():
    assert count([1, 2]) == 2
    assert count([7]) == 1
    assert first([4, 9]) == 4
    assert first([2, 3]) == 2
    assert byte_at(b"ab", 1) == 98
    assert byte_at(b"xyz", 1) == 121
    assert byte_at(b"ab", 0) == 97
    assert byte_at(b"xyz", 2) == 122
    assert row_at([[1], [2, 3]], 1) == [2, 3]
    assert row_at([[4, 5], [6]], 0) == [4, 5]
"#;
        let receipt =
            discover_and_synthesize_python_repository(&SourceBoundRepositoryDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
                source_relative_path: PathBuf::from("collection_primitives.py"),
                source: source.to_string(),
                test_sources: vec![RepositoryTestSourceIR {
                    relative_path: PathBuf::from("tests/test_collection_primitives.py"),
                    source: tests.to_string(),
                }],
                python_executable,
                target_symbols: ["count", "first", "byte_at", "row_at"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 2,
                max_candidates: 1_024,
            })
            .unwrap();
        let by_symbol = receipt
            .alternatives
            .iter()
            .map(|alternative| (alternative.requested_public_symbol.as_str(), alternative))
            .collect::<BTreeMap<_, _>>();
        assert!(matches!(
            by_symbol["count"].synthesis.winning_goal.postimage,
            TypedSyntaxExpressionIR::Length { .. }
        ));
        assert!(
            replay_source_bound_patch(source, &by_symbol["count"].replayable_patch)
                .unwrap()
                .contains("len(")
        );
        for symbol in ["first", "byte_at", "row_at"] {
            let alternative = by_symbol[symbol];
            assert!(matches!(
                alternative.synthesis.winning_goal.postimage,
                TypedSyntaxExpressionIR::Index { .. }
            ));
            assert_eq!(
                alternative.synthesis.template.public_observations_passed,
                alternative.synthesis.winning_goal.public_observations.len()
            );
            assert!(
                replay_source_bound_patch(source, &alternative.replayable_patch)
                    .unwrap()
                    .contains('[')
            );
            assert!(
                alternative
                    .replayable_patch
                    .candidate_materialization_is_one_to_one
            );
        }
        assert_eq!(
            by_symbol["row_at"].function_template.output_type,
            ProgramType::SequenceInt
        );
    }

    #[test]
    fn repository_strings_reach_language_neutral_concat_length_and_unicode_index() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def combine(left: str, right: str) -> str:
    return right + left

def width(value: str) -> int:
    return 0

def first(value: str) -> str:
    return ""
"#;
        let tests = r#"def test_strings():
    assert combine("ab", "cd") == "abcd"
    assert combine("x", "yz") == "xyz"
    assert width("hello") == 5
    assert width("한글") == 2
    assert first("alpha") == "a"
    assert first("βeta") == "β"
"#;
        let receipt =
            discover_and_synthesize_python_repository(&SourceBoundRepositoryDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
                source_relative_path: PathBuf::from("strings.py"),
                source: source.to_string(),
                test_sources: vec![RepositoryTestSourceIR {
                    relative_path: PathBuf::from("tests/test_strings.py"),
                    source: tests.to_string(),
                }],
                python_executable: python_executable.clone(),
                target_symbols: ["combine", "width", "first"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 2,
                max_candidates: 1_024,
            })
            .unwrap();
        let by_symbol = receipt
            .alternatives
            .iter()
            .map(|alternative| (alternative.requested_public_symbol.as_str(), alternative))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            by_symbol["combine"].function_template.output_type,
            ProgramType::String
        );
        assert!(matches!(
            by_symbol["combine"].synthesis.winning_goal.postimage,
            TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                ..
            }
        ));
        assert!(matches!(
            by_symbol["width"].synthesis.winning_goal.postimage,
            TypedSyntaxExpressionIR::Length { .. }
        ));
        assert!(matches!(
            by_symbol["first"].synthesis.winning_goal.postimage,
            TypedSyntaxExpressionIR::Index { .. }
        ));
        assert!(
            replay_source_bound_patch(source, &by_symbol["combine"].replayable_patch)
                .unwrap()
                .contains("(left) + (right)")
        );
        assert!(
            replay_source_bound_patch(source, &by_symbol["width"].replayable_patch)
                .unwrap()
                .contains("len((value))")
        );
        assert!(
            replay_source_bound_patch(source, &by_symbol["first"].replayable_patch)
                .unwrap()
                .contains("[0]")
        );
        let combined =
            replay_source_bound_patch(source, &receipt.patch_variants[0].replayable_patch).unwrap();
        let execution = Command::new(python_executable)
            .args(["-X", "utf8", "-c"])
            .arg(format!("{combined}\n{tests}\ntest_strings()\n"))
            .output()
            .unwrap();
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
    }

    #[test]
    fn repository_string_normalization_reaches_source_bound_executable_templates() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def clean(value: str) -> str:
    return value

def lowercase(value: str) -> str:
    return value

def uppercase(value: str) -> str:
    return value
"#;
        let tests = r#"def test_normalization():
    assert clean("  Alpha ") == "Alpha"
    assert clean("\tBeta\n") == "Beta"
    assert lowercase("Alpha") == "alpha"
    assert lowercase("MiXeD") == "mixed"
    assert uppercase("Alpha") == "ALPHA"
    assert uppercase("mixed") == "MIXED"
"#;
        let receipt =
            discover_and_synthesize_python_repository(&SourceBoundRepositoryDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
                source_relative_path: PathBuf::from("normalization.py"),
                source: source.to_string(),
                test_sources: vec![RepositoryTestSourceIR {
                    relative_path: PathBuf::from("tests/test_normalization.py"),
                    source: tests.to_string(),
                }],
                python_executable: python_executable.clone(),
                target_symbols: ["clean", "lowercase", "uppercase"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 1,
                max_candidates: 64,
            })
            .unwrap();
        let by_symbol = receipt
            .alternatives
            .iter()
            .map(|alternative| (alternative.requested_public_symbol.as_str(), alternative))
            .collect::<BTreeMap<_, _>>();
        for (symbol, expected_operator, source_token) in [
            ("clean", StringTransformOperator::Trim, ".strip()"),
            ("lowercase", StringTransformOperator::Lowercase, ".lower()"),
            ("uppercase", StringTransformOperator::Uppercase, ".upper()"),
        ] {
            assert!(matches!(
                by_symbol[symbol].synthesis.winning_goal.postimage,
                TypedSyntaxExpressionIR::StringTransform { operator, .. }
                    if operator == expected_operator
            ));
            assert!(
                replay_source_bound_patch(source, &by_symbol[symbol].replayable_patch)
                    .unwrap()
                    .contains(source_token)
            );
        }
        let combined =
            replay_source_bound_patch(source, &receipt.patch_variants[0].replayable_patch).unwrap();
        let execution = Command::new(python_executable)
            .args(["-X", "utf8", "-c"])
            .arg(format!("{combined}\n{tests}\ntest_normalization()\n"))
            .output()
            .unwrap();
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
    }

    #[test]
    fn correct_string_normalization_is_observed_without_inventing_a_repair() {
        let Some(python_executable) = python() else {
            return;
        };
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("normalization.py"),
            source: r#"def clean(value: str) -> str:
    return value.strip()

def lowercase(value: str) -> str:
    return value.lower()

def uppercase(value: str) -> str:
    return value.upper()
"#
            .to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_normalization.py"),
                source: r#"def test_normalization():
    assert clean("  Alpha ") == "Alpha"
    assert lowercase("MiXeD") == "mixed"
    assert uppercase("Alpha") == "ALPHA"
"#
                .to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 64,
        };
        let error = discover_and_synthesize_python_repository(&request).unwrap_err();
        assert_eq!(
            error,
            CausalFrontendFailure::public("NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
        );
    }

    #[test]
    fn repository_boundary_relations_reach_atomic_executable_templates() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def different(left: int, right: int) -> bool:
    return False

def within(value: int, limit: int) -> bool:
    return value < limit

def at_least(value: int, floor: int) -> bool:
    return value > floor

def lexical_within(value: str, limit: str) -> bool:
    return value < limit
"#;
        let tests = r#"def test_relations():
    assert different(1, 2)
    assert different(2, 1)
    assert not different(2, 2)
    assert not different(7, 7)
    assert different(5, 9)
    assert within(1, 2)
    assert within(5, 5)
    assert not within(7, 3)
    assert within(-2, -1)
    assert not within(4, 2)
    assert at_least(2, 1)
    assert at_least(5, 5)
    assert not at_least(3, 7)
    assert at_least(-1, -2)
    assert not at_least(2, 4)
    assert lexical_within("alpha", "beta")
    assert lexical_within("beta", "beta")
    assert not lexical_within("gamma", "beta")
"#;
        let receipt =
            discover_and_synthesize_python_repository(&SourceBoundRepositoryDiscoveryRequestIR {
                schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
                source_relative_path: PathBuf::from("relations.py"),
                source: source.to_string(),
                test_sources: vec![RepositoryTestSourceIR {
                    relative_path: PathBuf::from("tests/test_relations.py"),
                    source: tests.to_string(),
                }],
                python_executable: python_executable.clone(),
                target_symbols: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                max_expression_depth: 1,
                max_candidates: 1_024,
            })
            .unwrap();
        let by_symbol = receipt
            .alternatives
            .iter()
            .map(|alternative| (alternative.requested_public_symbol.as_str(), alternative))
            .collect::<BTreeMap<_, _>>();
        for symbol in ["different", "within", "at_least", "lexical_within"] {
            let selected = &by_symbol[symbol].synthesis.winning_goal.postimage;
            assert!(
                matches!(
                    selected,
                    TypedSyntaxExpressionIR::Binary {
                        operator: BinaryOperator::NotEqual
                            | BinaryOperator::LessThanOrEqual
                            | BinaryOperator::GreaterThanOrEqual,
                        ..
                    }
                ),
                "symbol={symbol} selected={selected:?}"
            );
            let replayed =
                replay_source_bound_patch(source, &by_symbol[symbol].replayable_patch).unwrap();
            assert!(
                replayed.contains(" != ") || replayed.contains(" <= ") || replayed.contains(" >= ")
            );
        }
        let combined =
            replay_source_bound_patch(source, &receipt.patch_variants[0].replayable_patch).unwrap();
        let execution = Command::new(python_executable)
            .args(["-X", "utf8", "-c"])
            .arg(format!("{combined}\n{tests}\ntest_relations()\n"))
            .output()
            .unwrap();
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
    }

    #[test]
    fn correct_boundary_relations_do_not_generate_repairs() {
        let Some(python_executable) = python() else {
            return;
        };
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("relations.py"),
            source: r#"def different(left: int, right: int) -> bool:
    return left != right

def within(value: int, limit: int) -> bool:
    return value <= limit

def at_least(value: int, floor: int) -> bool:
    return value >= floor

def lexical_within(value: str, limit: str) -> bool:
    return value <= limit
"#
            .to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_relations.py"),
                source: r#"def test_relations():
    assert different(1, 2)
    assert not different(2, 2)
    assert not different(7, 7)
    assert within(5, 5)
    assert not within(7, 3)
    assert within(-2, -1)
    assert at_least(5, 5)
    assert not at_least(3, 7)
    assert at_least(-1, -2)
    assert lexical_within("alpha", "beta")
    assert lexical_within("beta", "beta")
    assert not lexical_within("gamma", "beta")
"#
                .to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 1_024,
        };
        assert_eq!(
            discover_and_synthesize_python_repository(&request).unwrap_err(),
            CausalFrontendFailure::public("NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
        );
    }

    #[test]
    fn sparse_boundary_evidence_is_classified_as_public_information_insufficient() {
        let Some(python_executable) = python() else {
            return;
        };
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("sparse_relation.py"),
            source: "def at_least(value: int, floor: int) -> bool:\n    return value > floor\n"
                .to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_sparse_relation.py"),
                source: r#"def test_sparse_relation():
    assert at_least(2, 1)
    assert at_least(2, 2)
    assert not at_least(1, 2)
"#
                .to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 1_024,
        };
        let error = discover_and_synthesize_python_repository(&request).unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert!(
            error.detail.starts_with(
                "BOUNDED_COMPOSITION:TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_HYPOTHESES:"
            ),
            "{}",
            error.detail
        );
    }

    #[test]
    fn sparse_conditional_evidence_preserves_public_information_failure() {
        let Some(python_executable) = python() else {
            return;
        };
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("sparse_conditional.py"),
            source: "def classify(left: int, right: int) -> int:\n    return 0\n".to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "sparse-conditional".to_string(),
                public_symbol: "classify".to_string(),
                public_observations: observations(&[(2, 1, 1), (2, 2, 1), (1, 2, -1)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 1,
                max_candidates: 1_024,
            }],
        };
        let error = analyze_and_synthesize_source_bound(&request).unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert!(
            error.detail.starts_with(
                "BOUNDED_COMPOSITION:TYPED_MECHANISM_PUBLIC_INFORMATION_INSUFFICIENT:MINIMAL_CONDITIONAL_HYPOTHESES:"
            ),
            "{}",
            error.detail
        );
    }

    #[test]
    fn python_source_subexpression_seeds_the_common_bounded_kernel() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "def area(left: int, right: int) -> int:\n    return ((left + right) * (left - right)) + 1\n";
        let public_observations = [(5_i64, 2_i64), (4, 1), (3, -2), (-4, 1)]
            .into_iter()
            .map(|(left, right)| TypedMechanismObservationIR {
                operands: BTreeMap::from([
                    ("left".to_string(), Value::Int(left)),
                    ("right".to_string(), Value::Int(right)),
                ]),
                expected_postimage: Value::Int((left + right) * (left - right)),
            })
            .collect();
        let receipt = analyze_and_synthesize_source_bound(&SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("area.py"),
            source: source.to_string(),
            python_executable: python_executable.clone(),
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "source-seeded-area".to_string(),
                public_symbol: "area".to_string(),
                public_observations,
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 1,
                max_candidates: 256,
            }],
        })
        .unwrap();
        let alternative = &receipt.alternatives[0];
        assert!(matches!(
            alternative.synthesis.winning_goal.postimage,
            TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Multiply,
                ..
            }
        ));
        assert!(alternative
            .function_template
            .cuts
            .iter()
            .any(|cut| cut.postimage_template.is_some()));
        let repaired = replay_source_bound_patch(source, &alternative.replayable_patch).unwrap();
        assert!(repaired.contains("_b_core_left * _b_core_right"));
        assert!(repaired.contains("_b_core_left + _b_core_right"));
        assert!(repaired.contains("_b_core_left - _b_core_right"));
        validate_source_bound_causal_receipt(&receipt, source).unwrap();
        validate_source_bound_causal_receipt_with_python(&receipt, source, &python_executable)
            .unwrap();
    }

    #[test]
    fn typed_integer_semantics_are_identical_in_python_lowering() {
        let Some(python_executable) = python() else {
            return;
        };
        let sources = BTreeMap::new();
        let operand_types = BTreeMap::new();
        let cases = [
            (BinaryOperator::Add, i64::MAX, 1, i64::MAX),
            (BinaryOperator::Subtract, i64::MIN, 1, i64::MIN),
            (BinaryOperator::Multiply, i64::MAX, 2, i64::MAX),
            (BinaryOperator::Divide, -7, 3, -2),
            (BinaryOperator::Divide, 7, -3, -2),
            (BinaryOperator::Divide, i64::MIN, -1, i64::MAX),
            (BinaryOperator::Modulo, -7, 3, -1),
            (BinaryOperator::Modulo, 7, -3, 1),
            (BinaryOperator::Modulo, i64::MIN, -1, 0),
        ];
        for (operator, left, right, expected) in cases {
            let expression = TypedSyntaxExpressionIR::Binary {
                operator,
                left: Box::new(TypedSyntaxExpressionIR::IntLiteral { value: left }),
                right: Box::new(TypedSyntaxExpressionIR::IntLiteral { value: right }),
            };
            let emitted = python_expression(&expression, &sources, &operand_types).unwrap();
            let output = Command::new(&python_executable)
                .args(["-I", "-S", "-c", &format!("print({emitted})")])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "operator={operator:?} stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).trim(),
                expected.to_string(),
                "operator={operator:?} emitted={emitted}"
            );
        }
    }

    #[test]
    fn python_floor_and_modulo_are_not_mislabeled_as_typed_ir_seeds() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "def floor(left: int, right: int) -> int:\n    return left // right\n\ndef modulo(left: int, right: int) -> int:\n    return left % right\n";
        let response = run_python_host(
            &python_executable,
            source,
            &["floor".to_string(), "modulo".to_string()],
        )
        .unwrap();
        assert!(response.ok, "{:?}", response.detail);
        assert_eq!(response.definitions.len(), 2);
        assert!(response.definitions.iter().all(|definition| definition
            .cuts
            .iter()
            .all(|cut| cut.postimage_template.is_none())));
    }

    #[test]
    fn exact_owner_and_dependency_closure_reach_atomic_materialization() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"class Shadow:
    @staticmethod
    def distance(left: int, right: int) -> int:
        return 999

class Rational:
    @staticmethod
    def distance(left: int, right: int) -> int:
        parsed = parse_expr(left, right)
        return parsed

def parse_expr(first: int, second: int) -> int:
    return evaluateFalse(first, second)

def evaluateFalse(primary: int, secondary: int) -> int:
    return transformer_visitor(primary, secondary)

def transformer_visitor(value: int, baseline: int) -> int:
    return value + baseline
"#;
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: source.to_string(),
            python_executable: python_executable.clone(),
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "rational-distance".to_string(),
                public_symbol: "Rational.distance".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        validate_source_bound_causal_receipt(&receipt, source).unwrap();
        validate_source_bound_causal_receipt_with_python(&receipt, source, &python_executable)
            .unwrap();
        assert!(receipt.public_symbol_owner_preserved);
        assert!(receipt.execution_dependency_closure_preserved);
        assert!(receipt.single_and_multi_edit_share_atomic_path);
        let mut forged_origin = receipt.clone();
        forged_origin.alternatives[0].function_template.cuts[0].postimage_source = "0".to_string();
        forged_origin.alternatives[0]
            .function_template
            .source_template_sha256 =
            source_bound_function_template_hash(&forged_origin.alternatives[0].function_template)
                .unwrap();
        forged_origin.receipt_sha256 = source_bound_receipt_hash(&forged_origin).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_origin, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_TEMPLATE_POSTIMAGE_ORIGIN")
        );
        let mut forged_template_hash = receipt.clone();
        forged_template_hash.alternatives[0]
            .function_template
            .source_template_sha256 = "0".repeat(64);
        forged_template_hash.receipt_sha256 =
            source_bound_receipt_hash(&forged_template_hash).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_template_hash, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_TEMPLATE_HASH_BINDING")
        );
        let mut forged_ast_derivation = receipt.clone();
        forged_ast_derivation.alternatives[0].function_template.cuts[0].postimage_template =
            Some(TypedSyntaxExpressionIR::IntLiteral { value: 0 });
        forged_ast_derivation.alternatives[0]
            .function_template
            .source_template_sha256 = source_bound_function_template_hash(
            &forged_ast_derivation.alternatives[0].function_template,
        )
        .unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt_with_python(
                &forged_ast_derivation,
                source,
                &python_executable,
            )
            .unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_PYTHON_AST_DERIVATION")
        );
        let mut forged_type_derivation = receipt.clone();
        forged_type_derivation.alternatives[0]
            .function_template
            .operands[0]
            .value_type = ProgramType::Bool;
        forged_type_derivation.alternatives[0]
            .function_template
            .source_template_sha256 = source_bound_function_template_hash(
            &forged_type_derivation.alternatives[0].function_template,
        )
        .unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt_with_python(
                &forged_type_derivation,
                source,
                &python_executable,
            )
            .unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_PYTHON_AST_DERIVATION")
        );
        let mut forged_seed_binding = receipt.clone();
        forged_seed_binding.alternatives[0]
            .synthesis
            .synthesis_request
            .as_mut()
            .unwrap()
            .provenance
            .retain(|item| !item.starts_with("SOURCE_SEED_SET_SHA256:"));
        forged_seed_binding.receipt_sha256 =
            source_bound_receipt_hash(&forged_seed_binding).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_seed_binding, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_SEED_SYNTHESIS_BINDING")
        );
        for conflicting_binding in [
            "SOURCE_TEMPLATE_SHA256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "SOURCE_SEED_SET_SHA256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "SOURCE_SEED_COUNT:999",
        ] {
            let mut forged_duplicate_binding = receipt.clone();
            forged_duplicate_binding.alternatives[0]
                .synthesis
                .synthesis_request
                .as_mut()
                .unwrap()
                .provenance
                .push(conflicting_binding.to_string());
            forged_duplicate_binding.receipt_sha256 =
                source_bound_receipt_hash(&forged_duplicate_binding).unwrap();
            assert_eq!(
                validate_source_bound_causal_receipt(&forged_duplicate_binding, source)
                    .unwrap_err(),
                CausalFrontendFailure::conflict("SOURCE_SEED_SYNTHESIS_BINDING")
            );
        }
        let mut forged_owner = receipt.clone();
        forged_owner.alternatives[0].function_template.owner = "Shadow".to_string();
        forged_owner.receipt_sha256 = source_bound_receipt_hash(&forged_owner).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_owner, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_OWNER_CLAIM")
        );
        let mut forged_closure = receipt.clone();
        forged_closure.alternatives[0]
            .function_template
            .execution_dependency_closure
            .clear();
        forged_closure.receipt_sha256 = source_bound_receipt_hash(&forged_closure).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_closure, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_CLOSURE_CLAIM")
        );
        let mut forged_atomic = receipt.clone();
        let SourceEditAtom::AtomicMultiEdit { edits } =
            &forged_atomic.patch_variants[0].replayable_patch.edit
        else {
            panic!("atomic path required")
        };
        forged_atomic.patch_variants[0].replayable_patch.edit = edits[0].clone();
        forged_atomic.receipt_sha256 = source_bound_receipt_hash(&forged_atomic).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_atomic, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_ATOMIC_PATH_CLAIM")
        );
        let mut forged_hash = receipt.clone();
        forged_hash.receipt_sha256 = "0".repeat(64);
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_hash, source).unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_HASH")
        );
        let mut forged_synthesis = receipt.clone();
        forged_synthesis.alternatives[0]
            .synthesis
            .winning_goal
            .postimage = TypedSyntaxExpressionIR::IntLiteral { value: 0 };
        forged_synthesis.receipt_sha256 = source_bound_receipt_hash(&forged_synthesis).unwrap();
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_synthesis, source).unwrap_err(),
            CausalFrontendFailure::conflict(
                "SOURCE_BOUND_RECEIPT_OWNER_SYNTHESIS:TYPED_MECHANISM_COUNTEREXAMPLE:0"
            )
        );
        let alternative = &receipt.alternatives[0];
        assert!(alternative
            .closure_candidates
            .iter()
            .flat_map(|candidate| &candidate.function_template.cuts)
            .any(|cut| cut.postimage_template.is_some()));
        assert!(alternative
            .synthesis
            .synthesis_request
            .as_ref()
            .unwrap()
            .provenance
            .iter()
            .any(|item| item.starts_with("SOURCE_SEED_SET_SHA256:")));
        assert_eq!(alternative.function_template.owner, "Rational");
        assert_eq!(
            alternative.function_template.execution_dependency_closure,
            [
                "Rational.distance",
                "parse_expr",
                "evaluateFalse",
                "transformer_visitor"
            ]
        );
        assert_eq!(alternative.closure_candidates.len(), 3);
        let mut forged_missing_closure_evidence = receipt.clone();
        forged_missing_closure_evidence.alternatives[0]
            .closure_candidates
            .remove(1);
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_missing_closure_evidence, source)
                .unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_CLOSURE_CLAIM")
        );
        let mut forged_duplicate_closure_evidence = receipt.clone();
        let duplicate =
            forged_duplicate_closure_evidence.alternatives[0].closure_candidates[0].clone();
        forged_duplicate_closure_evidence.alternatives[0]
            .closure_candidates
            .push(duplicate);
        assert_eq!(
            validate_source_bound_causal_receipt(&forged_duplicate_closure_evidence, source)
                .unwrap_err(),
            CausalFrontendFailure::conflict("SOURCE_BOUND_RECEIPT_CLOSURE_CLAIM")
        );
        assert_eq!(alternative.candidate_validation_processes, 1);
        let owner_operator = typed_mechanism_improvement_operator_from_receipt(
            &alternative.synthesis,
            alternative.synthesis.receipt_sha256.clone(),
        )
        .unwrap();
        assert!(alternative.synthesis.candidates_enumerated > 1);
        let closure_search_metrics = alternative
            .closure_candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.function_template.qualified_symbol.clone(),
                    candidate.synthesis.candidates_enumerated,
                    candidate.synthesis.preferred_operator_attempts,
                    candidate.synthesis.selected_operator_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            alternative.closure_candidates.iter().all(|candidate| {
                candidate.synthesis.candidates_enumerated
                    < alternative.synthesis.candidates_enumerated
                    && candidate.synthesis.preferred_operator_attempts == 1
                    && candidate.synthesis.selected_operator_id.as_deref()
                        == Some(owner_operator.operator_id.as_str())
            }),
            "owner={} metrics={closure_search_metrics:?}",
            owner_operator.operator_id
        );
        assert_eq!(
            alternative
                .closure_candidates
                .iter()
                .map(|candidate| candidate.function_template.qualified_symbol.as_str())
                .collect::<Vec<_>>(),
            ["parse_expr", "evaluateFalse", "transformer_visitor"]
        );
        assert_eq!(
            alternative.closure_candidates[2].public_operand_bindings,
            BTreeMap::from([
                ("baseline".to_string(), "right".to_string()),
                ("value".to_string(), "left".to_string())
            ])
        );
        assert_eq!(
            receipt.patch_variants[0].selected_template_symbols,
            ["transformer_visitor"]
        );
        let first_variant_source =
            replay_source_bound_patch(source, &receipt.patch_variants[0].replayable_patch).unwrap();
        assert!(first_variant_source.contains("parsed = parse_expr(left, right)"));
        assert!(first_variant_source.contains("return evaluateFalse(first, second)"));
        assert!(!serde_json::to_string(&receipt)
            .unwrap()
            .contains("candidate_source"));
        let mut tampered_variant = receipt.patch_variants[0].replayable_patch.clone();
        tampered_variant.candidate_sha256 = "0".repeat(64);
        assert_eq!(
            replay_source_bound_patch(source, &tampered_variant)
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::ConflictingSourceBoundEdits
        );
        assert!(
            replay_source_bound_patch(source, &alternative.replayable_patch)
                .unwrap()
                .contains(" if ")
        );
        assert!(matches!(
            alternative.replayable_patch.edit,
            SourceEditAtom::AtomicMultiEdit { .. }
        ));
        assert!(
            alternative
                .replayable_patch
                .candidate_materialization_is_one_to_one
        );
        let mut saturated = alternative.clone();
        let repeated = saturated
            .closure_candidates
            .last()
            .expect("deep closure candidate")
            .clone();
        while saturated.closure_candidates.len() <= MAX_SOURCE_BOUND_PATCH_VARIANTS {
            saturated.closure_candidates.push(repeated.clone());
        }
        let bounded = build_source_bound_patch_variants(source, &[saturated.clone()]).unwrap();
        assert!(bounded.len() <= MAX_SOURCE_BOUND_PATCH_VARIANTS);
        assert!(bounded
            .iter()
            .any(|variant| variant.selected_candidate_indices == [0]));
        let selected =
            select_source_bound_patch_proposals(std::slice::from_ref(&saturated), bounded).unwrap();
        assert!(selected.len() <= MAX_SELECTED_SOURCE_PROPOSALS);
        assert_eq!(
            selected,
            select_source_bound_patch_proposals(
                std::slice::from_ref(&saturated),
                build_source_bound_patch_variants(source, &[saturated.clone()]).unwrap(),
            )
            .unwrap()
        );
    }

    #[test]
    fn conflicting_dependency_operand_transport_fails_closed_to_the_public_owner() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def distance(left: int, right: int) -> int:
    if left > right:
        return helper(left, right)
    else:
        return helper(right, left)

def helper(first: int, second: int) -> int:
    return 0
"#;
        let receipt = analyze_and_synthesize_source_bound(&SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "ambiguous-helper-transport".to_string(),
                public_symbol: "distance".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        })
        .unwrap();
        assert_eq!(
            receipt.alternatives[0]
                .function_template
                .execution_dependency_closure,
            ["distance", "helper"]
        );
        assert!(receipt.alternatives[0].closure_candidates.is_empty());
        assert_eq!(
            receipt.alternatives[0].closure_candidate_rejections.len(),
            1
        );
        assert_eq!(
            receipt.alternatives[0].closure_candidate_rejections[0].failure_kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert!(receipt.alternatives[0].closure_candidate_rejections[0]
            .detail
            .starts_with("DEPENDENCY_OPERAND_BINDING_AMBIGUOUS:"));
        assert_eq!(receipt.patch_variants.len(), 1);
        assert_eq!(
            receipt.patch_variants[0].selected_template_symbols,
            ["distance"]
        );
    }

    #[test]
    fn diamond_dependency_transport_does_not_leak_the_first_path_mapping() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def distance(left: int, right: int) -> int:
    if left > right:
        return forward(left, right)
    else:
        return reverse(left, right)

def forward(left: int, right: int) -> int:
    return helper(left, right)

def reverse(left: int, right: int) -> int:
    return helper(right, left)

def helper(first: int, second: int) -> int:
    return 0
"#;
        let receipt = analyze_and_synthesize_source_bound(&SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("diamond.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "diamond-transport".to_string(),
                public_symbol: "distance".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        })
        .unwrap();
        let alternative = &receipt.alternatives[0];
        assert_eq!(
            alternative
                .closure_candidates
                .iter()
                .map(|candidate| candidate.function_template.qualified_symbol.as_str())
                .collect::<Vec<_>>(),
            ["forward", "reverse"]
        );
        assert!(alternative
            .closure_candidate_rejections
            .iter()
            .any(|rejection| {
                rejection.qualified_symbol == "helper"
                    && rejection.failure_kind
                        == CausalFrontendFailureKind::PublicInformationInsufficient
                    && rejection
                        .detail
                        .starts_with("DEPENDENCY_OPERAND_BINDING_AMBIGUOUS:")
            }));
        assert!(receipt.patch_variants.iter().all(|variant| {
            !variant
                .selected_template_symbols
                .iter()
                .any(|symbol| symbol == "helper")
        }));
    }

    #[test]
    fn conditional_multi_location_uses_the_same_atomic_path() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "# 한글 byte span canary\r\ndef distance(left: int, right: int) -> int:\r\n    if left == right:\r\n        return 1\r\n    else:\r\n        return 2\r\n";
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("거리.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "distance".to_string(),
                public_symbol: "distance".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let patch = &receipt.alternatives[0].replayable_patch;
        let SourceEditAtom::AtomicMultiEdit { edits } = &patch.edit else {
            panic!("atomic path required")
        };
        assert_eq!(edits.len(), 3);
        assert_eq!(
            apply_edit_atom(source, &patch.edit).unwrap(),
            replay_source_bound_patch(source, patch).unwrap()
        );
        assert!(replay_source_bound_patch(source, patch)
            .unwrap()
            .starts_with("# 한글 byte span canary\r\n"));
    }

    #[test]
    fn conditional_source_accepts_a_simpler_unconditional_postimage() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def combine(left: int, right: int) -> int:
    if left > right:
        return 0
    else:
        return 1
"#;
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("conditional_combine.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "simpler-combine".to_string(),
                public_symbol: "combine".to_string(),
                public_observations: observations(&[(2, 3, 5), (4, 7, 11), (-3, 8, 5)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 1,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let alternative = &receipt.alternatives[0];
        assert!(!alternative.synthesis.conditional_synthesized);
        let SourceEditAtom::AtomicMultiEdit { edits } = &alternative.replayable_patch.edit else {
            panic!("atomic path required")
        };
        assert_eq!(edits.len(), 2);
        let candidate = replay_source_bound_patch(source, &alternative.replayable_patch).unwrap();
        assert!(candidate.contains("return (lambda _b_core_left, _b_core_right:"));
        assert!(candidate.contains("_b_core_left + _b_core_right"));
        assert!(candidate.contains("if left > right:"));
    }

    #[test]
    fn conditional_fallthrough_return_reaches_the_atomic_lowering_path() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "# boundary byte span canary\r\ndef distance(left: int, right: int) -> int:\r\n    if left == right:\r\n        return 1\r\n    return 2\r\n";
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("fallthrough.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "fallthrough-distance".to_string(),
                public_symbol: "distance".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let alternative = &receipt.alternatives[0];
        assert_eq!(
            alternative
                .function_template
                .cuts
                .iter()
                .map(|cut| cut.branch)
                .collect::<Vec<_>>(),
            [CausalCutBranch::Then, CausalCutBranch::Unconditional]
        );
        let SourceEditAtom::AtomicMultiEdit { edits } = &alternative.replayable_patch.edit else {
            panic!("atomic path required")
        };
        assert_eq!(edits.len(), 3);
        let candidate = replay_source_bound_patch(source, &alternative.replayable_patch).unwrap();
        assert!(candidate.starts_with("# boundary byte span canary\r\n"));
    }

    #[test]
    fn ambiguous_mixed_conditional_topology_remains_fail_closed() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def classify(left: int, right: int) -> int:
    if left > right:
        return left - right
    if left == right:
        return 0
    return right - left
"#;
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("mixed.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "mixed-conditional".to_string(),
                public_symbol: "classify".to_string(),
                public_observations: observations(&[
                    (9, 4, 5),
                    (3, 8, 5),
                    (-2, -9, 7),
                    (-8, -3, 5),
                ]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        assert_eq!(
            analyze_and_synthesize_source_bound(&request).unwrap_err(),
            CausalFrontendFailure::unsupported("CONDITIONAL_CUT_TOPOLOGY_UNSUPPORTED")
        );
    }

    #[test]
    fn unselected_unsupported_functions_do_not_poison_observation_typed_target() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = r#"def unrelated(*args):
    yield from args

def noisy(left, right):
    return left * right

def add(left, right):
    def nested():
        return noisy(left, right)
    return 0
"#;
        let request = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("ordinary_python.py"),
            source: source.to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "unannotated-add".to_string(),
                public_symbol: "add".to_string(),
                public_observations: observations(&[(2, 3, 5), (4, 7, 11)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let alternative = &receipt.alternatives[0];
        assert_eq!(
            alternative.function_template.execution_dependency_closure,
            ["add"]
        );
        assert_eq!(
            alternative.function_template.output_type_evidence,
            "PUBLIC_OBSERVATION"
        );
        assert!(alternative
            .function_template
            .operand_type_evidence
            .values()
            .all(|evidence| evidence == "PUBLIC_OBSERVATION"));
        let candidate = replay_source_bound_patch(source, &alternative.replayable_patch).unwrap();
        assert!(candidate.contains("return (lambda _b_core_left, _b_core_right:"));
        assert!(candidate.contains("_b_core_left + _b_core_right"));
    }

    #[test]
    fn selected_unsupported_closure_and_conflicting_declaration_fail_precisely() {
        let Some(python_executable) = python() else {
            return;
        };
        let unsupported = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: "def gather(*args):\n    yield from args\n".to_string(),
            python_executable: python_executable.clone(),
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "gather".to_string(),
                public_symbol: "gather".to_string(),
                public_observations: vec![TypedMechanismObservationIR {
                    operands: BTreeMap::from([("args".to_string(), Value::Int(1))]),
                    expected_postimage: Value::Int(1),
                }],
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 1,
                max_candidates: 16,
            }],
        };
        assert_eq!(
            analyze_and_synthesize_source_bound(&unsupported)
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::UnsupportedLanguageSyntax
        );

        let conflicting = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: "def add(left: bool, right: bool) -> bool:\n    return False\n".to_string(),
            python_executable,
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "conflicting-add".to_string(),
                public_symbol: "add".to_string(),
                public_observations: observations(&[(2, 3, 5), (4, 7, 11)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let error = analyze_and_synthesize_source_bound(&conflicting).unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert!(error.detail.starts_with("DECLARED_PUBLIC_TYPE_CONFLICT:"));
    }

    #[test]
    fn repository_tests_autonomously_derive_a_contradicted_source_bound_goal() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "def add(left, right):\n    return 0\n";
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("calculator.py"),
            source: source.to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("test_calculator.py"),
                source: "def test_add():\n    assert add(2, 3) == 5\n    assert add(left=4, right=7) == 11\n"
                    .to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 2,
            max_candidates: 1_024,
        };
        let receipt = discover_and_synthesize_python_repository(&request).unwrap();
        assert_eq!(receipt.alternatives.len(), 1);
        assert!(receipt.alternatives[0]
            .alternative_id
            .starts_with("AUTO:STATIC_PUBLIC_CONTRADICTION:add:"));
        let candidate =
            replay_source_bound_patch(source, &receipt.alternatives[0].replayable_patch).unwrap();
        assert!(candidate.contains("return (lambda _b_core_left, _b_core_right:"));
        assert!(candidate.contains("_b_core_left + _b_core_right"));
    }

    #[test]
    fn diagnostic_target_does_not_override_insufficient_public_evidence() {
        let Some(python_executable) = python() else {
            return;
        };
        let tests = vec![RepositoryTestSourceIR {
            relative_path: PathBuf::from("test_engine.py"),
            source: "def test_distance():\n    assert Rational.distance(9, 4) == 5\n    assert Rational.distance(2, 7) == 5\n"
                .to_string(),
        }];
        let source = r#"class Shadow:
    @staticmethod
    def distance(left, right):
        return left - right

class Rational:
    @staticmethod
    def distance(left, right):
        return parse_expr(left, right)

def parse_expr(left, right):
    return left + right
"#;
        let base = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: source.to_string(),
            test_sources: tests,
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 2,
            max_candidates: 1_024,
        };
        assert_eq!(
            discover_and_synthesize_python_repository(&base)
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        let targeted = SourceBoundRepositoryDiscoveryRequestIR {
            target_symbols: vec!["Rational.distance".to_string()],
            ..base.clone()
        };
        assert_eq!(
            discover_and_synthesize_python_repository(&targeted)
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        let sufficient = SourceBoundRepositoryDiscoveryRequestIR {
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("test_engine.py"),
                source: "def test_distance():\n    assert Rational.distance(9, 4) == 5\n    assert Rational.distance(3, 8) == 5\n    assert Rational.distance(-2, -9) == 7\n    assert Rational.distance(-8, -3) == 5\n"
                    .to_string(),
            }],
            target_symbols: vec!["Rational.distance".to_string()],
            ..base
        };
        let receipt = discover_and_synthesize_python_repository(&sufficient).unwrap();
        assert_eq!(
            receipt.alternatives[0]
                .function_template
                .execution_dependency_closure,
            ["Rational.distance", "parse_expr"]
        );
        assert!(receipt.alternatives[0]
            .alternative_id
            .starts_with("AUTO:FAILED_DIAGNOSTIC_TARGET:Rational.distance:"));
    }

    #[test]
    fn exact_failure_ontology_replaces_generic_incomplete_plan() {
        let missing = SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("engine.py"),
            source: "def known(value: int) -> int:\n    return value\n".to_string(),
            python_executable: python().unwrap_or_default(),
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "missing".to_string(),
                public_symbol: "Rational.missing".to_string(),
                public_observations: vec![TypedMechanismObservationIR {
                    operands: BTreeMap::from([("value".to_string(), Value::Int(1))]),
                    expected_postimage: Value::Int(1),
                }],
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 1,
                max_candidates: 16,
            }],
        };
        if missing.python_executable.as_os_str().is_empty() {
            return;
        }
        assert_eq!(
            analyze_and_synthesize_source_bound(&missing)
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::PublicInformationInsufficient
        );
        assert_eq!(
            language_backend_for_path(Path::new("engine.ts"))
                .unwrap_err()
                .kind,
            CausalFrontendFailureKind::UnsupportedLanguageSyntax
        );
    }

    #[test]
    fn missing_product_class_declarations_reach_one_atomic_verified_variant() {
        let Some(python_executable) = python() else {
            return;
        };
        let source = "class Shadow:\r\n    pass\r\n\r\nclass RenamedPolicy:\r\n    \"\"\"한글 설명\"\"\"\r\n    pass\r\n";
        let tests = r#"def test_policy_contract():
    assert RenamedPolicy.marker == "준비"
    assert RenamedPolicy.limit == 3
"#;
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("policy.py"),
            source: source.to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_policy.py"),
                source: tests.to_string(),
            }],
            python_executable: python_executable.clone(),
            target_symbols: vec!["RenamedPolicy.marker".to_string()],
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 64,
        };
        let receipt = discover_and_synthesize_python_repository(&request).unwrap();
        assert!(receipt.alternatives.is_empty());
        assert_eq!(receipt.declaration_alternatives.len(), 2);
        assert_eq!(receipt.patch_variants.len(), 1);
        assert!(receipt.public_symbol_owner_preserved);
        assert!(receipt.execution_dependency_closure_preserved);
        assert!(receipt.single_and_multi_edit_share_atomic_path);
        assert!(receipt.declaration_alternatives.iter().all(|declaration| {
            declaration
                .requested_public_symbol
                .starts_with("RenamedPolicy.")
                && declaration.declaration_template.qualified_owner == "RenamedPolicy"
                && declaration.candidate_validation_processes == 1
        }));
        let candidate =
            replay_source_bound_patch(source, &receipt.patch_variants[0].replayable_patch).unwrap();
        assert!(candidate.contains("limit = 3"));
        assert!(candidate.contains("marker = '준비'"));
        assert!(candidate.contains("\"\"\"한글 설명\"\"\""));
        assert!(!candidate.replace("\r\n", "").contains('\n'));
        let execution = Command::new(&python_executable)
            .args(["-X", "utf8", "-c"])
            .arg(format!("{candidate}\n{tests}\ntest_policy_contract()\n"))
            .output()
            .unwrap();
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
        validate_source_bound_causal_receipt_with_python(&receipt, source, &python_executable)
            .unwrap();
    }

    #[test]
    fn declaration_owner_evidence_ignores_test_fixture_classes() {
        let Some(python_executable) = python() else {
            return;
        };
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("catalog.py"),
            source: "class Catalog:\n    pass\n".to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_catalog.py"),
                source: r#"class Catalog:
    label = "fixture"

def test_fixture_is_not_product_evidence():
    assert Catalog.label == "ready"
"#
                .to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 64,
        };
        assert_eq!(
            discover_and_synthesize_python_repository(&request).unwrap_err(),
            CausalFrontendFailure::public("NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
        );
    }

    #[test]
    fn existing_or_conflicting_class_declaration_evidence_fails_closed() {
        let Some(python_executable) = python() else {
            return;
        };
        let base = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("catalog.py"),
            source: "class Catalog:\n    label = 'ready'\n".to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_catalog.py"),
                source: "def test_catalog():\n    assert Catalog.label == 'ready'\n".to_string(),
            }],
            python_executable: python_executable.clone(),
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 64,
        };
        assert_eq!(
            discover_and_synthesize_python_repository(&base).unwrap_err(),
            CausalFrontendFailure::public("NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
        );
        let incorrect = SourceBoundRepositoryDiscoveryRequestIR {
            source: "class Catalog:\n    label = 'blocked'\n".to_string(),
            ..base.clone()
        };
        let incorrect_receipt = discover_and_synthesize_python_repository(&incorrect).unwrap();
        assert_eq!(incorrect_receipt.declaration_alternatives.len(), 1);
        assert_eq!(
            incorrect_receipt.declaration_alternatives[0]
                .declaration_template
                .operation,
            SourceBoundDeclarationOperation::Replace
        );
        let repaired = replay_source_bound_patch(
            &incorrect.source,
            &incorrect_receipt.patch_variants[0].replayable_patch,
        )
        .unwrap();
        assert!(repaired.contains("label = 'ready'"));
        let conflicting = SourceBoundRepositoryDiscoveryRequestIR {
            source: "class Catalog:\n    pass\n".to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_catalog.py"),
                source: "def test_catalog():\n    assert Catalog.label == 'ready'\n    assert Catalog.label == 'blocked'\n".to_string(),
            }],
            ..base
        };
        let error = discover_and_synthesize_python_repository(&conflicting).unwrap_err();
        assert_eq!(
            error.kind,
            CausalFrontendFailureKind::ConflictingSourceBoundEdits
        );
        assert!(error
            .detail
            .starts_with("CONFLICTING_PUBLIC_DECLARATION_POSTIMAGES:Catalog.label"));
    }

    #[test]
    fn irrelevant_typed_operator_memory_cannot_suppress_exact_declaration_template() {
        let Some(python_executable) = python() else {
            return;
        };
        let prior_source = "def add(left: int, right: int) -> int:\n    return 0\n";
        let prior_receipt = analyze_and_synthesize_source_bound(&SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("prior.py"),
            source: prior_source.to_string(),
            python_executable: python_executable.clone(),
            alternatives: vec![SourceBoundCausalAlternativeIR {
                alternative_id: "prior-add".to_string(),
                public_symbol: "add".to_string(),
                public_observations: observations(&[(2, 3, 5), (4, 7, 11)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        })
        .unwrap();
        let irrelevant_operator = typed_mechanism_improvement_operator_from_receipt(
            &prior_receipt.alternatives[0].synthesis,
            "a".repeat(64),
        )
        .unwrap();
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("policy.py"),
            source: "class Policy:\n    pass\n".to_string(),
            test_sources: vec![RepositoryTestSourceIR {
                relative_path: PathBuf::from("tests/test_policy.py"),
                source: "def test_policy():\n    assert Policy.marker == 'ready'\n".to_string(),
            }],
            python_executable,
            target_symbols: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            max_expression_depth: 1,
            max_candidates: 64,
        };
        let without_memory = discover_and_synthesize_python_repository(&request).unwrap();
        let with_memory = discover_and_synthesize_python_repository_with_operators(
            &request,
            &[irrelevant_operator],
        )
        .unwrap();
        assert_eq!(with_memory, without_memory);
    }

    #[test]
    fn overlapping_source_bound_edits_have_precise_failure() {
        let source = "abcdef";
        let edit = SourceEditAtom::AtomicMultiEdit {
            edits: vec![
                replacement_edit(source, ByteRange { start: 1, end: 4 }, "x".to_string()).unwrap(),
                replacement_edit(source, ByteRange { start: 3, end: 5 }, "y".to_string()).unwrap(),
            ],
        };
        let error = apply_edit_atom(source, &edit).unwrap_err();
        let classified = if error.contains("OVERLAPPING") {
            CausalFrontendFailure::conflict(error)
        } else {
            CausalFrontendFailure::unsupported(error)
        };
        assert_eq!(
            classified.kind,
            CausalFrontendFailureKind::ConflictingSourceBoundEdits
        );
    }
}
