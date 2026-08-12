use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use super::model::{
    ApiDefinition, BinaryOperator, Effect, ImageValue, NodeKind, ProgramIR, ProgramNode,
    ProgramType, ScalarExpression, StringTransformOperator, UnaryOperator, Value,
};

pub const CALLABLE_SOURCE_SCHEMA_REVISION: u64 = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustArtifact {
    pub program_id: String,
    pub source: String,
    pub source_sha256: String,
    pub reads_input_file: bool,
    pub writes_output_file: bool,
}

pub fn emit_rust(
    ir: &ProgramIR,
    apis: &[ApiDefinition],
    inputs: &BTreeMap<String, Value>,
) -> Result<RustArtifact, String> {
    let mut output = String::new();
    output.push_str("#[derive(Clone, Debug)]\nstruct Sem5Image { width: usize, height: usize, channels: usize, pixels: Vec<i64> }\n\n");
    for api in apis {
        output.push_str(&emit_api(api)?);
        output.push('\n');
    }
    output.push_str("fn main() {\n");
    let reads_input_file = ir
        .inputs
        .iter()
        .any(|binding| binding.value_type == ProgramType::Bytes);
    for binding in &ir.inputs {
        let value = inputs
            .get(&binding.name)
            .ok_or_else(|| format!("EMITTER_MISSING_INPUT:{}", binding.name))?;
        if value.program_type() != binding.value_type {
            return Err(format!("EMITTER_INPUT_TYPE:{}", binding.name));
        }
        let initializer = if binding.value_type == ProgramType::Bytes {
            "std::fs::read(\"input.bin\").expect(\"sandbox input\")".to_string()
        } else {
            rust_literal(value)?
        };
        output.push_str(&format!(
            "    let mut {}: {} = {};\n",
            binding.name,
            rust_type(&binding.value_type),
            initializer
        ));
    }
    let mut context = EmitContext {
        declared: ir
            .inputs
            .iter()
            .map(|binding| binding.name.clone())
            .collect(),
        binding_types: ir
            .inputs
            .iter()
            .map(|binding| (binding.name.clone(), binding.value_type.clone()))
            .collect(),
        next_loop: 0,
        result_declared: false,
        lint_clean: false,
    };
    emit_statement(&ir.root, 1, &mut context, &mut output)?;
    if !context.result_declared {
        return Err("EMITTER_NO_RETURN".to_string());
    }
    let writes_output_file = ir.allowed_effects.contains(&Effect::SandboxFileWrite)
        && ir.output_type == ProgramType::Bytes;
    if writes_output_file {
        output.push_str(
            "    std::fs::write(\"output.bin\", &sem5_result).expect(\"sandbox output\");\n",
        );
    }
    output.push_str(&format_output(&ir.output_type));
    output.push_str("}\n");
    let source_sha256 = hex_sha256(output.as_bytes());
    Ok(RustArtifact {
        program_id: ir.program_id.clone(),
        source: output,
        source_sha256,
        reads_input_file,
        writes_output_file,
    })
}

/// Emits a repository-native callable instead of a demonstration-bound
/// `main`.  The generated function accepts the same typed value transport as
/// the interpreter, so a successfully installed composition can participate
/// in later reasoning rather than merely compiling as unreachable code.
pub fn emit_rust_callable(
    ir: &ProgramIR,
    apis: &[ApiDefinition],
    program_ir_sha256: &str,
) -> Result<RustArtifact, String> {
    if program_ir_sha256.len() != 64
        || !program_ir_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("CALLABLE_PROGRAM_IR_SHA256_INVALID".to_string());
    }
    let mut output = String::new();
    output.push_str("#![allow(dead_code, unused_imports, unused_parens, unused_variables)]\n\n");
    output.push_str("use std::collections::BTreeMap;\n");
    output.push_str("use crate::sem5::model::{ImageValue, Value};\n\n");
    output.push_str("#[derive(Clone, Debug)]\nstruct Sem5Image { width: usize, height: usize, channels: usize, pixels: Vec<i64> }\n\n");
    for api in apis {
        output.push_str(&emit_api(api)?);
        output.push('\n');
    }
    output.push_str("pub const GENERATED_CAPABILITY_ACTIVE: bool = true;\n");
    output.push_str(&format!(
        "pub const GENERATED_SOURCE_SCHEMA_REVISION: u64 = {CALLABLE_SOURCE_SCHEMA_REVISION};\n"
    ));
    output.push_str(&format!(
        "pub const GENERATED_PROGRAM_ID: &str = {:?};\n",
        ir.program_id
    ));
    output.push_str(&format!(
        "pub const GENERATED_PROGRAM_IR_SHA256: &str = {:?};\n\n",
        program_ir_sha256
    ));
    output.push_str(
        "pub fn run_generated_capability(inputs: &BTreeMap<String, Value>) -> Result<Value, String> {\n",
    );
    for binding in &ir.inputs {
        let initializer = callable_input_initializer(&binding.name, &binding.value_type);
        output.push_str(&format!(
            "    let {}{}: {} = {};\n",
            if binding.mutable { "mut " } else { "" },
            binding.name,
            rust_type(&binding.value_type),
            initializer
        ));
    }
    let mut context = EmitContext {
        declared: ir
            .inputs
            .iter()
            .map(|binding| binding.name.clone())
            .collect(),
        binding_types: ir
            .inputs
            .iter()
            .map(|binding| (binding.name.clone(), binding.value_type.clone()))
            .collect(),
        next_loop: 0,
        result_declared: false,
        lint_clean: true,
    };
    emit_statement(&ir.root, 1, &mut context, &mut output)?;
    if !context.result_declared {
        return Err("EMITTER_NO_RETURN".to_string());
    }
    output.push_str(&callable_output(&ir.output_type));
    output.push_str("}\n");
    let source_sha256 = hex_sha256(output.as_bytes());
    Ok(RustArtifact {
        program_id: ir.program_id.clone(),
        source: output,
        source_sha256,
        reads_input_file: false,
        writes_output_file: false,
    })
}

fn callable_input_initializer(name: &str, value_type: &ProgramType) -> String {
    let access = format!("inputs.get({name:?})");
    let expected = match value_type {
        ProgramType::Int => "Some(Value::Int(value)) => *value".to_string(),
        ProgramType::Bool => "Some(Value::Bool(value)) => *value".to_string(),
        ProgramType::String => "Some(Value::String(value)) => value.clone()".to_string(),
        ProgramType::SequenceInt => {
            "Some(Value::Sequence(value)) => value.clone()".to_string()
        }
        ProgramType::NestedSequenceInt => {
            "Some(Value::NestedSequence(value)) => value.clone()".to_string()
        }
        ProgramType::Bytes => "Some(Value::Bytes(value)) => value.clone()".to_string(),
        ProgramType::Image => "Some(Value::Image(value)) => Sem5Image { width: value.width, height: value.height, channels: value.channels, pixels: value.pixels.clone() }".to_string(),
        ProgramType::Unit => "Some(Value::Unit) => ()".to_string(),
    };
    format!(
        "match {access} {{ {expected}, _ => return Err({:?}.to_string()) }}",
        format!("GENERATED_CAPABILITY_INPUT_TYPE:{name}")
    )
}

fn callable_output(output_type: &ProgramType) -> String {
    let expression = match output_type {
        ProgramType::Int => "Value::Int(sem5_result)",
        ProgramType::Bool => "Value::Bool(sem5_result)",
        ProgramType::String => "Value::String(sem5_result)",
        ProgramType::SequenceInt => "Value::Sequence(sem5_result)",
        ProgramType::NestedSequenceInt => "Value::NestedSequence(sem5_result)",
        ProgramType::Bytes => "Value::Bytes(sem5_result)",
        ProgramType::Image => "Value::Image(ImageValue { width: sem5_result.width, height: sem5_result.height, channels: sem5_result.channels, pixels: sem5_result.pixels })",
        ProgramType::Unit => "Value::Unit",
    };
    format!("    Ok({expression})\n")
}

fn emit_api(api: &ApiDefinition) -> Result<String, String> {
    if api
        .inputs
        .iter()
        .any(|input| !matches!(input, ProgramType::Int | ProgramType::Bool))
        || !matches!(api.output, ProgramType::Int | ProgramType::Bool)
        || api.effect != Effect::Pure
    {
        return Err(format!(
            "RUST_SCALAR_API_SIGNATURE_UNSUPPORTED:{}",
            api.api_token
        ));
    }
    let parameters = api
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| format!("a{index}: {}", rust_type(input)))
        .collect::<Vec<_>>()
        .join(", ");
    let body = emit_scalar_expression(&api.formal_body)?;
    Ok(format!(
        "fn {}({parameters}) -> {} {{ {body} }}\n",
        api.api_token,
        rust_type(&api.output)
    ))
}

struct EmitContext {
    declared: BTreeSet<String>,
    binding_types: BTreeMap<String, ProgramType>,
    next_loop: usize,
    result_declared: bool,
    lint_clean: bool,
}

fn emit_statement(
    node: &ProgramNode,
    indent: usize,
    context: &mut EmitContext,
    output: &mut String,
) -> Result<(), String> {
    let prefix = "    ".repeat(indent);
    match &node.kind {
        NodeKind::Store { name, value } => {
            let expression = emit_expression_mode(value, context.lint_clean)?;
            if context.declared.insert(name.clone()) {
                context
                    .binding_types
                    .insert(name.clone(), value.meta.output_type.clone());
                output.push_str(&format!(
                    "{prefix}let mut {name}: {} = {expression};\n",
                    rust_type(&value.meta.output_type)
                ));
            } else if let Some(assignment) = compound_assignment(name, value, context.lint_clean)? {
                output.push_str(&format!("{prefix}{assignment};\n"));
            } else {
                output.push_str(&format!("{prefix}{name} = {expression};\n"));
            }
        }
        NodeKind::SequenceWrite {
            binding,
            index,
            value,
        } => {
            let index = emit_expression_mode(index, context.lint_clean)?;
            let value = emit_expression_mode(value, context.lint_clean)?;
            match context.binding_types.get(binding) {
                Some(ProgramType::Bytes) => output.push_str(&format!(
                    "{prefix}{binding}[({index}) as usize] = ({value}) as u8;\n"
                )),
                Some(ProgramType::Image) => output.push_str(&format!(
                    "{prefix}{binding}.pixels[({index}) as usize] = {value};\n"
                )),
                _ => output.push_str(&format!(
                    "{prefix}{binding}[({index}) as usize] = {value};\n"
                )),
            }
        }
        NodeKind::SequenceAppend { binding, value } => {
            let expression = emit_expression_mode(value, context.lint_clean)?;
            let expression = if context.lint_clean {
                strip_one_outer_pair(&expression)
            } else {
                expression
            };
            output.push_str(&format!("{prefix}{binding}.push({expression});\n"));
        }
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => {
            let condition = emit_expression_mode(condition, context.lint_clean)?;
            let condition = if context.lint_clean {
                strip_one_outer_pair(&condition)
            } else {
                condition
            };
            output.push_str(&format!("{prefix}if {condition} {{\n"));
            emit_statement(then_node, indent + 1, context, output)?;
            if context.lint_clean && statement_is_noop(else_node) {
                output.push_str(&format!("{prefix}}}\n"));
            } else {
                output.push_str(&format!("{prefix}}} else {{\n"));
                emit_statement(else_node, indent + 1, context, output)?;
                output.push_str(&format!("{prefix}}}\n"));
            }
        }
        NodeKind::Loop {
            source,
            item_binding,
            index_binding,
            body,
        } => {
            let loop_id = context.next_loop;
            context.next_loop += 1;
            let source_expression = emit_expression_mode(source, context.lint_clean)?;
            let raw_item = format!("raw_item_{loop_id}");
            let raw_index = format!("raw_index_{loop_id}");
            let iterator = if source.meta.output_type == ProgramType::Image {
                format!("({source_expression}).pixels.clone().into_iter()")
            } else if context.lint_clean {
                format!(
                    "{}.clone().into_iter()",
                    strip_one_outer_pair(&source_expression)
                )
            } else {
                format!("({source_expression}).clone().into_iter()")
            };
            output.push_str(&format!(
                "{prefix}for ({raw_index}, {raw_item}) in {iterator}.enumerate() {{\n"
            ));
            let item_type = match source.meta.output_type {
                ProgramType::NestedSequenceInt => ProgramType::SequenceInt,
                _ => ProgramType::Int,
            };
            let item_expression = if source.meta.output_type == ProgramType::Bytes {
                format!("i64::from({raw_item})")
            } else {
                raw_item
            };
            output.push_str(&format!(
                "{}let {item_binding}: {} = {item_expression};\n",
                "    ".repeat(indent + 1),
                rust_type(&item_type)
            ));
            output.push_str(&format!(
                "{}let {index_binding}: i64 = {raw_index} as i64;\n",
                "    ".repeat(indent + 1)
            ));
            context.declared.insert(item_binding.clone());
            context.declared.insert(index_binding.clone());
            context
                .binding_types
                .insert(item_binding.clone(), item_type);
            context
                .binding_types
                .insert(index_binding.clone(), ProgramType::Int);
            emit_statement(body, indent + 1, context, output)?;
            output.push_str(&format!("{prefix}}}\n"));
        }
        NodeKind::Return { value } => {
            let keyword = if context.result_declared { "" } else { "let " };
            output.push_str(&format!(
                "{prefix}{keyword}sem5_result: {} = {};\n",
                rust_type(&value.meta.output_type),
                emit_expression_mode(value, context.lint_clean)?
            ));
            context.result_declared = true;
        }
        NodeKind::Block { nodes } => {
            for child in nodes {
                emit_statement(child, indent, context, output)?;
            }
        }
        NodeKind::Break => output.push_str(&format!("{prefix}break;\n")),
        NodeKind::Continue => output.push_str(&format!("{prefix}continue;\n")),
        NodeKind::Literal { value: Value::Unit } => {
            if !context.lint_clean {
                output.push_str(&format!("{prefix}();\n"));
            }
        }
        _ => {
            output.push_str(&format!(
                "{prefix}let _ = {};\n",
                emit_expression_mode(node, context.lint_clean)?
            ));
        }
    }
    Ok(())
}

fn strip_one_outer_pair(expression: &str) -> String {
    expression
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(expression)
        .to_string()
}

fn statement_is_noop(node: &ProgramNode) -> bool {
    match &node.kind {
        NodeKind::Literal { value: Value::Unit } => true,
        NodeKind::Block { nodes } => nodes.iter().all(statement_is_noop),
        _ => false,
    }
}

fn literal_int(node: &ProgramNode) -> Option<i64> {
    match &node.kind {
        NodeKind::Literal {
            value: Value::Int(value),
        } => Some(*value),
        _ => None,
    }
}

fn compound_assignment(
    binding: &str,
    value: &ProgramNode,
    lint_clean: bool,
) -> Result<Option<String>, String> {
    if !lint_clean {
        return Ok(None);
    }
    if value.meta.output_type == ProgramType::String {
        return Ok(None);
    }
    let NodeKind::BinaryOp {
        operator,
        left,
        right,
    } = &value.kind
    else {
        return Ok(None);
    };
    let left_binding = match &left.kind {
        NodeKind::Variable { name } | NodeKind::Load { name } => name,
        _ => return Ok(None),
    };
    if left_binding != binding {
        return Ok(None);
    }
    let token = match operator {
        BinaryOperator::Add => "+=",
        BinaryOperator::Subtract => "-=",
        BinaryOperator::Multiply => "*=",
        BinaryOperator::Divide => "/=",
        BinaryOperator::Modulo => "%=",
        BinaryOperator::And => "&=",
        BinaryOperator::Or => "|=",
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::LessThan
        | BinaryOperator::LessThanOrEqual
        | BinaryOperator::GreaterThan
        | BinaryOperator::GreaterThanOrEqual => {
            return Ok(None);
        }
    };
    let right = emit_expression_mode(right, true)?;
    Ok(Some(format!(
        "{binding} {token} {}",
        strip_one_outer_pair(&right)
    )))
}

fn emit_expression_mode(node: &ProgramNode, lint_clean: bool) -> Result<String, String> {
    match &node.kind {
        NodeKind::Literal { value } => rust_literal(value),
        NodeKind::Variable { name } | NodeKind::Load { name } => Ok(name.clone()),
        NodeKind::UnaryOp { operator, input } => {
            let operator = match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            };
            Ok(format!(
                "({operator}{})",
                emit_expression_mode(input, lint_clean)?
            ))
        }
        NodeKind::StringTransform { operator, input } => {
            let receiver = emit_postfix_receiver(input, lint_clean)?;
            Ok(match operator {
                StringTransformOperator::Trim => format!("{receiver}.trim().to_string()"),
                StringTransformOperator::Lowercase => format!("{receiver}.to_lowercase()"),
                StringTransformOperator::Uppercase => format!("{receiver}.to_uppercase()"),
            })
        }
        NodeKind::BinaryOp {
            operator,
            left,
            right,
        } => {
            if lint_clean {
                match (operator, literal_int(left), literal_int(right)) {
                    (BinaryOperator::Multiply, Some(1), _) => {
                        return emit_expression_mode(right, true);
                    }
                    (BinaryOperator::Multiply, _, Some(1))
                    | (BinaryOperator::Add, _, Some(0))
                    | (BinaryOperator::Subtract, _, Some(0))
                    | (BinaryOperator::Divide, _, Some(1)) => {
                        return emit_expression_mode(left, true);
                    }
                    (BinaryOperator::Add, Some(0), _) => {
                        return emit_expression_mode(right, true);
                    }
                    _ => {}
                }
            }
            let left_source = emit_expression_mode(left, lint_clean)?;
            let right_source = emit_expression_mode(right, lint_clean)?;
            if *operator == BinaryOperator::Add
                && left.meta.output_type == ProgramType::String
                && right.meta.output_type == ProgramType::String
            {
                Ok(format!(
                    "format!(\"{{}}{{}}\", {left_source}, {right_source})"
                ))
            } else {
                Ok(format!(
                    "({left_source} {} {right_source})",
                    binary_token(*operator)
                ))
            }
        }
        NodeKind::SequenceCreate { elements } => Ok(format!(
            "vec![{}]",
            elements
                .iter()
                .map(|element| emit_expression_mode(element, lint_clean))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        NodeKind::SequenceRead { sequence, index } => {
            let sequence_source = emit_postfix_receiver(sequence, lint_clean)?;
            if sequence.meta.output_type == ProgramType::String {
                return Ok(format!(
                    "{sequence_source}.chars().nth({} as usize).expect(\"typed string index\").to_string()",
                    emit_expression_mode(index, lint_clean)?
                ));
            }
            let access = if sequence.meta.output_type == ProgramType::Image {
                format!(
                    "{}.pixels[{} as usize]",
                    sequence_source,
                    emit_expression_mode(index, lint_clean)?
                )
            } else {
                format!(
                    "{}[{} as usize]",
                    sequence_source,
                    emit_expression_mode(index, lint_clean)?
                )
            };
            Ok(if node.meta.output_type == ProgramType::SequenceInt {
                format!("{access}.clone()")
            } else if sequence.meta.output_type == ProgramType::Bytes {
                format!("i64::from({access})")
            } else {
                access
            })
        }
        NodeKind::SequenceLength { sequence } => {
            let source = emit_postfix_receiver(sequence, lint_clean)?;
            if sequence.meta.output_type == ProgramType::Image {
                Ok(format!("{source}.pixels.len() as i64"))
            } else if sequence.meta.output_type == ProgramType::String {
                Ok(format!("{source}.chars().count() as i64"))
            } else {
                Ok(format!("{source}.len() as i64"))
            }
        }
        NodeKind::Call { api_token, args } => Ok(format!(
            "{}({})",
            api_token,
            args.iter()
                .map(|argument| emit_expression_mode(argument, lint_clean))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => Ok(format!(
            "if {} {{ {} }} else {{ {} }}",
            emit_expression_mode(condition, lint_clean)?,
            emit_expression_mode(then_node, lint_clean)?,
            emit_expression_mode(else_node, lint_clean)?
        )),
        NodeKind::Block { nodes } => Ok(format!(
            "{{ {} }}",
            nodes
                .iter()
                .map(|child| emit_expression_mode(child, lint_clean))
                .collect::<Result<Vec<_>, _>>()?
                .join("; ")
        )),
        _ => Err(format!("NODE_NOT_EXPRESSION:{}", node.meta.node_id)),
    }
}

fn emit_postfix_receiver(node: &ProgramNode, lint_clean: bool) -> Result<String, String> {
    let emitted = emit_expression_mode(node, lint_clean)?;
    Ok(match &node.kind {
        NodeKind::Variable { .. }
        | NodeKind::Load { .. }
        | NodeKind::SequenceCreate { .. }
        | NodeKind::SequenceRead { .. }
        | NodeKind::Call { .. } => emitted,
        _ => format!("({emitted})"),
    })
}

fn emit_scalar_expression(expression: &ScalarExpression) -> Result<String, String> {
    match expression {
        ScalarExpression::Argument { index } => Ok(format!("a{index}")),
        ScalarExpression::Constant { value } => Ok(format!("{value}i64")),
        ScalarExpression::BoolConstant { value } => Ok(value.to_string()),
        ScalarExpression::Unary { operator, input } => Ok(format!(
            "({}{})",
            match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            },
            emit_scalar_expression(input)?
        )),
        ScalarExpression::StringTransform { operator, input } => {
            let receiver = emit_scalar_expression(input)?;
            Ok(match operator {
                StringTransformOperator::Trim => format!("({receiver}).trim().to_string()"),
                StringTransformOperator::Lowercase => format!("({receiver}).to_lowercase()"),
                StringTransformOperator::Uppercase => format!("({receiver}).to_uppercase()"),
            })
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => Ok(format!(
            "({} {} {})",
            emit_scalar_expression(left)?,
            binary_token(*operator),
            emit_scalar_expression(right)?
        )),
        ScalarExpression::Length { .. } | ScalarExpression::Index { .. } => {
            Err("SCALAR_COLLECTION_EXPRESSION_REQUIRES_TYPED_PROGRAM_LOWERING".to_string())
        }
        ScalarExpression::OpaqueCall { api_token, args } => Ok(format!(
            "{}({})",
            api_token,
            args.iter()
                .map(emit_scalar_expression)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
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

fn rust_literal(value: &Value) -> Result<String, String> {
    match value {
        Value::Int(value) => Ok(format!("{value}i64")),
        Value::Bool(value) => Ok(value.to_string()),
        Value::String(value) => Ok(format!("{value:?}.to_string()")),
        Value::Sequence(values) => Ok(format!(
            "vec![{}]",
            values
                .iter()
                .map(|value| format!("{value}i64"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Value::NestedSequence(rows) => Ok(format!(
            "vec![{}]",
            rows.iter()
                .map(|row| format!(
                    "vec![{}]",
                    row.iter()
                        .map(|value| format!("{value}i64"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Value::Bytes(values) => Ok(format!(
            "vec![{}]",
            values
                .iter()
                .map(|value| format!("{value}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Value::Image(ImageValue {
            width,
            height,
            channels,
            pixels,
        }) => Ok(format!(
            "Sem5Image {{ width: {width}, height: {height}, channels: {channels}, pixels: vec![{}] }}",
            pixels
                .iter()
                .map(|value| format!("{value}i64"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Value::Unit => Ok("()".to_string()),
    }
}

fn format_output(output_type: &ProgramType) -> String {
    match output_type {
        ProgramType::Int | ProgramType::Bool | ProgramType::String => {
            "    println!(\"{}\", sem5_result);\n".to_string()
        }
        ProgramType::SequenceInt | ProgramType::Bytes => {
            "    println!(\"{:?}\", sem5_result);\n".to_string()
        }
        ProgramType::Image => "    println!(\"{}:{}:{}:{:?}\", sem5_result.width, sem5_result.height, sem5_result.channels, sem5_result.pixels);\n".to_string(),
        ProgramType::NestedSequenceInt => {
            "    println!(\"{:?}\", sem5_result);\n".to_string()
        }
        ProgramType::Unit => "    println!(\"unit\");\n".to_string(),
    }
}

pub fn render_value(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Sequence(values) => format!("{values:?}"),
        Value::NestedSequence(values) => format!("{values:?}"),
        Value::Bytes(values) => format!("{values:?}"),
        Value::Image(image) => format!(
            "{}:{}:{}:{:?}",
            image.width, image.height, image.channels, image.pixels
        ),
        Value::Unit => "unit".to_string(),
    }
}

pub fn emit_neutral_text(ir: &ProgramIR) -> String {
    fn visit(node: &ProgramNode, depth: usize, output: &mut String) {
        output.push_str(&format!(
            "{}{} : {:?} effects={:?}\n",
            "  ".repeat(depth),
            node.meta.node_id,
            node.meta.output_type,
            node.meta.effects
        ));
        for child in super_children(&node.kind) {
            visit(child, depth + 1, output);
        }
    }
    let mut output = format!("program {} -> {:?}\n", ir.program_id, ir.output_type);
    visit(&ir.root, 0, &mut output);
    output
}

fn super_children(kind: &NodeKind) -> Vec<&ProgramNode> {
    match kind {
        NodeKind::Store { value, .. }
        | NodeKind::UnaryOp { input: value, .. }
        | NodeKind::StringTransform { input: value, .. }
        | NodeKind::SequenceLength { sequence: value }
        | NodeKind::Return { value } => vec![value],
        NodeKind::BinaryOp { left, right, .. } => vec![left, right],
        NodeKind::SequenceCreate { elements } | NodeKind::Call { args: elements, .. } => {
            elements.iter().collect()
        }
        NodeKind::SequenceRead { sequence, index } => vec![sequence, index],
        NodeKind::SequenceWrite { index, value, .. } => vec![index, value],
        NodeKind::SequenceAppend { value, .. } => vec![value],
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => vec![condition, then_node, else_node],
        NodeKind::Loop { source, body, .. } => vec![source, body],
        NodeKind::Block { nodes } => nodes.iter().collect(),
        NodeKind::Literal { .. }
        | NodeKind::Variable { .. }
        | NodeKind::Load { .. }
        | NodeKind::Break
        | NodeKind::Continue => Vec::new(),
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem5::model::SynthesisCondition;
    use crate::sem5::{learner, tasks};

    fn test_node(node_id: &str, output_type: ProgramType, kind: NodeKind) -> ProgramNode {
        ProgramNode {
            meta: crate::sem5::model::NodeMeta {
                node_id: node_id.to_string(),
                input_types: Vec::new(),
                output_type,
                preconditions: Vec::new(),
                effects: Vec::new(),
                data_dependencies: Vec::new(),
                control_dependencies: Vec::new(),
                provenance: vec!["POSTFIX_PRECEDENCE_TEST".to_string()],
                primitive_cost: 1,
            },
            kind,
        }
    }

    #[test]
    fn postfix_lowering_omits_redundant_operand_parens_but_groups_branches() {
        let direct = test_node(
            "direct-length",
            ProgramType::Int,
            NodeKind::SequenceLength {
                sequence: Box::new(test_node(
                    "values",
                    ProgramType::SequenceInt,
                    NodeKind::Variable {
                        name: "values".to_string(),
                    },
                )),
            },
        );
        assert_eq!(
            emit_expression_mode(&direct, true).unwrap(),
            "values.len() as i64"
        );

        let branch = test_node(
            "branch-length",
            ProgramType::Int,
            NodeKind::SequenceLength {
                sequence: Box::new(test_node(
                    "branch",
                    ProgramType::SequenceInt,
                    NodeKind::If {
                        condition: Box::new(test_node(
                            "flag",
                            ProgramType::Bool,
                            NodeKind::Variable {
                                name: "flag".to_string(),
                            },
                        )),
                        then_node: Box::new(test_node(
                            "left",
                            ProgramType::SequenceInt,
                            NodeKind::SequenceCreate {
                                elements: vec![test_node(
                                    "one",
                                    ProgramType::Int,
                                    NodeKind::Literal {
                                        value: Value::Int(1),
                                    },
                                )],
                            },
                        )),
                        else_node: Box::new(test_node(
                            "right",
                            ProgramType::SequenceInt,
                            NodeKind::SequenceCreate {
                                elements: vec![test_node(
                                    "two",
                                    ProgramType::Int,
                                    NodeKind::Literal {
                                        value: Value::Int(2),
                                    },
                                )],
                            },
                        )),
                    },
                )),
            },
        );
        assert_eq!(
            emit_expression_mode(&branch, true).unwrap(),
            "(if flag { vec![1i64] } else { vec![2i64] }).len() as i64"
        );
    }

    #[test]
    fn rust_emission_is_deterministic_and_language_separated() {
        let sets = tasks::generate_task_sets(41);
        let candidates = learner::discover_candidates(&sets.discovery);
        let promotions = learner::initial_promotions(&candidates, &sets.calibration);
        let task = &sets.blind[0];
        let ir = learner::synthesize(
            &task.visible,
            SynthesisCondition::FirstPrinciplesD,
            &promotions,
        )
        .expect("synthesize");
        let cases = tasks::generate_property_cases(&task.visible, 41);
        let inputs = &cases[0];
        let first = emit_rust(&ir, &task.visible.definitions, inputs).expect("emit");
        let second = emit_rust(&ir, &task.visible.definitions, inputs).expect("emit");
        assert_eq!(first, second);
        assert!(first.source.contains("fn main()"));
        let ir_sha256 = hex_sha256(&serde_json::to_vec(&ir).expect("program ir"));
        let callable =
            emit_rust_callable(&ir, &task.visible.definitions, &ir_sha256).expect("callable Rust");
        assert!(callable
            .source
            .contains("pub const GENERATED_CAPABILITY_ACTIVE: bool = true"));
        assert!(callable
            .source
            .contains("pub fn run_generated_capability(inputs:"));
        assert!(callable.source.contains("Result<Value, String>"));
        assert!(callable.source.contains(&ir_sha256));
        assert!(!callable.source.contains("fn main()"));
        assert!(!callable.reads_input_file);
        assert!(!callable.writes_output_file);
        assert!(emit_neutral_text(&ir).contains("program"));
    }

    #[test]
    fn scalar_api_lowering_preserves_mixed_int_bool_types() {
        let predicate = ApiDefinition {
            api_token: "is_positive".to_string(),
            inputs: vec![ProgramType::Int],
            output: ProgramType::Bool,
            effect: Effect::Pure,
            preconditions: Vec::new(),
            postconditions: vec!["returns whether the input is positive".to_string()],
            formal_body: ScalarExpression::Binary {
                operator: BinaryOperator::GreaterThan,
                left: Box::new(ScalarExpression::Argument { index: 0 }),
                right: Box::new(ScalarExpression::Constant { value: 0 }),
            },
            examples: Vec::new(),
            randomized_symbol: false,
            provenance: vec!["TYPED_LOWERING_TEST".to_string()],
        };
        let negation = ApiDefinition {
            api_token: "invert".to_string(),
            inputs: vec![ProgramType::Bool],
            output: ProgramType::Bool,
            effect: Effect::Pure,
            preconditions: Vec::new(),
            postconditions: vec!["negates the input".to_string()],
            formal_body: ScalarExpression::Unary {
                operator: UnaryOperator::Not,
                input: Box::new(ScalarExpression::Argument { index: 0 }),
            },
            examples: Vec::new(),
            randomized_symbol: false,
            provenance: vec!["TYPED_LOWERING_TEST".to_string()],
        };

        assert_eq!(
            emit_api(&predicate).unwrap(),
            "fn is_positive(a0: i64) -> bool { (a0 > 0i64) }\n"
        );
        assert_eq!(
            emit_api(&negation).unwrap(),
            "fn invert(a0: bool) -> bool { (!a0) }\n"
        );
    }
}
