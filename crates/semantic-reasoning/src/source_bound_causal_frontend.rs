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

use crate::self_repair_contract::sha256;
use crate::sem5::model::{BinaryOperator, DataSplit, Effect, ProgramType, UnaryOperator};
use crate::sem5::typed_mechanism::{
    synthesize_typed_mechanism_goal, SourceOperandIR, TypedMechanismObservationIR,
    TypedMechanismSynthesisGoalIR, TypedMechanismSynthesisReceiptIR, TypedSyntaxExpressionIR,
    TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
};
use crate::structural_source_repair::{apply_edit_atom, ByteRange, SourceEditAtom};

pub const SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_REQUEST_1";
pub const SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA: &str = "B_CORE_SOURCE_BOUND_CAUSAL_RECEIPT_1";
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CAUSAL_ALTERNATIVES: usize = 32;
const MAX_DEPENDENCY_CLOSURE: usize = 64;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundCausalReceiptIR {
    pub schema: String,
    pub source_relative_path: PathBuf,
    pub language_backend: SourceLanguageBackend,
    pub predecessor_sha256: String,
    pub alternatives: Vec<SourceBoundAlternativeReceiptIR>,
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
}

#[derive(Debug, Deserialize)]
struct PythonOperand {
    name: String,
    annotation: String,
}

#[derive(Debug, Deserialize)]
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
    compile(tree, "<b-core-source-bound>", "exec", ast.PyCF_ONLY_AST)
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
    if node.args.vararg is not None or node.args.kwarg is not None:
        emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_VARIADIC_PUBLIC_SYMBOL:" + qualified)
    unsupported = (ast.Yield, ast.YieldFrom, ast.NamedExpr)
    if any(isinstance(item, unsupported) for item in ast.walk(node)):
        emit_failure("UNSUPPORTED_LANGUAGE_SYNTAX", "PYTHON_UNSUPPORTED_NODE:" + qualified)
    effects = set()
    for item in ast.walk(node):
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
        "return_annotation": annotation(node.returns), "effects": sorted(effects)
    }

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
    for item in ast.walk(definition["node"]):
        if isinstance(item, ast.Call):
            resolved = resolve_call(qualified, item)
            if resolved and resolved != qualified: dependencies.add(resolved)
    definition["direct_dependencies"] = sorted(dependencies)

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

selected = []
for requested in symbols:
    if requested not in definitions:
        emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "EXACT_PUBLIC_SYMBOL_NOT_FOUND:" + str(requested))
    closure, pending = [], [requested]
    seen = set()
    while pending:
        current = pending.pop(0)
        if current in seen: continue
        seen.add(current); closure.append(current)
        if len(closure) > max_closure:
            emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "DEPENDENCY_CLOSURE_BUDGET:" + requested)
        pending.extend(dependency for dependency in definitions[current]["direct_dependencies"] if dependency not in seen)
    definition = definitions[requested]
    cuts = cuts_for(definition)
    if not cuts:
        emit_failure("PUBLIC_INFORMATION_INSUFFICIENT", "PUBLIC_SYMBOL_POSTIMAGE_MISSING:" + requested)
    selected.append({
        "qualified_symbol": requested, "owner": definition["owner"],
        "is_async": definition["is_async"], "operands": definition["operands"],
        "return_annotation": definition["return_annotation"], "effects": definition["effects"],
        "direct_dependencies": definition["direct_dependencies"],
        "execution_dependency_closure": closure, "cuts": cuts,
    })

json.dump({"ok": True, "definitions": selected}, sys.stdout, ensure_ascii=False)
"#;

fn map_python_type(annotation: &str) -> Option<ProgramType> {
    match annotation.trim() {
        "int" | "builtins.int" => Some(ProgramType::Int),
        "bool" | "builtins.bool" => Some(ProgramType::Bool),
        "bytes" | "builtins.bytes" => Some(ProgramType::Bytes),
        "None" | "NoneType" => Some(ProgramType::Unit),
        _ => None,
    }
}

fn run_python_host(
    executable: &Path,
    source: &str,
    symbols: &[String],
) -> Result<PythonHostResponse, CausalFrontendFailure> {
    if !executable.is_file() {
        return Err(CausalFrontendFailure::public(format!(
            "PYTHON_EXECUTABLE_MISSING:{}",
            executable.display()
        )));
    }
    let input = serde_json::to_vec(&serde_json::json!({
        "source": source,
        "symbols": symbols,
        "max_closure": MAX_DEPENDENCY_CLOSURE,
    }))
    .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_INPUT:{error}")))?;
    let mut child = Command::new(executable)
        // `-X utf8` is required on Windows hosts whose redirected stdio
        // inherits a legacy code page.  Without it a valid Korean identifier
        // can enter Python as surrogate-escaped bytes and invalidate the AST
        // byte spans even though the Rust request is valid UTF-8.
        .args(["-X", "utf8", "-I", "-S", "-c", PYTHON_AST_HOST])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CausalFrontendFailure::public(format!("PYTHON_HOST_SPAWN:{error}")))?;
    child
        .stdin
        .take()
        .ok_or_else(|| CausalFrontendFailure::public("PYTHON_HOST_STDIN_MISSING"))?
        .write_all(&input)
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
    serde_json::from_slice(&stdout)
        .map_err(|error| CausalFrontendFailure::unsupported(format!("PYTHON_HOST_OUTPUT:{error}")))
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

fn convert_python_definition(
    definition: PythonFunctionDefinition,
) -> Result<SourceBoundFunctionTemplateIR, CausalFrontendFailure> {
    let operands = definition
        .operands
        .into_iter()
        .map(|operand| {
            let value_type = map_python_type(&operand.annotation).ok_or_else(|| {
                CausalFrontendFailure::public(format!(
                    "PUBLIC_OPERAND_TYPE_MISSING:{}:{}",
                    definition.qualified_symbol, operand.name
                ))
            })?;
            Ok(SourceOperandIR {
                role: operand.name.clone(),
                source: operand.name,
                value_type,
            })
        })
        .collect::<Result<Vec<_>, CausalFrontendFailure>>()?;
    if operands.is_empty() {
        return Err(CausalFrontendFailure::public(format!(
            "PUBLIC_OPERANDS_MISSING:{}",
            definition.qualified_symbol
        )));
    }
    let output_type = map_python_type(&definition.return_annotation).ok_or_else(|| {
        CausalFrontendFailure::public(format!(
            "PUBLIC_OUTPUT_TYPE_MISSING:{}",
            definition.qualified_symbol
        ))
    })?;
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
        effects: definition.effects,
        direct_dependencies: definition.direct_dependencies,
        execution_dependency_closure: definition.execution_dependency_closure,
        cuts,
        source_template_sha256,
    })
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
    let mut receipts = Vec::with_capacity(request.alternatives.len());
    for alternative in &request.alternatives {
        let definition = definitions.get(&alternative.public_symbol).ok_or_else(|| {
            CausalFrontendFailure::public(format!(
                "EXACT_PUBLIC_SYMBOL_NOT_RETURNED:{}",
                alternative.public_symbol
            ))
        })?;
        let function_template = convert_python_definition(PythonFunctionDefinition {
            qualified_symbol: definition.qualified_symbol.clone(),
            owner: definition.owner.clone(),
            is_async: definition.is_async,
            operands: definition
                .operands
                .iter()
                .map(|operand| PythonOperand {
                    name: operand.name.clone(),
                    annotation: operand.annotation.clone(),
                })
                .collect(),
            return_annotation: definition.return_annotation.clone(),
            effects: definition.effects.clone(),
            direct_dependencies: definition.direct_dependencies.clone(),
            execution_dependency_closure: definition.execution_dependency_closure.clone(),
            cuts: definition
                .cuts
                .iter()
                .map(|cut| PythonCut {
                    branch: cut.branch.clone(),
                    condition_source: cut.condition_source.clone(),
                    condition_start: cut.condition_start,
                    condition_end: cut.condition_end,
                    postimage_source: cut.postimage_source.clone(),
                    postimage_start: cut.postimage_start,
                    postimage_end: cut.postimage_end,
                })
                .collect(),
        })?;
        if function_template.qualified_symbol != alternative.public_symbol
            || function_template.execution_dependency_closure.first()
                != Some(&alternative.public_symbol)
        {
            return Err(CausalFrontendFailure::public(
                "PUBLIC_SYMBOL_OR_CLOSURE_IDENTITY_LOST",
            ));
        }
        let synthesis_request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: alternative.alternative_id.clone(),
            split: DataSplit::FreshBlind,
            operands: function_template.operands.clone(),
            output_type: function_template.output_type.clone(),
            definitions: Vec::new(),
            allowed_effects: if alternative.allowed_effects.is_empty() {
                vec![Effect::Pure]
            } else {
                alternative.allowed_effects.clone()
            },
            preconditions: vec![format!(
                "exact public symbol {} is source bound",
                alternative.public_symbol
            )],
            postconditions: vec!["satisfy all public postimage observations".to_string()],
            invariants: vec![
                "preserve exact public symbol owner".to_string(),
                "preserve same-file execution dependency closure".to_string(),
            ],
            public_observations: alternative.public_observations.clone(),
            require_conditional: alternative.require_conditional,
            max_expression_depth: alternative.max_expression_depth,
            max_candidates: alternative.max_candidates,
            provenance: vec![
                "PYTHON_AST_SOURCE_BOUND_CAUSAL_CUT".to_string(),
                format!(
                    "SOURCE_TEMPLATE_SHA256:{}",
                    function_template.source_template_sha256
                ),
            ],
        };
        let synthesis = synthesize_typed_mechanism_goal(&synthesis_request).map_err(|error| {
            CausalFrontendFailure::unsupported(format!("BOUNDED_COMPOSITION:{error}"))
        })?;
        let materialized_patch =
            materialize_python_synthesis(&request.source, &function_template, &synthesis)?;
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
        receipts.push(SourceBoundAlternativeReceiptIR {
            alternative_id: alternative.alternative_id.clone(),
            requested_public_symbol: alternative.public_symbol.clone(),
            function_template,
            synthesis,
            materialized_patch,
        });
    }
    let mut receipt = SourceBoundCausalReceiptIR {
        schema: SOURCE_BOUND_CAUSAL_RECEIPT_SCHEMA.to_string(),
        source_relative_path: request.source_relative_path.clone(),
        language_backend: backend,
        predecessor_sha256: sha256(request.source.as_bytes()),
        alternatives: receipts,
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

def parse_expr(left: int, right: int) -> int:
    return evaluateFalse(left, right)

def evaluateFalse(left: int, right: int) -> int:
    return transformer_visitor(left, right)

def transformer_visitor(left: int, right: int) -> int:
    return left + right
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
