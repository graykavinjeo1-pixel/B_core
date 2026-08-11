use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use super::model::{
    ApiDefinition, BinaryOperator, Effect, ImageValue, NodeKind, ProgramIR, ProgramNode,
    ProgramType, ScalarExpression, UnaryOperator, Value,
};

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
    output.push_str("#![allow(dead_code)]\n\n");
    output.push_str("use std::collections::BTreeMap;\n");
    output.push_str("use crate::sem5::model::{ImageValue, Value};\n\n");
    output.push_str("#[derive(Clone, Debug)]\nstruct Sem5Image { width: usize, height: usize, channels: usize, pixels: Vec<i64> }\n\n");
    for api in apis {
        output.push_str(&emit_api(api)?);
        output.push('\n');
    }
    output.push_str("pub const GENERATED_CAPABILITY_ACTIVE: bool = true;\n");
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
        ProgramType::SequenceInt => "Value::Sequence(sem5_result)",
        ProgramType::NestedSequenceInt => "Value::NestedSequence(sem5_result)",
        ProgramType::Bytes => "Value::Bytes(sem5_result)",
        ProgramType::Image => "Value::Image(ImageValue { width: sem5_result.width, height: sem5_result.height, channels: sem5_result.channels, pixels: sem5_result.pixels })",
        ProgramType::Unit => "Value::Unit",
    };
    format!("    Ok({expression})\n")
}

fn emit_api(api: &ApiDefinition) -> Result<String, String> {
    if api.inputs.iter().any(|input| input != &ProgramType::Int)
        || api.output != ProgramType::Int
        || api.effect != Effect::Pure
    {
        return Err(format!("RUST_MIN_API_SIGNATURE:{}", api.api_token));
    }
    let parameters = api
        .inputs
        .iter()
        .enumerate()
        .map(|(index, _)| format!("a{index}: i64"))
        .collect::<Vec<_>>()
        .join(", ");
    let body = emit_scalar_expression(&api.formal_body)?;
    Ok(format!(
        "fn {}({parameters}) -> i64 {{ {body} }}\n",
        api.api_token
    ))
}

struct EmitContext {
    declared: BTreeSet<String>,
    binding_types: BTreeMap<String, ProgramType>,
    next_loop: usize,
    result_declared: bool,
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
            let expression = emit_expression(value)?;
            if context.declared.insert(name.clone()) {
                context
                    .binding_types
                    .insert(name.clone(), value.meta.output_type.clone());
                output.push_str(&format!(
                    "{prefix}let mut {name}: {} = {expression};\n",
                    rust_type(&value.meta.output_type)
                ));
            } else {
                output.push_str(&format!("{prefix}{name} = {expression};\n"));
            }
        }
        NodeKind::SequenceWrite {
            binding,
            index,
            value,
        } => {
            let index = emit_expression(index)?;
            let value = emit_expression(value)?;
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
            output.push_str(&format!(
                "{prefix}{binding}.push({});\n",
                emit_expression(value)?
            ));
        }
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => {
            output.push_str(&format!("{prefix}if {} {{\n", emit_expression(condition)?));
            emit_statement(then_node, indent + 1, context, output)?;
            output.push_str(&format!("{prefix}}} else {{\n"));
            emit_statement(else_node, indent + 1, context, output)?;
            output.push_str(&format!("{prefix}}}\n"));
        }
        NodeKind::Loop {
            source,
            item_binding,
            index_binding,
            body,
        } => {
            let loop_id = context.next_loop;
            context.next_loop += 1;
            let source_expression = emit_expression(source)?;
            let raw_item = format!("raw_item_{loop_id}");
            let raw_index = format!("raw_index_{loop_id}");
            let iterator = if source.meta.output_type == ProgramType::Image {
                format!("({source_expression}).pixels.clone().into_iter()")
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
                emit_expression(value)?
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
            output.push_str(&format!("{prefix}();\n"));
        }
        _ => {
            output.push_str(&format!("{prefix}let _ = {};\n", emit_expression(node)?));
        }
    }
    Ok(())
}

fn emit_expression(node: &ProgramNode) -> Result<String, String> {
    match &node.kind {
        NodeKind::Literal { value } => rust_literal(value),
        NodeKind::Variable { name } | NodeKind::Load { name } => Ok(name.clone()),
        NodeKind::UnaryOp { operator, input } => {
            let operator = match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            };
            Ok(format!("({operator}{})", emit_expression(input)?))
        }
        NodeKind::BinaryOp {
            operator,
            left,
            right,
        } => Ok(format!(
            "({} {} {})",
            emit_expression(left)?,
            binary_token(*operator),
            emit_expression(right)?
        )),
        NodeKind::SequenceCreate { elements } => Ok(format!(
            "vec![{}]",
            elements
                .iter()
                .map(emit_expression)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        NodeKind::SequenceRead { sequence, index } => {
            let access = if sequence.meta.output_type == ProgramType::Image {
                format!(
                    "({}).pixels[({}) as usize]",
                    emit_expression(sequence)?,
                    emit_expression(index)?
                )
            } else {
                format!(
                    "({})[({}) as usize]",
                    emit_expression(sequence)?,
                    emit_expression(index)?
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
        NodeKind::Call { api_token, args } => Ok(format!(
            "{}({})",
            api_token,
            args.iter()
                .map(emit_expression)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => Ok(format!(
            "if {} {{ {} }} else {{ {} }}",
            emit_expression(condition)?,
            emit_expression(then_node)?,
            emit_expression(else_node)?
        )),
        NodeKind::Block { nodes } => Ok(format!(
            "{{ {} }}",
            nodes
                .iter()
                .map(emit_expression)
                .collect::<Result<Vec<_>, _>>()?
                .join("; ")
        )),
        _ => Err(format!("NODE_NOT_EXPRESSION:{}", node.meta.node_id)),
    }
}

fn emit_scalar_expression(expression: &ScalarExpression) -> Result<String, String> {
    match expression {
        ScalarExpression::Argument { index } => Ok(format!("a{index}")),
        ScalarExpression::Constant { value } => Ok(format!("{value}i64")),
        ScalarExpression::Unary { operator, input } => Ok(format!(
            "({}{})",
            match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            },
            emit_scalar_expression(input)?
        )),
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
        BinaryOperator::LessThan => "<",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
    }
}

fn rust_type(value_type: &ProgramType) -> &'static str {
    match value_type {
        ProgramType::Int => "i64",
        ProgramType::Bool => "bool",
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
        ProgramType::Int | ProgramType::Bool => {
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
}
