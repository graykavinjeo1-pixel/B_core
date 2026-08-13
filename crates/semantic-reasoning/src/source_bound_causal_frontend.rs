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
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::bounded_parallel::{
    map_ordered_batched_by as parallel_map_ordered_batched_by, worker_count_for,
};
use crate::self_repair_contract::sha256;
use crate::sem5::model::{
    BinaryOperator, DataSplit, Effect, ProgramType, StringTransformOperator, UnaryOperator,
};
use crate::sem5::typed_mechanism::{
    synthesize_typed_mechanism_goal_with_source_seeds_and_priors,
    typed_mechanism_improvement_operator_from_receipt, validate_typed_mechanism_synthesis_receipt,
    SourceOperandIR, TypedMechanismImprovementOperatorIR, TypedMechanismObservationIR,
    TypedMechanismSynthesisGoalIR, TypedMechanismSynthesisReceiptIR, TypedSyntaxExpressionIR,
    TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
};
use crate::structural_source_repair::{apply_edit_atom, ByteRange, SourceEditAtom};

pub const SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_REQUEST_1";
pub const SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_RECEIPT_1";
pub const SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA: &str =
    "B_CORE_SOURCE_BOUND_REPOSITORY_DISCOVERY_1";
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
            ast.FloorDiv: "DIVIDE", ast.Mod: "MODULO",
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

results = []
for ordinal, candidate in enumerate(candidates):
    source = candidate.get("source") if isinstance(candidate, dict) else None
    public_symbol = candidate.get("public_symbol") if isinstance(candidate, dict) else None
    if not isinstance(source, str) or not isinstance(public_symbol, str) or not public_symbol:
        results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "CANDIDATE_INPUT:" + str(ordinal)})
        continue
    try:
        tree = ast.parse(source, filename="<b-core-candidate-" + str(ordinal) + ">", type_comments=True)
        compile(tree, "<b-core-candidate-" + str(ordinal) + ">", "exec")
    except (SyntaxError, ValueError, TypeError) as error:
        results.append({"ok": False, "failure": "UNSUPPORTED_LANGUAGE_SYNTAX", "detail": "CANDIDATE_PARSE:" + str(error)})
        continue
    if public_symbol not in qualified_functions(tree):
        results.append({"ok": False, "failure": "PUBLIC_INFORMATION_INSUFFICIENT", "detail": "MATERIALIZED_PUBLIC_SYMBOL_IDENTITY_LOST:" + public_symbol})
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

definitions = {}
def collect(items, prefix):
    for item in items:
        if isinstance(item, ast.ClassDef):
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

observations = {}
for test in tests:
    path, test_source = test.get("relative_path", "<test>"), test.get("source")
    if not isinstance(test_source, str):
        fail("PUBLIC_INFORMATION_INSUFFICIENT", "TEST_SOURCE_MISSING:" + str(path))
    try:
        tree = ast.parse(test_source, filename=str(path), type_comments=True)
        compile(tree, str(path), "exec")
    except (SyntaxError, ValueError, TypeError) as error:
        fail("UNSUPPORTED_LANGUAGE_SYNTAX", "TEST_PARSE:" + str(path) + ":" + str(error))
    for node in ast.walk(tree):
        observation = None
        if isinstance(node, ast.Assert):
            test_node = node.test
            if isinstance(test_node, ast.Compare) and len(test_node.ops) == 1 and isinstance(test_node.ops[0], ast.Eq) and len(test_node.comparators) == 1:
                left, right = test_node.left, test_node.comparators[0]
                if isinstance(left, ast.Call): observation = call_observation(left, literal(right))
                elif isinstance(right, ast.Call): observation = call_observation(right, literal(left))
            elif isinstance(test_node, ast.Call):
                observation = call_observation(test_node, True)
            elif isinstance(test_node, ast.UnaryOp) and isinstance(test_node.op, ast.Not) and isinstance(test_node.operand, ast.Call):
                observation = call_observation(test_node.operand, False)
        if observation is not None:
            qualified, values, expected = observation
            key = json.dumps([{role: encoded(value) for role, value in values.items()}, encoded(expected)], sort_keys=True, ensure_ascii=False)
            observations.setdefault(qualified, {})[key] = {"values": values, "expected": expected}

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
if not alternatives:
    fail("PUBLIC_INFORMATION_INSUFFICIENT", "NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE")
json.dump({"ok": True, "alternatives": alternatives}, sys.stdout, ensure_ascii=False)
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

fn validate_python_candidate_batch(
    executable: &Path,
    candidates: &[(&str, &str)],
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
            let next_bytes = candidates[end].0.len();
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
                .map(|(source, public_symbol)| serde_json::json!({
                    "source": source,
                    "public_symbol": public_symbol,
                }))
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

fn host_failure(response: &PythonHostResponse) -> Option<CausalFrontendFailure> {
    if response.ok {
        return None;
    }
    let detail = response
        .detail
        .clone()
        .unwrap_or_else(|| "PYTHON_HOST_UNCLASSIFIED".to_string());
    Some(match response.failure.as_deref() {
        Some("PUBLIC_INFORMATION_INSUFFICIENT") => CausalFrontendFailure::public(detail),
        Some("CONFLICTING_SOURCE_BOUND_EDITS") => CausalFrontendFailure::conflict(detail),
        _ => CausalFrontendFailure::unsupported(detail),
    })
}

fn classified_host_failure(failure: Option<&str>, detail: Option<&str>) -> CausalFrontendFailure {
    let detail = detail.unwrap_or("PYTHON_HOST_UNCLASSIFIED").to_string();
    match failure {
        Some("PUBLIC_INFORMATION_INSUFFICIENT") => CausalFrontendFailure::public(detail),
        Some("CONFLICTING_SOURCE_BOUND_EDITS") => CausalFrontendFailure::conflict(detail),
        _ => CausalFrontendFailure::unsupported(detail),
    }
}

fn failure_kind_from_code(code: &str) -> CausalFrontendFailureKind {
    match code {
        "CONFLICTING_SOURCE_BOUND_EDITS" => CausalFrontendFailureKind::ConflictingSourceBoundEdits,
        "UNSUPPORTED_LANGUAGE_SYNTAX" => CausalFrontendFailureKind::UnsupportedLanguageSyntax,
        _ => CausalFrontendFailureKind::PublicInformationInsufficient,
    }
}

/// Discover public literal observations from Python tests, retain only an
/// explicit hole, a statically contradicted implementation, or a symbol bound
/// by a failing diagnostic, then run the exact same source-bound synthesis and
/// atomic materialization path as an explicit causal request.
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
    if alternatives.is_empty() {
        return Err(CausalFrontendFailure::public(
            "NO_EVIDENCE_BOUND_REPAIR_ALTERNATIVE",
        ));
    }
    analyze_and_synthesize_source_bound_with_operators(
        &SourceBoundCausalRequestIR {
            schema: SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA.to_string(),
            source_relative_path: request.source_relative_path.clone(),
            source: request.source.clone(),
            python_executable: request.python_executable.clone(),
            alternatives,
        },
        operators,
    )
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
        TypedSyntaxExpressionIR::Unary { operator, input } => Ok(format!(
            "({}{})",
            match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "not ",
            },
            python_expression(input, sources)?
        )),
        TypedSyntaxExpressionIR::StringTransform { operator, input } => Ok(format!(
            "({}).{}()",
            python_expression(input, sources)?,
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
        } => Ok(format!(
            "({} {} {})",
            python_expression(left, sources)?,
            match operator {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "//",
                BinaryOperator::Modulo => "%",
                BinaryOperator::Equal => "==",
                BinaryOperator::NotEqual => "!=",
                BinaryOperator::LessThan => "<",
                BinaryOperator::LessThanOrEqual => "<=",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::GreaterThanOrEqual => ">=",
                BinaryOperator::And => "and",
                BinaryOperator::Or => "or",
            },
            python_expression(right, sources)?
        )),
        TypedSyntaxExpressionIR::Length { input } => {
            Ok(format!("len({})", python_expression(input, sources)?))
        }
        TypedSyntaxExpressionIR::Index { collection, index } => Ok(format!(
            "({})[{}]",
            python_expression(collection, sources)?,
            python_expression(index, sources)?
        )),
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(format!(
            "{api_token}({})",
            arguments
                .iter()
                .map(|argument| python_expression(argument, sources))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
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
    let goal = &synthesis.winning_goal;
    let condition = goal
        .condition
        .as_ref()
        .map(|expression| python_expression(expression, &sources))
        .transpose()?;
    let postimage = python_expression(&goal.postimage, &sources)?;
    let otherwise = goal
        .otherwise
        .as_ref()
        .map(|expression| python_expression(expression, &sources))
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
    let expected = format!("SOURCE_SEED_SET_SHA256:{}", source_seed_set_sha256(&seeds)?);
    let request = synthesis
        .synthesis_request
        .as_ref()
        .ok_or_else(|| CausalFrontendFailure::conflict("SOURCE_SEED_SYNTHESIS_REQUEST_MISSING"))?;
    if !request.provenance.contains(&expected)
        || !request
            .provenance
            .contains(&format!("SOURCE_SEED_COUNT:{}", seeds.len()))
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
) -> Result<MaterializedSourceBoundPatchIR, CausalFrontendFailure> {
    let mut edits = Vec::new();
    for patch in patches {
        match &patch.edit {
            SourceEditAtom::AtomicMultiEdit { edits: nested } => {
                edits.extend(nested.iter().cloned())
            }
            edit => edits.push(edit.clone()),
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
    if candidate_source == source {
        return Err(CausalFrontendFailure::unsupported(
            "COMBINED_SOURCE_BOUND_PATCH_NO_OP",
        ));
    }
    let replay = apply_edit_atom(source, &edit).map_err(CausalFrontendFailure::conflict)?;
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let candidate_replay_sha256 = sha256(replay.as_bytes());
    if replay != candidate_source || candidate_replay_sha256 != candidate_sha256 {
        return Err(CausalFrontendFailure::conflict(
            "COMBINED_CANDIDATE_MATERIALIZATION_DIVERGED",
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

fn source_bound_receipt_claims(receipt: &SourceBoundCausalReceiptIR) -> (bool, bool, bool) {
    let owner_preserved = receipt.alternatives.iter().all(|alternative| {
        alternative.requested_public_symbol == alternative.function_template.qualified_symbol
            && alternative.function_template.owner
                == qualified_symbol_owner(&alternative.function_template.qualified_symbol)
            && alternative.closure_candidates.iter().all(|candidate| {
                candidate.function_template.owner
                    == qualified_symbol_owner(&candidate.function_template.qualified_symbol)
            })
    });
    let closure_preserved = receipt.alternatives.iter().all(|alternative| {
        let closure = &alternative.function_template.execution_dependency_closure;
        template_closure_is_preserved(&alternative.function_template)
            && alternative.closure_candidates.iter().all(|candidate| {
                closure.get(candidate.closure_ordinal)
                    == Some(&candidate.function_template.qualified_symbol)
                    && template_closure_is_preserved(&candidate.function_template)
            })
            && alternative
                .closure_candidate_rejections
                .iter()
                .all(|rejection| {
                    closure.get(rejection.closure_ordinal) == Some(&rejection.qualified_symbol)
                })
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
    if build_source_bound_patch_variants(source, &receipt.alternatives)? != receipt.patch_variants {
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

fn build_source_bound_patch_variants(
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
        let materialized_patch = match combine_source_bound_patches(source, &patches) {
            Ok(patch) => patch,
            Err(error) if error.kind == CausalFrontendFailureKind::ConflictingSourceBoundEdits => {
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
                closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                    closure_ordinal,
                    qualified_symbol: rejection.qualified_symbol.clone(),
                    failure_kind: failure_kind_from_code(&rejection.failure),
                    detail: rejection.detail.clone(),
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
                                    qualified_symbol: candidate
                                        .function_template
                                        .qualified_symbol
                                        .clone(),
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
    let patch_variants = build_source_bound_patch_variants(&request.source, &receipts)?;
    let mut receipt = SourceBoundCausalReceiptIR {
        schema: SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA.to_string(),
        source_relative_path: request.source_relative_path.clone(),
        language_backend: backend,
        predecessor_sha256: sha256(request.source.as_bytes()),
        alternatives: receipts,
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
    validate_source_bound_causal_receipt(&receipt, &request.source)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem5::model::Value;

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
            python_executable,
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
        assert!(repaired.contains("((left) + (right)) * ((left) - (right))"));
        validate_source_bound_causal_receipt(&receipt, source).unwrap();
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
            python_executable,
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
        let bounded = build_source_bound_patch_variants(source, &[saturated]).unwrap();
        assert!(bounded.len() <= MAX_SOURCE_BOUND_PATCH_VARIANTS);
        assert!(bounded
            .iter()
            .any(|variant| variant.selected_candidate_indices == [0]));
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
        assert!(candidate.contains("return ((left) + (right))"));
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
        assert!(
            replay_source_bound_patch(source, &alternative.replayable_patch)
                .unwrap()
                .contains("return ((left) + (right))")
        );
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
        assert!(
            replay_source_bound_patch(source, &receipt.alternatives[0].replayable_patch)
                .unwrap()
                .contains("return ((left) + (right))")
        );
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
