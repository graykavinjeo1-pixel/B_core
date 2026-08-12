use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    ApiDefinition, BinaryOperator, Effect, NodeKind, ProgramIR, ProgramNode, ProgramType,
    ScalarExpression, UnaryOperator, Value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeEffectAudit {
    pub output_type: ProgramType,
    pub observed_effects: BTreeSet<Effect>,
    pub visited_nodes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub value: Value,
    pub observed_effects: BTreeSet<Effect>,
    pub steps: usize,
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone)]
enum Signal {
    Value(Value),
    Return(Value),
    Break,
    Continue,
}

pub fn type_check(ir: &ProgramIR, apis: &[ApiDefinition]) -> Result<TypeEffectAudit, String> {
    let mut environment = ir
        .inputs
        .iter()
        .map(|binding| (binding.name.clone(), binding.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let mutable = ir
        .inputs
        .iter()
        .filter(|binding| binding.mutable)
        .map(|binding| binding.name.clone())
        .collect::<BTreeSet<_>>();
    let api_map = apis
        .iter()
        .map(|api| (api.api_token.clone(), api))
        .collect::<BTreeMap<_, _>>();
    let mut effects = BTreeSet::new();
    let mut visited = 0;
    let output = check_node(
        &ir.root,
        &mut environment,
        &mutable,
        &api_map,
        &mut effects,
        &mut visited,
        false,
    )?;
    if output != ir.output_type {
        return Err(format!(
            "PROGRAM_OUTPUT_TYPE_MISMATCH:{output:?}!={:?}",
            ir.output_type
        ));
    }
    let allowed = ir.allowed_effects.iter().cloned().collect::<BTreeSet<_>>();
    let forbidden = effects.difference(&allowed).cloned().collect::<Vec<_>>();
    if !forbidden.is_empty() {
        return Err(format!("FORBIDDEN_EFFECTS:{forbidden:?}"));
    }
    Ok(TypeEffectAudit {
        output_type: output,
        observed_effects: effects,
        visited_nodes: visited,
    })
}

#[allow(clippy::too_many_arguments)]
fn check_node(
    node: &ProgramNode,
    environment: &mut BTreeMap<String, ProgramType>,
    mutable_inputs: &BTreeSet<String>,
    apis: &BTreeMap<String, &ApiDefinition>,
    effects: &mut BTreeSet<Effect>,
    visited: &mut usize,
    in_loop: bool,
) -> Result<ProgramType, String> {
    *visited += 1;
    for effect in &node.meta.effects {
        effects.insert(effect.clone());
    }
    let inferred = match &node.kind {
        NodeKind::Literal { value } => value.program_type(),
        NodeKind::Variable { name } | NodeKind::Load { name } => environment
            .get(name)
            .cloned()
            .ok_or_else(|| format!("UNKNOWN_BINDING:{name}"))?,
        NodeKind::Store { name, value } => {
            let value_type = check_node(
                value,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            if let Some(existing) = environment.get(name) {
                if existing != &value_type {
                    return Err(format!("STORE_TYPE_MISMATCH:{name}"));
                }
                if mutable_inputs.contains(name)
                    || !node.meta.effects.contains(&Effect::LocalMutation)
                {
                    // Local bindings are mutable by construction; input mutation is explicit.
                }
            } else {
                environment.insert(name.clone(), value_type);
            }
            effects.insert(Effect::LocalMutation);
            ProgramType::Unit
        }
        NodeKind::UnaryOp { operator, input } => {
            let input_type = check_node(
                input,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            match (operator, input_type) {
                (UnaryOperator::Negate, ProgramType::Int) => ProgramType::Int,
                (UnaryOperator::Not, ProgramType::Bool) => ProgramType::Bool,
                _ => return Err("UNARY_TYPE_MISMATCH".to_string()),
            }
        }
        NodeKind::BinaryOp {
            operator,
            left,
            right,
        } => {
            let left_type = check_node(
                left,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            let right_type = check_node(
                right,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            infer_binary(*operator, left_type, right_type)?
        }
        NodeKind::SequenceCreate { elements } => {
            for element in elements {
                if check_node(
                    element,
                    environment,
                    mutable_inputs,
                    apis,
                    effects,
                    visited,
                    in_loop,
                )? != ProgramType::Int
                {
                    return Err("SEQUENCE_ELEMENT_NOT_INT".to_string());
                }
            }
            ProgramType::SequenceInt
        }
        NodeKind::SequenceRead { sequence, index } => {
            let sequence_type = check_node(
                sequence,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            let index_type = check_node(
                index,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            if index_type != ProgramType::Int {
                return Err("SEQUENCE_INDEX_NOT_INT".to_string());
            }
            match sequence_type {
                ProgramType::SequenceInt | ProgramType::Bytes | ProgramType::Image => {
                    ProgramType::Int
                }
                ProgramType::NestedSequenceInt => ProgramType::SequenceInt,
                _ => return Err("SEQUENCE_READ_SOURCE_TYPE".to_string()),
            }
        }
        NodeKind::SequenceLength { sequence } => {
            let sequence_type = check_node(
                sequence,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            if !matches!(
                sequence_type,
                ProgramType::SequenceInt
                    | ProgramType::NestedSequenceInt
                    | ProgramType::Bytes
                    | ProgramType::Image
            ) {
                return Err("SEQUENCE_LENGTH_SOURCE_TYPE".to_string());
            }
            ProgramType::Int
        }
        NodeKind::SequenceWrite {
            binding,
            index,
            value,
        } => {
            let binding_type = environment
                .get(binding)
                .cloned()
                .ok_or_else(|| format!("UNKNOWN_BUFFER:{binding}"))?;
            if !matches!(
                binding_type,
                ProgramType::SequenceInt | ProgramType::Bytes | ProgramType::Image
            ) || check_node(
                index,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )? != ProgramType::Int
                || check_node(
                    value,
                    environment,
                    mutable_inputs,
                    apis,
                    effects,
                    visited,
                    in_loop,
                )? != ProgramType::Int
            {
                return Err("SEQUENCE_WRITE_TYPE_MISMATCH".to_string());
            }
            effects.insert(Effect::BufferMutation);
            ProgramType::Unit
        }
        NodeKind::SequenceAppend { binding, value } => {
            if environment.get(binding) != Some(&ProgramType::SequenceInt)
                || check_node(
                    value,
                    environment,
                    mutable_inputs,
                    apis,
                    effects,
                    visited,
                    in_loop,
                )? != ProgramType::Int
            {
                return Err("SEQUENCE_APPEND_TYPE_MISMATCH".to_string());
            }
            effects.insert(Effect::BufferMutation);
            ProgramType::Unit
        }
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => {
            if check_node(
                condition,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )? != ProgramType::Bool
            {
                return Err("IF_CONDITION_NOT_BOOL".to_string());
            }
            let mut then_env = environment.clone();
            let mut else_env = environment.clone();
            let then_type = check_node(
                then_node,
                &mut then_env,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            let else_type = check_node(
                else_node,
                &mut else_env,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            if then_type != else_type {
                return Err("IF_BRANCH_TYPE_MISMATCH".to_string());
            }
            then_type
        }
        NodeKind::Loop {
            source,
            item_binding,
            index_binding,
            body,
        } => {
            let source_type = check_node(
                source,
                environment,
                mutable_inputs,
                apis,
                effects,
                visited,
                in_loop,
            )?;
            let item_type = match source_type {
                ProgramType::SequenceInt | ProgramType::Bytes | ProgramType::Image => {
                    ProgramType::Int
                }
                ProgramType::NestedSequenceInt => ProgramType::SequenceInt,
                _ => return Err("LOOP_SOURCE_TYPE".to_string()),
            };
            let mut loop_env = environment.clone();
            loop_env.insert(item_binding.clone(), item_type);
            loop_env.insert(index_binding.clone(), ProgramType::Int);
            let body_type = check_node(
                body,
                &mut loop_env,
                mutable_inputs,
                apis,
                effects,
                visited,
                true,
            )?;
            if body_type != ProgramType::Unit {
                return Err("LOOP_BODY_NOT_UNIT".to_string());
            }
            effects.insert(Effect::LocalMutation);
            ProgramType::Unit
        }
        NodeKind::Call { api_token, args } => {
            let api = apis
                .get(api_token)
                .ok_or_else(|| format!("UNKNOWN_API:{api_token}"))?;
            if args.len() != api.inputs.len() {
                return Err(format!("API_ARITY:{api_token}"));
            }
            for (argument, expected) in args.iter().zip(&api.inputs) {
                let actual = check_node(
                    argument,
                    environment,
                    mutable_inputs,
                    apis,
                    effects,
                    visited,
                    in_loop,
                )?;
                if &actual != expected {
                    return Err(format!("API_ARGUMENT_TYPE:{api_token}"));
                }
            }
            effects.insert(api.effect.clone());
            api.output.clone()
        }
        NodeKind::Return { value } => check_node(
            value,
            environment,
            mutable_inputs,
            apis,
            effects,
            visited,
            in_loop,
        )?,
        NodeKind::Block { nodes } => {
            let mut output = ProgramType::Unit;
            for child in nodes {
                output = check_node(
                    child,
                    environment,
                    mutable_inputs,
                    apis,
                    effects,
                    visited,
                    in_loop,
                )?;
            }
            output
        }
        NodeKind::Break | NodeKind::Continue => {
            if !in_loop {
                return Err("LOOP_CONTROL_OUTSIDE_LOOP".to_string());
            }
            ProgramType::Unit
        }
    };
    if inferred != node.meta.output_type {
        return Err(format!(
            "NODE_META_TYPE_MISMATCH:{}:{inferred:?}!={:?}",
            node.meta.node_id, node.meta.output_type
        ));
    }
    Ok(inferred)
}

fn infer_binary(
    operator: BinaryOperator,
    left: ProgramType,
    right: ProgramType,
) -> Result<ProgramType, String> {
    use BinaryOperator as Op;
    use ProgramType as Ty;
    match (operator, left, right) {
        (Op::Add | Op::Subtract | Op::Multiply | Op::Divide | Op::Modulo, Ty::Int, Ty::Int) => {
            Ok(Ty::Int)
        }
        (Op::Equal | Op::LessThan | Op::GreaterThan, Ty::Int, Ty::Int) => Ok(Ty::Bool),
        (Op::Equal, Ty::Bool, Ty::Bool) => Ok(Ty::Bool),
        (Op::And | Op::Or, Ty::Bool, Ty::Bool) => Ok(Ty::Bool),
        _ => Err("BINARY_TYPE_MISMATCH".to_string()),
    }
}

pub fn execute(
    ir: &ProgramIR,
    inputs: &BTreeMap<String, Value>,
    apis: &[ApiDefinition],
    files: BTreeMap<String, Vec<u8>>,
) -> Result<ExecutionOutcome, String> {
    type_check(ir, apis)?;
    let mut environment = BTreeMap::new();
    for binding in &ir.inputs {
        let value = inputs
            .get(&binding.name)
            .cloned()
            .ok_or_else(|| format!("MISSING_INPUT:{}", binding.name))?;
        if value.program_type() != binding.value_type {
            return Err(format!("INPUT_TYPE:{}", binding.name));
        }
        environment.insert(binding.name.clone(), value);
    }
    let api_map = apis
        .iter()
        .map(|api| (api.api_token.clone(), api))
        .collect::<BTreeMap<_, _>>();
    let mut effects = BTreeSet::new();
    let mut steps = 0;
    let mut file_state = files;
    let signal = eval_node(
        &ir.root,
        &mut environment,
        &api_map,
        &mut effects,
        &mut steps,
        &mut file_state,
    )?;
    let value = match signal {
        Signal::Value(value) | Signal::Return(value) => value,
        Signal::Break | Signal::Continue => return Err("UNHANDLED_LOOP_CONTROL".to_string()),
    };
    Ok(ExecutionOutcome {
        value,
        observed_effects: effects,
        steps,
        files: file_state,
    })
}

fn eval_node(
    node: &ProgramNode,
    environment: &mut BTreeMap<String, Value>,
    apis: &BTreeMap<String, &ApiDefinition>,
    effects: &mut BTreeSet<Effect>,
    steps: &mut usize,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<Signal, String> {
    *steps += 1;
    if *steps > 100_000 {
        return Err("IR_STEP_LIMIT".to_string());
    }
    let value = match &node.kind {
        NodeKind::Literal { value } => Signal::Value(value.clone()),
        NodeKind::Variable { name } | NodeKind::Load { name } => Signal::Value(
            environment
                .get(name)
                .cloned()
                .ok_or_else(|| format!("UNKNOWN_BINDING:{name}"))?,
        ),
        NodeKind::Store { name, value } => {
            let value = signal_value(eval_node(value, environment, apis, effects, steps, files)?)?;
            environment.insert(name.clone(), value);
            effects.insert(Effect::LocalMutation);
            Signal::Value(Value::Unit)
        }
        NodeKind::UnaryOp { operator, input } => {
            let input = signal_value(eval_node(input, environment, apis, effects, steps, files)?)?;
            Signal::Value(eval_unary(*operator, input)?)
        }
        NodeKind::BinaryOp {
            operator,
            left,
            right,
        } => {
            let left = signal_value(eval_node(left, environment, apis, effects, steps, files)?)?;
            let right = signal_value(eval_node(right, environment, apis, effects, steps, files)?)?;
            Signal::Value(eval_binary(*operator, left, right)?)
        }
        NodeKind::SequenceCreate { elements } => {
            let mut output = Vec::with_capacity(elements.len());
            for element in elements {
                output.push(as_int(signal_value(eval_node(
                    element,
                    environment,
                    apis,
                    effects,
                    steps,
                    files,
                )?)?)?);
            }
            Signal::Value(Value::Sequence(output))
        }
        NodeKind::SequenceRead { sequence, index } => {
            let sequence = signal_value(eval_node(
                sequence,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?;
            let index = as_index(signal_value(eval_node(
                index,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?)?;
            Signal::Value(read_index(sequence, index)?)
        }
        NodeKind::SequenceLength { sequence } => {
            let sequence = signal_value(eval_node(
                sequence,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?;
            Signal::Value(Value::Int(sequence_length(&sequence)?))
        }
        NodeKind::SequenceWrite {
            binding,
            index,
            value,
        } => {
            let index = as_index(signal_value(eval_node(
                index,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?)?;
            let value = as_int(signal_value(eval_node(
                value,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?)?;
            write_index(environment, binding, index, value)?;
            effects.insert(Effect::BufferMutation);
            Signal::Value(Value::Unit)
        }
        NodeKind::SequenceAppend { binding, value } => {
            let value = as_int(signal_value(eval_node(
                value,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?)?;
            match environment.get_mut(binding) {
                Some(Value::Sequence(values)) => values.push(value),
                _ => return Err(format!("APPEND_TARGET:{binding}")),
            }
            effects.insert(Effect::BufferMutation);
            Signal::Value(Value::Unit)
        }
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => {
            let condition = as_bool(signal_value(eval_node(
                condition,
                environment,
                apis,
                effects,
                steps,
                files,
            )?)?)?;
            if condition {
                eval_node(then_node, environment, apis, effects, steps, files)?
            } else {
                eval_node(else_node, environment, apis, effects, steps, files)?
            }
        }
        NodeKind::Loop {
            source,
            item_binding,
            index_binding,
            body,
        } => {
            let source =
                signal_value(eval_node(source, environment, apis, effects, steps, files)?)?;
            let items = iterable_values(source)?;
            for (index, item) in items.into_iter().enumerate() {
                environment.insert(item_binding.clone(), item);
                environment.insert(index_binding.clone(), Value::Int(index as i64));
                match eval_node(body, environment, apis, effects, steps, files)? {
                    Signal::Break => break,
                    Signal::Continue | Signal::Value(_) => {}
                    signal @ Signal::Return(_) => return Ok(signal),
                }
            }
            effects.insert(Effect::LocalMutation);
            Signal::Value(Value::Unit)
        }
        NodeKind::Call { api_token, args } => {
            let api = apis
                .get(api_token)
                .ok_or_else(|| format!("UNKNOWN_API:{api_token}"))?;
            let mut values = Vec::with_capacity(args.len());
            for argument in args {
                values.push(signal_value(eval_node(
                    argument,
                    environment,
                    apis,
                    effects,
                    steps,
                    files,
                )?)?);
            }
            effects.insert(api.effect.clone());
            Signal::Value(eval_scalar(&api.formal_body, &values, apis)?)
        }
        NodeKind::Return { value } => {
            let value = signal_value(eval_node(value, environment, apis, effects, steps, files)?)?;
            Signal::Return(value)
        }
        NodeKind::Block { nodes } => {
            let mut last = Signal::Value(Value::Unit);
            for child in nodes {
                last = eval_node(child, environment, apis, effects, steps, files)?;
                if matches!(last, Signal::Return(_) | Signal::Break | Signal::Continue) {
                    break;
                }
            }
            last
        }
        NodeKind::Break => Signal::Break,
        NodeKind::Continue => Signal::Continue,
    };
    let _ = files;
    Ok(value)
}

pub fn eval_scalar(
    expression: &ScalarExpression,
    arguments: &[Value],
    apis: &BTreeMap<String, &ApiDefinition>,
) -> Result<Value, String> {
    match expression {
        ScalarExpression::Argument { index } => arguments
            .get(*index)
            .cloned()
            .ok_or_else(|| format!("SCALAR_ARGUMENT:{index}")),
        ScalarExpression::Constant { value } => Ok(Value::Int(*value)),
        ScalarExpression::BoolConstant { value } => Ok(Value::Bool(*value)),
        ScalarExpression::Unary { operator, input } => {
            eval_unary(*operator, eval_scalar(input, arguments, apis)?)
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => eval_binary(
            *operator,
            eval_scalar(left, arguments, apis)?,
            eval_scalar(right, arguments, apis)?,
        ),
        ScalarExpression::Length { input } => Ok(Value::Int(sequence_length(&eval_scalar(
            input, arguments, apis,
        )?)?)),
        ScalarExpression::Index { collection, index } => {
            let collection = eval_scalar(collection, arguments, apis)?;
            let index = as_index(eval_scalar(index, arguments, apis)?)?;
            read_index(collection, index)
        }
        ScalarExpression::OpaqueCall { api_token, args } => {
            let api = apis
                .get(api_token)
                .ok_or_else(|| format!("UNKNOWN_API:{api_token}"))?;
            let values = args
                .iter()
                .map(|argument| eval_scalar(argument, arguments, apis))
                .collect::<Result<Vec<_>, _>>()?;
            eval_scalar(&api.formal_body, &values, apis)
        }
    }
}

fn eval_unary(operator: UnaryOperator, value: Value) -> Result<Value, String> {
    match (operator, value) {
        (UnaryOperator::Negate, Value::Int(value)) => Ok(Value::Int(-value)),
        (UnaryOperator::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        _ => Err("UNARY_VALUE_TYPE".to_string()),
    }
}

fn eval_binary(operator: BinaryOperator, left: Value, right: Value) -> Result<Value, String> {
    use BinaryOperator as Op;
    match (operator, left, right) {
        (Op::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.saturating_add(b))),
        (Op::Subtract, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.saturating_sub(b))),
        (Op::Multiply, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.saturating_mul(b))),
        (Op::Divide, Value::Int(_), Value::Int(0)) => Err("DIVISION_BY_ZERO".to_string()),
        (Op::Divide, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
        (Op::Modulo, Value::Int(_), Value::Int(0)) => Err("MODULO_BY_ZERO".to_string()),
        (Op::Modulo, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
        (Op::Equal, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
        (Op::Equal, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (Op::LessThan, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
        (Op::GreaterThan, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
        (Op::And, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Op::Or, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
        _ => Err("BINARY_VALUE_TYPE".to_string()),
    }
}

fn signal_value(signal: Signal) -> Result<Value, String> {
    match signal {
        Signal::Value(value) => Ok(value),
        Signal::Return(_) => Err("NESTED_RETURN_IN_EXPRESSION".to_string()),
        Signal::Break | Signal::Continue => Err("LOOP_CONTROL_IN_EXPRESSION".to_string()),
    }
}

fn as_int(value: Value) -> Result<i64, String> {
    match value {
        Value::Int(value) => Ok(value),
        _ => Err("EXPECTED_INT".to_string()),
    }
}

fn as_bool(value: Value) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err("EXPECTED_BOOL".to_string()),
    }
}

fn as_index(value: Value) -> Result<usize, String> {
    let value = as_int(value)?;
    usize::try_from(value).map_err(|_| "NEGATIVE_INDEX".to_string())
}

fn read_index(value: Value, index: usize) -> Result<Value, String> {
    match value {
        Value::Sequence(values) => values
            .get(index)
            .copied()
            .map(Value::Int)
            .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string()),
        Value::NestedSequence(values) => values
            .get(index)
            .cloned()
            .map(Value::Sequence)
            .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string()),
        Value::Bytes(values) => values
            .get(index)
            .copied()
            .map(|value| Value::Int(i64::from(value)))
            .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string()),
        Value::Image(image) => image
            .pixels
            .get(index)
            .copied()
            .map(Value::Int)
            .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string()),
        _ => Err("NOT_INDEXABLE".to_string()),
    }
}

fn sequence_length(value: &Value) -> Result<i64, String> {
    let length = match value {
        Value::Sequence(values) => values.len(),
        Value::NestedSequence(values) => values.len(),
        Value::Bytes(values) => values.len(),
        Value::Image(image) => image.pixels.len(),
        _ => return Err("SEQUENCE_LENGTH_SOURCE_TYPE".to_string()),
    };
    i64::try_from(length).map_err(|_| "SEQUENCE_LENGTH_OVERFLOW".to_string())
}

fn write_index(
    environment: &mut BTreeMap<String, Value>,
    binding: &str,
    index: usize,
    value: i64,
) -> Result<(), String> {
    match environment.get_mut(binding) {
        Some(Value::Sequence(values)) => {
            *values
                .get_mut(index)
                .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string())? = value
        }
        Some(Value::Bytes(values)) => {
            *values
                .get_mut(index)
                .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string())? =
                u8::try_from(value).map_err(|_| "BYTE_RANGE".to_string())?;
        }
        Some(Value::Image(image)) => {
            *image
                .pixels
                .get_mut(index)
                .ok_or_else(|| "INDEX_OUT_OF_BOUNDS".to_string())? = value
        }
        _ => return Err("NOT_WRITABLE_BUFFER".to_string()),
    }
    Ok(())
}

fn iterable_values(value: Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Sequence(values) => Ok(values.into_iter().map(Value::Int).collect()),
        Value::NestedSequence(values) => Ok(values.into_iter().map(Value::Sequence).collect()),
        Value::Bytes(values) => Ok(values
            .into_iter()
            .map(|value| Value::Int(i64::from(value)))
            .collect()),
        Value::Image(image) => Ok(image.pixels.into_iter().map(Value::Int).collect()),
        _ => Err("NOT_ITERABLE".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem5::model::{BindingSpec, NodeMeta};

    fn meta(id: &str, output_type: ProgramType, effects: Vec<Effect>) -> NodeMeta {
        NodeMeta {
            node_id: id.to_string(),
            input_types: Vec::new(),
            output_type,
            preconditions: Vec::new(),
            effects,
            data_dependencies: Vec::new(),
            control_dependencies: Vec::new(),
            provenance: vec!["TEST".to_string()],
            primitive_cost: 1,
        }
    }

    #[test]
    fn executes_typed_scalar_program() {
        let root = ProgramNode {
            meta: meta("return", ProgramType::Int, vec![Effect::Pure]),
            kind: NodeKind::Return {
                value: Box::new(ProgramNode {
                    meta: meta("add", ProgramType::Int, vec![Effect::Pure]),
                    kind: NodeKind::BinaryOp {
                        operator: BinaryOperator::Add,
                        left: Box::new(ProgramNode {
                            meta: meta("load", ProgramType::Int, vec![Effect::Pure]),
                            kind: NodeKind::Load {
                                name: "x".to_string(),
                            },
                        }),
                        right: Box::new(ProgramNode {
                            meta: meta("one", ProgramType::Int, vec![Effect::Pure]),
                            kind: NodeKind::Literal {
                                value: Value::Int(1),
                            },
                        }),
                    },
                }),
            },
        };
        let ir = ProgramIR {
            program_id: "test".to_string(),
            inputs: vec![BindingSpec {
                name: "x".to_string(),
                value_type: ProgramType::Int,
                mutable: false,
            }],
            output_type: ProgramType::Int,
            allowed_effects: vec![Effect::Pure],
            root,
            concept_ids: Vec::new(),
            graph_edges: Vec::new(),
            provenance: vec!["TEST".to_string()],
            primitive_expanded_nodes: 4,
            operational_nodes: 4,
            solution_graph_depth: 3,
            primitive_expanded_depth: 3,
            search_trajectory_depth: 3,
            simultaneous_subproblems: 1,
            recombinations: 0,
        };
        let inputs = BTreeMap::from([("x".to_string(), Value::Int(4))]);
        let outcome = execute(&ir, &inputs, &[], BTreeMap::new()).expect("execute");
        assert_eq!(outcome.value, Value::Int(5));
    }

    #[test]
    fn rejects_forbidden_effect() {
        let root = ProgramNode {
            meta: meta("store", ProgramType::Unit, vec![Effect::LocalMutation]),
            kind: NodeKind::Store {
                name: "x".to_string(),
                value: Box::new(ProgramNode {
                    meta: meta("one", ProgramType::Int, vec![Effect::Pure]),
                    kind: NodeKind::Literal {
                        value: Value::Int(1),
                    },
                }),
            },
        };
        let ir = ProgramIR {
            program_id: "bad-effect".to_string(),
            inputs: Vec::new(),
            output_type: ProgramType::Unit,
            allowed_effects: vec![Effect::Pure],
            root,
            concept_ids: Vec::new(),
            graph_edges: Vec::new(),
            provenance: vec!["TEST".to_string()],
            primitive_expanded_nodes: 2,
            operational_nodes: 2,
            solution_graph_depth: 2,
            primitive_expanded_depth: 2,
            search_trajectory_depth: 2,
            simultaneous_subproblems: 1,
            recombinations: 0,
        };
        assert!(type_check(&ir, &[]).is_err());
    }
}
