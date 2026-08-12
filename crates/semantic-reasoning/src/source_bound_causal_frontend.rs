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
use crate::sem5::model::{BinaryOperator, DataSplit, Effect, ProgramType, UnaryOperator};
use crate::sem5::typed_mechanism::{
    synthesize_typed_mechanism_goal_with_priors, SourceOperandIR,
    TypedMechanismImprovementOperatorIR, TypedMechanismObservationIR,
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
    pub postimage_source: String,
    pub postimage_range: ByteRange,
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
    pub cuts: Vec<SourceBoundCausalCutIR>,
    pub source_template_sha256: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundAlternativeReceiptIR {
    pub alternative_id: String,
    pub requested_public_symbol: String,
    pub function_template: SourceBoundFunctionTemplateIR,
    pub synthesis: TypedMechanismSynthesisReceiptIR,
    pub materialized_patch: MaterializedSourceBoundPatchIR,
    #[serde(default)]
    pub closure_candidates: Vec<SourceBoundClosureCandidateReceiptIR>,
    #[serde(default)]
    pub closure_candidate_rejections: Vec<SourceBoundClosureCandidateRejectionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundClosureCandidateReceiptIR {
    pub closure_ordinal: usize,
    pub public_operand_bindings: BTreeMap<String, String>,
    pub function_template: SourceBoundFunctionTemplateIR,
    pub synthesis: TypedMechanismSynthesisReceiptIR,
    pub materialized_patch: MaterializedSourceBoundPatchIR,
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
    pub materialized_patch: MaterializedSourceBoundPatchIR,
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
    postimage_source: String,
    postimage_start: usize,
    postimage_end: usize,
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

def cuts_for(definition):
    cuts = []
    def visit_statements(statements, guard=None, branch="UNCONDITIONAL"):
        for statement in statements:
            if isinstance(statement, ast.Return) and statement.value is not None:
                cuts.append({
                    "branch": branch,
                    "condition_source": source_segment(guard) if guard is not None else None,
                    "condition_start": byte_offset(guard) if guard is not None else None,
                    "condition_end": byte_offset(guard, True) if guard is not None else None,
                    "postimage_source": source_segment(statement.value),
                    "postimage_start": byte_offset(statement.value),
                    "postimage_end": byte_offset(statement.value, True),
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
            "public_operand_bindings": closure_bindings,
        })
    selected.append({
        "qualified_symbol": requested, "owner": definition["owner"],
        "is_async": definition["is_async"], "operands": definition["operands"],
        "return_annotation": definition["return_annotation"], "effects": definition["effects"],
        "direct_dependencies": definition["direct_dependencies"],
        "execution_dependency_closure": closure, "cuts": cuts,
        "closure_templates": closure_templates,
        "closure_rejections": closure_rejections,
    })

json.dump({"ok": True, "definitions": selected}, sys.stdout, ensure_ascii=False)
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
    if isinstance(node, ast.Constant) and isinstance(node.value, (bool, int, bytes)): return node.value
    if isinstance(node, ast.List):
        values = [safe_eval(element, environment) for element in node.elts]
        return values if all(value is not UNKNOWN for value in values) else UNKNOWN
    if isinstance(node, ast.Name): return environment.get(node.id, UNKNOWN)
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
                postimage_source: cut.postimage_source,
                postimage_range: ByteRange {
                    start: cut.postimage_start,
                    end: cut.postimage_end,
                },
            })
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    let source_template_sha256 = sha256(
        serde_json::to_vec(&(
            &definition.qualified_symbol,
            &operands,
            &output_type,
            &definition.execution_dependency_closure,
            &cuts,
        ))
        .map_err(|error| CausalFrontendFailure::public(format!("TEMPLATE_HASH:{error}")))?
        .as_slice(),
    );
    Ok(SourceBoundFunctionTemplateIR {
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
        cuts,
        source_template_sha256,
    })
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
                BinaryOperator::LessThan => "<",
                BinaryOperator::GreaterThan => ">",
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
    if !unconditional.is_empty() {
        if unconditional.len() != template.cuts.len() {
            return Err(CausalFrontendFailure::unsupported(
                "MIXED_CONDITIONAL_AND_UNCONDITIONAL_CUTS",
            ));
        }
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
        let condition = condition.ok_or_else(|| {
            CausalFrontendFailure::unsupported("CONDITIONAL_SOURCE_CUT_REQUIRES_CONDITION")
        })?;
        let otherwise = otherwise.ok_or_else(|| {
            CausalFrontendFailure::unsupported("CONDITIONAL_SOURCE_CUT_REQUIRES_OTHERWISE")
        })?;
        let condition_ranges = template
            .cuts
            .iter()
            .filter_map(|cut| cut.condition_range)
            .collect::<BTreeSet<_>>();
        if condition_ranges.len() != 1
            || !template
                .cuts
                .iter()
                .any(|cut| cut.branch == CausalCutBranch::Then)
            || !template
                .cuts
                .iter()
                .any(|cut| cut.branch == CausalCutBranch::Else)
        {
            return Err(CausalFrontendFailure::unsupported(
                "CONDITIONAL_CUT_TOPOLOGY_UNSUPPORTED",
            ));
        }
        edits.push(replacement_edit(
            source,
            *condition_ranges.iter().next().expect("one range"),
            condition,
        )?);
        for cut in &template.cuts {
            let replacement = match cut.branch {
                CausalCutBranch::Then => postimage.clone(),
                CausalCutBranch::Else => otherwise.clone(),
                CausalCutBranch::Unconditional => unreachable!("checked above"),
            };
            edits.push(replacement_edit(source, cut.postimage_range, replacement)?);
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
    let synthesis =
        synthesize_typed_mechanism_goal_with_priors(&synthesis_request, applicable_operators)
            .map_err(|error| {
                CausalFrontendFailure::unsupported(format!("BOUNDED_COMPOSITION:{error}"))
            })?;
    let materialized_patch = materialize_python_synthesis(source, template, &synthesis)?;
    Ok((synthesis, materialized_patch))
}

fn combine_source_bound_patches(
    source: &str,
    patches: &[&MaterializedSourceBoundPatchIR],
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

fn build_source_bound_patch_variants(
    source: &str,
    alternatives: &[SourceBoundAlternativeReceiptIR],
) -> Result<Vec<SourceBoundPatchVariantIR>, CausalFrontendFailure> {
    let mut selections = vec![Vec::<usize>::new()];
    for alternative in alternatives {
        let mut choices = (1..=alternative.closure_candidates.len()).collect::<Vec<_>>();
        // Prefer the deepest safely transported dependency, but retain every
        // shallower dependency and the public owner as bounded fallbacks.
        choices.reverse();
        choices.push(0);
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
                patches.push(&alternative.materialized_patch);
                symbols.push(alternative.function_template.qualified_symbol.clone());
            } else {
                let candidate = alternative
                    .closure_candidates
                    .get(selected - 1)
                    .ok_or_else(|| {
                        CausalFrontendFailure::public("PATCH_VARIANT_CANDIDATE_INDEX")
                    })?;
                patches.push(&candidate.materialized_patch);
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
            materialized_patch,
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
            let candidate_response = run_python_host(
                &request.python_executable,
                &materialized_patch.candidate_source,
                std::slice::from_ref(&alternative.public_symbol),
            )?;
            if let Some(failure) = host_failure(&candidate_response) {
                return Err(failure);
            }
            if candidate_response.definitions.len() != 1
                || candidate_response.definitions[0].qualified_symbol != alternative.public_symbol
            {
                return Err(CausalFrontendFailure::public(
                    "MATERIALIZED_PUBLIC_SYMBOL_IDENTITY_LOST",
                ));
            }
            let mut closure_candidates = Vec::new();
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
                    &operator_type_index,
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
                let closure_candidate_response = run_python_host(
                    &request.python_executable,
                    &closure_patch.candidate_source,
                    std::slice::from_ref(&alternative.public_symbol),
                )?;
                if let Some(failure) = host_failure(&closure_candidate_response) {
                    closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                        closure_ordinal,
                        qualified_symbol: closure_template.qualified_symbol.clone(),
                        failure_kind: failure.kind,
                        detail: failure.detail,
                    });
                    continue;
                }
                if closure_candidate_response.definitions.len() != 1
                    || closure_candidate_response.definitions[0].qualified_symbol
                        != alternative.public_symbol
                {
                    closure_candidate_rejections.push(SourceBoundClosureCandidateRejectionIR {
                        closure_ordinal,
                        qualified_symbol: closure_template.qualified_symbol.clone(),
                        failure_kind: CausalFrontendFailureKind::PublicInformationInsufficient,
                        detail: "CLOSURE_MATERIALIZATION_PUBLIC_SYMBOL_IDENTITY_LOST".to_string(),
                    });
                    continue;
                }
                closure_candidates.push(SourceBoundClosureCandidateReceiptIR {
                    closure_ordinal,
                    public_operand_bindings: bindings,
                    function_template: closure_template,
                    synthesis: closure_synthesis,
                    materialized_patch: closure_patch,
                });
            }
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
                materialized_patch,
                closure_candidates,
                closure_candidate_rejections,
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
        public_symbol_owner_preserved: true,
        execution_dependency_closure_preserved: true,
        single_and_multi_edit_share_atomic_path: true,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(
        serde_json::to_vec(&receipt)
            .map_err(|error| CausalFrontendFailure::public(format!("RECEIPT_HASH:{error}")))?
            .as_slice(),
    );
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
                .materialized_patch
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
                    .materialized_patch
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
        assert!(by_symbol["count"]
            .materialized_patch
            .candidate_source
            .contains("len("));
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
            assert!(alternative
                .materialized_patch
                .candidate_source
                .contains('['));
            assert!(
                alternative
                    .materialized_patch
                    .candidate_materialization_is_one_to_one
            );
        }
        assert_eq!(
            by_symbol["row_at"].function_template.output_type,
            ProgramType::SequenceInt
        );
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
                public_observations: observations(&[(9, 4, 5), (2, 7, 5), (8, 8, 0)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let alternative = &receipt.alternatives[0];
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
        assert!(receipt.patch_variants[0]
            .materialized_patch
            .candidate_source
            .contains("parsed = parse_expr(left, right)"));
        assert!(receipt.patch_variants[0]
            .materialized_patch
            .candidate_source
            .contains("return evaluateFalse(first, second)"));
        assert!(alternative
            .materialized_patch
            .candidate_source
            .contains(" if "));
        assert!(matches!(
            alternative.materialized_patch.edit,
            SourceEditAtom::AtomicMultiEdit { .. }
        ));
        assert!(
            alternative
                .materialized_patch
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
                public_observations: observations(&[(9, 4, 5), (2, 7, 5), (8, 8, 0)]),
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
                public_observations: observations(&[(9, 4, 5), (2, 7, 5), (8, 8, 0)]),
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
                public_observations: observations(&[(7, 3, 4), (2, 8, 6), (5, 5, 0)]),
                allowed_effects: vec![Effect::Pure],
                require_conditional: true,
                max_expression_depth: 2,
                max_candidates: 1_024,
            }],
        };
        let receipt = analyze_and_synthesize_source_bound(&request).unwrap();
        let patch = &receipt.alternatives[0].materialized_patch;
        let SourceEditAtom::AtomicMultiEdit { edits } = &patch.edit else {
            panic!("atomic path required")
        };
        assert_eq!(edits.len(), 3);
        assert_eq!(
            apply_edit_atom(source, &patch.edit).unwrap(),
            patch.candidate_source
        );
        assert!(patch
            .candidate_source
            .starts_with("# 한글 byte span canary\r\n"));
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
        assert!(alternative
            .materialized_patch
            .candidate_source
            .contains("return ((left) + (right))"));
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
        let request = SourceBoundRepositoryDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA.to_string(),
            source_relative_path: PathBuf::from("calculator.py"),
            source: "def add(left, right):\n    return 0\n".to_string(),
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
        assert!(receipt.alternatives[0]
            .materialized_patch
            .candidate_source
            .contains("return ((left) + (right))"));
    }

    #[test]
    fn discovery_abstains_on_correct_or_ambiguous_code_but_accepts_diagnostic_target() {
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
            ..base
        };
        let receipt = discover_and_synthesize_python_repository(&targeted).unwrap();
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
