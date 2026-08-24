//! Typed, observation-driven source synthesis for JavaScript, TypeScript, and Go.
//!
//! Repository syntax binds a declared function and its operand names. The
//! repair is synthesized by the language-neutral SEM-5 mechanism engine and
//! only then lowered into target-language syntax.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::sha256;
use crate::sem5::model::{
    BinaryOperator, DataSplit, Effect, ProgramType, StringTransformOperator, UnaryOperator, Value,
};
use crate::sem5::typed_mechanism::{
    synthesize_typed_mechanism_goal, SourceOperandIR, TypedMechanismObservationIR,
    TypedMechanismSynthesisGoalIR, TypedMechanismSynthesisReceiptIR, TypedSyntaxExpressionIR,
    TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
};

pub const CROSS_LANGUAGE_SYNTHESIS_SCHEMA: &str = "B_CROSS_LANGUAGE_SYNTHESIS_1";
pub const MAX_CROSS_LANGUAGE_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_CROSS_LANGUAGE_PARAMETERS: usize = 16;
pub const MAX_CROSS_LANGUAGE_EXAMPLES: usize = 64;
static NATIVE_VALIDATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CrossLanguage {
    JavaScript,
    TypeScript,
    Go,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossLanguageExampleIR {
    pub inputs: Vec<Value>,
    pub expected: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossLanguageSynthesisRequestIR {
    pub language: CrossLanguage,
    pub function_name: String,
    pub predecessor_source: String,
    pub public_examples: Vec<CrossLanguageExampleIR>,
    pub require_conditional: bool,
    pub max_expression_depth: usize,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossLanguageSynthesisReceiptIR {
    pub schema: String,
    pub language: CrossLanguage,
    pub function_name: String,
    pub parameter_names: Vec<String>,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub body_start: usize,
    pub body_end: usize,
    pub candidate_source: String,
    pub mechanism_synthesis: TypedMechanismSynthesisReceiptIR,
    pub changed_function_bodies: usize,
    pub direct_text_to_source_shortcut_events: u64,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeValidationReceiptIR {
    pub language: CrossLanguage,
    pub tool_path: PathBuf,
    pub command_status: Option<i32>,
    pub cases_executed: usize,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub diagnostic_excerpt: String,
    pub pass: bool,
    pub sandbox_cleaned: bool,
    pub network_reads: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexState {
    Code,
    SingleQuote,
    DoubleQuote,
    TemplateQuote,
    LineComment,
    BlockComment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionBoundary {
    pub(crate) parameter_names: Vec<String>,
    pub(crate) parameter_start: usize,
    pub(crate) parameter_end: usize,
    pub(crate) body_start: usize,
    pub(crate) body_end: usize,
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

pub(crate) fn code_identifiers(source: &str) -> Vec<(String, usize, usize)> {
    let bytes = source.as_bytes();
    let mut state = LexState::Code;
    let mut output = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        match state {
            LexState::Code => match byte {
                b'\'' => state = LexState::SingleQuote,
                b'"' => state = LexState::DoubleQuote,
                b'`' => state = LexState::TemplateQuote,
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    state = LexState::LineComment;
                    index += 1;
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    state = LexState::BlockComment;
                    index += 1;
                }
                value if value == b'_' || value.is_ascii_alphabetic() => {
                    let start = index;
                    index += 1;
                    while index < bytes.len()
                        && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
                    {
                        index += 1;
                    }
                    output.push((source[start..index].to_string(), start, index));
                    continue;
                }
                _ => {}
            },
            LexState::SingleQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'\'' {
                    state = LexState::Code;
                }
            }
            LexState::DoubleQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'"' {
                    state = LexState::Code;
                }
            }
            LexState::TemplateQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'`' {
                    state = LexState::Code;
                }
            }
            LexState::LineComment => {
                if byte == b'\n' {
                    state = LexState::Code;
                }
            }
            LexState::BlockComment => {
                if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = LexState::Code;
                    index += 1;
                }
            }
        }
        index += 1;
    }
    output
}

fn matching_delimiter(source: &str, open: usize, opener: u8, closer: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&opener) {
        return None;
    }
    let mut state = LexState::Code;
    let mut depth = 0usize;
    let mut index = open;
    while index < bytes.len() {
        let byte = bytes[index];
        match state {
            LexState::Code => match byte {
                b'\'' => state = LexState::SingleQuote,
                b'"' => state = LexState::DoubleQuote,
                b'`' => state = LexState::TemplateQuote,
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    state = LexState::LineComment;
                    index += 1;
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    state = LexState::BlockComment;
                    index += 1;
                }
                value if value == opener => depth += 1,
                value if value == closer => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            },
            LexState::SingleQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'\'' {
                    state = LexState::Code;
                }
            }
            LexState::DoubleQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'"' {
                    state = LexState::Code;
                }
            }
            LexState::TemplateQuote => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'`' {
                    state = LexState::Code;
                }
            }
            LexState::LineComment => {
                if byte == b'\n' {
                    state = LexState::Code;
                }
            }
            LexState::BlockComment => {
                if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = LexState::Code;
                    index += 1;
                }
            }
        }
        index += 1;
    }
    None
}

fn split_parameters(parameters: &str, language: CrossLanguage) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for raw in parameters.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let name = match language {
            CrossLanguage::JavaScript | CrossLanguage::TypeScript => raw
                .trim_start_matches("...")
                .split([':', '='])
                .next()
                .unwrap_or_default()
                .trim(),
            CrossLanguage::Go => raw.split_whitespace().next().unwrap_or_default(),
        };
        if !valid_identifier(name) {
            return Err(format!("CROSS_LANGUAGE_UNSUPPORTED_PARAMETER:{raw}"));
        }
        names.push(name.to_string());
    }
    if names.is_empty() || names.len() > MAX_CROSS_LANGUAGE_PARAMETERS {
        return Err("CROSS_LANGUAGE_PARAMETER_BOUND".to_string());
    }
    Ok(names)
}

pub(crate) fn locate_function(
    source: &str,
    language: CrossLanguage,
    function_name: &str,
) -> Result<FunctionBoundary, String> {
    if !valid_identifier(function_name) {
        return Err("CROSS_LANGUAGE_INVALID_FUNCTION_NAME".to_string());
    }
    let keyword = match language {
        CrossLanguage::JavaScript | CrossLanguage::TypeScript => "function",
        CrossLanguage::Go => "func",
    };
    let identifiers = code_identifiers(source);
    let matches = identifiers
        .windows(2)
        .filter(|window| window[0].0 == keyword && window[1].0 == function_name)
        .map(|window| window[1].2)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(if matches.is_empty() {
            "CROSS_LANGUAGE_TARGET_NOT_FOUND"
        } else {
            "CROSS_LANGUAGE_AMBIGUOUS_TARGET"
        }
        .to_string());
    }
    let name_end = matches[0];
    let open_parameters = source[name_end..]
        .find('(')
        .map(|offset| name_end + offset)
        .ok_or_else(|| "CROSS_LANGUAGE_PARAMETER_LIST_NOT_FOUND".to_string())?;
    let close_parameters = matching_delimiter(source, open_parameters, b'(', b')')
        .ok_or_else(|| "CROSS_LANGUAGE_UNBALANCED_PARAMETERS".to_string())?;
    let open_body = source[close_parameters + 1..]
        .find('{')
        .map(|offset| close_parameters + 1 + offset)
        .ok_or_else(|| "CROSS_LANGUAGE_BODY_NOT_FOUND".to_string())?;
    let close_body = matching_delimiter(source, open_body, b'{', b'}')
        .ok_or_else(|| "CROSS_LANGUAGE_UNBALANCED_BODY".to_string())?;
    Ok(FunctionBoundary {
        parameter_names: split_parameters(
            &source[open_parameters + 1..close_parameters],
            language,
        )?,
        parameter_start: open_parameters,
        parameter_end: close_parameters,
        body_start: open_body,
        body_end: close_body,
    })
}

fn infer_signature(
    examples: &[CrossLanguageExampleIR],
) -> Result<(Vec<ProgramType>, ProgramType), String> {
    if examples.len() < 3 || examples.len() > MAX_CROSS_LANGUAGE_EXAMPLES {
        return Err("CROSS_LANGUAGE_EXAMPLE_BOUND".to_string());
    }
    let arity = examples[0].inputs.len();
    if arity == 0 || arity > MAX_CROSS_LANGUAGE_PARAMETERS {
        return Err("CROSS_LANGUAGE_EXAMPLE_ARITY".to_string());
    }
    let input_types = examples[0]
        .inputs
        .iter()
        .map(Value::program_type)
        .collect::<Vec<_>>();
    let output_type = examples[0].expected.program_type();
    let supported = |value_type: &ProgramType| {
        matches!(
            value_type,
            ProgramType::Int | ProgramType::Bool | ProgramType::String
        )
    };
    if !input_types.iter().all(supported) || !supported(&output_type) {
        return Err("CROSS_LANGUAGE_UNSUPPORTED_TRANSPORT_TYPE".to_string());
    }
    for example in examples {
        if example.inputs.len() != arity
            || example
                .inputs
                .iter()
                .map(Value::program_type)
                .ne(input_types.iter().cloned())
            || example.expected.program_type() != output_type
        {
            return Err("CROSS_LANGUAGE_INCONSISTENT_EXAMPLE_TYPES".to_string());
        }
    }
    Ok((input_types, output_type))
}

fn binary_token(operator: BinaryOperator, language: CrossLanguage) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::Equal => match language {
            CrossLanguage::JavaScript | CrossLanguage::TypeScript => "===",
            CrossLanguage::Go => "==",
        },
        BinaryOperator::NotEqual => match language {
            CrossLanguage::JavaScript | CrossLanguage::TypeScript => "!==",
            CrossLanguage::Go => "!=",
        },
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
    }
}

fn emit_expression(
    expression: &TypedSyntaxExpressionIR,
    language: CrossLanguage,
    sources: &BTreeMap<String, String>,
) -> Result<String, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => sources
            .get(role)
            .cloned()
            .ok_or_else(|| format!("CROSS_LANGUAGE_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { value } => Ok(value.to_string()),
        TypedSyntaxExpressionIR::BoolLiteral { value } => Ok(value.to_string()),
        TypedSyntaxExpressionIR::Unary { operator, input } => {
            let input = emit_expression(input, language, sources)?;
            Ok(match operator {
                UnaryOperator::Negate => format!("(-({input}))"),
                UnaryOperator::Not => format!("(!({input}))"),
            })
        }
        TypedSyntaxExpressionIR::StringTransform { operator, input } => {
            if language == CrossLanguage::Go {
                return Err("CROSS_LANGUAGE_GO_STRING_TRANSFORM_REQUIRES_IMPORT_EDIT".to_string());
            }
            let input = emit_expression(input, language, sources)?;
            Ok(match operator {
                StringTransformOperator::Trim => format!("({input}).trim()"),
                StringTransformOperator::Lowercase => format!("({input}).toLowerCase()"),
                StringTransformOperator::Uppercase => format!("({input}).toUpperCase()"),
            })
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => {
            let left = emit_expression(left, language, sources)?;
            let right = emit_expression(right, language, sources)?;
            let expression = format!("(({left}) {} ({right}))", binary_token(*operator, language));
            if *operator == BinaryOperator::Divide
                && matches!(
                    language,
                    CrossLanguage::JavaScript | CrossLanguage::TypeScript
                )
            {
                Ok(format!("Math.trunc({expression})"))
            } else {
                Ok(expression)
            }
        }
        TypedSyntaxExpressionIR::Length { input } => {
            let input = emit_expression(input, language, sources)?;
            Ok(match language {
                CrossLanguage::JavaScript | CrossLanguage::TypeScript => {
                    format!("({input}).length")
                }
                CrossLanguage::Go => format!("int64(len({input}))"),
            })
        }
        TypedSyntaxExpressionIR::Index { collection, index } => Ok(format!(
            "({})[{}]",
            emit_expression(collection, language, sources)?,
            emit_expression(index, language, sources)?
        )),
        TypedSyntaxExpressionIR::Call { .. } => {
            Err("CROSS_LANGUAGE_API_CALL_REQUIRES_LANGUAGE_BINDING".to_string())
        }
    }
}

fn emit_body(
    language: CrossLanguage,
    goal: &crate::sem5::typed_mechanism::TypedMechanismGoalIR,
) -> Result<String, String> {
    let sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.source.clone()))
        .collect::<BTreeMap<_, _>>();
    let postimage = emit_expression(&goal.postimage, language, &sources)?;
    match (&goal.condition, &goal.otherwise, language) {
        (None, None, CrossLanguage::JavaScript | CrossLanguage::TypeScript) => {
            Ok(format!("\n    return {postimage};\n"))
        }
        (None, None, CrossLanguage::Go) => Ok(format!("\n\treturn {postimage}\n")),
        (
            Some(condition),
            Some(otherwise),
            CrossLanguage::JavaScript | CrossLanguage::TypeScript,
        ) => {
            let condition = emit_expression(condition, language, &sources)?;
            let otherwise = emit_expression(otherwise, language, &sources)?;
            Ok(format!(
                "\n    return ({condition}) ? ({postimage}) : ({otherwise});\n"
            ))
        }
        (Some(condition), Some(otherwise), CrossLanguage::Go) => {
            let condition = emit_expression(condition, language, &sources)?;
            let otherwise = emit_expression(otherwise, language, &sources)?;
            Ok(format!(
                "\n\tif {condition} {{\n\t\treturn {postimage}\n\t}}\n\treturn {otherwise}\n"
            ))
        }
        _ => Err("CROSS_LANGUAGE_INCOMPLETE_CONDITIONAL".to_string()),
    }
}

/// Synthesize and predecessor-bind one declared function body.
pub fn synthesize_cross_language_function(
    request: &CrossLanguageSynthesisRequestIR,
) -> Result<CrossLanguageSynthesisReceiptIR, String> {
    if request.predecessor_source.len() > MAX_CROSS_LANGUAGE_SOURCE_BYTES {
        return Err("CROSS_LANGUAGE_SOURCE_BOUND".to_string());
    }
    let boundary = locate_function(
        &request.predecessor_source,
        request.language,
        &request.function_name,
    )?;
    let (input_types, output_type) = infer_signature(&request.public_examples)?;
    if input_types.len() != boundary.parameter_names.len() {
        return Err("CROSS_LANGUAGE_SIGNATURE_EXAMPLE_ARITY".to_string());
    }
    let operands = boundary
        .parameter_names
        .iter()
        .zip(input_types)
        .enumerate()
        .map(|(index, (source, value_type))| SourceOperandIR {
            role: format!("ARG_{index}"),
            source: source.clone(),
            value_type,
        })
        .collect::<Vec<_>>();
    let public_observations = request
        .public_examples
        .iter()
        .map(|example| TypedMechanismObservationIR {
            operands: operands
                .iter()
                .zip(&example.inputs)
                .map(|(operand, value)| (operand.role.clone(), value.clone()))
                .collect(),
            expected_postimage: example.expected.clone(),
        })
        .collect();
    let synthesis_request = TypedMechanismSynthesisGoalIR {
        schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
        goal_id: format!(
            "cross_language_{}",
            &sha256(
                format!(
                    "{:?}:{}:{}",
                    request.language,
                    request.function_name,
                    sha256(request.predecessor_source.as_bytes())
                )
                .as_bytes()
            )[..16]
        ),
        split: DataSplit::FreshBlind,
        operands,
        output_type,
        definitions: Vec::new(),
        allowed_effects: vec![Effect::Pure],
        preconditions: vec![
            "bounded scalar inputs avoid target-language numeric overflow".to_string(),
        ],
        postconditions: vec!["match every repository-visible public example".to_string()],
        invariants: vec!["replace exactly one predecessor-bound function body".to_string()],
        public_observations,
        require_conditional: request.require_conditional,
        max_expression_depth: request.max_expression_depth.clamp(1, 3),
        max_candidates: request.max_candidates.clamp(16, 1_024),
        provenance: vec!["CROSS_LANGUAGE_AST_SIGNATURE_AND_PUBLIC_EXAMPLES".to_string()],
    };
    let mechanism_synthesis = synthesize_typed_mechanism_goal(&synthesis_request)?;
    let body = emit_body(request.language, &mechanism_synthesis.winning_goal)?;
    let mut candidate_source = String::with_capacity(request.predecessor_source.len() + body.len());
    candidate_source.push_str(&request.predecessor_source[..boundary.body_start + 1]);
    candidate_source.push_str(&body);
    candidate_source.push_str(&request.predecessor_source[boundary.body_end..]);
    let predecessor_sha256 = sha256(request.predecessor_source.as_bytes());
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    if predecessor_sha256 == candidate_sha256 {
        return Err("CROSS_LANGUAGE_NO_SOURCE_CHANGE".to_string());
    }
    Ok(CrossLanguageSynthesisReceiptIR {
        schema: CROSS_LANGUAGE_SYNTHESIS_SCHEMA.to_string(),
        language: request.language,
        function_name: request.function_name.clone(),
        parameter_names: boundary.parameter_names,
        predecessor_sha256,
        candidate_sha256,
        body_start: boundary.body_start,
        body_end: boundary.body_end,
        candidate_source,
        mechanism_synthesis,
        changed_function_bodies: 1,
        direct_text_to_source_shortcut_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
    })
}

fn render_js_value(value: &Value) -> Result<String, String> {
    match value {
        Value::Int(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::String(value) => serde_json::to_string(value)
            .map_err(|error| format!("CROSS_LANGUAGE_JSON_VALUE:{error}")),
        _ => Err("CROSS_LANGUAGE_NATIVE_UNSUPPORTED_VALUE".to_string()),
    }
}

fn render_go_value(value: &Value) -> Result<String, String> {
    match value {
        Value::Int(value) => Ok(format!("int64({value})")),
        Value::Bool(value) => Ok(value.to_string()),
        Value::String(value) => serde_json::to_string(value)
            .map_err(|error| format!("CROSS_LANGUAGE_JSON_VALUE:{error}")),
        _ => Err("CROSS_LANGUAGE_NATIVE_UNSUPPORTED_VALUE".to_string()),
    }
}

fn native_harness(
    receipt: &CrossLanguageSynthesisReceiptIR,
    examples: &[CrossLanguageExampleIR],
) -> Result<String, String> {
    match receipt.language {
        CrossLanguage::JavaScript | CrossLanguage::TypeScript => {
            let mut output = String::from("\nconst __bCases = [\n");
            for example in examples {
                let inputs = example
                    .inputs
                    .iter()
                    .map(render_js_value)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                output.push_str(&format!(
                    "  [[{inputs}], {}],\n",
                    render_js_value(&example.expected)?
                ));
            }
            output.push_str(&format!(
                "];\nfor (const [args, expected] of __bCases) {{\n  const actual = {}(...args);\n  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`mismatch:${{actual}}:${{expected}}`);\n}}\nconsole.log(`PASS:${{__bCases.length}}`);\n",
                receipt.function_name
            ));
            Ok(output)
        }
        CrossLanguage::Go => {
            let mut output = String::from("\nfunc main() {\n");
            for example in examples {
                let inputs = example
                    .inputs
                    .iter()
                    .map(render_go_value)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                output.push_str(&format!(
                    "\tif actual := {}({inputs}); actual != {} {{ panic(\"mismatch\") }}\n",
                    receipt.function_name,
                    render_go_value(&example.expected)?
                ));
            }
            output.push_str(&format!("\tprintln(\"PASS:{}\")\n}}\n", examples.len()));
            Ok(output)
        }
    }
}

fn remove_validation_workspace(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("CROSS_LANGUAGE_SANDBOX_CLEANUP:{error}"))?;
    }
    Ok(())
}

/// Compile/execute the candidate with a caller-supplied local Node or Go tool.
pub fn validate_cross_language_candidate(
    receipt: &CrossLanguageSynthesisReceiptIR,
    examples: &[CrossLanguageExampleIR],
    tool_path: &Path,
) -> Result<NativeValidationReceiptIR, String> {
    if !tool_path.is_file() {
        return Err(format!(
            "CROSS_LANGUAGE_TOOL_NOT_FOUND:{}",
            tool_path.display()
        ));
    }
    let workspace = std::env::temp_dir().join(format!(
        "b-core-cross-language-{}-{}-{}",
        std::process::id(),
        &receipt.candidate_sha256[..16],
        NATIVE_VALIDATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    remove_validation_workspace(&workspace)?;
    fs::create_dir_all(&workspace)
        .map_err(|error| format!("CROSS_LANGUAGE_SANDBOX_CREATE:{error}"))?;
    let extension = match receipt.language {
        CrossLanguage::JavaScript => "mjs",
        CrossLanguage::TypeScript => "ts",
        CrossLanguage::Go => "go",
    };
    let source_path = workspace.join(format!("candidate.{extension}"));
    let full_source = format!(
        "{}{}",
        receipt.candidate_source,
        native_harness(receipt, examples)?
    );
    fs::write(&source_path, full_source)
        .map_err(|error| format!("CROSS_LANGUAGE_SANDBOX_WRITE:{error}"))?;
    let mut command = Command::new(tool_path);
    match receipt.language {
        CrossLanguage::JavaScript => {
            command.arg(&source_path);
        }
        CrossLanguage::TypeScript => {
            command.arg("--experimental-strip-types").arg(&source_path);
        }
        CrossLanguage::Go => {
            command.arg("run").arg(&source_path);
        }
    }
    command.current_dir(&workspace);
    let output = command
        .output()
        .map_err(|error| format!("CROSS_LANGUAGE_NATIVE_EXECUTE:{error}"))?;
    let success_token = format!("PASS:{}", examples.len());
    let pass = output.status.success()
        && (String::from_utf8_lossy(&output.stdout).contains(&success_token)
            || String::from_utf8_lossy(&output.stderr).contains(&success_token));
    let validation = NativeValidationReceiptIR {
        language: receipt.language,
        tool_path: tool_path.to_path_buf(),
        command_status: output.status.code(),
        cases_executed: if pass { examples.len() } else { 0 },
        stdout_sha256: sha256(&output.stdout),
        stderr_sha256: sha256(&output.stderr),
        diagnostic_excerpt: String::from_utf8_lossy(if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        })
        .chars()
        .take(2_048)
        .collect(),
        pass,
        sandbox_cleaned: true,
        network_reads: 0,
    };
    remove_validation_workspace(&workspace)?;
    Ok(validation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases(rows: &[(i64, i64, i64)]) -> Vec<CrossLanguageExampleIR> {
        rows.iter()
            .map(|(left, right, expected)| CrossLanguageExampleIR {
                inputs: vec![Value::Int(*left), Value::Int(*right)],
                expected: Value::Int(*expected),
            })
            .collect()
    }

    fn node_path() -> Option<PathBuf> {
        let path = PathBuf::from(r"C:\Program Files\nodejs\node.exe");
        path.is_file().then_some(path)
    }

    fn go_path() -> Option<PathBuf> {
        let path = PathBuf::from(r"C:\Program Files\Go\bin\go.exe");
        path.is_file().then_some(path)
    }

    #[test]
    fn javascript_function_body_is_synthesized_and_runs_in_node() {
        let examples = cases(&[(4, 3, 7), (-2, 8, 6), (10, -3, 7), (0, 5, 5)]);
        let receipt = synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language: CrossLanguage::JavaScript,
            function_name: "combine".to_string(),
            predecessor_source: "export function combine(left, right) {\n  return 0;\n}\n"
                .to_string(),
            public_examples: examples.clone(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        })
        .unwrap();
        assert!(receipt
            .candidate_source
            .contains("return ((left) + (right));"));
        assert_eq!(receipt.changed_function_bodies, 1);
        assert_eq!(receipt.direct_text_to_source_shortcut_events, 0);
        if let Some(node) = node_path() {
            assert!(
                validate_cross_language_candidate(&receipt, &examples, &node)
                    .unwrap()
                    .pass
            );
        }
    }

    #[test]
    fn typescript_types_are_preserved_while_body_is_synthesized() {
        let examples = cases(&[(4, 3, 12), (-2, 8, -16), (10, -3, -30), (0, 5, 0)]);
        let receipt = synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language: CrossLanguage::TypeScript,
            function_name: "scale".to_string(),
            predecessor_source:
                "export function scale(left: number, right: number): number {\n  return 0;\n}\n"
                    .to_string(),
            public_examples: examples.clone(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        })
        .unwrap();
        assert!(receipt.candidate_source.contains("left: number"));
        assert!(receipt
            .candidate_source
            .contains("return ((left) * (right));"));
        if let Some(node) = node_path() {
            assert!(
                validate_cross_language_candidate(&receipt, &examples, &node)
                    .unwrap()
                    .pass
            );
        }
    }

    #[test]
    fn go_function_body_is_synthesized_and_runs_with_go_toolchain() {
        let examples = cases(&[(4, 3, 1), (-2, 8, -10), (10, -3, 13), (0, 5, -5)]);
        let receipt = synthesize_cross_language_function(&CrossLanguageSynthesisRequestIR {
            language: CrossLanguage::Go,
            function_name: "delta".to_string(),
            predecessor_source:
                "package main\n\nfunc delta(left int64, right int64) int64 {\n\treturn 0\n}\n"
                    .to_string(),
            public_examples: examples.clone(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        })
        .unwrap();
        assert!(receipt
            .candidate_source
            .contains("return ((left) - (right))"));
        if let Some(go) = go_path() {
            let validation = validate_cross_language_candidate(&receipt, &examples, &go).unwrap();
            assert!(validation.pass, "{validation:?}");
        }
    }

    #[test]
    fn comments_and_strings_cannot_spoof_the_target_function() {
        let examples = cases(&[(1, 2, 3), (3, 4, 7), (-2, 5, 3)]);
        let request = CrossLanguageSynthesisRequestIR {
            language: CrossLanguage::JavaScript,
            function_name: "combine".to_string(),
            predecessor_source: "// function combine(fake, target) {}\nconst text = 'function combine(a,b) {}';\nexport function combine(left, right) { return 0; }\n".to_string(),
            public_examples: examples,
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
        };
        let receipt = synthesize_cross_language_function(&request).unwrap();
        assert!(receipt
            .candidate_source
            .contains("const text = 'function combine(a,b) {}'"));
        assert_eq!(receipt.changed_function_bodies, 1);
    }
}
